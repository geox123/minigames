//! The ship: where it starts, how it turns and thrusts against the pull, and that it
//! stays on the field — via the seam.

mod common;

use asteroids_remix_core::{LOGICAL_HEIGHT, LOGICAL_WIDTH, WELL_CORE_RADIUS};
use common::{game, still, thrust, turn_left, turn_right};

#[test]
fn a_new_run_places_the_ship_clear_of_the_well() {
    let game = game(1);
    let ship = game.ship();
    assert_eq!(ship.vx, 0.0);
    assert_eq!(ship.vy, 0.0);
    // The single well sits at the centre; the ship starts well outside its core.
    let well = game.wells().next().expect("a well is on the field");
    let dist = ((ship.x - well.x).powi(2) + (ship.y - well.y).powi(2)).sqrt();
    assert!(
        dist > WELL_CORE_RADIUS * 4.0,
        "the ship starts clear of the core"
    );
}

#[test]
fn turning_swings_the_facing_both_ways() {
    let mut game = game(1);
    // Turning right winds the facing up...
    for _ in 0..40 {
        game.step(turn_right());
    }
    let turned = game.ship().angle;
    assert!(
        (0.5..2.0).contains(&turned),
        "turning right winds the facing, got {turned}"
    );
    // ...and turning left unwinds it (kept clear of the zero-crossing).
    let mut last = turned;
    for _ in 0..20 {
        game.step(turn_left());
        let now = game.ship().angle;
        assert!(now < last, "turning left unwinds it");
        last = now;
    }
}

#[test]
fn thrust_adds_to_the_gravity() {
    // From the same start (facing up, the well below), one step of thrust versus one
    // of coasting: thrust pushes the ship upward, against the downward pull, so its
    // velocity comes out more upward (less positive vy) than the coasting ship's.
    let mut thrusting = game(1);
    let mut coasting = game(1);
    thrusting.step(thrust());
    coasting.step(still());
    let t = thrusting.ship();
    let c = coasting.ship();
    assert!(t.vy < c.vy, "thrust pushes up against the pull");
    assert!(c.vy - t.vy > 2.5, "and it is a real push, not a nudge");
}

#[test]
fn the_ship_stays_within_the_field() {
    // Flying hard for a long time, the ship always wraps back into the field — it
    // never escapes the toroidal edges.
    let mut game = game(1);
    for _ in 0..1500 {
        game.step(thrust());
        let ship = game.ship();
        assert!(
            (0.0..LOGICAL_WIDTH).contains(&ship.x) && (0.0..LOGICAL_HEIGHT).contains(&ship.y),
            "the ship stays on the field ({}, {})",
            ship.x,
            ship.y
        );
    }
}
