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
//! This is the first slice ([ticket A1](https://github.com/geox123/minigames/issues/130)):
//! the gravity field and the ship that flies it. Firing, rocks, accretion, the
//! collapse, enemies, bosses, the modes and the meta arrive in the later tickets;
//! the ship's **loadout** is handed *in* at construction, so the core never knows the
//! word "unlock" — it only ever flies what it is given.

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

/// What happened during a single [`Game::step`], for the shell to react to. It grows
/// as accretion, the collapse and the rest arrive in the later tickets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The ship fired a shot this step.
    pub fired: bool,
    /// A shot split a rock this step.
    pub rock_split: bool,
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

/// The whole run: the ship, the gravity wells that pull on it, and the seed the run
/// began on. Advanced only through [`Game::step`]; everything else is read-only.
pub struct Game {
    ship: ShipState,
    thrusting: bool,
    wells: Vec<WellState>,
    asteroids: Vec<AsteroidState>,
    shots: Vec<ShotState>,
    /// Steps until the held stream may loose its next shot.
    fire_cooldown: u64,
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
            fire_cooldown: 0,
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
        self.advance_ship(input);
        if input.fire && self.fire_cooldown == 0 {
            self.fire();
            events.fired = true;
            self.fire_cooldown = FIRE_INTERVAL;
        }
        self.fire_cooldown = self.fire_cooldown.saturating_sub(1);
        self.advance_shots();
        self.advance_asteroids();
        self.resolve_shot_hits(&mut events);
        events
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

/// Whether two circles — each a `(centre-x, centre-y, radius)` — overlap, measured
/// across the toroidal field.
fn overlap(a: (f32, f32, f32), b: (f32, f32, f32)) -> bool {
    let dx = ring_delta(a.0, b.0, LOGICAL_WIDTH);
    let dy = ring_delta(a.1, b.1, LOGICAL_HEIGHT);
    let r = a.2 + b.2;
    dx * dx + dy * dy < r * r
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
}
