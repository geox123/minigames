//! Drawing the core's state onto the logical canvas, in the era's stark look.
//!
//! Everything here works in the core's logical units; scaling to the real
//! window happens once, when the canvas is blitted to the screen. The invaders
//! are plain blocks for now — the authored sprites and the cabinet's colour
//! bands are their own ticket.

use macroquad::prelude::*;
use stepfall_core::{
    BOMB_HEIGHT, BOMB_WIDTH, BUNKER_CELL, CANNON_HEIGHT, CANNON_WIDTH, Game, INVADER_HEIGHT,
    INVADER_WIDTH, LOGICAL_HEIGHT, LOGICAL_WIDTH, Phase, SAUCER_HEIGHT, SAUCER_WIDTH, SHOT_HEIGHT,
    SHOT_WIDTH,
};

use crate::app::Mode;
use shell_kit::font;

/// Text sizes, as the pixel scale each is drawn at.
const TITLE_SCALE: f32 = 4.0;
const OPTION_SCALE: f32 = 2.0;
const HINT_SCALE: f32 = 1.0;

/// The line the cannon rides, and the field it defends.
const GROUND: Color = color_u8!(60, 220, 90, 255);
/// The mystery saucer — red, the one splash of colour in the era's palette.
const SAUCER: Color = color_u8!(220, 70, 70, 255);
/// A little cannon icon per remaining life, along the top-right.
const LIFE_ICON_W: f32 = 11.0;
const LIFE_ICON_H: f32 = 4.0;
const LIFE_ICON_GAP: f32 = 4.0;

/// Draws one frame of the game onto the logical canvas. `best` is the session's
/// high score, shown in the HUD.
pub fn draw(game: &Game, best: u32) {
    clear_background(BLACK);

    // The mystery saucer, when it's crossing.
    if let Some(saucer) = game.saucer() {
        draw_rectangle(saucer.x, saucer.y, SAUCER_WIDTH, SAUCER_HEIGHT, SAUCER);
    }

    // The invaders — plain white blocks until the sprites ticket.
    for invader in game.invaders() {
        draw_rectangle(invader.x, invader.y, INVADER_WIDTH, INVADER_HEIGHT, WHITE);
    }

    // The bunkers — green cover, wearing holes as it is eaten from both sides.
    for block in game.bunker_blocks() {
        draw_rectangle(block.x, block.y, BUNKER_CELL, BUNKER_CELL, GROUND);
    }

    // Bombs falling, and the player's shot climbing.
    for bomb in game.bombs() {
        draw_rectangle(bomb.x, bomb.y, BOMB_WIDTH, BOMB_HEIGHT, WHITE);
    }
    if let Some(shot) = game.shot() {
        draw_rectangle(shot.x, shot.y, SHOT_WIDTH, SHOT_HEIGHT, WHITE);
    }

    // The cannon, and the ground line it rides along.
    let cannon = game.cannon();
    draw_rectangle(cannon.x, cannon.y, CANNON_WIDTH, CANNON_HEIGHT, GROUND);
    let base = cannon.y + CANNON_HEIGHT + 2.0;
    draw_rectangle(0.0, base, LOGICAL_WIDTH, 1.0, GROUND);

    draw_hud(game, best);
    if game.phase() == Phase::GameOver {
        draw_game_over();
    }
}

/// The top strip: the score at the left, the session's best in the middle, and
/// the lives left as little cannon icons at the right.
fn draw_hud(game: &Game, best: u32) {
    font::draw(&format!("{:04}", game.score()), 6.0, 6.0, HINT_SCALE, WHITE);

    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("HI {best:04}"),
        6.0,
        HINT_SCALE,
        GRAY,
    );

    let mut x = LOGICAL_WIDTH - 6.0 - LIFE_ICON_W;
    for _ in 0..game.lives() {
        draw_rectangle(x, 6.0, LIFE_ICON_W, LIFE_ICON_H, GROUND);
        x -= LIFE_ICON_W + LIFE_ICON_GAP;
    }
}

/// The card shown once every life is spent.
fn draw_game_over() {
    font::draw_centred(
        LOGICAL_WIDTH,
        "GAME OVER",
        LOGICAL_HEIGHT / 2.0 - 12.0,
        OPTION_SCALE,
        WHITE,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "PRESS R TO PLAY AGAIN",
        LOGICAL_HEIGHT / 2.0 + 12.0,
        HINT_SCALE,
        GROUND,
    );
}

/// Draws the Collection's mode-select: the two takes STEPFALL ships. The
/// Faithful is playable; the Remix shows as locked, coming later.
pub fn mode_select(highlight: Mode) {
    clear_background(BLACK);

    font::draw_centred(LOGICAL_WIDTH, "STEPFALL", 44.0, TITLE_SCALE, WHITE);
    font::draw_centred(
        LOGICAL_WIDTH,
        "THE FAITHFUL AND THE REMIX",
        84.0,
        HINT_SCALE,
        GRAY,
    );
    option("FAITHFUL", 128.0, highlight == Mode::Faithful, false);
    option("REMIX", 160.0, highlight == Mode::Remix, true);
    if highlight == Mode::Remix {
        font::draw_centred(LOGICAL_WIDTH, "COMING SOON", 182.0, HINT_SCALE, GRAY);
    }
    font::draw_centred(
        LOGICAL_WIDTH,
        "ARROWS TO CHOOSE   ENTER TO SELECT",
        220.0,
        HINT_SCALE,
        GRAY,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "AFTER THE 1978 INVASION ORIGINAL",
        LOGICAL_HEIGHT - 20.0,
        HINT_SCALE,
        GRAY,
    );
}

/// One menu row: its label, marked with a caret when highlighted and dimmed
/// when it is locked.
fn option(label: &str, y: f32, highlighted: bool, locked: bool) {
    let colour = if locked { GRAY } else { WHITE };
    let width = font::text_width(label, OPTION_SCALE);
    let x = (LOGICAL_WIDTH - width) / 2.0;
    font::draw(label, x, y, OPTION_SCALE, colour);
    if highlighted {
        font::draw(
            ">",
            x - font::text_width("> ", OPTION_SCALE),
            y,
            OPTION_SCALE,
            colour,
        );
    }
}

/// Draws the paused banner over a frozen game.
pub fn paused_overlay() {
    font::draw_centred(
        LOGICAL_WIDTH,
        "PAUSED",
        LOGICAL_HEIGHT / 2.0 - 12.0,
        OPTION_SCALE,
        WHITE,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "P RESUME   R RESTART   ESC QUIT",
        LOGICAL_HEIGHT / 2.0 + 12.0,
        HINT_SCALE,
        WHITE,
    );
}
