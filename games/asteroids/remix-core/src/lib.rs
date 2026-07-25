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
//! rocks that split and curve under the pull (A2); the well **accreting** rocks for a
//! streak-fed score, with a ship that falls into a core — or is struck by a rock —
//! destroyed (A3); the signature quartet — slingshot, skim and collapse (A4); and now
//! an **orbital enemy zoo** that rides the same gravity — Orbiters that settle into an
//! orbit and fire, Divers that fall in on a bent path, Mines that drift inert until
//! the ship nears, and Shepherds that herd rocks at the player — arriving in waves
//! that rotate through the kinds and escalate (A5). Bosses, the modes and the meta
//! arrive in the later tickets; the ship's **loadout** is handed *in* at construction,
//! so the core never knows the word "unlock" — it only ever flies what it is given.

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

/// The orbital enemy zoo. Every enemy rides the same gravity as the rest of the
/// field; a kind's *behaviour* is what it does on top of that pull. Its collision
/// radius — its hull, for shots, the ship, and the well core.
pub const ENEMY_RADIUS: f32 = 16.0;

/// The radius an **Orbiter** (and, a little wider, a **Shepherd**) is placed at
/// around its anchor well; a near-circular orbit's speed falls out of the gravity
/// model (see [`orbital_speed`]), so an enemy dropped there truly orbits the pull.
const ORBITER_RADIUS: f32 = 250.0;
const SHEPHERD_RADIUS: f32 = 320.0;
/// A Shepherd holds a slower, looser orbit than an Orbiter, so it drifts about the
/// field herding rather than settling into a tight ring.
const SHEPHERD_ORBIT_FRACTION: f32 = 0.85;

/// A **Diver** enters at a field edge and crosses on a gravity-bent path, aimed just
/// off the well so it slings by; it leaves (despawns) after this long if it survives,
/// and flies at this speed before the pull bends it.
const DIVER_LIFE: f32 = 8.0;
const DIVER_SPEED: f32 = 240.0;
/// How far off dead-centre a Diver aims, in radians — enough to sling by, not dive in.
const DIVER_AIM_OFFSET: f32 = 0.35;

/// A **Mine** drifts inert until the ship comes within this, then wakes and thrusts
/// straight at it; asleep it only creeps, at this drift speed.
const MINE_WAKE_RADIUS: f32 = 170.0;
const MINE_THRUST: f32 = 240.0;
const MINE_DRIFT_SPEED: f32 = 20.0;
/// The band of distances from a well a Mine is scattered into — clear of the core,
/// short of the edge.
const MINE_SPAWN_MIN: f32 = 180.0;
const MINE_SPAWN_MAX: f32 = 340.0;

/// Enemy fire: how fast a pellet flies before the gravity bends it (it curves like
/// everything else), how fast each wave of escalation adds to that, how long a shot
/// lives, and its radius.
const ENEMY_SHOT_SPEED: f32 = 300.0;
const ENEMY_SHOT_SPEED_PER_WAVE: f32 = 10.0;
const ENEMY_SHOT_LIFE: f32 = 3.0;
const ENEMY_SHOT_RADIUS: f32 = 3.0;

/// A Shepherd's herd: the velocity impulse it lends the nearest rock, aimed at the
/// ship, each time its cadence comes round.
const SHEPHERD_NUDGE: f32 = 60.0;

/// Waves. The field of enemies must be cleared before the next wave; then a gap, then
/// a fresh wave rotated through the zoo. The first wave's size and the cap it grows
/// to, how many waves in the escalation plateaus, and how many steps of cadence one
/// wave of escalation shaves off.
const ENEMY_WAVE_GAP: f32 = 2.5;
const WAVE_BASE_COUNT: usize = 2;
const WAVE_COUNT_CAP: usize = 6;
const ESCALATION_CAP: u32 = 8;
const CADENCE_PER_WAVE: u32 = 12;
/// The enemy kinds a wave draws from, in rotation order.
const ENEMY_ZOO: [EnemyKind; 4] = [
    EnemyKind::Orbiter,
    EnemyKind::Diver,
    EnemyKind::Mine,
    EnemyKind::Shepherd,
];

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

/// The kind of an enemy — its entry, how it rides the gravity, and how it threatens.
/// This is the pattern zoo: each kind reads and menaces differently. Exposed so the
/// shell can draw a distinct silhouette per kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    /// Settles into an orbit of a well and fires aimed shots at the ship.
    Orbiter,
    /// Enters at a field edge and falls across on a gravity-bent path, firing ahead.
    Diver,
    /// Drifts inert in the pull until the ship nears, then wakes and rushes it,
    /// detonating on contact.
    Mine,
    /// Herds rocks toward the ship — nudges the rock nearest it at the player.
    Shepherd,
}

impl EnemyKind {
    /// The kind's baseline fire (or, for a Shepherd, herd) cadence in steps, before
    /// the run's escalation tightens it. A [`Mine`](EnemyKind::Mine) never fires, so
    /// its cadence is zero — the sentinel [`Game::enemy_cadence`] reads as "no fire".
    fn base_cadence(self) -> u32 {
        match self {
            EnemyKind::Orbiter => 96,
            EnemyKind::Diver => 72,
            EnemyKind::Mine => 0,
            EnemyKind::Shepherd => 120,
        }
    }

    /// The tightest cadence it may reach as the run escalates, so heavy waves never
    /// overwhelm the field.
    fn min_cadence(self) -> u32 {
        match self {
            EnemyKind::Orbiter => 48,
            EnemyKind::Diver => 40,
            EnemyKind::Mine => 0,
            EnemyKind::Shepherd => 72,
        }
    }

    /// What downing one is worth. The well eating one instead pays half (see
    /// [`Game::resolve_enemy_accretion`]) — a kill you did not make.
    fn score(self) -> u32 {
        match self {
            EnemyKind::Orbiter => 200,
            EnemyKind::Diver => 150,
            EnemyKind::Mine => 100,
            EnemyKind::Shepherd => 250,
        }
    }
}

/// An enemy riding the gravity field, as the shell should draw it. `x`/`y` are its
/// centre; `kind` is its silhouette and behaviour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub kind: EnemyKind,
}

/// An enemy shot in flight, as the shell should draw it — a small warm blip that,
/// like everything else, curves in the gravity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyBullet {
    pub x: f32,
    pub y: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to. It grows
/// as the collapse and the rest arrive in the later tickets.
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
    /// An enemy loosed a shot this step.
    pub enemy_fired: bool,
    /// An enemy was destroyed this step — by a shot, the collapse, or the well.
    pub enemy_destroyed: bool,
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

/// An enemy's live state: its centre and velocity (its own motion plus whatever the
/// gravity has added), its kind, the steps since it last fired (phased at spawn so a
/// wave staggers), the seconds it has left (finite only for a Diver), and — for a
/// Mine — whether it has woken.
#[derive(Clone, Copy)]
struct EnemyState {
    kind: EnemyKind,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    fire_tick: u32,
    life: f32,
    awake: bool,
}

/// An enemy shot's live state: its centre, its velocity, and the seconds of life it
/// has left. Like the player's shots, it feels the gravity and curves.
#[derive(Clone, Copy)]
struct EnemyBulletState {
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
    blasts: Vec<BlastState>,
    /// The enemies riding the field, and the shots they have loosed.
    enemies: Vec<EnemyState>,
    enemy_bullets: Vec<EnemyBulletState>,
    /// Seconds until the next wave flies in, once the field of enemies is clear.
    wave_timer: f32,
    /// How many waves have flown in so far — drives the rotation through the zoo and
    /// the escalation (size, cadence, fire speed).
    waves_spawned: u32,
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
            enemies: Vec::new(),
            enemy_bullets: Vec::new(),
            wave_timer: ENEMY_WAVE_GAP,
            waves_spawned: 0,
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
        self.advance_enemies(&mut events);
        self.advance_enemy_bullets();
        self.advance_blasts();
        if self.invuln > 0.0 {
            self.invuln = (self.invuln - TIMESTEP).max(0.0);
        }
        self.decay_feed_streak();

        self.resolve_shot_hits(&mut events);
        self.resolve_accretion(&mut events);
        self.resolve_enemy_accretion(&mut events);
        if self.ship_alive && self.invuln <= 0.0 {
            self.resolve_ship_death(&mut events);
        }
        self.manage_waves();
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

    /// The ship is destroyed if it falls into a well's core, is struck by a rock, runs
    /// into a **Mine**, or is caught by enemy fire. Each enemy kind keeps its own
    /// threat: a Mine is the contact hazard (it detonates on the ship), an Orbiter and
    /// Diver menace with their fire, a Shepherd with the rocks it herds — so the other
    /// craft can be flown past, only their shots (and the rocks) bite.
    fn resolve_ship_death(&mut self, events: &mut Events) {
        let ship = (self.ship.x, self.ship.y, SHIP_RADIUS);
        let into_well = in_any_core(&self.wells, self.ship.x, self.ship.y);
        let hit_rock = self
            .asteroids
            .iter()
            .any(|a| overlap(ship, (a.x, a.y, a.size.radius())));
        let hit_mine = self
            .enemies
            .iter()
            .any(|e| e.kind == EnemyKind::Mine && overlap(ship, (e.x, e.y, ENEMY_RADIUS)));
        let hit_bullet = self
            .enemy_bullets
            .iter()
            .any(|b| overlap(ship, (b.x, b.y, ENEMY_SHOT_RADIUS)));
        if into_well || hit_rock || hit_mine || hit_bullet {
            // The Mine the ship ran into detonates with it.
            self.enemies
                .retain(|e| e.kind != EnemyKind::Mine || !overlap(ship, (e.x, e.y, ENEMY_RADIUS)));
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
    /// that shatters the fine debris and enemies it catches — the screen-clearing
    /// panic button — and flings the rest of the rocks outward.
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
        // It sweeps the field clear of enemies and their fire, scoring each craft.
        if !self.enemies.is_empty() {
            for e in &self.enemies {
                self.score += e.kind.score();
            }
            self.enemies.clear();
            events.enemy_destroyed = true;
        }
        self.enemy_bullets.clear();
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
        self.enemy_bullets.clear();
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

    /// Flies every enemy: each rides the wells' pull, then adds its own behaviour on
    /// top — a Mine wakes and rushes the ship, a Diver burns down its life. Divers
    /// whose run is spent leave the field. Once moved, the settled enemies act (fire
    /// or herd) on their cadences, but only with a ship on the field to aim at.
    fn advance_enemies(&mut self, events: &mut Events) {
        let (ship_x, ship_y) = (self.ship.x, self.ship.y);
        let ship_alive = self.ship_alive;
        for e in &mut self.enemies {
            let (gx, gy) = gravity_at(&self.wells, e.x, e.y);
            e.vx += gx * TIMESTEP;
            e.vy += gy * TIMESTEP;
            match e.kind {
                EnemyKind::Mine if ship_alive => {
                    let dx = ring_delta(ship_x, e.x, LOGICAL_WIDTH);
                    let dy = ring_delta(ship_y, e.y, LOGICAL_HEIGHT);
                    let d2 = dx * dx + dy * dy;
                    if d2 <= MINE_WAKE_RADIUS * MINE_WAKE_RADIUS {
                        e.awake = true;
                    }
                    if e.awake {
                        let d = d2.sqrt().max(1.0);
                        e.vx += MINE_THRUST * dx / d * TIMESTEP;
                        e.vy += MINE_THRUST * dy / d * TIMESTEP;
                    }
                }
                EnemyKind::Diver => e.life -= TIMESTEP,
                _ => {}
            }
            e.x = wrap(e.x + e.vx * TIMESTEP, LOGICAL_WIDTH);
            e.y = wrap(e.y + e.vy * TIMESTEP, LOGICAL_HEIGHT);
        }
        self.enemies
            .retain(|e| e.kind != EnemyKind::Diver || e.life > 0.0);

        self.enemy_actions(events);
    }

    /// Every settled enemy acts on its own cadence — the pattern zoo. An Orbiter fires
    /// an aimed shot, a Diver fires ahead along its dive, and a Shepherd herds the
    /// rock nearest it toward the ship. A Mine has no cadence — it only ever rushes.
    /// Enemies keep firing through the brief pause after a death; they aim wherever the
    /// ship last was.
    fn enemy_actions(&mut self, events: &mut Events) {
        let (ship_x, ship_y) = (self.ship.x, self.ship.y);
        for i in 0..self.enemies.len() {
            let kind = self.enemies[i].kind;
            let cadence = self.enemy_cadence(kind);
            if cadence == 0 {
                continue;
            }
            let ready = {
                let e = &mut self.enemies[i];
                e.fire_tick += 1;
                if e.fire_tick < cadence {
                    false
                } else {
                    e.fire_tick = 0;
                    true
                }
            };
            if !ready {
                continue;
            }
            let (ex, ey, evx, evy) = {
                let e = &self.enemies[i];
                (e.x, e.y, e.vx, e.vy)
            };
            match kind {
                EnemyKind::Orbiter => {
                    self.spawn_enemy_shot(ex, ey, ring_aim(ex, ey, ship_x, ship_y));
                    events.enemy_fired = true;
                }
                EnemyKind::Diver => {
                    self.spawn_enemy_shot(ex, ey, evy.atan2(evx));
                    events.enemy_fired = true;
                }
                EnemyKind::Shepherd => self.herd_nearest_rock(ex, ey, ship_x, ship_y),
                EnemyKind::Mine => {}
            }
        }
    }

    /// Looses an enemy shot from `(x, y)` on `heading`, at a speed that quickens as the
    /// run escalates. Like the player's fire, it then feels the gravity and curves.
    fn spawn_enemy_shot(&mut self, x: f32, y: f32, heading: f32) {
        let speed = ENEMY_SHOT_SPEED + self.escalation() as f32 * ENEMY_SHOT_SPEED_PER_WAVE;
        self.enemy_bullets.push(EnemyBulletState {
            x,
            y,
            vx: heading.cos() * speed,
            vy: heading.sin() * speed,
            life: ENEMY_SHOT_LIFE,
        });
    }

    /// Nudges the rock nearest `(ex, ey)` toward the ship — a Shepherd herding the
    /// field at the player. Does nothing if no rocks remain.
    fn herd_nearest_rock(&mut self, ex: f32, ey: f32, ship_x: f32, ship_y: f32) {
        let nearest = self
            .asteroids
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ring_dist_sq(ex, ey, a.x, a.y).total_cmp(&ring_dist_sq(ex, ey, b.x, b.y))
            })
            .map(|(i, _)| i);
        if let Some(i) = nearest {
            let rock = &mut self.asteroids[i];
            let heading = ring_aim(rock.x, rock.y, ship_x, ship_y);
            rock.vx += heading.cos() * SHEPHERD_NUDGE;
            rock.vy += heading.sin() * SHEPHERD_NUDGE;
        }
    }

    /// Flies every enemy shot under the wells' pull, wrapping it, and drops the ones
    /// that burn out or are swallowed by a well's core.
    fn advance_enemy_bullets(&mut self) {
        for b in &mut self.enemy_bullets {
            let (gx, gy) = gravity_at(&self.wells, b.x, b.y);
            b.vx += gx * TIMESTEP;
            b.vy += gy * TIMESTEP;
            b.x = wrap(b.x + b.vx * TIMESTEP, LOGICAL_WIDTH);
            b.y = wrap(b.y + b.vy * TIMESTEP, LOGICAL_HEIGHT);
            b.life -= TIMESTEP;
        }
        let wells = &self.wells;
        self.enemy_bullets
            .retain(|b| b.life > 0.0 && !in_any_core(wells, b.x, b.y));
    }

    /// Enemies that cross a well's core are devoured like rocks — removed and scored,
    /// though a kill the well made pays the player only half.
    fn resolve_enemy_accretion(&mut self, events: &mut Events) {
        let mut i = 0;
        while i < self.enemies.len() {
            let e = self.enemies[i];
            if in_any_core(&self.wells, e.x, e.y) {
                self.score += e.kind.score() / 2;
                events.enemy_destroyed = true;
                self.enemies.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Runs the wave clock: the field of enemies must be cleared, then after a gap the
    /// next wave flies in — rotated through the zoo and escalated. A living wave holds
    /// the gap open, so waves never overlap.
    fn manage_waves(&mut self) {
        if !self.enemies.is_empty() {
            self.wave_timer = ENEMY_WAVE_GAP;
            return;
        }
        if self.wave_timer > 0.0 {
            self.wave_timer -= TIMESTEP;
            return;
        }
        self.spawn_wave();
    }

    /// Sends in a fresh wave: as many enemies as [`Game::wave_size`], their kinds
    /// rotated through the zoo from a per-wave offset so the mix turns over run to run.
    fn spawn_wave(&mut self) {
        self.waves_spawned += 1;
        self.wave_timer = ENEMY_WAVE_GAP;
        let count = self.wave_size();
        let offset = self.waves_spawned.saturating_sub(1) as usize;
        for k in 0..count {
            let kind = ENEMY_ZOO[(offset + k) % ENEMY_ZOO.len()];
            self.spawn_enemy(kind);
        }
    }

    /// How many enemies this wave carries — growing with the waves flown, capped so a
    /// late field never floods.
    fn wave_size(&self) -> usize {
        (WAVE_BASE_COUNT + self.waves_spawned.saturating_sub(1) as usize / 2).min(WAVE_COUNT_CAP)
    }

    /// How far the run has escalated — climbing with each wave, then holding at a cap
    /// so the pressure plateaus rather than runs away.
    fn escalation(&self) -> u32 {
        self.waves_spawned.saturating_sub(1).min(ESCALATION_CAP)
    }

    /// How often `kind` fires (or herds), tightening as the run escalates but never
    /// past its own floor. A [`Mine`](EnemyKind::Mine) returns zero — it never fires.
    fn enemy_cadence(&self, kind: EnemyKind) -> u32 {
        let base = kind.base_cadence();
        if base == 0 {
            return 0;
        }
        base.saturating_sub(self.escalation() * CADENCE_PER_WAVE)
            .max(kind.min_cadence())
    }

    /// Places one enemy of `kind` on the field, riding the gravity from the off: an
    /// Orbiter or Shepherd is dropped into an orbit of a chosen well (with the
    /// tangential velocity a near-circular orbit needs); a Diver enters at an edge,
    /// aimed just off a well so it slings by; a Mine is scattered into the field at a
    /// creep. Its fire is phased so a wave staggers rather than volleys as one.
    fn spawn_enemy(&mut self, kind: EnemyKind) {
        let anchor = self.wells[self.rng.below(self.wells.len() as u64) as usize];
        let (wx, wy) = (anchor.x, anchor.y);
        let fire_tick = self.rng.below(30) as u32;
        let (x, y, vx, vy) = match kind {
            EnemyKind::Orbiter | EnemyKind::Shepherd => {
                let (r, frac) = if kind == EnemyKind::Orbiter {
                    (ORBITER_RADIUS, 1.0)
                } else {
                    (SHEPHERD_RADIUS, SHEPHERD_ORBIT_FRACTION)
                };
                let a = self.rng.range(0.0, TAU);
                let (rx, ry) = (a.cos(), a.sin());
                let sign = if self.rng.below(2) == 0 { 1.0 } else { -1.0 };
                let v = orbital_speed(r) * frac;
                // The tangent to the orbit, its handedness set by `sign`.
                (wx + rx * r, wy + ry * r, -ry * sign * v, rx * sign * v)
            }
            EnemyKind::Diver => {
                let (ex, ey) = self.random_edge_point();
                let off = if self.rng.below(2) == 0 {
                    DIVER_AIM_OFFSET
                } else {
                    -DIVER_AIM_OFFSET
                };
                let h = ring_aim(ex, ey, wx, wy) + off;
                (ex, ey, h.cos() * DIVER_SPEED, h.sin() * DIVER_SPEED)
            }
            EnemyKind::Mine => {
                let (mx, my) = self.random_ring_point(wx, wy, MINE_SPAWN_MIN, MINE_SPAWN_MAX);
                let a = self.rng.range(0.0, TAU);
                (
                    mx,
                    my,
                    a.cos() * MINE_DRIFT_SPEED,
                    a.sin() * MINE_DRIFT_SPEED,
                )
            }
        };
        self.enemies.push(EnemyState {
            kind,
            x,
            y,
            vx,
            vy,
            fire_tick,
            life: if kind == EnemyKind::Diver {
                DIVER_LIFE
            } else {
                f32::INFINITY
            },
            awake: false,
        });
    }

    /// A seeded point on one of the four field edges — where a Diver enters.
    fn random_edge_point(&mut self) -> (f32, f32) {
        match self.rng.below(4) {
            0 => (self.rng.range(0.0, LOGICAL_WIDTH), 0.0),
            1 => (self.rng.range(0.0, LOGICAL_WIDTH), LOGICAL_HEIGHT),
            2 => (0.0, self.rng.range(0.0, LOGICAL_HEIGHT)),
            _ => (LOGICAL_WIDTH, self.rng.range(0.0, LOGICAL_HEIGHT)),
        }
    }

    /// A seeded point whose distance from `(wx, wy)` falls in `[lo, hi]` — a band
    /// clear of the well's core but short of the field edge.
    fn random_ring_point(&mut self, wx: f32, wy: f32, lo: f32, hi: f32) -> (f32, f32) {
        loop {
            let x = self.rng.range(0.0, LOGICAL_WIDTH);
            let y = self.rng.range(0.0, LOGICAL_HEIGHT);
            let d2 = ring_dist_sq(wx, wy, x, y);
            if d2 >= lo * lo && d2 <= hi * hi {
                break (x, y);
            }
        }
    }

    /// Resolves shots striking rocks and enemies. A struck rock splits into two faster
    /// fragments (a small into none); a struck enemy is downed and scored. Either way
    /// the shot is spent. Rock scoring proper still comes from accretion.
    fn resolve_shot_hits(&mut self, events: &mut Events) {
        let mut fragments = Vec::new();
        let mut i = 0;
        while i < self.shots.len() {
            let (sx, sy) = (self.shots[i].x, self.shots[i].y);
            let rock_hit = self
                .asteroids
                .iter()
                .position(|a| overlap((sx, sy, SHOT_RADIUS), (a.x, a.y, a.size.radius())));
            if let Some(j) = rock_hit {
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
                continue;
            }
            let enemy_hit = self
                .enemies
                .iter()
                .position(|e| overlap((sx, sy, SHOT_RADIUS), (e.x, e.y, ENEMY_RADIUS)));
            if let Some(j) = enemy_hit {
                let enemy = self.enemies.swap_remove(j);
                self.score += enemy.kind.score();
                events.enemy_destroyed = true;
                self.shots.swap_remove(i);
                continue;
            }
            i += 1;
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

    /// The enemies riding the field, as the shell should draw them.
    pub fn enemies(&self) -> impl Iterator<Item = Enemy> + '_ {
        self.enemies.iter().map(|e| Enemy {
            x: e.x,
            y: e.y,
            kind: e.kind,
        })
    }

    /// How many enemies are on the field.
    pub fn enemy_count(&self) -> usize {
        self.enemies.len()
    }

    /// The enemy shots in flight, as the shell should draw them.
    pub fn enemy_bullets(&self) -> impl Iterator<Item = EnemyBullet> + '_ {
        self.enemy_bullets
            .iter()
            .map(|b| EnemyBullet { x: b.x, y: b.y })
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

/// The heading from `(fx, fy)` toward the nearest image of `(tx, ty)` across the
/// toroidal field — how an enemy aims at the ship, or a Shepherd points a rock at it.
fn ring_aim(fx: f32, fy: f32, tx: f32, ty: f32) -> f32 {
    let dx = ring_delta(tx, fx, LOGICAL_WIDTH);
    let dy = ring_delta(ty, fy, LOGICAL_HEIGHT);
    dy.atan2(dx)
}

/// The speed of a near-circular orbit at radius `r` in the well's field — from
/// `v²/r = GRAVITY/r²`, the balance of centripetal need against the inverse-square
/// pull, so an enemy dropped there with this tangential speed rides the gravity round.
fn orbital_speed(r: f32) -> f32 {
    (GRAVITY / r).sqrt()
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

    /// A uniform integer in `0..n` — for a seeded choice among a handful of options
    /// (which well to anchor to, which edge to enter from, which way to orbit).
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n.max(1)
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

    /// Drops an enemy of `kind` at `(x, y)`, at rest, and hands back its index.
    fn plant_enemy(game: &mut Game, kind: EnemyKind, x: f32, y: f32) -> usize {
        game.enemies.push(EnemyState {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            fire_tick: 0,
            life: if kind == EnemyKind::Diver {
                DIVER_LIFE
            } else {
                f32::INFINITY
            },
            awake: false,
        });
        game.enemies.len() - 1
    }

    fn plant_enemy_bullet(game: &mut Game, x: f32, y: f32) {
        game.enemy_bullets.push(EnemyBulletState {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            life: ENEMY_SHOT_LIFE,
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

    // The enemy zoo. A wave arriving, rotating and firing is reachable by honest play
    // (see `tests/enemies.rs`); the combats below need an enemy set exactly against the
    // ship, a shot or the well — which honest play cannot practically stage — so they
    // plant the piece and then drive it through the real `step` path.

    #[test]
    fn a_shot_downs_an_enemy_and_scores_it() {
        let mut game = empty_field(1);
        plant_enemy(&mut game, EnemyKind::Orbiter, 300.0, 300.0);
        plant_shot(&mut game, 300.0, 300.0);

        let events = game.step(Input::default());

        assert!(events.enemy_destroyed);
        assert_eq!(game.enemy_count(), 0);
        assert_eq!(game.score(), EnemyKind::Orbiter.score());
    }

    #[test]
    fn enemy_fire_destroys_the_ship() {
        let mut game = empty_field(1);
        game.invuln = 0.0;
        let (sx, sy) = (game.ship.x, game.ship.y);
        plant_enemy_bullet(&mut game, sx, sy);

        let events = game.step(Input::default());

        assert!(events.ship_destroyed);
        assert_eq!(game.lives(), LIVES_START - 1);
    }

    #[test]
    fn running_into_a_mine_detonates_both() {
        let mut game = empty_field(1);
        game.invuln = 0.0;
        let (sx, sy) = (game.ship.x, game.ship.y);
        plant_enemy(&mut game, EnemyKind::Mine, sx, sy);

        let events = game.step(Input::default());

        assert!(events.ship_destroyed);
        assert_eq!(game.enemy_count(), 0, "the mine detonates with the ship");
    }

    #[test]
    fn a_collapse_sweeps_the_enemies() {
        let mut game = empty_field(1);
        game.collapse_meter = 1.0;
        plant_enemy(&mut game, EnemyKind::Orbiter, 300.0, 300.0);
        plant_enemy(&mut game, EnemyKind::Mine, 700.0, 500.0);

        let events = game.step(Input {
            collapse: true,
            ..Default::default()
        });

        assert!(events.collapse_fired);
        assert!(events.enemy_destroyed);
        assert_eq!(game.enemy_count(), 0, "the shockwave clears the field");
        assert_eq!(
            game.score(),
            EnemyKind::Orbiter.score() + EnemyKind::Mine.score()
        );
    }

    #[test]
    fn the_well_accretes_an_enemy_that_falls_in() {
        let mut game = empty_field(1);
        plant_enemy(&mut game, EnemyKind::Mine, CENTER_X, CENTER_Y);

        let events = game.step(Input::default());

        assert!(events.enemy_destroyed);
        assert_eq!(game.enemy_count(), 0);
        assert_eq!(
            game.score(),
            EnemyKind::Mine.score() / 2,
            "a kill the well made pays half"
        );
    }

    #[test]
    fn a_mine_wakes_and_rushes_a_nearing_ship() {
        let mut game = empty_field(1);
        // A mine just inside its wake radius, the ship off to its left.
        let (sx, sy) = (game.ship.x, game.ship.y);
        let i = plant_enemy(&mut game, EnemyKind::Mine, sx + MINE_WAKE_RADIUS - 20.0, sy);

        game.step(Input::default());

        let mine = game.enemies[i];
        assert!(mine.awake, "the mine wakes when the ship comes near");
        assert!(mine.vx < 0.0, "and thrusts toward the ship (to its left)");
    }

    #[test]
    fn a_mine_stays_inert_while_the_ship_is_far() {
        let mut game = empty_field(1);
        // Well clear of the wake radius, in a still spot away from the ship and well.
        let i = plant_enemy(&mut game, EnemyKind::Mine, 120.0, 120.0);

        game.step(Input::default());

        assert!(!game.enemies[i].awake, "a distant mine drifts on, inert");
    }

    #[test]
    fn a_shepherd_herds_the_nearest_rock_at_the_ship() {
        let mut game = empty_field(1);
        plant_enemy(&mut game, EnemyKind::Shepherd, 300.0, 400.0);
        plant_rock(&mut game, AsteroidSize::Large, 340.0, 400.0, 0.0);
        // Arm the shepherd so its herd comes round on the next step.
        game.enemies[0].fire_tick = game.enemy_cadence(EnemyKind::Shepherd) - 1;

        let ship = game.ship();
        // The rock's speed along the heading toward the ship, before and after.
        let toward = |a: &AsteroidState| {
            let h = ring_aim(a.x, a.y, ship.x, ship.y);
            a.vx * h.cos() + a.vy * h.sin()
        };
        let before = toward(&game.asteroids[0]);
        game.step(Input::default());
        let after = toward(&game.asteroids[0]);

        assert!(
            after - before > SHEPHERD_NUDGE * 0.5,
            "the herd shoves the rock at the ship (Δ {})",
            after - before
        );
    }

    #[test]
    fn an_orbiter_holds_its_orbit() {
        // Dropped at the orbit radius with the tangential speed the model gives, an
        // orbiter circles the well — never falling into the core, never flung away.
        let mut game = empty_field(1);
        let r = ORBITER_RADIUS;
        let v = orbital_speed(r);
        game.enemies.push(EnemyState {
            kind: EnemyKind::Orbiter,
            x: CENTER_X + r,
            y: CENTER_Y,
            vx: 0.0,
            vy: v, // tangential to the +x radius
            fire_tick: 0,
            life: f32::INFINITY,
            awake: false,
        });

        let (mut min_d, mut max_d, mut min_x) = (f32::INFINITY, 0.0_f32, f32::INFINITY);
        for _ in 0..1200 {
            game.step(Input::default());
            let e = game.enemies[0];
            let d = ((e.x - CENTER_X).powi(2) + (e.y - CENTER_Y).powi(2)).sqrt();
            min_d = min_d.min(d);
            max_d = max_d.max(d);
            min_x = min_x.min(e.x);
        }
        assert!(
            min_d > WELL_CORE_RADIUS * 2.0,
            "the orbiter never falls into the core (min {min_d})"
        );
        assert!(max_d < r * 1.8, "and never escapes (max {max_d})");
        assert!(
            min_x < CENTER_X,
            "and it truly swings around, reaching the far side"
        );
    }

    #[test]
    fn a_diver_leaves_when_its_run_is_spent() {
        let mut game = empty_field(1);
        game.enemies.push(EnemyState {
            kind: EnemyKind::Diver,
            x: 120.0,
            y: 120.0,
            vx: -40.0,
            vy: -40.0, // heading away from the central well, so it is not accreted first
            fire_tick: 0,
            life: 3.0 * TIMESTEP,
            awake: false,
        });

        for _ in 0..4 {
            game.step(Input::default());
        }

        assert!(
            game.enemies().all(|e| e.kind != EnemyKind::Diver),
            "a spent diver leaves the field"
        );
        assert_eq!(game.enemy_count(), 0, "and no fresh wave has arrived yet");
    }

    #[test]
    fn waves_rotate_through_the_whole_zoo() {
        // Clearing the field between waves (in play, the collapse or the guns) cycles
        // the kinds: over a run of waves, every kind in the zoo turns up.
        let mut game = empty_field(1);
        let mut seen: Vec<EnemyKind> = Vec::new();
        for _ in 0..(ENEMY_ZOO.len() * 3) {
            game.enemies.clear();
            game.spawn_wave();
            for e in &game.enemies {
                if !seen.contains(&e.kind) {
                    seen.push(e.kind);
                }
            }
        }
        for kind in ENEMY_ZOO {
            assert!(seen.contains(&kind), "every kind appears: missing {kind:?}");
        }
    }

    #[test]
    fn waves_escalate_in_size_and_tighten_fire() {
        // Reaching a late wave by honest play means clearing every wave before it — a
        // long, aim-perfect run — so the wave counter is set directly and the size and
        // cadence read straight off it.
        let mut game = empty_field(1);
        game.waves_spawned = 1;
        let early_size = game.wave_size();
        let early_cadence = game.enemy_cadence(EnemyKind::Orbiter);

        game.waves_spawned = ESCALATION_CAP + 1;
        let late_size = game.wave_size();
        let late_cadence = game.enemy_cadence(EnemyKind::Orbiter);

        assert!(late_size > early_size, "later waves bring more enemies");
        assert!(late_cadence < early_cadence, "and fire tighter");
        assert!(late_size <= WAVE_COUNT_CAP, "but never past the cap");
    }
}
