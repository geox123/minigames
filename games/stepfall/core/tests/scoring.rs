//! Scoring and the saucer, through the public seam. The precise saucer prize
//! table (and its 300-point quirk) is white-box tested in the crate, since honest
//! play cannot line a saucer up under an open lane at an exact shot count; here we
//! drive what play can reach: that kills score by row, and that the saucer appears
//! on its terms and heads the way the shot count decides.

mod common;

use common::{MAX_STEPS, game, push, shooting};
use stepfall_core::{Move, SAUCER_MIN_INVADERS};

/// What an invader in `row` (0 the top) should score.
fn expected_row_score(row: usize) -> u32 {
    match row {
        0 => 30,
        1 | 2 => 20,
        _ => 10,
    }
}

#[test]
fn invaders_score_by_row_and_a_full_screen_is_990() {
    // The full formation the seam reports is worth 990.
    let start = game(1);
    let full: u32 = start.invaders().map(|i| expected_row_score(i.row)).sum();
    assert_eq!(full, 990, "a cleared screen scores 990");

    // Every kill adds exactly its row's value — 10, 20 or 30.
    let mut game = game(1);
    let mut last = game.score();
    let mut sweep = Move::Right;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..MAX_STEPS {
        let events = game.step(shooting(sweep));
        if let Some(row) = events.invader_killed {
            let gained = game.score() - last;
            assert_eq!(
                gained,
                expected_row_score(row as usize),
                "a row-{row} kill scores its row's value"
            );
            seen.insert(gained);
        }
        last = game.score();
        // Ride the walls back and forth so shots reach every column.
        if game.cannon().x <= 8.5 {
            sweep = Move::Right;
        } else if game.cannon().x >= 200.0 {
            sweep = Move::Left;
        }
        if seen.len() == 3 {
            break;
        }
    }
    assert_eq!(seen.len(), 3, "kills scored all of 10, 20 and 30");
}

#[test]
fn the_saucer_appears_only_while_the_formation_is_thick() {
    // Hold fire (so the formation stays full) and hug the left wall (so the bombs
    // miss); a saucer eventually crosses the top.
    let mut game = game(1);
    let mut appeared = false;
    for _ in 0..8_000 {
        game.step(push(Move::Left));
        if game.saucer().is_some() {
            appeared = true;
            break;
        }
    }
    assert!(appeared, "a saucer crosses while the formation is thick");
    assert!(
        game.alive() >= SAUCER_MIN_INVADERS,
        "and only while at least eight invaders remain"
    );
}

#[test]
fn the_saucer_heads_the_way_the_shot_count_decides() {
    // Drives to the first saucer of a game and reports whether it runs rightward.
    // Throughout, the cannon hugs the left wall, where the bombs miss it.
    fn runs_right(warmup_shots: u32) -> bool {
        let mut game = game(1);
        // Fire the warm-up shots to set the parity.
        for _ in 0..warmup_shots {
            // One shot at a time: wait for the previous to clear first.
            loop {
                let before = game.shot().is_some();
                game.step(shooting(Move::Left));
                if !before && game.shot().is_some() {
                    break;
                }
            }
        }
        // Then hold fire and wait for the saucer, and see which way it travels.
        for _ in 0..8_000 {
            game.step(push(Move::Left));
            if let Some(entry) = game.saucer() {
                // Let it run a couple of interrupts, then compare.
                for _ in 0..4 {
                    game.step(push(Move::Left));
                }
                let now = game.saucer().expect("the saucer is still crossing");
                return now.x > entry.x;
            }
        }
        panic!("no saucer appeared");
    }

    // An even shot count and an odd one send the saucer opposite ways.
    assert_ne!(
        runs_right(0),
        runs_right(1),
        "the saucer's heading flips with the parity of the shot count"
    );
}
