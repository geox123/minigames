//! Helpers shared by PULSE's core tests. Everything drives the game the way a
//! player does — through the public seam, by holding keys.

#![allow(dead_code)]

use pong_remix_core::{
    Axis, Ball, Game, Input, LOGICAL_HEIGHT, PADDLE_HEIGHT, PADDLE_SPEED, PADDLE_TOP_GAP, Side,
    TIMESTEP,
};

/// A generous ceiling on how long a test plays before giving up.
pub const MAX_STEPS: usize = 40_000;

/// The speed of the ball, ignoring direction.
pub fn speed(ball: Ball) -> f32 {
    ball.vx.hypot(ball.vy)
}

/// The key a player would hold to bring their paddle's centre to `target`.
pub fn axis_towards(paddle_top: f32, target: f32) -> Axis {
    let centre = paddle_top + PADDLE_HEIGHT / 2.0;
    let slack = PADDLE_SPEED * TIMESTEP;
    if centre < target - slack {
        Axis::Down
    } else if centre > target + slack {
        Axis::Up
    } else {
        Axis::Hold
    }
}

/// Input for two players who both follow the ball, each holding their paddle's
/// centre `offset` units from it.
pub fn tracking(game: &Game, left_offset: f32, right_offset: f32) -> Input {
    let ball = game.ball();
    Input {
        left: axis_towards(game.paddle(Side::Left).y, ball.y + left_offset),
        right: axis_towards(game.paddle(Side::Right).y, ball.y + right_offset),
    }
}

/// Whether a paddle can actually centre itself on `target`.
pub fn reachable(target: f32) -> bool {
    let half = PADDLE_HEIGHT / 2.0;
    target >= PADDLE_TOP_GAP + half && target <= LOGICAL_HEIGHT - half
}

/// Plays until `side`'s paddle strikes the ball with `offset` between the ball
/// and the paddle's centre, returning the ball as it comes off.
pub fn strike(game: &mut Game, side: Side, offset: f32) -> Ball {
    for _ in 0..MAX_STEPS {
        let before = game.ball();
        let input = match side {
            Side::Left => tracking(game, offset, 0.0),
            Side::Right => tracking(game, 0.0, offset),
        };
        let approaching = match side {
            Side::Left => before.vx < 0.0,
            Side::Right => before.vx > 0.0,
        };
        if game.step(input).paddle_hit && approaching && reachable(before.y + offset) {
            return game.ball();
        }
    }
    panic!("the {side:?} paddle never cleanly struck the ball");
}

/// Input where one player follows the ball while `loser` parks at the bottom.
pub fn conceding(game: &Game, loser: Side) -> Input {
    let mut input = tracking(game, 0.0, 0.0);
    match loser {
        Side::Left => input.left = Axis::Down,
        Side::Right => input.right = Axis::Down,
    }
    input
}

/// Plays until a point is scored against `loser`, returning the scorer.
pub fn play_a_point(game: &mut Game, loser: Side) -> Side {
    for _ in 0..MAX_STEPS {
        let input = conceding(game, loser);
        if let Some(scorer) = game.step(input).scored {
            return scorer;
        }
    }
    panic!("no point was scored against the {loser:?} player");
}

/// Waits out the serve pause and reports the serve direction (-1 left, 1 right).
pub fn serve_direction(game: &mut Game) -> f32 {
    for _ in 0..MAX_STEPS {
        game.step(Input::default());
        let ball = game.ball();
        if ball.vx != 0.0 {
            return ball.vx.signum();
        }
    }
    panic!("the ball was never served");
}
