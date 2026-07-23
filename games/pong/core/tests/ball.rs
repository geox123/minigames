//! Ball behaviour, driven through the core's public seam only.

use pong_core::{BALL_SIZE, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH};

/// Long enough for the ball to reach every wall several times over.
const LONG_RUN: usize = 6_000;

#[test]
fn ball_never_leaves_the_play_field() {
    let mut game = Game::new(7);

    for step in 0..LONG_RUN {
        game.step();
        let ball = game.ball();
        let half = BALL_SIZE / 2.0;
        assert!(
            ball.y - half >= -0.001 && ball.y + half <= LOGICAL_HEIGHT + 0.001,
            "step {step}: ball escaped vertically at y = {}",
            ball.y
        );
        assert!(
            ball.x - half >= -0.001 && ball.x + half <= LOGICAL_WIDTH + 0.001,
            "step {step}: ball escaped horizontally at x = {}",
            ball.x
        );
    }
}

#[test]
fn ball_reverses_direction_when_it_meets_a_wall() {
    let mut game = Game::new(7);
    let start = game.ball();
    let (mut flipped_vertically, mut flipped_horizontally) = (false, false);

    for _ in 0..LONG_RUN {
        game.step();
        let ball = game.ball();
        flipped_vertically |= ball.vy.signum() != start.vy.signum();
        flipped_horizontally |= ball.vx.signum() != start.vx.signum();
    }

    assert!(
        flipped_vertically,
        "ball never bounced off a horizontal wall"
    );
    assert!(
        flipped_horizontally,
        "ball never bounced off a vertical wall"
    );
}

#[test]
fn a_wall_bounce_is_reported_to_the_shell() {
    let mut game = Game::new(7);

    let bounces = (0..LONG_RUN).filter(|_| game.step().wall_bounce).count();

    assert!(bounces > 0, "no wall bounce was ever reported");
}
