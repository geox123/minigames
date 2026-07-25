//! The ship: how it turns, thrusts, coasts and wraps — via the seam.

mod common;

use asteroids_core::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use common::{game, speed, still, thrust, turn_left, turn_right};
use std::f32::consts::TAU;

#[test]
fn a_new_game_places_the_ship_at_the_centre_facing_up() {
    let ship = game(1).ship();
    assert!(
        (ship.x - LOGICAL_WIDTH / 2.0).abs() < 1e-3,
        "centred horizontally"
    );
    assert!(
        (ship.y - LOGICAL_HEIGHT / 2.0).abs() < 1e-3,
        "centred vertically"
    );
    assert_eq!(ship.angle, 0.0, "facing straight up");
    assert_eq!(ship.vx, 0.0);
    assert_eq!(ship.vy, 0.0);
    assert!(!ship.thrusting);
}

#[test]
fn turning_right_increases_the_facing_angle() {
    let mut game = game(1);
    let mut last = game.ship().angle;
    for _ in 0..30 {
        game.step(turn_right());
        let now = game.ship().angle;
        assert!(now > last, "the facing keeps rotating clockwise");
        last = now;
    }
    // ~30 steps at the turn rate lands roughly a radian round.
    assert!(
        (0.5..1.5).contains(&last),
        "turned about a radian, got {last}"
    );
}

#[test]
fn turning_left_unwinds_the_facing() {
    let mut game = game(1);
    // Wind the facing well clockwise first, so unwinding stays clear of the
    // zero-crossing where the angle would wrap around.
    for _ in 0..40 {
        game.step(turn_right());
    }
    let mut last = game.ship().angle;
    for _ in 0..20 {
        game.step(turn_left());
        let now = game.ship().angle;
        assert!(now < last, "turning left keeps walking the facing back");
        last = now;
    }
    assert!(last > 0.3, "and stays clear of straight up, got {last}");
}

#[test]
fn thrusting_up_from_rest_drives_the_ship_up() {
    let mut game = game(1);
    for _ in 0..10 {
        game.step(thrust());
    }
    let ship = game.ship();
    assert!(ship.vy < 0.0, "gained upward velocity");
    assert!(
        ship.vx.abs() < 1e-3,
        "and none sideways, got vx {}",
        ship.vx
    );
    assert!(ship.y < LOGICAL_HEIGHT / 2.0, "and moved up the field");
}

#[test]
fn thrust_pushes_along_the_facing() {
    let mut game = game(1);
    // Turn until the ship faces roughly right (a quarter-turn clockwise).
    while game.ship().angle < TAU / 4.0 {
        game.step(turn_right());
    }
    for _ in 0..10 {
        game.step(thrust());
    }
    let ship = game.ship();
    assert!(ship.vx > 0.0, "facing right, thrust drives it right");
    assert!(
        ship.vx > ship.vy.abs() * 10.0,
        "and the push is dominantly sideways (vx {}, vy {})",
        ship.vx,
        ship.vy
    );
}

#[test]
fn coasting_bleeds_the_ships_speed_off() {
    let mut game = game(1);
    // Build some speed.
    for _ in 0..60 {
        game.step(thrust());
    }
    // Then let go and coast; the speed should fall every step.
    let mut last = speed(game.ship());
    assert!(last > 0.0, "started the coast with real speed");
    for _ in 0..120 {
        game.step(still());
        let now = speed(game.ship());
        assert!(
            now < last,
            "friction keeps bleeding speed off ({now} !< {last})"
        );
        last = now;
    }
}

#[test]
fn the_ship_has_a_top_speed() {
    let mut game = game(1);
    let mut speeds = Vec::new();
    let mut last = 0.0_f32;
    for _ in 0..600 {
        game.step(thrust());
        let now = speed(game.ship());
        assert!(now >= last - 1e-3, "speed climbs to the cap, never past it");
        assert!(now < 1000.0, "and stays comfortably bounded, got {now}");
        speeds.push(now);
        last = now;
    }
    // It plateaus: the last stretch is flat, so a real ceiling holds it.
    let end = *speeds.last().unwrap();
    let earlier = speeds[speeds.len() - 50];
    assert!(
        (end - earlier).abs() < 0.5,
        "the top speed is a stable plateau"
    );
    assert!(end > 0.0);
}

#[test]
fn the_ship_wraps_around_the_edges() {
    let mut game = game(1);
    // Thrust straight up: the ship rides off the top edge and reappears at the
    // bottom, over and over.
    let mut prev_y = game.ship().y;
    let mut wraps = 0;
    for _ in 0..500 {
        game.step(thrust());
        let ship = game.ship();
        assert!(
            (0.0..LOGICAL_WIDTH).contains(&ship.x) && (0.0..LOGICAL_HEIGHT).contains(&ship.y),
            "the ship never leaves the field ({}, {})",
            ship.x,
            ship.y
        );
        assert!(
            (ship.x - LOGICAL_WIDTH / 2.0).abs() < 1e-3,
            "thrusting up keeps it on its column"
        );
        // A jump from near the top to near the bottom is a wrap.
        if ship.y - prev_y > LOGICAL_HEIGHT / 2.0 {
            wraps += 1;
        }
        prev_y = ship.y;
    }
    assert!(wraps >= 1, "the ship wrapped the top edge at least once");
}
