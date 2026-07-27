//! GNASH — a faithful recreation of the 1980 arcade maze-chase original (shipped
//! under an invented name; see ADR 0005). No affiliation with the rights holder.
//!
//! This binary is the shell: it owns the window, the real clock, real input and
//! rendering. Every rule of the game lives in `gnash_core`.

use gnash::{App, blit_canvas, logical_camera, logical_canvas};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    let mut conf = Conf {
        window_title: "GNASH".to_owned(),
        window_width: gnash::SCREEN_W as i32,
        window_height: gnash::SCREEN_H as i32,
        window_resizable: true,
        ..Default::default()
    };
    // The field is rendered to an offscreen canvas and scaled up; in the browser
    // that offscreen framebuffer needs a WebGL2 context (the default WebGL1 rejects
    // the binding and the canvas stays black).
    conf.platform.webgl_version = miniquad::conf::WebGLVersion::WebGL2;
    conf
}

#[macroquad::main(window_conf)]
async fn main() {
    let canvas = logical_canvas();
    let camera = logical_camera(&canvas);
    let mut app = App::new();

    loop {
        // Everything the game draws goes onto the logical canvas...
        set_camera(&camera);
        app.frame();
        set_default_camera();

        // ...which is then scaled up, whole-number, to fill the window.
        clear_background(BLACK);
        blit_canvas(&canvas.texture);

        next_frame().await;
    }
}
