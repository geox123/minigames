//! Hand-authored pixel sprites, as const string-art bitmaps — no asset files, and
//! nothing traced from the original: these are STEPFALL's own creatures, drawn to
//! evoke the era per [ADR 0003] and [ADR 0004]. Each row is an equal-length
//! string; any non-space cell is a lit pixel, drawn one logical unit square.
//!
//! [ADR 0003]: ../../../docs/adr/0003-code-drawn-visuals.md
//! [ADR 0004]: ../../../docs/adr/0004-space-invaders-ip-recheck.md

use macroquad::prelude::*;
use stepfall_core::BombKind;

/// A sprite: rows of equal-length string-art.
pub type Sprite = &'static [&'static str];

/// Draws `sprite` with its top-left at `(x, y)`, one logical unit per pixel.
pub fn blit(sprite: Sprite, x: f32, y: f32, color: Color) {
    for (row, line) in sprite.iter().enumerate() {
        for (col, byte) in line.bytes().enumerate() {
            if byte != b' ' {
                draw_rectangle(x + col as f32, y + row as f32, 1.0, 1.0, color);
            }
        }
    }
}

/// Draws `sprite` centred inside the box at `(x, y)` sized `(w, h)`.
pub fn blit_centred(sprite: Sprite, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let sw = sprite.first().map_or(0, |r| r.len()) as f32;
    let sh = sprite.len() as f32;
    blit(sprite, x + (w - sw) / 2.0, y + (h - sh) / 2.0, color);
}

/// The two march frames for an invader in `row` (0 the top). The top row is the
/// small sentinel, the middle two the strider, the bottom two the lurker.
pub fn invader_frames(row: usize) -> [Sprite; 2] {
    match row {
        0 => SENTINEL,
        1 | 2 => STRIDER,
        _ => LURKER,
    }
}

/// The bomb sprite for a falling bomb of `kind`.
pub fn bomb_sprite(kind: BombKind) -> Sprite {
    match kind {
        BombKind::Rolling => BOMB_ROLLING,
        BombKind::Squiggly => BOMB_SQUIGGLY,
        BombKind::Plunger => BOMB_PLUNGER,
    }
}

/// The small top-row invader, worth the most — two frames, legs shuffling.
const SENTINEL: [Sprite; 2] = [
    &[
        "   XX   ", "  XXXX  ", " X XX X ", " XXXXXX ", " XXXXXX ", "  X  X  ", " XX  XX ",
        "X  XX  X",
    ],
    &[
        "   XX   ", "  XXXX  ", " X XX X ", " XXXXXX ", " XXXXXX ", "  X  X  ", "  X  X  ",
        " X    X ",
    ],
];

/// The middle-rows invader — antennae and legs swap between frames.
const STRIDER: [Sprite; 2] = [
    &[
        "  X     X  ",
        "   X   X   ",
        "  XXXXXXX  ",
        " XX XXX XX ",
        "XXXXXXXXXXX",
        "X XXXXXXX X",
        "X X     X X",
        "   XX XX   ",
    ],
    &[
        "X         X",
        " X       X ",
        "  XXXXXXX  ",
        " XX XXX XX ",
        "XXXXXXXXXXX",
        " XXXXXXXXX ",
        "  X     X  ",
        " X       X ",
    ],
];

/// The big bottom-rows invader — the widest, legs shuffling underneath.
const LURKER: [Sprite; 2] = [
    &[
        "    XXX    ",
        " XXXXXXXXX ",
        "XXXXXXXXXXX",
        "XX XXXXX XX",
        "XXXXXXXXXXX",
        "  XX X XX  ",
        " XX  X  XX ",
        "X X     X X",
    ],
    &[
        "    XXX    ",
        " XXXXXXXXX ",
        "XXXXXXXXXXX",
        "XX XXXXX XX",
        "XXXXXXXXXXX",
        "  XXX XXX  ",
        " X  X X  X ",
        "  X     X  ",
    ],
];

/// The player's cannon, barrel up.
pub const CANNON: Sprite = &[
    "      X      ",
    "     XXX     ",
    "     XXX     ",
    "  XXXXXXXXX  ",
    " XXXXXXXXXXX ",
    "XXXXXXXXXXXXX",
    "XXXXXXXXXXXXX",
    "XXXXXXXXXXXXX",
];

/// The mystery saucer, windows and landing lights beneath.
pub const SAUCER: Sprite = &[
    "     XXXXXX     ",
    "   XXXXXXXXXX   ",
    "  XXXXXXXXXXXX  ",
    " XXXXXXXXXXXXXX ",
    "XXXXXXXXXXXXXXXX",
    " XX XX XX XX XX ",
    "   X  X  X  X   ",
];

/// The three bombs, each with its own falling shape.
const BOMB_ROLLING: Sprite = &[" X ", "X  ", " X ", "  X", " X ", "X  ", " X "];
const BOMB_SQUIGGLY: Sprite = &["  X", " X ", "X  ", " X ", "  X", " X ", "X  "];
const BOMB_PLUNGER: Sprite = &[" X ", "XXX", " X ", " X ", " X ", " X ", " X "];

/// The burst a destroyed invader leaves for a moment.
pub const BLAST: Sprite = &[
    "  X     X  ",
    "X  X   X  X",
    " X  X X  X ",
    "  X XXX X  ",
    "XXXXXXXXXXX",
    "  X XXX X  ",
    " X  X X  X ",
    "X  X   X  X",
];

/// The wider scatter of a cannon blown apart.
pub const CANNON_BLAST: Sprite = &[
    "X X   X   X X",
    " X X X X X X ",
    "  XX XXX XX  ",
    " XXXXXXXXXXX ",
    "  XX XXX XX  ",
    " X X X X X X ",
];
