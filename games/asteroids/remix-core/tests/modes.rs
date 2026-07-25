//! The modes through the seam. A run opens with no outcome, and a run that spends its
//! last life resolves as `Lost` — both reachable by honest play. Winning Orbit and the
//! endless-boss behaviour need a felled boss, which a scripted test cannot stage, so
//! those live in the crate's white-box tests.

mod common;

use asteroids_remix_core::{Game, Input, Loadout, Mode, Outcome, Phase};
use common::{game, still};

#[test]
fn a_new_run_has_no_outcome_yet() {
    for mode in [Mode::Orbit, Mode::Maelstrom, Mode::Daily] {
        let game = Game::new(1, mode, Loadout::default());
        assert!(
            game.outcome().is_none(),
            "a fresh {mode:?} run is unresolved"
        );
    }
}

#[test]
fn spending_the_last_life_loses_the_run() {
    // Doing nothing, the well pulls the ship in life after life until the run is over —
    // and it resolves as Lost.
    let mut game = game(1);
    let mut ended = false;
    for _ in 0..6000 {
        game.step(still());
        if game.phase() == Phase::Over {
            ended = true;
            break;
        }
    }
    assert!(ended, "an idle run eventually ends");
    assert_eq!(game.outcome(), Some(Outcome::Lost), "and it is a loss");
}

#[test]
fn the_daily_run_replays_for_a_shared_seed() {
    // Daily hands the core the day's seed; the same seed replays the same run, so
    // everyone facing a given day faces the same field.
    let script = |i: usize| Input {
        thrust: i.is_multiple_of(2),
        fire: i.is_multiple_of(3),
        ..Default::default()
    };
    let mut a = Game::new(20260725, Mode::Daily, Loadout::default());
    let mut b = Game::new(20260725, Mode::Daily, Loadout::default());
    for i in 0..3000 {
        a.step(script(i));
        b.step(script(i));
    }
    assert_eq!(a.ship(), b.ship());
    assert_eq!(a.score(), b.score());
    assert_eq!(a.stage(), b.stage());
}
