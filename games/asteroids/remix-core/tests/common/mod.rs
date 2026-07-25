//! Helpers shared by ACCRETE's core tests. Everything drives the run the way a
//! player does — through the public seam.

#![allow(dead_code)]

use asteroids_remix_core::{Game, Input, Loadout, Mode, Ship};

/// A run on `seed`, in the default mode with an empty loadout.
pub fn game(seed: u64) -> Game {
    Game::new(seed, Mode::Orbit, Loadout::default())
}

/// Doing nothing.
pub fn still() -> Input {
    Input::default()
}

/// Holding thrust, nothing else.
pub fn thrust() -> Input {
    Input {
        thrust: true,
        ..Default::default()
    }
}

/// Turning clockwise (to the ship's right).
pub fn turn_right() -> Input {
    Input {
        turn_right: true,
        ..Default::default()
    }
}

/// Turning anticlockwise (to the ship's left).
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
