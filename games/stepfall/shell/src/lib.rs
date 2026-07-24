//! The reusable pieces of the STEPFALL shell: the front-end state machine, the
//! logical canvas the game is drawn to, and the drawing code. The binary in
//! `main.rs` is the game; splitting these out lets a dev tool in `examples/`
//! render real frames without a human at the keyboard.

pub mod app;
pub mod audio;
pub mod render;
pub mod sprites;

pub use app::App;
pub use audio::Audio;

use macroquad::prelude::*;
use stepfall_core::{Input, LOGICAL_HEIGHT, LOGICAL_WIDTH, Move};

/// Reads the cannon off the keyboard: the left/right arrows or A/D to move, and
/// Space (or Up) to fire.
pub fn read_input() -> Input {
    let left = is_key_down(KeyCode::Left) || is_key_down(KeyCode::A);
    let right = is_key_down(KeyCode::Right) || is_key_down(KeyCode::D);
    Input {
        cannon: match (left, right) {
            (true, false) => Move::Left,
            (false, true) => Move::Right,
            _ => Move::Hold,
        },
        fire: is_key_down(KeyCode::Space) || is_key_down(KeyCode::Up),
    }
}

/// Reads the HAILFALL ship off the keyboard: arrows or WASD to fly, Space/Z to
/// fire, Shift to focus, X to dash, C to spend an overdrive. (Only movement
/// matters until the later tickets wire the rest.)
pub fn read_remix_input() -> stepfall_remix_core::Input {
    stepfall_remix_core::Input {
        left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
        right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
        up: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
        down: is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
        fire: is_key_down(KeyCode::Space) || is_key_down(KeyCode::Z),
        focus: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
        dash: is_key_pressed(KeyCode::X),
        bomb: is_key_pressed(KeyCode::C),
    }
}

/// The STEPFALL canvas, at the original's portrait logical resolution.
pub fn logical_canvas() -> RenderTarget {
    shell_kit::screen::logical_canvas(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32)
}

/// A camera mapping STEPFALL's logical units onto its canvas.
pub fn logical_camera(canvas: &RenderTarget) -> Camera2D {
    shell_kit::screen::logical_camera(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT)
}

/// Blits STEPFALL's canvas to the window.
pub fn blit_canvas(canvas: &Texture2D) {
    shell_kit::screen::blit_canvas(canvas, LOGICAL_WIDTH, LOGICAL_HEIGHT, Vec2::ZERO);
}
