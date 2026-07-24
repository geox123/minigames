//! Waves and the descent, through the public seam. The wave *turn* (clearing a
//! whole formation) is white-box tested in the crate, since an honest full clear
//! is as impractical here as a perfect Breakout wall-clear. What play can reach —
//! and what this file drives — is the descent that ends you: thin the formation
//! so it grinds down quickly, dodge the return fire, and let the march itself
//! reach the cannon's row and end the game.

mod common;

use common::{MAX_STEPS, formation_bottom, game};
use stepfall_core::{BOMB_WIDTH, CANNON_WIDTH, Game, Input, LOGICAL_WIDTH, Move, Phase};

/// Honest play through the accessors: flee toward the side with more room from
/// the bombs bearing down, drifting back to the middle when the sky is clear.
fn dodge(game: &Game) -> Move {
    let half = CANNON_WIDTH / 2.0;
    let centre = game.cannon().x + half;
    let line = game.cannon().y;

    // The bombs low enough to matter soon.
    let threats: Vec<f32> = game
        .bombs()
        .filter(|b| b.y < line && b.y > line - 96.0)
        .map(|b| b.x + BOMB_WIDTH / 2.0)
        .collect();

    if threats.is_empty() {
        return if centre > 118.0 {
            Move::Left
        } else if centre < 106.0 {
            Move::Right
        } else {
            Move::Hold
        };
    }

    // Clearance a candidate cannon centre would have from the nearest bomb.
    let clearance = |x: f32| {
        threats
            .iter()
            .map(|t| (t - x).abs())
            .fold(f32::INFINITY, f32::min)
    };
    let can_left = centre > 8.0 + half + 1.0;
    let can_right = centre < LOGICAL_WIDTH - 8.0 - half - 1.0;
    let left = if can_left {
        clearance(centre - 4.0)
    } else {
        -1.0
    };
    let right = if can_right {
        clearance(centre + 4.0)
    } else {
        -1.0
    };
    let stay = clearance(centre);

    if left >= right && left > stay {
        Move::Left
    } else if right > left && right > stay {
        Move::Right
    } else {
        Move::Hold
    }
}

#[test]
fn the_formation_grinds_down_to_the_cannons_row_and_ends_the_game() {
    let mut game = game(1);
    let mut ended = false;
    for _ in 0..MAX_STEPS {
        // Thin the formation to hurry its descent, but stop well short of a full
        // clear (which would just turn a fresh wave). Firing costs the dodge
        // nothing — it rides the same input.
        let fire = game.alive() > 10;
        let events = game.step(Input {
            cannon: dodge(&game),
            fire,
        });
        if events.game_over {
            ended = true;
            break;
        }
    }
    assert!(ended, "the game ended");
    assert_eq!(game.phase(), Phase::GameOver);
    // The formation is down at the cannon's row: it was the descent that ended
    // the game, not a bomb spending the last life (which would leave the
    // formation up in the sky).
    assert!(
        formation_bottom(&game) >= game.cannon().y,
        "the formation reached the cannon's row"
    );
}
