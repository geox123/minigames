//! Drawing the Asteroids field: bright vector outlines on black with a soft glow —
//! the vector cabinet's look. Everything here is macroquad glue; the shapes are
//! authored in code, none traced from the original
//! ([ADR 0003](../../../docs/adr/0003-code-drawn-visuals.md)). Each rock size has
//! its own irregular silhouette, and explosions scatter fading line-shards.

use asteroids_core::{
    Asteroid, AsteroidSize, Blast, Game, LOGICAL_HEIGHT, LOGICAL_WIDTH, SHIP_RADIUS, Saucer,
    SaucerBullet, Ship, Shot,
};
use asteroids_remix_core::{
    Asteroid as RemixAsteroid, Boss as RemixBoss, Enemy as RemixEnemy,
    EnemyBullet as RemixEnemyBullet, EnemyKind, Game as RemixGame, Mode as RunMode,
    Pickup as RemixPickup, PowerUp, Ship as RemixShip, Well,
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

/// ACCRETE's gravity well — a warm star-glow.
const WELL_COLOR: Color = color_u8!(255, 220, 130, 255);

/// ACCRETE's enemy craft, and the warm colour of their fire — a hostile pink set
/// against the ship's white and the well's gold so a threat reads at a glance.
const ENEMY_COLOR: Color = color_u8!(255, 100, 150, 255);
const ENEMY_FIRE_COLOR: Color = color_u8!(255, 120, 90, 255);

/// ACCRETE's power-ups — a cool green-cyan "boon" colour, apart from the warm hazards;
/// individual pickups tint from it by kind. Also the shield's protective ring.
const PICKUP_COLOR: Color = color_u8!(120, 255, 180, 255);
const SHIELD_COLOR: Color = color_u8!(120, 220, 255, 255);

/// The boss — a rival well — in a hot hostile red, its weak-point cores in a bright
/// warning yellow that says "strike here."
const BOSS_COLOR: Color = color_u8!(255, 90, 70, 255);
const WEAK_POINT_COLOR: Color = color_u8!(255, 230, 90, 255);

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

/// Strokes a regular `sides`-gon outline of radius `r` centred at `(cx, cy)`.
fn stroke_polygon(cx: f32, cy: f32, r: f32, sides: usize, color: Color) {
    for i in 0..sides {
        let a0 = TAU * i as f32 / sides as f32;
        let a1 = TAU * (i + 1) as f32 / sides as f32;
        stroke(
            cx + a0.cos() * r,
            cy + a0.sin() * r,
            cx + a1.cos() * r,
            cy + a1.sin() * r,
            color,
        );
    }
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
    draw_blast_at(blast.x, blast.y, blast.progress);
}

/// Draws an explosion of `progress` (0→1) at `(x, y)` — shared by the Faithful and
/// ACCRETE.
fn draw_blast_at(x: f32, y: f32, progress: f32) {
    const SHARDS: usize = 8;
    let inner = 3.0 + 10.0 * progress;
    let outer = 6.0 + 26.0 * progress;
    let color = Color::new(1.0, 1.0, 1.0, (1.0 - progress).clamp(0.0, 1.0));
    for i in 0..SHARDS {
        // A slight turn as they fly, so the burst reads as motion, not a static star.
        let a = TAU * i as f32 / SHARDS as f32 + progress * 0.6;
        draw_line(
            x + a.cos() * inner,
            y + a.sin() * inner,
            x + a.cos() * outer,
            y + a.sin() * outer,
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
    draw_ship_glyph(ship.x, ship.y, ship.angle, ship.thrusting, SHIP_RADIUS);
}

/// Draws a ship triangle of `radius` at `(x, y)` facing `angle`, wrapped across the
/// edges, with a flickering thrust flame while `thrusting`. Shared by the Faithful
/// and ACCRETE, which fly the same shape on the same toroidal field.
fn draw_ship_glyph(x: f32, y: f32, angle: f32, thrusting: bool, radius: f32) {
    // Forward along the facing (angle 0 points up), and right, perpendicular to it.
    let (fx, fy) = (angle.sin(), -angle.cos());
    let (rx, ry) = (angle.cos(), angle.sin());

    let nose = vec2(x + fx * radius * 1.3, y + fy * radius * 1.3);
    let base_x = x - fx * radius * 0.8;
    let base_y = y - fy * radius * 0.8;
    let left = vec2(base_x - rx * radius * 0.9, base_y - ry * radius * 0.9);
    let right = vec2(base_x + rx * radius * 0.9, base_y + ry * radius * 0.9);

    draw_wrapped(x, y, radius * 1.3, |ox, oy| {
        stroke(nose.x + ox, nose.y + oy, left.x + ox, left.y + oy, WHITE);
        stroke(nose.x + ox, nose.y + oy, right.x + ox, right.y + oy, WHITE);
        stroke(left.x + ox, left.y + oy, right.x + ox, right.y + oy, WHITE);
    });

    // A thrust flame that blinks on and off, like the original's.
    if thrusting && ((get_time() * 20.0) as i64).rem_euclid(2) == 0 {
        let fl = vec2(base_x - rx * radius * 0.45, base_y - ry * radius * 0.45);
        let fr = vec2(base_x + rx * radius * 0.45, base_y + ry * radius * 0.45);
        let tip = vec2(x - fx * radius * 1.7, y - fy * radius * 1.7);
        draw_wrapped(x, y, radius * 1.7, |ox, oy| {
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

/// The Collection's two-takes screen: the Faithful and ACCRETE, both playable.
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
    option("ACCRETE", 360.0, highlight == Mode::Remix, false);
    if highlight == Mode::Remix {
        font::draw_centred(LOGICAL_WIDTH, "THE GRAVITY REMIX", 404.0, HINT_SCALE, GRAY);
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

/// Draws a live ACCRETE run: the gravity wells, the rocks orbiting them, the enemy
/// craft and their fire, the shots, and the ship — in neon, with the HUD carrying the
/// session/persisted `best`. (The accretion glow and the lensing arrive with the look
/// ticket.)
pub fn draw_remix(game: &RemixGame, best: u32) {
    clear_background(BLACK);
    for well in game.wells() {
        draw_well(well);
    }
    for rock in game.asteroids() {
        draw_remix_rock(rock);
    }
    for enemy in game.enemies() {
        draw_remix_enemy(enemy);
    }
    if let Some(boss) = game.boss() {
        draw_remix_boss(boss, game);
    }
    for bullet in game.enemy_bullets() {
        draw_remix_enemy_bullet(bullet);
    }
    for pickup in game.pickups() {
        draw_remix_pickup(pickup);
    }
    for shot in game.shots() {
        blip(shot.x, shot.y, 2.5, WHITE);
    }
    for blast in game.blasts() {
        draw_blast_at(blast.x, blast.y, blast.progress);
    }
    // A fresh, protected ship blinks; a downed one is off the field.
    let visible = !game.ship_invulnerable() || ((get_time() * 10.0) as i64).rem_euclid(2) == 0;
    if game.ship_alive() && visible {
        draw_remix_ship(game.ship());
        if game.has_shield() {
            draw_shield(game.ship());
        }
    }
    draw_remix_hud(game, best);
}

/// ACCRETE's HUD: the score and best, the ships left, the accretion feed streak, the
/// current system, and the earned weapon tier while it is running.
fn draw_remix_hud(game: &RemixGame, best: u32) {
    font::draw(&game.score().to_string(), 20.0, 20.0, OPTION_SCALE, WHITE);
    if best > 0 {
        font::draw_centred(
            LOGICAL_WIDTH,
            &format!("BEST {best}"),
            24.0,
            HINT_SCALE,
            GRAY,
        );
    }
    let system = format!("SYSTEM {}", game.stage());
    font::draw(
        &system,
        LOGICAL_WIDTH - font::text_width(&system, HINT_SCALE) - 20.0,
        24.0,
        HINT_SCALE,
        GRAY,
    );
    for i in 0..game.lives() {
        draw_ship_icon(28.0 + i as f32 * 24.0, 60.0);
    }
    let streak = game.feed_streak();
    if streak > 1 {
        font::draw_centred(
            LOGICAL_WIDTH,
            &format!("FEED x{streak}"),
            44.0,
            HINT_SCALE,
            WELL_COLOR,
        );
    }
    // The weapon rung, named, once the ship has climbed off the base tier.
    let weapon = match game.weapon_level() {
        0 => "",
        1 => "SPREAD",
        2 => "PIERCE",
        _ => "RAPID",
    };
    if !weapon.is_empty() {
        font::draw(weapon, 20.0, 84.0, HINT_SCALE, PICKUP_COLOR);
    }
    draw_collapse_meter(game.collapse_meter());
}

/// The collapse meter, a bar along the foot of the field; full and bright when a
/// collapse is ready to spend.
fn draw_collapse_meter(meter: f32) {
    let (bw, bh) = (240.0, 10.0);
    let bx = (LOGICAL_WIDTH - bw) / 2.0;
    let by = LOGICAL_HEIGHT - 28.0;
    draw_rectangle_lines(bx, by, bw, bh, STROKE, dim(WELL_COLOR));
    draw_rectangle(bx, by, bw * meter.clamp(0.0, 1.0), bh, WELL_COLOR);
    if meter >= 1.0 {
        font::draw_centred(
            LOGICAL_WIDTH,
            "COLLAPSE READY",
            by - 16.0,
            HINT_SCALE,
            WELL_COLOR,
        );
    }
}

/// ACCRETE's run summary — VICTORY when the finite Orbit ladder is beaten, RUN OVER
/// otherwise — with the run's score, the system it reached, the rocks it fed the wells,
/// the mode `best`, and the way on (restart / back out).
pub fn remix_summary(game: &RemixGame, best: u32) {
    let won = game.outcome() == Some(asteroids_remix_core::Outcome::Won);
    let (banner, colour) = if won {
        ("VICTORY", WELL_COLOR)
    } else {
        ("RUN OVER", WHITE)
    };
    let cx = LOGICAL_WIDTH;
    let mid = LOGICAL_HEIGHT / 2.0;
    font::draw_centred(cx, banner, mid - 80.0, TITLE_SCALE, colour);
    font::draw_centred(
        cx,
        &format!("SCORE {}", game.score()),
        mid,
        OPTION_SCALE,
        WHITE,
    );
    font::draw_centred(
        cx,
        &format!("SYSTEM {}", game.stage()),
        mid + 32.0,
        HINT_SCALE,
        GRAY,
    );
    font::draw_centred(
        cx,
        &format!("ROCKS ACCRETED {}", game.rocks_accreted()),
        mid + 52.0,
        HINT_SCALE,
        GRAY,
    );
    if best > 0 {
        font::draw_centred(
            cx,
            &format!("BEST {best}"),
            mid + 72.0,
            HINT_SCALE,
            WELL_COLOR,
        );
    }
    font::draw_centred(cx, "R RESTART   ESC MODES", mid + 104.0, HINT_SCALE, GRAY);
}

/// ACCRETE's mode picker — Orbit (the finite ladder), Maelstrom and Daily (endless).
/// The row order is the single source in [`crate::app::REMIX_MODES`], so the on-screen
/// order and the highlight navigation can never drift apart.
pub fn remix_select(highlight: RunMode) {
    clear_background(BLACK);
    font::draw_centred(LOGICAL_WIDTH, "ACCRETE", 150.0, TITLE_SCALE, WELL_COLOR);
    font::draw_centred(LOGICAL_WIDTH, "CHOOSE A MODE", 224.0, HINT_SCALE, GRAY);

    for (i, mode) in crate::app::REMIX_MODES.iter().enumerate() {
        let (label, blurb) = mode_blurb(*mode);
        let y = 320.0 + i as f32 * 54.0;
        option(label, y, *mode == highlight, false);
        if *mode == highlight {
            font::draw_centred(LOGICAL_WIDTH, blurb, y + 24.0, HINT_SCALE, GRAY);
        }
    }
    font::draw_centred(
        LOGICAL_WIDTH,
        "UP/DOWN TO CHOOSE   ENTER TO FLY   ESC BACK",
        LOGICAL_HEIGHT - 60.0,
        HINT_SCALE,
        GRAY,
    );
}

/// A mode's picker label and its one-line blurb.
fn mode_blurb(mode: RunMode) -> (&'static str, &'static str) {
    match mode {
        RunMode::Orbit => ("ORBIT", "A WINNABLE LADDER OF SYSTEMS"),
        RunMode::Maelstrom => ("MAELSTROM", "ENDLESS - CHASE A HIGH SCORE"),
        RunMode::Daily => ("DAILY", "TODAY'S SEEDED RUN, SHARED BY ALL"),
    }
}

/// A rock on the gravity field: a glowing polygon at its radius. (The look ticket
/// gives it the Faithful's authored silhouette.)
fn draw_remix_rock(rock: RemixAsteroid) {
    stroke_polygon(rock.x, rock.y, rock.size.radius(), 10, WHITE);
}

/// A gravity well: a bright star core ringed by a faint halo.
fn draw_well(well: Well) {
    let r = asteroids_remix_core::WELL_CORE_RADIUS;
    draw_circle(well.x, well.y, r * 2.4, dim(WELL_COLOR));
    draw_circle_lines(well.x, well.y, r, STROKE, WELL_COLOR);
    draw_circle(well.x, well.y, 3.5, WELL_COLOR);
}

/// The ACCRETE ship: the same glowing triangle the Faithful flies.
fn draw_remix_ship(ship: RemixShip) {
    draw_ship_glyph(
        ship.x,
        ship.y,
        ship.angle,
        ship.thrusting,
        asteroids_remix_core::SHIP_RADIUS,
    );
}

/// An enemy craft: a distinct neon silhouette per kind, wrapped across the field
/// edges. Each kind reads at a glance — a hexagonal Orbiter, a diving dart, a spiked
/// Mine, a pronged Shepherd.
fn draw_remix_enemy(enemy: RemixEnemy) {
    let r = asteroids_remix_core::ENEMY_RADIUS;
    draw_wrapped(enemy.x, enemy.y, r * 1.4, |ox, oy| {
        let (cx, cy) = (enemy.x + ox, enemy.y + oy);
        match enemy.kind {
            EnemyKind::Orbiter => draw_enemy_polygon(cx, cy, r, 6),
            EnemyKind::Diver => draw_enemy_diver(cx, cy, r),
            EnemyKind::Mine => draw_enemy_mine(cx, cy, r),
            EnemyKind::Shepherd => draw_enemy_shepherd(cx, cy, r),
        }
    });
}

/// A regular `sides`-gon outline at radius `r`, with a bright core — the Orbiter.
fn draw_enemy_polygon(cx: f32, cy: f32, r: f32, sides: usize) {
    stroke_polygon(cx, cy, r, sides, ENEMY_COLOR);
    blip(cx, cy, 2.5, ENEMY_COLOR);
}

/// A downward dart — the Diver, caught mid-fall.
fn draw_enemy_diver(cx: f32, cy: f32, r: f32) {
    let nose = (cx, cy + r);
    let left = (cx - r * 0.8, cy - r * 0.7);
    let right = (cx + r * 0.8, cy - r * 0.7);
    stroke(nose.0, nose.1, left.0, left.1, ENEMY_COLOR);
    stroke(nose.0, nose.1, right.0, right.1, ENEMY_COLOR);
    stroke(left.0, left.1, right.0, right.1, ENEMY_COLOR);
}

/// A spiked circle — the Mine, a naval hazard drifting in the pull.
fn draw_enemy_mine(cx: f32, cy: f32, r: f32) {
    draw_circle_lines(cx, cy, r * 0.6, STROKE, ENEMY_COLOR);
    for i in 0..8 {
        let a = TAU * i as f32 / 8.0;
        stroke(
            cx + a.cos() * r * 0.6,
            cy + a.sin() * r * 0.6,
            cx + a.cos() * r * 1.15,
            cy + a.sin() * r * 1.15,
            ENEMY_COLOR,
        );
    }
    blip(cx, cy, 2.0, ENEMY_COLOR);
}

/// A diamond with two forward-reaching prongs — the Shepherd, its crook out to herd.
fn draw_enemy_shepherd(cx: f32, cy: f32, r: f32) {
    let pts = [(0.0, -r), (r * 0.8, 0.0), (0.0, r), (-r * 0.8, 0.0)];
    for i in 0..pts.len() {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[(i + 1) % pts.len()];
        stroke(cx + ax, cy + ay, cx + bx, cy + by, ENEMY_COLOR);
    }
    stroke(cx - r * 0.8, cy, cx - r * 1.3, cy - r * 0.4, ENEMY_COLOR);
    stroke(cx - r * 0.8, cy, cx - r * 1.3, cy + r * 0.4, ENEMY_COLOR);
}

/// An enemy shot: a small warm blip, set apart from the player's white fire.
fn draw_remix_enemy_bullet(bullet: RemixEnemyBullet) {
    blip(bullet.x, bullet.y, 2.5, ENEMY_FIRE_COLOR);
}

/// A power-up adrift on the field: a glowing rotating square in its kind's tint, with
/// a letter marking what it grants — W(eapon), S(hield), C(ollapse).
fn draw_remix_pickup(pickup: RemixPickup) {
    let (color, label) = match pickup.kind {
        PowerUp::Weapon => (PICKUP_COLOR, "W"),
        PowerUp::Shield => (SHIELD_COLOR, "S"),
        PowerUp::Collapse => (WELL_COLOR, "C"),
    };
    let r = 9.0;
    // A diamond (a square on its point) so a boon reads apart from the enemies' hulls.
    let pts = [(0.0, -r), (r, 0.0), (0.0, r), (-r, 0.0)];
    for i in 0..pts.len() {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[(i + 1) % pts.len()];
        stroke(
            pickup.x + ax,
            pickup.y + ay,
            pickup.x + bx,
            pickup.y + by,
            color,
        );
    }
    let w = font::text_width(label, HINT_SCALE);
    font::draw(label, pickup.x - w / 2.0, pickup.y - 3.0, HINT_SCALE, color);
}

/// The shield: a protective ring around the ship while one is up.
fn draw_shield(ship: RemixShip) {
    let r = asteroids_remix_core::SHIP_RADIUS * 1.7;
    draw_circle(ship.x, ship.y, r + STROKE, dim(SHIELD_COLOR));
    draw_circle_lines(ship.x, ship.y, r, STROKE, SHIELD_COLOR);
}

/// The boss: a rival well — a hostile glow, an armoured hull ring with a bright core,
/// its rotating weak-point cores picked out in warning yellow, and a health bar above.
fn draw_remix_boss(boss: RemixBoss, game: &RemixGame) {
    let hull = asteroids_remix_core::BOSS_HULL_RADIUS;
    draw_circle(boss.x, boss.y, hull * 1.4, dim(BOSS_COLOR));
    draw_circle_lines(boss.x, boss.y, hull, STROKE, BOSS_COLOR);
    draw_circle(boss.x, boss.y, 5.0, BOSS_COLOR);

    let wr = asteroids_remix_core::WEAK_POINT_RADIUS;
    for wp in game.weak_points() {
        draw_circle(wp.x, wp.y, wr * 1.6, dim(WEAK_POINT_COLOR));
        draw_circle_lines(wp.x, wp.y, wr, STROKE, WEAK_POINT_COLOR);
        draw_circle(wp.x, wp.y, 2.5, WEAK_POINT_COLOR);
    }

    // A health bar riding above the hull.
    let (bw, bh) = (hull * 2.0, 6.0);
    let bx = boss.x - bw / 2.0;
    let by = boss.y - hull - 16.0;
    let frac = boss.hp as f32 / boss.max_hp.max(1) as f32;
    draw_rectangle_lines(bx, by, bw, bh, STROKE, dim(BOSS_COLOR));
    draw_rectangle(bx, by, bw * frac.clamp(0.0, 1.0), bh, BOSS_COLOR);
}
