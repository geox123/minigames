//! Spin: moving the paddle as you strike curves the ball's flight.

mod common;

use common::{speed, strike_moving};
use pong_remix_core::{Axis, Game, Input, LOGICAL_WIDTH, Side};

/// How the ball's vertical velocity changes over a short free flight right after
/// a strike made while the paddle was moving in `dir`. Negative means it curved
/// upward, positive downward.
fn curve_after_strike(dir: Axis) -> f32 {
    let mut game = Game::new(7);
    let struck = strike_moving(&mut game, Side::Left, dir);
    let vy0 = struck.vy;
    for _ in 0..14 {
        let events = game.step(Input::default());
        if events.wall_bounce || events.paddle_hit || events.scored.is_some() {
            break;
        }
    }
    game.ball().vy - vy0
}

#[test]
fn paddle_motion_curves_the_ball() {
    let up = curve_after_strike(Axis::Up);
    let down = curve_after_strike(Axis::Down);

    assert!(
        up < -1.0,
        "an upward-moving strike should curve the ball upward, got Δvy = {up}"
    );
    assert!(
        down > 1.0,
        "a downward-moving strike should curve the ball downward, got Δvy = {down}"
    );
}

#[test]
fn spin_curves_without_changing_speed() {
    let mut game = Game::new(7);
    let struck = strike_moving(&mut game, Side::Left, Axis::Up);
    let opening = speed(struck);

    for _ in 0..14 {
        let events = game.step(Input::default());
        if events.wall_bounce || events.paddle_hit || events.scored.is_some() {
            break;
        }
    }

    assert!(
        (speed(game.ball()) - opening).abs() < 2.0,
        "spin should curve the ball, not speed it up: {} vs {opening}",
        speed(game.ball())
    );
}

#[test]
fn a_spun_ball_stays_returnable() {
    // Even a hard up-strike keeps enough horizontal speed to cross the field —
    // spin bends the shot but never traps the ball against a wall.
    let mut game = Game::new(7);
    strike_moving(&mut game, Side::Left, Axis::Up);

    for _ in 0..600 {
        game.step(Input::default());
        let ball = game.ball();
        let speed = speed(ball);
        if speed > 0.0 {
            assert!(
                ball.vx.abs() >= speed * 0.3,
                "the ball lost too much horizontal speed: vx = {}, speed = {speed}",
                ball.vx
            );
        }
        // Reaching the far half proves it kept crossing rather than stalling.
        if ball.x > LOGICAL_WIDTH * 0.6 {
            return;
        }
    }
    panic!("the spun ball never crossed to the far half");
}
