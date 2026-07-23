//! Dev tool: plays a scripted match and writes a frame of it to a PNG, so
//! layout, orientation and scaling can be checked without a human at the
//! keyboard. It reads back macroquad's own framebuffer — it never captures the
//! desktop.
//!
//! ```text
//! cargo run -p pong --example canvas_shot -- <out-dir> [rally|gameover]
//! ```
//!
//! Writes `screen.png` (what the window shows, letterboxing and all) and
//! `canvas.png` (the logical canvas on its own).

use macroquad::prelude::*;
use pong::{blit_canvas, logical_camera, logical_canvas, render};
use pong_core::{Axis, Game, Input, PADDLE_HEIGHT, PADDLE_SPEED, Phase, Players, Side, TIMESTEP};

#[macroquad::main("canvas shot")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = args.next().unwrap_or_else(|| ".".to_owned());
    let scene = args.next().unwrap_or_else(|| "rally".to_owned());

    let canvas = logical_canvas();
    let camera = logical_camera(&canvas);
    let mut game = Game::new(Players::Two, 7);

    // A parallel PULSE game, played the same way, for the neon scenes.
    let mut pulse = pong_remix_core::Game::new(7);

    // The left player follows the ball; the right player sits at the bottom of
    // the field and concedes, so the scene has a score on the board.
    let steps = if scene == "gameover" { 200_000 } else { 2_400 };
    for _ in 0..steps {
        if scene == "gameover" && matches!(game.phase(), Phase::GameOver { .. }) {
            break;
        }
        let input = Input {
            left: follow(game.paddle(Side::Left).y, game.ball().y),
            right: Axis::Down,
        };
        game.step(input);

        let pulse_input = pong_remix_core::Input {
            left: pulse_follow(pulse.paddle(pong_remix_core::Side::Left).y, pulse.ball().y),
            right: pulse_follow(pulse.paddle(pong_remix_core::Side::Right).y, pulse.ball().y),
            // Charge the left player so the charge meter is visible in the scene.
            charge_left: true,
            ..Default::default()
        };
        pulse.step(pulse_input);
    }

    // For the PULSE scene, play a clean centred rally until a multiball is
    // collected, then a little longer so the two balls separate visibly.
    if scene == "pulse" {
        let mut after_split = 0;
        for _ in 0..40_000 {
            if pulse.balls().count() > 1 {
                after_split += 1;
                if after_split > 40 {
                    break;
                }
            }
            let input = pong_remix_core::Input {
                left: pulse_follow(pulse.paddle(pong_remix_core::Side::Left).y, pulse.ball().y),
                right: pulse_follow(pulse.paddle(pong_remix_core::Side::Right).y, pulse.ball().y),
                ..Default::default()
            };
            pulse.step(input);
        }
    }

    // Two frames: the first gets the window up, the second is the one captured
    // — the screen has to be read back before it is swapped away.
    for frame in 0..2 {
        set_camera(&camera);
        match scene.as_str() {
            "mode" => render::mode_select(pong::app::Mode::Remix),
            "select" => render::player_select(Players::One),
            "pulse" => render::draw_pulse(&pulse),
            "paused" => {
                render::draw(&game);
                render::paused_overlay();
            }
            _ => render::draw(&game),
        }
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

    println!("wrote {out_dir}/screen.png and {out_dir}/canvas.png ({scene})");
}

/// The key a player would hold to bring their paddle's centre onto `target`.
fn follow(paddle_top: f32, target: f32) -> Axis {
    let centre = paddle_top + PADDLE_HEIGHT / 2.0;
    let slack = PADDLE_SPEED * TIMESTEP;
    if centre < target - slack {
        Axis::Down
    } else if centre > target + slack {
        Axis::Up
    } else {
        Axis::Hold
    }
}

/// The same, for a PULSE paddle.
fn pulse_follow(paddle_top: f32, target: f32) -> pong_remix_core::Axis {
    let centre = paddle_top + pong_remix_core::PADDLE_HEIGHT / 2.0;
    let slack = pong_remix_core::PADDLE_SPEED * pong_remix_core::TIMESTEP;
    if centre < target - slack {
        pong_remix_core::Axis::Down
    } else if centre > target + slack {
        pong_remix_core::Axis::Up
    } else {
        pong_remix_core::Axis::Hold
    }
}
