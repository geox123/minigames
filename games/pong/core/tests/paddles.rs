//! Paddle control and the limits of the play field.

use pong_core::{Axis, Game, Input, LOGICAL_HEIGHT, PADDLE_HEIGHT, Side, TIMESTEP};

/// Long enough for a paddle to cross the field several times over.
const LONG_HOLD: usize = (5.0 / TIMESTEP) as usize;

fn hold(game: &mut Game, input: Input, steps: usize) {
    for _ in 0..steps {
        game.step(input);
    }
}

#[test]
fn each_paddle_answers_to_its_own_player() {
    let mut game = Game::new(7);
    let (left_start, right_start) = (game.paddle(Side::Left).y, game.paddle(Side::Right).y);

    hold(
        &mut game,
        Input {
            left: Axis::Up,
            right: Axis::Down,
        },
        30,
    );

    assert!(
        game.paddle(Side::Left).y < left_start,
        "left paddle did not move up"
    );
    assert!(
        game.paddle(Side::Right).y > right_start,
        "right paddle did not move down"
    );
}

#[test]
fn a_players_keys_leave_the_other_paddle_alone() {
    let mut game = Game::new(7);
    let right_start = game.paddle(Side::Right).y;

    hold(
        &mut game,
        Input {
            left: Axis::Up,
            right: Axis::Hold,
        },
        30,
    );

    assert_eq!(
        game.paddle(Side::Right).y,
        right_start,
        "right paddle moved on the left player's key"
    );
}

#[test]
fn a_paddle_stops_at_the_bottom_of_the_field() {
    let mut game = Game::new(7);

    hold(
        &mut game,
        Input {
            left: Axis::Down,
            right: Axis::Down,
        },
        LONG_HOLD,
    );

    let bottom = game.paddle(Side::Left).y + PADDLE_HEIGHT;
    assert!(
        (bottom - LOGICAL_HEIGHT).abs() < 0.001,
        "paddle rests at {bottom}, not flush with the bottom of the field"
    );
}

/// The 1972 original's paddles could not quite reach the top of the screen — a
/// hardware limitation Atari shipped, and part of how the game plays.
#[test]
fn a_paddle_cannot_reach_the_very_top_of_the_field() {
    let mut game = Game::new(7);

    hold(
        &mut game,
        Input {
            left: Axis::Up,
            right: Axis::Up,
        },
        LONG_HOLD,
    );

    assert!(
        game.paddle(Side::Left).y > 0.0,
        "paddle reached the top of the field; the original's could not"
    );
}
