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
//! **toroidal**: the ship, the rocks and the shots all wrap around every edge, and
//! so does collision.
//!
//! # What is here so far
//!
//! The ship flies Newtonian ([T1](https://github.com/geox123/minigames/issues/111)) —
//! it rotates in place, thrusts along its facing, then coasts under a gentle
//! friction with a capped top speed. It **fires**
//! ([T3](https://github.com/geox123/minigames/issues/113)): at most four shots at
//! once, each a fixed world speed along the facing (the ship's own velocity is *not*
//! added, so a fast ship can outrun its own fire) that expires after a fixed range.
//! Shots **split** rocks — a large into two mediums, a medium into two smalls, a
//! small into nothing, the fragments faster than their parent — scoring 20 / 50 / 100
//! by size. Colliding with a rock **destroys the ship**: a life is lost, an
//! explosion plays, and the ship reappears in the centre once the centre is clear;
//! the game ends when the last ship is gone. Saucers, hyperspace, waves and the
//! bonus ship arrive in the later tickets; everything hangs off the single
//! [`Game::step`] seam.

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
/// collision and as the scale the shell draws it at.
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

/// The player's ships to start, the pause the game holds after one is destroyed,
/// how long a fresh ship is protected on arrival, and how clear the centre must be
/// before it may arrive.
pub const LIVES_START: u32 = 3;
const DEATH_PAUSE: f32 = 1.2;
const SPAWN_INVULN: f32 = 2.0;
const RESPAWN_CLEAR_RADIUS: f32 = 180.0;

/// The player's shots: how fast one flies (a fixed *world* speed — the ship's
/// velocity is deliberately not added), how long it lives before expiring, how many
/// may be in flight at once, and its collision radius.
const SHOT_SPEED: f32 = 450.0;
const SHOT_LIFE: f32 = 1.1;
const MAX_SHOTS: usize = 4;
const SHOT_RADIUS: f32 = 2.0;

/// A split rock's fragments fly off at the parent's speed times a factor in this
/// range — always above 1, so the field speeds up as it breaks.
const FRAGMENT_SPEED_MIN: f32 = 1.1;
const FRAGMENT_SPEED_MAX: f32 = 1.6;

/// How long an explosion lingers for the shell to draw, in seconds.
const BLAST_LIFE: f32 = 0.4;

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
            AsteroidSize::Large => 60.0,
            AsteroidSize::Medium => 30.0,
            AsteroidSize::Small => 15.0,
        }
    }

    /// What shooting the rock scores — the smaller and faster, the more it is worth,
    /// as the original rewarded.
    fn score(self) -> u32 {
        match self {
            AsteroidSize::Large => 20,
            AsteroidSize::Medium => 50,
            AsteroidSize::Small => 100,
        }
    }

    /// The size the rock breaks into — two of these — or `None` for a small, which
    /// is destroyed outright.
    fn child(self) -> Option<AsteroidSize> {
        match self {
            AsteroidSize::Large => Some(AsteroidSize::Medium),
            AsteroidSize::Medium => Some(AsteroidSize::Small),
            AsteroidSize::Small => None,
        }
    }
}

/// What the player is doing this step. `hyperspace` is wired now so the input shape
/// stays stable across tickets, but the core ignores it until the teleport lands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// Rotate anticlockwise (to the ship's left).
    pub turn_left: bool,
    /// Rotate clockwise (to the ship's right).
    pub turn_right: bool,
    /// Thrust along the ship's facing.
    pub thrust: bool,
    /// Fire a shot. A shot is loosed on the press, not while the button is held, so
    /// firing is a matter of tapping — one shot per press, up to the on-screen cap.
    pub fire: bool,
    /// Jump to hyperspace. *(Ignored until hyperspace lands.)*
    pub hyperspace: bool,
}

/// The player's ship, as the shell should draw it. `x`/`y` are its centre in the
/// logical field; `angle` is its facing in radians, `0` pointing straight up and
/// increasing clockwise (so `TAU/4` faces right). `vx`/`vy` are its current
/// velocity — exposed because the shell may draw motion from it and because later
/// tickets hand a wrecked ship's momentum on. The shell should only draw the ship
/// while [`Game::ship_alive`] is true.
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

/// One of the player's shots in flight, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub x: f32,
    pub y: f32,
}

/// An explosion in progress, for the shell to draw for a few frames where a rock or
/// the ship was destroyed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    pub x: f32,
    pub y: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to (its
/// sounds and juice). The authoritative score, lives and phase are read from the
/// game's accessors; these are the one-step cues.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The ship fired a shot this step.
    pub fired: bool,
    /// A shot destroyed a rock this step.
    pub rock_destroyed: bool,
    /// The ship was destroyed this step and a life was lost.
    pub ship_destroyed: bool,
    /// The last ship was spent this step — the game is over.
    pub game_over: bool,
}

/// Where a game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The game is being played.
    Playing,
    /// Every ship has been spent.
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

/// A shot's live state: its centre, its constant velocity, and the seconds of life
/// it has left before it expires.
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

/// The whole game: the ship, the rocks, the shots and explosions, the score and
/// lives, and the seeded RNG that lays out the field and scatters the fragments.
/// Advanced only through [`Game::step`]; everything else is read-only.
pub struct Game {
    ship: ShipState,
    /// Whether the ship thrusted on the most recent step — surfaced for the flame.
    thrusting: bool,
    /// Whether the ship is on the field. It is off between destruction and the
    /// centre clearing for its return.
    ship_alive: bool,
    /// Seconds left in the pause after a destruction, before the ship tries to
    /// return.
    dead_timer: f32,
    /// Seconds of arrival protection left; while it lasts the ship cannot be hit.
    invuln: f32,
    /// Whether the destruction that is playing out was the last life — the game
    /// ends rather than the ship returning.
    ending: bool,
    /// Whether the fire button was down last step, so firing triggers on the press.
    fire_was_down: bool,
    asteroids: Vec<AsteroidState>,
    shots: Vec<ShotState>,
    blasts: Vec<BlastState>,
    score: u32,
    lives: u32,
    rng: Rng,
    phase: Phase,
    /// Steps taken so far.
    steps: u64,
    /// The seed the game began on, so a restart replays it exactly.
    seed: u64,
}

impl Game {
    /// Starts a new game. The same seed always produces the same game: the ship
    /// waits at the centre facing up under brief arrival protection, and a field of
    /// [`INITIAL_ASTEROIDS`] large rocks is laid out around it, each with a seeded
    /// heading and drift.
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
            ship_alive: true,
            dead_timer: 0.0,
            invuln: SPAWN_INVULN,
            ending: false,
            fire_was_down: false,
            asteroids: Vec::with_capacity(INITIAL_ASTEROIDS),
            shots: Vec::new(),
            blasts: Vec::new(),
            score: 0,
            lives: LIVES_START,
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
        let mut events = Events::default();
        if self.phase == Phase::GameOver {
            return events;
        }

        // Firing triggers on a fresh press, so tapping fires one shot at a time.
        let fire_pressed = input.fire && !self.fire_was_down;
        self.fire_was_down = input.fire;

        if self.ship_alive {
            self.advance_ship(input);
            if fire_pressed && self.shots.len() < MAX_SHOTS {
                self.fire();
                events.fired = true;
            }
        } else {
            self.advance_death(&mut events);
        }

        self.advance_shots();
        self.advance_asteroids();
        self.advance_blasts();
        if self.invuln > 0.0 {
            self.invuln = (self.invuln - TIMESTEP).max(0.0);
        }

        self.resolve_shot_hits(&mut events);
        if self.ship_alive && self.invuln <= 0.0 {
            self.resolve_ship_hits(&mut events);
        }

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
            let (fx, fy) = facing(self.ship.angle);
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

    /// Loosens a shot from the ship's nose, at a fixed world speed along the facing.
    /// The ship's own velocity is *not* added — the original's quirk, so a ship at
    /// full tilt can overtake and fly into its own fire.
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

    /// Runs the pause after a destruction: hold for a beat, then either end the game
    /// (if that was the last life) or bring the ship back once the centre is clear.
    fn advance_death(&mut self, events: &mut Events) {
        if self.dead_timer > 0.0 {
            self.dead_timer -= TIMESTEP;
            return;
        }
        if self.ending {
            self.phase = Phase::GameOver;
            events.game_over = true;
            return;
        }
        if self.center_clear() {
            self.ship = ShipState {
                x: CENTER_X,
                y: CENTER_Y,
                vx: 0.0,
                vy: 0.0,
                angle: 0.0,
            };
            self.ship_alive = true;
            self.invuln = SPAWN_INVULN;
        }
    }

    /// Whether the centre is clear enough of rocks for the ship to return.
    fn center_clear(&self) -> bool {
        self.asteroids.iter().all(|a| {
            let dx = a.x - CENTER_X;
            let dy = a.y - CENTER_Y;
            let clear = RESPAWN_CLEAR_RADIUS + a.size.radius();
            dx * dx + dy * dy >= clear * clear
        })
    }

    /// Drifts every rock in a straight line, wrapping it at the field edges.
    fn advance_asteroids(&mut self) {
        for a in &mut self.asteroids {
            a.x = wrap(a.x + a.vx * TIMESTEP, LOGICAL_WIDTH);
            a.y = wrap(a.y + a.vy * TIMESTEP, LOGICAL_HEIGHT);
        }
    }

    /// Flies every shot, wrapping it, and drops the ones whose range has run out.
    fn advance_shots(&mut self) {
        for s in &mut self.shots {
            s.x = wrap(s.x + s.vx * TIMESTEP, LOGICAL_WIDTH);
            s.y = wrap(s.y + s.vy * TIMESTEP, LOGICAL_HEIGHT);
            s.life -= TIMESTEP;
        }
        self.shots.retain(|s| s.life > 0.0);
    }

    /// Burns down the explosions and drops the ones that have finished.
    fn advance_blasts(&mut self) {
        for b in &mut self.blasts {
            b.timer -= TIMESTEP;
        }
        self.blasts.retain(|b| b.timer > 0.0);
    }

    /// Resolves shots striking rocks: the rock is scored and destroyed, its
    /// fragments (if any) scattered faster than it drifted, and the shot spent.
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
                self.score += rock.size.score();
                events.rock_destroyed = true;
                self.blasts.push(BlastState {
                    x: rock.x,
                    y: rock.y,
                    timer: BLAST_LIFE,
                });
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
                // The shot is spent; `swap_remove` drops in the last shot here, so
                // don't advance `i` — that new occupant is checked next pass.
                self.shots.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.asteroids.append(&mut fragments);
    }

    /// Resolves the ship striking a rock: a life is lost, an explosion plays, and
    /// the ship leaves the field to return from the centre — or, if that was the
    /// last life, the game begins to end.
    fn resolve_ship_hits(&mut self, events: &mut Events) {
        let struck = self.asteroids.iter().any(|a| {
            overlap(
                (self.ship.x, self.ship.y, SHIP_RADIUS),
                (a.x, a.y, a.size.radius()),
            )
        });
        if !struck {
            return;
        }
        self.lives = self.lives.saturating_sub(1);
        self.ship_alive = false;
        self.thrusting = false;
        self.dead_timer = DEATH_PAUSE;
        self.ending = self.lives == 0;
        self.shots.clear();
        self.blasts.push(BlastState {
            x: self.ship.x,
            y: self.ship.y,
            timer: BLAST_LIFE,
        });
        events.ship_destroyed = true;
    }

    /// The ship, as the shell should draw it (only while [`Game::ship_alive`]).
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

    /// Whether the ship is on the field this step. It is off during the pause
    /// between destruction and its return.
    pub fn ship_alive(&self) -> bool {
        self.ship_alive
    }

    /// Whether the ship is under arrival protection this step — the shell may blink
    /// it to show it cannot yet be hit.
    pub fn ship_invulnerable(&self) -> bool {
        self.invuln > 0.0
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

    /// The shots in flight, as the shell should draw them.
    pub fn shots(&self) -> impl Iterator<Item = Shot> + '_ {
        self.shots.iter().map(|s| Shot { x: s.x, y: s.y })
    }

    /// The explosions in progress, as the shell should draw them.
    pub fn blasts(&self) -> impl Iterator<Item = Blast> + '_ {
        self.blasts.iter().map(|b| Blast { x: b.x, y: b.y })
    }

    /// The running score.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// The ships left, the one in play included.
    pub fn lives(&self) -> u32 {
        self.lives
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

/// The unit facing vector for `angle` (angle 0 points up, increasing clockwise).
fn facing(angle: f32) -> (f32, f32) {
    (angle.sin(), -angle.cos())
}

/// Wraps a coordinate into `[0, max)` — the field's toroidal topology.
fn wrap(v: f32, max: f32) -> f32 {
    v.rem_euclid(max)
}

/// Whether two circles — each a `(centre-x, centre-y, radius)` — overlap, measured
/// across the toroidal field so a pair straddling opposite edges (and touching
/// through the wrap) still counts.
fn overlap(a: (f32, f32, f32), b: (f32, f32, f32)) -> bool {
    let dx = ring_delta(a.0, b.0, LOGICAL_WIDTH);
    let dy = ring_delta(a.1, b.1, LOGICAL_HEIGHT);
    let r = a.2 + b.2;
    dx * dx + dy * dy < r * r
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

#[cfg(test)]
mod tests {
    //! Firing outcomes, splitting, and the ship's death and return. Aiming a shot at
    //! a chosen rock — or steering the ship onto one — is impractical for a scripted
    //! player, so these plant the exact rock (or a rock on the ship) and then drive
    //! the real [`Game::step`] path over it: only the setup reaches inside, the
    //! transition runs through the same code play does. Firing cadence, the shot's
    //! fixed speed and determinism are reachable by honest play and tested in
    //! `tests/`.
    use super::*;

    /// A game with the field cleared, so a test can plant exactly what it needs.
    fn empty_game() -> Game {
        let mut game = Game::new(1);
        game.asteroids.clear();
        game
    }

    /// Plants one rock of `size` at `(x, y)` drifting at `speed` to the right.
    fn plant_rock(game: &mut Game, size: AsteroidSize, x: f32, y: f32, speed: f32) {
        game.asteroids.push(AsteroidState {
            x,
            y,
            vx: speed,
            vy: 0.0,
            size,
        });
    }

    /// Plants a shot sitting right on `(x, y)`, so the next step's collision pass
    /// finds it on whatever rock is there.
    fn plant_shot(game: &mut Game, x: f32, y: f32) {
        game.shots.push(ShotState {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            life: SHOT_LIFE,
        });
    }

    fn speed_of(a: &AsteroidState) -> f32 {
        (a.vx * a.vx + a.vy * a.vy).sqrt()
    }

    #[test]
    fn a_shot_expires_after_its_range() {
        // On a cleared field the shot can only vanish by outliving its range.
        let mut game = empty_game();
        game.step(Input {
            fire: true,
            ..Default::default()
        });
        assert_eq!(game.shots().count(), 1, "a shot is loosed");

        for _ in 0..(SHOT_LIFE / TIMESTEP) as usize + 5 {
            game.step(Input::default());
        }
        assert_eq!(game.shots().count(), 0, "and it expires after its range");
    }

    #[test]
    fn a_ship_at_full_tilt_outruns_its_own_shot() {
        // The shot's speed is fixed and below the ship's cap, so a ship racing after
        // it is faster — it can overtake and fly into its own fire.
        let mut game = empty_game();
        let thrust = Input {
            thrust: true,
            ..Default::default()
        };
        for _ in 0..400 {
            game.step(thrust); // build up to the top speed
        }
        let ship = game.ship();
        let ship_speed = (ship.vx * ship.vx + ship.vy * ship.vy).sqrt();

        game.step(Input {
            thrust: true,
            fire: true,
            ..Default::default()
        });
        let before = game.shots[0];
        game.step(thrust);
        let after = game.shots[0];
        let dx = ring_delta(after.x, before.x, LOGICAL_WIDTH);
        let dy = ring_delta(after.y, before.y, LOGICAL_HEIGHT);
        let shot_speed = (dx * dx + dy * dy).sqrt() / TIMESTEP;

        assert!(
            ship_speed > shot_speed,
            "a full-tilt ship ({ship_speed}) outruns its shot ({shot_speed})"
        );
    }

    #[test]
    fn a_large_rock_splits_into_two_mediums_and_scores_twenty() {
        let mut game = empty_game();
        plant_rock(&mut game, AsteroidSize::Large, 300.0, 300.0, 25.0);
        plant_shot(&mut game, 300.0, 300.0);

        let events = game.step(Input::default());

        assert!(events.rock_destroyed);
        assert_eq!(game.score(), 20);
        assert_eq!(game.asteroid_count(), 2);
        assert!(game.asteroids().all(|a| a.size == AsteroidSize::Medium));
    }

    #[test]
    fn a_medium_rock_splits_into_two_smalls_and_scores_fifty() {
        let mut game = empty_game();
        plant_rock(&mut game, AsteroidSize::Medium, 300.0, 300.0, 25.0);
        plant_shot(&mut game, 300.0, 300.0);

        game.step(Input::default());

        assert_eq!(game.score(), 50);
        assert_eq!(game.asteroid_count(), 2);
        assert!(game.asteroids().all(|a| a.size == AsteroidSize::Small));
    }

    #[test]
    fn a_small_rock_is_destroyed_outright_and_scores_a_hundred() {
        let mut game = empty_game();
        plant_rock(&mut game, AsteroidSize::Small, 300.0, 300.0, 25.0);
        plant_shot(&mut game, 300.0, 300.0);

        game.step(Input::default());

        assert_eq!(game.score(), 100);
        assert_eq!(game.asteroid_count(), 0);
    }

    #[test]
    fn fragments_fly_faster_than_their_parent() {
        let mut game = empty_game();
        let parent_speed = 40.0;
        plant_rock(&mut game, AsteroidSize::Large, 300.0, 300.0, parent_speed);
        plant_shot(&mut game, 300.0, 300.0);

        game.step(Input::default());

        for fragment in &game.asteroids {
            assert!(
                speed_of(fragment) > parent_speed,
                "a fragment ({}) should outrun its parent ({parent_speed})",
                speed_of(fragment)
            );
        }
    }

    #[test]
    fn a_rock_striking_the_ship_costs_a_life_and_clears_it_off() {
        let mut game = empty_game();
        game.invuln = 0.0; // past arrival protection
        plant_rock(&mut game, AsteroidSize::Large, CENTER_X, CENTER_Y, 0.0);

        let events = game.step(Input::default());

        assert!(events.ship_destroyed);
        assert_eq!(game.lives(), LIVES_START - 1);
        assert!(!game.ship_alive());
        assert!(game.blasts().count() >= 1, "an explosion plays");
    }

    #[test]
    fn arrival_protection_shields_a_fresh_ship() {
        let mut game = empty_game();
        // The ship starts under protection, so a rock on it does nothing...
        plant_rock(&mut game, AsteroidSize::Large, CENTER_X, CENTER_Y, 0.0);
        let events = game.step(Input::default());
        assert!(!events.ship_destroyed);
        assert_eq!(game.lives(), LIVES_START);

        // ...but once it lapses, the same rock destroys it.
        game.invuln = 0.0;
        game.step(Input::default());
        assert!(!game.ship_alive());
        assert_eq!(game.lives(), LIVES_START - 1);
    }

    #[test]
    fn a_downed_ship_returns_only_once_the_centre_is_clear() {
        let mut game = empty_game();
        game.invuln = 0.0;
        // A rock destroys the ship, then parks squarely on the centre.
        plant_rock(&mut game, AsteroidSize::Large, CENTER_X, CENTER_Y, 0.0);
        game.step(Input::default());
        assert!(!game.ship_alive());

        // Run well past the death pause: the ship cannot return onto the rock.
        for _ in 0..(2.0 / TIMESTEP) as usize {
            game.step(Input::default());
        }
        assert!(!game.ship_alive(), "it will not return onto a rock");

        // Clear the centre; now it comes back, under fresh protection.
        game.asteroids.clear();
        for _ in 0..(DEATH_PAUSE / TIMESTEP) as usize + 2 {
            game.step(Input::default());
        }
        assert!(game.ship_alive(), "a clear centre lets it return");
        assert!(game.ship_invulnerable(), "and it arrives protected");
        let ship = game.ship();
        assert!((ship.x - CENTER_X).abs() < 1e-3 && (ship.y - CENTER_Y).abs() < 1e-3);
    }

    #[test]
    fn spending_the_last_life_ends_the_game() {
        let mut game = empty_game();
        game.invuln = 0.0;
        game.lives = 1;
        plant_rock(&mut game, AsteroidSize::Large, CENTER_X, CENTER_Y, 0.0);

        let mut over = false;
        for _ in 0..(2.0 / TIMESTEP) as usize {
            if game.step(Input::default()).game_over {
                over = true;
            }
        }
        assert!(over, "the last life ends the game");
        assert_eq!(game.phase(), Phase::GameOver);
        assert_eq!(game.lives(), 0);
    }
}
