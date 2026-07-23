//! Shield, Widen and Slow-mo: the three pickups beyond Multiball.

mod common;

use common::{speed, tracking};
use pong_remix_core::{
    Axis, Game, PADDLE_HEIGHT, PickupKind, SLOWMO_TIME, Side, TIMESTEP, WIDEN_TIME,
};

/// Steps a centred rally on `game` for one frame and returns the events.
fn rally_step(game: &mut Game) {
    let input = tracking(game, 0.0, 0.0);
    game.step(input);
}

#[test]
fn all_four_pickup_kinds_can_spawn() {
    let mut game = Game::new(3);
    let mut seen = [false; 4];
    for _ in 0..common::MAX_STEPS {
        rally_step(&mut game);
        if let Some(pickup) = game.pickup() {
            let i = match pickup.kind {
                PickupKind::Multiball => 0,
                PickupKind::Shield => 1,
                PickupKind::Widen => 2,
                PickupKind::SlowMo => 3,
            };
            seen[i] = true;
        }
        if seen.iter().all(|&s| s) {
            return;
        }
    }
    panic!("not every pickup kind spawned: {seen:?}");
}

#[test]
fn widen_enlarges_a_paddle_then_reverts() {
    let mut game = Game::new(7);
    let mut widened = None;
    for _ in 0..common::MAX_STEPS {
        rally_step(&mut game);
        for side in [Side::Left, Side::Right] {
            if game.paddle_height(side) > PADDLE_HEIGHT + 0.1 {
                widened = Some(side);
            }
        }
        if widened.is_some() {
            break;
        }
    }
    let side = widened.expect("no widen was ever collected");
    assert!(game.paddle_height(side) > PADDLE_HEIGHT);

    // Play well past the widen's lifetime; it should return to normal length.
    for _ in 0..((WIDEN_TIME / TIMESTEP) as usize + 200) {
        rally_step(&mut game);
    }
    assert!(
        (game.paddle_height(side) - PADDLE_HEIGHT).abs() < 0.1,
        "the widen should have reverted, but the paddle is {} long",
        game.paddle_height(side)
    );
}

#[test]
fn slowmo_slows_the_balls_then_reverts() {
    let mut game = Game::new(7);
    for _ in 0..common::MAX_STEPS {
        rally_step(&mut game);
        if game.slow_motion() {
            break;
        }
    }
    assert!(game.slow_motion(), "no slow-mo was ever collected");

    // A ball covers less ground per step than its speed would imply while slow.
    let before = game.ball();
    rally_step(&mut game);
    let after = game.ball();
    let travelled = (after.x - before.x).hypot(after.y - before.y);
    let full_step = speed(before) * TIMESTEP;
    assert!(
        travelled < full_step * 0.75,
        "the ball should crawl while slowed: moved {travelled}, full step {full_step}"
    );

    for _ in 0..((SLOWMO_TIME / TIMESTEP) as usize + 60) {
        rally_step(&mut game);
    }
    assert!(!game.slow_motion(), "slow-mo should have worn off");
}

#[test]
fn a_shield_saves_exactly_one_goal_then_is_spent() {
    let mut game = Game::new(7);
    let mut shielded = None;
    for _ in 0..common::MAX_STEPS {
        rally_step(&mut game);
        for side in [Side::Left, Side::Right] {
            if game.has_shield(side) {
                shielded = Some(side);
            }
        }
        if shielded.is_some() {
            break;
        }
    }
    let defender = shielded.expect("no shield was ever collected");
    let attacker = defender.opposite();
    let conceded_before = game.score(attacker);

    // The shielded player abandons their goal while the other keeps returning
    // the ball at it, forcing the shield to make its one save.
    for _ in 0..common::MAX_STEPS {
        let ball = game.ball();
        let mut input = tracking(&game, 0.0, 0.0);
        match defender {
            Side::Left => input.left = Axis::Down,
            Side::Right => input.right = Axis::Down,
        }
        let _ = ball;
        game.step(input);
        if !game.has_shield(defender) {
            break;
        }
    }

    assert!(
        !game.has_shield(defender),
        "the shield should have been spent on a save"
    );
    assert_eq!(
        game.score(attacker),
        conceded_before,
        "the shield should have saved the goal, not conceded it"
    );
}
