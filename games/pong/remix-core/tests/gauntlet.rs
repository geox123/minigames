//! Gauntlet: solo survival that escalates, is seeded, and ends when the goal
//! is breached.

mod common;

use common::axis_towards;
use pong_remix_core::{Axis, Game, Input, LOGICAL_HEIGHT, Phase, Side};

/// Input defending the left goal by tracking the nearest incoming ball.
fn defend(game: &Game) -> Input {
    // Track whichever ball is heading left and furthest along toward the goal.
    let target = game
        .balls()
        .filter(|b| b.vx < 0.0)
        .min_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        .map(|b| b.y)
        .unwrap_or(LOGICAL_HEIGHT / 2.0);
    Input {
        left: axis_towards(game.paddle(Side::Left).y, target),
        ..Default::default()
    }
}

#[test]
fn a_gauntlet_run_ends_when_the_goal_is_breached() {
    // A player who never moves lets the first ball straight past.
    let mut game = Game::new_gauntlet(7);
    let mut ended = false;
    for _ in 0..common::MAX_STEPS {
        game.step(Input {
            left: Axis::Down,
            ..Default::default()
        });
        if game.phase() == Phase::RunOver {
            ended = true;
            break;
        }
    }
    assert!(ended, "the run never ended though the goal was abandoned");
}

#[test]
fn defending_scores_and_survives_longer_than_conceding() {
    let mut defender = Game::new_gauntlet(7);
    let mut idle = Game::new_gauntlet(7);

    let defender_steps = run_length(&mut defender, true);
    let idle_steps = run_length(&mut idle, false);

    assert!(
        defender_steps > idle_steps,
        "defending ({defender_steps}) should outlast conceding ({idle_steps})"
    );
    assert!(
        defender.gauntlet_score() > 0,
        "a defended run should score above zero"
    );
}

/// Plays a Gauntlet run to its end, returning how many steps it lasted. If
/// `defend_goal`, the player tracks the balls; otherwise it abandons the goal.
fn run_length(game: &mut Game, defend_goal: bool) -> usize {
    for step in 0..common::MAX_STEPS {
        if game.phase() == Phase::RunOver {
            return step;
        }
        let input = if defend_goal {
            defend(game)
        } else {
            Input {
                left: Axis::Up,
                ..Default::default()
            }
        };
        game.step(input);
    }
    common::MAX_STEPS
}

#[test]
fn the_barrage_escalates_to_several_balls() {
    let mut game = Game::new_gauntlet(7);
    let mut max_balls = 1;
    for _ in 0..common::MAX_STEPS {
        if game.phase() == Phase::RunOver {
            break;
        }
        game.step(defend(&game));
        max_balls = max_balls.max(game.balls().count());
    }
    assert!(
        max_balls >= 2,
        "the Gauntlet should add balls over time, but peaked at {max_balls}"
    );
}

#[test]
fn a_gauntlet_run_is_deterministic_from_its_seed() {
    assert_eq!(
        deterministic_run(42),
        deterministic_run(42),
        "same seed gave different runs"
    );
    assert_ne!(
        deterministic_run(1),
        deterministic_run(2),
        "different seeds gave identical runs"
    );
}

/// Plays a defended Gauntlet run and samples the ball as it goes. The defence
/// keeps the run long enough to diverge by seed, while staying a pure function
/// of the (seed-determined) game state — so a repeat of the same seed replays.
fn deterministic_run(seed: u64) -> Vec<(f32, f32, f32, f32)> {
    let mut game = Game::new_gauntlet(seed);
    let mut samples = Vec::new();
    for step in 0..8_000 {
        if game.phase() == Phase::RunOver {
            break;
        }
        game.step(defend(&game));
        if step % 30 == 0 {
            let ball = game.ball();
            samples.push((ball.x, ball.y, ball.vx, ball.vy));
        }
    }
    samples
}

#[test]
fn a_gauntlet_run_can_be_restarted() {
    let mut game = Game::new_gauntlet(7);
    for _ in 0..common::MAX_STEPS {
        game.step(Input {
            left: Axis::Down,
            ..Default::default()
        });
        if game.phase() == Phase::RunOver {
            break;
        }
    }
    assert_eq!(game.phase(), Phase::RunOver);

    game.restart();
    assert_ne!(game.phase(), Phase::RunOver);
    assert_eq!(game.gauntlet_score(), 0, "a restarted run starts from zero");
}
