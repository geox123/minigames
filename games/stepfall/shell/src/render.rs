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

/// A small ship pip per remaining life in HAILFALL's HUD.
const LIFE_PIP_W: f32 = 6.0;
const LIFE_PIP_H: f32 = 5.0;

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
const NEON_ENEMY_FIRE: Color = color_u8!(255, 130, 90, 255);
/// The near-black a silhouette detail is punched out in.
const NEON_DETAIL: Color = color_u8!(10, 8, 14, 255);

/// A colour per squadron kind, so the zoo reads at a glance.
const NEON_DART: Color = color_u8!(255, 90, 170, 255);
const NEON_WEAVER: Color = color_u8!(120, 225, 130, 255);
const NEON_TURRET: Color = color_u8!(255, 170, 80, 255);
const NEON_SPINNER: Color = color_u8!(185, 130, 255, 255);
const NEON_WALL: Color = color_u8!(150, 190, 230, 255);
const NEON_BOMBER: Color = color_u8!(110, 240, 205, 255);

/// The Faithful's three bombs, each its own tint in the Remix.
const BOMB_ROLLING: Color = color_u8!(255, 220, 90, 255);
const BOMB_SQUIGGLY: Color = color_u8!(150, 240, 120, 255);
const BOMB_PLUNGER: Color = color_u8!(255, 100, 90, 255);

/// Power-up tints, and the mothership's phase colours and its bright cores.
const PICK_WEAPON: Color = color_u8!(255, 240, 140, 255);
const PICK_SHIELD: Color = color_u8!(90, 220, 255, 255);
const PICK_OVERDRIVE: Color = color_u8!(255, 120, 210, 255);
const BOSS_PHASE: [Color; 3] = [
    color_u8!(200, 80, 180, 255),
    color_u8!(230, 120, 90, 255),
    color_u8!(240, 70, 70, 255),
];
const NEON_CORE: Color = color_u8!(255, 245, 160, 255);

/// Draws one frame of HAILFALL — the Remix — onto the logical canvas: the
/// mothership, the squadron zoo each in its own colour, the fire on both sides,
/// power-ups, the ship and the HUD.
pub fn draw_remix(game: &stepfall_remix_core::Game, best: u32) {
    use stepfall_remix_core::{
        OVERDRIVE_MAX, PLAYER_BULLET_HEIGHT, PLAYER_BULLET_WIDTH, SHIP_HEIGHT, SHIP_WIDTH,
    };
    clear_background(color_u8!(4, 6, 14, 255));

    // The mothership, behind the swarm.
    draw_boss(game);

    // The squadron, each kind in its own colour and silhouette.
    for enemy in game.enemies() {
        draw_enemy(enemy);
    }

    // Power-ups drifting down to be caught.
    for pickup in game.pickups() {
        draw_pickup(pickup);
    }

    // The enemy fire, falling — pellets, and the Faithful's bombs by kind.
    for bullet in game.enemy_bullets() {
        draw_enemy_bullet(bullet);
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

    // The ship: a bright arrowhead over a glow, flashing while it is spared.
    let ship = game.ship();
    let hull = if game.invulnerable() {
        NEON_GLOW
    } else {
        NEON_SHIP
    };
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
        hull,
    );
    // The true hitbox shows as a bright pip while focusing.
    if game.focusing() {
        draw_rectangle(cx - 1.0, ship.y + SHIP_HEIGHT / 2.0 - 1.0, 2.0, 2.0, WHITE);
    }

    // The overdrive meter along the foot — bright when a nova is ready.
    let fill = (game.overdrive() / OVERDRIVE_MAX).clamp(0.0, 1.0);
    let ready = fill >= 1.0;
    draw_rectangle(
        6.0,
        LOGICAL_HEIGHT - 4.0,
        (LOGICAL_WIDTH - 12.0) * fill,
        2.0,
        if ready { NEON_BULLET } else { NEON_GLOW },
    );

    draw_remix_hud(game, best);
}

/// The colour a squadron kind wears.
fn enemy_colour(kind: stepfall_remix_core::EnemyKind) -> Color {
    use stepfall_remix_core::EnemyKind;
    match kind {
        EnemyKind::Dart => NEON_DART,
        EnemyKind::Weaver => NEON_WEAVER,
        EnemyKind::Turret => NEON_TURRET,
        EnemyKind::Spinner => NEON_SPINNER,
        EnemyKind::Wall => NEON_WALL,
        EnemyKind::Bomber => NEON_BOMBER,
    }
}

/// Draws one enemy: a coloured hull with a silhouette detail that reads its kind.
fn draw_enemy(enemy: stepfall_remix_core::Enemy) {
    use stepfall_remix_core::{ENEMY_HEIGHT, ENEMY_WIDTH, EnemyKind};
    let (x, y, w, h) = (enemy.x, enemy.y, ENEMY_WIDTH, ENEMY_HEIGHT);
    draw_rectangle(x, y, w, h, enemy_colour(enemy.kind));
    match enemy.kind {
        EnemyKind::Dart => draw_triangle(
            vec2(x + w / 2.0, y + h),
            vec2(x + 3.0, y + h - 3.0),
            vec2(x + w - 3.0, y + h - 3.0),
            NEON_DETAIL,
        ),
        EnemyKind::Weaver => {
            draw_rectangle(x, y + 3.0, 2.0, 4.0, NEON_DETAIL);
            draw_rectangle(x + w - 2.0, y + 3.0, 2.0, 4.0, NEON_DETAIL);
        }
        EnemyKind::Turret => {
            draw_rectangle(x + w / 2.0 - 2.0, y + h / 2.0 - 2.0, 4.0, 4.0, NEON_DETAIL);
        }
        EnemyKind::Spinner => {
            draw_rectangle(x + 2.0, y + h / 2.0 - 1.0, w - 4.0, 2.0, NEON_DETAIL);
            draw_rectangle(x + w / 2.0 - 1.0, y + 2.0, 2.0, h - 4.0, NEON_DETAIL);
        }
        EnemyKind::Wall => draw_rectangle(x + 1.0, y + 3.0, w - 2.0, 2.0, NEON_DETAIL),
        EnemyKind::Bomber => draw_rectangle(x + 3.0, y + h - 4.0, w - 6.0, 3.0, NEON_DETAIL),
    }
}

/// Draws one enemy bullet, centred on `(x, y)`: a pellet, or a fatter bomb tinted
/// by its Faithful kind.
fn draw_enemy_bullet(bullet: stepfall_remix_core::EnemyBullet) {
    use stepfall_remix_core::{BombKind, ENEMY_BULLET_SIZE, ShotKind};
    let (colour, size) = match bullet.kind {
        ShotKind::Pellet => (NEON_ENEMY_FIRE, ENEMY_BULLET_SIZE),
        ShotKind::Bomb(BombKind::Rolling) => (BOMB_ROLLING, ENEMY_BULLET_SIZE + 1.0),
        ShotKind::Bomb(BombKind::Squiggly) => (BOMB_SQUIGGLY, ENEMY_BULLET_SIZE + 1.0),
        ShotKind::Bomb(BombKind::Plunger) => (BOMB_PLUNGER, ENEMY_BULLET_SIZE + 1.0),
    };
    draw_rectangle(
        bullet.x - size / 2.0,
        bullet.y - size / 2.0,
        size,
        size,
        colour,
    );
}

/// Draws one power-up, centred on `(x, y)`: a soft glow under a shape that reads
/// its kind — a chevron to step the weapon, a ring for a shield, a diamond for
/// an overdrive charge.
fn draw_pickup(pickup: stepfall_remix_core::Pickup) {
    use stepfall_remix_core::PowerUp;
    let (x, y) = (pickup.x, pickup.y);
    let colour = match pickup.kind {
        PowerUp::Weapon => PICK_WEAPON,
        PowerUp::Shield => PICK_SHIELD,
        PowerUp::Overdrive => PICK_OVERDRIVE,
    };
    let mut glow = colour;
    glow.a = 0.28;
    draw_circle(x, y, 5.0, glow);
    match pickup.kind {
        PowerUp::Weapon => draw_triangle(
            vec2(x, y - 3.0),
            vec2(x - 3.0, y + 2.0),
            vec2(x + 3.0, y + 2.0),
            colour,
        ),
        PowerUp::Shield => draw_circle_lines(x, y, 3.0, 1.0, colour),
        PowerUp::Overdrive => draw_poly(x, y, 4, 3.2, 45.0, colour),
    }
}

/// Draws the mothership, if one is on the field: a phase-coloured hull, its bright
/// weak-point cores, and a health bar beneath.
fn draw_boss(game: &stepfall_remix_core::Game) {
    use stepfall_remix_core::{BOSS_HEIGHT, BOSS_WIDTH};
    let Some(boss) = game.boss() else {
        return;
    };
    let colour = BOSS_PHASE[(boss.phase as usize).min(2)];
    let mut glow = colour;
    glow.a = 0.30;
    draw_rectangle(
        boss.x - 3.0,
        boss.y - 3.0,
        BOSS_WIDTH + 6.0,
        BOSS_HEIGHT + 6.0,
        glow,
    );
    draw_rectangle(boss.x, boss.y, BOSS_WIDTH, BOSS_HEIGHT, colour);
    draw_rectangle(
        boss.x + 3.0,
        boss.y + 2.0,
        BOSS_WIDTH - 6.0,
        3.0,
        NEON_DETAIL,
    );

    // Weak-point cores, pulsing bright to draw the eye and the fire.
    let pulse = 0.6 + 0.4 * ((get_time() as f32 * 6.0).sin() * 0.5 + 0.5);
    let mut core = NEON_CORE;
    core.a = pulse;
    for wp in game.boss_weak_points() {
        draw_rectangle(wp.x, wp.y, wp.w, wp.h, core);
        draw_rectangle_lines(wp.x, wp.y, wp.w, wp.h, 1.0, WHITE);
    }

    // The health bar beneath the hull.
    let frac = boss.hp as f32 / boss.max_hp.max(1) as f32;
    let bar_y = boss.y + BOSS_HEIGHT + 2.0;
    draw_rectangle(boss.x, bar_y, BOSS_WIDTH, 2.0, NEON_DETAIL);
    draw_rectangle(boss.x, bar_y, BOSS_WIDTH * frac, 2.0, NEON_CORE);
}

/// HAILFALL's HUD: score, the best to beat, the stage, and the lives left.
fn draw_remix_hud(game: &stepfall_remix_core::Game, best: u32) {
    font::draw(
        &format!("{:06}", game.score()),
        6.0,
        6.0,
        HINT_SCALE,
        NEON_BULLET,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("BEST {best:06}"),
        6.0,
        HINT_SCALE,
        NEON_SHIP,
    );
    let stage = format!("STAGE {}", game.stage() + 1);
    let stage_w = font::text_width(&stage, HINT_SCALE);
    font::draw(
        &stage,
        LOGICAL_WIDTH - 6.0 - stage_w,
        6.0,
        HINT_SCALE,
        NEON_DART,
    );

    // Lives as small ship pips under the stage label, right-aligned.
    let mut x = LOGICAL_WIDTH - 6.0 - LIFE_PIP_W;
    for _ in 0..game.lives() {
        draw_triangle(
            vec2(x + LIFE_PIP_W / 2.0, 14.0),
            vec2(x, 14.0 + LIFE_PIP_H),
            vec2(x + LIFE_PIP_W, 14.0 + LIFE_PIP_H),
            NEON_SHIP,
        );
        x -= LIFE_PIP_W + 3.0;
    }
}

/// The card that resolves a HAILFALL run: won or lost, the score and stage
/// reached, the mode's best, anything newly unlocked, and the way on.
pub fn remix_summary(
    game: &stepfall_remix_core::Game,
    mode: stepfall_remix_core::Mode,
    best: u32,
    earned: &[stepfall_remix_core::meta::Content],
) {
    use stepfall_remix_core::{Mode as RunMode, Outcome};

    // Dim the frozen field behind the card.
    draw_rectangle(
        0.0,
        0.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        color_u8!(4, 6, 14, 200),
    );

    let won = game.outcome() == Some(Outcome::Won);
    let mid = LOGICAL_HEIGHT / 2.0;
    font::draw_centred(
        LOGICAL_WIDTH,
        if won { "VICTORY" } else { "RUN OVER" },
        mid - 44.0,
        OPTION_SCALE,
        if won { NEON_BULLET } else { NEON_DART },
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("SCORE {:06}", game.score()),
        mid - 12.0,
        HINT_SCALE,
        NEON_SHIP,
    );
    font::draw_centred(
        LOGICAL_WIDTH,
        &format!("STAGE {}", game.stage() + 1),
        mid,
        HINT_SCALE,
        NEON_SHIP,
    );
    let best_line = match mode {
        RunMode::Onslaught => format!("ONSLAUGHT BEST {best:06}"),
        RunMode::Daily => format!("DAILY BEST {best:06}"),
        RunMode::Sortie if won => "SORTIE CLEARED".to_string(),
        RunMode::Sortie => "SORTIE FAILED".to_string(),
    };
    font::draw_centred(LOGICAL_WIDTH, &best_line, mid + 12.0, HINT_SCALE, NEON_GLOW);

    // Call out anything this run newly unlocked, in gold.
    if !earned.is_empty() {
        let names = earned
            .iter()
            .map(|c| c.label())
            .collect::<Vec<_>>()
            .join(" ");
        font::draw_centred(
            LOGICAL_WIDTH,
            &format!("UNLOCKED {names}"),
            mid + 28.0,
            HINT_SCALE,
            NEON_BULLET,
        );
    }

    font::draw_centred(
        LOGICAL_WIDTH,
        "R PLAY AGAIN   ESC MENU",
        mid + 46.0,
        HINT_SCALE,
        NEON_GLOW,
    );
}

/// HAILFALL's mode picker — the three runs the Remix offers.
pub fn remix_select(highlight: stepfall_remix_core::Mode) {
    use stepfall_remix_core::Mode as RunMode;
    clear_background(color_u8!(4, 6, 14, 255));

    font::draw_centred(LOGICAL_WIDTH, "HAILFALL", 44.0, TITLE_SCALE, NEON_DART);
    font::draw_centred(
        LOGICAL_WIDTH,
        "CHOOSE YOUR RUN",
        86.0,
        HINT_SCALE,
        NEON_GLOW,
    );

    remix_option("SORTIE", 120.0, highlight == RunMode::Sortie);
    remix_option("ONSLAUGHT", 150.0, highlight == RunMode::Onslaught);
    remix_option("DAILY", 180.0, highlight == RunMode::Daily);

    font::draw_centred(
        LOGICAL_WIDTH,
        "ENTER PLAY    ESC BACK",
        224.0,
        HINT_SCALE,
        NEON_GLOW,
    );
}

/// One line of the mode picker, marked when highlighted.
fn remix_option(label: &str, y: f32, highlighted: bool) {
    let width = font::text_width(label, OPTION_SCALE);
    let x = (LOGICAL_WIDTH - width) / 2.0;
    let colour = if highlighted { NEON_SHIP } else { NEON_GLOW };
    font::draw(label, x, y, OPTION_SCALE, colour);
    if highlighted {
        font::draw(
            ">",
            x - font::text_width("> ", OPTION_SCALE),
            y,
            OPTION_SCALE,
            NEON_BULLET,
        );
    }
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
