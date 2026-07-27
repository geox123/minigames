//! Drawing GNASH's board and eater to the logical canvas.
//!
//! Placeholder-plain by design: the authored look — the chomp animation, the
//! hunters' tracking eyes and frightened faces, the maze's neon — lands in T9. This
//! is the tracer bullet that proves the core drives a real window, so walls are flat
//! blocks and the eater is a plain disc.

use gnash_core::{COLS, Game, Pickup, ROWS, TILE, Tile, tile_center};
use macroquad::prelude::*;

use crate::{MAZE_TOP, SCREEN_W};

// Placeholder palette — the era's blue maze on black, replaced with the authored
// look in T9.
const WALL: Color = Color::new(0.15, 0.18, 0.85, 1.0);
const GATE: Color = Color::new(0.95, 0.72, 0.82, 1.0);
const PICKUP: Color = Color::new(1.0, 0.72, 0.62, 1.0);
const EATER: Color = YELLOW;

/// The footer text's top edge — just below the maze, within the 16px footer band.
const FOOTER_Y: f32 = MAZE_TOP + (ROWS as f32) * (TILE as f32) + 1.0;

/// Draws the whole frame: the maze, its pickups, the eater, the HUD, and a pause
/// overlay when held.
pub fn draw(game: &Game, paused: bool) {
    clear_background(BLACK);
    draw_maze(game);
    draw_eater(game);
    draw_hud(game);
    if paused {
        shell_kit::font::draw_centred(SCREEN_W, "PAUSED", MAZE_TOP + 116.0, 3.0, WHITE);
    }
}

/// Draws the walls, the gate, and the dots and power pellets still on the board.
fn draw_maze(game: &Game) {
    let t = TILE as f32;
    for row in 0..ROWS as i32 {
        for col in 0..COLS as i32 {
            let x = col as f32 * t;
            let y = MAZE_TOP + row as f32 * t;
            match game.tile(col, row) {
                Tile::Wall => draw_rectangle(x, y, t, t, WALL),
                Tile::Gate => draw_rectangle(x, y + t / 2.0 - 1.0, t, 2.0, GATE),
                Tile::Path => {}
            }
            let (cx, cy) = tile_center(col, row);
            let (px, py) = (cx as f32, MAZE_TOP + cy as f32);
            match game.pickup(col, row) {
                Pickup::Dot => draw_rectangle(px - 1.0, py - 1.0, 2.0, 2.0, PICKUP),
                Pickup::PowerPellet => draw_circle(px, py, 3.0, PICKUP),
                Pickup::None => {}
            }
        }
    }
}

/// Draws the eater — a plain disc for now; the facing chomp is T9.
fn draw_eater(game: &Game) {
    let eater = game.eater();
    draw_circle(eater.x as f32, MAZE_TOP + eater.y as f32, 3.5, EATER);
}

/// Draws the score along the top bar and the game's name along the footer.
fn draw_hud(game: &Game) {
    shell_kit::font::draw(&format!("SCORE {}", game.score()), 8.0, 8.0, 2.0, WHITE);
    shell_kit::font::draw_centred(SCREEN_W, "GNASH", FOOTER_Y, 2.0, DARKGRAY);
}
