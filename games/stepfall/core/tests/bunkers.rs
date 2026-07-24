//! The bunkers — the cover the game slowly takes away — eroding from both sides,
//! driven through the public seam.

mod common;

use common::{formation_bottom, game, push, shooting};
use stepfall_core::{BUNKERS, CANNON_WIDTH, Game, LOGICAL_WIDTH, Move, Phase, SHOT_WIDTH};

/// The centre of bunker `i`, in logical units.
fn bunker_centre(i: usize) -> f32 {
    (i as f32 + 0.5) / BUNKERS as f32 * LOGICAL_WIDTH
}

/// How many blocks the bunker nearest `centre` has along its lowest row — the
/// row only a shot climbing from below can eat first.
fn bottom_row_blocks(game: &Game, centre: f32) -> usize {
    let half = LOGICAL_WIDTH / (2.0 * BUNKERS as f32);
    let ys: Vec<f32> = game
        .bunker_blocks()
        .filter(|b| (b.x - centre).abs() < half)
        .map(|b| b.y)
        .collect();
    let floor = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    if floor.is_finite() {
        ys.iter().filter(|&&y| y >= floor - 0.5).count()
    } else {
        0
    }
}

/// Slides the cannon so its shot would rise at `shot_x`, without firing.
fn aim(game: &mut Game, shot_x: f32) {
    let target = shot_x - (CANNON_WIDTH - SHOT_WIDTH) / 2.0;
    for _ in 0..400 {
        let x = game.cannon().x;
        if (x - target).abs() < 0.6 {
            break;
        }
        game.step(push(if x > target { Move::Left } else { Move::Right }));
    }
}

#[test]
fn four_bunkers_stand_between_the_cannon_and_the_sky() {
    let game = game(1);
    let blocks: Vec<_> = game.bunker_blocks().collect();
    assert!(!blocks.is_empty(), "the bunkers stand");

    // One bunker in each quarter of the field.
    for i in 0..BUNKERS {
        let lo = i as f32 / BUNKERS as f32 * LOGICAL_WIDTH;
        let hi = (i as f32 + 1.0) / BUNKERS as f32 * LOGICAL_WIDTH;
        assert!(
            blocks.iter().any(|b| b.x >= lo && b.x < hi),
            "a bunker stands in quarter {i}"
        );
    }

    // Below where the formation begins, above the cannon.
    let cannon_top = game.cannon().y;
    let formation = formation_bottom(&game);
    assert!(
        blocks.iter().all(|b| b.y > formation && b.y < cannon_top),
        "the bunkers stand between the formation and the cannon"
    );
}

#[test]
fn a_players_shot_eats_a_bunker_from_below() {
    let mut game = game(1);
    let centre = bunker_centre(0);

    // Aim at a solid column, clear of the central arch, and count the lowest row.
    aim(&mut game, centre - 8.0);
    let bottom_before = bottom_row_blocks(&game, centre);
    assert!(bottom_before > 0, "the bunker's floor starts intact");

    // Hold fire; nudge back onto the column each step in case a hit knocks the
    // cannon back to the middle.
    let mut ate = false;
    let target = centre - 8.0 - (CANNON_WIDTH - SHOT_WIDTH) / 2.0;
    for _ in 0..300 {
        let x = game.cannon().x;
        let dir = if x > target + 0.5 {
            Move::Left
        } else if x < target - 0.5 {
            Move::Right
        } else {
            Move::Hold
        };
        game.step(shooting(dir));
        if bottom_row_blocks(&game, centre) < bottom_before {
            ate = true;
            break;
        }
    }
    assert!(ate, "shots eroded the bunker's lowest row — from below");
}

#[test]
fn an_invader_bomb_eats_a_bunker_from_above() {
    let mut game = game(1);
    let before = game.bunker_blocks().count();

    // Hug the left wall and never fire, so the only thing touching the bunkers is
    // the invaders' return fire falling from above.
    let mut eroded = false;
    for _ in 0..3_000 {
        game.step(push(Move::Left));
        if game.bunker_blocks().count() < before {
            eroded = true;
            break;
        }
    }
    assert!(eroded, "the bombs ate into a bunker");
    assert_eq!(game.phase(), Phase::Playing, "the parked cannon survived");

    // It was the bombs, not the formation: the invaders never reached the cover.
    let bunker_top = game
        .bunker_blocks()
        .map(|b| b.y)
        .fold(f32::INFINITY, f32::min);
    assert!(
        formation_bottom(&game) < bunker_top,
        "the formation is still above the bunkers, so the bombs did the eroding"
    );
}
