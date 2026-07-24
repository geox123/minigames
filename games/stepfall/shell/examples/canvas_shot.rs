//! Dev tool: plays the game a while and writes a frame to PNGs, so layout,
//! orientation and scaling can be checked without a human at the keyboard. It
//! reads back macroquad's own framebuffer — it never captures the desktop.
//!
//! ```text
//! cargo run -p stepfall --example canvas_shot -- <out-dir> [steps]
//! ```

use macroquad::prelude::*;
use stepfall::{blit_canvas, logical_camera, logical_canvas, render};
use stepfall_core::{Game, Input, Move};

#[macroquad::main("stepfall canvas shot")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = args.next().unwrap_or_else(|| ".".to_owned());
    let steps: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(600);

    let canvas = logical_canvas();
    let camera = logical_camera(&canvas);
    let mut game = Game::new(7);
    for i in 0..steps {
        // Drift the cannon back and forth so the scene isn't static.
        let cannon = if (i / 90) % 2 == 0 {
            Move::Right
        } else {
            Move::Left
        };
        game.step(Input { cannon });
    }

    for frame in 0..2 {
        set_camera(&camera);
        render::draw(&game);
        set_default_camera();

        clear_background(DARKGRAY);
        blit_canvas(&canvas.texture);

        if frame == 1 {
            canvas
                .texture
                .get_texture_data()
                .export_png(&format!("{out_dir}/stepfall.png"));
        }

        next_frame().await;
    }

    println!("wrote {out_dir}/stepfall.png");
}
