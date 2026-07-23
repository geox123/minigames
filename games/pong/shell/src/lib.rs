//! The shell's reusable pieces: the front-end state machine, the logical
//! canvas the game is drawn to, and the drawing code.
//!
//! The binary in `main.rs` is the game. Splitting these out lets the dev tools
//! in `examples/` render real frames without a human at the keyboard.

pub mod app;
pub mod audio;
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

/// The offscreen canvas the game is drawn to, at the Faithful's logical
/// resolution. Nearest-neighbour filtering keeps it crisp when it is scaled up
/// to the window.
pub fn logical_canvas() -> RenderTarget {
    let canvas = render_target(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32);
    canvas.texture.set_filter(FilterMode::Nearest);
    canvas
}

/// A camera that maps the core's logical units onto the logical canvas, with
/// the origin at the top left.
pub fn logical_camera(canvas: &RenderTarget) -> Camera2D {
    let mut camera =
        Camera2D::from_display_rect(Rect::new(0.0, 0.0, LOGICAL_WIDTH, LOGICAL_HEIGHT));
    camera.render_target = Some(canvas.clone());
    camera
}

/// Blits the logical canvas to the window: centred, aspect ratio preserved and
/// — whenever the window is big enough — scaled by a whole number, so every
/// logical pixel stays the same size and nothing smears.
pub fn blit_canvas(canvas: &Texture2D) {
    let fit = (screen_width() / LOGICAL_WIDTH).min(screen_height() / LOGICAL_HEIGHT);
    let scale = if fit >= 1.0 { fit.floor() } else { fit };
    let size = vec2(LOGICAL_WIDTH * scale, LOGICAL_HEIGHT * scale);
    let origin = (vec2(screen_width(), screen_height()) - size) / 2.0;

    draw_texture_ex(
        canvas,
        origin.x,
        origin.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(size),
            // Render targets come out bottom-up.
            flip_y: true,
            ..Default::default()
        },
    );
}
