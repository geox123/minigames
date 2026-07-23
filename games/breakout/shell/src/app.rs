//! The shell's front-end: the mode-select in front of a match and the flow
//! between it and play. Everything here is window, input and rendering glue
//! around the pure core, which is why it lives in the shell, not `breakout_core`.

use breakout_core::{Game, TIMESTEP};
use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;

use crate::audio::Audio;
use crate::{read_input, render};

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make
/// the game try to catch up by simulating seconds at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The two takes every Game in the Collection ships. Breakout's Remix is not
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
    /// A Faithful match in progress.
    Match {
        game: Game,
        /// Left-over real time not yet folded into a fixed step.
        accumulator: Accumulator,
        /// Whether the match is paused.
        paused: bool,
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
                    // Only the Faithful is playable; committing to the locked
                    // Remix does nothing.
                    if *highlight == Mode::Faithful {
                        self.start_match();
                    }
                } else {
                    render::mode_select(*highlight);
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
