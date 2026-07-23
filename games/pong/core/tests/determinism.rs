//! The core is deterministic: the same seed and the same inputs always replay
//! the same game, which is what makes any of the other tests meaningful.

use pong_core::{Axis, Game, Input, Players, Side};

/// A snapshot of everything a player can see.
type Snapshot = (f32, f32, f32, f32, u32, u32);

/// Plays a long match against a fixed, varied input script and samples the
/// game as it goes.
fn replay(seed: u64) -> Vec<Snapshot> {
    let mut game = Game::new(Players::Two, seed);
    let mut samples = Vec::new();

    for step in 0..20_000 {
        // Nothing clever, just an input pattern with no common period.
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
