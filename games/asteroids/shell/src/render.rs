//! Drawing the Asteroids field: bright vector outlines on black with a soft glow —
//! the vector cabinet's look. Everything here is macroquad glue; the shapes are
//! authored in code, none traced from the original
//! ([ADR 0003](../../../docs/adr/0003-code-drawn-visuals.md)). Each rock size has
//! its own irregular silhouette, and explosions scatter fading line-shards.

use asteroids_core::{
    Asteroid, AsteroidSize, Blast, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH, SHIP_RADIUS, Saucer,
    SaucerBullet, Ship, Shot,
};
use macroquad::prelude::*;
use shell_kit::font;
use std::f32::consts::TAU;

use crate::app::Mode;

const TITLE_SCALE: f32 = 4.0;
const OPTION_SCALE: f32 = 2.0;
const HINT_SCALE: f32 = 1.0;

/// The line weight of the vector outlines, in logical units.
const STROKE: f32 = 2.0;

/// The saucer's hull, and the warm colour of its fire — set apart from the player's
/// white so the threat reads at a glance.
const SAUCER_COLOR: Color = color_u8!(140, 220, 255, 255);
const SAUCER_FIRE_COLOR: Color = color_u8!(255, 120, 90, 255);

/// One irregular rock silhouette per size: radius multipliers at evenly spaced
/// angles, so each size reads as its own lumpy polygon. Authored in code, nothing
/// traced from the original (ADR 0002 / 0003).
const LARGE_ROCK: [f32; 12] = [
    1.0, 0.82, 1.06, 0.72, 0.98, 0.88, 1.08, 0.78, 1.0, 0.9, 1.05, 0.8,
];
const MEDIUM_ROCK: [f32; 11] = [
    0.95, 1.08, 0.75, 1.0, 0.85, 1.05, 0.7, 0.98, 0.9, 1.06, 0.82,
];
const SMALL_ROCK: [f32; 9] = [1.0, 0.78, 1.08, 0.85, 0.95, 0.72, 1.05, 0.9, 0.82];

/// Draws a vector line with a soft glow — a wide, dim underlay beneath the crisp
/// bright line, evoking the vector cabinet's bloom.
fn stroke(x1: f32, y1: f32, x2: f32, y2: f32, color: Color) {
    draw_line(x1, y1, x2, y2, STROKE * 2.6, dim(color));
    draw_line(x1, y1, x2, y2, STROKE, color);
}

/// Draws a glowing blip — a small bright dot over a dim halo.
fn blip(x: f32, y: f32, r: f32, color: Color) {
    draw_circle(x, y, r * 2.2, dim(color));
    draw_circle(x, y, r, color);
}

/// The same colour at a low alpha, for the glow underlays.
fn dim(color: Color) -> Color {
    Color::new(color.r, color.g, color.b, 0.22)
}

/// Draws a live game: the rocks, shots and explosions, the ship (while it is on the
/// field), and the HUD (with the session `best`).
pub fn draw(game: &Game, best: u32) {
    clear_background(BLACK);
    for rock in game.asteroids() {
        draw_asteroid(rock);
    }
    for shot in game.shots() {
        draw_shot(shot);
    }
    if let Some(saucer) = game.saucer() {
        draw_saucer(saucer);
    }
    for bullet in game.saucer_bullets() {
        draw_saucer_bullet(bullet);
    }
    for blast in game.blasts() {
        draw_blast(blast);
    }
    if game.ship_alive() && ship_visible(game) {
        draw_ship(game.ship());
    }
    draw_hud(game, best);
}

/// Whether to draw the ship this frame — a fresh, protected ship blinks to show it
/// cannot yet be hit.
fn ship_visible(game: &Game) -> bool {
    !game.ship_invulnerable() || ((get_time() * 10.0) as i64).rem_euclid(2) == 0
}

/// A shot: a small glowing blip.
fn draw_shot(shot: Shot) {
    blip(shot.x, shot.y, 2.5, WHITE);
}

/// The saucer: a vector hull, wrapped vertically (it leaves at the horizontal edges
/// rather than wrapping, so only its lane wraps).
fn draw_saucer(saucer: Saucer) {
    let r = saucer.size.radius();
    for oy in axis_offsets(saucer.y, r, LOGICAL_HEIGHT)
        .into_iter()
        .flatten()
    {
        draw_saucer_hull(saucer.x, saucer.y + oy, r);
    }
}

/// One saucer hull centred at `(cx, cy)`: an elongated hexagon with a small dome.
fn draw_saucer_hull(cx: f32, cy: f32, r: f32) {
    let hull = [
        (-r, 0.0),
        (-r * 0.45, -r * 0.4),
        (r * 0.45, -r * 0.4),
        (r, 0.0),
        (r * 0.45, r * 0.45),
        (-r * 0.45, r * 0.45),
    ];
    for i in 0..hull.len() {
        let (ax, ay) = hull[i];
        let (bx, by) = hull[(i + 1) % hull.len()];
        stroke(cx + ax, cy + ay, cx + bx, cy + by, SAUCER_COLOR);
    }
    stroke(cx - r * 0.25, cy - r * 0.4, cx, cy - r * 0.75, SAUCER_COLOR);
    stroke(cx, cy - r * 0.75, cx + r * 0.25, cy - r * 0.4, SAUCER_COLOR);
}

/// A saucer bullet: a small warm glowing blip, set apart from the player's white
/// fire.
fn draw_saucer_bullet(bullet: SaucerBullet) {
    blip(bullet.x, bullet.y, 2.5, SAUCER_FIRE_COLOR);
}

/// An explosion: a burst of line-shards flying outward from where a rock or the ship
/// broke, expanding and fading over the blast's life.
fn draw_blast(blast: Blast) {
    const SHARDS: usize = 8;
    let p = blast.progress;
    let inner = 3.0 + 10.0 * p;
    let outer = 6.0 + 26.0 * p;
    let color = Color::new(1.0, 1.0, 1.0, (1.0 - p).clamp(0.0, 1.0));
    for i in 0..SHARDS {
        // A slight turn as they fly, so the burst reads as motion, not a static star.
        let a = TAU * i as f32 / SHARDS as f32 + p * 0.6;
        draw_line(
            blast.x + a.cos() * inner,
            blast.y + a.sin() * inner,
            blast.x + a.cos() * outer,
            blast.y + a.sin() * outer,
            STROKE,
            color,
        );
    }
}

/// The HUD: the running score and session best, the wave, and the ships left drawn
/// as little icons.
fn draw_hud(game: &Game, best: u32) {
    font::draw(&game.score().to_string(), 20.0, 20.0, OPTION_SCALE, WHITE);
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("BEST {best}"),
        24.0,
        HINT_SCALE,
        GRAY,
    );
    let wave = format!("WAVE {}", game.wave());
    font::draw(
        &wave,
        LOGICAL_WIDTH - font::text_width(&wave, HINT_SCALE) - 20.0,
        24.0,
        HINT_SCALE,
        GRAY,
    );
    for i in 0..game.lives() {
        draw_ship_icon(28.0 + i as f32 * 24.0, 60.0);
    }
}

/// A small upward-pointing ship, for the lives readout.
fn draw_ship_icon(x: f32, y: f32) {
    let s = 8.0;
    stroke(x, y - s, x - s * 0.7, y + s * 0.7, WHITE);
    stroke(x, y - s, x + s * 0.7, y + s * 0.7, WHITE);
    stroke(x - s * 0.7, y + s * 0.7, x + s * 0.7, y + s * 0.7, WHITE);
}

/// Draws the game-over banner over the final field, with the run's score and the
/// session best.
pub fn game_over(game: &Game, best: u32) {
    font::draw_centred(
        LOGICAL_WIDTH,
        "GAME OVER",
        LOGICAL_HEIGHT / 2.0 - 44.0,
        TITLE_SCALE,
        WHITE,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("SCORE {}", game.score()),
        LOGICAL_HEIGHT / 2.0 + 20.0,
        OPTION_SCALE,
        WHITE,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("BEST {best}"),
        LOGICAL_HEIGHT / 2.0 + 50.0,
        HINT_SCALE,
        GRAY,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "R RESTART   ESC QUIT",
        LOGICAL_HEIGHT / 2.0 + 72.0,
        HINT_SCALE,
        GRAY,
    );
}

/// The ship: a vector triangle pointing along its facing, with a flickering thrust
/// flame behind it while it thrusts.
fn draw_ship(ship: Ship) {
    // Forward along the facing (angle 0 points up), and right, perpendicular to it.
    let (fx, fy) = (ship.angle.sin(), -ship.angle.cos());
    let (rx, ry) = (ship.angle.cos(), ship.angle.sin());

    let nose = vec2(
        ship.x + fx * SHIP_RADIUS * 1.3,
        ship.y + fy * SHIP_RADIUS * 1.3,
    );
    let base_x = ship.x - fx * SHIP_RADIUS * 0.8;
    let base_y = ship.y - fy * SHIP_RADIUS * 0.8;
    let left = vec2(
        base_x - rx * SHIP_RADIUS * 0.9,
        base_y - ry * SHIP_RADIUS * 0.9,
    );
    let right = vec2(
        base_x + rx * SHIP_RADIUS * 0.9,
        base_y + ry * SHIP_RADIUS * 0.9,
    );

    draw_wrapped(ship.x, ship.y, SHIP_RADIUS * 1.3, |ox, oy| {
        stroke(nose.x + ox, nose.y + oy, left.x + ox, left.y + oy, WHITE);
        stroke(nose.x + ox, nose.y + oy, right.x + ox, right.y + oy, WHITE);
        stroke(left.x + ox, left.y + oy, right.x + ox, right.y + oy, WHITE);
    });

    // A thrust flame that blinks on and off, like the original's.
    if ship.thrusting && ((get_time() * 20.0) as i64).rem_euclid(2) == 0 {
        let fl = vec2(
            base_x - rx * SHIP_RADIUS * 0.45,
            base_y - ry * SHIP_RADIUS * 0.45,
        );
        let fr = vec2(
            base_x + rx * SHIP_RADIUS * 0.45,
            base_y + ry * SHIP_RADIUS * 0.45,
        );
        let tip = vec2(
            ship.x - fx * SHIP_RADIUS * 1.7,
            ship.y - fy * SHIP_RADIUS * 1.7,
        );
        draw_wrapped(ship.x, ship.y, SHIP_RADIUS * 1.7, |ox, oy| {
            stroke(fl.x + ox, fl.y + oy, tip.x + ox, tip.y + oy, ORANGE);
            stroke(fr.x + ox, fr.y + oy, tip.x + ox, tip.y + oy, ORANGE);
        });
    }
}

/// A rock: its size's irregular closed silhouette at its radius, glowing, wrapped
/// across the edges.
fn draw_asteroid(rock: Asteroid) {
    let r = rock.size.radius();
    let shape = rock_shape(rock.size);
    let n = shape.len();
    draw_wrapped(rock.x, rock.y, r, |ox, oy| {
        let cx = rock.x + ox;
        let cy = rock.y + oy;
        for i in 0..n {
            let (ax, ay) = rock_point(cx, cy, r, shape, i);
            let (bx, by) = rock_point(cx, cy, r, shape, (i + 1) % n);
            stroke(ax, ay, bx, by, WHITE);
        }
    });
}

/// The `i`th vertex of a rock `shape` of radius `r` centred at `(cx, cy)`.
fn rock_point(cx: f32, cy: f32, r: f32, shape: &[f32], i: usize) -> (f32, f32) {
    let a = TAU * i as f32 / shape.len() as f32;
    (cx + a.cos() * r * shape[i], cy + a.sin() * r * shape[i])
}

/// The silhouette for a rock of `size`.
fn rock_shape(size: AsteroidSize) -> &'static [f32] {
    match size {
        AsteroidSize::Large => &LARGE_ROCK,
        AsteroidSize::Medium => &MEDIUM_ROCK,
        AsteroidSize::Small => &SMALL_ROCK,
    }
}

/// Calls `draw` at the object's real position, and again shifted by a field
/// width/height wherever the object (within `radius`) straddles an edge — so a
/// rock leaving the right reappears on the left, as the toroidal field demands.
fn draw_wrapped(x: f32, y: f32, radius: f32, mut draw: impl FnMut(f32, f32)) {
    for dx in axis_offsets(x, radius, LOGICAL_WIDTH).into_iter().flatten() {
        for dy in axis_offsets(y, radius, LOGICAL_HEIGHT)
            .into_iter()
            .flatten()
        {
            draw(dx, dy);
        }
    }
}

/// The offsets to draw an object at along one axis: always `0`, plus one wrapped
/// copy when it comes within `radius` of an edge. A rock is never within `radius`
/// of both edges at once, so there is at most one wrap copy per axis.
fn axis_offsets(pos: f32, radius: f32, max: f32) -> [Option<f32>; 2] {
    let wrap = if pos < radius {
        Some(max)
    } else if pos > max - radius {
        Some(-max)
    } else {
        None
    };
    [Some(0.0), wrap]
}

/// The Collection's two-takes screen: the Faithful, playable, and the Remix,
/// locked until it is built.
pub fn mode_select(highlight: Mode) {
    clear_background(BLACK);

    font::draw_centred(LOGICAL_WIDTH, "ASTEROIDS", 150.0, TITLE_SCALE, WHITE);
    font::draw_centred(
        LOGICAL_WIDTH,
        "THE FAITHFUL AND THE REMIX",
        224.0,
        HINT_SCALE,
        GRAY,
    );
    option("FAITHFUL", 310.0, highlight == Mode::Faithful, false);
    option("REMIX (SOON)", 360.0, highlight == Mode::Remix, true);
    if highlight == Mode::Remix {
        font::draw_centred(
            LOGICAL_WIDTH,
            "THE REMIX IS STILL TO COME",
            404.0,
            HINT_SCALE,
            GRAY,
        );
    }
    font::draw_centred(
        LOGICAL_WIDTH,
        "ARROWS TO CHOOSE   ENTER TO SELECT",
        480.0,
        HINT_SCALE,
        GRAY,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "ARROWS FLY   SPACE FIRE   SHIFT HYPERSPACE   F FULLSCREEN",
        510.0,
        HINT_SCALE,
        GRAY,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        "AFTER THE 1979 ATARI ORIGINAL",
        LOGICAL_HEIGHT - 40.0,
        HINT_SCALE,
        GRAY,
    );
}

/// One menu row: its label, marked with a caret when highlighted and dimmed when
/// it is locked.
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
        LOGICAL_HEIGHT / 2.0 - 16.0,
        OPTION_SCALE,
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
