//! The orbital enemy zoo through the seam: a wave flies in, leads with the expected
//! kinds, opens fire, and replays deterministically. Downing enemies (with the guns
//! or the collapse), the mine's wake, the shepherd's herd and the escalation are set
//! up directly in the crate's white-box tests, since honest play cannot stage them.

mod common;

use asteroids_remix_core::{EnemyKind, Input};
use common::{game, still};

#[test]
fn a_wave_of_enemies_flies_in() {
    let mut game = game(1);
    // The field opens clear of enemies; the first wave arrives after the opening gap.
    assert_eq!(game.enemy_count(), 0, "the run opens with an empty sky");
    let mut arrived = false;
    for _ in 0..600 {
        game.step(still());
        if game.enemy_count() > 0 {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "a wave flies in after the opening gap");
}

#[test]
fn the_first_wave_leads_with_an_orbiter_and_a_diver() {
    let mut game = game(1);
    for _ in 0..600 {
        game.step(still());
        if game.enemy_count() > 0 {
            break;
        }
    }
    let kinds: Vec<_> = game.enemies().map(|e| e.kind).collect();
    assert!(
        kinds.contains(&EnemyKind::Orbiter),
        "the first wave leads with an Orbiter (got {kinds:?})"
    );
    assert!(
        kinds.contains(&EnemyKind::Diver),
        "and a Diver (got {kinds:?})"
    );
}

#[test]
fn settled_enemies_open_fire() {
    // Left to itself, the run's enemies loose fire — the Orbiter aims at the ship, the
    // Diver fires ahead. They keep firing even through the pause after a death.
    let mut game = game(1);
    let mut fired = false;
    for _ in 0..1000 {
        let events = game.step(still());
        if events.enemy_fired || game.enemy_bullets().count() > 0 {
            fired = true;
            break;
        }
    }
    assert!(fired, "settled enemies open fire");
}

#[test]
fn the_zoo_replays_deterministically() {
    // A turning, thrusting, firing run replays identically on the same seed — enemies,
    // their fire and the score all included.
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
    let (ea, eb): (Vec<_>, Vec<_>) = (a.enemies().collect(), b.enemies().collect());
    assert_eq!(ea, eb, "the same seed and inputs replay the same enemies");
    let (ba, bb): (Vec<_>, Vec<_>) = (a.enemy_bullets().collect(), b.enemy_bullets().collect());
    assert_eq!(ba, bb, "and the same enemy fire");
    assert_eq!(a.score(), b.score());
}
