//! RIFT — Breakout's Remix — in the shell: reading its paddle off the keyboard
//! and drawing its state to the logical canvas.
//!
//! RIFT shares the Faithful's portrait logical resolution, so it reuses the same
//! canvas, camera and blit; only its input, look and HUD are its own. Everything
//! here is engine glue around the pure `breakout_remix_core`, which owns the
//! rules. It draws the descent — the wall, the depth and guardian markers, the
//! lives — and the run-summary card when a run is won or lost. The brick zoo and
//! the boons arrive in later work.

use breakout_remix_core::{BALL_SIZE, DEPTHS, Game, Input, Kind, Move, PADDLE_HEIGHT, Phase};
use breakout_remix_core::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;

use shell_kit::font;

/// RIFT's palette — cool violets and cyans on a deep indigo field, deliberately
/// apart from the Faithful's stark yellow-green-orange-red on black so the two
/// takes read at a glance.
const BACKGROUND: Color = color_u8!(16, 14, 32, 255);
const FRAME: Color = color_u8!(88, 78, 140, 255);
const PADDLE: Color = color_u8!(90, 230, 220, 255);
const BALL: Color = color_u8!(240, 250, 255, 255);
const HUD_TEXT: Color = color_u8!(200, 190, 230, 255);
const HUD_ACCENT: Color = color_u8!(90, 230, 220, 255);

/// The colour of each brick band, low (bottom) to high (top): teal, azure,
/// violet, magenta — cool and ascending, RIFT's own.
const BAND_COLOURS: [Color; 4] = [
    color_u8!(70, 200, 180, 255),  // teal
    color_u8!(80, 160, 240, 255),  // azure
    color_u8!(150, 110, 240, 255), // violet
    color_u8!(230, 90, 200, 255),  // magenta
];

/// Armoured bricks read as steel — darker once cracked.
const ARMOURED: Color = color_u8!(122, 128, 148, 255);
const ARMOURED_CRACKED: Color = color_u8!(84, 88, 104, 255);
/// Mirror bricks read as pale silver with a diagonal sheen.
const MIRROR: Color = color_u8!(206, 222, 236, 255);
const MIRROR_SHEEN: Color = color_u8!(250, 252, 255, 255);

/// A colour to mark a guardian wall, and the run-lost card.
const GUARDIAN: Color = color_u8!(230, 90, 200, 255);

/// The HUD lives in the strip above the wall.
const HUD_SCALE: f32 = 2.0;
const HEADING_SCALE: f32 = 3.0;
const HUD_TOP: f32 = 8.0;
const HINT_SCALE: f32 = 1.0;
/// A little paddle icon per remaining life, along the top-right.
const LIFE_ICON_W: f32 = 12.0;
const LIFE_ICON_H: f32 = 3.0;
const LIFE_ICON_GAP: f32 = 4.0;

/// Reads RIFT's paddle off the keyboard: the left/right arrows or A/D.
pub fn read_input() -> Input {
    let left = is_key_down(KeyCode::Left) || is_key_down(KeyCode::A);
    let right = is_key_down(KeyCode::Right) || is_key_down(KeyCode::D);
    Input {
        paddle: match (left, right) {
            (true, false) => Move::Left,
            (false, true) => Move::Right,
            _ => Move::Hold,
        },
    }
}

/// Draws one frame of RIFT onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BACKGROUND);

    // The three walls the ball plays off — left, top, right — framing the field.
    let w = 2.0;
    draw_rectangle(0.0, 0.0, LOGICAL_WIDTH, w, FRAME);
    draw_rectangle(0.0, 0.0, w, LOGICAL_HEIGHT, FRAME);
    draw_rectangle(LOGICAL_WIDTH - w, 0.0, w, LOGICAL_HEIGHT, FRAME);

    // The brick wall, each brick inset a touch so the rows read apart, coloured
    // by kind: normal bricks by band, armoured as steel (darker when cracked),
    // mirrors as pale silver with a diagonal sheen.
    for brick in game.bricks() {
        let colour = match brick.kind {
            Kind::Normal => BAND_COLOURS[brick.band as usize],
            Kind::Armoured if brick.damaged => ARMOURED_CRACKED,
            Kind::Armoured => ARMOURED,
            Kind::Mirror => MIRROR,
        };
        let (bx, by) = (brick.x + 0.5, brick.y + 0.5);
        let (bw, bh) = (brick.width - 1.0, brick.height - 1.0);
        draw_rectangle(bx, by, bw, bh, colour);
        if brick.kind == Kind::Mirror {
            draw_line(bx, by + bh, bx + bw, by, 1.0, MIRROR_SHEEN);
        }
    }

    let paddle = game.paddle();
    draw_rectangle(paddle.x, paddle.y, paddle.width, PADDLE_HEIGHT, PADDLE);

    // The ball is only in flight while the run is live; hide it behind the
    // run-summary card once the run is over.
    if matches!(game.phase(), Phase::Serving | Phase::Playing) {
        let ball = game.ball();
        let half = BALL_SIZE / 2.0;
        draw_rectangle(ball.x - half, ball.y - half, BALL_SIZE, BALL_SIZE, BALL);
    }

    draw_hud(game);
}

/// The score, the current depth (marked when a guardian is up), and the lives
/// left, along the top strip.
fn draw_hud(game: &Game) {
    // Score, top-left.
    font::draw(&game.score().to_string(), 6.0, HUD_TOP, HUD_SCALE, HUD_TEXT);

    // Depth of the descent, centred — flagged when the guardian is up.
    let (label, colour) = if game.on_guardian() {
        (
            format!("DEPTH {} OF {DEPTHS}  GUARDIAN", game.depth()),
            GUARDIAN,
        )
    } else {
        (format!("DEPTH {} OF {DEPTHS}", game.depth()), HUD_ACCENT)
    };
    font::draw_centred(LOGICAL_WIDTH, &label, HUD_TOP + 3.0, HINT_SCALE, colour);

    // Lives left, as little paddle icons top-right.
    let mut x = LOGICAL_WIDTH - 6.0 - LIFE_ICON_W;
    for _ in 0..game.lives() {
        draw_rectangle(x, HUD_TOP + 2.0, LIFE_ICON_W, LIFE_ICON_H, PADDLE);
        x -= LIFE_ICON_W + LIFE_ICON_GAP;
    }
}

/// The card shown once a run is won or lost: how deep it got, its score, and the
/// best depth reached so far. Drawn over the frozen field.
pub fn run_summary(game: &Game, best_depth: u32) {
    let (line, colour) = match game.phase() {
        Phase::Won => ("RUN CLEARED", HUD_ACCENT),
        Phase::Lost => ("RUN OVER", GUARDIAN),
        _ => return,
    };
    font::draw_centred(
        LOGICAL_WIDTH,
        line,
        LOGICAL_HEIGHT / 2.0 - 32.0,
        HEADING_SCALE,
        colour,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("DEPTH {}   SCORE {}", game.depth(), game.score()),
        LOGICAL_HEIGHT / 2.0,
        HINT_SCALE,
        HUD_TEXT,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("BEST DEPTH {best_depth}"),
        LOGICAL_HEIGHT / 2.0 + 12.0,
        HINT_SCALE,
        HUD_ACCENT,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "R TO DIVE AGAIN   ESC TO QUIT",
        LOGICAL_HEIGHT / 2.0 + 32.0,
        HINT_SCALE,
        HUD_TEXT,
    );
}
