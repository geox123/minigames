//! The shell's front-end: the mode-select in front of a match and the flow
//! between it and play. Everything here is window, input and rendering glue
//! around the pure core, which is why it lives in the shell, not `breakout_core`.

use breakout_core::{Game, TIMESTEP};
use breakout_remix_core::meta::{Content, Outcome, Unlocked};
use breakout_remix_core::{
    BALL_SIZE as RIFT_BALL_SIZE, Game as RiftGame, Phase as RiftPhase, TIMESTEP as RIFT_TIMESTEP,
};
use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;

use crate::audio::Audio;
use crate::fx::Fx;
use crate::{read_input, render, rift};

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make
/// the game try to catch up by simulating seconds at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The two takes every Game in the Collection ships. Both are now playable: the
/// Faithful, and RIFT — its Remix.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The faithful recreation.
    Faithful,
    /// RIFT, the reimagined version.
    Remix,
}

/// The modes RIFT itself offers, chosen from its own menu.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RiftMode {
    /// A fresh, randomly-seeded descent.
    Run,
    /// The day's shared seed — one fair attempt, the day's best kept.
    Daily,
    /// A mastery ladder: win to unlock the next, tougher tier.
    Ascension,
}

impl RiftMode {
    /// A short name for the summary card and the menu.
    pub fn label(self) -> &'static str {
        match self {
            RiftMode::Run => "RUN",
            RiftMode::Daily => "DAILY",
            RiftMode::Ascension => "ASCENSION",
        }
    }
}

/// A row on RIFT's menu: one of its modes to play, or the collection to browse.
/// Keeping this separate from [`RiftMode`] means a mode is always something you
/// can actually start a run in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuRow {
    /// Start a run in this mode.
    Mode(RiftMode),
    /// Open the collection screen.
    Collection,
}

impl MenuRow {
    /// Every row, in the order the menu lists them.
    pub const ROWS: [MenuRow; 4] = [
        MenuRow::Mode(RiftMode::Run),
        MenuRow::Mode(RiftMode::Daily),
        MenuRow::Mode(RiftMode::Ascension),
        MenuRow::Collection,
    ];

    /// The next row down (the menu wraps).
    fn next(self) -> Self {
        let at = Self::ROWS.iter().position(|row| *row == self).unwrap_or(0);
        Self::ROWS[(at + 1) % Self::ROWS.len()]
    }

    /// The row's label.
    pub fn label(self) -> &'static str {
        match self {
            MenuRow::Mode(mode) => mode.label(),
            MenuRow::Collection => "COLLECTION",
        }
    }
}

/// Which screen the player is looking at.
enum Screen {
    /// The Collection's two-takes screen: Faithful or RIFT.
    ModeSelect { highlight: Mode },
    /// A Faithful match in progress.
    Match {
        game: Game,
        /// Left-over real time not yet folded into a fixed step.
        accumulator: Accumulator,
        /// Whether the match is paused.
        paused: bool,
    },
    /// RIFT's own menu: its modes, and the collection.
    RiftMenu { highlight: MenuRow },
    /// The collection screen, browsing what has been unlocked.
    Collection { unlocked: Unlocked },
    /// A RIFT run in progress. The run is boxed: it is much the largest thing a
    /// screen carries, and every other screen would otherwise pay for its size.
    Rift {
        game: Box<RiftGame>,
        /// Left-over real time not yet folded into a fixed step.
        accumulator: Accumulator,
        /// Whether the run is paused.
        paused: bool,
        /// Which mode this run is.
        mode: RiftMode,
        /// The calendar day this run belongs to (a Daily's key; 0 for a Run).
        day: u32,
        /// The best depth to beat and show for this mode, updated when beaten.
        best: u32,
        /// The content this player has unlocked; the run draws its pool from it.
        unlocked: Unlocked,
        /// What this run newly unlocked, announced on the summary card.
        earned: Vec<Content>,
        /// The run's feel — trail, particles, shake, hit-stop.
        fx: Fx,
    },
}

/// The whole shell: the current screen, the seed source for new matches, the
/// sounds, and whether the window is fullscreen.
pub struct App {
    screen: Screen,
    next_seed: u64,
    audio: Audio,
    fullscreen: bool,
    /// The screen-shake offset to blit by this frame, in logical units.
    blit_shake: Vec2,
}

impl App {
    /// Opens the shell on the mode-select screen.
    pub fn new(audio: Audio) -> Self {
        Self {
            screen: Screen::ModeSelect {
                highlight: Mode::Faithful,
            },
            next_seed: seed_from_clock(),
            audio,
            fullscreen: false,
            blit_shake: Vec2::ZERO,
        }
    }

    /// The screen-shake offset the window should blit the canvas by this frame.
    pub fn blit_shake(&self) -> Vec2 {
        self.blit_shake
    }

    /// Advances the shell by one real frame: reads input, runs whatever the
    /// current screen does, and draws it to the logical canvas.
    pub fn frame(&mut self) {
        // Fullscreen can be toggled from anywhere in the shell.
        if is_key_pressed(KeyCode::F) {
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
        }
        // Reset the shake each frame; a RIFT run sets it below.
        self.blit_shake = Vec2::ZERO;

        match &mut self.screen {
            Screen::ModeSelect { highlight } => {
                if mode_select_input(highlight) {
                    match *highlight {
                        Mode::Faithful => self.start_match(),
                        Mode::Remix => self.open_rift_menu(),
                    }
                } else {
                    render::mode_select(*highlight);
                }
            }
            Screen::RiftMenu { highlight } => {
                // Backing out returns to the Collection's two-takes screen.
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                if pressed_menu_move() {
                    *highlight = highlight.next();
                }
                // Copy the choice out before the &mut self call, so the argument
                // doesn't hold a borrow of `self.screen` across it.
                let chosen = *highlight;
                if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                    match chosen {
                        MenuRow::Mode(mode) => self.start_run(mode),
                        MenuRow::Collection => self.open_collection(),
                    }
                } else {
                    render::rift_menu(chosen, breakout_storage::ascension_tier());
                }
            }
            Screen::Collection { unlocked } => {
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                    self.open_rift_menu();
                    return;
                }
                rift::draw_collection(*unlocked);
            }
            Screen::Match {
                game,
                accumulator,
                paused,
            } => {
                // Backing out of a match returns to the Collection's mode-select.
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                if is_key_pressed(KeyCode::P) {
                    *paused = !*paused;
                }
                if is_key_pressed(KeyCode::R) {
                    game.restart();
                    *paused = false;
                }

                if !*paused {
                    let input = read_input();
                    for _ in 0..accumulator.steps(get_frame_time()) {
                        self.audio.play(game.step(input));
                    }
                } else {
                    // Don't let paused wall-time pile up and fast-forward on resume.
                    accumulator.reset();
                }

                render::draw(game);
                if *paused {
                    render::paused_overlay();
                }
            }
            Screen::Rift {
                game,
                accumulator,
                paused,
                mode,
                day,
                best,
                unlocked,
                earned,
                fx,
            } => {
                // Backing out of a run returns to RIFT's mode menu.
                if is_key_pressed(KeyCode::Escape) {
                    self.open_rift_menu();
                    return;
                }
                if is_key_pressed(KeyCode::R) {
                    game.restart();
                    *paused = false;
                }
                // Pause only applies while the ball is live, not mid-draft.
                let live = matches!(game.phase(), RiftPhase::Serving | RiftPhase::Playing);
                if live && is_key_pressed(KeyCode::P) {
                    *paused = !*paused;
                }

                let dt = get_frame_time();
                fx.update(dt);

                if game.phase() == RiftPhase::Drafting {
                    // A draft is a menu, not real-time: one input per frame, off
                    // the accumulator, so a held key never repeats.
                    accumulator.reset();
                    self.audio.play_rift(game.step(rift::read_draft_input()));
                    if game.phase() != RiftPhase::Drafting {
                        // A boon was just taken — the draft is done.
                        self.audio.play_select();
                        fx.beat();
                    }
                } else if !*paused && !fx.frozen() {
                    let input = rift::read_play_input();
                    for _ in 0..accumulator.steps(dt) {
                        let events = game.step(input);
                        self.audio.play_rift(events);
                        fx.on_step(events, game.ball(), game.phase() == RiftPhase::Playing);
                        // A run that just ended may better its mode's record: a
                        // deeper Run/Daily, or — on a win — the next Ascension
                        // tier unlocked.
                        if events.won || events.lost {
                            match mode {
                                RiftMode::Run => {
                                    let depth = game.depth();
                                    if depth > *best {
                                        *best = depth;
                                        breakout_storage::set_best_depth(depth);
                                    }
                                }
                                RiftMode::Daily => {
                                    let depth = game.depth();
                                    if depth > *best {
                                        *best = depth;
                                        breakout_storage::set_daily_best(*day, depth);
                                    }
                                }
                                RiftMode::Ascension => {
                                    if events.won {
                                        let next = game.tier() + 1;
                                        if next > *best {
                                            *best = next;
                                            breakout_storage::set_ascension_tier(next);
                                        }
                                    }
                                }
                            }

                            // What the run achieved may earn new content, shared
                            // across every mode — one collection per player.
                            let newly = unlocked.record(Outcome {
                                won: events.won,
                                depth: game.depth(),
                                score: game.score(),
                                tier: game.tier(),
                            });
                            if !newly.is_empty() {
                                breakout_storage::set_unlocked_bits(unlocked.bits());
                                earned.extend(newly);
                            }
                        }
                    }
                } else {
                    // Paused, or held for hit-stop: don't pile up wall-time.
                    accumulator.reset();
                }

                self.blit_shake = Vec2::from(fx.shake_offset());

                rift::draw(game);
                let ball = game.ball();
                fx.draw(RIFT_BALL_SIZE, ball.vx.hypot(ball.vy));
                match game.phase() {
                    RiftPhase::Drafting => rift::draw_draft(game),
                    RiftPhase::Won | RiftPhase::Lost => {
                        let best_line = match mode {
                            RiftMode::Run => format!("RUN BEST DEPTH {best}"),
                            RiftMode::Daily => format!("DAILY BEST DEPTH {best}"),
                            RiftMode::Ascension => {
                                format!("ASCENSION TIER {}   BEST {best}", game.tier())
                            }
                        };
                        rift::run_summary(game, &best_line, &earned_line(earned));
                    }
                    _ => {}
                }
                if *paused {
                    render::paused_overlay();
                }
            }
        }
    }

    fn return_to_mode_select(&mut self) {
        self.screen = Screen::ModeSelect {
            highlight: Mode::Faithful,
        };
    }

    /// Consumes the next seed, advancing it so the next match differs.
    fn take_seed(&mut self) -> u64 {
        let seed = self.next_seed;
        self.next_seed = self.next_seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        seed
    }

    fn start_match(&mut self) {
        let game = Game::new(self.take_seed());
        self.screen = Screen::Match {
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            paused: false,
        };
    }

    fn open_rift_menu(&mut self) {
        self.screen = Screen::RiftMenu {
            highlight: MenuRow::Mode(RiftMode::Run),
        };
    }

    /// Opens the collection on what this player has unlocked so far.
    fn open_collection(&mut self) {
        self.screen = Screen::Collection {
            unlocked: Unlocked::from_bits(breakout_storage::unlocked_bits()),
        };
    }

    /// Starts a RIFT run in the chosen mode. A Run takes a fresh seed and the
    /// saved Run best; a Daily takes the day's shared seed and the day's best; an
    /// Ascension takes a fresh seed at the highest unlocked tier.
    fn start_run(&mut self, mode: RiftMode) {
        // A run draws only on what this player has earned, so the game opens up
        // as it is played. The core just receives a pool; it knows no more.
        let unlocked = Unlocked::from_bits(breakout_storage::unlocked_bits());
        let pool = unlocked.pool();
        let (game, day, best) = match mode {
            RiftMode::Run => (
                RiftGame::new_run(self.take_seed(), &pool),
                0,
                breakout_storage::best_depth(),
            ),
            RiftMode::Daily => {
                let day = today();
                (
                    RiftGame::new_run(u64::from(day), &pool),
                    day,
                    breakout_storage::daily_best(day),
                )
            }
            RiftMode::Ascension => {
                let tier = breakout_storage::ascension_tier();
                (
                    RiftGame::new_ascension(self.take_seed(), tier, &pool),
                    0,
                    tier,
                )
            }
        };
        self.screen = Screen::Rift {
            game: Box::new(game),
            accumulator: Accumulator::new(RIFT_TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            mode,
            day,
            best,
            unlocked,
            earned: Vec::new(),
            fx: Fx::default(),
        };
    }
}

/// The run-summary line announcing what a run unlocked, or empty if it unlocked
/// nothing. A big haul is trimmed so the line stays on the card.
fn earned_line(earned: &[Content]) -> String {
    const SHOWN: usize = 3;
    if earned.is_empty() {
        return String::new();
    }
    let names: Vec<&str> = earned.iter().take(SHOWN).map(|c| c.label()).collect();
    let mut line = format!("UNLOCKED  {}", names.join("  "));
    if earned.len() > SHOWN {
        line.push_str(&format!("  +{} MORE", earned.len() - SHOWN));
    }
    line
}

/// Today's calendar day, as whole days since the Unix epoch. The core stays
/// clock-free; only the shell reads the clock, so a Daily's seed is shared by
/// everyone playing on the same day.
fn today() -> u32 {
    (miniquad::date::now() / 86_400.0) as u32
}

/// Reads the mode-select screen, moving the highlight between the two takes.
/// Returns whether the player committed to the highlighted one.
fn mode_select_input(highlight: &mut Mode) -> bool {
    if pressed_menu_move() {
        *highlight = match *highlight {
            Mode::Faithful => Mode::Remix,
            Mode::Remix => Mode::Faithful,
        };
    }
    is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)
}

/// Whether the player nudged a menu highlight this frame.
fn pressed_menu_move() -> bool {
    is_key_pressed(KeyCode::Up)
        || is_key_pressed(KeyCode::Down)
        || is_key_pressed(KeyCode::Left)
        || is_key_pressed(KeyCode::Right)
        || is_key_pressed(KeyCode::W)
        || is_key_pressed(KeyCode::S)
}

/// A seed for a match. The core is deterministic by design, so the only
/// nondeterminism in the game is this one number, read from the clock.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
