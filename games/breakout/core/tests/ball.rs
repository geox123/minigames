//! Ball behaviour against the walls, and determinism — through the seam only.

use breakout_core::{BALL_SIZE, Game, Input, LOGICAL_WIDTH, Phase};

const LONG_RUN: usize = 8_000;

#[test]
fn the_ball_stays_within_the_side_and_top_walls() {
    let mut game = Game::new(7);
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
    let mut game = Game::new(7);
    let bounces = (0..LONG_RUN)
        .filter(|_| game.step(Input::default()).wall_bounce)
        .count();
    assert!(bounces > 0, "no wall bounce was ever reported");
}

#[test]
fn the_same_seed_replays_the_same_game() {
    let replay = |seed| {
        let mut game = Game::new(seed);
        let mut path = Vec::new();
        for step in 0..3_000 {
            if game.phase() == Phase::GameOver {
                break;
            }
            game.step(Input::default());
            if step % 20 == 0 {
                let ball = game.ball();
                path.push((ball.x, ball.y, ball.vx, ball.vy));
            }
        }
        path
    };
    assert_eq!(replay(20_260_723), replay(20_260_723));
    assert_ne!(replay(1), replay(2));
}
