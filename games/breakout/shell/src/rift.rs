//! RIFT — Breakout's Remix — in the shell: reading its paddle off the keyboard
//! and drawing its state to the logical canvas.
//!
//! RIFT shares the Faithful's portrait logical resolution, so it reuses the same
//! canvas, camera and blit; only its input, look and HUD are its own. Everything
//! here is engine glue around the pure `breakout_remix_core`, which owns the
//! rules. It draws the descent — the wall, the depth and guardian markers, the
//! lives — the boon draft held between walls, and the run-summary card when a run
//! is won or lost.

use breakout_remix_core::meta::{self, Content, Unlocked};
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
/// Explosive bricks glow hot with a bright core.
const EXPLOSIVE: Color = color_u8!(240, 130, 60, 255);
const EXPLOSIVE_CORE: Color = color_u8!(255, 240, 180, 255);
/// Mover bricks are lime with a track line across the middle.
const MOVER: Color = color_u8!(170, 215, 90, 255);
const MOVER_TRACK: Color = color_u8!(54, 74, 28, 255);
/// Spawner bricks are rose with a small plus.
const SPAWNER: Color = color_u8!(235, 150, 195, 255);
const SPAWNER_MARK: Color = color_u8!(255, 235, 245, 255);

/// A colour to mark a guardian wall, and the run-lost card.
const GUARDIAN: Color = color_u8!(230, 90, 200, 255);
/// A dim scrim drawn over the frozen field behind the draft.
const DRAFT_SCRIM: Color = color_u8!(12, 10, 26, 222);
/// Warm gold, for content a run just added to the collection.
const UNLOCK: Color = color_u8!(255, 214, 110, 255);
/// Dim, for content still out there to earn.
const LOCKED: Color = color_u8!(120, 114, 148, 255);

/// The HUD lives in the strip above the wall.
const HUD_SCALE: f32 = 2.0;
const HEADING_SCALE: f32 = 3.0;
const HUD_TOP: f32 = 8.0;
const HINT_SCALE: f32 = 1.0;
/// A little paddle icon per remaining life, along the top-right.
const LIFE_ICON_W: f32 = 12.0;
const LIFE_ICON_H: f32 = 3.0;
const LIFE_ICON_GAP: f32 = 4.0;

/// Reads RIFT's paddle off the keyboard while the ball is live: the left/right
/// arrows or A/D, held.
pub fn read_play_input() -> Input {
    let left = is_key_down(KeyCode::Left) || is_key_down(KeyCode::A);
    let right = is_key_down(KeyCode::Right) || is_key_down(KeyCode::D);
    Input {
        paddle: match (left, right) {
            (true, false) => Move::Left,
            (false, true) => Move::Right,
            _ => Move::Hold,
        },
        ..Default::default()
    }
}

/// Reads a draft input: arrows move the highlight (one step per press), Enter or
/// Space takes the boon, Tab re-rolls. Edge-triggered, since a draft is a menu.
pub fn read_draft_input() -> Input {
    let paddle = if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
        Move::Left
    } else if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
        Move::Right
    } else {
        Move::Hold
    };
    Input {
        paddle,
        confirm: is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space),
        reroll: is_key_pressed(KeyCode::Tab),
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
            Kind::Explosive => EXPLOSIVE,
            Kind::Mover => MOVER,
            Kind::Spawner => SPAWNER,
        };
        let (bx, by) = (brick.x + 0.5, brick.y + 0.5);
        let (bw, bh) = (brick.width - 1.0, brick.height - 1.0);
        draw_rectangle(bx, by, bw, bh, colour);
        let (cx, cy) = (bx + bw / 2.0, by + bh / 2.0);
        match brick.kind {
            // A diagonal sheen marks the mirror.
            Kind::Mirror => draw_line(bx, by + bh, bx + bw, by, 1.0, MIRROR_SHEEN),
            // A bright core marks the explosive.
            Kind::Explosive => draw_rectangle(cx - 1.0, cy - 1.0, 2.0, 2.0, EXPLOSIVE_CORE),
            // A track line marks the mover.
            Kind::Mover => draw_line(bx + 1.0, cy, bx + bw - 1.0, cy, 1.0, MOVER_TRACK),
            // A small plus marks the spawner.
            Kind::Spawner => {
                draw_line(cx - 1.5, cy, cx + 1.5, cy, 1.0, SPAWNER_MARK);
                draw_line(cx, cy - 1.5, cx, cy + 1.5, 1.0, SPAWNER_MARK);
            }
            _ => {}
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

/// The card shown once a run is won or lost: how deep it got, its score, a
/// mode-specific `best_line` (best depth, or the Ascension tier), and — when the
/// run earned any — an `earned_line` naming what it unlocked. Drawn over the
/// frozen field.
pub fn run_summary(game: &Game, best_line: &str, earned_line: &str) {
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
        best_line,
        LOGICAL_HEIGHT / 2.0 + 12.0,
        HINT_SCALE,
        HUD_ACCENT,
    );
    // What the run added to the collection, when it added anything.
    if !earned_line.is_empty() {
        font::draw_centred(
            LOGICAL_WIDTH,
            earned_line,
            LOGICAL_HEIGHT / 2.0 + 26.0,
            HINT_SCALE,
            UNLOCK,
        );
    }
    font::draw_centred(
        LOGICAL_WIDTH,
        "R TO DIVE AGAIN   ESC TO QUIT",
        LOGICAL_HEIGHT / 2.0 + 44.0,
        HINT_SCALE,
        HUD_TEXT,
    );
}

/// The collection: every piece of content RIFT can offer, earned or still out
/// there. Unlocked items read in gold; locked ones show the condition that earns
/// them, so there is always a next goal.
pub fn draw_collection(unlocked: Unlocked) {
    clear_background(BACKGROUND);

    let have = meta::ALL.iter().filter(|c| unlocked.has(**c)).count();
    font::draw_centred(LOGICAL_WIDTH, "COLLECTION", 20.0, HEADING_SCALE, HUD_ACCENT);
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("{have} OF {} UNLOCKED", meta::ALL.len()),
        44.0,
        HINT_SCALE,
        HUD_TEXT,
    );

    let mut y = 64.0;
    font::draw_centred(LOGICAL_WIDTH, "BRICKS", y, HINT_SCALE, GUARDIAN);
    y += 14.0;
    for content in meta::ALL {
        if matches!(content, Content::Brick(_)) {
            collection_row(content, unlocked.has(content), y);
            y += 13.0;
        }
    }

    y += 8.0;
    font::draw_centred(LOGICAL_WIDTH, "BOONS", y, HINT_SCALE, GUARDIAN);
    y += 14.0;
    for content in meta::ALL {
        if matches!(content, Content::Boon(_)) {
            collection_row(content, unlocked.has(content), y);
            y += 13.0;
        }
    }

    font::draw_centred(
        LOGICAL_WIDTH,
        "ESC  BACK",
        LOGICAL_HEIGHT - 22.0,
        HINT_SCALE,
        HUD_TEXT,
    );
}

/// One collection row: the name alone once earned, or the name and the condition
/// that unlocks it while still locked.
fn collection_row(content: Content, unlocked: bool, y: f32) {
    let (text, colour) = if unlocked {
        (content.label().to_string(), UNLOCK)
    } else {
        (
            format!("{}   {}", content.label(), content.condition()),
            LOCKED,
        )
    };
    font::draw_centred(LOGICAL_WIDTH, &text, y, HINT_SCALE, colour);
}

/// The boon draft held between walls: three offers, keyboard-chosen, with a
/// reroll. Drawn over a dim scrim on the frozen field.
pub fn draw_draft(game: &Game) {
    draw_rectangle(0.0, 0.0, LOGICAL_WIDTH, LOGICAL_HEIGHT, DRAFT_SCRIM);
    font::draw_centred(
        LOGICAL_WIDTH,
        "DRAFT A BOON",
        34.0,
        HEADING_SCALE,
        HUD_ACCENT,
    );

    let highlight = game.draft_highlight();
    let mut y = 84.0;
    for (i, boon) in game.offers().iter().enumerate() {
        let colour = if i == highlight { HUD_ACCENT } else { HUD_TEXT };
        let title = format!("{}  [{}]", boon.title(), boon.home().label());
        let w = font::text_width(&title, HUD_SCALE);
        let x = (LOGICAL_WIDTH - w) / 2.0;
        font::draw(&title, x, y, HUD_SCALE, colour);
        if i == highlight {
            font::draw(
                ">",
                x - font::text_width("> ", HUD_SCALE),
                y,
                HUD_SCALE,
                colour,
            );
        }
        font::draw_centred(
            LOGICAL_WIDTH,
            boon.description(),
            y + 12.0,
            HINT_SCALE,
            HUD_TEXT,
        );
        y += 40.0;
    }

    let rerolls = game.rerolls();
    let hint = if rerolls > 0 {
        format!("ENTER TAKE    TAB REROLL ({rerolls})")
    } else {
        "ENTER TAKE".to_string()
    };
    font::draw_centred(LOGICAL_WIDTH, &hint, 276.0, HINT_SCALE, HUD_ACCENT);
    font::draw_centred(LOGICAL_WIDTH, "ARROWS CHOOSE", 288.0, HINT_SCALE, GRAY);
}
