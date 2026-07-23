//! Drawing the core's state onto the logical canvas, in the era's stark look.

use breakout_core::{BALL_SIZE, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;

/// Draws one frame of the game onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BLACK);

    // The three walls the ball plays off — left, top, right — framing the field.
    let w = 2.0;
    draw_rectangle(0.0, 0.0, LOGICAL_WIDTH, w, WHITE);
    draw_rectangle(0.0, 0.0, w, LOGICAL_HEIGHT, WHITE);
    draw_rectangle(LOGICAL_WIDTH - w, 0.0, w, LOGICAL_HEIGHT, WHITE);

    let ball = game.ball();
    let half = BALL_SIZE / 2.0;
    draw_rectangle(ball.x - half, ball.y - half, BALL_SIZE, BALL_SIZE, WHITE);
}
