//! The computer opponent: it has to hold a rally up, and it has to be beatable.

mod common;

use common::{MAX_STEPS, tracking};
use pong_core::{Axis, Game, Input, LOGICAL_HEIGHT, PADDLE_HEIGHT, Players, Side};

#[test]
fn the_computer_takes_over_the_right_paddle_in_a_one_player_game() {
    let mut alone = Game::new(Players::One, 7);
    let mut together = Game::new(Players::Two, 7);

    // The right player holds a key down. In a two-player game that is the only
    // thing moving their paddle; in a one-player game the computer has it.
    let held = Input {
        left: Axis::Hold,
        right: Axis::Up,
    };
    let mut diverged = false;
    for _ in 0..600 {
        alone.step(held);
        together.step(held);
        diverged |= alone.paddle(Side::Right).y != together.paddle(Side::Right).y;
    }

    assert!(diverged, "the computer never took over the right paddle");
}

#[test]
fn the_computer_keeps_a_rally_going() {
    let mut game = Game::new(Players::One, 7);
    let mut returns = 0;

    for _ in 0..MAX_STEPS {
        let approaching = game.ball().vx > 0.0;
        let input = tracking(&game, 0.0, 0.0);
        if game.step(input).paddle_hit && approaching {
            returns += 1;
        }
    }

    assert!(
        returns > 20,
        "the computer only returned the ball {returns} times in a long match"
    );
}

/// The opponent watches the ball rather than working out where it will end up,
/// and it is a shade slower than a player — so a player who aims away from it
/// can win points off it.
#[test]
fn the_computer_can_be_beaten_by_aiming_away_from_it() {
    let mut game = Game::new(Players::One, 7);

    for _ in 0..MAX_STEPS {
        // Strike the ball off the end of the paddle that sends it to the corner
        // the opponent is furthest from.
        let opponent = game.paddle(Side::Right).y + PADDLE_HEIGHT / 2.0;
        let aim = if opponent < LOGICAL_HEIGHT / 2.0 {
            -18.0
        } else {
            18.0
        };

        let input = tracking(&game, aim, 0.0);
        if game.step(input).scored == Some(Side::Left) {
            return;
        }
    }

    panic!("the computer never conceded a point, so it cannot be beaten");
}
