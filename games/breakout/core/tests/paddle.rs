//! The paddle, the serve, deflection, and losing a turn — through the seam only.

use breakout_core::{BALL_SIZE, Game, Input, LOGICAL_WIDTH, Move, Phase, TIMESTEP, TURNS};

const LONG: usize = (8.0 / TIMESTEP) as usize;

fn hold(game: &mut Game, mv: Move, steps: usize) {
    for _ in 0..steps {
        game.step(Input { paddle: mv });
    }
}

#[test]
fn the_paddle_moves_and_clamps_to_the_field() {
    let mut game = Game::new(7);
    let start = game.paddle().x;

    hold(&mut game, Move::Right, 20);
    assert!(game.paddle().x > start, "paddle did not move right");

    hold(&mut game, Move::Right, LONG);
    let paddle = game.paddle();
    assert!(
        (paddle.x + paddle.width - LOGICAL_WIDTH).abs() < 0.5,
        "paddle did not stop at the right wall (x={}, w={})",
        paddle.x,
        paddle.width
    );

    hold(&mut game, Move::Left, LONG);
    assert!(
        game.paddle().x.abs() < 0.5,
        "paddle did not stop at the left wall"
    );
}

#[test]
fn the_ball_serves_from_the_paddle() {
    let mut game = Game::new(7);
    // Before the serve the ball rests just above the paddle and does not move.
    assert_eq!(game.phase(), Phase::Serving);
    let resting = game.ball();
    game.step(Input::default());
    assert_eq!(game.ball().y, resting.y, "ball moved before it was served");

    // After the serve pause it launches upward.
    for _ in 0..LONG {
        game.step(Input::default());
        if game.ball().vy != 0.0 {
            break;
        }
    }
    assert!(game.ball().vy < 0.0, "the ball should launch upward");
    assert_eq!(game.phase(), Phase::Playing);
}

/// Plays until the paddle strikes the ball with its centre `offset` from the
/// ball, returning the ball as it comes off.
fn strike(game: &mut Game, offset: f32) -> breakout_core::Ball {
    for _ in 0..40_000 {
        let ball = game.ball();
        // Track the ball, offset, so it lands off-centre on the paddle.
        let target = ball.x + offset;
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let mv = if centre < target - 2.0 {
            Move::Right
        } else if centre > target + 2.0 {
            Move::Left
        } else {
            Move::Hold
        };
        if game.step(Input { paddle: mv }).paddle_hit {
            return game.ball();
        }
    }
    panic!("the paddle never struck the ball");
}

#[test]
fn where_the_ball_strikes_the_paddle_decides_where_it_goes() {
    // A positive offset puts the paddle's centre to the right of the ball, so
    // the ball lands left of centre and is sent left; a negative offset sends it
    // right.
    let left = strike(&mut Game::new(7), 12.0);
    let right = strike(&mut Game::new(7), -12.0);
    assert!(
        left.vx < 0.0,
        "a strike left of centre should go left, got {}",
        left.vx
    );
    assert!(
        right.vx > 0.0,
        "a strike right of centre should go right, got {}",
        right.vx
    );
    // Both come back up the field.
    assert!(
        left.vy < 0.0 && right.vy < 0.0,
        "a struck ball should fly upward"
    );
}

#[test]
fn a_ball_past_the_paddle_costs_a_turn_and_runs_out_the_game() {
    let mut game = Game::new(7);
    assert_eq!(game.turns(), TURNS);

    // Never move the paddle from the wall; the ball is served to the middle and
    // falls past. Each miss costs a turn until the game is over.
    let mut seen_game_over = false;
    for _ in 0..(TURNS as usize + 2) * LONG {
        hold(&mut game, Move::Left, 1);
        if game.phase() == Phase::GameOver {
            seen_game_over = true;
            break;
        }
    }
    assert!(
        seen_game_over,
        "the game never ended after every turn was lost"
    );
    assert_eq!(game.turns(), 0);
}

#[test]
fn the_ball_stays_in_the_field_while_in_play() {
    let mut game = Game::new(7);
    let half = BALL_SIZE / 2.0;
    for _ in 0..LONG {
        // Keep the paddle under the ball so play continues.
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let mv = if centre < ball.x - 2.0 {
            Move::Right
        } else if centre > ball.x + 2.0 {
            Move::Left
        } else {
            Move::Hold
        };
        game.step(Input { paddle: mv });
        let ball = game.ball();
        assert!(ball.x - half >= -0.01 && ball.x + half <= LOGICAL_WIDTH + 0.01);
        assert!(
            ball.y - half >= -0.01,
            "ball escaped through the top at y={}",
            ball.y
        );
    }
}
