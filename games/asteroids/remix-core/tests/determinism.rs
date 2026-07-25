//! The run replays: the same seed and inputs always produce the same run — via the
//! seam. (There is no seeded randomness yet, so this is the seam's contract; it grows
//! teeth as spawns and fire arrive.)

mod common;

use asteroids_remix_core::{Game, Input, Loadout, Mode};

fn scripted(i: usize) -> Input {
    Input {
        turn_left: i.is_multiple_of(7),
        turn_right: i.is_multiple_of(3),
        thrust: i.is_multiple_of(2),
        ..Default::default()
    }
}

#[test]
fn same_seed_and_inputs_replay_identically() {
    let mut a = Game::new(42, Mode::Orbit, Loadout::default());
    let mut b = Game::new(42, Mode::Orbit, Loadout::default());
    for i in 0..2000 {
        a.step(scripted(i));
        b.step(scripted(i));
    }
    assert_eq!(
        a.ship(),
        b.ship(),
        "the same seed and inputs replay exactly"
    );
}

#[test]
fn restart_replays_from_the_same_seed() {
    let mut game = Game::new(99, Mode::Orbit, Loadout::default());
    for i in 0..1000 {
        game.step(scripted(i));
    }
    let first = game.ship();

    game.restart();
    for i in 0..1000 {
        game.step(scripted(i));
    }
    assert_eq!(game.ship(), first, "a restart replays the very same run");
}
