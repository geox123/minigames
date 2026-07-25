//! The well's pull — via the seam.

mod common;

use common::{game, speed, still};

#[test]
fn the_well_pulls_a_still_ship_in() {
    let mut game = game(1);
    let start = game.ship();
    for _ in 0..60 {
        game.step(still());
    }
    let now = game.ship();
    assert!(now.y > start.y, "the well pulls the ship toward the centre");
    assert!(
        speed(now) > speed(start),
        "and it gathers speed as it falls"
    );
}

#[test]
fn the_pull_accelerates_the_ship_on_approach() {
    // Falling straight in from rest, the ship speeds up every step of the approach,
    // and by more as it nears the well — an inverse-square pull growing with 1/d².
    let mut game = game(1);
    let mut last = speed(game.ship());
    let mut first_gain = None;
    let mut last_gain = 0.0;
    for _ in 0..80 {
        game.step(still());
        let now = speed(game.ship());
        let gain = now - last;
        assert!(gain > 0.0, "the pull keeps adding speed on the way in");
        first_gain.get_or_insert(gain);
        last_gain = gain;
        last = now;
    }
    assert!(
        last_gain > first_gain.unwrap(),
        "and the pull strengthens as the ship nears the well"
    );
}
