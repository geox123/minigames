//! Pong — a faithful recreation.
//!
//! This binary is the shell: it owns the window, the real clock, real input,
//! rendering and audio. Every rule of the game lives in `pong_core`.

use macroquad::prelude::*;
use pong::{App, blit_canvas, logical_camera, logical_canvas};
use pong_core::{LOGICAL_HEIGHT, LOGICAL_WIDTH};

fn window_conf() -> Conf {
    Conf {
        window_title: "Pong — Faithful".to_owned(),
        window_width: LOGICAL_WIDTH as i32 * 3,
        window_height: LOGICAL_HEIGHT as i32 * 3,
        window_resizable: true,
        ..Default::default()
    }
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

        // ...which is then scaled up to fill the window.
        clear_background(BLACK);
        blit_canvas(&canvas.texture);

        next_frame().await;
    }
}
