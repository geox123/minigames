//! Drawing GNASH's authored look (T9), entirely in code (ADR 0003, ADR 0005).
//!
//! The cast is dental, not spectral. The eater is a bare pair of **jaws** — two
//! toothed bars gnashing as they travel, never a wedge-mouthed disc. The hunters
//! are the mouth's four kinds of **tooth**, hanging crown-down like an upper jaw,
//! each its own silhouette and colour, with the classic eyes that track their
//! heading, a frightened face, and end-of-window flashes. The sweets climb the
//! ladder drawn grander at each rung. Everything else stays deliberately plain —
//! the juice budget is uneven on purpose (spec #158).

use gnash_core::{
    COLS, Dir, Game, Hunter, HunterKind, HunterMode, Phase, Pickup, ROWS, Sweet, TILE, Tile,
    tile_center,
};
use macroquad::prelude::*;

use crate::{MAZE_TOP, SCREEN_W, app::Overlay};

const WALL: Color = Color::new(0.15, 0.18, 0.85, 1.0);
const WALL_INNER: Color = Color::new(0.09, 0.11, 0.55, 1.0);
/// The maze holds its breath while the hunt is flipped: the walls go midnight.
const WALL_FRIGHT: Color = Color::new(0.10, 0.08, 0.45, 1.0);
const WALL_INNER_FRIGHT: Color = Color::new(0.06, 0.05, 0.28, 1.0);
const GATE: Color = Color::new(0.95, 0.72, 0.82, 1.0);
const PICKUP: Color = Color::new(1.0, 0.72, 0.62, 1.0);
const JAWS: Color = Color::new(1.0, 0.92, 0.35, 1.0);
const FRIGHT_BODY: Color = Color::new(0.16, 0.18, 0.75, 1.0);
const FRIGHT_FACE: Color = Color::new(0.95, 0.85, 0.8, 1.0);

/// How many frames each end-of-window flash lasts (half on, half off).
const FLASH_FRAMES: u32 = 28;

/// The footer text's top edge — just below the maze, within the 16px footer band.
const FOOTER_Y: f32 = MAZE_TOP + (ROWS as f32) * (TILE as f32) + 1.0;

/// A hunter's body colour, by its mind.
fn body_colour(kind: HunterKind) -> Color {
    match kind {
        HunterKind::Canine => Color::new(0.95, 0.15, 0.10, 1.0),
        HunterKind::Incisor => Color::new(1.0, 0.60, 0.75, 1.0),
        HunterKind::Wisdom => Color::new(0.20, 0.90, 0.95, 1.0),
        HunterKind::Molar => Color::new(1.0, 0.65, 0.15, 1.0),
    }
}

/// Draws the whole frame: maze, sweets, cast, HUD, and whatever overlay the shell
/// is holding (ready beat, catch popup, death scatter, pause, game over).
pub fn draw(game: &Game, tick: u32, overlay: &Overlay) {
    clear_background(BLACK);
    let frightened = game.frightened_frames_left() > 0;
    draw_maze(game, tick, frightened);
    if let Some(sweet) = game.sweet() {
        draw_sweet(&sweet);
    }
    // During the death scatter the cast is already home; the scatter plays where
    // the jaws fell, and the fresh positions stay hidden until the beat ends.
    match overlay {
        Overlay::DeathScatter { x, y, t } => draw_death_scatter(*x, *y, *t),
        _ => {
            draw_hunters(game, tick);
            draw_jaws(game);
        }
    }
    draw_hud(game);
    draw_overlay_text(game, overlay);
}

/// Draws the walls (bricked: a bright shell over a darker heart), the gate, the
/// dots, and the power pellets — which blink, always, like something alive.
fn draw_maze(game: &Game, tick: u32, frightened: bool) {
    let t = TILE as f32;
    let (wall, inner) = if frightened {
        (WALL_FRIGHT, WALL_INNER_FRIGHT)
    } else {
        (WALL, WALL_INNER)
    };
    for row in 0..ROWS as i32 {
        for col in 0..COLS as i32 {
            let x = col as f32 * t;
            let y = MAZE_TOP + row as f32 * t;
            match game.tile(col, row) {
                Tile::Wall => {
                    draw_rectangle(x, y, t, t, wall);
                    draw_rectangle(x + 1.0, y + 1.0, t - 2.0, t - 2.0, inner);
                }
                Tile::Gate => draw_rectangle(x, y + t / 2.0 - 1.0, t, 2.0, GATE),
                Tile::Path => {}
            }
            let (cx, cy) = tile_center(col, row);
            let (px, py) = (cx as f32, MAZE_TOP + cy as f32);
            match game.pickup(col, row) {
                Pickup::Dot => draw_rectangle(px - 1.0, py - 1.0, 2.0, 2.0, PICKUP),
                Pickup::PowerPellet => {
                    if (tick / 12).is_multiple_of(2) {
                        draw_circle(px, py, 3.0, PICKUP);
                    }
                }
                Pickup::None => {}
            }
        }
    }
}

/// Draws the eater: a bare pair of jaws, two toothed bars that gnash as they
/// travel. The chomp phase rides the eater's own position, so the jaws work while
/// it moves and hang still while it feeds or waits.
fn draw_jaws(game: &Game) {
    let eater = game.eater();
    let (cx, cy) = (eater.x as f32, MAZE_TOP + eater.y as f32);
    let (f, s) = axes(eater.dir);
    // The gap between the jaws breathes 1..3 px with distance travelled.
    let phase = ((eater.x + eater.y) as f32 * 0.8).sin();
    let gap = 2.0 + phase.abs() * 1.5;
    for side in [-1.0f32, 1.0] {
        let jaw = vec2(cx, cy) + s * side * gap;
        // The jaw bar, 8px long along the facing.
        draw_line(
            jaw.x - f.x * 4.0,
            jaw.y - f.y * 4.0,
            jaw.x + f.x * 4.0,
            jaw.y + f.y * 4.0,
            2.0,
            JAWS,
        );
        // Three teeth on each bar, biting inward.
        for i in -1..=1 {
            let root = jaw + f * (i as f32 * 3.0);
            let tip = root - s * side * (gap - 0.5);
            let half = f * 1.2;
            draw_triangle(
                vec2(root.x - half.x, root.y - half.y),
                vec2(root.x + half.x, root.y + half.y),
                vec2(tip.x, tip.y),
                JAWS,
            );
        }
    }
}

/// Draws all four hunters by their mode.
fn draw_hunters(game: &Game, tick: u32) {
    for hunter in game.hunters() {
        match hunter.mode {
            HunterMode::Eyes => draw_eyes_only(&hunter),
            HunterMode::Frightened => draw_frightened(game, &hunter, tick),
            HunterMode::Hunting => draw_tooth(&hunter, body_colour(hunter.kind)),
        }
    }
}

/// Draws a hunter as its tooth: a root block hanging from above, its crown biting
/// downward in the silhouette of its kind, and the tracking eyes on the root.
fn draw_tooth(hunter: &Hunter, body: Color) {
    let cx = hunter.x as f32;
    let cy = MAZE_TOP + hunter.y as f32;
    // The root block, 10 wide and 6 tall, centred a little high.
    draw_rectangle(cx - 5.0, cy - 6.0, 10.0, 6.0, body);
    // The crown, by kind, biting down from the root's underside at cy.
    match hunter.kind {
        // The fang: one long point.
        HunterKind::Canine => {
            draw_triangle(
                vec2(cx - 5.0, cy),
                vec2(cx + 5.0, cy),
                vec2(cx, cy + 6.0),
                body,
            );
        }
        // The front tooth: a flat chisel edge.
        HunterKind::Incisor => {
            draw_rectangle(cx - 5.0, cy, 10.0, 4.0, body);
        }
        // The crooked latecomer: two uneven points, leaning.
        HunterKind::Wisdom => {
            draw_triangle(
                vec2(cx - 5.0, cy),
                vec2(cx, cy),
                vec2(cx - 4.0, cy + 5.0),
                body,
            );
            draw_triangle(
                vec2(cx - 1.0, cy),
                vec2(cx + 5.0, cy),
                vec2(cx + 4.0, cy + 3.0),
                body,
            );
        }
        // The back tooth: two broad blunt cusps.
        HunterKind::Molar => {
            draw_rectangle(cx - 5.0, cy, 4.0, 4.0, body);
            draw_rectangle(cx + 1.0, cy, 4.0, 4.0, body);
        }
    }
    draw_tracking_eyes(cx, cy - 3.5, hunter.dir);
}

/// The classic pair of eyes, pupils leaning the way the hunter heads.
fn draw_tracking_eyes(cx: f32, cy: f32, dir: Dir) {
    let (f, _) = axes(dir);
    for side in [-2.5f32, 2.5] {
        draw_rectangle(cx + side - 1.5, cy - 1.5, 3.0, 3.0, WHITE);
        draw_rectangle(
            cx + side - 0.5 + f.x,
            cy - 0.5 + f.y,
            1.5,
            1.5,
            Color::new(0.1, 0.1, 0.4, 1.0),
        );
    }
}

/// A frightened hunter: the same silhouette gone midnight blue, dot eyes, a
/// worried zigzag mouth — flashing pale when the window is about to shut.
fn draw_frightened(game: &Game, hunter: &Hunter, tick: u32) {
    let left = game.frightened_frames_left();
    let flashing = left <= game.frightened_flashes() * FLASH_FRAMES;
    let pale = flashing && (tick / (FLASH_FRAMES / 2)).is_multiple_of(2);
    let body = if pale { FRIGHT_FACE } else { FRIGHT_BODY };
    let face = if pale { FRIGHT_BODY } else { FRIGHT_FACE };
    let cx = hunter.x as f32;
    let cy = MAZE_TOP + hunter.y as f32;
    draw_rectangle(cx - 5.0, cy - 6.0, 10.0, 8.0, body);
    for side in [-2.5f32, 2.5] {
        draw_rectangle(cx + side - 0.75, cy - 4.0, 1.5, 1.5, face);
    }
    // The zigzag mouth.
    for i in 0..4 {
        let x = cx - 4.0 + i as f32 * 2.0;
        let up = i % 2 == 0;
        draw_line(
            x,
            if up { cy + 1.0 } else { cy - 0.5 },
            x + 2.0,
            if up { cy - 0.5 } else { cy + 1.0 },
            1.0,
            face,
        );
    }
}

/// A caught hunter: nothing left but the eyes, racing home.
fn draw_eyes_only(hunter: &Hunter) {
    draw_tracking_eyes(
        hunter.x as f32,
        MAZE_TOP + hunter.y as f32 - 1.0,
        hunter.dir,
    );
}

/// Draws the sweet at its rung — each grander than the last, from a humble candy
/// drop up to the gateau no tooth should meet.
fn draw_sweet(sweet: &Sweet) {
    let x = sweet.x as f32;
    let y = MAZE_TOP + sweet.y as f32;
    let red = Color::new(0.95, 0.25, 0.35, 1.0);
    let pink = Color::new(1.0, 0.65, 0.8, 1.0);
    let cream = Color::new(1.0, 0.95, 0.85, 1.0);
    let brown = Color::new(0.55, 0.33, 0.18, 1.0);
    match sweet.value {
        // A candy drop.
        100 => draw_circle(x, y, 2.5, red),
        // A wrapped candy: the drop grows wings.
        300 => {
            draw_circle(x, y, 2.5, red);
            draw_triangle(
                vec2(x - 2.5, y),
                vec2(x - 5.0, y - 2.5),
                vec2(x - 5.0, y + 2.5),
                pink,
            );
            draw_triangle(
                vec2(x + 2.5, y),
                vec2(x + 5.0, y - 2.5),
                vec2(x + 5.0, y + 2.5),
                pink,
            );
        }
        // A lollipop.
        500 => {
            draw_line(x, y, x, y + 5.0, 1.0, cream);
            draw_circle(x, y - 1.0, 3.0, pink);
            draw_circle(x, y - 1.0, 1.2, red);
        }
        // A chocolate bar.
        700 => {
            draw_rectangle(x - 4.0, y - 2.5, 8.0, 5.0, brown);
            draw_line(x, y - 2.5, x, y + 2.5, 1.0, BLACK);
            draw_line(x - 4.0, y, x + 4.0, y, 1.0, BLACK);
        }
        // An ice pop.
        1000 => {
            draw_rectangle(x - 2.5, y - 5.0, 5.0, 8.0, pink);
            draw_line(x, y + 3.0, x, y + 6.0, 1.0, cream);
        }
        // A doughnut.
        2000 => {
            draw_circle(x, y, 4.0, pink);
            draw_circle(x, y, 1.5, BLACK);
        }
        // A slice of cake.
        3000 => {
            draw_triangle(
                vec2(x - 4.0, y + 3.0),
                vec2(x + 4.0, y + 3.0),
                vec2(x, y - 4.0),
                cream,
            );
            draw_line(x - 2.0, y - 0.5, x + 2.0, y - 0.5, 1.5, red);
        }
        // The gateau.
        _ => {
            draw_rectangle(x - 5.0, y, 10.0, 3.5, brown);
            draw_rectangle(x - 3.5, y - 3.0, 7.0, 3.0, cream);
            draw_circle(x, y - 4.0, 1.5, red);
        }
    }
}

/// The death beat: the jaws burst into loose teeth where they fell.
fn draw_death_scatter(x: f32, y: f32, t: u32) {
    let cy = MAZE_TOP + y;
    let r = t as f32 * 0.35;
    let fade = 1.0 - (t as f32 / 60.0).min(1.0);
    let colour = Color::new(JAWS.r, JAWS.g, JAWS.b, fade);
    for i in 0..6 {
        let angle = i as f32 * std::f32::consts::TAU / 6.0 + 0.4;
        let (dx, dy) = (angle.cos() * r, angle.sin() * r);
        let (px, py) = (x + dx, cy + dy);
        draw_triangle(
            vec2(px - 1.5, py - 1.5),
            vec2(px + 1.5, py - 1.5),
            vec2(px, py + 2.0),
            colour,
        );
    }
}

/// Score and level along the top, the lives (as little jaw glyphs) and the
/// wordmark along the footer.
fn draw_hud(game: &Game) {
    shell_kit::font::draw(&format!("SCORE {}", game.score()), 8.0, 8.0, 2.0, WHITE);
    let level = format!("LVL {}", game.level());
    let w = shell_kit::font::text_width(&level, 2.0);
    shell_kit::font::draw(&level, SCREEN_W - w - 8.0, 8.0, 2.0, WHITE);
    for life in 0..game.lives().min(6) {
        let x = 8.0 + life as f32 * 10.0;
        draw_line(x, FOOTER_Y + 4.0, x + 6.0, FOOTER_Y + 4.0, 2.0, JAWS);
        draw_line(x, FOOTER_Y + 8.0, x + 6.0, FOOTER_Y + 8.0, 2.0, JAWS);
    }
    shell_kit::font::draw_centred(SCREEN_W, "GNASH", FOOTER_Y, 2.0, DARKGRAY);
}

/// The overlay's text, if it carries any.
fn draw_overlay_text(game: &Game, overlay: &Overlay) {
    let mid = MAZE_TOP + 140.0;
    match overlay {
        Overlay::Ready(_) => {
            shell_kit::font::draw_centred(SCREEN_W, "READY!", mid, 2.0, JAWS);
        }
        Overlay::CatchPopup { x, y, value, .. } => {
            let text = format!("{value}");
            let w = shell_kit::font::text_width(&text, 1.0);
            shell_kit::font::draw(&text, *x - w / 2.0, MAZE_TOP + *y - 3.0, 1.0, WHITE);
        }
        Overlay::Paused => {
            shell_kit::font::draw_centred(SCREEN_W, "PAUSED", mid, 3.0, WHITE);
        }
        _ => {}
    }
    if game.phase() == Phase::GameOver {
        shell_kit::font::draw_centred(
            SCREEN_W,
            "GAME OVER",
            mid,
            3.0,
            Color::new(0.95, 0.25, 0.2, 1.0),
        );
        shell_kit::font::draw_centred(SCREEN_W, "R TO RESTART", mid + 16.0, 1.0, DARKGRAY);
    }
}

/// Unit forward and side vectors for a heading, in screen space.
fn axes(dir: Dir) -> (Vec2, Vec2) {
    let f = match dir {
        Dir::Up => vec2(0.0, -1.0),
        Dir::Down => vec2(0.0, 1.0),
        Dir::Left => vec2(-1.0, 0.0),
        Dir::Right => vec2(1.0, 0.0),
    };
    (f, vec2(-f.y, f.x))
}
