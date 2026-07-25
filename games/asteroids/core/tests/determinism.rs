//! The core replays: the same seed and inputs always produce the same game — via
//! the seam.

mod common;

use asteroids_core::{Asteroid, Game, Input, Ship};

/// A varied but fixed input for step `i` — turning, thrusting and idling in a
/// repeating pattern, so replays exercise real motion rather than a still ship.
fn scripted(i: usize) -> Input {
    Input {
        turn_left: i.is_multiple_of(7),
        turn_right: i.is_multiple_of(3),
        thrust: i.is_multiple_of(2),
        ..Default::default()
    }
}

fn snapshot(game: &Game) -> (Ship, Vec<Asteroid>) {
    (game.ship(), game.asteroids().collect())
}

#[test]
fn same_seed_and_inputs_replay_identically() {
    let mut a = Game::new(42);
    let mut b = Game::new(42);
    for i in 0..2000 {
        a.step(scripted(i));
        b.step(scripted(i));
    }
    assert_eq!(
        snapshot(&a),
        snapshot(&b),
        "identical seed and inputs replay exactly"
    );
}

#[test]
fn restart_replays_from_the_same_seed() {
    let mut game = Game::new(99);
    for i in 0..1000 {
        game.step(scripted(i));
    }
    let first = snapshot(&game);

    game.restart();
    for i in 0..1000 {
        game.step(scripted(i));
    }
    assert_eq!(
        snapshot(&game),
        first,
        "a restart replays the very same game"
    );
}

#[test]
fn different_seeds_lay_out_different_fields() {
    let one: Vec<Asteroid> = Game::new(1).asteroids().collect();
    let two: Vec<Asteroid> = Game::new(2).asteroids().collect();
    assert_ne!(one, two, "different seeds place the rocks differently");
}
