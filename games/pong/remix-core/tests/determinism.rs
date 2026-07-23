//! PULSE's core is deterministic: the same seed and inputs replay the same game.

use pong_remix_core::{Axis, Game, Input, Side};

type Snapshot = (f32, f32, f32, f32, u32, u32);

fn replay(seed: u64) -> Vec<Snapshot> {
    let mut game = Game::new(seed);
    let mut samples = Vec::new();
    for step in 0..20_000 {
        let input = Input {
            left: match (step / 37) % 3 {
                0 => Axis::Up,
                1 => Axis::Down,
                _ => Axis::Hold,
            },
            right: match (step / 53) % 3 {
                0 => Axis::Down,
                1 => Axis::Hold,
                _ => Axis::Up,
            },
            ..Default::default()
        };
        game.step(input);
        if step % 250 == 0 {
            let ball = game.ball();
            samples.push((
                ball.x,
                ball.y,
                game.paddle(Side::Left).y,
                game.paddle(Side::Right).y,
                game.score(Side::Left),
                game.score(Side::Right),
            ));
        }
    }
    samples
}

#[test]
fn the_same_seed_and_inputs_replay_the_same_game() {
    assert_eq!(replay(20_260_723), replay(20_260_723));
}

#[test]
fn a_different_seed_plays_out_differently() {
    assert_ne!(replay(1), replay(2));
}
