//! Duel: a best-of-five match, with a beat between games.

mod common;

use common::{conceding, play_a_point};
use pong_remix_core::{Axis, Game, Input, Phase, Side};

/// Plays a Duel where the left player concedes every point, so the right player
/// takes each game. Returns the game once it reaches a terminal or between beat.
fn concede_a_game(game: &mut Game) {
    for _ in 0..common::MAX_STEPS {
        match game.phase() {
            Phase::BetweenGames | Phase::MatchOver { .. } => return,
            _ => {}
        }
        game.step(conceding(game, Side::Left));
    }
    panic!("a game never resolved");
}

#[test]
fn a_game_won_advances_the_match_but_not_over_a_single_game() {
    let mut game = Game::new_duel(7);
    concede_a_game(&mut game);

    assert_eq!(
        game.games_won(Side::Right),
        1,
        "the game winner should be up 1"
    );
    assert_eq!(game.games_won(Side::Left), 0);
    assert_eq!(
        game.phase(),
        Phase::BetweenGames,
        "one game in, the match should pause between games, not end"
    );
}

#[test]
fn there_is_a_beat_between_games() {
    let mut game = Game::new_duel(7);
    concede_a_game(&mut game);
    assert_eq!(game.phase(), Phase::BetweenGames);

    // The beat lasts a moment, then the next game serves.
    let mut resumed = false;
    for _ in 0..(2 * (1.0 / pong_remix_core::TIMESTEP) as usize + 240) {
        game.step(Input::default());
        if matches!(game.phase(), Phase::Serving | Phase::Rally) {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "the next game never started after the beat");
    assert_eq!(game.score(Side::Right), 0, "the next game starts from love");
    assert_eq!(
        game.games_won(Side::Right),
        1,
        "games already won carry into the next game"
    );
}

#[test]
fn the_match_is_won_at_three_games() {
    let mut game = Game::new_duel(7);
    for _ in 0..5 {
        if let Phase::MatchOver { .. } = game.phase() {
            break;
        }
        // Concede a whole game, then let the beat pass.
        concede_a_game(&mut game);
        for _ in 0..(2 * (1.0 / pong_remix_core::TIMESTEP) as usize + 240) {
            if !matches!(game.phase(), Phase::BetweenGames) {
                break;
            }
            game.step(Input::default());
        }
    }

    assert_eq!(
        game.phase(),
        Phase::MatchOver {
            winner: Side::Right
        }
    );
    assert_eq!(
        game.games_won(Side::Right),
        3,
        "the match is a best-of-five"
    );
}

#[test]
fn a_duel_game_plays_the_normal_versus_rules() {
    // Points still work exactly as in a single Versus game.
    let mut game = Game::new_duel(7);
    assert_eq!(play_a_point(&mut game, Side::Left), Side::Right);
    assert_eq!(game.score(Side::Right), 1);
}

#[test]
fn a_one_player_duel_gives_the_right_paddle_to_the_computer() {
    let mut cpu = Game::new_duel_cpu(7);
    let mut two = Game::new_duel(7);
    let mut diverged = false;
    for _ in 0..900 {
        cpu.step(Input {
            left: Axis::Hold,
            ..Default::default()
        });
        two.step(Input {
            left: Axis::Hold,
            ..Default::default()
        });
        diverged |= cpu.paddle(Side::Right).y != two.paddle(Side::Right).y;
    }
    assert!(
        diverged,
        "the computer never took the right paddle in a Duel"
    );
}
