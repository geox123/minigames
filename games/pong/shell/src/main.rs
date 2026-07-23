//! Pong — a faithful recreation.
//!
//! This binary is the shell: it owns the window, the real clock, real input,
//! rendering and audio. Every rule of the game lives in `pong_core`.

use macroquad::prelude::*;
use pong::{blit_canvas, logical_camera, logical_canvas, render};
use pong_core::{Axis, Game, Input, LOGICAL_HEIGHT, LOGICAL_WIDTH, TIMESTEP};

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make
/// the game try to catch up by simulating minutes at once.
const MAX_FRAME_TIME: f32 = 0.25;

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

    let mut game = Game::new(seed_from_clock());
    let mut accumulator = 0.0;

    loop {
        if is_key_pressed(KeyCode::R) {
            game.restart();
        }

        let input = read_input();
        accumulator = (accumulator + get_frame_time()).min(MAX_FRAME_TIME);
        while accumulator >= TIMESTEP {
            game.step(input);
            accumulator -= TIMESTEP;
        }

        set_camera(&camera);
        render::draw(&game);
        set_default_camera();

        clear_background(BLACK);
        blit_canvas(&canvas.texture);

        next_frame().await;
    }
}

/// Reads both players off one keyboard, as the original's two knobs.
fn read_input() -> Input {
    Input {
        left: axis(KeyCode::W, KeyCode::S),
        right: axis(KeyCode::Up, KeyCode::Down),
    }
}

fn axis(up: KeyCode, down: KeyCode) -> Axis {
    match (is_key_down(up), is_key_down(down)) {
        (true, false) => Axis::Up,
        (false, true) => Axis::Down,
        // Both or neither: stay put.
        _ => Axis::Hold,
    }
}

/// A seed for the core's generator. The core is deterministic by design, so the
/// only nondeterminism in the game is this one number, read once at startup.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
