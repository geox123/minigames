//! The GNASH shell's front-end: it boots straight into the Faithful and wraps the
//! pure core in the Collection's standard pause / restart / fullscreen chrome. The
//! mode-select, the hunters' rendering and the sound arrive in later tickets; this
//! is the tracer bullet that puts the maze and the eater on a real window.

use gnash_core::{Game, TIMESTEP};
use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;

use crate::{read_input, render};

/// How much real time a single frame may contribute to the simulation. Without this
/// cap, one long stall (a dragged window, a backgrounded tab) would make the game try
/// to catch up by simulating seconds at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The whole shell: the game in play, the fixed-timestep accumulator banking real
/// time into 60 Hz steps, and whether it is paused or fullscreen.
pub struct App {
    game: Game,
    accumulator: Accumulator,
    paused: bool,
    fullscreen: bool,
}

impl App {
    /// Opens the shell on a fresh game, seeded from the clock (the core's only
    /// nondeterminism).
    pub fn new() -> Self {
        Self {
            game: Game::new(seed_from_clock()),
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            fullscreen: false,
        }
    }

    /// One real frame: honour the chrome keys, advance the game by the whole fixed
    /// steps now due, and draw.
    pub fn frame(&mut self) {
        if is_key_pressed(KeyCode::F) {
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
        }
        if is_key_pressed(KeyCode::R) {
            self.game.restart();
            self.paused = false;
            self.accumulator.reset();
        }
        if is_key_pressed(KeyCode::P) {
            self.paused = !self.paused;
        }

        if self.paused {
            // Don't let paused wall-time pile up and fast-forward on resume.
            self.accumulator.reset();
        } else {
            let input = read_input();
            for _ in 0..self.accumulator.steps(get_frame_time()) {
                self.game.step(input);
            }
        }

        render::draw(&self.game, self.paused);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// A seed for a game, read from the clock — the core is deterministic, so this one
/// number is the only nondeterminism in a run.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
