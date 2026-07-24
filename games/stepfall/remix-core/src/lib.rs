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
//! The ship flies the lower band with its full toolkit — fire, dash, focus and a
//! graze-fed overdrive — against a **zoo** of squadrons: darts and weavers, ring
//! turrets and spiral spinners, wall gunners and bombers that drop the Faithful's
//! three bombs, all quickening and speeding up as the run wears on. The modes and
//! the meta arrive in later tickets. The ship's **loadout** is handed *in* at
//! construction, so the core never knows the concept of "unlocks" — it only flies
//! whatever it is given.

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

/// The weapon ladder: its top tier, the tighter cadence a rapid tier fires on, the
/// outward lean of a spread's flanking shots, and the offset of a drone's stream.
const WEAPON_MAX: u32 = 4;
const RAPID_FIRE_INTERVAL: u32 = 5;
const SPREAD_VX: f32 = 70.0;
const DRONE_OFFSET: f32 = 9.0;

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

/// The pattern zoo rotates through this many squadron templates before it comes
/// round again; each full turn thickens the field by one, up to a cap.
const TEMPLATE_COUNT: u32 = 6;
const MAX_EXTRA: u32 = 3;
/// How the run escalates: fire tightens and speeds up with each wave until the
/// pressure plateaus at this many waves in.
const WAVE_CAP: u32 = 8;
const CADENCE_PER_WAVE: u32 = 7;
const SPEED_PER_WAVE: f32 = 7.0;

/// Where emplacements (turrets, spinners, wall gunners) anchor, and the inset the
/// row of them keeps from the field's sides.
const TURRET_ROW: f32 = 46.0;
const EMPLACE_MARGIN: f32 = 26.0;
/// The low row bombers march along, close enough for their bombs to bite.
const BOMBER_ROW: f32 = 96.0;

/// Return fire: bullet size and base speed, and the fan a spread fires.
pub const ENEMY_BULLET_SIZE: f32 = 3.0;
const ENEMY_BULLET_SPEED: f32 = 95.0;
const SPREAD_COUNT: u32 = 5;
const SPREAD_STEP: f32 = 0.22;

/// A turret's ring: how many bullets ring the circle, and how far the ring twists
/// between shots so successive rings interleave.
const RING_COUNT: u32 = 12;
const RING_TWIST: f32 = 0.19;
/// How far a spinner advances its arm each shot — the sweep that traces a spiral.
const SPIN_STEP: f32 = 0.55;
/// A wall of fire: how many slots span the field, the inset it keeps, and how much
/// slower than ordinary fire it falls so it can be threaded.
const WALL_SLOTS: usize = 11;
const WALL_MARGIN: f32 = 12.0;
const WALL_SPEED_MULT: f32 = 0.9;

/// The Faithful's three bombs, reimagined as falling fire: the base fall speed, a
/// plunger's committed drop, a rolling bomb's sideways drift, and the rate and
/// reach of a squiggly bomb's weave.
const BOMB_FALL_SPEED: f32 = 82.0;
const PLUNGER_SPEED: f32 = 150.0;
const ROLL_DRIFT: f32 = 52.0;
const WIGGLE_RATE: f32 = 0.14;
const WIGGLE_AMP: f32 = 9.0;

/// The ship's true hitbox is far smaller than its hull — a bullet only bites if
/// it strikes this tiny square at the ship's heart.
const HITBOX_SIZE: f32 = 3.0;

/// Lives a run starts with, and how long the ship is spared after a hit.
pub const LIVES_START: u32 = 3;
const HIT_INVULN: u32 = 90;

/// The dash: how fast it bursts, how long it lasts and covers the ship, and how
/// long before it can be used again.
const DASH_SPEED: f32 = 420.0;
const DASH_TICKS: u32 = 16;
const DASH_COOLDOWN: u32 = 48;

/// Focus: how much it slows the ship, and the tighter hitbox it flies on.
const FOCUS_SPEED_MULT: f32 = 0.45;
const FOCUS_HITBOX: f32 = 1.5;

/// Grazing: how near an enemy bullet must pass the ship's heart to count, how
/// much each graze charges the overdrive, and a full meter.
const GRAZE_RADIUS: f32 = 11.0;
const GRAZE_CHARGE: f32 = 0.06;
pub const OVERDRIVE_MAX: f32 = 1.0;

/// Power-ups: how often a downed enemy leaves one, how big a pickup is (caught
/// against the whole hull, not the tiny hitbox), and how fast it drifts down.
const DROP_CHANCE: u32 = 6;
const PICKUP_SIZE: f32 = 6.0;
const PICKUP_FALL_SPEED: f32 = 55.0;

/// The mothership — a boss that caps a stage, a callback to the Faithful's saucer
/// grown large. Its size, where it settles, how it flies in and sways, and the
/// squadrons a stage runs before it arrives.
pub const BOSS_WIDTH: f32 = 52.0;
pub const BOSS_HEIGHT: f32 = 22.0;
const BOSS_TOP: f32 = 40.0;
const BOSS_ENTRY_SPEED: f32 = 60.0;
const BOSS_SWAY_AMP: f32 = 46.0;
const BOSS_SWAY_RATE: f32 = 0.7;
const WAVES_PER_STAGE: u32 = 3;
/// How many stages a Sortie runs; felling the final stage's mothership wins it.
const SORTIE_STAGES: u32 = 4;
/// Its health, deepening each stage; the speed and sweep of its fire; and how big
/// a bite a nova takes out of it.
const BOSS_BASE_HP: u32 = 60;
const BOSS_HP_PER_STAGE: u32 = 24;
const BOSS_BULLET_SPEED: f32 = 90.0;
const BOSS_SPIN_STEP: f32 = 0.42;
const NOVA_BOSS_DAMAGE: u32 = 8;
/// The weak points on the hull — only a shot that finds a core does damage; the
/// rest of the hull is armour. Each is `(dx, dy, w, h)` from the boss's top-left.
const BOSS_WEAK_POINTS: [(f32, f32, f32, f32); 3] = [
    (9.0, 8.0, 8.0, 8.0),
    (22.0, 5.0, 8.0, 11.0),
    (35.0, 8.0, 8.0, 8.0),
];

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

/// The kind of an enemy — its entry, its idle motion, and above all the pattern
/// it fires. This is the pattern zoo: each kind reads and threatens differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    /// Fires a single aimed dart straight at the ship.
    Dart,
    /// Fires an aimed fan — a spread that blooms toward the ship.
    Weaver,
    /// A stationary emplacement that fires rings in every direction.
    Turret,
    /// A stationary emplacement whose single stream sweeps around, tracing a spiral.
    Spinner,
    /// A stationary gunner that drops a wall of fire, leaving one gap to thread.
    Wall,
    /// Drops the Faithful's rolling, squiggly and plunger bombs as a callback.
    Bomber,
}

impl EnemyKind {
    /// The kind's baseline cadence, before the run's escalation tightens it. A
    /// spinner fires often (few bullets, tracing an arc); a turret rarely (a whole
    /// ring at once).
    fn base_cadence(self) -> u32 {
        match self {
            EnemyKind::Dart => 132,
            EnemyKind::Weaver => 156,
            EnemyKind::Turret => 192,
            EnemyKind::Spinner => 13,
            EnemyKind::Wall => 168,
            EnemyKind::Bomber => 96,
        }
    }

    /// The tightest its cadence may reach as the run escalates, so heavy patterns
    /// never overwhelm the field.
    fn min_cadence(self) -> u32 {
        match self {
            EnemyKind::Spinner => 8,
            EnemyKind::Turret => 132,
            EnemyKind::Wall => 120,
            _ => 66,
        }
    }

    /// Whether it rides the formation's sway (a flyer) or holds still (an emplacement).
    fn sways(self) -> bool {
        matches!(
            self,
            EnemyKind::Dart | EnemyKind::Weaver | EnemyKind::Bomber
        )
    }
}

/// An enemy still flying, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enemy {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Which kind it is — its silhouette and the pattern it fires.
    pub kind: EnemyKind,
}

/// Which of the Faithful's bombs an enemy dropped — a deliberate callback to
/// STEPFALL's return fire, reimagined here as falling shots with their own motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BombKind {
    /// Drifts sideways as it falls, rolling across the field.
    Rolling,
    /// Weaves left and right on the way down.
    Squiggly,
    /// Commits straight down, and fast.
    Plunger,
}

/// What an enemy bullet is: an ordinary pellet, or one of the Faithful's bombs.
/// The shell draws them apart; the rules treat them the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotKind {
    /// A plain pellet, flying a straight heading.
    Pellet,
    /// One of the Faithful's bombs, falling with its own motion.
    Bomb(BombKind),
}

/// An enemy bullet in flight, as the shell should draw it — a small square
/// centred on `(x, y)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyBullet {
    /// Centre x.
    pub x: f32,
    /// Centre y.
    pub y: f32,
    /// Whether it is a plain pellet or one of the Faithful's bombs.
    pub kind: ShotKind,
}

/// What a power-up grants when the ship catches it — earned in a run by clearing
/// enemies, never handed down (that is Phase B's loadout).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PowerUp {
    /// Steps the ship's weapon up its ladder (spread → pierce → rapid → drones).
    Weapon,
    /// Raises a shield that soaks the next hit.
    Shield,
    /// Fills the overdrive meter, a nova ready.
    Overdrive,
}

/// A power-up drifting down the field, as the shell should draw it — a small square
/// centred on `(x, y)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pickup {
    /// Centre x.
    pub x: f32,
    /// Centre y.
    pub y: f32,
    /// What it grants when caught.
    pub kind: PowerUp,
}

/// The mothership on the field, as the shell should draw it — its hull anchored at
/// the top-left `(x, y)`, its health, and which phase it is running.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boss {
    /// Left edge of the hull.
    pub x: f32,
    /// Top edge of the hull.
    pub y: f32,
    /// Health left; the boss falls when it reaches zero.
    pub hp: u32,
    /// The health it entered with, for a health bar.
    pub max_hp: u32,
    /// The phase it is running, `0` (calm) to `2` (enraged).
    pub phase: u8,
}

/// One of the mothership's weak points, in absolute field coordinates — only a
/// shot that finds one does damage; the rest of the hull is armour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeakPoint {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
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
    /// The ship dashed this step.
    pub dashed: bool,
    /// A bullet was grazed this step, charging the overdrive.
    pub grazed: bool,
    /// A full overdrive was spent on a nova this step.
    pub overdrive_fired: bool,
    /// A power-up was collected this step.
    pub power_up_taken: bool,
    /// A shield soaked a hit this step, sparing a life.
    pub shield_broke: bool,
    /// A shot found the mothership's weak point this step.
    pub boss_hit: bool,
    /// The mothership shifted into a new phase this step.
    pub boss_phase_changed: bool,
    /// A mothership was felled this step.
    pub boss_cleared: bool,
    /// A stage was cleared this step and the run plays on (a non-final mothership fell).
    pub stage_cleared: bool,
    /// The run was won this step — a Sortie's final mothership fell.
    pub run_won: bool,
    /// The last life was spent this step — the run is over (lost).
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

/// How a run ended — set once it is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Won — a Sortie's final mothership fell.
    Won,
    /// Lost — the last life was spent.
    Lost,
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

/// One of the ship's shots in flight: its position, its velocity (logical units
/// per second — the spread's flankers lean outward), and whether it pierces on
/// through the enemies it downs rather than spending itself on the first.
#[derive(Clone, Copy)]
struct ShotState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    pierce: bool,
}

/// A power-up drifting down the field, and what it grants when caught.
#[derive(Clone, Copy)]
struct PickupState {
    x: f32,
    y: f32,
    kind: PowerUp,
}

/// An enemy bullet's position and velocity (logical units per second), whether it
/// has already been grazed (so one bullet charges the meter once), and what kind
/// it is — a pellet flies its velocity, a bomb follows its own motion.
#[derive(Clone, Copy)]
struct EnemyBulletState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    grazed: bool,
    kind: ShotKind,
    /// Steps lived, for a squiggly bomb's weave.
    age: u32,
    /// The column it fell from — the axis a squiggly bomb weaves about.
    origin_x: f32,
}

/// How a squadron flies in: straight down from above, or in from the two sides.
#[derive(Clone, Copy)]
enum Entry {
    Top,
    Sides,
}

/// One enemy of a squadron: its kind, the home it flies to and holds, its live
/// position, whether it has arrived and whether it rides the sway, and the
/// per-enemy counters that pace and shape its fire.
#[derive(Clone, Copy)]
struct EnemyState {
    kind: EnemyKind,
    home_x: f32,
    home_y: f32,
    x: f32,
    y: f32,
    /// Whether it has reached its home and may fire.
    entered: bool,
    /// Whether it rides the formation's side-to-side sway.
    sways: bool,
    /// Interrupts since it last fired, phased at spawn so a squadron staggers.
    fire_tick: u32,
    /// A spinner's current arm angle; advanced each shot to sweep a spiral.
    spin: f32,
    /// Shots fired so far — cycles a bomber's bomb kind and a wall's gap.
    salvo: u32,
}

/// The mothership: where its hull sits, whether it has flown in, its health, the
/// phase it runs, and the counters that pace and sweep its fire.
#[derive(Clone, Copy)]
struct Mothership {
    x: f32,
    y: f32,
    home_y: f32,
    entered: bool,
    hp: u32,
    max_hp: u32,
    phase: u8,
    fire_tick: u32,
    spin: f32,
    salvo: u32,
}

/// A game of HAILFALL.
pub struct Game {
    /// Left edge of the ship.
    ship_x: f32,
    /// Top edge of the ship.
    ship_y: f32,
    /// The ship's shots in flight.
    bullets: Vec<ShotState>,
    /// Interrupts until the ship may fire again.
    fire_cooldown: u32,
    /// The ship's weapon tier, `0..=WEAPON_MAX`, stepped up by power-ups.
    weapon_level: u32,
    /// Whether a shield is up to soak the next hit.
    shield: bool,
    /// The power-ups drifting down the field, waiting to be caught.
    pickups: Vec<PickupState>,
    /// The enemies currently flying.
    enemies: Vec<EnemyState>,
    /// The enemy bullets in the air.
    enemy_bullets: Vec<EnemyBulletState>,
    /// Interrupts until the next squadron flies in, once the field is clear.
    wave_gap: u32,
    /// How many squadrons have flown in so far — drives the escalation and the
    /// rotation through the pattern zoo.
    waves_spawned: u32,
    /// Squadrons cleared in the current stage; at [`WAVES_PER_STAGE`] the
    /// mothership arrives instead of another squadron.
    waves_this_stage: u32,
    /// The stage the run is on — deepens the mothership and its fire.
    stage: u32,
    /// The mothership, present only while a boss caps the stage.
    boss: Option<Mothership>,
    /// Lives left; the run ends when this reaches zero.
    lives: u32,
    /// Interrupts of invulnerability left after a hit or during a dash.
    invuln: u32,
    /// Interrupts of dash left, the heading it bursts along, and its cooldown.
    dash_ticks: u32,
    dash_dir: (f32, f32),
    dash_cooldown: u32,
    /// Whether focus is held this step (a tighter hitbox).
    focusing: bool,
    /// The overdrive meter, `0.0..=OVERDRIVE_MAX`, charged by grazing.
    overdrive: f32,
    score: u32,
    /// The run's randomness.
    rng: Rng,
    mode: Mode,
    #[allow(dead_code)]
    loadout: Loadout,
    phase: Phase,
    /// How the run ended, once it is over.
    outcome: Option<Outcome>,
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
            weapon_level: 0,
            shield: false,
            pickups: Vec::new(),
            enemies: Vec::new(),
            enemy_bullets: Vec::new(),
            wave_gap: WAVE_GAP,
            waves_spawned: 0,
            waves_this_stage: 1,
            stage: 0,
            boss: None,
            lives: LIVES_START,
            invuln: 0,
            dash_ticks: 0,
            dash_dir: (0.0, 0.0),
            dash_cooldown: 0,
            focusing: false,
            overdrive: 0.0,
            score: 0,
            rng: Rng::new(seed),
            mode,
            loadout,
            phase: Phase::Playing,
            outcome: None,
            steps: 0,
            seed,
        };
        game.spawn_wave();
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

    /// The power-ups drifting down the field, for the shell to draw.
    pub fn pickups(&self) -> impl Iterator<Item = Pickup> + '_ {
        self.pickups.iter().map(|p| Pickup {
            x: p.x,
            y: p.y,
            kind: p.kind,
        })
    }

    /// The ship's weapon tier, from `0` (base) up to [`WEAPON_MAX`].
    pub fn weapon_level(&self) -> u32 {
        self.weapon_level
    }

    /// Whether a shield is up to soak the next hit.
    pub fn has_shield(&self) -> bool {
        self.shield
    }

    /// The enemies flying, for the shell to draw.
    pub fn enemies(&self) -> impl Iterator<Item = Enemy> + '_ {
        self.enemies.iter().map(|e| Enemy {
            x: e.x,
            y: e.y,
            kind: e.kind,
        })
    }

    /// The enemy bullets in the air, for the shell to draw.
    pub fn enemy_bullets(&self) -> impl Iterator<Item = EnemyBullet> + '_ {
        self.enemy_bullets.iter().map(|b| EnemyBullet {
            x: b.x,
            y: b.y,
            kind: b.kind,
        })
    }

    /// Lives left.
    pub fn lives(&self) -> u32 {
        self.lives
    }

    /// Whether the ship is currently spared after a hit or during a dash.
    pub fn invulnerable(&self) -> bool {
        self.invuln > 0
    }

    /// Whether focus is held this step — the shell reveals the true hitbox while it is.
    pub fn focusing(&self) -> bool {
        self.focusing
    }

    /// The overdrive meter, from `0.0` (empty) to `OVERDRIVE_MAX` (a nova ready).
    pub fn overdrive(&self) -> f32 {
        self.overdrive
    }

    /// The mothership, if one is on the field, for the shell to draw.
    pub fn boss(&self) -> Option<Boss> {
        self.boss.as_ref().map(|b| Boss {
            x: b.x,
            y: b.y,
            hp: b.hp,
            max_hp: b.max_hp,
            phase: b.phase,
        })
    }

    /// The mothership's weak points in field coordinates, for the shell to mark —
    /// empty when no boss is up.
    pub fn boss_weak_points(&self) -> Vec<WeakPoint> {
        match &self.boss {
            Some(b) => BOSS_WEAK_POINTS
                .iter()
                .map(|&(dx, dy, w, h)| WeakPoint {
                    x: b.x + dx,
                    y: b.y + dy,
                    w,
                    h,
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// The stage the run is on, deepening as motherships fall.
    pub fn stage(&self) -> u32 {
        self.stage
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

    /// How the run ended, or `None` while it is still being played.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
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
        self.dash_cooldown = self.dash_cooldown.saturating_sub(1);
        self.focusing = input.focus;
        self.try_dash(input, &mut events);
        self.fly(input);
        self.fire(input, &mut events);
        self.advance_bullets();
        self.advance_enemies();
        self.resolve_hits(&mut events);
        self.enemy_fire();
        self.advance_boss(&mut events);
        self.advance_enemy_bullets(&mut events);
        self.advance_pickups(&mut events);
        self.try_overdrive(input, &mut events);
        self.manage_waves();
        events
    }

    /// Begins a dash on the tap, if it is off cooldown and not already dashing:
    /// a fast burst along the held heading (up, if none), covering the ship in
    /// invulnerability for its duration.
    fn try_dash(&mut self, input: Input, events: &mut Events) {
        if !input.dash || self.dash_cooldown > 0 || self.dash_ticks > 0 {
            return;
        }
        let dx = f32::from(input.right) - f32::from(input.left);
        let dy = f32::from(input.down) - f32::from(input.up);
        let len = (dx * dx + dy * dy).sqrt();
        self.dash_dir = if len > 0.0 {
            (dx / len, dy / len)
        } else {
            (0.0, -1.0)
        };
        self.dash_ticks = DASH_TICKS;
        self.dash_cooldown = DASH_COOLDOWN;
        self.invuln = self.invuln.max(DASH_TICKS);
        events.dashed = true;
    }

    /// Flies the ship, kept within the lower band. A dash bursts along its
    /// heading; otherwise the ship moves on the input, slowed while focusing.
    fn fly(&mut self, input: Input) {
        if self.dash_ticks > 0 {
            self.dash_ticks -= 1;
            let travel = DASH_SPEED * TIMESTEP;
            self.ship_x += self.dash_dir.0 * travel;
            self.ship_y += self.dash_dir.1 * travel;
        } else {
            let speed = if input.focus {
                SHIP_SPEED * FOCUS_SPEED_MULT
            } else {
                SHIP_SPEED
            };
            let travel = speed * TIMESTEP;
            let dx = f32::from(input.right) - f32::from(input.left);
            let dy = f32::from(input.down) - f32::from(input.up);
            self.ship_x += dx * travel;
            self.ship_y += dy * travel;
        }
        self.ship_x = self
            .ship_x
            .clamp(MARGIN, LOGICAL_WIDTH - MARGIN - SHIP_WIDTH);
        self.ship_y = self
            .ship_y
            .clamp(BAND_TOP, LOGICAL_HEIGHT - MARGIN - SHIP_HEIGHT);
    }

    /// Fires on its cadence while fire is held. The weapon ladder shapes the volley:
    /// a single shot at base, a spread fan once it opens, side drones at the top —
    /// or a concentrated twin while focusing. Pierce and the rapid cadence ride the
    /// ladder too.
    fn fire(&mut self, input: Input, events: &mut Events) {
        self.fire_cooldown = self.fire_cooldown.saturating_sub(1);
        if !input.fire || self.fire_cooldown != 0 {
            return;
        }
        let cx = self.ship_x + SHIP_WIDTH / 2.0 - PLAYER_BULLET_WIDTH / 2.0;
        let y = self.ship_y - PLAYER_BULLET_HEIGHT;
        let pierce = self.weapon_level >= 2;
        let up = -PLAYER_BULLET_SPEED;
        if input.focus {
            // A concentrated twin, for precise, heavy fire while threading.
            self.push_shot(cx - 2.0, y, 0.0, up, pierce);
            self.push_shot(cx + 2.0, y, 0.0, up, pierce);
        } else if self.weapon_level == 0 {
            self.push_shot(cx, y, 0.0, up, pierce);
        } else {
            // A spread fan, once the ladder opens up.
            self.push_shot(cx, y, 0.0, up, pierce);
            self.push_shot(cx, y, -SPREAD_VX, up, pierce);
            self.push_shot(cx, y, SPREAD_VX, up, pierce);
            if self.weapon_level >= WEAPON_MAX {
                // Side drones add two parallel streams.
                self.push_shot(cx - DRONE_OFFSET, y, 0.0, up, pierce);
                self.push_shot(cx + DRONE_OFFSET, y, 0.0, up, pierce);
            }
        }
        self.fire_cooldown = if self.weapon_level >= 3 {
            RAPID_FIRE_INTERVAL
        } else {
            FIRE_INTERVAL
        };
        events.shot_fired = true;
    }

    /// Adds one of the ship's shots at `(x, y)` with velocity `(vx, vy)`.
    fn push_shot(&mut self, x: f32, y: f32, vx: f32, vy: f32, pierce: bool) {
        self.bullets.push(ShotState {
            x,
            y,
            vx,
            vy,
            pierce,
        });
    }

    /// Flies every shot on its heading, retiring the ones that leave the field.
    fn advance_bullets(&mut self) {
        for b in &mut self.bullets {
            b.x += b.vx * TIMESTEP;
            b.y += b.vy * TIMESTEP;
        }
        self.bullets.retain(|b| {
            b.y + PLAYER_BULLET_HEIGHT > 0.0
                && b.x + PLAYER_BULLET_WIDTH > 0.0
                && b.x < LOGICAL_WIDTH
        });
    }

    /// Flies each enemy along its entry path to its home, then holds it there —
    /// swaying formations side to side as one, keeping emplacements still.
    fn advance_enemies(&mut self) {
        let sway = SWAY_AMP * (self.steps as f32 * TIMESTEP * SWAY_RATE).sin();
        let step = ENTRY_SPEED * TIMESTEP;
        for e in &mut self.enemies {
            if e.entered {
                e.x = e.home_x + if e.sways { sway } else { 0.0 };
                e.y = e.home_y;
                continue;
            }
            let (dx, dy) = (e.home_x - e.x, e.home_y - e.y);
            let dist = dx.hypot(dy);
            if dist <= step {
                e.x = e.home_x;
                e.y = e.home_y;
                e.entered = true;
            } else {
                e.x += dx / dist * step;
                e.y += dy / dist * step;
            }
        }
    }

    /// Resolves each shot against the enemies. An ordinary shot spends itself on the
    /// first enemy it overlaps; a piercing shot downs every enemy it touches and
    /// flies on. A downed enemy sometimes leaves a power-up where it fell.
    fn resolve_hits(&mut self, events: &mut Events) {
        let mut survivors = Vec::with_capacity(self.bullets.len());
        for bullet in std::mem::take(&mut self.bullets) {
            let rect = (
                bullet.x,
                bullet.y,
                PLAYER_BULLET_WIDTH,
                PLAYER_BULLET_HEIGHT,
            );
            // The mothership's hull stops any shot; only one that finds a core bites.
            if let Some(weak) = self.boss_bullet_hit(rect) {
                if weak {
                    self.damage_boss(1, events);
                }
                continue;
            }
            if bullet.pierce {
                let mut i = 0;
                while i < self.enemies.len() {
                    let e = self.enemies[i];
                    if overlaps(rect, (e.x, e.y, ENEMY_WIDTH, ENEMY_HEIGHT)) {
                        self.down_enemy(i, events);
                    } else {
                        i += 1;
                    }
                }
                survivors.push(bullet);
            } else if let Some(i) = self
                .enemies
                .iter()
                .position(|e| overlaps(rect, (e.x, e.y, ENEMY_WIDTH, ENEMY_HEIGHT)))
            {
                self.down_enemy(i, events);
            } else {
                survivors.push(bullet);
            }
        }
        self.bullets = survivors;
    }

    /// Downs the enemy at `i`: scores it, and sometimes drops a power-up where it fell.
    fn down_enemy(&mut self, i: usize, events: &mut Events) {
        let e = self.enemies.swap_remove(i);
        self.score += ENEMY_SCORE;
        events.enemy_killed = true;
        self.maybe_drop(e.x, e.y);
    }

    /// Sometimes leaves a power-up where an enemy fell — a weapon step most often,
    /// an overdrive charge or a shield less so.
    fn maybe_drop(&mut self, x: f32, y: f32) {
        if self.rng.below(DROP_CHANCE) != 0 {
            return;
        }
        let kind = match self.rng.below(100) {
            0..=54 => PowerUp::Weapon,
            55..=84 => PowerUp::Overdrive,
            _ => PowerUp::Shield,
        };
        self.pickups.push(PickupState {
            x: x + ENEMY_WIDTH / 2.0,
            y: y + ENEMY_HEIGHT / 2.0,
            kind,
        });
    }

    /// Drifts every power-up down, retiring those that fall past the foot, and lets
    /// the ship catch any its whole hull touches — not just the tiny hitbox.
    fn advance_pickups(&mut self, events: &mut Events) {
        let hull = (self.ship_x, self.ship_y, SHIP_WIDTH, SHIP_HEIGHT);
        let dy = PICKUP_FALL_SPEED * TIMESTEP;
        let mut survivors = Vec::with_capacity(self.pickups.len());
        for mut p in std::mem::take(&mut self.pickups) {
            p.y += dy;
            if p.y - PICKUP_SIZE / 2.0 > LOGICAL_HEIGHT {
                continue;
            }
            let rect = (
                p.x - PICKUP_SIZE / 2.0,
                p.y - PICKUP_SIZE / 2.0,
                PICKUP_SIZE,
                PICKUP_SIZE,
            );
            if overlaps(rect, hull) {
                self.take_powerup(p.kind, events);
                continue;
            }
            survivors.push(p);
        }
        self.pickups = survivors;
    }

    /// Applies a caught power-up: steps the weapon up its ladder, raises a shield,
    /// or fills the overdrive.
    fn take_powerup(&mut self, kind: PowerUp, events: &mut Events) {
        match kind {
            PowerUp::Weapon => self.weapon_level = (self.weapon_level + 1).min(WEAPON_MAX),
            PowerUp::Shield => self.shield = true,
            PowerUp::Overdrive => self.overdrive = OVERDRIVE_MAX,
        }
        events.power_up_taken = true;
    }

    /// Every settled enemy fires its own pattern on its own cadence — the pattern
    /// zoo. Aimed darts and blooming spreads, turret rings and spinner spirals,
    /// wall gunners and bomb-dropping bombers, all quickening and speeding up as
    /// the run climbs.
    fn enemy_fire(&mut self) {
        let speed = self.wave_bullet_speed();
        let ship = (
            self.ship_x + SHIP_WIDTH / 2.0,
            self.ship_y + SHIP_HEIGHT / 2.0,
        );
        for i in 0..self.enemies.len() {
            let kind = self.enemies[i].kind;
            if !self.enemies[i].entered {
                continue;
            }
            let cadence = self.enemy_cadence(kind);
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
            let (mx, my, spin, salvo) = {
                let e = &mut self.enemies[i];
                let (spin, salvo) = (e.spin, e.salvo);
                e.spin += SPIN_STEP;
                e.salvo = e.salvo.wrapping_add(1);
                (e.x + ENEMY_WIDTH / 2.0, e.y + ENEMY_HEIGHT, spin, salvo)
            };
            let aim = (ship.1 - my).atan2(ship.0 - mx);
            match kind {
                EnemyKind::Dart => self.spawn_pellet(mx, my, aim, speed),
                EnemyKind::Weaver => {
                    let half = (SPREAD_COUNT as f32 - 1.0) / 2.0;
                    for k in 0..SPREAD_COUNT {
                        self.spawn_pellet(mx, my, aim + (k as f32 - half) * SPREAD_STEP, speed);
                    }
                }
                EnemyKind::Turret => {
                    let twist = salvo as f32 * RING_TWIST;
                    let step = std::f32::consts::TAU / RING_COUNT as f32;
                    for k in 0..RING_COUNT {
                        self.spawn_pellet(mx, my, twist + k as f32 * step, speed);
                    }
                }
                EnemyKind::Spinner => self.spawn_pellet(mx, my, spin, speed),
                EnemyKind::Wall => self.spawn_wall(my, salvo, speed),
                EnemyKind::Bomber => {
                    let bomb = [BombKind::Rolling, BombKind::Squiggly, BombKind::Plunger]
                        [(salvo % 3) as usize];
                    self.spawn_bomb(mx, my, bomb, salvo);
                }
            }
        }
    }

    /// How often `kind` fires, tightening as the run escalates but never past its
    /// own floor, so late waves rain fire without seizing up.
    fn enemy_cadence(&self, kind: EnemyKind) -> u32 {
        kind.base_cadence()
            .saturating_sub(self.escalation() * CADENCE_PER_WAVE)
            .max(kind.min_cadence())
    }

    /// How far the run has escalated — climbing with each wave, then holding at a
    /// cap so the pressure plateaus rather than runs away.
    fn escalation(&self) -> u32 {
        self.waves_spawned.saturating_sub(1).min(WAVE_CAP)
    }

    /// How fast enemy fire flies this wave — faster the deeper the run.
    fn wave_bullet_speed(&self) -> f32 {
        ENEMY_BULLET_SPEED + self.escalation() as f32 * SPEED_PER_WAVE
    }

    /// Adds a pellet at `(x, y)` flying along `angle` at `speed`.
    fn spawn_pellet(&mut self, x: f32, y: f32, angle: f32, speed: f32) {
        self.enemy_bullets.push(EnemyBulletState {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            grazed: false,
            kind: ShotKind::Pellet,
            age: 0,
            origin_x: x,
        });
    }

    /// Drops a full-width wall of fire from height `y`, leaving one slot open — the
    /// gap sliding across as the gunner keeps firing, so it must be re-read.
    fn spawn_wall(&mut self, y: f32, salvo: u32, speed: f32) {
        let gap = salvo as usize % WALL_SLOTS;
        let span = LOGICAL_WIDTH - 2.0 * WALL_MARGIN;
        for slot in 0..WALL_SLOTS {
            if slot == gap {
                continue;
            }
            let x = WALL_MARGIN + slot as f32 * span / (WALL_SLOTS as f32 - 1.0);
            self.enemy_bullets.push(EnemyBulletState {
                x,
                y,
                vx: 0.0,
                vy: speed * WALL_SPEED_MULT,
                grazed: false,
                kind: ShotKind::Pellet,
                age: 0,
                origin_x: x,
            });
        }
    }

    /// Drops one of the Faithful's bombs from `(x, y)` — a plunger straight and
    /// fast, a rolling bomb drifting to a side, a squiggly bomb set to weave.
    fn spawn_bomb(&mut self, x: f32, y: f32, kind: BombKind, salvo: u32) {
        let (vx, vy) = match kind {
            BombKind::Plunger => (0.0, PLUNGER_SPEED),
            BombKind::Rolling => (
                if salvo.is_multiple_of(2) {
                    ROLL_DRIFT
                } else {
                    -ROLL_DRIFT
                },
                BOMB_FALL_SPEED,
            ),
            BombKind::Squiggly => (0.0, BOMB_FALL_SPEED),
        };
        self.enemy_bullets.push(EnemyBulletState {
            x,
            y,
            vx,
            vy,
            grazed: false,
            kind: ShotKind::Bomb(kind),
            age: 0,
            origin_x: x,
        });
    }

    /// Flies every enemy bullet, retiring the ones that leave the field. A bullet
    /// that strikes the ship's tiny hitbox — unless it is spared — costs a life;
    /// one that skims close without hitting is grazed, charging the overdrive.
    fn advance_enemy_bullets(&mut self, events: &mut Events) {
        let hitbox = self.hitbox();
        let (cx, cy) = (
            self.ship_x + SHIP_WIDTH / 2.0,
            self.ship_y + SHIP_HEIGHT / 2.0,
        );
        let mut survivors = Vec::with_capacity(self.enemy_bullets.len());
        let mut struck = false;
        for mut b in std::mem::take(&mut self.enemy_bullets) {
            b.age += 1;
            if let ShotKind::Bomb(BombKind::Squiggly) = b.kind {
                b.y += b.vy * TIMESTEP;
                b.x = b.origin_x + (b.age as f32 * WIGGLE_RATE).sin() * WIGGLE_AMP;
            } else {
                b.x += b.vx * TIMESTEP;
                b.y += b.vy * TIMESTEP;
            }
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
                continue;
            }
            if !b.grazed && (b.x - cx).hypot(b.y - cy) < GRAZE_RADIUS {
                b.grazed = true;
                self.overdrive = (self.overdrive + GRAZE_CHARGE).min(OVERDRIVE_MAX);
                events.grazed = true;
            }
            survivors.push(b);
        }
        self.enemy_bullets = survivors;
        if struck {
            if self.shield {
                // The shield soaks the hit and breaks, sparing the ship for a beat.
                self.shield = false;
                self.invuln = self.invuln.max(HIT_INVULN);
                events.shield_broke = true;
            } else {
                self.lose_life(events);
            }
        }
    }

    /// Spends a full overdrive on a nova: clears the sky of enemy fire, downs every
    /// enemy on the field, and takes a heavy bite out of the mothership.
    fn try_overdrive(&mut self, input: Input, events: &mut Events) {
        if !input.bomb || self.overdrive < OVERDRIVE_MAX {
            return;
        }
        self.overdrive = 0.0;
        self.enemy_bullets.clear();
        let downed = self.enemies.len() as u32;
        if downed > 0 {
            self.score += ENEMY_SCORE * downed;
            self.enemies.clear();
            events.enemy_killed = true;
        }
        if self.boss.is_some() {
            self.damage_boss(NOVA_BOSS_DAMAGE, events);
            self.fell_boss_if_dead(events);
        }
        events.overdrive_fired = true;
    }

    /// The ship's true hitbox — the tiny square at its heart, tighter still while
    /// focusing.
    fn hitbox(&self) -> (f32, f32, f32, f32) {
        let size = if self.focusing {
            FOCUS_HITBOX
        } else {
            HITBOX_SIZE
        };
        let cx = self.ship_x + SHIP_WIDTH / 2.0;
        let cy = self.ship_y + SHIP_HEIGHT / 2.0;
        (cx - size / 2.0, cy - size / 2.0, size, size)
    }

    /// Spends a life to a hit: spares the ship for a beat, and ends the run if
    /// that was the last life.
    fn lose_life(&mut self, events: &mut Events) {
        events.player_hit = true;
        self.lives -= 1;
        self.invuln = HIT_INVULN;
        if self.lives == 0 {
            self.phase = Phase::Over;
            self.outcome = Some(Outcome::Lost);
            events.run_over = true;
        }
    }

    /// Once the field is clear, a short beat later sends in the next thing: another
    /// squadron, or — when the stage has run its squadrons — the mothership that
    /// caps it.
    fn manage_waves(&mut self) {
        if self.boss.is_some() {
            return;
        }
        if !self.enemies.is_empty() {
            self.wave_gap = WAVE_GAP;
            return;
        }
        if self.wave_gap > 0 {
            self.wave_gap -= 1;
            return;
        }
        if self.waves_this_stage >= WAVES_PER_STAGE {
            self.spawn_boss();
            self.waves_this_stage = 0;
        } else {
            self.spawn_wave();
            self.waves_this_stage += 1;
        }
    }

    /// Sends the mothership flying in from above the top of the field, its health
    /// deepening with the stage.
    fn spawn_boss(&mut self) {
        let max_hp = BOSS_BASE_HP + self.stage * BOSS_HP_PER_STAGE;
        self.boss = Some(Mothership {
            x: (LOGICAL_WIDTH - BOSS_WIDTH) / 2.0,
            y: -BOSS_HEIGHT,
            home_y: BOSS_TOP,
            entered: false,
            hp: max_hp,
            max_hp,
            phase: 0,
            fire_tick: 0,
            spin: 0.0,
            salvo: 0,
        });
    }

    /// Flies the mothership in, holds and sways it once settled, re-phases it as its
    /// health falls, and runs its phase's fire pattern on a cadence.
    fn advance_boss(&mut self, events: &mut Events) {
        let (fire, phase, mx, my, spin, salvo) = {
            let steps = self.steps;
            let Some(boss) = self.boss.as_mut() else {
                return;
            };
            if boss.entered {
                boss.y = boss.home_y;
                boss.x = (LOGICAL_WIDTH - BOSS_WIDTH) / 2.0
                    + BOSS_SWAY_AMP * (steps as f32 * TIMESTEP * BOSS_SWAY_RATE).sin();
            } else {
                boss.x = (LOGICAL_WIDTH - BOSS_WIDTH) / 2.0;
                boss.y += BOSS_ENTRY_SPEED * TIMESTEP;
                if boss.y >= boss.home_y {
                    boss.y = boss.home_y;
                    boss.entered = true;
                }
            }
            let new_phase = boss_phase_for(boss.hp, boss.max_hp);
            if new_phase != boss.phase {
                boss.phase = new_phase;
                events.boss_phase_changed = true;
            }
            let mut fire = false;
            if boss.entered {
                boss.fire_tick += 1;
                if boss.fire_tick >= boss_cadence(boss.phase) {
                    boss.fire_tick = 0;
                    fire = true;
                }
            }
            let (spin, salvo) = (boss.spin, boss.salvo);
            if fire {
                boss.spin += BOSS_SPIN_STEP;
                boss.salvo = boss.salvo.wrapping_add(1);
            }
            (
                fire,
                boss.phase,
                boss.x + BOSS_WIDTH / 2.0,
                boss.y + BOSS_HEIGHT,
                spin,
                salvo,
            )
        };
        if fire {
            self.boss_fire(phase, mx, my, spin, salvo);
        }
        self.fell_boss_if_dead(events);
    }

    /// Runs the mothership's fire for `phase`: an aimed spread while calm, a ring
    /// and a spiral arm as it presses, twin spirals and a fast aimed shot enraged.
    fn boss_fire(&mut self, phase: u8, mx: f32, my: f32, spin: f32, salvo: u32) {
        let speed = BOSS_BULLET_SPEED + self.stage as f32 * SPEED_PER_WAVE;
        let aim = (self.ship_y + SHIP_HEIGHT / 2.0 - my).atan2(self.ship_x + SHIP_WIDTH / 2.0 - mx);
        match phase {
            0 => {
                let half = (SPREAD_COUNT as f32 - 1.0) / 2.0;
                for k in 0..SPREAD_COUNT {
                    self.spawn_pellet(mx, my, aim + (k as f32 - half) * SPREAD_STEP, speed);
                }
            }
            1 => {
                let twist = salvo as f32 * RING_TWIST;
                let step = std::f32::consts::TAU / RING_COUNT as f32;
                for k in 0..RING_COUNT {
                    self.spawn_pellet(mx, my, twist + k as f32 * step, speed);
                }
                self.spawn_pellet(mx, my, spin, speed);
            }
            _ => {
                self.spawn_pellet(mx, my, spin, speed);
                self.spawn_pellet(mx, my, spin + std::f32::consts::PI, speed);
                self.spawn_pellet(mx, my, aim, speed * 1.2);
            }
        }
    }

    /// Whether a shot's `rect` struck the mothership — `None` if it missed the hull,
    /// `Some(true)` if it found a weak point, `Some(false)` if the armour stopped it.
    fn boss_bullet_hit(&self, rect: (f32, f32, f32, f32)) -> Option<bool> {
        let boss = self.boss.as_ref()?;
        if !overlaps(rect, (boss.x, boss.y, BOSS_WIDTH, BOSS_HEIGHT)) {
            return None;
        }
        let weak = BOSS_WEAK_POINTS
            .iter()
            .any(|&(dx, dy, w, h)| overlaps(rect, (boss.x + dx, boss.y + dy, w, h)));
        Some(weak)
    }

    /// Takes `amount` of health off the mothership.
    fn damage_boss(&mut self, amount: u32, events: &mut Events) {
        if let Some(boss) = self.boss.as_mut() {
            boss.hp = boss.hp.saturating_sub(amount);
            events.boss_hit = true;
        }
    }

    /// Fells the mothership once its health is gone: clears it, advances the stage,
    /// and primes the beat before the next stage's first squadron.
    fn fell_boss_if_dead(&mut self, events: &mut Events) {
        if self.boss.as_ref().is_some_and(|b| b.hp == 0) {
            self.boss = None;
            self.stage += 1;
            events.boss_cleared = true;
            if self.mode == Mode::Sortie && self.stage >= SORTIE_STAGES {
                // The final mothership of a Sortie has fallen — the run is won.
                self.phase = Phase::Over;
                self.outcome = Some(Outcome::Won);
                events.run_won = true;
            } else {
                // A stage cleared; the run plays on (endlessly for Onslaught/Daily).
                self.wave_gap = WAVE_GAP;
                events.stage_cleared = true;
            }
        }
    }

    /// Sends in the next squadron, rotating through the pattern zoo and thickening
    /// each time the rotation comes round again — the run's steady escalation.
    fn spawn_wave(&mut self) {
        let wave = self.waves_spawned;
        self.waves_spawned += 1;
        let extra = (wave / TEMPLATE_COUNT).min(MAX_EXTRA) as usize;
        match wave % TEMPLATE_COUNT {
            0 => self.spawn_grid(
                EnemyKind::Dart,
                SQUAD_COLS,
                SQUAD_ROWS + extra,
                Entry::Top,
                SQUAD_TOP,
            ),
            1 => self.spawn_grid(
                EnemyKind::Weaver,
                5 + extra,
                2,
                Entry::Sides,
                SQUAD_TOP + 6.0,
            ),
            2 => self.spawn_emplacements(EnemyKind::Turret, 3 + extra),
            3 => self.spawn_emplacements(EnemyKind::Spinner, 2 + extra),
            4 => self.spawn_grid(EnemyKind::Bomber, 4 + extra, 1, Entry::Top, BOMBER_ROW),
            _ => self.spawn_emplacements(EnemyKind::Wall, 1 + extra),
        }
    }

    /// Lays out a rectangular formation of `kind`, flying in from `entry` to settle
    /// with its top row at `top`. Formations ride the sway; emplacements hold still.
    fn spawn_grid(&mut self, kind: EnemyKind, cols: usize, rows: usize, entry: Entry, top: f32) {
        let span = cols.saturating_sub(1) as f32 * ENEMY_GAP_X;
        let first_centre = (LOGICAL_WIDTH - span) / 2.0;
        for row in 0..rows {
            for col in 0..cols {
                let centre = first_centre + col as f32 * ENEMY_GAP_X;
                let home_x = centre - ENEMY_WIDTH / 2.0;
                let home_y = top + row as f32 * ENEMY_GAP_Y;
                let (sx, sy) = match entry {
                    Entry::Top => (
                        home_x,
                        home_y - LOGICAL_HEIGHT * 0.6 - row as f32 * ENEMY_GAP_Y,
                    ),
                    Entry::Sides if col.is_multiple_of(2) => (-ENEMY_WIDTH - 8.0, home_y),
                    Entry::Sides => (LOGICAL_WIDTH + 8.0, home_y),
                };
                self.push_enemy(kind, home_x, home_y, sx, sy);
            }
        }
    }

    /// Lays out a row of stationary emplacements of `kind`, spread across the upper
    /// field and descending from above the top edge to their anchors.
    fn spawn_emplacements(&mut self, kind: EnemyKind, count: usize) {
        let count = count.max(1);
        let usable = LOGICAL_WIDTH - 2.0 * EMPLACE_MARGIN;
        for i in 0..count {
            let home_x = if count == 1 {
                (LOGICAL_WIDTH - ENEMY_WIDTH) / 2.0
            } else {
                EMPLACE_MARGIN + i as f32 * usable / (count as f32 - 1.0) - ENEMY_WIDTH / 2.0
            };
            self.push_enemy(
                kind,
                home_x,
                TURRET_ROW,
                home_x,
                TURRET_ROW - LOGICAL_HEIGHT * 0.6,
            );
        }
    }

    /// Adds one enemy of `kind` bound for `(home_x, home_y)`, entering from
    /// `(sx, sy)`, its fire phased and its spiral arm seeded so a squadron staggers.
    fn push_enemy(&mut self, kind: EnemyKind, home_x: f32, home_y: f32, sx: f32, sy: f32) {
        let fire_tick = self.rng.below(kind.base_cadence());
        let spin = self.rng.below(628) as f32 / 100.0;
        self.enemies.push(EnemyState {
            kind,
            home_x,
            home_y,
            x: sx,
            y: sy,
            entered: false,
            sways: kind.sways(),
            fire_tick,
            spin,
            salvo: 0,
        });
    }
}

/// Whether two rectangles, each `(x, y, width, height)`, overlap.
fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// The mothership's phase for its remaining health: calm above two-thirds, pressing
/// above a third, enraged below.
fn boss_phase_for(hp: u32, max_hp: u32) -> u8 {
    if hp * 3 > max_hp * 2 {
        0
    } else if hp * 3 > max_hp {
        1
    } else {
        2
    }
}

/// The mothership's fire cadence for a phase — quicker the more enraged it is.
fn boss_cadence(phase: u8) -> u32 {
    match phase {
        0 => 40,
        1 => 28,
        _ => 18,
    }
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
            grazed: false,
            kind: ShotKind::Pellet,
            age: 0,
            origin_x: x,
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

    #[test]
    fn a_dash_bursts_far_and_spares_the_ship() {
        // A dash covers far more ground than ordinary flight over the same steps,
        // and the ship is invulnerable while it dashes.
        let dash = {
            let mut g = game();
            let start = g.ship().x;
            for _ in 0..DASH_TICKS {
                g.step(Input {
                    right: true,
                    dash: true,
                    ..Default::default()
                });
            }
            assert!(g.invulnerable(), "the ship is spared while dashing");
            g.ship().x - start
        };
        let flight = {
            let mut g = game();
            let start = g.ship().x;
            for _ in 0..DASH_TICKS {
                g.step(Input {
                    right: true,
                    ..Default::default()
                });
            }
            g.ship().x - start
        };
        assert!(
            dash > flight * 2.0,
            "a dash bursts far past ordinary flight"
        );
    }

    #[test]
    fn focus_slows_the_ship_and_twins_its_fire() {
        // Slower under focus.
        let normal = press(
            Input {
                right: true,
                ..Default::default()
            },
            30,
        )
        .ship()
        .x;
        let focused = press(
            Input {
                right: true,
                focus: true,
                ..Default::default()
            },
            30,
        )
        .ship()
        .x;
        let start = game().ship().x;
        assert!(focused - start < normal - start, "focus slows the ship");

        // A focused shot is a concentrated twin.
        let mut g = game();
        g.step(Input {
            fire: true,
            focus: true,
            ..Default::default()
        });
        assert_eq!(
            g.bullets().count(),
            2,
            "focus concentrates fire into a twin"
        );
    }

    #[test]
    fn grazing_a_bullet_charges_the_overdrive() {
        let mut game = game();
        let (cx, cy) = ship_centre(&game);
        // Close enough to graze, clear of the tiny hitbox.
        bullet_at(&mut game, cx, cy - (GRAZE_RADIUS - 1.0));
        assert_eq!(game.overdrive(), 0.0);

        let events = game.step(Input::default());

        assert!(events.grazed, "skimming a bullet grazes it");
        assert!(game.overdrive() > 0.0, "a graze charges the overdrive");
        assert_eq!(game.lives(), LIVES_START, "a graze is not a hit");
    }

    #[test]
    fn a_full_overdrive_spends_on_a_nova() {
        let mut game = game();
        game.overdrive = OVERDRIVE_MAX;
        bullet_at(&mut game, 60.0, 60.0);
        bullet_at(&mut game, 160.0, 90.0);
        assert!(game.enemies().count() > 0, "a squadron is on the field");

        let events = game.step(Input {
            bomb: true,
            ..Default::default()
        });

        assert!(events.overdrive_fired, "a full overdrive fires the nova");
        assert_eq!(game.enemy_bullets().count(), 0, "the nova clears the sky");
        assert_eq!(game.enemies().count(), 0, "the nova downs the field");
        assert_eq!(game.overdrive(), 0.0, "the nova spends the meter");
    }

    /// Clears the field and settles a lone enemy of `kind`, its fire primed to go
    /// off on the very next step, so a pattern can be read in isolation.
    fn only_enemy(game: &mut Game, kind: EnemyKind, x: f32, y: f32) {
        game.enemies.clear();
        game.enemy_bullets.clear();
        game.enemies.push(EnemyState {
            kind,
            home_x: x,
            home_y: y,
            x,
            y,
            entered: true,
            sways: false,
            fire_tick: kind.base_cadence() - 1,
            spin: 0.0,
            salvo: 0,
        });
    }

    #[test]
    fn darts_aim_one_shot_and_weavers_fan_a_spread() {
        let mut g = game();
        only_enemy(&mut g, EnemyKind::Dart, 100.0, 40.0);
        g.step(Input::default());
        assert_eq!(
            g.enemy_bullets().count(),
            1,
            "a dart is a single aimed shot"
        );

        let mut g = game();
        only_enemy(&mut g, EnemyKind::Weaver, 100.0, 40.0);
        g.step(Input::default());
        assert_eq!(
            g.enemy_bullets().count() as u32,
            SPREAD_COUNT,
            "a weaver fans a spread"
        );
    }

    #[test]
    fn a_turret_fires_a_ring_in_every_direction() {
        let mut g = game();
        only_enemy(&mut g, EnemyKind::Turret, 100.0, 40.0);
        g.step(Input::default());

        let (ox, oy) = (100.0 + ENEMY_WIDTH / 2.0, 40.0 + ENEMY_HEIGHT);
        let shots: Vec<_> = g.enemy_bullets().collect();
        assert!(shots.len() as u32 >= RING_COUNT, "a ring fills the circle");
        assert!(shots.iter().any(|b| b.x < ox - 0.3), "fire to the left");
        assert!(shots.iter().any(|b| b.x > ox + 0.3), "fire to the right");
        assert!(shots.iter().any(|b| b.y < oy - 0.3), "fire upward");
        assert!(shots.iter().any(|b| b.y > oy + 0.3), "fire downward");
    }

    #[test]
    fn a_spinner_sweeps_its_stream_around() {
        let mut g = game();
        only_enemy(&mut g, EnemyKind::Spinner, 100.0, 40.0);
        let (ox, oy) = (100.0 + ENEMY_WIDTH / 2.0, 40.0 + ENEMY_HEIGHT);

        // Read the heading of successive shots; a spinner advances its arm each
        // shot, so the headings rotate rather than repeat.
        let mut headings = Vec::new();
        let mut seen = 0;
        for _ in 0..2_000 {
            g.step(Input::default());
            let bullets: Vec<_> = g.enemy_bullets().collect();
            if bullets.len() > seen {
                let b = bullets.last().unwrap();
                headings.push((b.y - oy).atan2(b.x - ox));
                seen = bullets.len();
                if headings.len() >= 4 {
                    break;
                }
            }
        }
        assert!(headings.len() >= 4, "the spinner kept firing");
        for pair in headings.windows(2) {
            assert!(
                (pair[0] - pair[1]).abs() > 0.1,
                "the stream swept to a new heading"
            );
        }
    }

    #[test]
    fn a_wall_spans_the_field_with_a_gap_to_thread() {
        let mut g = game();
        only_enemy(&mut g, EnemyKind::Wall, 112.0, 30.0);
        g.step(Input::default());

        let xs: Vec<f32> = g.enemy_bullets().map(|b| b.x).collect();
        assert_eq!(
            xs.len(),
            WALL_SLOTS - 1,
            "a wall drops every slot but one — the gap"
        );
        let min = xs.iter().copied().fold(f32::INFINITY, f32::min);
        let max = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > LOGICAL_WIDTH * 0.6, "the wall spans the field");
    }

    #[test]
    fn bombers_drop_the_faithfuls_three_bombs() {
        let mut g = game();
        // Off to the side, so its bombs fall clear of the ship and it keeps dropping.
        only_enemy(&mut g, EnemyKind::Bomber, 40.0, 40.0);

        let mut kinds = std::collections::HashSet::new();
        for _ in 0..3_000 {
            g.step(Input::default());
            for b in g.enemy_bullets() {
                if let ShotKind::Bomb(k) = b.kind {
                    kinds.insert(k);
                }
            }
            if kinds.len() == 3 {
                break;
            }
        }
        assert!(kinds.contains(&BombKind::Rolling), "a rolling bomb dropped");
        assert!(
            kinds.contains(&BombKind::Squiggly),
            "a squiggly bomb dropped"
        );
        assert!(kinds.contains(&BombKind::Plunger), "a plunger dropped");
    }

    #[test]
    fn a_squiggly_bomb_weaves_where_a_plunger_falls_straight() {
        // Squiggly: its x wanders off the column it fell from.
        let mut g = game();
        g.enemies.clear();
        g.enemy_bullets.clear();
        g.spawn_bomb(60.0, 20.0, BombKind::Squiggly, 0);
        let mut weaved = false;
        for _ in 0..200 {
            g.step(Input::default());
            if let Some(b) = g.enemy_bullets().next()
                && (b.x - 60.0).abs() > 2.0
            {
                weaved = true;
                break;
            }
        }
        assert!(weaved, "a squiggly bomb weaves off its column");

        // Plunger: straight down, its column held.
        let mut g = game();
        g.enemies.clear();
        g.enemy_bullets.clear();
        g.spawn_bomb(60.0, 20.0, BombKind::Plunger, 0);
        let mut fell = false;
        for _ in 0..120 {
            g.step(Input::default());
            if let Some(b) = g.enemy_bullets().next() {
                assert!((b.x - 60.0).abs() < 0.001, "a plunger holds its column");
                if b.y > 40.0 {
                    fell = true;
                }
            }
            if fell {
                break;
            }
        }
        assert!(fell, "a plunger falls");
    }

    #[test]
    fn the_run_escalates_faster_fire_and_a_thicker_field() {
        // Bullet speed climbs with the wave.
        let speed_at = |waves: u32| {
            let mut g = game();
            only_enemy(&mut g, EnemyKind::Dart, 100.0, 40.0);
            g.waves_spawned = waves;
            g.step(Input::default());
            let (mx, my) = (100.0 + ENEMY_WIDTH / 2.0, 40.0 + ENEMY_HEIGHT);
            let b = g.enemy_bullets().next().expect("the dart fired");
            (b.x - mx).hypot(b.y - my) / TIMESTEP
        };
        assert!(
            speed_at(WAVE_CAP + 2) > speed_at(1),
            "a later wave's fire flies faster"
        );

        // A later wave fields more enemies than the first.
        let first = game().enemies().count();
        let mut g = game();
        g.enemies.clear();
        g.waves_spawned = TEMPLATE_COUNT; // the next dart grid, a full cycle on
        g.spawn_wave();
        assert!(
            g.enemies().count() > first,
            "the field thickens as the run wears on"
        );
    }

    #[test]
    fn a_seed_and_inputs_replay_identically() {
        let script = |i: usize| Input {
            left: i.is_multiple_of(7),
            right: i.is_multiple_of(3),
            up: i.is_multiple_of(5),
            down: i.is_multiple_of(11),
            fire: i.is_multiple_of(2),
            focus: i.is_multiple_of(13),
            dash: i.is_multiple_of(37),
            bomb: i.is_multiple_of(97),
        };
        let run = || {
            let mut g = Game::new(0x00C0_FFEE, Mode::Daily, Loadout::default());
            for i in 0..4_000 {
                g.step(script(i));
            }
            let enemies: Vec<(f32, f32)> = g.enemies().map(|e| (e.x, e.y)).collect();
            let bullets: Vec<(f32, f32)> = g.enemy_bullets().map(|b| (b.x, b.y)).collect();
            (g.score(), g.lives(), enemies, bullets)
        };
        assert!(
            run() == run(),
            "the same seed and inputs replay the same run"
        );
    }

    /// Drops a power-up of `kind` right onto the ship, for it to catch this step.
    fn pickup_on_ship(game: &mut Game, kind: PowerUp) {
        let s = game.ship();
        game.pickups.push(PickupState {
            x: s.x + SHIP_WIDTH / 2.0,
            y: s.y + SHIP_HEIGHT / 2.0,
            kind,
        });
    }

    /// Settles a lone, still Dart at `(x, y)` — for reading a shot's path against it.
    fn settled_enemy(game: &mut Game, x: f32, y: f32) {
        game.enemies.push(EnemyState {
            kind: EnemyKind::Dart,
            home_x: x,
            home_y: y,
            x,
            y,
            entered: true,
            sways: false,
            fire_tick: 0,
            spin: 0.0,
            salvo: 0,
        });
    }

    fn firing_input() -> Input {
        Input {
            fire: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_weapon_pickup_steps_the_ladder_and_caps() {
        let mut g = game();
        assert_eq!(g.weapon_level(), 0, "a run starts at the base weapon");

        pickup_on_ship(&mut g, PowerUp::Weapon);
        let events = g.step(Input::default());
        assert!(events.power_up_taken, "the ship catches the power-up");
        assert_eq!(g.weapon_level(), 1, "a weapon pickup steps the ladder");

        for _ in 0..WEAPON_MAX + 3 {
            pickup_on_ship(&mut g, PowerUp::Weapon);
            g.step(Input::default());
        }
        assert_eq!(
            g.weapon_level(),
            WEAPON_MAX,
            "the ladder caps at its top tier"
        );
    }

    #[test]
    fn the_ladder_widens_the_volley() {
        // Base: a single shot.
        let mut g = game();
        g.step(firing_input());
        assert_eq!(g.bullets().count(), 1, "the base weapon fires one shot");

        // Spread: a three-way fan that leans outward.
        let mut g = game();
        g.weapon_level = 1;
        g.step(firing_input());
        let xs: Vec<f32> = g.bullets().map(|b| b.x).collect();
        assert_eq!(xs.len(), 3, "the spread fires a three-way fan");
        let min = xs.iter().copied().fold(f32::INFINITY, f32::min);
        let max = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.5, "the fan leans to both sides");

        // Drones: the top tier adds two parallel streams.
        let mut g = game();
        g.weapon_level = WEAPON_MAX;
        g.step(firing_input());
        assert_eq!(g.bullets().count(), 5, "drones add two parallel streams");
    }

    #[test]
    fn a_piercing_shot_downs_more_than_an_ordinary_one() {
        let column = 100.0 + ENEMY_WIDTH / 2.0 - PLAYER_BULLET_WIDTH / 2.0;

        // Piercing: flies through both stacked enemies.
        let mut g = game();
        g.enemies.clear();
        g.bullets.clear();
        settled_enemy(&mut g, 100.0, 60.0);
        settled_enemy(&mut g, 100.0, 40.0);
        g.bullets.push(ShotState {
            x: column,
            y: 74.0,
            vx: 0.0,
            vy: -PLAYER_BULLET_SPEED,
            pierce: true,
        });
        for _ in 0..80 {
            g.step(Input::default());
            if g.enemies().count() == 0 {
                break;
            }
        }
        assert_eq!(g.enemies().count(), 0, "a piercing shot downs both");

        // Ordinary: spends itself on the first; the second survives.
        let mut g = game();
        g.enemies.clear();
        g.bullets.clear();
        settled_enemy(&mut g, 100.0, 60.0);
        settled_enemy(&mut g, 100.0, 40.0);
        g.bullets.push(ShotState {
            x: column,
            y: 74.0,
            vx: 0.0,
            vy: -PLAYER_BULLET_SPEED,
            pierce: false,
        });
        for _ in 0..80 {
            g.step(Input::default());
        }
        assert_eq!(
            g.enemies().count(),
            1,
            "an ordinary shot stops at the first"
        );
    }

    #[test]
    fn the_rapid_tier_fires_faster() {
        let volleys_over = |level: u32, steps: usize| {
            let mut g = game();
            g.weapon_level = level;
            let mut shots = 0;
            for _ in 0..steps {
                if g.step(firing_input()).shot_fired {
                    shots += 1;
                }
            }
            shots
        };
        assert!(
            volleys_over(3, 120) > volleys_over(0, 120),
            "the rapid tier fires more volleys over the same time"
        );
    }

    #[test]
    fn a_shield_soaks_one_hit_then_is_gone() {
        let mut g = game();
        g.shield = true;
        let (cx, cy) = ship_centre(&g);
        bullet_at(&mut g, cx, cy);

        let events = g.step(Input::default());
        assert!(events.shield_broke, "the shield soaks the hit");
        assert!(!events.player_hit, "no life is lost");
        assert_eq!(g.lives(), LIVES_START, "the shield spared the life");
        assert!(!g.has_shield(), "the shield is spent");

        // Wait out the spare, then a second hit costs a life.
        for _ in 0..HIT_INVULN + 1 {
            g.step(Input::default());
        }
        let (cx, cy) = ship_centre(&g);
        bullet_at(&mut g, cx, cy);
        let events = g.step(Input::default());
        assert!(
            events.player_hit,
            "with the shield gone, a hit costs a life"
        );
        assert_eq!(g.lives(), LIVES_START - 1);
    }

    #[test]
    fn an_overdrive_charge_fills_the_meter() {
        let mut g = game();
        assert_eq!(g.overdrive(), 0.0);
        pickup_on_ship(&mut g, PowerUp::Overdrive);
        g.step(Input::default());
        assert_eq!(
            g.overdrive(),
            OVERDRIVE_MAX,
            "an overdrive charge fills the meter"
        );
    }

    #[test]
    fn downing_enemies_sometimes_drops_a_pickup() {
        // Down enemies through the real kill path; over many kills at least one
        // leaves a power-up, but not every kill does.
        let mut g = game();
        let mut kills = 0;
        let mut drops = 0;
        while kills < 300 {
            g.enemies.clear();
            g.pickups.clear();
            settled_enemy(&mut g, 40.0, 30.0);
            let mut events = Events::default();
            g.down_enemy(0, &mut events);
            kills += 1;
            drops += g.pickups.len();
        }
        assert!(drops > 0, "some downed enemies drop a power-up");
        assert!(drops < 300, "but not every one does");
    }

    /// Places a still player shot centred on `(cx, cy)` — for reading it against a
    /// weak point or the hull.
    fn still_bullet(game: &mut Game, cx: f32, cy: f32) {
        game.bullets.push(ShotState {
            x: cx - PLAYER_BULLET_WIDTH / 2.0,
            y: cy - PLAYER_BULLET_HEIGHT / 2.0,
            vx: 0.0,
            vy: 0.0,
            pierce: false,
        });
    }

    /// Spawns the mothership and flies it in, keeping the ship spared through the
    /// setup, so a boss fight can be staged and its real step path exercised.
    fn boss_in_play() -> Game {
        let mut g = game();
        g.enemies.clear();
        g.spawn_boss();
        for _ in 0..220 {
            g.invuln = 10_000;
            g.step(Input::default());
        }
        g
    }

    #[test]
    fn a_stage_caps_with_a_mothership() {
        let mut g = game();
        g.enemies.clear();
        g.waves_this_stage = WAVES_PER_STAGE;
        g.wave_gap = 0;
        g.step(Input::default());
        assert!(
            g.boss().is_some(),
            "after its squadrons, the stage caps with the mothership"
        );
    }

    #[test]
    fn a_mothership_takes_damage_at_a_weak_point() {
        let mut g = boss_in_play();
        let boss = g.boss().expect("the mothership is in play");
        let hp_before = boss.hp;
        let (dx, dy, w, h) = BOSS_WEAK_POINTS[1];
        still_bullet(&mut g, boss.x + dx + w / 2.0, boss.y + dy + h / 2.0);

        g.invuln = 10_000;
        let events = g.step(Input::default());
        assert!(events.boss_hit, "a shot into a core wounds the mothership");
        assert!(g.boss().unwrap().hp < hp_before, "and takes health off");
    }

    #[test]
    fn the_hull_armours_off_the_weak_points() {
        let mut g = boss_in_play();
        let boss = g.boss().unwrap();
        let hp_before = boss.hp;
        // A corner of the hull, clear of every core.
        still_bullet(&mut g, boss.x + 2.0, boss.y + 2.0);

        g.invuln = 10_000;
        let events = g.step(Input::default());
        assert!(!events.boss_hit, "the armour takes no damage");
        assert_eq!(g.boss().unwrap().hp, hp_before, "health holds");
        assert_eq!(g.bullets().count(), 0, "but the hull still stops the shot");
    }

    #[test]
    fn felling_the_mothership_clears_the_stage() {
        let mut g = boss_in_play();
        let stage_before = g.stage();
        g.boss.as_mut().unwrap().hp = 1;
        let boss = g.boss().unwrap();
        let (dx, dy, w, h) = BOSS_WEAK_POINTS[1];
        still_bullet(&mut g, boss.x + dx + w / 2.0, boss.y + dy + h / 2.0);

        g.invuln = 10_000;
        let events = g.step(Input::default());
        assert!(
            events.boss_cleared,
            "the last core hit fells the mothership"
        );
        assert!(g.boss().is_none(), "the mothership is gone");
        assert_eq!(g.stage(), stage_before + 1, "and the stage advances");
    }

    #[test]
    fn a_nova_bites_the_mothership() {
        let mut g = boss_in_play();
        let hp_before = g.boss().unwrap().hp;
        g.overdrive = OVERDRIVE_MAX;

        g.invuln = 10_000;
        g.step(Input {
            bomb: true,
            ..Default::default()
        });
        assert!(
            g.boss().unwrap().hp <= hp_before - NOVA_BOSS_DAMAGE,
            "a nova takes a heavy bite out of the mothership"
        );
    }

    #[test]
    fn the_mothership_runs_multi_phase_patterns() {
        let mut g = boss_in_play();
        assert_eq!(g.boss().unwrap().phase, 0, "it opens in its calm phase");

        let max = g.boss().unwrap().max_hp;
        let mut phases = std::collections::HashSet::new();
        let mut changed = false;
        let mut fired = false;
        for _ in 0..(max * 2) {
            if let Some(boss) = g.boss.as_mut()
                && boss.hp > 0
            {
                boss.hp -= 1;
            }
            g.invuln = 10_000;
            let events = g.step(Input::default());
            changed |= events.boss_phase_changed;
            fired |= g.enemy_bullets().next().is_some();
            match g.boss() {
                Some(boss) => {
                    phases.insert(boss.phase);
                }
                None => break,
            }
        }
        assert!(changed, "a phase change fires as it wears down");
        assert!(phases.contains(&2), "it reaches its enraged phase");
        assert!(fired, "it fills the field with fire");
    }

    /// Spawns a mothership and fells it on the next step, returning that step's
    /// events — for driving the stage/mode flow without staging a whole fight.
    fn spawn_and_fell_boss(g: &mut Game) -> Events {
        g.enemies.clear();
        g.spawn_boss();
        g.boss.as_mut().unwrap().hp = 0;
        g.invuln = 10_000;
        g.step(Input::default())
    }

    #[test]
    fn a_sortie_ends_in_a_win_at_the_final_mothership() {
        let mut g = Game::new(1, Mode::Sortie, Loadout::default());
        g.stage = SORTIE_STAGES - 1; // the final stage
        let events = spawn_and_fell_boss(&mut g);
        assert!(events.run_won, "felling the final mothership wins the run");
        assert!(
            !events.stage_cleared,
            "the final fall is a win, not a stage clear"
        );
        assert_eq!(g.phase(), Phase::Over);
        assert_eq!(g.outcome(), Some(Outcome::Won));
    }

    #[test]
    fn a_sortie_stage_clear_carries_health_on() {
        let mut g = Game::new(1, Mode::Sortie, Loadout::default());
        g.stage = 0;
        g.lives = 2; // some health already spent
        let events = spawn_and_fell_boss(&mut g);
        assert!(events.stage_cleared, "an early mothership clears the stage");
        assert!(!events.run_won, "the run is not yet won");
        assert_eq!(g.phase(), Phase::Playing, "the run plays on");
        assert_eq!(g.lives(), 2, "health carries across the stage");
        assert_eq!(g.stage(), 1, "onto the next stage");
    }

    #[test]
    fn onslaught_never_wins_and_keeps_deepening() {
        let mut g = Game::new(1, Mode::Onslaught, Loadout::default());
        for _ in 0..(SORTIE_STAGES + 3) {
            let events = spawn_and_fell_boss(&mut g);
            assert!(!events.run_won, "onslaught never declares a win");
            assert!(events.stage_cleared, "each mothership just clears a stage");
            assert_eq!(g.phase(), Phase::Playing, "onslaught plays on");
        }
        assert!(g.stage() > SORTIE_STAGES, "and the stages keep deepening");
    }

    #[test]
    fn a_daily_replays_identically_for_its_seed() {
        // Keep the ship alive so the run unfolds; the seed drives the enemy fire, so
        // the bullet stream is the seed's fingerprint.
        let play = |seed: u64| {
            let mut g = Game::new(seed, Mode::Daily, Loadout::default());
            for i in 0..1_500usize {
                g.invuln = 10_000;
                g.step(Input {
                    fire: i.is_multiple_of(2),
                    ..Default::default()
                });
            }
            let fire: Vec<(f32, f32)> = g.enemy_bullets().map(|b| (b.x, b.y)).collect();
            (g.score(), fire)
        };
        assert!(
            play(0x0000_DA11) == play(0x0000_DA11),
            "the same day replays the same run"
        );
        assert!(
            play(0x0000_DA11) != play(0x0000_BEEF),
            "a different day is a different run"
        );
    }

    #[test]
    fn a_lost_run_reports_the_loss() {
        let mut g = game();
        g.lives = 1;
        let (cx, cy) = ship_centre(&g);
        bullet_at(&mut g, cx, cy);
        let events = g.step(Input::default());
        assert!(events.run_over, "the last life ends the run");
        assert_eq!(g.outcome(), Some(Outcome::Lost), "as a loss");
    }

    /// A generous ceiling on how long a firing test plays before giving up.
    const MAX_STEPS: usize = 20_000;
}
