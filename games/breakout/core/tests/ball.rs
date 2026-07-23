//! Ball behaviour in the walking skeleton, driven through the core's seam only.
//! (The bottom is a wall for now; a later slice turns it into the paddle line.)

use breakout_core::{BALL_SIZE, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH};

/// Long enough for the ball to reach every wall several times over.
const LONG_RUN: usize = 8_000;

#[test]
fn the_ball_never_leaves_the_field() {
    let mut game = Game::new(7);
    let half = BALL_SIZE / 2.0;
    for step in 0..LONG_RUN {
        game.step();
        let ball = game.ball();
        assert!(
            ball.x - half >= -0.01 && ball.x + half <= LOGICAL_WIDTH + 0.01,
            "step {step}: ball escaped horizontally at x = {}",
            ball.x
        );
        assert!(
            ball.y - half >= -0.01 && ball.y + half <= LOGICAL_HEIGHT + 0.01,
            "step {step}: ball escaped vertically at y = {}",
            ball.y
        );
    }
}

#[test]
fn the_ball_rebounds_off_every_wall() {
    let mut game = Game::new(7);
    let start = game.ball();
    let (mut flipped_x, mut flipped_y) = (false, false);
    for _ in 0..LONG_RUN {
        game.step();
        let ball = game.ball();
        flipped_x |= ball.vx.signum() != start.vx.signum();
        flipped_y |= ball.vy.signum() != start.vy.signum();
    }
    assert!(flipped_x, "the ball never bounced off a side wall");
    assert!(flipped_y, "the ball never bounced off the top or bottom");
}

#[test]
fn a_wall_bounce_is_reported_to_the_shell() {
    let mut game = Game::new(7);
    let bounces = (0..LONG_RUN).filter(|_| game.step().wall_bounce).count();
    assert!(bounces > 0, "no wall bounce was ever reported");
}

#[test]
fn the_same_seed_replays_the_same_ball() {
    let replay = |seed| {
        let mut game = Game::new(seed);
        let mut path = Vec::new();
        for step in 0..2_000 {
            game.step();
            if step % 50 == 0 {
                let ball = game.ball();
                path.push((ball.x, ball.y));
            }
        }
        path
    };
    assert_eq!(replay(20_260_723), replay(20_260_723));
    assert_ne!(replay(1), replay(2));
}
