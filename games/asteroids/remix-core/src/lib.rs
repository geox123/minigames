//! The pure, deterministic core of **ACCRETE** — Asteroids' Remix, a gravity
//! reimagining of the 1979 vector rock-shooter.
//!
//! Where the Faithful is Newtonian *drift* on an empty field, ACCRETE drops a
//! **gravity well** — a star — into the heart of that field and lets it pull on
//! everything. You fly the Faithful's ship *against* the pull: thrust to hold an
//! orbit, coast when you let go, and **slingshot** on a close pass to whip up speed.
//! Like every core in the Collection it owns the rules and knows nothing of
//! rendering, audio, windows or wall-clock time, and advances in fixed timesteps so
//! a seed and a sequence of inputs always replay the same run.
//!
//! It plays on the Faithful's **1024×768 toroidal field**, so one shell canvas
//! serves both takes; gravity reaches *across* the wrap, toward the nearest image of
//! the well.
//!
//! So far: the gravity field and the Newtonian ship (A1); a held stream of fire and
//! rocks that split and curve under the pull (A2); and the well **accreting** rocks
//! for a streak-fed score, with a ship that falls into a core — or is struck by a
//! rock — destroyed (A3). The collapse, enemies, bosses, the modes and the meta
//! arrive in the later tickets; the ship's **loadout** is handed *in* at
//! construction, so the core never knows the word "unlock" — it only ever flies what
//! it is given.

use core::f32::consts::TAU;

/// Width of the toroidal play field, in logical units — shared with the Faithful.
pub const LOGICAL_WIDTH: f32 = 1024.0;
/// Height of the toroidal play field, in logical units — shared with the Faithful.
pub const LOGICAL_HEIGHT: f32 = 768.0;

/// Length of a single simulation step, in seconds — the Collection's 120 Hz.
pub const TIMESTEP: f32 = 1.0 / 120.0;

/// The ship's collision radius, in logical units — used for collision (later
/// tickets) and as the scale the shell draws it at.
pub const SHIP_RADIUS: f32 = 14.0;

/// How fast the ship turns, in radians per second. A **feel constant** — the motion
/// model is fixed, but this and the ones below are tuned against the running shell.
const SHIP_TURN_RATE: f32 = 4.0;
/// The acceleration thrust adds along the facing, in units per second² — a touch
/// stronger than the Faithful's so the ship can fight the pull.
const SHIP_THRUST: f32 = 380.0;
/// A gentle space-friction, per second — light enough that orbits hold with only the
/// occasional nudge.
const SHIP_FRICTION: f32 = 0.25;
/// The top speed the ship may reach, in units per second — high, so a slingshot pays.
const SHIP_MAX_SPEED: f32 = 640.0;

/// The gravity well: the strength of its pull (an inverse-square constant), how close
/// a body may get before the pull is softened (so it never blows up), and the radius
/// of its core — its event horizon, where bodies are consumed (from the later
/// tickets; drawn from the start).
const GRAVITY: f32 = 8_000_000.0;
const SOFTENING: f32 = 44.0;
pub const WELL_CORE_RADIUS: f32 = 26.0;

/// The centre of the field, where the well sits and the ship starts clear of it.
const CENTER_X: f32 = LOGICAL_WIDTH / 2.0;
const CENTER_Y: f32 = LOGICAL_HEIGHT / 2.0;
/// How far above the well the ship begins, at rest.
const SHIP_START_OFFSET: f32 = 260.0;

/// The player's fire: a held stream. Bullet speed, the steps between shots while
/// fire is held, how long a shot lives before it burns out, and its radius. Shots
/// feel the gravity too, so they curve on the way out.
const SHOT_SPEED: f32 = 520.0;
const FIRE_INTERVAL: u64 = 8;
const SHOT_LIFE: f32 = 1.6;
const SHOT_RADIUS: f32 = 3.0;

/// The rocks a field opens with, how fast a large one drifts to start, and how a
/// split fragment's speed multiplies its parent's — always above 1, so a broken
/// field speeds up.
const INITIAL_ROCKS: usize = 5;
const ROCK_MIN_SPEED: f32 = 30.0;
const ROCK_MAX_SPEED: f32 = 70.0;
const FRAGMENT_SPEED_MIN: f32 = 1.1;
const FRAGMENT_SPEED_MAX: f32 = 1.6;
/// No rock spawns within this of the well, so a fresh field never starts inside it.
const ROCK_SAFE_RADIUS: f32 = 200.0;

/// The player's ships to start, the pause the run holds after one is destroyed, and
/// how long a fresh ship is protected on arrival.
pub const LIVES_START: u32 = 3;
const DEATH_PAUSE: f32 = 1.2;
const SPAWN_INVULN: f32 = 2.5;

/// Accretion: how long a feed streak lives between rocks (each rock resets it), how
/// high the streak counts, and how many rocks the streak lifts the score by one
/// multiple — so a fast, steady feed of the well pays far more than an idle one.
const FEED_WINDOW: f32 = 2.0;
const FEED_STREAK_CAP: u32 = 16;
const FEED_PER_MULTIPLE: u32 = 4;

/// The collapse: how wide a band beyond a well's core the ship skims to charge the
/// meter, how much one pass through that band charges it (so a few skims fill it),
/// and how hard a collapse flings the rocks outward. Skimming only counts while the
/// ship is vulnerable — flirting with the core is the price of the charge.
const SKIM_BAND: f32 = 46.0;
const SKIM_CHARGE: f32 = 0.34;
const COLLAPSE_IMPULSE: f32 = 440.0;

/// Which run this is. Inert for now — every mode plays the same until the modes
/// ticket shapes their ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// A finite, winnable ladder of systems.
    #[default]
    Orbit,
    /// Endless, the well tightening and the field flooding, scored for survival.
    Maelstrom,
    /// Endless, seeded by the calendar day.
    Daily,
}

/// What the run was built with — the ship options earned in the meta. Empty and
/// inert until the meta ticket fills it in; it exists now so `new` keeps a stable
/// shape (the core is always handed a loadout, and never knows where it came from).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loadout {}

/// What the player is doing this step. `fire` and `collapse` are wired now so the
/// input shape stays stable across tickets, but the core ignores them until the
/// tickets that add firing and the collapse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// Rotate anticlockwise (to the ship's left).
    pub turn_left: bool,
    /// Rotate clockwise (to the ship's right).
    pub turn_right: bool,
    /// Thrust along the ship's facing.
    pub thrust: bool,
    /// Fire. *(Ignored until firing lands.)*
    pub fire: bool,
    /// Spend a full meter on a collapse. *(Ignored until the collapse lands.)*
    pub collapse: bool,
}

/// The player's ship, as the shell should draw it. `x`/`y` are its centre; `angle`
/// is its facing in radians, `0` straight up, increasing clockwise. `vx`/`vy` are its
/// velocity — the shell may draw motion from it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ship {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub angle: f32,
    /// Whether the ship is thrusting this step — the shell draws the flame from it.
    pub thrusting: bool,
}

/// A gravity well on the field, as the shell should draw it. `x`/`y` are its centre;
/// [`WELL_CORE_RADIUS`] is the radius of its consuming core.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Well {
    pub x: f32,
    pub y: f32,
}

/// The three sizes a rock comes in. A shot splits a large into mediums and a medium
/// into smalls; a small is destroyed outright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl AsteroidSize {
    /// The rock's collision radius, in logical units.
    pub fn radius(self) -> f32 {
        match self {
            AsteroidSize::Large => 58.0,
            AsteroidSize::Medium => 30.0,
            AsteroidSize::Small => 15.0,
        }
    }

    /// The size the rock breaks into — two of these — or `None` for a small.
    fn child(self) -> Option<AsteroidSize> {
        match self {
            AsteroidSize::Large => Some(AsteroidSize::Medium),
            AsteroidSize::Medium => Some(AsteroidSize::Small),
            AsteroidSize::Small => None,
        }
    }

    /// What the well scores for devouring a rock of this size — the larger the rock,
    /// the more it is worth to feed it in whole.
    fn accrete_score(self) -> u32 {
        match self {
            AsteroidSize::Large => 150,
            AsteroidSize::Medium => 75,
            AsteroidSize::Small => 30,
        }
    }
}

/// A rock adrift on the gravity field, as the shell should draw it. `x`/`y` are its
/// centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Asteroid {
    pub x: f32,
    pub y: f32,
    pub size: AsteroidSize,
}

/// One of the player's shots in flight, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub x: f32,
    pub y: f32,
}

/// An explosion in progress, for the shell to draw where the ship was destroyed.
/// `progress` runs `0.0` (just born) to `1.0` (done).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    pub x: f32,
    pub y: f32,
    pub progress: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to. It grows
/// as accretion, the collapse and the rest arrive in the later tickets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The ship fired a shot this step.
    pub fired: bool,
    /// A shot split a rock this step.
    pub rock_split: bool,
    /// The well accreted a rock this step.
    pub accreted: bool,
    /// The ship skimmed a well's edge this step, charging the collapse meter.
    pub skimmed: bool,
    /// A collapse was fired this step.
    pub collapse_fired: bool,
    /// The ship was destroyed this step and a life was lost.
    pub ship_destroyed: bool,
    /// The last ship was spent this step — the run is over.
    pub game_over: bool,
}

/// Where a run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The run is being played.
    Playing,
    /// The run is over.
    Over,
}

/// The ship's live kinematic state.
#[derive(Clone, Copy)]
struct ShipState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    angle: f32,
}

/// A well's live state.
#[derive(Clone, Copy)]
struct WellState {
    x: f32,
    y: f32,
}

/// A rock's live state: its centre, its velocity (drift plus whatever the gravity has
/// added), and its size.
#[derive(Clone, Copy)]
struct AsteroidState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: AsteroidSize,
}

/// A shot's live state: its centre, its velocity, and the seconds of life it has left.
#[derive(Clone, Copy)]
struct ShotState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
}

/// An explosion in progress, with the seconds it has left to burn.
#[derive(Clone, Copy)]
struct BlastState {
    x: f32,
    y: f32,
    timer: f32,
}

/// How long an explosion lingers for the shell to draw, in seconds.
const BLAST_LIFE: f32 = 0.5;

/// The whole run: the ship, the gravity wells that pull on it, and the seed the run
/// began on. Advanced only through [`Game::step`]; everything else is read-only.
pub struct Game {
    ship: ShipState,
    thrusting: bool,
    wells: Vec<WellState>,
    asteroids: Vec<AsteroidState>,
    shots: Vec<ShotState>,
    blasts: Vec<BlastState>,
    /// Steps until the held stream may loose its next shot.
    fire_cooldown: u64,
    /// Whether the ship is on the field; false during the pause after a destruction.
    ship_alive: bool,
    /// Seconds left in the pause after a destruction, before the ship returns.
    dead_timer: f32,
    /// Seconds of arrival protection left; while it lasts the ship cannot be hit.
    invuln: f32,
    /// Whether the destruction playing out was the last life — the run ends.
    ending: bool,
    /// The accretion feed streak, and the seconds it survives before it lapses — a
    /// steady feed of the well ramps the score.
    feed_streak: u32,
    feed_timer: f32,
    /// The collapse meter (0..=1), whether the ship was skimming a band last step (so
    /// a skim charges once per pass), and whether collapse was held last step (so it
    /// fires on the press).
    collapse_meter: f32,
    skimming: bool,
    collapse_was_down: bool,
    lives: u32,
    mode: Mode,
    /// The loadout the run was built with — inert now, but stored so a restart
    /// replays the very same run once the meta ticket gives it teeth.
    loadout: Loadout,
    phase: Phase,
    score: u32,
    rng: Rng,
    steps: u64,
    seed: u64,
}

impl Game {
    /// Starts a run on `seed` in `mode`, flying `loadout`. The same seed and inputs
    /// always replay the same run. (The mode and loadout are inert until the later
    /// tickets give them teeth.)
    pub fn new(seed: u64, mode: Mode, loadout: Loadout) -> Self {
        let mut game = Self {
            ship: ShipState {
                x: CENTER_X,
                y: CENTER_Y - SHIP_START_OFFSET,
                vx: 0.0,
                vy: 0.0,
                angle: 0.0,
            },
            thrusting: false,
            wells: vec![WellState {
                x: CENTER_X,
                y: CENTER_Y,
            }],
            asteroids: Vec::with_capacity(INITIAL_ROCKS),
            shots: Vec::new(),
            blasts: Vec::new(),
            fire_cooldown: 0,
            ship_alive: true,
            dead_timer: 0.0,
            invuln: SPAWN_INVULN,
            ending: false,
            feed_streak: 0,
            feed_timer: 0.0,
            collapse_meter: 0.0,
            skimming: false,
            collapse_was_down: false,
            lives: LIVES_START,
            mode,
            loadout,
            phase: Phase::Playing,
            score: 0,
            rng: Rng::new(seed),
            steps: 0,
            seed,
        };
        game.spawn_field(INITIAL_ROCKS);
        game
    }

    /// Lays out `count` large rocks, none within [`ROCK_SAFE_RADIUS`] of the well,
    /// each drifting at a seeded heading and speed.
    fn spawn_field(&mut self, count: usize) {
        for _ in 0..count {
            let (x, y) = loop {
                let x = self.rng.range(0.0, LOGICAL_WIDTH);
                let y = self.rng.range(0.0, LOGICAL_HEIGHT);
                let dx = x - CENTER_X;
                let dy = y - CENTER_Y;
                if dx * dx + dy * dy >= ROCK_SAFE_RADIUS * ROCK_SAFE_RADIUS {
                    break (x, y);
                }
            };
            let heading = self.rng.range(0.0, TAU);
            let speed = self.rng.range(ROCK_MIN_SPEED, ROCK_MAX_SPEED);
            self.asteroids.push(AsteroidState {
                x,
                y,
                vx: heading.cos() * speed,
                vy: heading.sin() * speed,
                size: AsteroidSize::Large,
            });
        }
    }

    /// Advances the run one fixed timestep, returning what happened for the shell to
    /// react to.
    pub fn step(&mut self, input: Input) -> Events {
        self.steps += 1;
        let mut events = Events::default();
        if self.phase == Phase::Over {
            return events;
        }
        let collapse_pressed = input.collapse && !self.collapse_was_down;
        self.collapse_was_down = input.collapse;

        if self.ship_alive {
            self.advance_ship(input);
            if input.fire && self.fire_cooldown == 0 {
                self.fire();
                events.fired = true;
                self.fire_cooldown = FIRE_INTERVAL;
            }
            self.resolve_skim(&mut events);
            self.try_collapse(collapse_pressed, &mut events);
        } else {
            self.advance_death(&mut events);
            self.skimming = false;
        }
        self.fire_cooldown = self.fire_cooldown.saturating_sub(1);

        self.advance_shots();
        self.advance_asteroids();
        self.advance_blasts();
        if self.invuln > 0.0 {
            self.invuln = (self.invuln - TIMESTEP).max(0.0);
        }
        self.decay_feed_streak();

        self.resolve_shot_hits(&mut events);
        self.resolve_accretion(&mut events);
        if self.ship_alive && self.invuln <= 0.0 {
            self.resolve_ship_death(&mut events);
        }
        events
    }

    /// Runs the pause after a destruction: hold for a beat, then either end the run
    /// (if that was the last life) or return the ship to its start under protection.
    fn advance_death(&mut self, events: &mut Events) {
        if self.dead_timer > 0.0 {
            self.dead_timer -= TIMESTEP;
            return;
        }
        if self.ending {
            self.phase = Phase::Over;
            events.game_over = true;
            return;
        }
        self.ship = ShipState {
            x: CENTER_X,
            y: CENTER_Y - SHIP_START_OFFSET,
            vx: 0.0,
            vy: 0.0,
            angle: 0.0,
        };
        self.ship_alive = true;
        self.invuln = SPAWN_INVULN;
    }

    /// Burns down the explosions and drops the ones that have finished.
    fn advance_blasts(&mut self) {
        for b in &mut self.blasts {
            b.timer -= TIMESTEP;
        }
        self.blasts.retain(|b| b.timer > 0.0);
    }

    /// Lapses the accretion feed streak once no rock has fed the well for a while.
    fn decay_feed_streak(&mut self) {
        if self.feed_timer > 0.0 {
            self.feed_timer -= TIMESTEP;
            if self.feed_timer <= 0.0 {
                self.feed_streak = 0;
            }
        }
    }

    /// Rocks that cross a well's core are devoured — removed and scored, worth more
    /// the larger the rock and the steadier the feed (a streak lifts the score).
    fn resolve_accretion(&mut self, events: &mut Events) {
        let mut i = 0;
        while i < self.asteroids.len() {
            let rock = self.asteroids[i];
            if in_any_core(&self.wells, rock.x, rock.y) {
                self.feed_streak = (self.feed_streak + 1).min(FEED_STREAK_CAP);
                self.feed_timer = FEED_WINDOW;
                let multiple = 1 + self.feed_streak / FEED_PER_MULTIPLE;
                self.score += rock.size.accrete_score() * multiple;
                events.accreted = true;
                self.asteroids.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// The ship is destroyed if it falls into a well's core or is struck by a rock.
    fn resolve_ship_death(&mut self, events: &mut Events) {
        let ship = (self.ship.x, self.ship.y, SHIP_RADIUS);
        let into_well = in_any_core(&self.wells, self.ship.x, self.ship.y);
        let hit_rock = self
            .asteroids
            .iter()
            .any(|a| overlap(ship, (a.x, a.y, a.size.radius())));
        if into_well || hit_rock {
            self.destroy_ship(events);
        }
    }

    /// Charges the collapse meter when the ship enters a well's skim band — once per
    /// pass, and only while it is vulnerable, so the charge is earned by real risk.
    /// The "charging" latch folds in the vulnerability gate, so a pass that only
    /// becomes at-risk part-way through still earns its charge on that fresh edge.
    fn resolve_skim(&mut self, events: &mut Events) {
        let charging = self.ship_in_skim_band() && self.invuln <= 0.0;
        if charging && !self.skimming {
            self.collapse_meter = (self.collapse_meter + SKIM_CHARGE).min(1.0);
            events.skimmed = true;
        }
        self.skimming = charging;
    }

    /// Whether the ship is within a well's skim band — just outside a core, not in it.
    fn ship_in_skim_band(&self) -> bool {
        let inner = WELL_CORE_RADIUS;
        let outer = WELL_CORE_RADIUS + SKIM_BAND;
        self.wells.iter().any(|w| {
            let d2 = ring_dist_sq(w.x, w.y, self.ship.x, self.ship.y);
            d2 >= inner * inner && d2 <= outer * outer
        })
    }

    /// Spends a full meter, on the press, on a collapse: a shockwave from the wells
    /// that destroys the fine debris it catches and flings the rest of the rocks
    /// outward. (It also destroys the enemies it catches, once they arrive.)
    fn try_collapse(&mut self, pressed: bool, events: &mut Events) {
        if !pressed || self.collapse_meter < 1.0 {
            return;
        }
        // The shockwave shatters the small, fast debris and blows the rest outward.
        self.asteroids.retain(|a| a.size != AsteroidSize::Small);
        for a in &mut self.asteroids {
            let (gx, gy) = gravity_at(&self.wells, a.x, a.y);
            let mag = (gx * gx + gy * gy).sqrt();
            if mag > 0.0 {
                a.vx -= gx / mag * COLLAPSE_IMPULSE;
                a.vy -= gy / mag * COLLAPSE_IMPULSE;
            }
        }
        self.collapse_meter = 0.0;
        events.collapse_fired = true;
    }

    /// Destroys the ship: a life lost, an explosion, the sky cleared, the streak
    /// broken, and the pause before it returns — or the run's end, on the last life.
    fn destroy_ship(&mut self, events: &mut Events) {
        self.lives = self.lives.saturating_sub(1);
        self.ship_alive = false;
        self.thrusting = false;
        self.dead_timer = DEATH_PAUSE;
        self.ending = self.lives == 0;
        self.shots.clear();
        self.feed_streak = 0;
        self.blasts.push(BlastState {
            x: self.ship.x,
            y: self.ship.y,
            timer: BLAST_LIFE,
        });
        events.ship_destroyed = true;
    }

    /// Loosens a shot from the ship's nose along its facing. The shot flies at a fixed
    /// world speed and then feels the gravity like everything else, so it curves.
    fn fire(&mut self) {
        let (fx, fy) = facing(self.ship.angle);
        self.shots.push(ShotState {
            x: self.ship.x + fx * SHIP_RADIUS * 1.3,
            y: self.ship.y + fy * SHIP_RADIUS * 1.3,
            vx: fx * SHOT_SPEED,
            vy: fy * SHOT_SPEED,
            life: SHOT_LIFE,
        });
    }

    /// Flies every shot under the wells' pull, wrapping it, and drops the spent ones.
    fn advance_shots(&mut self) {
        for s in &mut self.shots {
            let (gx, gy) = gravity_at(&self.wells, s.x, s.y);
            s.vx += gx * TIMESTEP;
            s.vy += gy * TIMESTEP;
            s.x = wrap(s.x + s.vx * TIMESTEP, LOGICAL_WIDTH);
            s.y = wrap(s.y + s.vy * TIMESTEP, LOGICAL_HEIGHT);
            s.life -= TIMESTEP;
        }
        self.shots.retain(|s| s.life > 0.0);
    }

    /// Drifts every rock under the wells' pull, wrapping it — so the field orbits.
    fn advance_asteroids(&mut self) {
        for a in &mut self.asteroids {
            let (gx, gy) = gravity_at(&self.wells, a.x, a.y);
            a.vx += gx * TIMESTEP;
            a.vy += gy * TIMESTEP;
            a.x = wrap(a.x + a.vx * TIMESTEP, LOGICAL_WIDTH);
            a.y = wrap(a.y + a.vy * TIMESTEP, LOGICAL_HEIGHT);
        }
    }

    /// Resolves shots striking rocks: the rock splits into two faster fragments (a
    /// small into none) and the shot is spent. Scoring comes with accretion.
    fn resolve_shot_hits(&mut self, events: &mut Events) {
        let mut fragments = Vec::new();
        let mut i = 0;
        while i < self.shots.len() {
            let (sx, sy) = (self.shots[i].x, self.shots[i].y);
            let hit = self
                .asteroids
                .iter()
                .position(|a| overlap((sx, sy, SHOT_RADIUS), (a.x, a.y, a.size.radius())));
            if let Some(j) = hit {
                let rock = self.asteroids.swap_remove(j);
                events.rock_split = true;
                if let Some(child) = rock.size.child() {
                    let parent_speed = (rock.vx * rock.vx + rock.vy * rock.vy).sqrt();
                    for _ in 0..2 {
                        let heading = self.rng.range(0.0, TAU);
                        let speed =
                            parent_speed * self.rng.range(FRAGMENT_SPEED_MIN, FRAGMENT_SPEED_MAX);
                        fragments.push(AsteroidState {
                            x: rock.x,
                            y: rock.y,
                            vx: heading.cos() * speed,
                            vy: heading.sin() * speed,
                            size: child,
                        });
                    }
                }
                self.shots.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.asteroids.append(&mut fragments);
    }

    /// Turns, thrusts and moves the ship, under the wells' pull, wrapping it at the
    /// field edges.
    fn advance_ship(&mut self, input: Input) {
        if input.turn_left && !input.turn_right {
            self.ship.angle -= SHIP_TURN_RATE * TIMESTEP;
        }
        if input.turn_right && !input.turn_left {
            self.ship.angle += SHIP_TURN_RATE * TIMESTEP;
        }
        self.ship.angle = self.ship.angle.rem_euclid(TAU);

        self.thrusting = input.thrust;
        if input.thrust {
            let (fx, fy) = facing(self.ship.angle);
            self.ship.vx += SHIP_THRUST * fx * TIMESTEP;
            self.ship.vy += SHIP_THRUST * fy * TIMESTEP;
        }

        // The wells pull the ship in.
        let (gx, gy) = gravity_at(&self.wells, self.ship.x, self.ship.y);
        self.ship.vx += gx * TIMESTEP;
        self.ship.vy += gy * TIMESTEP;

        // A gentle friction, then a hard top speed.
        let drag = 1.0 - SHIP_FRICTION * TIMESTEP;
        self.ship.vx *= drag;
        self.ship.vy *= drag;
        let speed_sq = self.ship.vx * self.ship.vx + self.ship.vy * self.ship.vy;
        if speed_sq > SHIP_MAX_SPEED * SHIP_MAX_SPEED {
            let scale = SHIP_MAX_SPEED / speed_sq.sqrt();
            self.ship.vx *= scale;
            self.ship.vy *= scale;
        }

        self.ship.x = wrap(self.ship.x + self.ship.vx * TIMESTEP, LOGICAL_WIDTH);
        self.ship.y = wrap(self.ship.y + self.ship.vy * TIMESTEP, LOGICAL_HEIGHT);
    }

    /// The ship, as the shell should draw it.
    pub fn ship(&self) -> Ship {
        Ship {
            x: self.ship.x,
            y: self.ship.y,
            vx: self.ship.vx,
            vy: self.ship.vy,
            angle: self.ship.angle,
            thrusting: self.thrusting,
        }
    }

    /// The gravity wells on the field, as the shell should draw them.
    pub fn wells(&self) -> impl Iterator<Item = Well> + '_ {
        self.wells.iter().map(|w| Well { x: w.x, y: w.y })
    }

    /// The rocks adrift on the field, as the shell should draw them.
    pub fn asteroids(&self) -> impl Iterator<Item = Asteroid> + '_ {
        self.asteroids.iter().map(|a| Asteroid {
            x: a.x,
            y: a.y,
            size: a.size,
        })
    }

    /// How many rocks are on the field.
    pub fn asteroid_count(&self) -> usize {
        self.asteroids.len()
    }

    /// The shots in flight, as the shell should draw them.
    pub fn shots(&self) -> impl Iterator<Item = Shot> + '_ {
        self.shots.iter().map(|s| Shot { x: s.x, y: s.y })
    }

    /// The explosions in progress, as the shell should draw them.
    pub fn blasts(&self) -> impl Iterator<Item = Blast> + '_ {
        self.blasts.iter().map(|b| Blast {
            x: b.x,
            y: b.y,
            progress: 1.0 - b.timer / BLAST_LIFE,
        })
    }

    /// The ships left, the one in play included.
    pub fn lives(&self) -> u32 {
        self.lives
    }

    /// Whether the ship is on the field this step (false during the death pause).
    pub fn ship_alive(&self) -> bool {
        self.ship_alive
    }

    /// Whether the ship is under arrival protection — the shell may blink it.
    pub fn ship_invulnerable(&self) -> bool {
        self.invuln > 0.0
    }

    /// The accretion feed streak — how many rocks the well has devoured in a row.
    pub fn feed_streak(&self) -> u32 {
        self.feed_streak
    }

    /// The collapse meter, `0.0..=1.0` — a full meter can be spent on a collapse.
    pub fn collapse_meter(&self) -> f32 {
        self.collapse_meter
    }

    /// The running score.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Which mode this run is.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Where the run is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Starts the run over from the beginning; the same seed, mode and loadout replay
    /// it exactly.
    pub fn restart(&mut self) {
        *self = Self::new(self.seed, self.mode, self.loadout);
    }
}

/// The unit facing vector for `angle` (angle 0 points up, increasing clockwise).
fn facing(angle: f32) -> (f32, f32) {
    (angle.sin(), -angle.cos())
}

/// Wraps a coordinate into `[0, max)` — the field's toroidal topology.
fn wrap(v: f32, max: f32) -> f32 {
    v.rem_euclid(max)
}

/// The shortest signed distance from `b` to `a` on a `max`-wide ring.
fn ring_delta(a: f32, b: f32, max: f32) -> f32 {
    let mut d = a - b;
    if d > max / 2.0 {
        d -= max;
    } else if d < -max / 2.0 {
        d += max;
    }
    d
}

/// The gravitational acceleration on a body at `(x, y)` — the sum of every well's
/// inverse-square pull, softened near a core, measured across the toroidal field so
/// the pull is toward the nearest image of each well. A free function (not a `&self`
/// method) so a body can integrate it while its own `Vec` is being iterated.
fn gravity_at(wells: &[WellState], x: f32, y: f32) -> (f32, f32) {
    let mut ax = 0.0;
    let mut ay = 0.0;
    for well in wells {
        let dx = ring_delta(well.x, x, LOGICAL_WIDTH);
        let dy = ring_delta(well.y, y, LOGICAL_HEIGHT);
        let dist_sq = (dx * dx + dy * dy).max(SOFTENING * SOFTENING);
        let dist = dist_sq.sqrt();
        let accel = GRAVITY / dist_sq;
        ax += accel * dx / dist;
        ay += accel * dy / dist;
    }
    (ax, ay)
}

/// The squared distance between `(ax, ay)` and `(bx, by)` across the toroidal field.
fn ring_dist_sq(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ring_delta(ax, bx, LOGICAL_WIDTH);
    let dy = ring_delta(ay, by, LOGICAL_HEIGHT);
    dx * dx + dy * dy
}

/// Whether two circles — each a `(centre-x, centre-y, radius)` — overlap, measured
/// across the toroidal field.
fn overlap(a: (f32, f32, f32), b: (f32, f32, f32)) -> bool {
    let r = a.2 + b.2;
    ring_dist_sq(a.0, a.1, b.0, b.1) < r * r
}

/// Whether `(x, y)` is inside any well's consuming core, across the toroidal field.
fn in_any_core(wells: &[WellState], x: f32, y: f32) -> bool {
    wells
        .iter()
        .any(|w| ring_dist_sq(w.x, w.y, x, y) < WELL_CORE_RADIUS * WELL_CORE_RADIUS)
}

/// The Collection's small deterministic PRNG: splitmix64 to spread the seed, then
/// xorshift for the stream. Kept local so the core has no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        Self(z | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A uniform float in `lo..hi`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * unit
    }
}

#[cfg(test)]
mod tests {
    //! The slingshot — a tangential close pass of the well — is impractical to stage
    //! by honest play from the fixed start, so it is set up directly (the ship placed
    //! with a tangential velocity) and then driven through the real [`Game::step`]
    //! path. Everything else is reachable and lives in `tests/`.
    use super::*;

    fn speed(ship: Ship) -> f32 {
        (ship.vx * ship.vx + ship.vy * ship.vy).sqrt()
    }

    #[test]
    fn a_close_pass_slingshots_the_ship() {
        let mut game = Game::new(1, Mode::Orbit, Loadout::default());
        // Abreast of the well and moving tangentially past it — it swings around the
        // core, whipped up *and bent aside*: a slingshot, not a straight radial fall.
        game.ship = ShipState {
            x: CENTER_X - 120.0,
            y: CENTER_Y,
            vx: 0.0,
            vy: 260.0,
            angle: 0.0,
        };
        let start_speed = speed(game.ship());
        let start_dir = game.ship.vy.atan2(game.ship.vx);

        let mut peak_speed = start_speed;
        let mut peak_deflection = 0.0_f32;
        for _ in 0..200 {
            game.step(Input::default());
            let ship = game.ship();
            peak_speed = peak_speed.max(speed(ship));
            let dir = ship.vy.atan2(ship.vx);
            peak_deflection = peak_deflection.max(ring_delta(dir, start_dir, TAU).abs());
        }
        assert!(
            peak_speed > start_speed * 1.15,
            "the pass whips the ship up"
        );
        assert!(
            peak_deflection > 0.6,
            "and bends its heading — a swing-by, not a radial fall (got {peak_deflection})"
        );
    }

    /// A run with the field cleared, so a test can plant exactly what it needs.
    fn empty_field(seed: u64) -> Game {
        let mut game = Game::new(seed, Mode::Orbit, Loadout::default());
        game.asteroids.clear();
        game
    }

    fn plant_rock(game: &mut Game, size: AsteroidSize, x: f32, y: f32, drift: f32) {
        game.asteroids.push(AsteroidState {
            x,
            y,
            vx: drift,
            vy: 0.0,
            size,
        });
    }

    fn plant_shot(game: &mut Game, x: f32, y: f32) {
        game.shots.push(ShotState {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            life: SHOT_LIFE,
        });
    }

    #[test]
    fn a_shot_curves_under_gravity() {
        let mut game = empty_field(1);
        // A shot crossing above the well, moving right: the pull bends it downward.
        game.shots.push(ShotState {
            x: CENTER_X - 100.0,
            y: CENTER_Y - 120.0,
            vx: 300.0,
            vy: 0.0,
            life: SHOT_LIFE,
        });
        game.step(Input::default());
        assert!(
            game.shots[0].vy > 0.0,
            "the shot is bent toward the well, gaining downward velocity"
        );
    }

    #[test]
    fn a_shot_splits_a_large_rock_into_two_mediums() {
        let mut game = empty_field(1);
        plant_rock(&mut game, AsteroidSize::Large, 300.0, 300.0, 20.0);
        plant_shot(&mut game, 300.0, 300.0);

        let events = game.step(Input::default());

        assert!(events.rock_split);
        assert_eq!(game.asteroid_count(), 2);
        assert!(game.asteroids().all(|a| a.size == AsteroidSize::Medium));
    }

    #[test]
    fn a_small_rock_is_destroyed_outright() {
        let mut game = empty_field(1);
        plant_rock(&mut game, AsteroidSize::Small, 300.0, 300.0, 20.0);
        plant_shot(&mut game, 300.0, 300.0);

        game.step(Input::default());

        assert_eq!(game.asteroid_count(), 0);
    }

    #[test]
    fn fragments_are_faster_than_their_parent() {
        let mut game = empty_field(1);
        let parent_drift = 40.0;
        plant_rock(&mut game, AsteroidSize::Large, 300.0, 300.0, parent_drift);
        plant_shot(&mut game, 300.0, 300.0);

        game.step(Input::default());

        for fragment in &game.asteroids {
            let s = (fragment.vx * fragment.vx + fragment.vy * fragment.vy).sqrt();
            assert!(s > parent_drift, "a fragment ({s}) outruns its parent");
        }
    }

    #[test]
    fn a_rock_in_the_core_is_accreted_and_scored() {
        let mut game = empty_field(1);
        plant_rock(&mut game, AsteroidSize::Large, CENTER_X, CENTER_Y, 0.0);

        let events = game.step(Input::default());

        assert!(events.accreted, "the well devours the rock");
        assert_eq!(game.asteroid_count(), 0);
        assert_eq!(game.score(), AsteroidSize::Large.accrete_score());
    }

    #[test]
    fn a_steady_feed_ramps_the_score() {
        let mut game = empty_field(1);
        for _ in 0..8 {
            plant_rock(&mut game, AsteroidSize::Large, CENTER_X, CENTER_Y, 0.0);
        }
        game.step(Input::default());
        assert_eq!(game.feed_streak(), 8);
        assert!(
            game.score() > 8 * AsteroidSize::Large.accrete_score(),
            "a streak pays more than a flat rate ({})",
            game.score()
        );
    }

    #[test]
    fn the_ship_falls_into_the_core_and_dies() {
        let mut game = empty_field(1);
        game.invuln = 0.0;
        game.ship.x = CENTER_X;
        game.ship.y = CENTER_Y;

        let events = game.step(Input::default());

        assert!(events.ship_destroyed);
        assert_eq!(game.lives(), LIVES_START - 1);
        assert!(!game.ship_alive());
        assert!(game.blasts().count() >= 1);
    }

    #[test]
    fn a_rock_strikes_the_ship() {
        let mut game = empty_field(1);
        game.invuln = 0.0;
        let (sx, sy) = (game.ship.x, game.ship.y);
        plant_rock(&mut game, AsteroidSize::Large, sx, sy, 0.0);

        let events = game.step(Input::default());

        assert!(events.ship_destroyed);
        assert_eq!(game.lives(), LIVES_START - 1);
    }

    #[test]
    fn arrival_protection_shields_a_fresh_ship() {
        let mut game = empty_field(1);
        // The ship starts protected, so falling into the core does nothing...
        game.ship.x = CENTER_X;
        game.ship.y = CENTER_Y;
        assert!(!game.step(Input::default()).ship_destroyed);
        assert_eq!(game.lives(), LIVES_START);

        // ...but once it lapses, the same core destroys it.
        game.invuln = 0.0;
        game.ship.x = CENTER_X;
        game.ship.y = CENTER_Y;
        game.step(Input::default());
        assert!(!game.ship_alive());
    }

    #[test]
    fn a_downed_ship_returns_after_the_pause() {
        let mut game = empty_field(1);
        game.invuln = 0.0;
        game.ship.x = CENTER_X;
        game.ship.y = CENTER_Y;
        game.step(Input::default());
        assert!(!game.ship_alive());

        for _ in 0..(DEATH_PAUSE / TIMESTEP) as usize + 2 {
            game.step(Input::default());
        }
        assert!(game.ship_alive(), "the ship returns after the pause");
        assert!(game.ship_invulnerable(), "under fresh protection");
        let ship = game.ship();
        assert!(
            (ship.x - CENTER_X).abs() < 1e-3
                && (ship.y - (CENTER_Y - SHIP_START_OFFSET)).abs() < 1e-3
        );
    }

    #[test]
    fn spending_the_last_life_ends_the_run() {
        let mut game = empty_field(1);
        game.invuln = 0.0;
        game.lives = 1;
        game.ship.x = CENTER_X;
        game.ship.y = CENTER_Y;

        let mut over = false;
        for _ in 0..(2.0 / TIMESTEP) as usize {
            if game.step(Input::default()).game_over {
                over = true;
            }
        }
        assert!(over, "the last life ends the run");
        assert_eq!(game.phase(), Phase::Over);
        assert_eq!(game.lives(), 0);
    }

    /// Pins the ship, at rest, inside a well's skim band.
    fn park_in_band(game: &mut Game) {
        game.ship.x = CENTER_X;
        game.ship.y = CENTER_Y - (WELL_CORE_RADIUS + SKIM_BAND / 2.0);
        game.ship.vx = 0.0;
        game.ship.vy = 0.0;
    }

    #[test]
    fn skimming_a_well_charges_the_meter() {
        let mut game = empty_field(1);
        game.invuln = 0.0;
        park_in_band(&mut game);

        let events = game.step(Input::default());

        assert!(events.skimmed);
        assert!(game.collapse_meter() > 0.0);
    }

    #[test]
    fn a_skim_charges_once_per_pass() {
        let mut game = empty_field(1);
        game.invuln = 0.0;

        park_in_band(&mut game);
        game.step(Input::default());
        let after_first = game.collapse_meter();
        assert!(after_first > 0.0, "entering the band charges");

        park_in_band(&mut game);
        game.step(Input::default());
        assert_eq!(
            game.collapse_meter(),
            after_first,
            "staying in the band does not re-charge"
        );

        // Leave the band, then return — a fresh pass charges again.
        game.ship.x = CENTER_X;
        game.ship.y = 40.0;
        game.ship.vx = 0.0;
        game.ship.vy = 0.0;
        game.step(Input::default());
        park_in_band(&mut game);
        game.step(Input::default());
        assert!(
            game.collapse_meter() > after_first,
            "a fresh pass charges again"
        );
    }

    #[test]
    fn a_full_meter_fires_a_collapse_that_flings_rocks() {
        let mut game = empty_field(1);
        game.collapse_meter = 1.0;
        plant_rock(
            &mut game,
            AsteroidSize::Large,
            CENTER_X + 150.0,
            CENTER_Y,
            0.0,
        );
        let before = {
            let a = game.asteroids[0];
            (a.vx * a.vx + a.vy * a.vy).sqrt()
        };

        let events = game.step(Input {
            collapse: true,
            ..Default::default()
        });

        assert!(events.collapse_fired);
        assert_eq!(game.collapse_meter(), 0.0, "the meter is spent");
        let after = {
            let a = game.asteroids[0];
            (a.vx * a.vx + a.vy * a.vy).sqrt()
        };
        assert!(
            after > before + 200.0,
            "the collapse flings the rock outward"
        );
    }

    #[test]
    fn a_collapse_needs_a_full_meter() {
        let mut game = empty_field(1);
        game.collapse_meter = 0.5;

        let events = game.step(Input {
            collapse: true,
            ..Default::default()
        });

        assert!(!events.collapse_fired);
        assert_eq!(game.collapse_meter(), 0.5, "a partial meter cannot fire");
    }

    #[test]
    fn a_collapse_shatters_the_fine_debris() {
        let mut game = empty_field(1);
        game.collapse_meter = 1.0;
        plant_rock(
            &mut game,
            AsteroidSize::Small,
            CENTER_X + 150.0,
            CENTER_Y,
            0.0,
        );
        plant_rock(
            &mut game,
            AsteroidSize::Large,
            CENTER_X - 150.0,
            CENTER_Y,
            0.0,
        );

        game.step(Input {
            collapse: true,
            ..Default::default()
        });

        assert_eq!(
            game.asteroid_count(),
            1,
            "the small debris is caught, the large flung"
        );
        assert!(game.asteroids().all(|a| a.size == AsteroidSize::Large));
    }

    #[test]
    fn a_skim_charges_when_the_pass_turns_risky() {
        let mut game = empty_field(1);
        // In the band, but still protected — no charge yet.
        game.invuln = 1.0;
        park_in_band(&mut game);
        game.step(Input::default());
        assert_eq!(game.collapse_meter(), 0.0, "no charge while protected");

        // Protection lapses while still in the band: the fresh at-risk edge charges.
        game.invuln = 0.0;
        park_in_band(&mut game);
        game.step(Input::default());
        assert!(
            game.collapse_meter() > 0.0,
            "the pass charges once it turns risky"
        );
    }
}
