//! Ascension through the seam: a tiered run reports its tier, tier 0 is a plain Orbit
//! run, and a restart replays the tier. The tier's escalation of the fields and bosses,
//! and a tiered win, are white-boxed in the crate (honest play can't clear a ladder).

use asteroids_remix_core::{Game, Input, Loadout, Mode};

#[test]
fn an_ascension_run_reports_its_tier() {
    let game = Game::new_ascension(1, 3, Loadout::default());
    assert_eq!(game.tier(), 3);
}

#[test]
fn tier_zero_is_a_plain_orbit_run() {
    let ascension = Game::new_ascension(7, 0, Loadout::default());
    let plain = Game::new(7, Mode::Orbit, Loadout::default());
    // Same seed, tier 0 — the two runs are the same field.
    assert_eq!(ascension.tier(), 0);
    assert_eq!(ascension.mode(), plain.mode());
    assert_eq!(ascension.asteroid_count(), plain.asteroid_count());
    assert_eq!(ascension.ship(), plain.ship());
}

#[test]
fn a_plain_run_has_no_tier() {
    let game = Game::new(1, Mode::Maelstrom, Loadout::default());
    assert_eq!(game.tier(), 0);
}

#[test]
fn a_higher_tier_opens_with_a_denser_field() {
    let plain = Game::new_ascension(1, 0, Loadout::default());
    let ascended = Game::new_ascension(1, 3, Loadout::default());
    assert!(
        ascended.asteroid_count() > plain.asteroid_count(),
        "a higher tier crowds the opening field with more rocks"
    );
}

#[test]
fn a_restart_replays_the_tier() {
    let mut game = Game::new_ascension(9, 2, Loadout::default());
    for i in 0..300u32 {
        game.step(Input {
            thrust: i.is_multiple_of(2),
            ..Default::default()
        });
    }
    game.restart();
    assert_eq!(game.tier(), 2, "the restart keeps the Ascension tier");
}
