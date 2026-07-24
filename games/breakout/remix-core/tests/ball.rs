//! Ball behaviour against the walls, and wall-bounce reporting — via the seam.

mod common;

use breakout_remix_core::{BALL_SIZE, Input, LOGICAL_WIDTH};
use common::run;

const LONG_RUN: usize = 8_000;

#[test]
fn the_ball_stays_within_the_side_and_top_walls() {
    let mut game = run(7);
    let half = BALL_SIZE / 2.0;
    for step in 0..LONG_RUN {
        game.step(Input::default());
        let ball = game.ball();
        assert!(
            ball.x - half >= -0.01 && ball.x + half <= LOGICAL_WIDTH + 0.01,
            "step {step}: ball escaped horizontally at x = {}",
            ball.x
        );
        assert!(
            ball.y - half >= -0.01,
            "step {step}: ball escaped through the top at y = {}",
            ball.y
        );
    }
}

#[test]
fn a_wall_bounce_is_reported_to_the_shell() {
    let mut game = run(7);
    let bounces = (0..LONG_RUN)
        .filter(|_| game.step(Input::default()).wall_bounce)
        .count();
    assert!(bounces > 0, "no wall bounce was ever reported");
}
