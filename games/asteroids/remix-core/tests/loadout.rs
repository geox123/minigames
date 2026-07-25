//! The loadout seeds the ship — the meta's effect on a run, through the `Game` seam.
//! The meta module builds a `Loadout`; here we prove the core reads it (and knows
//! nothing of where it came from). The meta's own bookkeeping is unit-tested in the
//! crate's `meta` module.

use asteroids_remix_core::{Game, Loadout, Mode};

#[test]
fn a_default_loadout_flies_the_base_ship() {
    let game = Game::new(1, Mode::Orbit, Loadout::default());
    assert_eq!(game.weapon_level(), 0);
    assert!(!game.has_shield());
    assert_eq!(game.lives(), 3);
    assert_eq!(game.collapse_meter(), 0.0);
}

#[test]
fn the_loadout_seeds_the_ship() {
    let loadout = Loadout {
        start_weapon: 2,
        shield: true,
        bonus_lives: 2,
        start_charge: true,
    };
    let game = Game::new(1, Mode::Orbit, loadout);
    assert_eq!(game.weapon_level(), 2, "the weapon starts stepped up");
    assert!(game.has_shield(), "the shield starts up");
    assert_eq!(game.lives(), 5, "bonus lives add to the base three");
    assert_eq!(
        game.collapse_meter(),
        1.0,
        "a starting charge fills the meter"
    );
}

#[test]
fn the_loadout_never_exceeds_the_weapon_cap() {
    let loadout = Loadout {
        start_weapon: 99,
        ..Default::default()
    };
    let game = Game::new(1, Mode::Orbit, loadout);
    assert_eq!(game.weapon_level(), 3, "a wild start weapon is capped");
}

#[test]
fn a_restart_replays_the_same_loadout() {
    let loadout = Loadout {
        start_weapon: 1,
        shield: true,
        ..Default::default()
    };
    let mut game = Game::new(1, Mode::Orbit, loadout);
    game.restart();
    assert_eq!(game.weapon_level(), 1, "the restart re-seeds the loadout");
    assert!(game.has_shield());
}
