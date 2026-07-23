//! The reusable pieces of the Breakout shell: the logical canvas and the
//! drawing code. The binary in `main.rs` is the game; splitting these out lets
//! the dev tool in `examples/` render real frames without a human at the
//! keyboard.

pub mod render;

use breakout_core::{Input, LOGICAL_HEIGHT, LOGICAL_WIDTH, Move};
use macroquad::prelude::*;

/// Reads the paddle off the keyboard: the left/right arrows or A/D.
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

/// The Breakout canvas, at the Faithful's portrait logical resolution.
pub fn logical_canvas() -> RenderTarget {
    shell_kit::screen::logical_canvas(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32)
}

/// A camera mapping Breakout's logical units onto its canvas.
pub fn logical_camera(canvas: &RenderTarget) -> Camera2D {
    shell_kit::screen::logical_camera(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT)
}

/// Blits Breakout's canvas to the window.
pub fn blit_canvas(canvas: &Texture2D) {
    shell_kit::screen::blit_canvas(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT, Vec2::ZERO);
}
