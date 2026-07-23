//! Drawing the core's state onto the logical canvas.
//!
//! Everything here works in the core's logical units; scaling to the real
//! window happens once, when the canvas is blitted to the screen.

use macroquad::prelude::*;
use pong_core::{BALL_SIZE, Game};

/// Draws one frame of the game onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BLACK);

    let ball = game.ball();
    let half = BALL_SIZE / 2.0;
    draw_rectangle(ball.x - half, ball.y - half, BALL_SIZE, BALL_SIZE, WHITE);
}
