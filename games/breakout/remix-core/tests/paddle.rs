//! The paddle: it moves between the walls and returns the ball upward — via the
//! seam.

mod common;

use breakout_remix_core::{Input, LOGICAL_WIDTH, Move};
use common::{MAX_STEPS, run, track};

#[test]
fn the_paddle_stays_between_the_walls() {
    let mut game = run(1);
    // Hold left long enough to reach the wall and press into it.
    for _ in 0..2_000 {
        game.step(Input {
            paddle: Move::Left,
            ..Default::default()
        });
    }
    let p = game.paddle();
    assert!(
        p.x >= -0.01,
        "paddle pushed past the left wall at x = {}",
        p.x
    );

    // Then hold right all the way across.
    for _ in 0..4_000 {
        game.step(Input {
            paddle: Move::Right,
            ..Default::default()
        });
    }
    let p = game.paddle();
    assert!(
        p.x + p.width <= LOGICAL_WIDTH + 0.01,
        "paddle pushed past the right wall to x = {}",
        p.x
    );
}

#[test]
fn the_paddle_returns_the_ball_upward() {
    let mut game = run(9);
    for _ in 0..MAX_STEPS {
        let paddle = track(&game, 0.0);
        if game
            .step(Input {
                paddle,
                ..Default::default()
            })
            .paddle_hit
        {
            assert!(
                game.ball().vy < 0.0,
                "after a paddle hit the ball should head up, got vy = {}",
                game.ball().vy
            );
            return;
        }
    }
    panic!("the paddle never returned the ball");
}
