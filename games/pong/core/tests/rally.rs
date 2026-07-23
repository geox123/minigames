//! How the ball comes off a paddle, and how a rally builds.

mod common;

use common::{centred_rally, speed, strike};
use pong_core::{Game, Players, Side};

/// The original's paddle is read in eight segments: where the ball lands along
/// the paddle decides the angle it leaves at, which is what lets a player aim.
#[test]
fn where_the_ball_strikes_the_paddle_decides_where_it_goes() {
    let struck_high = strike(&mut Game::new(Players::Two, 7), Side::Left, 12.0);
    let struck_centre = strike(&mut Game::new(Players::Two, 7), Side::Left, 0.0);
    let struck_low = strike(&mut Game::new(Players::Two, 7), Side::Left, -12.0);

    assert!(
        struck_high.vy < 0.0,
        "a ball landing above the paddle's centre should be sent upwards, got vy = {}",
        struck_high.vy
    );
    assert!(
        struck_low.vy > 0.0,
        "a ball landing below the paddle's centre should be sent downwards, got vy = {}",
        struck_low.vy
    );
    assert!(
        struck_centre.vy.abs() < struck_high.vy.abs(),
        "a ball off the centre of the paddle should fly flatter than one off its edge"
    );
}

#[test]
fn a_paddle_sends_the_ball_back_the_way_it_came() {
    let mut game = Game::new(Players::Two, 7);

    let off_the_left = strike(&mut game, Side::Left, 0.0);
    assert!(off_the_left.vx > 0.0, "left paddle did not turn the ball");

    let off_the_right = strike(&mut game, Side::Right, 0.0);
    assert!(off_the_right.vx < 0.0, "right paddle did not turn the ball");
}

#[test]
fn a_paddle_hit_is_reported_to_the_shell() {
    let mut game = Game::new(Players::Two, 7);

    // `strike` only returns on a reported hit, so reaching here is the assertion.
    strike(&mut game, Side::Left, 0.0);
}

/// The original sped the ball up twice as a rally went on, which is where the
/// tension in a long exchange comes from.
#[test]
fn the_ball_speeds_up_as_the_rally_goes_on() {
    let mut game = Game::new(Players::Two, 7);

    let rally = centred_rally(&mut game, 16);
    let (opening, middle, late) = (
        speed(rally[0]),
        speed(rally[SPEED_UP_HITS.0]),
        speed(rally[SPEED_UP_HITS.1]),
    );

    assert!(
        opening < middle && middle < late,
        "rally speeds went {opening} -> {middle} -> {late}, which does not build"
    );
}

#[test]
fn the_ball_holds_its_speed_between_the_step_ups() {
    let mut game = Game::new(Players::Two, 7);

    let rally = centred_rally(&mut game, 16);

    for hit in 1..SPEED_UP_HITS.0 {
        assert!(
            (speed(rally[hit]) - speed(rally[0])).abs() < 0.01,
            "the ball changed speed at hit {hit}, before the first step-up"
        );
    }
}

/// Zero-based indices of the hits at which the original stepped the ball up.
const SPEED_UP_HITS: (usize, usize) = (3, 11);
