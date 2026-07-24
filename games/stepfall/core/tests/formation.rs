//! The formation: how it marches, and how it turns at the edges — via the seam.

mod common;

use common::{MAX_STEPS, formation_top, game, push, still};
use stepfall_core::{COLS, INVADERS, Move, ROWS};

#[test]
fn a_fresh_formation_is_five_rows_of_eleven() {
    let game = game(1);
    assert_eq!(game.invaders().count(), INVADERS);
    assert_eq!(game.alive(), INVADERS as u32);

    // Every row is represented, eleven wide.
    for row in 0..ROWS {
        let in_row = game.invaders().filter(|i| i.row == row).count();
        assert_eq!(in_row, COLS, "row {row} should hold {COLS} invaders");
    }
}

#[test]
fn the_march_advances_one_invader_per_interrupt() {
    let mut game = game(1);
    let before: Vec<f32> = game.invaders().map(|i| i.x).collect();

    // One machine interrupt is two simulation steps.
    game.step(still());
    game.step(still());

    let after: Vec<f32> = game.invaders().map(|i| i.x).collect();
    let moved: Vec<(usize, f32, f32)> = before
        .iter()
        .zip(&after)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| (i, *a, *b))
        .collect();

    assert_eq!(moved.len(), 1, "exactly one invader moves per interrupt");
    let (_, from, to) = moved[0];
    assert_eq!(to - from, 2.0, "and it steps two pixels");
}

#[test]
fn a_full_pass_moves_every_invader_once() {
    let mut game = game(1);
    let before: Vec<f32> = game.invaders().map(|i| i.x).collect();

    // A pass costs one interrupt — two steps — per standing invader.
    for _ in 0..(INVADERS * 2) {
        game.step(still());
    }

    let after: Vec<f32> = game.invaders().map(|i| i.x).collect();
    for (i, (from, to)) in before.iter().zip(&after).enumerate() {
        assert_eq!(to - from, 2.0, "invader {i} should have stepped once");
    }
}

#[test]
fn the_formation_turns_at_an_edge_and_steps_down() {
    let mut game = game(1);
    let start_top = formation_top(&game);

    // These runs are long enough for return fire to matter, so the cannon hugs
    // the left wall — out of reach of bombs, which fall from the formation as it
    // marches to the *right* edge — leaving the march to be measured cleanly.
    let dodge = || push(Move::Left);

    // March until the formation reaches a wall and turns.
    let mut turned = false;
    for _ in 0..MAX_STEPS {
        if game.step(dodge()).turned {
            turned = true;
            break;
        }
    }
    assert!(turned, "the formation never reached an edge");
    assert_eq!(
        formation_top(&game),
        start_top,
        "it holds its height until it turns"
    );

    // The pass after the turn steps the whole formation down.
    for _ in 0..(INVADERS * 2) {
        game.step(dodge());
    }
    assert_eq!(
        formation_top(&game) - start_top,
        8.0,
        "the formation drops eight pixels"
    );
}

#[test]
fn the_formation_marches_back_the_other_way_after_turning() {
    let mut game = game(1);
    let dodge = || push(Move::Left);

    // Reach the first turn, then let the downward pass finish.
    for _ in 0..MAX_STEPS {
        if game.step(dodge()).turned {
            break;
        }
    }
    for _ in 0..(INVADERS * 2) {
        game.step(dodge());
    }

    // Now it should be heading the other way.
    let before: Vec<f32> = game.invaders().map(|i| i.x).collect();
    for _ in 0..(INVADERS * 2) {
        game.step(dodge());
    }
    let after: Vec<f32> = game.invaders().map(|i| i.x).collect();
    for (from, to) in before.iter().zip(&after) {
        assert_eq!(to - from, -2.0, "the formation reverses after a turn");
    }
}
