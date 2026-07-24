//! The shell's front-end: the mode-select in front of a game and the flow
//! between it and play. Everything here is window, input and rendering glue
//! around the pure core, which is why it lives in the shell, not `stepfall_core`.

use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;
use stepfall_core::{Game, Phase, TIMESTEP};
use stepfall_remix_core::{
    BOSS_HEIGHT, BOSS_WIDTH, Game as RemixGame, Mode as RunMode, Outcome as RunOutcome,
    Phase as RemixPhase, SHIP_HEIGHT, SHIP_WIDTH, meta,
};

use crate::fx::Fx;
use crate::{Audio, read_input, read_remix_input, render};

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make
/// the game try to catch up by simulating seconds at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The two takes every Game in the Collection ships — both playable: the Faithful
/// and HAILFALL, its Remix.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The faithful recreation.
    Faithful,
    /// The reimagined version, HAILFALL.
    Remix,
}

/// A row on HAILFALL's menu: one of its modes to play, the Ascension ladder, or
/// the collection to browse.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuRow {
    /// Start a run in this mode.
    Mode(RunMode),
    /// Start an Ascension run at the reached tier.
    Ascension,
    /// Open the collection screen.
    Collection,
}

impl MenuRow {
    /// The rows, top to bottom.
    pub const ROWS: [MenuRow; 5] = [
        MenuRow::Mode(RunMode::Sortie),
        MenuRow::Mode(RunMode::Onslaught),
        MenuRow::Mode(RunMode::Daily),
        MenuRow::Ascension,
        MenuRow::Collection,
    ];

    /// The label shown for this row.
    pub fn label(self) -> &'static str {
        match self {
            MenuRow::Mode(RunMode::Sortie) => "SORTIE",
            MenuRow::Mode(RunMode::Onslaught) => "ONSLAUGHT",
            MenuRow::Mode(RunMode::Daily) => "DAILY",
            MenuRow::Ascension => "ASCENSION",
            MenuRow::Collection => "COLLECTION",
        }
    }
}

/// Which screen the player is looking at.
enum Screen {
    /// The Collection's two-takes screen: Faithful or the locked Remix.
    ModeSelect { highlight: Mode },
    /// A Faithful game in progress. The game is boxed: it dwarfs the
    /// mode-select variant, so keeping it behind a pointer keeps `Screen` small.
    Match {
        game: Box<Game>,
        /// Left-over real time not yet folded into a fixed step.
        accumulator: Accumulator,
        /// Whether the game is paused.
        paused: bool,
        /// The march frame last seen, so a flip (one formation step) triggers the
        /// next march note; the note to play next; and whether the saucer's
        /// warble is currently sounding.
        march_frame: u8,
        march_note: usize,
        saucer_sounding: bool,
    },
    /// HAILFALL's menu — its three modes, and the collection.
    RemixSelect { highlight: MenuRow },
    /// The collection screen, browsing what has been unlocked.
    Collection { unlocked: meta::Unlocked },
    /// A HAILFALL run in progress — the Remix. Boxed like the Faithful's game.
    RemixMatch {
        game: Box<RemixGame>,
        accumulator: Accumulator,
        /// The run's feel: trails, particles, shake and hit-stop.
        fx: Fx,
        paused: bool,
        /// Which mode this run is.
        mode: RunMode,
        /// The calendar day a Daily run belongs to (its saved key; 0 otherwise).
        day: u32,
        /// The best score to beat and show for this mode (0 for Sortie/Ascension).
        best: u32,
        /// Whether this is an Ascension run — its tier is read from the game, and a
        /// win pushes the reached tier up.
        ascension: bool,
        /// The options unlocked so far — this run's loadout was built from it, and
        /// its outcome is recorded back into it.
        unlocked: meta::Unlocked,
        /// Options newly unlocked by this run, for the summary to call out.
        earned: Vec<meta::Content>,
        /// Whether this run's result has been saved yet (saved once, on run-over).
        saved: bool,
    },
}

/// The whole shell: the current screen, the seed source for new games, whether
/// the window is fullscreen, the session best, and the sound.
pub struct App {
    screen: Screen,
    next_seed: u64,
    fullscreen: bool,
    /// The best score this session, carried across restarts and new games.
    best: u32,
    /// How far to nudge the blit this frame — HAILFALL's screen shake, else zero.
    blit_shake: Vec2,
    audio: Audio,
}

impl App {
    /// Opens the shell on the mode-select screen.
    pub fn new(audio: Audio) -> Self {
        Self {
            screen: Screen::ModeSelect {
                highlight: Mode::Faithful,
            },
            next_seed: seed_from_clock(),
            fullscreen: false,
            best: 0,
            blit_shake: Vec2::ZERO,
            audio,
        }
    }

    /// How far to nudge the canvas when it is blitted this frame.
    pub fn blit_shake(&self) -> Vec2 {
        self.blit_shake
    }

    /// Advances the shell by one real frame: reads input, runs whatever the
    /// current screen does, and draws it to the logical canvas.
    pub fn frame(&mut self) {
        // Any shake is set anew each frame by the run that is playing.
        self.blit_shake = Vec2::ZERO;

        // Fullscreen can be toggled from anywhere in the shell.
        if is_key_pressed(KeyCode::F) {
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
        }

        match &mut self.screen {
            Screen::ModeSelect { highlight } => {
                if mode_select_input(highlight) {
                    match *highlight {
                        Mode::Faithful => self.start_match(),
                        // The Remix opens its own menu first.
                        Mode::Remix => self.open_remix_menu(),
                    }
                } else {
                    render::mode_select(*highlight);
                }
            }
            Screen::RemixSelect { highlight } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                if let Some(chosen) = remix_menu_input(highlight) {
                    match chosen {
                        MenuRow::Mode(mode) => self.start_remix_match(mode),
                        MenuRow::Ascension => self.start_ascension_match(),
                        MenuRow::Collection => self.open_collection(),
                    }
                } else {
                    render::remix_select(*highlight);
                }
            }
            Screen::Collection { unlocked } => {
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                    self.open_remix_menu();
                    return;
                }
                render::draw_collection(*unlocked);
            }
            Screen::Match {
                game,
                accumulator,
                paused,
                march_frame,
                march_note,
                saucer_sounding,
            } => {
                // Backing out of a game returns to the Collection's mode-select.
                if is_key_pressed(KeyCode::Escape) {
                    self.audio.set_saucer(false);
                    self.return_to_mode_select();
                    return;
                }
                if is_key_pressed(KeyCode::P) {
                    *paused = !*paused;
                }
                if is_key_pressed(KeyCode::R) {
                    game.restart();
                    *paused = false;
                    *march_frame = game.march_frame();
                    *march_note = 0;
                }

                if !*paused {
                    let input = read_input();
                    for _ in 0..accumulator.steps(get_frame_time()) {
                        self.audio.play(&game.step(input));
                    }
                    // The march is the sound of the game: one of the four
                    // descending notes each time the formation takes a step (its
                    // frame flips), so the tempo is the march's own — faster as it
                    // thins, frantic for the last invader.
                    if game.march_frame() != *march_frame {
                        *march_frame = game.march_frame();
                        self.audio.march_note(*march_note);
                        *march_note += 1;
                    }
                } else {
                    // Don't let paused wall-time pile up and fast-forward on resume.
                    accumulator.reset();
                }

                // The saucer warbles while it crosses a live, unpaused game.
                let should_warble =
                    !*paused && game.saucer().is_some() && game.phase() == Phase::Playing;
                if should_warble != *saucer_sounding {
                    *saucer_sounding = should_warble;
                    self.audio.set_saucer(should_warble);
                }

                self.best = self.best.max(game.score());
                render::draw(game, self.best);
                if *paused {
                    render::paused_overlay();
                }
            }
            Screen::RemixMatch {
                game,
                accumulator,
                fx,
                paused,
                mode,
                day,
                best,
                ascension,
                unlocked,
                earned,
                saved,
            } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                // A fresh run of the same mode, from the summary or mid-run.
                if is_key_pressed(KeyCode::R) {
                    let mode = *mode;
                    self.start_remix_match(mode);
                    return;
                }

                let dt = get_frame_time();
                fx.update(dt);

                let over = game.phase() == RemixPhase::Over;
                if !over {
                    if is_key_pressed(KeyCode::P) {
                        *paused = !*paused;
                    }
                    // Hit-stop holds the sim still for a beat on the big impacts.
                    if !*paused && !fx.frozen() {
                        let input = read_remix_input();
                        for _ in 0..accumulator.steps(dt) {
                            let events = game.step(input);
                            self.audio.play_remix(&events);
                            let ship = game.ship();
                            let ship_centre =
                                (ship.x + SHIP_WIDTH / 2.0, ship.y + SHIP_HEIGHT / 2.0);
                            let boss_centre = game
                                .boss()
                                .map(|b| (b.x + BOSS_WIDTH / 2.0, b.y + BOSS_HEIGHT / 2.0));
                            fx.on_step(
                                events,
                                ship_centre,
                                boss_centre,
                                game.phase() == RemixPhase::Playing,
                            );
                        }
                    } else {
                        accumulator.reset();
                    }
                }

                // On run-over, save the mode's best and record what the run earned
                // — both once — then resolve with a summary.
                if over && !*saved {
                    *saved = true;
                    let score = game.score();
                    match mode {
                        RunMode::Onslaught if score > *best => {
                            *best = score;
                            stepfall_storage::set_onslaught_best(score);
                        }
                        RunMode::Daily if score > *best => {
                            *best = score;
                            stepfall_storage::set_daily_best(*day, score);
                        }
                        _ => {}
                    }

                    // Record the run's outcome into the unlock set; save and call
                    // out anything newly earned.
                    let won = game.outcome() == Some(RunOutcome::Won);
                    let outcome = meta::Outcome {
                        won,
                        stages_cleared: game.stage(),
                        score,
                        tier: if *ascension { game.tier() } else { 0 },
                    };
                    let newly = unlocked.record(outcome);
                    if !newly.is_empty() {
                        stepfall_storage::set_unlocked_bits(unlocked.bits());
                        *earned = newly;
                    }

                    // Winning an Ascension run pushes the reached tier up a rung.
                    if *ascension && won {
                        let next = game.tier() + 1;
                        if next > stepfall_storage::ascension_tier() {
                            stepfall_storage::set_ascension_tier(next);
                        }
                    }
                }

                self.blit_shake = Vec2::from(fx.shake_offset());
                render::draw_remix(game, *best);
                fx.draw();
                if over {
                    render::remix_summary(game, *mode, *best, *ascension, earned);
                } else if *paused {
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

    /// Opens HAILFALL's menu on its first mode.
    fn open_remix_menu(&mut self) {
        self.screen = Screen::RemixSelect {
            highlight: MenuRow::Mode(RunMode::Sortie),
        };
    }

    /// Opens the collection on what this player has unlocked so far.
    fn open_collection(&mut self) {
        self.screen = Screen::Collection {
            unlocked: meta::Unlocked::from_bits(stepfall_storage::unlocked_bits()),
        };
    }

    /// Consumes the next seed, advancing it so the next game differs.
    fn take_seed(&mut self) -> u64 {
        let seed = self.next_seed;
        self.next_seed = self.next_seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        seed
    }

    fn start_match(&mut self) {
        let game = Box::new(Game::new(self.take_seed()));
        self.screen = Screen::Match {
            march_frame: game.march_frame(),
            march_note: 0,
            saucer_sounding: false,
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            paused: false,
        };
    }

    /// Starts a HAILFALL run in `mode`. A Sortie or Onslaught takes a fresh seed
    /// from the clock; a Daily takes the day's shared seed so everyone plays the
    /// same run, and both draw the best to beat from the save.
    fn start_remix_match(&mut self, mode: RunMode) {
        let (seed, day, best) = match mode {
            RunMode::Sortie => (self.take_seed(), 0, 0),
            RunMode::Onslaught => (self.take_seed(), 0, stepfall_storage::onslaught_best()),
            RunMode::Daily => {
                let day = today();
                (u64::from(day), day, stepfall_storage::daily_best(day))
            }
        };
        // The run flies whatever the player has earned — the core only ever sees
        // the loadout the meta builds; it never knows the word "unlock".
        let unlocked = meta::Unlocked::from_bits(stepfall_storage::unlocked_bits());
        let game = Box::new(RemixGame::new(seed, mode, unlocked.loadout()));
        self.screen = Screen::RemixMatch {
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            fx: Fx::default(),
            paused: false,
            mode,
            day,
            best,
            ascension: false,
            unlocked,
            earned: Vec::new(),
            saved: false,
        };
    }

    /// Starts an Ascension run at the reached tier — a Sortie escalated by the
    /// tier's modifiers. Winning it pushes the reached tier up.
    fn start_ascension_match(&mut self) {
        let tier = stepfall_storage::ascension_tier();
        let unlocked = meta::Unlocked::from_bits(stepfall_storage::unlocked_bits());
        let game = Box::new(RemixGame::new_ascension(
            self.take_seed(),
            tier,
            unlocked.loadout(),
        ));
        self.screen = Screen::RemixMatch {
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            fx: Fx::default(),
            paused: false,
            mode: RunMode::Sortie,
            day: 0,
            best: 0,
            ascension: true,
            unlocked,
            earned: Vec::new(),
            saved: false,
        };
    }
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

/// Reads HAILFALL's menu, cycling the highlight through its rows. Returns the
/// chosen row once the player commits to it.
fn remix_menu_input(highlight: &mut MenuRow) -> Option<MenuRow> {
    let rows = MenuRow::ROWS;
    let i = rows.iter().position(|r| r == highlight).unwrap_or(0);
    if pressed_menu_next() {
        *highlight = rows[(i + 1) % rows.len()];
    } else if pressed_menu_prev() {
        *highlight = rows[(i + rows.len() - 1) % rows.len()];
    }
    (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)).then_some(*highlight)
}

/// Whether the player nudged a menu highlight down/forward this frame.
fn pressed_menu_next() -> bool {
    is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S)
}

/// Whether the player nudged a menu highlight up/back this frame.
fn pressed_menu_prev() -> bool {
    is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W)
}

/// Today's calendar day, as whole days since the Unix epoch. The core stays
/// clock-free; only the shell reads the clock, so a Daily's seed is shared by
/// everyone playing on the same day.
fn today() -> u32 {
    (miniquad::date::now() / 86_400.0) as u32
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

/// A seed for a game. The core is deterministic by design, so the only
/// nondeterminism in the game is this one number, read from the clock.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
