//! Firing through the seam: the on-screen cap, tap-to-fire, the shot's fixed world
//! speed, and that firing replays deterministically.

mod common;

use asteroids_core::{Game, Input, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use common::{game, still, thrust};

/// Holding the fire button down, not turning or thrusting.
fn firing() -> Input {
    Input {
        fire: true,
        ..Default::default()
    }
}

/// The shortest signed distance from `b` to `a` on a `max`-wide ring, so a wrap
/// reads as a small step rather than a jump the width of the field.
fn ring_delta(a: f32, b: f32, max: f32) -> f32 {
    let mut d = a - b;
    if d > max / 2.0 {
        d -= max;
    } else if d < -max / 2.0 {
        d += max;
    }
    d
}

#[test]
fn holding_fire_looses_only_one_shot() {
    // Firing is on the press, not the hold: holding the button down fires exactly
    // once, no matter how long it is held.
    let mut game = game(1);
    let mut fires = 0;
    for _ in 0..30 {
        if game.step(firing()).fired {
            fires += 1;
        }
    }
    assert_eq!(fires, 1, "a held button fires just once");
}

#[test]
fn tapping_fire_is_capped_at_four_shots() {
    // Tap rapidly; at most four of the player's shots are ever on screen at once.
    let mut game = game(1);
    let mut max_seen = 0;
    for i in 0..16_u32 {
        // Fire on even steps, release on odd — six taps in the window.
        let input = if i.is_multiple_of(2) {
            firing()
        } else {
            still()
        };
        game.step(input);
        max_seen = max_seen.max(game.shots().count());
        assert!(game.shots().count() <= 4, "never more than four shots");
    }
    assert_eq!(max_seen, 4, "and the cap is actually reached");
}

#[test]
fn a_shots_speed_is_fixed_not_ship_relative() {
    // A shot flies at a fixed world speed along the facing; the ship's own velocity
    // is not added. So the same shot fired from a still ship and from one racing
    // forward travels at the same speed — the original's outrun-your-own-fire quirk.
    let from_rest = shot_speed(false);
    let from_racing = shot_speed(true);
    assert!(
        (from_rest - from_racing).abs() < 1.0,
        "shot speed is world-fixed: at rest {from_rest}, racing {from_racing}"
    );
}

/// Fires a shot (optionally after building up forward speed) and measures how fast
/// the shot itself travels over one step.
fn shot_speed(racing: bool) -> f32 {
    let mut game = game(1);
    if racing {
        for _ in 0..120 {
            game.step(thrust());
        }
    }
    game.step(firing()); // a fresh press looses one shot
    let a = game.shots().next().expect("a shot is in flight");
    game.step(still()); // let it fly one step, no new shot
    let b = game.shots().next().expect("the shot is still in flight");
    let dx = ring_delta(b.x, a.x, LOGICAL_WIDTH);
    let dy = ring_delta(b.y, a.y, LOGICAL_HEIGHT);
    (dx * dx + dy * dy).sqrt() / (1.0 / 120.0)
}

#[test]
fn firing_reports_a_fired_event_on_the_press() {
    let mut game = game(1);
    assert!(game.step(firing()).fired, "the press fires");
    assert!(!game.step(firing()).fired, "holding does not re-fire");
}

#[test]
fn firing_replays_deterministically() {
    // A firing, turning run replays identically on the same seed — including the
    // seeded scatter of any fragments it breaks off.
    let script = |i: usize| Input {
        turn_right: i.is_multiple_of(4),
        thrust: i.is_multiple_of(3),
        fire: i.is_multiple_of(5),
        ..Default::default()
    };
    let mut a = Game::new(7);
    let mut b = Game::new(7);
    for i in 0..3000 {
        a.step(script(i));
        b.step(script(i));
    }
    assert_eq!(a.score(), b.score());
    assert_eq!(a.asteroid_count(), b.asteroid_count());
    assert_eq!(a.ship(), b.ship());
    assert_eq!(a.lives(), b.lives());
}
