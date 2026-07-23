//! The logical canvas a Game draws to, and its crisp scaling to the window.
//!
//! A Game renders at its own low logical resolution to an offscreen canvas;
//! that canvas is then scaled to the window by a whole number (when it fits),
//! aspect ratio preserved, so every logical pixel stays square and nothing
//! smears. Works for any resolution — landscape or portrait.

use macroquad::prelude::*;

/// An offscreen canvas `width`×`height` logical units, with nearest-neighbour
/// filtering so it stays crisp when scaled up.
pub fn logical_canvas(width: u32, height: u32) -> RenderTarget {
    let canvas = render_target(width, height);
    canvas.texture.set_filter(FilterMode::Nearest);
    canvas
}

/// A camera mapping the Game's logical units onto `canvas`, origin top-left.
pub fn logical_camera(canvas: &RenderTarget, width: f32, height: f32) -> Camera2D {
    let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, width, height));
    camera.render_target = Some(canvas.clone());
    camera
}

/// Blits a `width`×`height` logical canvas to the window: centred, aspect ratio
/// preserved, and — when it fits — scaled by a whole number. `shake` nudges the
/// whole image by that many logical units, for screen shake.
pub fn blit_canvas(canvas: &Texture2D, width: f32, height: f32, shake: Vec2) {
    let fit = (screen_width() / width).min(screen_height() / height);
    let scale = if fit >= 1.0 { fit.floor() } else { fit };
    let size = vec2(width * scale, height * scale);
    let origin = (vec2(screen_width(), screen_height()) - size) / 2.0 + shake * scale;

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
