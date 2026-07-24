//! Dev tool: plays a RIFT run a while and writes a frame to PNGs, so its layout,
//! palette, HUD and run-summary can be checked without a human at the keyboard.
//! It reads back macroquad's own framebuffer — it never captures the desktop.
//!
//! ```text
//! cargo run -p breakout --example rift_shot -- <out-dir> [steps] [play|lose|menu]
//! ```
//! `play` (default) tracks the ball to show a live rally and the HUD; `lose`
//! dodges every ball to end the run and show the run-over card; `menu` captures
//! RIFT's mode menu.

use breakout::app::RiftMode;
use breakout::{blit_canvas, logical_camera, logical_canvas, render, rift};
use breakout_remix_core::{Game, Input, Move, Phase, Pool};
use macroquad::prelude::*;

#[macroquad::main("rift canvas shot")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = args.next().unwrap_or_else(|| ".".to_owned());
    let steps: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(240);
    let mode = args.next().unwrap_or_default();
    let dodge = mode == "lose";

    let canvas = logical_canvas();
    let camera = logical_camera(&canvas);

    if mode == "menu" {
        for frame in 0..2 {
            set_camera(&camera);
            render::rift_menu(RiftMode::Ascension, 3);
            set_default_camera();
            clear_background(DARKGRAY);
            blit_canvas(&canvas.texture);
            if frame == 1 {
                canvas
                    .texture
                    .get_texture_data()
                    .export_png(&format!("{out_dir}/rift-menu.png"));
            }
            next_frame().await;
        }
        println!("wrote {out_dir}/rift-menu.png");
        return;
    }
    let mut game = Game::new_run(7, &Pool::base());
    for _ in 0..steps {
        let ball = game.ball();
        let centre = game.paddle().x + game.paddle().width / 2.0;
        // Track the ball to show a rally, or dodge it to end the run.
        let paddle = if dodge {
            if ball.x < centre {
                Move::Right
            } else {
                Move::Left
            }
        } else if centre < ball.x - 2.0 {
            Move::Right
        } else if centre > ball.x + 2.0 {
            Move::Left
        } else {
            Move::Hold
        };
        game.step(Input {
            paddle,
            ..Default::default()
        });
        if dodge && game.phase() == Phase::Lost {
            break;
        }
    }

    let suffix = if dodge { "summary" } else { "play" };
    for frame in 0..2 {
        set_camera(&camera);
        rift::draw(&game);
        rift::run_summary(&game, "RUN BEST DEPTH 2");
        set_default_camera();

        clear_background(DARKGRAY);
        blit_canvas(&canvas.texture);

        if frame == 1 {
            canvas
                .texture
                .get_texture_data()
                .export_png(&format!("{out_dir}/rift-{suffix}.png"));
        }

        next_frame().await;
    }

    println!("wrote {out_dir}/rift-{suffix}.png");
}
