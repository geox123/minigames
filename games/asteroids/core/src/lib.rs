//! The pure, deterministic core of **Asteroids** — the Collection's faithful
//! recreation of Atari's 1979 vector arcade original, drawn (in the shell) as
//! programmatic vector polygons per
//! [ADR 0003](../../../docs/adr/0003-code-drawn-visuals.md) and shipped under its
//! own real name (Atari, arcade-era, low IP risk — the same posture as Pong and
//! Breakout).
//!
//! Like the Collection's other cores it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and advances in fixed timesteps
//! so a seed and a sequence of inputs always replay the same game.
//!
//! It plays out on the original's **1024×768 landscape vector field** — the
//! coordinate space the original's math ran in, not a raster pixel grid — which is
//! **toroidal**: the ship, the rocks and (later) shots all wrap around every edge.
//!
//! This is the first slice ([ticket T1](https://github.com/geox123/minigames/issues/111)):
//! the field, the Newtonian ship, and the large asteroids drifting across it. The
//! ship rotates in place, **thrusts** along its facing, then *coasts* — momentum
//! carries it, a gentle friction bleeds speed off, and a top speed caps it. Firing,
//! splitting, collisions, saucers, hyperspace, waves and scoring arrive in the
//! later tickets; everything hangs off the single [`Game::step`] seam established
//! here.

use core::f32::consts::TAU;

/// Width of the landscape play field, in logical units — the original's vector
/// coordinate space.
pub const LOGICAL_WIDTH: f32 = 1024.0;
/// Height of the landscape play field, in logical units — the original's vector
/// coordinate space.
pub const LOGICAL_HEIGHT: f32 = 768.0;

/// Length of a single simulation step, in seconds. The Collection's cores all run
/// at 120 Hz; Asteroids' motion is continuous, so there is no machine interrupt to
/// derive (as STEPFALL needed) — thrust, friction and drift simply integrate per
/// step.
pub const TIMESTEP: f32 = 1.0 / 120.0;

/// The ship's collision radius, in logical units — its rough half-extent, used for
/// collision (in later tickets) and as the scale the shell draws it at.
pub const SHIP_RADIUS: f32 = 14.0;

/// How fast the ship turns, in radians per second. A **feel constant**: the motion
/// model is fixed, but this and the three below are tuned against the running shell
/// in [ticket T2](https://github.com/geox123/minigames/issues/112).
const SHIP_TURN_RATE: f32 = 4.0;
/// The acceleration thrust adds along the ship's facing, in units per second².
const SHIP_THRUST: f32 = 350.0;
/// The gentle space-friction that bleeds the ship's speed off, per second — so the
/// ship coasts but eventually slows. Set below `SHIP_THRUST / SHIP_MAX_SPEED` so the
/// cap below genuinely binds under sustained thrust.
const SHIP_FRICTION: f32 = 0.5;
/// The top speed the ship may reach, in units per second.
const SHIP_MAX_SPEED: f32 = 480.0;

/// How many large rocks a fresh field opens with. (Waves add more from
/// [ticket T6](https://github.com/geox123/minigames/issues/116); here it is just
/// the opening field.)
pub const INITIAL_ASTEROIDS: usize = 4;

/// The slowest and fastest a large rock drifts, in units per second — its speed is
/// drawn from this range when the field is seeded.
const ASTEROID_MIN_SPEED: f32 = 30.0;
const ASTEROID_MAX_SPEED: f32 = 70.0;

/// No rock spawns within this distance of the field's centre, so a fresh field
/// never materialises a rock on top of the ship waiting in the middle.
const SAFE_SPAWN_RADIUS: f32 = 220.0;

/// The centre of the field, where the ship begins and returns to.
const CENTER_X: f32 = LOGICAL_WIDTH / 2.0;
const CENTER_Y: f32 = LOGICAL_HEIGHT / 2.0;

/// The three sizes a rock comes in. A shot splits a large into mediums and a medium
/// into smalls (from [ticket T3](https://github.com/geox123/minigames/issues/113));
/// here they carry only their drawn size and collision radius.
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
            AsteroidSize::Large => 60.0,
            AsteroidSize::Medium => 30.0,
            AsteroidSize::Small => 15.0,
        }
    }
}

/// What the player is doing this step. `fire` and `hyperspace` are wired now so the
/// input shape stays stable across tickets, but the core ignores them until the
/// tickets that add firing and the teleport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// Rotate anticlockwise (to the ship's left).
    pub turn_left: bool,
    /// Rotate clockwise (to the ship's right).
    pub turn_right: bool,
    /// Thrust along the ship's facing.
    pub thrust: bool,
    /// Fire a shot. *(Ignored until firing lands.)*
    pub fire: bool,
    /// Jump to hyperspace. *(Ignored until hyperspace lands.)*
    pub hyperspace: bool,
}

/// The player's ship, as the shell should draw it. `x`/`y` are its centre in the
/// logical field; `angle` is its facing in radians, `0` pointing straight up and
/// increasing clockwise (so `TAU/4` faces right). `vx`/`vy` are its current
/// velocity — exposed because the shell may draw motion from it and because later
/// tickets hand a wrecked ship's momentum on.
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

/// A rock adrift, as the shell should draw it. `x`/`y` are its centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Asteroid {
    pub x: f32,
    pub y: f32,
    pub size: AsteroidSize,
}

/// What happened during a single [`Game::step`], for the shell to react to. Empty
/// for now — firing, kills, deaths, scoring and waves fill it in the later tickets;
/// it exists from the start so the seam's shape is stable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {}

/// Where a game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The game is being played.
    Playing,
    /// Every ship has been spent. *(Reachable once dying lands.)*
    GameOver,
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

/// A rock's live state: its centre, its constant drift velocity, and its size.
#[derive(Clone, Copy)]
struct AsteroidState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: AsteroidSize,
}

/// The whole game: the ship, the rocks, and the seeded RNG that laid them out.
/// Advanced only through [`Game::step`]; everything else is read-only.
pub struct Game {
    ship: ShipState,
    /// Whether the ship thrusted on the most recent step — surfaced for the flame.
    thrusting: bool,
    asteroids: Vec<AsteroidState>,
    rng: Rng,
    phase: Phase,
    /// Steps taken so far.
    steps: u64,
    /// The seed the game began on, so a restart replays it exactly.
    seed: u64,
}

impl Game {
    /// Starts a new game. The same seed always produces the same game: the ship
    /// waits at the centre facing up, and a field of [`INITIAL_ASTEROIDS`] large
    /// rocks is laid out around it, each with a seeded heading and drift.
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            ship: ShipState {
                x: CENTER_X,
                y: CENTER_Y,
                vx: 0.0,
                vy: 0.0,
                angle: 0.0,
            },
            thrusting: false,
            asteroids: Vec::with_capacity(INITIAL_ASTEROIDS),
            rng: Rng::new(seed),
            phase: Phase::Playing,
            steps: 0,
            seed,
        };
        game.spawn_field(INITIAL_ASTEROIDS);
        game
    }

    /// Lays out `count` large rocks, none within [`SAFE_SPAWN_RADIUS`] of the
    /// centre, each drifting at a seeded heading and speed.
    fn spawn_field(&mut self, count: usize) {
        for _ in 0..count {
            let (x, y) = loop {
                let x = self.rng.range(0.0, LOGICAL_WIDTH);
                let y = self.rng.range(0.0, LOGICAL_HEIGHT);
                let dx = x - CENTER_X;
                let dy = y - CENTER_Y;
                if dx * dx + dy * dy >= SAFE_SPAWN_RADIUS * SAFE_SPAWN_RADIUS {
                    break (x, y);
                }
            };
            let heading = self.rng.range(0.0, TAU);
            let speed = self.rng.range(ASTEROID_MIN_SPEED, ASTEROID_MAX_SPEED);
            self.asteroids.push(AsteroidState {
                x,
                y,
                vx: heading.cos() * speed,
                vy: heading.sin() * speed,
                size: AsteroidSize::Large,
            });
        }
    }

    /// Advances the game one fixed timestep, returning what happened for the shell
    /// to react to.
    pub fn step(&mut self, input: Input) -> Events {
        self.steps += 1;
        let events = Events::default();
        if self.phase == Phase::GameOver {
            return events;
        }
        self.advance_ship(input);
        self.advance_asteroids();
        events
    }

    /// Turns, thrusts, coasts and moves the ship, wrapping it at the field edges.
    fn advance_ship(&mut self, input: Input) {
        // Turning: opposing presses cancel.
        if input.turn_left && !input.turn_right {
            self.ship.angle -= SHIP_TURN_RATE * TIMESTEP;
        }
        if input.turn_right && !input.turn_left {
            self.ship.angle += SHIP_TURN_RATE * TIMESTEP;
        }
        self.ship.angle = self.ship.angle.rem_euclid(TAU);

        // Thrust adds acceleration along the facing (angle 0 points up).
        self.thrusting = input.thrust;
        if input.thrust {
            let fx = self.ship.angle.sin();
            let fy = -self.ship.angle.cos();
            self.ship.vx += SHIP_THRUST * fx * TIMESTEP;
            self.ship.vy += SHIP_THRUST * fy * TIMESTEP;
        }

        // Space-friction bleeds speed off; then a hard cap holds the top speed.
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

    /// Drifts every rock in a straight line, wrapping it at the field edges.
    fn advance_asteroids(&mut self) {
        for a in &mut self.asteroids {
            a.x = wrap(a.x + a.vx * TIMESTEP, LOGICAL_WIDTH);
            a.y = wrap(a.y + a.vy * TIMESTEP, LOGICAL_HEIGHT);
        }
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

    /// The rocks adrift, as the shell should draw them.
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

    /// Where the game is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Starts the game over from the beginning; the same seed replays it exactly.
    pub fn restart(&mut self) {
        *self = Self::new(self.seed);
    }
}

/// Wraps a coordinate into `[0, max)` — the field's toroidal topology.
fn wrap(v: f32, max: f32) -> f32 {
    v.rem_euclid(max)
}

/// The Collection's small deterministic PRNG: splitmix64 to spread the seed, then
/// xorshift for the stream. Shared in spirit with the other cores' `Rng`; kept
/// local so the core has no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Mix the seed through splitmix64 so even adjacent seeds produce
        // well-separated states.
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        // xorshift's zero state is a fixed point; keep the state non-zero.
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
