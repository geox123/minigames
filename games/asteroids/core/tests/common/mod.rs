//! Helpers shared by Asteroids' core tests. Everything drives the game the way a
//! player does — through the public seam.

#![allow(dead_code)]

use asteroids_core::{Game, Input, Ship};

/// A generous ceiling on how long a test plays before giving up.
pub const MAX_STEPS: usize = 200_000;

/// A game on `seed`.
pub fn game(seed: u64) -> Game {
    Game::new(seed)
}

/// Doing nothing: no turn, no thrust.
pub fn still() -> Input {
    Input::default()
}

/// Holding thrust, not turning.
pub fn thrust() -> Input {
    Input {
        thrust: true,
        ..Default::default()
    }
}

/// Turning clockwise (to the ship's right), not thrusting.
pub fn turn_right() -> Input {
    Input {
        turn_right: true,
        ..Default::default()
    }
}

/// Turning anticlockwise (to the ship's left), not thrusting.
pub fn turn_left() -> Input {
    Input {
        turn_left: true,
        ..Default::default()
    }
}

/// The ship's current speed, in units per second.
pub fn speed(ship: Ship) -> f32 {
    (ship.vx * ship.vx + ship.vy * ship.vy).sqrt()
}
