//! The shell's reusable pieces: the front-end state machine, the logical
//! canvas the game is drawn to, and the drawing code.
//!
//! The binary in `main.rs` is the game. Splitting these out lets the dev tools
//! in `examples/` render real frames without a human at the keyboard.

pub mod app;
pub mod audio;
pub mod fx;
pub mod render;

pub use app::App;
pub use audio::Audio;

use macroquad::prelude::*;
use pong_core::{Axis, Input, LOGICAL_HEIGHT, LOGICAL_WIDTH};

/// Reads both players off one keyboard, as the original's two knobs: W/S on the
/// left, the arrow keys on the right.
pub fn read_input() -> Input {
    Input {
        left: axis(KeyCode::W, KeyCode::S),
        right: axis(KeyCode::Up, KeyCode::Down),
    }
}

fn axis(up: KeyCode, down: KeyCode) -> Axis {
    match (is_key_down(up), is_key_down(down)) {
        (true, false) => Axis::Up,
        (false, true) => Axis::Down,
        // Both or neither: stay put.
        _ => Axis::Hold,
    }
}

/// The Pong canvas, at the Faithful's logical resolution.
pub fn logical_canvas() -> RenderTarget {
    shell_kit::screen::logical_canvas(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32)
}

/// A camera mapping Pong's logical units onto its canvas.
pub fn logical_camera(canvas: &RenderTarget) -> Camera2D {
    shell_kit::screen::logical_camera(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT)
}

/// Blits Pong's canvas to the window, nudged by `shake`.
pub fn blit_canvas(canvas: &Texture2D, shake: Vec2) {
    shell_kit::screen::blit_canvas(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT, shake);
}
