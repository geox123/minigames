//! The reusable pieces of the Asteroids shell: the front-end state machine, the
//! logical canvas the game is drawn to, and the drawing code. The binary in
//! `main.rs` is the game; splitting these out keeps the window/input/render glue
//! out of the pure `asteroids_core`.

pub mod app;
pub mod render;

pub use app::App;

use asteroids_core::{Input, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;

/// Reads the ship off the keyboard: left/right arrows or A/D to turn, up or W to
/// thrust, Space to fire, and down/Shift for hyperspace.
pub fn read_input() -> Input {
    Input {
        turn_left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
        turn_right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
        thrust: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
        fire: is_key_down(KeyCode::Space),
        hyperspace: is_key_down(KeyCode::Down)
            || is_key_down(KeyCode::LeftShift)
            || is_key_down(KeyCode::RightShift),
    }
}

/// The Asteroids canvas, at the original's 1024×768 vector resolution.
pub fn logical_canvas() -> RenderTarget {
    shell_kit::screen::logical_canvas(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32)
}

/// A camera mapping Asteroids' logical units onto its canvas.
pub fn logical_camera(canvas: &RenderTarget) -> Camera2D {
    shell_kit::screen::logical_camera(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT)
}

/// Blits Asteroids' canvas to the window. (No screen shake yet — that is the
/// Remix's juice; the Faithful passes zero.)
pub fn blit_canvas(canvas: &Texture2D) {
    shell_kit::screen::blit_canvas(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT, Vec2::ZERO);
}
