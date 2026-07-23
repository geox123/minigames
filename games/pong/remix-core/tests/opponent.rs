//! The computer opponent: it has to keep a rally up, cope with the Remix's
//! mechanics, and stay beatable.

mod common;

use common::axis_towards;
use pong_remix_core::{Game, Input, LOGICAL_HEIGHT, PADDLE_HEIGHT, Phase, Side};

/// Input tracking the ball with the left paddle, right paddle idle.
fn track_left(game: &Game) -> Input {
    Input {
        left: axis_towards(game.paddle(Side::Left).y, game.ball().y),
        ..Default::default()
    }
}

#[test]
fn the_computer_drives_the_right_paddle_in_a_one_player_game() {
    let mut cpu = Game::new_versus_cpu(7);
    let mut two = Game::new(7);
    let mut diverged = false;
    // Neither game gets any input; only the computer moves a paddle on its own.
    for _ in 0..900 {
        cpu.step(Input::default());
        two.step(Input::default());
        diverged |= cpu.paddle(Side::Right).y != two.paddle(Side::Right).y;
    }
    assert!(diverged, "the computer never moved the right paddle");
}

#[test]
fn the_computer_keeps_a_rally_going() {
    let mut game = Game::new_versus_cpu(7);
    let mut returns = 0;
    // Over a long match the opponent also meets spin, pickups and multiball; it
    // returning the ball many times shows it copes with all of them.
    for _ in 0..30_000 {
        if matches!(game.phase(), Phase::GameOver { .. }) {
            game.restart();
        }
        let approaching = game.ball().vx > 0.0;
        if game.step(track_left(&game)).paddle_hit && approaching {
            returns += 1;
        }
    }
    assert!(
        returns > 15,
        "the opponent only returned the ball {returns} times in a long match"
    );
}

#[test]
fn the_opponent_copes_with_power_shots() {
    let mut game = Game::new_versus_cpu(7);
    let mut returns = 0;
    for _ in 0..30_000 {
        if matches!(game.phase(), Phase::GameOver { .. }) {
            game.restart();
        }
        // The left player charges constantly, firing fast power shots at the
        // computer, which must still return some of them.
        let input = Input {
            left: axis_towards(game.paddle(Side::Left).y, game.ball().y),
            charge_left: true,
            ..Default::default()
        };
        let approaching = game.ball().vx > 0.0;
        if game.step(input).paddle_hit && approaching {
            returns += 1;
        }
    }
    assert!(
        returns > 8,
        "the opponent couldn't handle power shots, returning only {returns}"
    );
}

/// Whether a sharp player — perfect tracking, aiming for the corner the
/// opponent is furthest from — beats the computer for this seed.
fn sharp_player_beats_opponent(seed: u64) -> bool {
    let mut game = Game::new_versus_cpu(seed);
    for _ in 0..200_000 {
        if let Phase::GameOver { winner } = game.phase() {
            return winner == Side::Left;
        }
        let opponent = game.paddle(Side::Right).y + PADDLE_HEIGHT / 2.0;
        let aim = if opponent < LOGICAL_HEIGHT / 2.0 {
            -18.0
        } else {
            18.0
        };
        let input = Input {
            left: axis_towards(game.paddle(Side::Left).y, game.ball().y + aim),
            ..Default::default()
        };
        game.step(input);
    }
    false
}

#[test]
fn the_opponent_is_beatable() {
    let seeds = [1u64, 2, 3, 7, 11, 42, 99, 12345];
    let wins = seeds
        .iter()
        .filter(|&&s| sharp_player_beats_opponent(s))
        .count();
    assert!(
        wins >= 2,
        "a sharp player should beat the opponent in a fair share of seeds, but won only {wins}/{}",
        seeds.len()
    );
}
