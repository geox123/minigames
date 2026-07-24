//! Firing: the cannon's shot, one at a time, destroying invaders — via the seam.

mod common;

use common::{MAX_STEPS, game, shooting};
use stepfall_core::Move;

#[test]
fn firing_launches_a_shot_that_climbs() {
    let mut game = game(1);
    game.step(shooting(Move::Hold));
    let launched = game.shot().expect("firing launches a shot");

    // It climbs on the next interrupt.
    game.step(shooting(Move::Hold));
    game.step(shooting(Move::Hold));
    let later = game.shot().expect("the shot is still in flight");
    assert!(later.y < launched.y, "the shot climbs the screen");
}

#[test]
fn a_shot_destroys_an_invader_and_scores() {
    let mut game = game(1);
    let before = game.alive();

    let mut killed = false;
    for _ in 0..MAX_STEPS {
        game.step(shooting(Move::Hold));
        if game.alive() < before {
            killed = true;
            break;
        }
    }
    assert!(killed, "a held shot never reached the formation");
    assert!(game.score() > 0, "destroying an invader scores");
}

#[test]
fn only_one_shot_is_in_flight_at_a_time() {
    let mut game = game(1);
    game.step(shooting(Move::Hold));
    let launched = game.shot().expect("the first shot launched").y;

    // Hold fire: the one shot keeps climbing, and is not relaunched from the
    // cannon while it flies.
    for _ in 0..10 {
        game.step(shooting(Move::Hold));
    }
    let now = game.shot().expect("the shot is still climbing").y;
    assert!(
        now < launched,
        "holding fire kept the single shot climbing, not relaunched from the cannon"
    );
}
