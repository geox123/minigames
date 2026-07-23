//! The shell's front-end: the screens in front of a match and the flow between
//! them. Everything here is window, input and rendering glue around the pure
//! core, which is why it lives in the shell rather than in `pong_core`.

use macroquad::prelude::*;
use pong_core::{Game, Players, TIMESTEP};

use crate::render;

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make
/// the game try to catch up by simulating minutes at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The two takes every Game in the Collection ships. Pong's Remix has not been
/// built yet, so it is shown but locked.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The faithful recreation — playable.
    Faithful,
    /// The reimagining — not built yet.
    Remix,
}

/// Which screen the player is looking at.
enum Screen {
    /// The Collection's two-takes screen: Faithful (playable) or Remix (locked).
    ModeSelect { highlight: Mode },
    /// Choosing one or two players before a Faithful match.
    PlayerSelect { highlight: Players },
    /// A match in progress.
    Playing {
        game: Game,
        /// Left-over real time not yet folded into a fixed step.
        accumulator: f32,
    },
}

/// The whole shell: the current screen and the seed source for new matches.
pub struct App {
    screen: Screen,
    next_seed: u64,
}

impl App {
    /// Opens the shell on the mode-select screen.
    pub fn new() -> Self {
        Self {
            screen: Screen::ModeSelect {
                highlight: Mode::Faithful,
            },
            next_seed: seed_from_clock(),
        }
    }

    /// Advances the shell by one real frame: reads input, runs whatever the
    /// current screen does, and draws it to the logical canvas.
    pub fn frame(&mut self) {
        match &mut self.screen {
            Screen::ModeSelect { highlight } => {
                if mode_select_input(highlight) {
                    // Remix is locked, so choosing it does nothing yet.
                    if *highlight == Mode::Faithful {
                        self.screen = Screen::PlayerSelect {
                            highlight: Players::Two,
                        };
                    }
                } else {
                    render::mode_select(*highlight);
                }
            }
            Screen::PlayerSelect { highlight } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                } else if let Some(chosen) = player_select_input(highlight) {
                    self.start_match(chosen);
                } else {
                    render::player_select(*highlight);
                }
            }
            Screen::Playing { game, accumulator } => {
                // Backing out of a match returns to the Collection's mode-select.
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                if is_key_pressed(KeyCode::R) {
                    game.restart();
                }

                let input = crate::read_input();
                *accumulator = (*accumulator + get_frame_time()).min(MAX_FRAME_TIME);
                while *accumulator >= TIMESTEP {
                    game.step(input);
                    *accumulator -= TIMESTEP;
                }
                render::draw(game);
            }
        }
    }

    fn return_to_mode_select(&mut self) {
        self.screen = Screen::ModeSelect {
            highlight: Mode::Faithful,
        };
    }

    fn start_match(&mut self, players: Players) {
        let game = Game::new(players, self.next_seed);
        // Move the seed on, so the next match does not replay this one.
        self.next_seed = self.next_seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        self.screen = Screen::Playing {
            game,
            accumulator: 0.0,
        };
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Reads the mode-select screen, moving the highlight between the two takes.
/// Returns whether the player committed to the highlighted one.
fn mode_select_input(highlight: &mut Mode) -> bool {
    if pressed_vertical() {
        *highlight = match *highlight {
            Mode::Faithful => Mode::Remix,
            Mode::Remix => Mode::Faithful,
        };
    }
    is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)
}

/// Reads the player-select screen. Returns the chosen player count once the
/// player commits, or `None` while they are still choosing.
fn player_select_input(highlight: &mut Players) -> Option<Players> {
    if pressed_vertical() {
        *highlight = match *highlight {
            Players::One => Players::Two,
            Players::Two => Players::One,
        };
    }
    if is_key_pressed(KeyCode::Key1) {
        *highlight = Players::One;
    }
    if is_key_pressed(KeyCode::Key2) {
        *highlight = Players::Two;
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
        return Some(*highlight);
    }
    None
}

/// Whether the player nudged a menu highlight this frame.
fn pressed_vertical() -> bool {
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
