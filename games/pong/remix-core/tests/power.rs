//! Power shots: charging slows the paddle, a charged return flies fast, and a
//! cooldown follows.

mod common;

use common::{axis_towards, speed};
use pong_remix_core::{Axis, BALL_SPEED, Game, Input, Side};

/// Holds the left player charging while it tracks and returns the ball, so its
/// next connecting return is a power shot. Returns the ball off that shot and
/// whether it was reported as a power hit.
fn charged_return(game: &mut Game) -> (f32, bool) {
    for _ in 0..common::MAX_STEPS {
        let ball = game.ball();
        let input = Input {
            left: axis_towards(game.paddle(Side::Left).y, ball.y),
            right: axis_towards(game.paddle(Side::Right).y, ball.y),
            charge_left: true,
            ..Default::default()
        };
        let approaching = ball.vx < 0.0;
        let events = game.step(input);
        if events.paddle_hit && approaching {
            return (speed(game.ball()), events.power_hit);
        }
    }
    panic!("the left player never landed a charged return");
}

#[test]
fn a_charged_return_is_a_fast_power_shot() {
    let mut game = Game::new(7);
    let (speed_off_paddle, was_power) = charged_return(&mut game);

    assert!(
        was_power,
        "a fully-charged return should be reported a power hit"
    );
    assert!(
        speed_off_paddle > BALL_SPEED + 20.0,
        "a power shot should be clearly faster than a normal return: {speed_off_paddle} vs {BALL_SPEED}"
    );
}

/// How far the left paddle travels over `steps` while holding Up, optionally
/// charging.
fn upward_travel(charging: bool, steps: usize) -> f32 {
    let mut game = Game::new(7);
    let start = game.paddle(Side::Left).y;
    for _ in 0..steps {
        game.step(Input {
            left: Axis::Up,
            charge_left: charging,
            ..Default::default()
        });
    }
    (start - game.paddle(Side::Left).y).abs()
}

#[test]
fn charging_slows_the_paddle() {
    let free = upward_travel(false, 30);
    let charging = upward_travel(true, 30);

    assert!(
        charging < free * 0.8,
        "charging should slow the paddle: charged {charging} vs free {free}"
    );
}

#[test]
fn a_power_shot_leaves_the_paddle_cooling_down() {
    let mut game = Game::new(7);
    // Land a power shot, which starts the left paddle's cooldown.
    charged_return(&mut game);

    assert!(
        game.cooling(Side::Left),
        "the paddle should be cooling down right after a power shot"
    );

    // While cooling, the paddle moves slower than a fresh one over the same span.
    let start = game.paddle(Side::Left).y;
    for _ in 0..20 {
        game.step(Input {
            left: Axis::Down,
            ..Default::default()
        });
    }
    let cooling_travel = (game.paddle(Side::Left).y - start).abs();
    let free_travel = upward_travel(false, 20); // same distance, fresh paddle

    assert!(
        cooling_travel < free_travel * 0.8,
        "a cooling paddle should move slower: {cooling_travel} vs {free_travel}"
    );
}

#[test]
fn charge_builds_while_held_and_resets_when_let_go() {
    let mut game = Game::new(7);
    assert_eq!(game.charge(Side::Left), 0.0);

    for _ in 0..40 {
        game.step(Input {
            charge_left: true,
            ..Default::default()
        });
    }
    let built = game.charge(Side::Left);
    assert!(built > 0.0, "charge should build while held");

    game.step(Input::default());
    assert_eq!(
        game.charge(Side::Left),
        0.0,
        "letting go should spend the charge"
    );
}
