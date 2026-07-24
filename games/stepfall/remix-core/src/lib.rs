//! The pure, deterministic core of **HAILFALL** — STEPFALL's Remix, a bullet-hell
//! reimagining of the 1978 invasion game.
//!
//! Where the Faithful is a rigid grid grinding down while you shuffle beneath it,
//! HAILFALL cuts the swarm loose: alien squadrons sweep in firing patterns that
//! fill the screen, and you fly a nimble ship through the storm. Like every core
//! in the Collection it owns the rules and knows nothing of rendering, audio,
//! windows or wall-clock time, and advances in fixed timesteps so a seed and a
//! sequence of inputs always replay the same run.
//!
//! It shares the Faithful's portrait field so one shell canvas serves both takes.
//!
//! This slice is the tracer bullet: the seam, and a ship you can fly within the
//! lower band. Fire, enemies, patterns, the tools and the modes arrive in later
//! tickets. The ship's **loadout** is handed *in* at construction, so the core
//! never knows the concept of "unlocks" — it only flies whatever it is given.

/// Width of the portrait play field, in logical units — shared with the Faithful.
pub const LOGICAL_WIDTH: f32 = 224.0;
/// Height of the portrait play field, in logical units — shared with the Faithful.
pub const LOGICAL_HEIGHT: f32 = 256.0;

/// Length of a single simulation step, in seconds — the Collection's 120 Hz.
pub const TIMESTEP: f32 = 1.0 / 120.0;

/// The ship's size, and how fast it flies.
pub const SHIP_WIDTH: f32 = 11.0;
pub const SHIP_HEIGHT: f32 = 8.0;
const SHIP_SPEED: f32 = 130.0;
/// How far from the field's side and foot the ship may travel.
const MARGIN: f32 = 8.0;
/// The ship flies within the lower band of the field: never above this line, so
/// it stays the defender at the bottom even with full freedom to weave.
const BAND_TOP: f32 = LOGICAL_HEIGHT * 0.5;

/// The ship's fire: bullet size, how fast it climbs, and the interrupts between
/// shots while fire is held.
pub const PLAYER_BULLET_WIDTH: f32 = 2.0;
pub const PLAYER_BULLET_HEIGHT: f32 = 6.0;
const PLAYER_BULLET_SPEED: f32 = 300.0;
const FIRE_INTERVAL: u32 = 9;

/// A squadron of enemies: how big each is, how many enter at once, how they are
/// spaced, and how they fly in and sway once settled.
pub const ENEMY_WIDTH: f32 = 12.0;
pub const ENEMY_HEIGHT: f32 = 10.0;
const SQUAD_COLS: usize = 6;
const SQUAD_ROWS: usize = 2;
const ENEMY_GAP_X: f32 = 26.0;
const ENEMY_GAP_Y: f32 = 18.0;
/// Where the squadron's top row settles, how fast it flies in, and its sway.
const SQUAD_TOP: f32 = 34.0;
const ENTRY_SPEED: f32 = 90.0;
const SWAY_AMP: f32 = 22.0;
const SWAY_RATE: f32 = 0.9;
/// Interrupts to wait before the next squadron flies in once the field is clear.
const WAVE_GAP: u32 = 90;
/// What downing one enemy scores.
const ENEMY_SCORE: u32 = 100;

/// Return fire: bullet size and speed, the interrupts between the squadron's
/// shots, and the fan a spread fires.
pub const ENEMY_BULLET_SIZE: f32 = 3.0;
const ENEMY_BULLET_SPEED: f32 = 95.0;
const ENEMY_FIRE_INTERVAL: u32 = 22;
const SPREAD_COUNT: u32 = 5;
const SPREAD_STEP: f32 = 0.22;

/// The ship's true hitbox is far smaller than its hull — a bullet only bites if
/// it strikes this tiny square at the ship's heart.
const HITBOX_SIZE: f32 = 3.0;

/// Lives a run starts with, and how long the ship is spared after a hit.
pub const LIVES_START: u32 = 3;
const HIT_INVULN: u32 = 90;

/// Which run a game is playing. The behaviours differ from a later ticket; here
/// the mode is only recorded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// A finite, winnable ladder of stages.
    #[default]
    Sortie,
    /// Endless, ever-denser waves, scored for survival.
    Onslaught,
    /// A date-seeded fixed run.
    Daily,
}

/// The ship's starting kit — the weapons and options a run flies with. Handed in
/// at construction so the core never knows "unlocks"; empty for now, filled as
/// the weapon and power-up tickets land.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loadout {}

/// What the player is doing this step. Movement is two-dimensional within the
/// band; the action buttons are wired up by later tickets but named now so the
/// input shape does not churn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// Hold to fire (from the firing ticket).
    pub fire: bool,
    /// Hold to move slow and precise (from the tools ticket).
    pub focus: bool,
    /// Tap to dash (from the tools ticket).
    pub dash: bool,
    /// Tap to spend a charged overdrive (from the tools ticket).
    pub bomb: bool,
}

/// The ship, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ship {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
}

/// One of the ship's shots in flight, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bullet {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
}

/// An enemy still flying, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enemy {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
}

/// An enemy bullet in flight, as the shell should draw it — a small square
/// centred on `(x, y)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyBullet {
    /// Centre x.
    pub x: f32,
    /// Centre y.
    pub y: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to. It
/// grows a field per ticket.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The ship fired a shot this step.
    pub shot_fired: bool,
    /// A shot downed an enemy this step.
    pub enemy_killed: bool,
    /// A bullet struck the ship this step and cost a life.
    pub player_hit: bool,
    /// The last life was spent this step — the run is over.
    pub run_over: bool,
}

/// Where a run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The run is being played.
    Playing,
    /// The run is over.
    Over,
}

/// A small, fast, deterministic RNG (xorshift64) — the run's only randomness,
/// seeded once, so a seed and inputs always replay the same run.
#[derive(Clone)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: (seed ^ 0x9e37_79b9_7f4a_7c15) | 1,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// A number in `0..n`.
    fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % u64::from(n)) as u32
    }
}

/// A bullet's position.
#[derive(Clone, Copy)]
struct Pos {
    x: f32,
    y: f32,
}

/// An enemy bullet's position and velocity (logical units per second).
#[derive(Clone, Copy)]
struct EnemyBulletState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

/// One enemy of a squadron: its home column, its live position, and the row it
/// settles at once it has flown in.
#[derive(Clone, Copy)]
struct EnemyState {
    home_x: f32,
    x: f32,
    y: f32,
    hold_y: f32,
}

/// A game of HAILFALL.
pub struct Game {
    /// Left edge of the ship.
    ship_x: f32,
    /// Top edge of the ship.
    ship_y: f32,
    /// The ship's shots in flight.
    bullets: Vec<Pos>,
    /// Interrupts until the ship may fire again.
    fire_cooldown: u32,
    /// The enemies currently flying.
    enemies: Vec<EnemyState>,
    /// The enemy bullets in the air.
    enemy_bullets: Vec<EnemyBulletState>,
    /// Interrupts since the squadron last fired.
    enemy_fire_tick: u32,
    /// Interrupts until the next squadron flies in, once the field is clear.
    wave_gap: u32,
    /// Lives left; the run ends when this reaches zero.
    lives: u32,
    /// Interrupts of invulnerability left after a hit.
    invuln: u32,
    score: u32,
    /// The run's randomness.
    rng: Rng,
    mode: Mode,
    #[allow(dead_code)]
    loadout: Loadout,
    phase: Phase,
    /// Steps taken so far.
    steps: u64,
    /// The seed the run began on, so a restart replays it exactly.
    seed: u64,
}

impl Game {
    /// Starts a new run on `seed`, in `mode`, flying `loadout`. The same seed and
    /// inputs always produce the same run.
    pub fn new(seed: u64, mode: Mode, loadout: Loadout) -> Self {
        let mut game = Self {
            ship_x: (LOGICAL_WIDTH - SHIP_WIDTH) / 2.0,
            ship_y: LOGICAL_HEIGHT - SHIP_HEIGHT - MARGIN * 3.0,
            bullets: Vec::new(),
            fire_cooldown: 0,
            enemies: Vec::new(),
            enemy_bullets: Vec::new(),
            enemy_fire_tick: 0,
            wave_gap: 0,
            lives: LIVES_START,
            invuln: 0,
            score: 0,
            rng: Rng::new(seed),
            mode,
            loadout,
            phase: Phase::Playing,
            steps: 0,
            seed,
        };
        game.spawn_squadron();
        game
    }

    /// The ship, as the shell should draw it.
    pub fn ship(&self) -> Ship {
        Ship {
            x: self.ship_x,
            y: self.ship_y,
        }
    }

    /// The ship's shots in flight, for the shell to draw.
    pub fn bullets(&self) -> impl Iterator<Item = Bullet> + '_ {
        self.bullets.iter().map(|b| Bullet { x: b.x, y: b.y })
    }

    /// The enemies flying, for the shell to draw.
    pub fn enemies(&self) -> impl Iterator<Item = Enemy> + '_ {
        self.enemies.iter().map(|e| Enemy { x: e.x, y: e.y })
    }

    /// The enemy bullets in the air, for the shell to draw.
    pub fn enemy_bullets(&self) -> impl Iterator<Item = EnemyBullet> + '_ {
        self.enemy_bullets
            .iter()
            .map(|b| EnemyBullet { x: b.x, y: b.y })
    }

    /// Lives left.
    pub fn lives(&self) -> u32 {
        self.lives
    }

    /// Whether the ship is currently spared after a hit (for the shell to flash).
    pub fn invulnerable(&self) -> bool {
        self.invuln > 0
    }

    /// The score so far.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Which run this is.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Where the run is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Starts the run over from the beginning; the same seed replays it.
    pub fn restart(&mut self) {
        *self = Self::new(self.seed, self.mode, self.loadout);
    }

    /// Advances the run by exactly one [`TIMESTEP`].
    pub fn step(&mut self, input: Input) -> Events {
        self.steps += 1;
        let mut events = Events::default();
        if self.phase == Phase::Over {
            return events;
        }
        self.invuln = self.invuln.saturating_sub(1);
        self.fly(input);
        self.fire(input, &mut events);
        self.advance_bullets();
        self.advance_enemies();
        self.resolve_hits(&mut events);
        self.enemy_fire();
        self.advance_enemy_bullets(&mut events);
        self.manage_waves();
        events
    }

    /// Flies the ship on the player's input, kept within the lower band.
    fn fly(&mut self, input: Input) {
        let travel = SHIP_SPEED * TIMESTEP;
        let dx = f32::from(input.right) - f32::from(input.left);
        let dy = f32::from(input.down) - f32::from(input.up);
        self.ship_x += dx * travel;
        self.ship_y += dy * travel;
        self.ship_x = self
            .ship_x
            .clamp(MARGIN, LOGICAL_WIDTH - MARGIN - SHIP_WIDTH);
        self.ship_y = self
            .ship_y
            .clamp(BAND_TOP, LOGICAL_HEIGHT - MARGIN - SHIP_HEIGHT);
    }

    /// Fires a shot on its cadence while fire is held — one every [`FIRE_INTERVAL`]
    /// interrupts, from the ship's nose.
    fn fire(&mut self, input: Input, events: &mut Events) {
        self.fire_cooldown = self.fire_cooldown.saturating_sub(1);
        if input.fire && self.fire_cooldown == 0 {
            self.bullets.push(Pos {
                x: self.ship_x + SHIP_WIDTH / 2.0 - PLAYER_BULLET_WIDTH / 2.0,
                y: self.ship_y - PLAYER_BULLET_HEIGHT,
            });
            self.fire_cooldown = FIRE_INTERVAL;
            events.shot_fired = true;
        }
    }

    /// Climbs every shot, retiring the ones that leave the top of the field.
    fn advance_bullets(&mut self) {
        let dy = PLAYER_BULLET_SPEED * TIMESTEP;
        for b in &mut self.bullets {
            b.y -= dy;
        }
        self.bullets.retain(|b| b.y + PLAYER_BULLET_HEIGHT > 0.0);
    }

    /// Flies the squadron in from above until it settles, then sways it gently
    /// side to side as one.
    fn advance_enemies(&mut self) {
        let sway = SWAY_AMP * (self.steps as f32 * TIMESTEP * SWAY_RATE).sin();
        let dy = ENTRY_SPEED * TIMESTEP;
        for e in &mut self.enemies {
            e.y = (e.y + dy).min(e.hold_y);
            e.x = e.home_x + sway;
        }
    }

    /// Spends each shot on the first enemy it overlaps, downing the enemy and
    /// scoring it.
    fn resolve_hits(&mut self, events: &mut Events) {
        let mut survivors = Vec::with_capacity(self.bullets.len());
        for bullet in std::mem::take(&mut self.bullets) {
            let rect = (
                bullet.x,
                bullet.y,
                PLAYER_BULLET_WIDTH,
                PLAYER_BULLET_HEIGHT,
            );
            let hit = self
                .enemies
                .iter()
                .position(|e| overlaps(rect, (e.x, e.y, ENEMY_WIDTH, ENEMY_HEIGHT)));
            if let Some(i) = hit {
                self.enemies.swap_remove(i);
                self.score += ENEMY_SCORE;
                events.enemy_killed = true;
            } else {
                survivors.push(bullet);
            }
        }
        self.bullets = survivors;
    }

    /// The squadron shoots back on a cadence: a settled enemy, chosen at random,
    /// fires either an aimed dart at the ship or a fan spread toward it.
    fn enemy_fire(&mut self) {
        self.enemy_fire_tick += 1;
        if self.enemy_fire_tick < ENEMY_FIRE_INTERVAL {
            return;
        }
        let shooters: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.y >= e.hold_y - 0.5)
            .map(|(i, _)| i)
            .collect();
        if shooters.is_empty() {
            return;
        }
        self.enemy_fire_tick = 0;

        let e = self.enemies[shooters[self.rng.below(shooters.len() as u32) as usize]];
        let ex = e.x + ENEMY_WIDTH / 2.0;
        let ey = e.y + ENEMY_HEIGHT;
        let angle =
            (self.ship_y + SHIP_HEIGHT / 2.0 - ey).atan2(self.ship_x + SHIP_WIDTH / 2.0 - ex);

        if self.rng.below(2) == 0 {
            self.spawn_enemy_bullet(ex, ey, angle);
        } else {
            let half = (SPREAD_COUNT as f32 - 1.0) / 2.0;
            for i in 0..SPREAD_COUNT {
                self.spawn_enemy_bullet(ex, ey, angle + (i as f32 - half) * SPREAD_STEP);
            }
        }
    }

    /// Adds an enemy bullet at `(x, y)` travelling along `angle`.
    fn spawn_enemy_bullet(&mut self, x: f32, y: f32, angle: f32) {
        self.enemy_bullets.push(EnemyBulletState {
            x,
            y,
            vx: angle.cos() * ENEMY_BULLET_SPEED,
            vy: angle.sin() * ENEMY_BULLET_SPEED,
        });
    }

    /// Flies every enemy bullet, retiring the ones that leave the field. A bullet
    /// that strikes the ship's tiny hitbox — unless it is spared after a hit —
    /// costs a life.
    fn advance_enemy_bullets(&mut self, events: &mut Events) {
        let hitbox = self.hitbox();
        let mut survivors = Vec::with_capacity(self.enemy_bullets.len());
        let mut struck = false;
        for mut b in std::mem::take(&mut self.enemy_bullets) {
            b.x += b.vx * TIMESTEP;
            b.y += b.vy * TIMESTEP;
            if b.x < -ENEMY_BULLET_SIZE
                || b.x > LOGICAL_WIDTH + ENEMY_BULLET_SIZE
                || b.y < -ENEMY_BULLET_SIZE
                || b.y > LOGICAL_HEIGHT + ENEMY_BULLET_SIZE
            {
                continue;
            }
            let rect = (
                b.x - ENEMY_BULLET_SIZE / 2.0,
                b.y - ENEMY_BULLET_SIZE / 2.0,
                ENEMY_BULLET_SIZE,
                ENEMY_BULLET_SIZE,
            );
            if !struck && self.invuln == 0 && overlaps(rect, hitbox) {
                struck = true;
            } else {
                survivors.push(b);
            }
        }
        self.enemy_bullets = survivors;
        if struck {
            self.lose_life(events);
        }
    }

    /// The ship's true hitbox — the tiny square at its heart.
    fn hitbox(&self) -> (f32, f32, f32, f32) {
        let cx = self.ship_x + SHIP_WIDTH / 2.0;
        let cy = self.ship_y + SHIP_HEIGHT / 2.0;
        (
            cx - HITBOX_SIZE / 2.0,
            cy - HITBOX_SIZE / 2.0,
            HITBOX_SIZE,
            HITBOX_SIZE,
        )
    }

    /// Spends a life to a hit: spares the ship for a beat, and ends the run if
    /// that was the last life.
    fn lose_life(&mut self, events: &mut Events) {
        events.player_hit = true;
        self.lives -= 1;
        self.invuln = HIT_INVULN;
        if self.lives == 0 {
            self.phase = Phase::Over;
            events.run_over = true;
        }
    }

    /// Sends in a fresh squadron a short beat after the field is cleared.
    fn manage_waves(&mut self) {
        if self.enemies.is_empty() {
            if self.wave_gap == 0 {
                self.spawn_squadron();
            } else {
                self.wave_gap -= 1;
            }
        } else {
            self.wave_gap = WAVE_GAP;
        }
    }

    /// Sends a squadron flying in from above the top of the field.
    fn spawn_squadron(&mut self) {
        let span = (SQUAD_COLS as f32 - 1.0) * ENEMY_GAP_X;
        let first_centre = (LOGICAL_WIDTH - span) / 2.0;
        for row in 0..SQUAD_ROWS {
            for col in 0..SQUAD_COLS {
                let centre = first_centre + col as f32 * ENEMY_GAP_X;
                let home_x = centre - ENEMY_WIDTH / 2.0;
                let hold_y = SQUAD_TOP + row as f32 * ENEMY_GAP_Y;
                let y = hold_y - LOGICAL_HEIGHT * 0.6 - row as f32 * ENEMY_GAP_Y;
                self.enemies.push(EnemyState {
                    home_x,
                    x: home_x,
                    y,
                    hold_y,
                });
            }
        }
    }
}

/// Whether two rectangles, each `(x, y, width, height)`, overlap.
fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        Game::new(1, Mode::Sortie, Loadout::default())
    }

    fn press(input: Input, steps: usize) -> Game {
        let mut game = game();
        for _ in 0..steps {
            game.step(input);
        }
        game
    }

    #[test]
    fn the_ship_starts_low_and_centred() {
        let ship = game().ship();
        assert!((ship.x - (LOGICAL_WIDTH - SHIP_WIDTH) / 2.0).abs() < 0.01);
        assert!(ship.y > BAND_TOP, "the ship starts down in the band");
    }

    #[test]
    fn the_ship_flies_on_input() {
        let start = game().ship();

        let right = press(
            Input {
                right: true,
                ..Default::default()
            },
            30,
        )
        .ship();
        assert!(right.x > start.x, "holding right flies right");

        let up = press(
            Input {
                up: true,
                ..Default::default()
            },
            30,
        )
        .ship();
        assert!(up.y < start.y, "holding up flies up");
    }

    #[test]
    fn the_ship_is_held_within_the_lower_band() {
        // Push hard into every corner; the ship never leaves its bounds.
        let up_left = press(
            Input {
                up: true,
                left: true,
                ..Default::default()
            },
            10_000,
        )
        .ship();
        assert!(up_left.x >= MARGIN - 0.01, "held off the left wall");
        assert!(
            up_left.y >= BAND_TOP - 0.01,
            "held below the band's ceiling"
        );

        let down_right = press(
            Input {
                down: true,
                right: true,
                ..Default::default()
            },
            10_000,
        )
        .ship();
        assert!(
            down_right.x <= LOGICAL_WIDTH - MARGIN - SHIP_WIDTH + 0.01,
            "held off the right wall"
        );
        assert!(
            down_right.y <= LOGICAL_HEIGHT - MARGIN - SHIP_HEIGHT + 0.01,
            "held off the field's foot"
        );
    }

    #[test]
    fn a_restart_returns_the_ship_to_the_start() {
        let mut game = game();
        let start = game.ship();
        press_into(&mut game, 200);
        game.restart();
        assert_eq!(game.ship(), start, "restart replays from the opening");
    }

    fn press_into(game: &mut Game, steps: usize) {
        for _ in 0..steps {
            game.step(Input {
                right: true,
                down: true,
                ..Default::default()
            });
        }
    }

    fn firing() -> Input {
        Input {
            fire: true,
            ..Default::default()
        }
    }

    #[test]
    fn holding_fire_launches_a_shot_that_climbs() {
        let mut game = game();
        game.step(firing());
        let launched = game
            .bullets()
            .next()
            .expect("holding fire launches a shot")
            .y;

        // The cadence holds the next shot back a few steps, so this one is alone
        // and climbing.
        for _ in 0..4 {
            game.step(firing());
        }
        let now = game.bullets().next().expect("the shot is in flight").y;
        assert!(now < launched, "the shot climbs the field");
    }

    #[test]
    fn a_squadron_flies_in_and_settles() {
        let mut game = game();
        assert_eq!(
            game.enemies().count(),
            SQUAD_COLS * SQUAD_ROWS,
            "a full squadron enters"
        );

        // Let it fly in; the top row settles in view near its hold row.
        for _ in 0..300 {
            game.step(Input::default());
        }
        let top = game.enemies().map(|e| e.y).fold(f32::INFINITY, f32::min);
        assert!(
            (top - SQUAD_TOP).abs() < 1.0,
            "the squadron settled at its row"
        );
    }

    #[test]
    fn a_shot_downs_an_enemy_and_scores() {
        let mut game = game();
        let before = game.enemies().count();

        // Hold fire; the squadron sways over the ship and a shot connects.
        let mut downed = false;
        for _ in 0..MAX_STEPS {
            let events = game.step(firing());
            if events.enemy_killed {
                downed = true;
                break;
            }
        }
        assert!(downed, "a held stream of fire never downed an enemy");
        assert!(game.enemies().count() < before, "the squadron thinned");
        assert!(game.score() > 0, "downing an enemy scores");
    }

    #[test]
    fn the_settled_squadron_fires_back() {
        let mut game = game();
        let mut fired = false;
        for _ in 0..1_500 {
            game.step(Input::default());
            if game.enemy_bullets().next().is_some() {
                fired = true;
                break;
            }
        }
        assert!(fired, "the settled squadron fires back");
    }

    /// Places a still enemy bullet centred on `(x, y)`.
    fn bullet_at(game: &mut Game, x: f32, y: f32) {
        game.enemy_bullets.push(EnemyBulletState {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
        });
    }

    fn ship_centre(game: &Game) -> (f32, f32) {
        let s = game.ship();
        (s.x + SHIP_WIDTH / 2.0, s.y + SHIP_HEIGHT / 2.0)
    }

    #[test]
    fn a_bullet_on_the_hitbox_costs_a_life() {
        let mut game = game();
        let (cx, cy) = ship_centre(&game);
        bullet_at(&mut game, cx, cy);

        let events = game.step(Input::default());

        assert!(events.player_hit, "a bullet on the hitbox strikes the ship");
        assert_eq!(game.lives(), LIVES_START - 1, "and costs a life");
    }

    #[test]
    fn a_shot_through_the_hull_but_off_the_hitbox_spares_the_ship() {
        let mut game = game();
        let (cx, cy) = ship_centre(&game);
        // Inside the wide hull, but clear of the tiny hitbox.
        bullet_at(&mut game, cx + 4.0, cy);

        game.step(Input::default());

        assert_eq!(game.lives(), LIVES_START, "only the tiny hitbox can be hit");
    }

    #[test]
    fn spending_the_last_life_ends_the_run() {
        let mut game = game();
        game.lives = 1;
        let (cx, cy) = ship_centre(&game);
        bullet_at(&mut game, cx, cy);

        let events = game.step(Input::default());

        assert!(events.run_over, "the last life ends the run");
        assert_eq!(game.phase(), Phase::Over);
    }

    /// A generous ceiling on how long a firing test plays before giving up.
    const MAX_STEPS: usize = 20_000;
}
