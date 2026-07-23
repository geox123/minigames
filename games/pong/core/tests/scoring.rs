//! Points, serves, and the end of a match.

mod common;

use common::{centred_rally, conceding, play_a_point, serve_direction, speed};
use pong_core::{BALL_SPEED, Game, Input, LOGICAL_HEIGHT, LOGICAL_WIDTH, Phase, Side, WIN_SCORE};

#[test]
fn a_ball_that_gets_past_a_paddle_scores_for_the_other_player() {
    let mut game = Game::new(7);

    let scorer = play_a_point(&mut game, Side::Left);

    assert_eq!(scorer, Side::Right);
    assert_eq!(game.score(Side::Right), 1);
    assert_eq!(game.score(Side::Left), 0);
}

#[test]
fn each_point_starts_again_from_the_middle_at_the_opening_speed() {
    let mut game = Game::new(7);

    // Get the ball up to its fastest, then lose the point.
    centred_rally(&mut game, 13);
    play_a_point(&mut game, Side::Left);

    let waiting = game.ball();
    assert_eq!(
        (waiting.x, waiting.y),
        (LOGICAL_WIDTH / 2.0, LOGICAL_HEIGHT / 2.0),
        "the ball does not wait in the middle of the field"
    );
    assert_eq!(game.phase(), Phase::Serving);

    serve_direction(&mut game);
    assert!(
        (speed(game.ball()) - BALL_SPEED).abs() < 0.01,
        "the new point started at {} rather than the opening speed",
        speed(game.ball())
    );
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
        assert_ne!(
            pair[0], pair[1],
            "two serves in a row went the same way: {serves:?}"
        );
    }
}

#[test]
fn the_first_player_to_eleven_points_wins() {
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
fn a_won_match_stays_won() {
    let mut game = Game::new(7);
    for _ in 0..WIN_SCORE {
        play_a_point(&mut game, Side::Left);
    }

    for _ in 0..2_000 {
        let input = conceding(&game, Side::Left);
        assert_eq!(game.step(input).scored, None);
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
fn a_finished_match_can_be_restarted() {
    let mut game = Game::new(7);
    for _ in 0..WIN_SCORE {
        play_a_point(&mut game, Side::Left);
    }

    game.restart();

    assert_eq!(game.score(Side::Left), 0);
    assert_eq!(game.score(Side::Right), 0);
    assert_eq!(game.phase(), Phase::Serving);

    assert_eq!(play_a_point(&mut game, Side::Left), Side::Right);
    assert_eq!(game.score(Side::Right), 1);
}

#[test]
fn a_match_can_be_restarted_in_the_middle() {
    let mut game = Game::new(7);
    play_a_point(&mut game, Side::Left);
    play_a_point(&mut game, Side::Left);

    game.restart();

    assert_eq!(game.score(Side::Right), 0);
    assert_eq!(game.phase(), Phase::Serving);
}

#[test]
fn the_ball_waits_before_each_serve() {
    let mut game = Game::new(7);

    let still = (0..30).all(|_| {
        game.step(Input::default());
        game.ball().vx == 0.0
    });

    assert!(still, "the ball was served with no pause at all");
    assert_eq!(game.phase(), Phase::Serving);
}
