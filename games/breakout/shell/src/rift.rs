//! RIFT — Breakout's Remix — in the shell: reading its paddle off the keyboard
//! and drawing its state to the logical canvas.
//!
//! RIFT shares the Faithful's portrait logical resolution, so it reuses the same
//! canvas, camera and blit; only its input, look and HUD are its own. Everything
//! here is engine glue around the pure `breakout_remix_core`, which owns the
//! rules. This is the descent skeleton: paddle, ball, one wall, and a HUD of
//! score and depth. Lives, guardians and the brick zoo arrive in later work.

use breakout_remix_core::{BALL_SIZE, Game, Input, Move, PADDLE_HEIGHT};
use breakout_remix_core::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;

use shell_kit::font;

/// RIFT's palette — cool violets and cyans on a deep indigo field, deliberately
/// apart from the Faithful's stark yellow-green-orange-red on black so the two
/// takes read at a glance.
const BACKGROUND: Color = color_u8!(16, 14, 32, 255);
const FRAME: Color = color_u8!(88, 78, 140, 255);
const PADDLE: Color = color_u8!(90, 230, 220, 255);
const BALL: Color = color_u8!(240, 250, 255, 255);
const HUD_TEXT: Color = color_u8!(200, 190, 230, 255);
const HUD_ACCENT: Color = color_u8!(90, 230, 220, 255);

/// The colour of each brick band, low (bottom) to high (top): teal, azure,
/// violet, magenta — cool and ascending, RIFT's own.
const BAND_COLOURS: [Color; 4] = [
    color_u8!(70, 200, 180, 255),  // teal
    color_u8!(80, 160, 240, 255),  // azure
    color_u8!(150, 110, 240, 255), // violet
    color_u8!(230, 90, 200, 255),  // magenta
];

/// The HUD lives in the strip above the wall.
const HUD_SCALE: f32 = 2.0;
const HUD_TOP: f32 = 8.0;
const HINT_SCALE: f32 = 1.0;

/// Reads RIFT's paddle off the keyboard: the left/right arrows or A/D.
pub fn read_input() -> Input {
    let left = is_key_down(KeyCode::Left) || is_key_down(KeyCode::A);
    let right = is_key_down(KeyCode::Right) || is_key_down(KeyCode::D);
    Input {
        paddle: match (left, right) {
            (true, false) => Move::Left,
            (false, true) => Move::Right,
            _ => Move::Hold,
        },
    }
}

/// Draws one frame of RIFT onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BACKGROUND);

    // The three walls the ball plays off — left, top, right — framing the field.
    let w = 2.0;
    draw_rectangle(0.0, 0.0, LOGICAL_WIDTH, w, FRAME);
    draw_rectangle(0.0, 0.0, w, LOGICAL_HEIGHT, FRAME);
    draw_rectangle(LOGICAL_WIDTH - w, 0.0, w, LOGICAL_HEIGHT, FRAME);

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
    draw_rectangle(paddle.x, paddle.y, paddle.width, PADDLE_HEIGHT, PADDLE);

    let ball = game.ball();
    let half = BALL_SIZE / 2.0;
    draw_rectangle(ball.x - half, ball.y - half, BALL_SIZE, BALL_SIZE, BALL);

    draw_hud(game);
}

/// The score and the current depth, along the top strip. Lives join the HUD
/// once the run's lives model lands.
fn draw_hud(game: &Game) {
    // Score, top-left.
    font::draw(&game.score().to_string(), 6.0, HUD_TOP, HUD_SCALE, HUD_TEXT);

    // Depth reached so far — one level per wall descended.
    let depth = game.walls_cleared() + 1;
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("DEPTH {depth}"),
        HUD_TOP + 3.0,
        HINT_SCALE,
        HUD_ACCENT,
    );
}
