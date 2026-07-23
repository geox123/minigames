//! Breakout — a faithful recreation.
//!
//! This binary is the shell: it owns the window, the real clock, real input,
//! rendering and audio. Every rule of the game lives in `breakout_core`.

use breakout::{App, Audio, blit_canvas, logical_camera, logical_canvas};
use breakout_core::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    let mut conf = Conf {
        window_title: "Breakout — Faithful".to_owned(),
        window_width: LOGICAL_WIDTH as i32 * 2,
        window_height: LOGICAL_HEIGHT as i32 * 2,
        window_resizable: true,
        ..Default::default()
    };
    // The field is rendered to an offscreen canvas and scaled up; in the browser
    // that offscreen framebuffer needs a WebGL2 context (the default WebGL1
    // rejects the binding and the canvas stays black).
    conf.platform.webgl_version = miniquad::conf::WebGLVersion::WebGL2;
    conf
}

#[macroquad::main(window_conf)]
async fn main() {
    let canvas = logical_canvas();
    let camera = logical_camera(&canvas);
    let mut app = App::new(Audio::load().await);

    loop {
        // Everything the game draws goes onto the logical canvas...
        set_camera(&camera);
        app.frame();
        set_default_camera();

        // ...which is then scaled up to fill the window.
        clear_background(BLACK);
        blit_canvas(&canvas.texture);

        next_frame().await;
    }
}
