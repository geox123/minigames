//! Drawing the core's state onto the logical canvas, in the era's stark look:
//! a monochrome game of hand-authored sprites (see [`crate::sprites`]) under the
//! cabinet's colour-overlay bands — red high, green low.
//!
//! Everything here works in the core's logical units; scaling to the real
//! window happens once, when the canvas is blitted to the screen.

use macroquad::prelude::*;
use stepfall_core::{
    BUNKER_CELL, CANNON_HEIGHT, CANNON_WIDTH, Game, INVADER_HEIGHT, INVADER_WIDTH, LOGICAL_HEIGHT,
    LOGICAL_WIDTH, Phase, SHOT_HEIGHT, SHOT_WIDTH,
};

use crate::app::Mode;
use crate::sprites;
use shell_kit::font;

/// Text sizes, as the pixel scale each is drawn at.
const TITLE_SCALE: f32 = 4.0;
const OPTION_SCALE: f32 = 2.0;
const HINT_SCALE: f32 = 1.0;

/// The cabinet's two colour-overlay strips: green low, red high, over an
/// otherwise white (monochrome) game.
const GROUND: Color = color_u8!(60, 220, 90, 255);
const RED_BAND: Color = color_u8!(220, 70, 70, 255);
/// Where the bands fall: red above this line, green below the other.
const RED_BAND_BELOW: f32 = 56.0;
const GREEN_BAND_ABOVE: f32 = 172.0;
/// A little cannon icon per remaining life, along the top-right.
const LIFE_ICON_W: f32 = 11.0;
const LIFE_ICON_H: f32 = 4.0;
const LIFE_ICON_GAP: f32 = 4.0;

/// The overlay colour at height `y`: red up top, green down low, white between.
fn band_tint(y: f32) -> Color {
    if y < RED_BAND_BELOW {
        RED_BAND
    } else if y >= GREEN_BAND_ABOVE {
        GROUND
    } else {
        WHITE
    }
}

/// Lays the two colour strips over the field as faint tints, the way the
/// cellophane overlays coloured the arcade's monochrome tube.
fn draw_bands() {
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        RED_BAND_BELOW,
        color_u8!(220, 70, 70, 24),
    );
    draw_rectangle(
        0.0,
        GREEN_BAND_ABOVE,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT - GREEN_BAND_ABOVE,
        color_u8!(60, 220, 90, 24),
    );
}

/// Draws one frame of the game onto the logical canvas. `best` is the session's
/// high score, shown in the HUD.
pub fn draw(game: &Game, best: u32) {
    clear_background(BLACK);
    draw_bands();

    // The mystery saucer, when it's crossing.
    if let Some(saucer) = game.saucer() {
        sprites::blit(sprites::SAUCER, saucer.x, saucer.y, band_tint(saucer.y));
    }

    // The invaders — each row's sprite, its two frames alternating with the march.
    let frame = game.march_frame() as usize;
    for invader in game.invaders() {
        let sprite = sprites::invader_frames(invader.row)[frame];
        sprites::blit_centred(
            sprite,
            invader.x,
            invader.y,
            INVADER_WIDTH,
            INVADER_HEIGHT,
            band_tint(invader.y),
        );
    }

    // Explosions where invaders were destroyed.
    for blast in game.blasts() {
        sprites::blit_centred(
            sprites::BLAST,
            blast.x,
            blast.y,
            INVADER_WIDTH,
            INVADER_HEIGHT,
            band_tint(blast.y),
        );
    }

    // The bunkers — green cover, wearing holes as it is eaten from both sides.
    for block in game.bunker_blocks() {
        draw_rectangle(block.x, block.y, BUNKER_CELL, BUNKER_CELL, GROUND);
    }

    // Bombs falling, and the player's shot climbing.
    for bomb in game.bombs() {
        sprites::blit(
            sprites::bomb_sprite(bomb.kind),
            bomb.x,
            bomb.y,
            band_tint(bomb.y),
        );
    }
    if let Some(shot) = game.shot() {
        draw_rectangle(shot.x, shot.y, SHOT_WIDTH, SHOT_HEIGHT, band_tint(shot.y));
    }

    // The cannon — its explosion while it dies, otherwise the cannon itself.
    let cannon = game.cannon();
    if game.cannon_dying() {
        sprites::blit_centred(
            sprites::CANNON_BLAST,
            cannon.x,
            cannon.y,
            CANNON_WIDTH,
            CANNON_HEIGHT,
            GROUND,
        );
    } else {
        sprites::blit(sprites::CANNON, cannon.x, cannon.y, GROUND);
    }

    // The ground line the cannon rides along.
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
    font::draw(
        &format!("{:04}", game.score()),
        6.0,
        6.0,
        HINT_SCALE,
        RED_BAND,
    );

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

/// HAILFALL's neon palette — the Remix's vivid look, against the Faithful's mono.
const NEON_SHIP: Color = color_u8!(90, 220, 255, 255);
const NEON_GLOW: Color = color_u8!(40, 120, 160, 255);
const NEON_BULLET: Color = color_u8!(255, 240, 140, 255);
const NEON_ENEMY: Color = color_u8!(255, 90, 170, 255);

/// Draws one frame of HAILFALL — the Remix — onto the logical canvas. The swarm,
/// the ship and its fire; the enemy fire, the tools and the HUD follow in later
/// tickets.
pub fn draw_remix(game: &stepfall_remix_core::Game) {
    use stepfall_remix_core::{
        ENEMY_HEIGHT, ENEMY_WIDTH, PLAYER_BULLET_HEIGHT, PLAYER_BULLET_WIDTH, SHIP_HEIGHT,
        SHIP_WIDTH,
    };
    clear_background(color_u8!(4, 6, 14, 255));

    // The squadron.
    for enemy in game.enemies() {
        draw_rectangle(enemy.x, enemy.y, ENEMY_WIDTH, ENEMY_HEIGHT, NEON_ENEMY);
        draw_rectangle(
            enemy.x + 2.0,
            enemy.y + 3.0,
            ENEMY_WIDTH - 4.0,
            2.0,
            color_u8!(20, 8, 16, 255),
        );
    }

    // The ship's fire, climbing.
    for bullet in game.bullets() {
        draw_rectangle(
            bullet.x,
            bullet.y,
            PLAYER_BULLET_WIDTH,
            PLAYER_BULLET_HEIGHT,
            NEON_BULLET,
        );
    }

    // The ship: a bright arrowhead over a glow.
    let ship = game.ship();
    draw_rectangle(
        ship.x - 1.0,
        ship.y - 1.0,
        SHIP_WIDTH + 2.0,
        SHIP_HEIGHT + 2.0,
        NEON_GLOW,
    );
    let cx = ship.x + SHIP_WIDTH / 2.0;
    draw_triangle(
        vec2(cx, ship.y),
        vec2(ship.x, ship.y + SHIP_HEIGHT),
        vec2(ship.x + SHIP_WIDTH, ship.y + SHIP_HEIGHT),
        NEON_SHIP,
    );
}

/// Draws the Collection's mode-select: the two takes STEPFALL ships. Both are now
/// playable — the Faithful, and HAILFALL, the Remix.
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
    option("HAILFALL", 160.0, highlight == Mode::Remix, false);
    if highlight == Mode::Remix {
        font::draw_centred(
            LOGICAL_WIDTH,
            "THE BULLET-HELL REMIX",
            182.0,
            HINT_SCALE,
            GRAY,
        );
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
