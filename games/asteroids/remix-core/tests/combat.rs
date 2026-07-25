//! Firing through the seam: the held stream, and that firing replays deterministically.

use asteroids_remix_core::{Game, Input, Loadout, Mode};

fn game(seed: u64) -> Game {
    Game::new(seed, Mode::Orbit, Loadout::default())
}

fn firing() -> Input {
    Input {
        fire: true,
        ..Default::default()
    }
}

#[test]
fn holding_fire_streams_shots() {
    let mut game = game(1);
    for _ in 0..40 {
        game.step(firing());
    }
    assert!(
        game.shots().count() > 1,
        "a held button streams several shots into flight"
    );
}

#[test]
fn firing_replays_deterministically() {
    // A firing, turning, thrusting run replays identically on the same seed —
    // including the seeded scatter of any fragments it breaks off.
    let script = |i: usize| Input {
        turn_right: i.is_multiple_of(4),
        thrust: i.is_multiple_of(3),
        fire: i.is_multiple_of(2),
        ..Default::default()
    };
    let mut a = game(7);
    let mut b = game(7);
    for i in 0..2500 {
        a.step(script(i));
        b.step(script(i));
    }
    assert_eq!(a.ship(), b.ship());
    assert_eq!(a.asteroid_count(), b.asteroid_count());
    let a_shots: Vec<_> = a.shots().collect();
    let b_shots: Vec<_> = b.shots().collect();
    assert_eq!(a_shots, b_shots);
}
