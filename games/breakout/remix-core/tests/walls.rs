//! Losing a ball is reported and a fresh ball is served — via the seam. The
//! wall-clear transition itself is impractical to reach by honest play (a
//! perfect paddle digs a channel the ball bounces in forever), so it is covered
//! by a white-box unit test in the crate's `lib.rs`.

mod common;

use breakout_remix_core::{Input, Move, Phase};
use common::{MAX_STEPS, run};

#[test]
fn dropping_the_ball_is_reported_and_a_new_ball_is_served() {
    let mut game = run(5);

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
        if game.step(Input { paddle }).lost_ball {
            lost = true;
            assert_eq!(
                game.phase(),
                Phase::Serving,
                "a lost ball re-parks on the paddle to serve"
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
