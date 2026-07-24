//! Drawing the core's state onto the logical canvas, in the era's stark look.
//!
//! Everything here works in the core's logical units; scaling to the real
//! window happens once, when the canvas is blitted to the screen. The invaders
//! are plain blocks for now — the authored sprites and the cabinet's colour
//! bands are their own ticket.

use macroquad::prelude::*;
use stepfall_core::{
    CANNON_HEIGHT, CANNON_WIDTH, Game, INVADER_HEIGHT, INVADER_WIDTH, LOGICAL_HEIGHT, LOGICAL_WIDTH,
};

use crate::app::Mode;
use shell_kit::font;

/// Text sizes, as the pixel scale each is drawn at.
const TITLE_SCALE: f32 = 4.0;
const OPTION_SCALE: f32 = 2.0;
const HINT_SCALE: f32 = 1.0;

/// The line the cannon rides, and the field it defends.
const GROUND: Color = color_u8!(60, 220, 90, 255);

/// Draws one frame of the game onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BLACK);

    // The invaders — plain white blocks until the sprites ticket.
    for invader in game.invaders() {
        draw_rectangle(invader.x, invader.y, INVADER_WIDTH, INVADER_HEIGHT, WHITE);
    }

    // The cannon, and the ground line it rides along.
    let cannon = game.cannon();
    draw_rectangle(cannon.x, cannon.y, CANNON_WIDTH, CANNON_HEIGHT, GROUND);
    let base = cannon.y + CANNON_HEIGHT + 2.0;
    draw_rectangle(0.0, base, LOGICAL_WIDTH, 1.0, GROUND);

    draw_hud(game);
}

/// A minimal top strip: how many invaders are still standing. Score, lives and
/// the high score arrive with their own tickets.
fn draw_hud(game: &Game) {
    font::draw(
        &format!("INVADERS {:02}", game.alive()),
        6.0,
        6.0,
        HINT_SCALE,
        WHITE,
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
