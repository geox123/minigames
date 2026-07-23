//! Dev tool: plays the game a while and writes a frame to PNGs, so layout,
//! orientation and scaling can be checked without a human at the keyboard. It
//! reads back macroquad's own framebuffer — it never captures the desktop.
//!
//! ```text
//! cargo run -p breakout --example canvas_shot -- <out-dir> [steps]
//! ```

use breakout::{blit_canvas, logical_camera, logical_canvas, render};
use breakout_core::Game;
use macroquad::prelude::*;

#[macroquad::main("breakout canvas shot")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = args.next().unwrap_or_else(|| ".".to_owned());
    let steps: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(180);

    let canvas = logical_canvas();
    let camera = logical_camera(&canvas);
    let mut game = Game::new(7);
    for _ in 0..steps {
        // Track the ball with the paddle so the scene shows a live rally.
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        let paddle = if centre < ball.x - 2.0 {
            breakout_core::Move::Right
        } else if centre > ball.x + 2.0 {
            breakout_core::Move::Left
        } else {
            breakout_core::Move::Hold
        };
        game.step(breakout_core::Input { paddle });
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
                .export_png(&format!("{out_dir}/canvas.png"));
            get_screen_data().export_png(&format!("{out_dir}/screen.png"));
        }

        next_frame().await;
    }

    println!("wrote {out_dir}/screen.png and {out_dir}/canvas.png");
}
