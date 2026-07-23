//! Drawing the core's state onto the logical canvas, in the era's stark look.

use breakout_core::{BALL_SIZE, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH, PADDLE_HEIGHT};
use macroquad::prelude::*;

/// The colour of each band, low (bottom) to high (top) — the original's
/// yellow, green, orange, red.
const BAND_COLOURS: [Color; 4] = [
    color_u8!(230, 220, 70, 255), // yellow
    color_u8!(90, 200, 90, 255),  // green
    color_u8!(230, 150, 60, 255), // orange
    color_u8!(220, 70, 60, 255),  // red
];

/// Draws one frame of the game onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BLACK);

    // The three walls the ball plays off — left, top, right — framing the field.
    let w = 2.0;
    draw_rectangle(0.0, 0.0, LOGICAL_WIDTH, w, WHITE);
    draw_rectangle(0.0, 0.0, w, LOGICAL_HEIGHT, WHITE);
    draw_rectangle(LOGICAL_WIDTH - w, 0.0, w, LOGICAL_HEIGHT, WHITE);

    // The brick wall, each brick inset a touch so the rows read apart.
    for brick in game.bricks() {
        let colour = BAND_COLOURS[brick.band as usize];
        draw_rectangle(
            brick.x + 0.5,
            brick.y + 0.5,
            brick.width - 1.0,
            brick.height - 1.0,
            colour,
        );
    }

    let paddle = game.paddle();
    draw_rectangle(paddle.x, paddle.y, paddle.width, PADDLE_HEIGHT, WHITE);

    let ball = game.ball();
    let half = BALL_SIZE / 2.0;
    draw_rectangle(ball.x - half, ball.y - half, BALL_SIZE, BALL_SIZE, WHITE);
}
