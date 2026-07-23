//! Restarting a finished game, through the step() seam. (Wall progression and
//! the win are driven from inside the core — see the unit tests in the crate —
//! because emptying a full wall by honest play is not practical.)

use breakout_core::{BRICK_COLS, BRICK_ROWS, Game, Input, Move, Phase, TURNS};

fn track(game: &Game) -> Move {
    let ball = game.ball();
    let centre = game.paddle().x + game.paddle().width / 2.0;
    if centre < ball.x - 2.0 {
        Move::Right
    } else if centre > ball.x + 2.0 {
        Move::Left
    } else {
        Move::Hold
    }
}

#[test]
fn a_finished_game_restarts_from_the_beginning() {
    let mut game = Game::new(7);

    // Play a while, breaking some bricks and scoring, then stop defending so all
    // three turns are lost and the game ends.
    let mut scored = false;
    for _ in 0..40_000 {
        // Track until we've broken a few bricks, then hold at the wall to die.
        let mv = if game.score() > 0 && scored {
            Move::Left
        } else {
            track(&game)
        };
        if game.step(Input { paddle: mv }).brick_broken.is_some() {
            scored = true;
        }
        if game.phase() == Phase::GameOver {
            break;
        }
    }
    assert_eq!(game.phase(), Phase::GameOver, "the game should have ended");
    assert!(game.score() > 0, "the run should have scored before ending");
    assert!(
        game.bricks_left() < (BRICK_ROWS * BRICK_COLS) as u32,
        "the run should have broken some bricks"
    );

    // Restart wipes the slate: score, turns, walls and the whole wall reset, and
    // the ball waits to be served again.
    game.restart();
    assert_eq!(game.phase(), Phase::Serving);
    assert_eq!(game.score(), 0);
    assert_eq!(game.turns(), TURNS);
    assert_eq!(game.walls_cleared(), 0);
    assert_eq!(game.bricks_left(), (BRICK_ROWS * BRICK_COLS) as u32);
}
