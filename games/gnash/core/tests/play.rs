//! GNASH played through the public [`Game::step`] seam only — the way the shell
//! drives it. These exercise determinism and honest play (steering the eater around
//! the maze and feeding), complementing the in-crate unit tests that plant exact
//! board states.

use gnash_core::{Dir, Game, Input, tile_at};

/// A scripted, engine-free input for step `n`, fully determined by the step index so
/// a run replays exactly. It holds a heading for a stretch (long enough to reach a
/// junction) then picks the next by a deterministic hash, so the eater wanders the
/// whole maze rather than idling in one already-eaten loop.
fn scripted_input(n: u64) -> Input {
    let phase = n / 16;
    let hash = phase
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let dir = match (hash >> 40) % 4 {
        0 => Dir::Left,
        1 => Dir::Up,
        2 => Dir::Right,
        _ => Dir::Down,
    };
    match dir {
        Dir::Up => Input {
            up: true,
            ..Default::default()
        },
        Dir::Down => Input {
            down: true,
            ..Default::default()
        },
        Dir::Left => Input {
            left: true,
            ..Default::default()
        },
        Dir::Right => Input {
            right: true,
            ..Default::default()
        },
    }
}

#[test]
fn the_same_seed_and_inputs_replay_identically() {
    let mut a = Game::new(42);
    let mut b = Game::new(42);
    for n in 0..4000 {
        let input = scripted_input(n);
        let ea = a.step(input);
        let eb = b.step(input);
        assert_eq!(ea, eb, "events diverged at step {n}");
        assert_eq!(a.eater(), b.eater(), "the eater diverged at step {n}");
        assert_eq!(
            a.hunters().collect::<Vec<_>>(),
            b.hunters().collect::<Vec<_>>(),
            "the hunters diverged at step {n}"
        );
        assert_eq!(a.score(), b.score(), "the score diverged at step {n}");
        assert_eq!(
            a.pickups_remaining(),
            b.pickups_remaining(),
            "the board diverged at step {n}"
        );
    }
}

#[test]
fn steering_the_eater_around_feeds_it() {
    // Drive the scripted route for a while; the eater should thread the maze and eat
    // a fair number of dots — evidence it is really navigating, not stuck.
    let mut game = Game::new(1);
    for n in 0..2000 {
        game.step(scripted_input(n));
    }
    assert!(
        game.score() >= 150,
        "the steered eater feeds, got {}",
        game.score()
    );
    assert!(
        game.pickups_remaining() < game.pickups_total(),
        "the board empties as it feeds"
    );
}

#[test]
fn the_eater_stays_on_the_grid() {
    // However it is steered, the eater never leaves the maze (bar the tunnel wrap,
    // which keeps it in-bounds too).
    let mut game = Game::new(3);
    for n in 0..3000 {
        game.step(scripted_input(n));
        let e = game.eater();
        let (col, row) = tile_at(e.x, e.y);
        assert!(
            (0..28).contains(&col) && (0..31).contains(&row),
            "the eater left the grid at step {n}: tile ({col}, {row})"
        );
    }
}
