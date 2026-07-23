//! Multiball: the core carries several balls at once, a seeded pickup splits an
//! extra one in, and any ball leaving a goal scores.

mod common;

use common::tracking;
use pong_remix_core::{Game, Input, Phase, PickupKind, Side};

/// Plays a centred rally until more than one ball is live, returning the game at
/// that moment. Deterministic: a fixed seed always collects a multiball here.
fn play_until_multiball(seed: u64) -> Game {
    let mut game = Game::new(seed);
    for _ in 0..common::MAX_STEPS {
        let input = tracking(&game, 0.0, 0.0);
        game.step(input);
        if game.balls().count() > 1 {
            return game;
        }
    }
    panic!("no multiball was collected within the step budget");
}

#[test]
fn a_pickup_spawns_during_a_rally() {
    let mut game = Game::new(7);
    let mut seen = false;
    for _ in 0..common::MAX_STEPS {
        let input = tracking(&game, 0.0, 0.0);
        game.step(input);
        if let Some(pickup) = game.pickup() {
            assert_eq!(pickup.kind, PickupKind::Multiball);
            seen = true;
            break;
        }
    }
    assert!(seen, "no pickup ever spawned during a rally");
}

#[test]
fn collecting_a_multiball_adds_a_ball() {
    let game = play_until_multiball(7);
    assert!(
        game.balls().count() >= 2,
        "collecting a multiball should put more than one ball in play"
    );
}

#[test]
fn several_balls_can_be_in_play_at_once() {
    // The very act of reaching two balls proves the core simulates more than one.
    let game = play_until_multiball(7);
    let count = game.balls().count();
    assert!(count >= 2, "expected multiple balls, saw {count}");
}

#[test]
fn every_ball_that_exits_scores() {
    let mut game = play_until_multiball(7);
    let balls_in_play = game.balls().count();
    let total_before = game.score(Side::Left) + game.score(Side::Right);

    // Both players abandon their goals, so every live ball runs out and scores.
    // The rally is over once all its balls have left and a fresh serve begins.
    for _ in 0..common::MAX_STEPS {
        game.step(Input {
            left: pong_remix_core::Axis::Down,
            right: pong_remix_core::Axis::Down,
            ..Default::default()
        });
        if game.phase() == Phase::Serving
            && game.score(Side::Left) + game.score(Side::Right) > total_before
        {
            break;
        }
    }

    let scored = game.score(Side::Left) + game.score(Side::Right) - total_before;
    assert!(
        scored >= balls_in_play as u32,
        "each of the {balls_in_play} balls should have scored as it left; only {scored} did"
    );
}

#[test]
fn pickup_spawns_are_deterministic() {
    // Two games from the same seed, driven identically, agree on when and where
    // the pickup appears.
    let mut a = Game::new(99);
    let mut b = Game::new(99);
    for _ in 0..6_000 {
        let ia = tracking(&a, 0.0, 0.0);
        let ib = tracking(&b, 0.0, 0.0);
        a.step(ia);
        b.step(ib);
        match (a.pickup(), b.pickup()) {
            (Some(pa), Some(pb)) => {
                assert_eq!((pa.x, pa.y), (pb.x, pb.y), "pickup positions diverged");
            }
            (None, None) => {}
            _ => panic!("one game had a pickup and the other did not"),
        }
    }
}
