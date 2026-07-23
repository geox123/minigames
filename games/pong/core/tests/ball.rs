//! How the ball behaves against the top and bottom of the play field.

mod common;

use common::tracking;
use pong_core::{BALL_SIZE, Game, LOGICAL_HEIGHT, Players};

/// A hundred seconds of play — many points, many rallies.
const LONG_RUN: usize = 12_000;

#[test]
fn the_ball_never_leaves_the_field_through_the_top_or_bottom() {
    let mut game = Game::new(Players::Two, 7);

    for step in 0..LONG_RUN {
        let input = tracking(&game, 0.0, 0.0);
        game.step(input);

        let ball = game.ball();
        let half = BALL_SIZE / 2.0;
        assert!(
            ball.y - half >= -0.001 && ball.y + half <= LOGICAL_HEIGHT + 0.001,
            "step {step}: the ball escaped the field at y = {}",
            ball.y
        );
    }
}

#[test]
fn the_ball_rebounds_off_the_top_and_bottom_without_losing_its_way() {
    let mut game = Game::new(Players::Two, 7);
    let mut rebounds = 0;

    for _ in 0..LONG_RUN {
        let before = game.ball();
        // Each player aims for the far edge of their paddle, which drives the
        // ball into the walls rather than flat down the middle.
        let input = tracking(&game, 12.0, -12.0);
        let events = game.step(input);

        if events.wall_bounce && !events.paddle_hit {
            let after = game.ball();
            assert_eq!(
                before.vy.signum(),
                -after.vy.signum(),
                "a wall should send the ball back the other way"
            );
            assert_eq!(
                before.vx.signum(),
                after.vx.signum(),
                "a wall should not turn the ball back down the field"
            );
            rebounds += 1;
        }
    }

    assert!(
        rebounds > 0,
        "the ball never reached the top or bottom of the field"
    );
}
