//! Helpers shared by RIFT's core tests. Everything drives the run the way a
//! player does — through the public seam, by holding keys.

#![allow(dead_code)]

use breakout_remix_core::{Game, Input, Move, Pool};

/// A generous ceiling on how long a test plays before giving up.
pub const MAX_STEPS: usize = 40_000;

/// A base-pool run on `seed`.
pub fn run(seed: u64) -> Game {
    Game::new_run(seed, &Pool::base())
}

/// The key that brings the paddle's centre toward the ball, `offset` aside.
pub fn track(game: &Game, offset: f32) -> Move {
    let ball = game.ball();
    let centre = game.paddle().x + game.paddle().width / 2.0;
    let target = ball.x + offset;
    if centre < target - 2.0 {
        Move::Right
    } else if centre > target + 2.0 {
        Move::Left
    } else {
        Move::Hold
    }
}

/// Plays a defended rally until `stop` fires or the budget runs out, the paddle
/// tracking the ball to keep it alive.
pub fn rally_until(game: &mut Game, mut stop: impl FnMut(&Game) -> bool) {
    for _ in 0..MAX_STEPS {
        if stop(game) {
            return;
        }
        let paddle = track(game, 0.0);
        game.step(Input {
            paddle,
            ..Default::default()
        });
    }
}
