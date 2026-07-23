//! Drawing the core's state onto the logical canvas.
//!
//! Everything here works in the core's logical units; scaling to the real
//! window happens once, when the canvas is blitted to the screen.

use macroquad::prelude::*;
use pong_core::{
    BALL_SIZE, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH, PADDLE_HEIGHT, PADDLE_WIDTH, Phase, Side,
};

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

    draw_net();
    draw_number(game.score(Side::Left), LOGICAL_WIDTH / 2.0 - SCORE_OFFSET);
    draw_number(game.score(Side::Right), LOGICAL_WIDTH / 2.0 + SCORE_OFFSET);

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
        centred_text(who, LOGICAL_HEIGHT / 2.0 - 8.0, 16);
        centred_text("PRESS R TO PLAY AGAIN", LOGICAL_HEIGHT / 2.0 + 12.0, 10);
    }
}

fn draw_net() {
    let x = (LOGICAL_WIDTH - NET_WIDTH) / 2.0;
    let mut y = 0.0;
    while y < LOGICAL_HEIGHT {
        draw_rectangle(x, y, NET_WIDTH, NET_DASH.min(LOGICAL_HEIGHT - y), WHITE);
        y += NET_DASH + NET_GAP;
    }
}

/// Draws `value` centred on `centre_x`, in seven-segment digits.
fn draw_number(value: u32, centre_x: f32) {
    let digits: Vec<u32> = value
        .to_string()
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();

    let span = digits.len() as f32 * DIGIT_WIDTH + (digits.len() as f32 - 1.0) * DIGIT_GAP;
    let mut x = centre_x - span / 2.0;
    for digit in digits {
        draw_digit(digit, x, SCORE_TOP);
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

fn draw_digit(digit: u32, x: f32, y: f32) {
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
            draw_rectangle(segment.0, segment.1, segment.2, segment.3, WHITE);
        }
    }
}

/// Draws a line of text centred across the field, with `y` as its baseline.
pub fn centred_text(text: &str, y: f32, size: u16) {
    let width = measure_text(text, None, size, 1.0).width;
    draw_text(text, (LOGICAL_WIDTH - width) / 2.0, y, size as f32, WHITE);
}
