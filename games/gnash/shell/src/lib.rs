//! The reusable pieces of the GNASH shell: the front-end, the logical canvas the
//! game is drawn to, and the drawing code. The binary in `main.rs` owns the window
//! and clock; all the rules live in the pure `gnash_core`.
//!
//! The canvas is the maze (224×248) plus HUD margins — a **224×288** field, the
//! original's screen, with a score bar above the maze and a footer below it.

pub mod app;
pub mod render;

pub use app::App;

use gnash_core::{Input, LOGICAL_WIDTH};
use macroquad::prelude::*;

/// The canvas width — the maze's, in logical pixels.
pub const SCREEN_W: f32 = LOGICAL_WIDTH as f32;
/// The canvas height — the maze (248) plus a 24px score bar above and a 16px footer
/// below, the original's 224×288 screen.
pub const SCREEN_H: f32 = 288.0;
/// Where the maze's top edge sits on the canvas — below the score bar.
pub const MAZE_TOP: f32 = 24.0;

/// Reads the eater off the keyboard: arrows or WASD. A held direction is the buffered
/// turn the core takes at the next opening.
pub fn read_input() -> Input {
    Input {
        up: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
        down: is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
        left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
        right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
    }
}

/// The GNASH canvas, at the original's 224×288 screen resolution.
pub fn logical_canvas() -> RenderTarget {
    shell_kit::screen::logical_canvas(SCREEN_W as u32, SCREEN_H as u32)
}

/// A camera mapping GNASH's logical units onto its canvas.
pub fn logical_camera(canvas: &RenderTarget) -> Camera2D {
    shell_kit::screen::logical_camera(canvas, SCREEN_W, SCREEN_H)
}

/// Blits GNASH's canvas to the window. The Faithful has no screen shake, so the
/// nudge is always zero.
pub fn blit_canvas(canvas: &Texture2D) {
    shell_kit::screen::blit_canvas(canvas, SCREEN_W, SCREEN_H, Vec2::ZERO);
}
