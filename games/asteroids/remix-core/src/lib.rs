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

/// What happened during a single [`Game::step`], for the shell to react to. Empty
/// for now — firing, accretion, the collapse and the rest fill it in the later
/// tickets; it exists from the start so the seam's shape is stable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {}

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

/// The whole run: the ship, the gravity wells that pull on it, and the seed the run
/// began on. Advanced only through [`Game::step`]; everything else is read-only.
pub struct Game {
    ship: ShipState,
    thrusting: bool,
    wells: Vec<WellState>,
    mode: Mode,
    /// The loadout the run was built with — inert now, but stored so a restart
    /// replays the very same run once the meta ticket gives it teeth.
    loadout: Loadout,
    phase: Phase,
    score: u32,
    steps: u64,
    seed: u64,
}

impl Game {
    /// Starts a run on `seed` in `mode`, flying `loadout`. The same seed and inputs
    /// always replay the same run. (The mode and loadout are inert until the later
    /// tickets give them teeth.)
    pub fn new(seed: u64, mode: Mode, loadout: Loadout) -> Self {
        Self {
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
            mode,
            loadout,
            phase: Phase::Playing,
            score: 0,
            steps: 0,
            seed,
        }
    }

    /// Advances the run one fixed timestep, returning what happened for the shell to
    /// react to.
    pub fn step(&mut self, input: Input) -> Events {
        self.steps += 1;
        let events = Events::default();
        if self.phase == Phase::Over {
            return events;
        }
        self.advance_ship(input);
        events
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
        let (gx, gy) = self.gravity_at(self.ship.x, self.ship.y);
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

    /// The gravitational acceleration on a body at `(x, y)` — the sum of every well's
    /// inverse-square pull, softened near a core, measured across the toroidal field
    /// so the pull is toward the nearest image of each well.
    fn gravity_at(&self, x: f32, y: f32) -> (f32, f32) {
        let mut ax = 0.0;
        let mut ay = 0.0;
        for well in &self.wells {
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
}
