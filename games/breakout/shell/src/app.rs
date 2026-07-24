//! The shell's front-end: the mode-select in front of a match and the flow
//! between it and play. Everything here is window, input and rendering glue
//! around the pure core, which is why it lives in the shell, not `breakout_core`.

use breakout_core::{Game, TIMESTEP};
use breakout_remix_core::{
    Game as RiftGame, Phase as RiftPhase, Pool as RiftPool, TIMESTEP as RIFT_TIMESTEP,
};
use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;

use crate::audio::Audio;
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
}

impl RiftMode {
    /// The next mode in the menu (the menu wraps).
    fn next(self) -> Self {
        match self {
            RiftMode::Run => RiftMode::Daily,
            RiftMode::Daily => RiftMode::Run,
        }
    }

    /// A short name for the summary card and the menu.
    pub fn label(self) -> &'static str {
        match self {
            RiftMode::Run => "RUN",
            RiftMode::Daily => "DAILY",
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
    /// RIFT's mode menu: Run or Daily.
    RiftMenu { highlight: RiftMode },
    /// A RIFT run in progress.
    Rift {
        game: RiftGame,
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
    },
}

/// The whole shell: the current screen, the seed source for new matches, the
/// sounds, and whether the window is fullscreen.
pub struct App {
    screen: Screen,
    next_seed: u64,
    audio: Audio,
    fullscreen: bool,
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
        }
    }

    /// Advances the shell by one real frame: reads input, runs whatever the
    /// current screen does, and draws it to the logical canvas.
    pub fn frame(&mut self) {
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
                    self.start_run(chosen);
                } else {
                    render::rift_menu(chosen);
                }
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

                if game.phase() == RiftPhase::Drafting {
                    // A draft is a menu, not real-time: one input per frame, off
                    // the accumulator, so a held key never repeats.
                    accumulator.reset();
                    self.audio.play_rift(game.step(rift::read_draft_input()));
                } else if !*paused {
                    let input = rift::read_play_input();
                    for _ in 0..accumulator.steps(get_frame_time()) {
                        let events = game.step(input);
                        self.audio.play_rift(events);
                        // A run that just ended may have set a new best for its
                        // mode.
                        if events.won || events.lost {
                            let depth = game.depth();
                            if depth > *best {
                                *best = depth;
                                match mode {
                                    RiftMode::Run => breakout_storage::set_best_depth(depth),
                                    RiftMode::Daily => {
                                        breakout_storage::set_daily_best(*day, depth)
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Don't let paused wall-time pile up and fast-forward on resume.
                    accumulator.reset();
                }

                rift::draw(game);
                match game.phase() {
                    RiftPhase::Drafting => rift::draw_draft(game),
                    RiftPhase::Won | RiftPhase::Lost => {
                        rift::run_summary(game, *best, mode.label())
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
            highlight: RiftMode::Run,
        };
    }

    /// Starts a RIFT run in the chosen mode. A Run takes a fresh seed and the
    /// saved Run best; a Daily takes the day's shared seed and the day's best.
    fn start_run(&mut self, mode: RiftMode) {
        let (seed, day, best) = match mode {
            RiftMode::Run => (self.take_seed(), 0, breakout_storage::best_depth()),
            RiftMode::Daily => {
                let day = today();
                (u64::from(day), day, breakout_storage::daily_best(day))
            }
        };
        let game = RiftGame::new_run(seed, &RiftPool::base());
        self.screen = Screen::Rift {
            game,
            accumulator: Accumulator::new(RIFT_TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            mode,
            day,
            best,
        };
    }
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
