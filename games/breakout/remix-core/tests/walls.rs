//! Losing balls, spending lives, and the run ending — via the seam. Winning and
//! the wall/guardian transitions are impractical to reach by honest play (a
//! perfect paddle digs a channel the ball bounces in forever), so those are
//! covered by white-box unit tests in the crate's `lib.rs`.

mod common;

use breakout_remix_core::{Input, LIVES_START, Move, Phase};
use common::{MAX_STEPS, run};

#[test]
fn dropping_the_ball_spends_a_life_and_serves_a_new_one() {
    let mut game = run(5);
    let lives_before = game.lives();
    assert_eq!(lives_before, LIVES_START, "a run opens with its full lives");

    // Dodge: steer the paddle away from the ball so it falls past.
    let mut lost = false;
    for _ in 0..MAX_STEPS {
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let paddle = if ball.x < centre {
            Move::Right
        } else {
            Move::Left
        };
        if game
            .step(Input {
                paddle,
                ..Default::default()
            })
            .lost_ball
        {
            lost = true;
            assert_eq!(game.lives(), lives_before - 1, "a lost ball spends a life");
            assert_eq!(
                game.phase(),
                Phase::Serving,
                "with lives left, a lost ball re-parks to serve"
            );
            break;
        }
    }
    assert!(lost, "the ball never dropped past the dodging paddle");

    // The new ball serves and play resumes.
    let mut resumed = false;
    for _ in 0..MAX_STEPS {
        game.step(Input::default());
        if game.phase() == Phase::Playing {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "a new ball was never served after the drop");
}

#[test]
fn spending_every_life_ends_the_run() {
    let mut game = run(6);

    let mut drops = 0;
    let mut lost_run = false;
    for _ in 0..(MAX_STEPS * 2) {
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let paddle = if ball.x < centre {
            Move::Right
        } else {
            Move::Left
        };
        let events = game.step(Input {
            paddle,
            ..Default::default()
        });
        if events.lost_ball {
            drops += 1;
        }
        if game.phase() == Phase::Lost {
            assert!(events.lost, "the losing drop is reported");
            lost_run = true;
            break;
        }
    }

    assert!(lost_run, "dodging every ball never ended the run");
    assert_eq!(game.lives(), 0, "no lives remain once the run is lost");
    assert_eq!(drops, LIVES_START, "the run lasted exactly its lives");
    assert!(
        game.depth() >= 1,
        "a depth reached is available for the summary"
    );
}
