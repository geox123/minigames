//! The opening field of rocks: how many there are, that they drift in straight
//! lines, and that they wrap — via the seam.

mod common;

use asteroids_core::{AsteroidSize, INITIAL_ASTEROIDS, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use common::{game, still};

/// The shortest signed distance from `a` to `b` on a `max`-wide ring, so a wrap
/// reads as a small step rather than a jump the width of the field.
fn ring_delta(a: f32, b: f32, max: f32) -> f32 {
    let mut d = b - a;
    if d > max / 2.0 {
        d -= max;
    } else if d < -max / 2.0 {
        d += max;
    }
    d
}

#[test]
fn a_new_field_has_four_large_rocks() {
    let game = game(1);
    assert_eq!(game.asteroid_count(), INITIAL_ASTEROIDS);
    assert_eq!(game.asteroids().count(), INITIAL_ASTEROIDS);
    assert!(
        game.asteroids().all(|a| a.size == AsteroidSize::Large),
        "a fresh field is all large rocks"
    );
}

#[test]
fn no_rock_spawns_on_top_of_the_ship() {
    // Across many seeds, every rock stands clear of the ship waiting at the centre.
    for seed in 0..200 {
        for rock in game(seed).asteroids() {
            let dx = rock.x - LOGICAL_WIDTH / 2.0;
            let dy = rock.y - LOGICAL_HEIGHT / 2.0;
            let dist = (dx * dx + dy * dy).sqrt();
            assert!(
                dist > 200.0,
                "seed {seed}: a rock spawned at {dist} from centre"
            );
        }
    }
}

#[test]
fn rocks_drift_in_straight_lines() {
    let mut game = game(7);
    // Each rock moves the same distance in the same direction every step: constant
    // velocity. Measure on the ring so a wrap doesn't masquerade as a turn.
    let mut prev: Vec<(f32, f32)> = game.asteroids().map(|a| (a.x, a.y)).collect();
    let mut first_delta: Vec<(f32, f32)> = Vec::new();
    for step in 0..20 {
        game.step(still());
        let now: Vec<(f32, f32)> = game.asteroids().map(|a| (a.x, a.y)).collect();
        for (i, (p, n)) in prev.iter().zip(&now).enumerate() {
            let dx = ring_delta(p.0, n.0, LOGICAL_WIDTH);
            let dy = ring_delta(p.1, n.1, LOGICAL_HEIGHT);
            if step == 0 {
                first_delta.push((dx, dy));
            } else {
                assert!(
                    (dx - first_delta[i].0).abs() < 1e-3 && (dy - first_delta[i].1).abs() < 1e-3,
                    "rock {i} drifts at a constant velocity"
                );
            }
        }
        prev = now;
    }
}

#[test]
fn rocks_wrap_and_never_leave_the_field() {
    let mut game = game(3);
    // Run long enough that even the slowest rock crosses the field, and check they
    // stay in bounds throughout — which they only can by wrapping.
    for _ in 0..6000 {
        game.step(still());
        for a in game.asteroids() {
            assert!(
                (0.0..LOGICAL_WIDTH).contains(&a.x) && (0.0..LOGICAL_HEIGHT).contains(&a.y),
                "a rock left the field at ({}, {})",
                a.x,
                a.y
            );
        }
    }
}
