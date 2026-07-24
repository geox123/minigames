//! Helpers shared by the Space Invaders core's tests. Everything drives the game
//! the way a player does — through the public seam.

#![allow(dead_code)]

use stepfall_core::{Game, Input, Move};

/// A generous ceiling on how long a test plays before giving up.
pub const MAX_STEPS: usize = 200_000;

/// A game on `seed`.
pub fn game(seed: u64) -> Game {
    Game::new(seed)
}

/// Input holding the cannon still.
pub fn still() -> Input {
    Input::default()
}

/// Input pushing the cannon one way.
pub fn push(cannon: Move) -> Input {
    Input { cannon }
}

/// The topmost edge of the formation.
pub fn formation_top(game: &Game) -> f32 {
    game.invaders().map(|i| i.y).fold(f32::INFINITY, f32::min)
}

/// The rightmost edge of the formation.
pub fn formation_right(game: &Game) -> f32 {
    game.invaders()
        .map(|i| i.x)
        .fold(f32::NEG_INFINITY, f32::max)
}
