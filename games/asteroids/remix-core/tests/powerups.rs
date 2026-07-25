//! Power-ups through the seam: the run begins kitless, and the drops enemies leave
//! replay deterministically. Catching a pickup and the shield soak are set up in the
//! crate's white-box tests, since flying the ship onto a dropped pickup — and downing
//! the enemy that drops it — cannot be staged cheaply by honest play.

mod common;

use asteroids_remix_core::Input;
use common::game;

#[test]
fn a_fresh_run_carries_no_kit() {
    let game = game(1);
    assert_eq!(game.weapon_level(), 0, "no weapon step handed down");
    assert!(!game.has_shield(), "and no shield — the run is kitless");
}

#[test]
fn power_ups_replay_deterministically() {
    // A firing, thrusting run downs enemies and scatters power-ups; on the same seed it
    // replays identically, pickups and the weapon it climbs to included.
    let script = |i: usize| Input {
        turn_right: i.is_multiple_of(5),
        thrust: i.is_multiple_of(2),
        fire: i.is_multiple_of(3),
        ..Default::default()
    };
    let mut a = game(11);
    let mut b = game(11);
    for i in 0..3000 {
        a.step(script(i));
        b.step(script(i));
    }
    let (pa, pb): (Vec<_>, Vec<_>) = (a.pickups().collect(), b.pickups().collect());
    assert_eq!(pa, pb, "the same seed scatters the same power-ups");
    assert_eq!(a.weapon_level(), b.weapon_level());
    assert_eq!(a.has_shield(), b.has_shield());
}
