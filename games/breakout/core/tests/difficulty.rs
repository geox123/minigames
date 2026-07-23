//! The faithful rising difficulty: the ball speeds up, the paddle shrinks.
//! All observed through the step() seam — the speed is read off the ball's
//! velocity, the shrink off the paddle's width.

use breakout_core::{Ball, Game, Input, Move, PADDLE_WIDTH};

/// The ball's speed: the magnitude of its velocity.
fn speed(ball: Ball) -> f32 {
    (ball.vx * ball.vx + ball.vy * ball.vy).sqrt()
}

/// The paddle move that keeps the paddle under the ball, so a rally endures.
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
fn the_ball_speeds_up_after_the_fourth_and_twelfth_return() {
    let mut game = Game::new(7);

    // Play a defended rally, noting the ball's speed as it comes off the paddle
    // on each return.
    let mut speed_on_return = Vec::new();
    for _ in 0..80_000 {
        let mv = track(&game);
        if game.step(Input { paddle: mv }).paddle_hit {
            speed_on_return.push(speed(game.ball()));
            if speed_on_return.len() >= 12 {
                break;
            }
        }
    }
    assert!(
        speed_on_return.len() >= 12,
        "the rally ended before 12 returns ({} seen)",
        speed_on_return.len()
    );

    // Speed never falls across the rally — difficulty only rises.
    for pair in speed_on_return.windows(2) {
        assert!(pair[1] >= pair[0] - 0.01, "the ball slowed down mid-rally");
    }
    // The 4th and 12th returns each ratchet the speed up. (Indices are 0-based,
    // so the 4th return is index 3, the 12th index 11.)
    assert!(
        speed_on_return[3] > speed_on_return[2] + 0.01,
        "the ball did not speed up on the 4th return"
    );
    assert!(
        speed_on_return[11] > speed_on_return[10] + 0.01,
        "the ball did not speed up on the 12th return"
    );
}

#[test]
fn reaching_a_high_band_speeds_the_ball_up() {
    let mut game = Game::new(5);

    // Play until the ball breaks its first high-band (orange) brick, watching
    // the speed before and after that break. Bands run 0 (low) to 3 (high); a
    // brick in band 2 or 3 is a high band.
    let mut sped_up_on_high_band = false;
    for _ in 0..80_000 {
        let mv = track(&game);
        let before_step = speed(game.ball());
        let events = game.step(Input { paddle: mv });
        if let Some(band) = events.brick_broken
            && band >= 2
            && speed(game.ball()) > before_step + 0.01
        {
            // The very step a high-band brick falls, the ball is faster than it
            // was going into the step.
            sped_up_on_high_band = true;
            break;
        }
    }
    assert!(
        sped_up_on_high_band,
        "breaking into a high band never sped the ball up"
    );
}

#[test]
fn the_paddle_halves_the_first_time_the_ball_reaches_the_top() {
    let mut game = Game::new(9);
    assert_eq!(
        game.paddle().width,
        PADDLE_WIDTH,
        "the paddle should start at full width"
    );

    // Play a long defended rally. Eventually the ball punches a column clear
    // and strikes the top wall, at which point the paddle halves for good.
    let half = breakout_core::BALL_SIZE / 2.0;
    let mut reached_top = false;
    for _ in 0..200_000 {
        let mv = track(&game);
        game.step(Input { paddle: mv });
        if game.ball().y <= half + 0.01 {
            reached_top = true;
            // The paddle has already halved by the time we observe the top hit.
            assert!(
                (game.paddle().width - PADDLE_WIDTH / 2.0).abs() < 0.01,
                "the paddle did not halve on reaching the top (width {})",
                game.paddle().width
            );
            break;
        }
        // Until the top is reached the paddle stays full width.
        assert_eq!(
            game.paddle().width,
            PADDLE_WIDTH,
            "the paddle shrank before the ball reached the top"
        );
    }
    assert!(reached_top, "the ball never reached the top wall");

    // Once halved it never grows back over a further stretch of play.
    for _ in 0..20_000 {
        let mv = track(&game);
        game.step(Input { paddle: mv });
        assert!(
            (game.paddle().width - PADDLE_WIDTH / 2.0).abs() < 0.01,
            "the paddle changed width after halving"
        );
    }
}
