//! The shell's front-end: the mode-select in front of a game and the flow
//! between it and play. Everything here is window, input and rendering glue
//! around the pure core, which is why it lives in the shell, not `stepfall_core`.

use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;
use stepfall_core::{Game, Phase, TIMESTEP};
use stepfall_remix_core::{Game as RemixGame, Loadout, Mode as RunMode, Phase as RemixPhase};

use crate::{Audio, read_input, read_remix_input, render};

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make
/// the game try to catch up by simulating seconds at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The two takes every Game in the Collection ships. STEPFALL's Remix is not
/// built yet, so only the Faithful is playable — the Remix shows as locked.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The faithful recreation.
    Faithful,
    /// The reimagined version — coming later.
    Remix,
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
    /// HAILFALL's mode picker — the Remix chooses Sortie, Onslaught or Daily.
    RemixSelect { highlight: RunMode },
    /// A HAILFALL run in progress — the Remix. Boxed like the Faithful's game.
    RemixMatch {
        game: Box<RemixGame>,
        accumulator: Accumulator,
        paused: bool,
        /// Which mode this run is.
        mode: RunMode,
        /// The calendar day a Daily run belongs to (its saved key; 0 otherwise).
        day: u32,
        /// The best score to beat and show for this mode (0 for Sortie).
        best: u32,
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
            audio,
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
                        // The Remix picks its mode first.
                        Mode::Remix => {
                            self.screen = Screen::RemixSelect {
                                highlight: RunMode::Sortie,
                            };
                        }
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
                if let Some(mode) = remix_select_input(highlight) {
                    self.start_remix_match(mode);
                } else {
                    render::remix_select(*highlight);
                }
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
                paused,
                mode,
                day,
                best,
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

                let over = game.phase() == RemixPhase::Over;
                if !over {
                    if is_key_pressed(KeyCode::P) {
                        *paused = !*paused;
                    }
                    if !*paused {
                        let input = read_remix_input();
                        for _ in 0..accumulator.steps(get_frame_time()) {
                            game.step(input);
                        }
                    } else {
                        accumulator.reset();
                    }
                }

                // On run-over, save the mode's best once, then resolve with a summary.
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
                }

                render::draw_remix(game, *best);
                if over {
                    render::remix_summary(game, *mode, *best);
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
        let game = Box::new(RemixGame::new(seed, mode, Loadout::default()));
        self.screen = Screen::RemixMatch {
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            mode,
            day,
            best,
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

/// Reads HAILFALL's mode picker, cycling the highlight through the three modes.
/// Returns the chosen mode once the player commits to it.
fn remix_select_input(highlight: &mut RunMode) -> Option<RunMode> {
    if pressed_menu_next() {
        *highlight = match *highlight {
            RunMode::Sortie => RunMode::Onslaught,
            RunMode::Onslaught => RunMode::Daily,
            RunMode::Daily => RunMode::Sortie,
        };
    } else if pressed_menu_prev() {
        *highlight = match *highlight {
            RunMode::Sortie => RunMode::Daily,
            RunMode::Onslaught => RunMode::Sortie,
            RunMode::Daily => RunMode::Onslaught,
        };
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
