//! The core is deterministic: the same seed and inputs replay the same game.

use invaders_core::{Game, Input, Move};

type Snapshot = (f32, f32, f32, u32);

fn replay(seed: u64) -> Vec<Snapshot> {
    let mut game = Game::new(seed);
    let mut samples = Vec::new();
    for step in 0..20_000 {
        let cannon = match (step / 47) % 3 {
            0 => Move::Left,
            1 => Move::Right,
            _ => Move::Hold,
        };
        game.step(Input { cannon });
        if step % 250 == 0 {
            let leader = game.invaders().next().expect("the formation still stands");
            samples.push((game.cannon().x, leader.x, leader.y, game.alive()));
        }
    }
    samples
}

#[test]
fn the_same_seed_and_inputs_replay_the_same_game() {
    assert_eq!(replay(20_260_724), replay(20_260_724));
}

#[test]
fn a_restart_replays_the_game_from_the_start() {
    let mut game = Game::new(7);
    let opening = (game.cannon().x, game.alive());

    for _ in 0..3_000 {
        game.step(Input {
            cannon: Move::Right,
        });
    }
    assert_ne!(game.cannon().x, opening.0, "the game moved on");

    game.restart();
    assert_eq!(
        (game.cannon().x, game.alive()),
        opening,
        "a restart puts the game back as it began"
    );
}
