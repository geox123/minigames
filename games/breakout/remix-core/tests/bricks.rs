//! The brick wall: layout, collision, destruction and scoring — via the seam.

mod common;

use breakout_remix_core::{BRICK_COLS, BRICK_ROWS, Game, Input, Kind, Move, Pool};
use common::{rally_until, run, track};

#[test]
fn the_wall_starts_full_in_four_bands() {
    let game = run(7);
    let bricks: Vec<_> = game.bricks().collect();
    assert_eq!(
        bricks.len(),
        BRICK_ROWS * BRICK_COLS,
        "the wall should be {BRICK_ROWS} rows of {BRICK_COLS}"
    );

    let mut bands: Vec<u8> = bricks.iter().map(|b| b.band).collect();
    bands.sort_unstable();
    bands.dedup();
    assert_eq!(bands, vec![0, 1, 2, 3], "there should be four bands");

    // The four bands score 1, 3, 5, 7 — the higher rows worth more.
    assert_eq!(game.band_points(0), 1);
    assert_eq!(game.band_points(1), 3);
    assert_eq!(game.band_points(2), 5);
    assert_eq!(game.band_points(3), 7);
}

#[test]
fn base_pool_walls_include_every_brick_kind() {
    // Base-pool walls are mostly normal bricks with the whole zoo sprinkled in;
    // gather across several seeds so every kind is bound to turn up.
    use std::collections::HashSet;
    let mut seen: HashSet<Kind> = HashSet::new();
    for seed in 0..12 {
        seen.extend(run(seed).bricks().map(|b| b.kind));
    }
    for kind in [
        Kind::Normal,
        Kind::Armoured,
        Kind::Mirror,
        Kind::Explosive,
        Kind::Mover,
        Kind::Spawner,
    ] {
        assert!(
            seen.contains(&kind),
            "{kind:?} should appear in base-pool walls"
        );
    }
}

#[test]
fn deflecting_the_ball_breaks_bricks_and_scores() {
    let mut game = run(7);
    let before = game.bricks().count();
    assert_eq!(game.score(), 0);

    // Play a defended rally until at least a few bricks have fallen.
    rally_until(&mut game, |g| g.bricks().count() + 5 <= before);

    let broken = before - game.bricks().count();
    assert!(broken >= 5, "expected several bricks broken, got {broken}");
    assert!(game.score() > 0, "breaking bricks should score");
    // The score is a sum of band points, so at least as large as the count.
    assert!(
        game.score() >= broken as u32,
        "score {} should be at least the {broken} bricks broken",
        game.score()
    );
}

#[test]
fn a_fast_ball_never_tunnels_through_the_wall() {
    // On a plain (normal-only) wall the ball keeps breaking bricks rather than
    // slipping through — the standing count only ever falls, one direct contact
    // at a time (a cleared wall refills, so ignore the step it jumps back up).
    // A normal-only pool isolates this from the zoo's chain-breaks and regrowth.
    let mut game = Game::new_run(3, &Pool::default());
    let full = BRICK_ROWS * BRICK_COLS;
    let mut last = game.bricks().count();
    for _ in 0..40_000 {
        let paddle = track(&game, 0.0);
        game.step(Input { paddle });
        let now = game.bricks().count();
        if now <= last {
            assert!(
                last - now <= 1,
                "more than one brick broke in a single step"
            );
        } else {
            assert_eq!(now, full, "the count only rises when a fresh wall comes up");
        }
        last = now;
    }
}

#[test]
fn a_missed_ball_leaves_the_wall_standing() {
    // Losing the ball does not clear or refill bricks — only breaking does.
    let mut game = run(4);
    let full = game.bricks().count();
    for _ in 0..common::MAX_STEPS {
        // Dodge the ball so it falls past the paddle.
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let paddle = if ball.x < centre {
            Move::Right
        } else {
            Move::Left
        };
        if game.step(Input { paddle }).lost_ball {
            break;
        }
    }
    assert!(
        game.bricks().count() <= full,
        "a lost ball must never add bricks"
    );
}
