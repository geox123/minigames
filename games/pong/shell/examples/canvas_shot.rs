//! Dev tool: renders a frame of the game and writes it to a PNG, so layout,
//! orientation and scaling can be checked without a human at the keyboard.
//!
//! It reads back macroquad's own framebuffer — it never captures the desktop.
//!
//! ```text
//! cargo run -p pong --example canvas_shot -- <out-dir> [steps]
//! ```
//!
//! Writes `screen.png` (what the window shows, letterboxing and all) and
//! `canvas.png` (the raw logical canvas, which is stored bottom-up).

use macroquad::prelude::*;
use pong::{blit_canvas, logical_camera, logical_canvas, render};
use pong_core::Game;

#[macroquad::main("canvas shot")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = args.next().unwrap_or_else(|| ".".to_owned());
    let steps: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    let canvas = logical_canvas();
    let camera = logical_camera(&canvas);

    let mut game = Game::new(1);
    for _ in 0..steps {
        game.step();
    }

    // Two frames: the first one gets the window up, the second is the one that
    // gets captured — the screen must be read back before it is swapped away.
    for frame in 0..2 {
        // A marker in the top-left corner of the logical field: the exported
        // images are only trustworthy if it lands top-left in `screen.png`.
        set_camera(&camera);
        render::draw(&game);
        draw_rectangle(0.0, 0.0, 16.0, 8.0, GRAY);
        set_default_camera();

        clear_background(DARKGRAY);
        blit_canvas(&canvas.texture);

        if frame == 1 {
            canvas
                .texture
                .get_texture_data()
                .export_png(&format!("{out_dir}/canvas.png"));
            get_screen_data().export_png(&format!("{out_dir}/screen.png"));
        }

        next_frame().await;
    }

    println!("wrote {out_dir}/screen.png and {out_dir}/canvas.png");
}
