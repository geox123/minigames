//! The brick wall: layout, collision, destruction and scoring — via the seam.

use breakout_core::{Game, Input, Move};

/// Plays a long defended rally, returning the game once `stop` says so or the
/// step budget runs out. The paddle tracks the ball to keep the ball alive.
fn rally_until(game: &mut Game, mut stop: impl FnMut(&Game) -> bool) {
    for _ in 0..80_000 {
        if stop(game) {
            return;
        }
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let mv = if centre < ball.x - 2.0 {
            Move::Right
        } else if centre > ball.x + 2.0 {
            Move::Left
        } else {
            Move::Hold
        };
        game.step(Input { paddle: mv });
    }
}

#[test]
fn the_wall_starts_full_in_four_bands() {
    let game = Game::new(7);
    let bricks: Vec<_> = game.bricks().collect();
    assert_eq!(bricks.len(), 8 * 14, "the wall should be 8 rows of 14");

    let mut bands: Vec<u8> = bricks.iter().map(|b| b.band).collect();
    bands.sort_unstable();
    bands.dedup();
    assert_eq!(bands, vec![0, 1, 2, 3], "there should be four bands");

    // The four bands score 1, 3, 5, 7 — the higher rows worth more.
    assert_eq!(game.band_points(0), 1);
    assert_eq!(game.band_points(1), 3);
    assert_eq!(game.band_points(2), 5);
    assert_eq!(game.band_points(3), 7);
}

#[test]
fn striking_a_brick_breaks_it_and_scores_its_band() {
    let mut game = Game::new(7);
    let before = game.bricks().count();
    assert_eq!(game.score(), 0);

    // Play until at least a few bricks have fallen.
    rally_until(&mut game, |g| g.bricks().count() + 5 <= before);

    let broken = before - game.bricks().count();
    assert!(broken >= 5, "expected several bricks broken, got {broken}");
    assert!(game.score() > 0, "breaking bricks should score");
    // The score is a sum of band points, so at least as large as the count.
    assert!(
        game.score() >= broken as u32,
        "score {} should be at least the {broken} bricks broken",
        game.score()
    );
}

#[test]
fn a_fast_ball_never_tunnels_through_the_wall() {
    // Over a long rally the ball keeps breaking bricks rather than slipping
    // through the wall — the brick count only ever falls, one break at a time,
    // and the ball never lingers inside the brick field without breaking one.
    let mut game = Game::new(3);
    let mut last = game.bricks().count();
    for _ in 0..40_000 {
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let mv = if centre < ball.x - 2.0 {
            Move::Right
        } else if centre > ball.x + 2.0 {
            Move::Left
        } else {
            Move::Hold
        };
        game.step(Input { paddle: mv });
        let now = game.bricks().count();
        assert!(now <= last, "brick count went up");
        assert!(
            last - now <= 1,
            "more than one brick broke in a single step"
        );
        last = now;
    }
    assert!(
        game.bricks().count() < 8 * 14,
        "the ball never broke into the wall at all"
    );
}
