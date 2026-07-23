//! Drawing the core's state onto the logical canvas.
//!
//! Everything here works in the core's logical units; scaling to the real
//! window happens once, when the canvas is blitted to the screen.

use macroquad::prelude::*;
use pong_core::{
    BALL_SIZE, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH, PADDLE_HEIGHT, PADDLE_WIDTH, Phase, Players,
    Side,
};

use crate::app::Mode;
use crate::font;

/// Text sizes, as the pixel scale each is drawn at.
const TITLE_SCALE: f32 = 5.0;
const HEADING_SCALE: f32 = 3.0;
const OPTION_SCALE: f32 = 2.0;
const HINT_SCALE: f32 = 1.0;

/// PULSE's neon palette, a deliberate contrast to the Faithful's white-on-black.
const NEON_BG: Color = color_u8!(10, 8, 20, 255);
const NEON_LEFT: Color = color_u8!(60, 240, 255, 255);
const NEON_RIGHT: Color = color_u8!(255, 70, 200, 255);
const NEON_BALL: Color = color_u8!(255, 245, 120, 255);
const NEON_DIM: Color = color_u8!(90, 80, 140, 255);
const NEON_SLOW: Color = color_u8!(180, 160, 255, 255);

/// The dashed line down the middle of the field.
const NET_WIDTH: f32 = 4.0;
const NET_DASH: f32 = 8.0;
const NET_GAP: f32 = 8.0;

/// The score is drawn in the chunky seven-segment digits the original used,
/// rather than in a font.
const DIGIT_WIDTH: f32 = 20.0;
const DIGIT_HEIGHT: f32 = 32.0;
const DIGIT_STROKE: f32 = 4.0;
const DIGIT_GAP: f32 = 6.0;
const SCORE_TOP: f32 = 20.0;
/// How far each score sits from the middle of the field.
const SCORE_OFFSET: f32 = 56.0;

/// Draws one frame of the match onto the logical canvas.
pub fn draw(game: &Game) {
    clear_background(BLACK);

    draw_net(WHITE);
    draw_number(
        game.score(Side::Left),
        LOGICAL_WIDTH / 2.0 - SCORE_OFFSET,
        WHITE,
    );
    draw_number(
        game.score(Side::Right),
        LOGICAL_WIDTH / 2.0 + SCORE_OFFSET,
        WHITE,
    );

    for side in [Side::Left, Side::Right] {
        let paddle = game.paddle(side);
        draw_rectangle(paddle.x, paddle.y, PADDLE_WIDTH, PADDLE_HEIGHT, WHITE);
    }

    let ball = game.ball();
    let half = BALL_SIZE / 2.0;
    draw_rectangle(ball.x - half, ball.y - half, BALL_SIZE, BALL_SIZE, WHITE);

    if let Phase::GameOver { winner } = game.phase() {
        let who = match winner {
            Side::Left => "LEFT PLAYER WINS",
            Side::Right => "RIGHT PLAYER WINS",
        };
        font::draw_centred(who, LOGICAL_HEIGHT / 2.0 - 20.0, OPTION_SCALE, WHITE);
        font::draw_centred(
            "PRESS R TO PLAY AGAIN",
            LOGICAL_HEIGHT / 2.0 + 6.0,
            HINT_SCALE,
            WHITE,
        );
    }
}

/// Draws one frame of a PULSE match onto the logical canvas, in its neon look.
pub fn draw_pulse(game: &pong_remix_core::Game) {
    use pong_remix_core::{Phase as PPhase, Side as PSide};

    clear_background(NEON_BG);

    draw_net(NEON_DIM);
    draw_number(
        game.score(PSide::Left),
        LOGICAL_WIDTH / 2.0 - SCORE_OFFSET,
        NEON_LEFT,
    );
    draw_number(
        game.score(PSide::Right),
        LOGICAL_WIDTH / 2.0 + SCORE_OFFSET,
        NEON_RIGHT,
    );

    for (side, colour) in [(PSide::Left, NEON_LEFT), (PSide::Right, NEON_RIGHT)] {
        let paddle = game.paddle(side);
        draw_rectangle(
            paddle.x,
            paddle.y,
            pong_remix_core::PADDLE_WIDTH,
            game.paddle_height(side),
            colour,
        );
        // A shielded goal glows with a bar down the wall behind the paddle.
        if game.has_shield(side) {
            let x = match side {
                PSide::Left => 0.0,
                PSide::Right => LOGICAL_WIDTH - 2.0,
            };
            draw_rectangle(x, 0.0, 2.0, LOGICAL_HEIGHT, colour);
        }
    }

    // The pickup, drawn as a hollow diamond so it reads as a target.
    if let Some(pickup) = game.pickup() {
        draw_pickup(pickup);
    }

    let half = pong_remix_core::BALL_SIZE / 2.0;
    for ball in game.balls() {
        draw_rectangle(
            ball.x - half,
            ball.y - half,
            pong_remix_core::BALL_SIZE,
            pong_remix_core::BALL_SIZE,
            NEON_BALL,
        );
    }

    // Each player's power-shot charge, as a bar climbing beside their paddle.
    draw_charge(game.charge(PSide::Left), 6.0, NEON_LEFT);
    draw_charge(
        game.charge(PSide::Right),
        LOGICAL_WIDTH - 6.0 - CHARGE_BAR_W,
        NEON_RIGHT,
    );

    if let PPhase::GameOver { winner } = game.phase() {
        let (who, colour) = match winner {
            PSide::Left => ("LEFT PLAYER WINS", NEON_LEFT),
            PSide::Right => ("RIGHT PLAYER WINS", NEON_RIGHT),
        };
        font::draw_centred(who, LOGICAL_HEIGHT / 2.0 - 20.0, OPTION_SCALE, colour);
        font::draw_centred(
            "PRESS R TO PLAY AGAIN",
            LOGICAL_HEIGHT / 2.0 + 6.0,
            HINT_SCALE,
            NEON_BALL,
        );
    }
}

/// Draws a pickup as a bright square outline — a target to steer the ball into.
fn draw_pickup(pickup: pong_remix_core::Pickup) {
    use pong_remix_core::PickupKind;
    let size = pong_remix_core::PICKUP_SIZE;
    // A colour per kind, so a glance tells them apart.
    let colour = match pickup.kind {
        PickupKind::Multiball => NEON_BALL,
        PickupKind::Shield => NEON_LEFT,
        PickupKind::Widen => NEON_RIGHT,
        PickupKind::SlowMo => NEON_SLOW,
    };
    draw_rectangle_lines(
        pickup.x - size / 2.0,
        pickup.y - size / 2.0,
        size,
        size,
        2.0,
        colour,
    );
    // A pip in the centre so it reads even at a glance.
    draw_rectangle(pickup.x - 1.0, pickup.y - 1.0, 2.0, 2.0, colour);
}

/// A power-shot charge meter, filling from the bottom.
const CHARGE_BAR_W: f32 = 4.0;
const CHARGE_BAR_H: f32 = 48.0;

fn draw_charge(charge: f32, x: f32, colour: Color) {
    let y = LOGICAL_HEIGHT / 2.0 - CHARGE_BAR_H / 2.0;
    // The empty track, dim.
    draw_rectangle(x, y, CHARGE_BAR_W, CHARGE_BAR_H, NEON_DIM);
    // The filled portion climbs from the bottom, brightening when full.
    let filled = CHARGE_BAR_H * charge.clamp(0.0, 1.0);
    let fill_colour = if charge >= 1.0 { NEON_BALL } else { colour };
    draw_rectangle(
        x,
        y + CHARGE_BAR_H - filled,
        CHARGE_BAR_W,
        filled,
        fill_colour,
    );
}

fn draw_net(colour: Color) {
    let x = (LOGICAL_WIDTH - NET_WIDTH) / 2.0;
    let mut y = 0.0;
    while y < LOGICAL_HEIGHT {
        draw_rectangle(x, y, NET_WIDTH, NET_DASH.min(LOGICAL_HEIGHT - y), colour);
        y += NET_DASH + NET_GAP;
    }
}

/// Draws `value` centred on `centre_x`, in seven-segment digits.
fn draw_number(value: u32, centre_x: f32, colour: Color) {
    let digits: Vec<u32> = value
        .to_string()
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();

    let span = digits.len() as f32 * DIGIT_WIDTH + (digits.len() as f32 - 1.0) * DIGIT_GAP;
    let mut x = centre_x - span / 2.0;
    for digit in digits {
        draw_digit(digit, x, SCORE_TOP, colour);
        x += DIGIT_WIDTH + DIGIT_GAP;
    }
}

/// The seven segments, in the usual order: top, top-right, bottom-right,
/// bottom, bottom-left, top-left, middle.
const SEGMENTS_PER_DIGIT: [[bool; 7]; 10] = [
    [true, true, true, true, true, true, false],     // 0
    [false, true, true, false, false, false, false], // 1
    [true, true, false, true, true, false, true],    // 2
    [true, true, true, true, false, false, true],    // 3
    [false, true, true, false, false, true, true],   // 4
    [true, false, true, true, false, true, true],    // 5
    [true, false, true, true, true, true, true],     // 6
    [true, true, true, false, false, false, false],  // 7
    [true, true, true, true, true, true, true],      // 8
    [true, true, true, true, false, true, true],     // 9
];

fn draw_digit(digit: u32, x: f32, y: f32, colour: Color) {
    let Some(lit) = SEGMENTS_PER_DIGIT.get(digit as usize) else {
        return;
    };

    let long = DIGIT_HEIGHT / 2.0 + DIGIT_STROKE / 2.0;
    let far_x = x + DIGIT_WIDTH - DIGIT_STROKE;
    let mid_y = y + (DIGIT_HEIGHT - DIGIT_STROKE) / 2.0;
    let low_y = y + DIGIT_HEIGHT - DIGIT_STROKE;

    let segments = [
        (x, y, DIGIT_WIDTH, DIGIT_STROKE),     // top
        (far_x, y, DIGIT_STROKE, long),        // top-right
        (far_x, mid_y, DIGIT_STROKE, long),    // bottom-right
        (x, low_y, DIGIT_WIDTH, DIGIT_STROKE), // bottom
        (x, mid_y, DIGIT_STROKE, long),        // bottom-left
        (x, y, DIGIT_STROKE, long),            // top-left
        (x, mid_y, DIGIT_WIDTH, DIGIT_STROKE), // middle
    ];

    for (segment, on) in segments.iter().zip(lit) {
        if *on {
            draw_rectangle(segment.0, segment.1, segment.2, segment.3, colour);
        }
    }
}

/// Draws the Collection's mode-select: the two takes Pong ships. Both are now
/// playable — the Faithful, and PULSE, its Remix.
pub fn mode_select(highlight: Mode) {
    clear_background(BLACK);

    font::draw_centred("PONG", 40.0, TITLE_SCALE, WHITE);
    font::draw_centred("THE FAITHFUL AND THE REMIX", 82.0, HINT_SCALE, GRAY);
    option("FAITHFUL", 118.0, highlight == Mode::Faithful, false);
    option("PULSE", 150.0, highlight == Mode::Remix, false);
    font::draw_centred(
        "ARROWS TO CHOOSE   ENTER TO SELECT",
        208.0,
        HINT_SCALE,
        GRAY,
    );
}

/// Draws the screen that picks one or two players before a match.
pub fn player_select(highlight: Players) {
    clear_background(BLACK);

    font::draw_centred("PONG", 44.0, TITLE_SCALE, WHITE);
    font::draw_centred("FAITHFUL", 90.0, OPTION_SCALE, GRAY);
    option("1 PLAYER", 128.0, highlight == Players::One, false);
    option("2 PLAYERS", 160.0, highlight == Players::Two, false);
    font::draw_centred("ARROWS TO CHOOSE   ENTER TO START", 208.0, HINT_SCALE, GRAY);
}

/// Draws the PULSE player-select, in its neon look.
pub fn pulse_player_select(highlight: Players) {
    clear_background(NEON_BG);

    font::draw_centred("PULSE", 44.0, TITLE_SCALE, NEON_LEFT);
    font::draw_centred("THE REMIX", 90.0, OPTION_SCALE, NEON_RIGHT);
    pulse_option("1 PLAYER", 128.0, highlight == Players::One);
    pulse_option("2 PLAYERS", 160.0, highlight == Players::Two);
    font::draw_centred(
        "ARROWS TO CHOOSE   ENTER TO START",
        208.0,
        HINT_SCALE,
        NEON_DIM,
    );
}

/// One neon menu row, with a caret when highlighted.
fn pulse_option(label: &str, y: f32, highlighted: bool) {
    let width = font::text_width(label, OPTION_SCALE);
    let x = (LOGICAL_WIDTH - width) / 2.0;
    font::draw(label, x, y, OPTION_SCALE, NEON_BALL);
    if highlighted {
        font::draw(
            ">",
            x - font::text_width("> ", OPTION_SCALE),
            y,
            OPTION_SCALE,
            NEON_BALL,
        );
    }
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
    font::draw_centred("PAUSED", LOGICAL_HEIGHT / 2.0 - 16.0, HEADING_SCALE, WHITE);
    font::draw_centred(
        "P RESUME   F FULLSCREEN   ESC QUIT",
        LOGICAL_HEIGHT / 2.0 + 16.0,
        HINT_SCALE,
        WHITE,
    );
}
