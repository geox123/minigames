//! The cannon: it slides along the bottom and stays on the field — via the seam.

mod common;

use common::{game, push, still};
use invaders_core::{CANNON_WIDTH, LOGICAL_WIDTH, Move};

#[test]
fn the_cannon_slides_both_ways() {
    let mut game = game(1);
    let start = game.cannon().x;

    for _ in 0..60 {
        game.step(push(Move::Right));
    }
    let right = game.cannon().x;
    assert!(right > start, "pushing right moves the cannon right");

    for _ in 0..120 {
        game.step(push(Move::Left));
    }
    assert!(game.cannon().x < right, "pushing left moves it back");
}

#[test]
fn the_cannon_holds_still_when_not_pushed() {
    let mut game = game(1);
    let start = game.cannon().x;
    for _ in 0..200 {
        game.step(still());
    }
    assert_eq!(game.cannon().x, start, "an unpushed cannon does not drift");
}

#[test]
fn the_cannon_stays_on_the_field() {
    let mut game = game(1);

    // Press into the left wall for far longer than the field is wide.
    for _ in 0..2_000 {
        game.step(push(Move::Left));
    }
    assert!(
        game.cannon().x >= 0.0,
        "the cannon stopped at the left wall, at x = {}",
        game.cannon().x
    );

    for _ in 0..4_000 {
        game.step(push(Move::Right));
    }
    assert!(
        game.cannon().x + CANNON_WIDTH <= LOGICAL_WIDTH,
        "the cannon stopped at the right wall, at x = {}",
        game.cannon().x
    );
}
