//! RIFT's core is deterministic: the same seed and inputs replay the same run.

use breakout_remix_core::{Game, Input, Move, Pool};

type Snapshot = (f32, f32, f32, f32, u32, u32);

fn replay(seed: u64) -> Vec<Snapshot> {
    let mut game = Game::new_run(seed, &Pool::base());
    let mut samples = Vec::new();
    for step in 0..20_000 {
        let paddle = match (step / 41) % 3 {
            0 => Move::Left,
            1 => Move::Right,
            _ => Move::Hold,
        };
        game.step(Input { paddle });
        if step % 250 == 0 {
            let ball = game.ball();
            let p = game.paddle();
            samples.push((ball.x, ball.y, ball.vx, p.x, game.score(), game.depth()));
        }
    }
    samples
}

#[test]
fn the_same_seed_and_inputs_replay_the_same_run() {
    assert_eq!(replay(20_260_723), replay(20_260_723));
}

#[test]
fn a_different_seed_plays_out_differently() {
    assert_ne!(replay(1), replay(2));
}
