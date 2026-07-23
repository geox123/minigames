//! Breakout — a faithful recreation.
//!
//! This binary is the shell: it owns the window, the real clock, real input,
//! rendering and audio. Every rule of the game lives in `breakout_core`.

use breakout::{blit_canvas, logical_camera, logical_canvas, read_input, render};
use breakout_core::{Game, LOGICAL_HEIGHT, LOGICAL_WIDTH, TIMESTEP};
use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;

/// The most real time a single frame may fold into the simulation.
const MAX_FRAME_TIME: f32 = 0.25;

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
    let mut game = Game::new(seed_from_clock());
    let mut accumulator = Accumulator::new(TIMESTEP, MAX_FRAME_TIME);

    loop {
        let input = read_input();
        for _ in 0..accumulator.steps(get_frame_time()) {
            game.step(input);
        }

        set_camera(&camera);
        render::draw(&game);
        set_default_camera();

        clear_background(BLACK);
        blit_canvas(&canvas.texture);

        next_frame().await;
    }
}

/// A seed for the core's generator, read once at startup — the game's only
/// nondeterminism.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
