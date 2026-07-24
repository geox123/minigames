//! Drawing the core's state onto the logical canvas, in the era's stark look.
//!
//! Everything here works in the core's logical units; scaling to the real
//! window happens once, when the canvas is blitted to the screen.

use breakout_core::{BALL_SIZE, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH, PADDLE_HEIGHT, Phase, WALLS};
use macroquad::prelude::*;

use crate::app::{Mode, RiftMode};
use shell_kit::font;

/// Text sizes, as the pixel scale each is drawn at.
const TITLE_SCALE: f32 = 5.0;
const HEADING_SCALE: f32 = 3.0;
const OPTION_SCALE: f32 = 2.0;
const HINT_SCALE: f32 = 1.0;

/// The colour of each band, low (bottom) to high (top) — the original's
/// yellow, green, orange, red.
const BAND_COLOURS: [Color; 4] = [
    color_u8!(230, 220, 70, 255), // yellow
    color_u8!(90, 200, 90, 255),  // green
    color_u8!(230, 150, 60, 255), // orange
    color_u8!(220, 70, 60, 255),  // red
];

/// The HUD lives in the strip above the wall.
const HUD_SCALE: f32 = 2.0;
const HUD_TOP: f32 = 8.0;
/// A little paddle icon per remaining ball, along the top-right.
const BALL_ICON_W: f32 = 12.0;
const BALL_ICON_H: f32 = 3.0;
const BALL_ICON_GAP: f32 = 4.0;

/// Draws one frame of the game onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BLACK);

    // The three walls the ball plays off — left, top, right — framing the field.
    let w = 2.0;
    draw_rectangle(0.0, 0.0, LOGICAL_WIDTH, w, WHITE);
    draw_rectangle(0.0, 0.0, w, LOGICAL_HEIGHT, WHITE);
    draw_rectangle(LOGICAL_WIDTH - w, 0.0, w, LOGICAL_HEIGHT, WHITE);

    // The brick wall, each brick inset a touch so the rows read apart.
    for brick in game.bricks() {
        let colour = BAND_COLOURS[brick.band as usize];
        draw_rectangle(
            brick.x + 0.5,
            brick.y + 0.5,
            brick.width - 1.0,
            brick.height - 1.0,
            colour,
        );
    }

    let paddle = game.paddle();
    draw_rectangle(paddle.x, paddle.y, paddle.width, PADDLE_HEIGHT, WHITE);

    let ball = game.ball();
    let half = BALL_SIZE / 2.0;
    draw_rectangle(ball.x - half, ball.y - half, BALL_SIZE, BALL_SIZE, WHITE);

    draw_hud(game);
    draw_endgame(game);
}

/// The score, the balls left, and which wall is in play, along the top strip.
fn draw_hud(game: &Game) {
    // Score, top-left.
    font::draw(&game.score().to_string(), 6.0, HUD_TOP, HUD_SCALE, WHITE);

    // Which wall of the two is up, centred.
    let wall = (game.walls_cleared() + 1).min(WALLS);
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("WALL {wall}"),
        HUD_TOP + 3.0,
        HINT_SCALE,
        GRAY,
    );

    // Balls remaining, as little paddle icons top-right.
    let mut x = LOGICAL_WIDTH - 6.0 - BALL_ICON_W;
    for _ in 0..game.turns() {
        draw_rectangle(x, HUD_TOP + 2.0, BALL_ICON_W, BALL_ICON_H, WHITE);
        x -= BALL_ICON_W + BALL_ICON_GAP;
    }
}

/// The card shown once a game is won or lost.
fn draw_endgame(game: &Game) {
    let (line, hint) = match game.phase() {
        Phase::GameOver => ("GAME OVER", "PRESS R TO PLAY AGAIN"),
        Phase::Won => ("YOU WIN", "PRESS R TO PLAY AGAIN"),
        _ => return,
    };
    font::draw_centred(
        LOGICAL_WIDTH,
        line,
        LOGICAL_HEIGHT / 2.0 - 16.0,
        HEADING_SCALE,
        WHITE,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        hint,
        LOGICAL_HEIGHT / 2.0 + 16.0,
        HINT_SCALE,
        GRAY,
    );
}

/// Draws the Collection's mode-select: the two takes Breakout ships, both now
/// playable — the Faithful, and RIFT, its Remix.
pub fn mode_select(highlight: Mode) {
    clear_background(BLACK);

    font::draw_centred(LOGICAL_WIDTH, "BREAKOUT", 48.0, TITLE_SCALE, WHITE);
    font::draw_centred(
        LOGICAL_WIDTH,
        "THE FAITHFUL AND THE REMIX",
        96.0,
        HINT_SCALE,
        GRAY,
    );
    option("FAITHFUL", 140.0, highlight == Mode::Faithful, false);
    option("REMIX", 176.0, highlight == Mode::Remix, false);
    // The Remix carries its own invented name; show it under the highlight.
    if highlight == Mode::Remix {
        font::draw_centred(LOGICAL_WIDTH, "RIFT", 200.0, HINT_SCALE, GRAY);
    }
    font::draw_centred(
        LOGICAL_WIDTH,
        "ARROWS TO CHOOSE   ENTER TO SELECT",
        260.0,
        HINT_SCALE,
        GRAY,
    );
}

/// Draws RIFT's own mode menu: Run or Daily, with a one-line blurb.
pub fn rift_menu(highlight: RiftMode) {
    clear_background(BLACK);

    font::draw_centred(LOGICAL_WIDTH, "RIFT", 48.0, TITLE_SCALE, WHITE);
    font::draw_centred(LOGICAL_WIDTH, "BREAKOUT REMIX", 96.0, HINT_SCALE, GRAY);
    option("RUN", 140.0, highlight == RiftMode::Run, false);
    option("DAILY", 176.0, highlight == RiftMode::Daily, false);
    let blurb = match highlight {
        RiftMode::Run => "A FRESH DESCENT",
        RiftMode::Daily => "TODAYS SHARED SEED",
    };
    font::draw_centred(LOGICAL_WIDTH, blurb, 200.0, HINT_SCALE, GRAY);
    font::draw_centred(
        LOGICAL_WIDTH,
        "ARROWS TO CHOOSE   ENTER TO SELECT",
        260.0,
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
        // A caret one glyph-width to the left of the label.
        font::draw(
            ">",
            x - font::text_width("> ", OPTION_SCALE),
            y,
            OPTION_SCALE,
            colour,
        );
    }
}

/// Draws the paused banner over a frozen match.
pub fn paused_overlay() {
    font::draw_centred(
        LOGICAL_WIDTH,
        "PAUSED",
        LOGICAL_HEIGHT / 2.0 - 16.0,
        HEADING_SCALE,
        WHITE,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "P RESUME   R RESTART   ESC QUIT",
        LOGICAL_HEIGHT / 2.0 + 16.0,
        HINT_SCALE,
        WHITE,
    );
}
