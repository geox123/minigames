//! PULSE's Versus baseline: paddles, deflection, and the match to seven.

mod common;

use common::{conceding, play_a_point, serve_direction, strike, tracking};
use pong_remix_core::{
    Axis, Game, Input, LOGICAL_HEIGHT, PADDLE_HEIGHT, Phase, Side, TIMESTEP, WIN_SCORE,
};

const LONG_HOLD: usize = (5.0 / TIMESTEP) as usize;

fn hold(game: &mut Game, input: Input, steps: usize) {
    for _ in 0..steps {
        game.step(input);
    }
}

#[test]
fn each_paddle_answers_to_its_own_player() {
    let mut game = Game::new(7);
    let (left, right) = (game.paddle(Side::Left).y, game.paddle(Side::Right).y);

    hold(
        &mut game,
        Input {
            left: Axis::Up,
            right: Axis::Down,
        },
        30,
    );

    assert!(game.paddle(Side::Left).y < left, "left paddle did not rise");
    assert!(
        game.paddle(Side::Right).y > right,
        "right paddle did not fall"
    );
}

#[test]
fn a_paddle_stays_within_the_field() {
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
        "paddle left the floor"
    );
}

#[test]
fn where_the_ball_strikes_the_paddle_decides_where_it_goes() {
    let high = strike(&mut Game::new(7), Side::Left, 12.0);
    let centre = strike(&mut Game::new(7), Side::Left, 0.0);
    let low = strike(&mut Game::new(7), Side::Left, -12.0);

    assert!(high.vy < 0.0, "a high strike should go up, got {}", high.vy);
    assert!(low.vy > 0.0, "a low strike should go down, got {}", low.vy);
    assert!(
        centre.vy.abs() < high.vy.abs(),
        "a centre strike should fly flatter than an edge strike"
    );
}

#[test]
fn a_paddle_sends_the_ball_back_the_way_it_came() {
    let mut game = Game::new(7);
    assert!(
        strike(&mut game, Side::Left, 0.0).vx > 0.0,
        "left didn't turn it"
    );
    assert!(
        strike(&mut game, Side::Right, 0.0).vx < 0.0,
        "right didn't turn it"
    );
}

#[test]
fn a_ball_past_a_paddle_scores_for_the_other_player() {
    let mut game = Game::new(7);
    assert_eq!(play_a_point(&mut game, Side::Left), Side::Right);
    assert_eq!(game.score(Side::Right), 1);
    assert_eq!(game.score(Side::Left), 0);
}

#[test]
fn serves_alternate_between_the_players() {
    let mut game = Game::new(7);
    let mut serves = Vec::new();
    for _ in 0..4 {
        serves.push(serve_direction(&mut game));
        play_a_point(&mut game, Side::Left);
    }
    for pair in serves.windows(2) {
        assert_ne!(pair[0], pair[1], "two serves went the same way: {serves:?}");
    }
}

#[test]
fn the_first_player_to_seven_wins() {
    let mut game = Game::new(7);
    for _ in 0..WIN_SCORE {
        play_a_point(&mut game, Side::Left);
    }
    assert_eq!(game.score(Side::Right), WIN_SCORE);
    assert_eq!(
        game.phase(),
        Phase::GameOver {
            winner: Side::Right
        }
    );
}

#[test]
fn a_won_match_stays_won_until_restart() {
    let mut game = Game::new(7);
    for _ in 0..WIN_SCORE {
        play_a_point(&mut game, Side::Left);
    }
    for _ in 0..1_000 {
        assert_eq!(game.step(conceding(&game, Side::Left)).scored, None);
    }

    game.restart();
    assert_eq!(game.score(Side::Left), 0);
    assert_eq!(game.score(Side::Right), 0);
    assert_eq!(game.phase(), Phase::Serving);
    assert_eq!(play_a_point(&mut game, Side::Left), Side::Right);
}

#[test]
fn the_ball_stays_in_the_field_through_a_long_exchange() {
    let mut game = Game::new(7);
    for _ in 0..12_000 {
        let input = tracking(&game, 12.0, -12.0);
        game.step(input);
        let ball = game.ball();
        assert!(
            ball.y >= -0.01 && ball.y <= LOGICAL_HEIGHT + 0.01,
            "ball left the field at y={}",
            ball.y
        );
    }
}
