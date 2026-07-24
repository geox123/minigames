//! The core is deterministic: the same seed and inputs replay the same game.

use stepfall_core::{Game, INVADERS, Input, Move};

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
        // Never fire, so the formation stays whole to sample its leader.
        game.step(Input {
            cannon,
            fire: false,
        });
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
    let opening_cannon = game.cannon().x;
    let opening_leader = game.invaders().next().unwrap();
    let leader_at = |g: &Game| {
        let l = g.invaders().next().unwrap();
        (l.x, l.y)
    };

    for _ in 0..3_000 {
        game.step(Input {
            cannon: Move::Right,
            fire: false,
        });
    }
    // The formation always marches, so it is the clean witness that the game
    // moved on (the cannon may have died and returned to the middle).
    assert_ne!(
        leader_at(&game),
        (opening_leader.x, opening_leader.y),
        "the game moved on"
    );

    game.restart();
    assert_eq!(
        game.cannon().x,
        opening_cannon,
        "the cannon is back at the start"
    );
    assert_eq!(
        leader_at(&game),
        (opening_leader.x, opening_leader.y),
        "the formation is back at the start"
    );
    assert_eq!(game.alive(), INVADERS as u32);
}
