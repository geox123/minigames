//! The pure, deterministic core of **GNASH** — the Collection's faithful
//! recreation of Namco's 1980 arcade maze-chase original, drawn (in the shell)
//! entirely in code per [ADR 0003](../../../docs/adr/0003-code-drawn-visuals.md)
//! and shipped under an **invented name** — Namco is a flagship, actively-enforced
//! franchise, so the per-title re-check lands against the real name, the maze and
//! the cast (see [ADR 0005](../../../docs/adr/0005-pac-man-ip-recheck.md)). Only the
//! *rules and feel* are faithful; the name, the layout and the characters are ours.
//!
//! Like the Collection's other cores it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and advances in fixed timesteps so
//! a seed and a sequence of inputs always replay the same game.
//!
//! It plays out on a **28×31 tile maze** (8-pixel tiles — a 224×248 logical field,
//! the space the original's math ran in), the first grid-bound Game in the
//! Collection. Everything the shell reads is a tile or a pixel on that grid.
//!
//! # What is here so far
//!
//! The **maze** ([T1](https://github.com/geox123/minigames/issues/159)) — an
//! original layout keeping the genre's structure: a corridor grid studded with
//! **dots** and **four power pellets** one near each corner, a central **pen** with
//! its gate, a wrapping middle-row **tunnel**, and the marked **no-up junctions**.
//!
//! The **eater** ([T2](https://github.com/geox123/minigames/issues/160)) threads
//! that maze: it moves tile-to-tile at the level-1 clip, **buffers** the next turn
//! (a pressed direction is taken at the first opening), **corners** (cutting a hair
//! before a tile centre for a sliver of extra ground), **wraps** through the tunnel,
//! and **eats** — a dot (10) stalls it a frame, a power pellet (50) three, the tax
//! that lets the hunt close in while it feeds; clearing all the pickups raises a
//! maze-cleared event.
//!
//! The first **hunter** ([T4](https://github.com/geox123/minigames/issues/162)) —
//! the **Canine**, the direct chaser — navigates the maze by the original's
//! one-tile-lookahead rule: at each tile centre it takes the exit whose next tile is
//! nearest its **target** (never reversing, ties broken up-left-down-right, and never
//! turning up at the marked junctions), targeting the eater's own tile. Contact costs
//! a life.
//!
//! The **four minds** ([T5](https://github.com/geox123/minigames/issues/163)) give
//! the hunt its character. The cast are the mouth's four kinds of tooth, each mind
//! matched to its tooth's nature: the **Canine** — the fang — targets the eater's
//! tile; the **Incisor** — the front tooth, first to cut — four tiles ahead of its
//! facing (with the original's up-facing overflow quirk); the **Wisdom** — the
//! crooked latecomer that pushes off the others — a point pincering off the Canine
//! (doubled through two-ahead); the **Molar** — the back tooth — the eater when far
//! but its own corner when within eight tiles. Each also
//! has a scatter corner it heads for. The Canine starts loose; the other three leave
//! the **pen** on their dot thresholds (or the post-death global counter), while the
//! hunt alternates between scatter and chase on the original's **per-level schedule
//! tiers** ([T6](https://github.com/geox123/minigames/issues/164)): level 1
//! breathes, levels 2–4 and 5+ stretch the third chase past seventeen minutes and
//! shrink the final scatter to a single frame, so deep in a level the hunters seem
//! to reverse without ever relaxing.
//!
//! A **power pellet flips the hunt**
//! ([T7](https://github.com/geox123/minigames/issues/165)): every hunter out on the
//! maze reverses, slows and turns **frightened**, wandering on the seeded RNG while
//! the scatter/chase clock holds its breath. The eater catches them for
//! **200 → 400 → 800 → 1600**, doubling within a pellet; a caught hunter is a pair
//! of **eyes** racing home to regenerate and re-enter through the release rules.
//! The window follows the original's per-level table, down to **zero blue time** at
//! the deep levels, where a pellet scores 50 and reverses no one.

/// The maze is 28 tiles wide and 31 tall — the original's playfield.
pub const COLS: usize = 28;
/// The maze is 28 tiles wide and 31 tall — the original's playfield.
pub const ROWS: usize = 31;
/// A tile is 8 logical pixels on a side, so movement and collision resolve on a
/// sub-tile grid the way the original's did.
pub const TILE: i32 = 8;
/// The maze's width in logical pixels (`COLS * TILE`). The shell scales this up.
pub const LOGICAL_WIDTH: i32 = COLS as i32 * TILE;
/// The maze's height in logical pixels (`ROWS * TILE`). The shell adds its own HUD
/// margins above and below this play area.
pub const LOGICAL_HEIGHT: i32 = ROWS as i32 * TILE;

/// Length of a single simulation step, in seconds. The original's speeds, phase
/// timings and eating stalls are all defined **per frame at 60 Hz**, so the core
/// steps at 60 Hz — the natural unit for reproducing its tables faithfully.
pub const TIMESTEP: f32 = 1.0 / 60.0;

/// The row the side **tunnel** runs along: an entity leaving one end re-enters the
/// other. Ghosts (a later ticket) crawl while crossing it.
pub const TUNNEL_ROW: usize = 14;

/// The four tiles at which a hunter may not choose to turn *upward* — the original's
/// route-shaping quirk, preserved in our own layout. Used by the pursuit AI (a later
/// ticket); recorded here with the maze it belongs to. Each is a genuine up-junction
/// (the tile above it is open), so the restriction bites.
pub const NO_UP_TILES: [(usize, usize); 4] = [(12, 11), (15, 11), (9, 17), (18, 17)];

/// The eater's start tile and facing — centred on this tile, heading left, as the
/// original opened. The tile carries no dot.
pub const EATER_START: (usize, usize) = (13, 23);

/// The pen's interior tile bounds (inclusive): the open box the hunters begin
/// inside, sealed but for the gate above it. The eater never enters. Exposed as the
/// seam the pursuit tickets read pen-membership from, since the interior is otherwise
/// an ordinary [`Tile::Path`]. See [`in_pen`].
pub const PEN_COLS: (i32, i32) = (11, 16);
pub const PEN_ROWS: (i32, i32) = (13, 15);

/// The half-tile offset a mover sits at when it is centred in a tile. Tiles are 8px,
/// so a centred mover is at offset 4 on each axis.
const HALF: i32 = TILE / 2;

/// The eater's speed at level 1, as a percentage of the base rate: it advances a
/// pixel on `EATER_SPEED` of every `SPEED_DEN` frames, so 80 is the original's 80%.
/// (Per-level speeds are a later ticket; this is the opening clip.)
const EATER_SPEED: i32 = 80;
/// The denominator the speed accumulator counts against — a percentage base, so a
/// mover's speed reads directly as a percent.
const SPEED_DEN: i32 = 100;
/// How many pixels before a tile centre a turn may be taken, cutting the corner —
/// the faithful edge over hunters that never corner.
const CORNER: i32 = 3;
/// Frames the eater freezes after eating — one on a dot, three on a power pellet:
/// the original's small tax that lets the hunt close in while it feeds.
const DOT_STALL: u32 = 1;
const POWER_PELLET_STALL: u32 = 3;
/// What a dot and a power pellet score.
const DOT_SCORE: u32 = 10;
const POWER_PELLET_SCORE: u32 = 50;

/// The Canine's start tile — just outside the pen, above the gate, where the direct
/// chaser begins already loose on the maze. It heads left from here.
pub const CANINE_START: (usize, usize) = (13, 11);
/// The other three hunters' start tiles, inside the pen, where they wait for their
/// staggered release thresholds.
pub const INCISOR_START: (usize, usize) = (13, 14);
pub const WISDOM_START: (usize, usize) = (11, 14);
pub const MOLAR_START: (usize, usize) = (16, 14);
/// A hunter's speed at level 1, as a percentage of the base rate — a touch under the
/// eater's, so a clean run stays ahead. (Per-level speeds are a later ticket.)
const HUNTER_SPEED: i32 = 75;
/// A hunter's speed while crossing the tunnel — it crawls there, the original's
/// let-off that a cornered player can exploit.
const HUNTER_TUNNEL_SPEED: i32 = 40;

/// How the minds aim: the Incisor looks this many tiles ahead of the eater; the
/// Wisdom pivots off a point this many ahead; the Molar breaks for its corner within
/// this many tiles of the eater.
const INCISOR_LOOKAHEAD: i32 = 4;
const WISDOM_PIVOT: i32 = 2;
const MOLAR_FLEE_TILES: i32 = 8;

/// The scatter/chase rhythm by level, in simulation frames — the original's three
/// tiers. Level 1 breathes; levels 2–4 stretch the third chase to over seventeen
/// minutes and shrink the last scatter to a **single frame** (the original's quirk:
/// deep in a level the hunters seem to reverse without ever relaxing — kept); level
/// 5 on starts meaner still. The final chase in every tier has no expiry.
const HUNT_SCHEDULE_L1: [(HuntPhase, u32); 8] = [
    (HuntPhase::Scatter, 7 * 60),
    (HuntPhase::Chase, 20 * 60),
    (HuntPhase::Scatter, 7 * 60),
    (HuntPhase::Chase, 20 * 60),
    (HuntPhase::Scatter, 5 * 60),
    (HuntPhase::Chase, 20 * 60),
    (HuntPhase::Scatter, 5 * 60),
    (HuntPhase::Chase, u32::MAX),
];
const HUNT_SCHEDULE_L2_4: [(HuntPhase, u32); 8] = [
    (HuntPhase::Scatter, 7 * 60),
    (HuntPhase::Chase, 20 * 60),
    (HuntPhase::Scatter, 7 * 60),
    (HuntPhase::Chase, 20 * 60),
    (HuntPhase::Scatter, 5 * 60),
    (HuntPhase::Chase, 1033 * 60),
    (HuntPhase::Scatter, 1),
    (HuntPhase::Chase, u32::MAX),
];
const HUNT_SCHEDULE_L5: [(HuntPhase, u32); 8] = [
    (HuntPhase::Scatter, 5 * 60),
    (HuntPhase::Chase, 20 * 60),
    (HuntPhase::Scatter, 5 * 60),
    (HuntPhase::Chase, 20 * 60),
    (HuntPhase::Scatter, 5 * 60),
    (HuntPhase::Chase, 1037 * 60),
    (HuntPhase::Scatter, 1),
    (HuntPhase::Chase, u32::MAX),
];

/// The scatter/chase schedule a level runs on: 1 / 2–4 / 5+, the original's tiers.
fn hunt_schedule(level: u32) -> &'static [(HuntPhase, u32)] {
    match level {
        0 | 1 => &HUNT_SCHEDULE_L1, // 0 unreachable; clamp to the opening tier
        2..=4 => &HUNT_SCHEDULE_L2_4,
        _ => &HUNT_SCHEDULE_L5,
    }
}

/// Personal dot thresholds for the three waiting hunters, in release order.
const INCISOR_RELEASE_DOTS: u32 = 0;
const WISDOM_RELEASE_DOTS: u32 = 30;
const MOLAR_RELEASE_DOTS: u32 = 60;
/// After a death, the original's global counter releases another hunter every seven
/// pickups. This counter is deliberately separate from the personal thresholds.
const GLOBAL_RELEASE_DOTS: u32 = 7;
/// A waiting hunter is forced out after four seconds without a pickup. (The
/// original tightens this to three seconds at level 5+; that lands with T8's
/// per-level tables.)
const RELEASE_TIMEOUT_FRAMES: u32 = 4 * 60;

/// A frightened hunter's speed — 50% of a frame's full budget, the original's
/// level-1 frightened rate. (Per-level frightened speeds land with T8's tables.)
const FRIGHTENED_SPEED: i32 = 50;
/// Eyes race home at about twice the hunting clip, and the tunnel does not slow
/// them — nothing does.
const EYES_SPEED: i32 = 160;
/// What catching frightened hunters scores: doubling with each catch on a single
/// pellet, resetting on the next pellet.
const CATCH_SCORES: [u32; 4] = [200, 400, 800, 1600];

/// The frightened window by level — `(frames, end flashes)`, the original's table:
/// six seconds at level 1, wobbling downward (with the odd recovery) to **zero** at
/// level 17 and from 19 on, where a power pellet buys no blue time at all and
/// reverses no one — the late game's cruelty, kept.
const FRIGHT_TABLE: [(u32, u32); 18] = [
    (6 * 60, 5), // level 1
    (5 * 60, 5),
    (4 * 60, 5),
    (3 * 60, 5),
    (2 * 60, 5),
    (5 * 60, 5),
    (2 * 60, 5),
    (2 * 60, 5),
    (60, 3),
    (5 * 60, 5), // level 10
    (2 * 60, 5),
    (60, 3),
    (60, 3),
    (3 * 60, 5),
    (60, 3),
    (60, 3),
    (0, 0),
    (60, 3), // level 18 — the one late reprieve
];

/// The frightened window a level grants; levels past the table's end grant none.
fn fright_for(level: u32) -> (u32, u32) {
    let index = level.saturating_sub(1) as usize;
    FRIGHT_TABLE.get(index).copied().unwrap_or((0, 0))
}

/// The tile eyes race for — the pen's centre, straight through the gate. Reaching
/// the pen regenerates the hunter.
const EYES_HOME: (i32, i32) = (13, 14);

/// The game's one source of randomness — a seeded xorshift, consumed only by
/// frightened wander, so a seed and an input sequence still replay identically.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        // Xorshift sticks at zero, so displace the seed and keep it nonzero.
        Self {
            state: (seed ^ 0x9E37_79B9_7F4A_7C15).max(1),
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

/// GNASH's original maze — our own layout (ADR 0005), left-right symmetric like the
/// original's. `#` wall, `.` dot, `o` power pellet, ` ` empty path, `-` the pen gate
/// (which only hunters pass). It keeps the structure the genre needs — a full dot
/// grid, four corner power pellets, a central pen, and a wrapping middle-row tunnel —
/// without reproducing Namco's walls.
const LAYOUT: [&str; ROWS] = [
    "############################", // 0
    "#............##............#", // 1
    "#.####.#####.##.#####.####.#", // 2
    "#o####.#####.##.#####.####o#", // 3
    "#.####.#####.##.#####.####.#", // 4
    "#..........................#", // 5
    "#.####.##.########.##.####.#", // 6
    "#.####.##.########.##.####.#", // 7
    "#......##....##....##......#", // 8
    "######.#####.##.#####.######", // 9
    "######.#####.##.#####.######", // 10
    "######.##          ##.######", // 11
    "###### ## ###--### ## ######", // 12
    "###### ## #      # ## ######", // 13
    "          #      #          ", // 14  (tunnel row — dotless)
    "###### ## #      # ## ######", // 15
    "###### ## ######## ## ######", // 16
    "######.##.        .##.######", // 17
    "######.##.########.##.######", // 18
    "######.##.########.##.######", // 19
    "#............##............#", // 20
    "#.####.#####.##.#####.####.#", // 21
    "#.####.#####.##.#####.####.#", // 22
    "#o..##.......  .......##..o#", // 23
    "###.##.##.########.##.##.###", // 24
    "###.##.##.########.##.##.###", // 25
    "#......##....##....##......#", // 26
    "#.##########.##.##########.#", // 27
    "#.##########.##.##########.#", // 28
    "#..........................#", // 29
    "############################", // 30
];

/// A cardinal heading. `None` is not a direction — an entity always has one — but a
/// *desired* turn may be absent, so callers use `Option<Dir>` for that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    /// The unit step in tiles (and, scaled, in pixels): `(dx, dy)` with `y` growing
    /// downward, as the grid is indexed.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    /// The reverse heading — the one a mover may not choose at a junction (except on
    /// a forced reversal), and the axis the pursuit AI forbids.
    pub fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }

    /// Whether this heading runs along the horizontal axis.
    fn horizontal(self) -> bool {
        matches!(self, Dir::Left | Dir::Right)
    }

    /// Whether two headings are at right angles — the shape of a genuine turn (as
    /// opposed to a straight-on or a reversal).
    fn perpendicular(self, other: Dir) -> bool {
        self.horizontal() != other.horizontal()
    }

    /// The tile one step from `(col, row)` in this heading.
    fn neighbor(self, col: i32, row: i32) -> (i32, i32) {
        let (dx, dy) = self.delta();
        (col + dx, row + dy)
    }
}

/// What a tile *is* — its fixed structure, as the shell should draw the maze and the
/// movers should read the walls. Pickups (dots, power pellets) are separate and
/// mutable, since they are eaten; see [`Game::pickup`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// A solid wall — no mover may enter.
    Wall,
    /// An open corridor tile a mover may occupy.
    Path,
    /// The pen's gate — hunters pass through it leaving and re-entering the pen; the
    /// eater treats it as a wall.
    Gate,
}

/// What edible thing sits on a tile — the mutable layer over the fixed [`Tile`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pickup {
    /// Nothing to eat here.
    None,
    /// A dot — the maze's staple, worth 10.
    Dot,
    /// A power pellet — one near each corner, worth 50, and (a later ticket) the
    /// flip that turns the hunt.
    PowerPellet,
}

/// The eater, as the shell should draw it: its centre in logical pixels and the way
/// it faces (which the shell animates the chomp along).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eater {
    pub x: i32,
    pub y: i32,
    pub dir: Dir,
}

/// The eater's live state, in logical pixels: where it is, the way it heads, the
/// turn it wants next (buffered until an opening takes it), a corner-cut in progress,
/// its fractional-speed accumulator, and any eating stall freezing it. The hunters
/// carry their own leaner [`HunterState`] — no buffered turn, corner or stall.
#[derive(Clone, Copy)]
struct MoverState {
    x: i32,
    y: i32,
    dir: Dir,
    /// The direction the player last asked for, kept until a turn can honour it.
    want: Option<Dir>,
    /// A corner-cut in progress: the perpendicular heading the eater is easing into,
    /// held until the travel axis re-centres. `None` outside a corner. (Only the
    /// eater corners; the hunters never set it.)
    turning: Option<Dir>,
    /// Counts up by the mover's speed each frame; every time it passes [`SPEED_DEN`]
    /// the mover advances one pixel, so a fractional speed averages out exactly.
    accum: i32,
    /// Frames left frozen after eating — the eater's feeding tax.
    stall: u32,
}

/// Which of the four minds a hunter has — each steers by its own target rule.
/// The cast are the mouth's four kinds of tooth, each named for the tooth whose
/// nature its mind shares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HunterKind {
    /// The fang. Targets the eater's own tile — the relentless direct chaser.
    Canine,
    /// The front tooth, first to cut. Targets four tiles ahead of the eater —
    /// cutting off where it is going.
    Incisor,
    /// The crooked latecomer, pushing off the others. Targets a point pincering
    /// off the Canine — swinging wildly as the pair move.
    Wisdom,
    /// The back tooth. Targets the eater when far, but breaks for its own corner
    /// when within eight tiles — it lopes in, loses nerve, and comes again.
    Molar,
}

/// What state a hunter is in — how it moves, how it meets the eater, and what the
/// shell draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HunterMode {
    /// On the hunt: steering by its mind (or its corner in scatter). Touching the
    /// eater costs a life.
    Hunting,
    /// Flipped by a power pellet: slowed, wandering at random, harmless — and worth
    /// catching.
    Frightened,
    /// Caught: a pair of eyes racing home to the pen to regenerate.
    Eyes,
}

/// The active part of the hunt's repeating rhythm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HuntPhase {
    /// Hunters steer toward their individual scatter corners.
    Scatter,
    /// Hunters steer according to their individual minds.
    Chase,
}

impl HunterKind {
    /// The off-maze corner this mind heads for in scatter mode (and the Molar breaks for
    /// when the eater comes close) — one per quadrant, so the four scatter apart.
    fn scatter_corner(self) -> (i32, i32) {
        match self {
            HunterKind::Canine => (COLS as i32 - 3, 0), // top-right
            HunterKind::Incisor => (2, 0),              // top-left
            HunterKind::Wisdom => (COLS as i32 - 1, ROWS as i32 - 1), // bottom-right
            HunterKind::Molar => (0, ROWS as i32 - 1),  // bottom-left
        }
    }
}

/// A hunter, as the shell should draw it: its centre in logical pixels, the way it
/// heads (which the shell points its eyes along), and which mind it is (its colour).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hunter {
    pub x: i32,
    pub y: i32,
    pub dir: Dir,
    pub kind: HunterKind,
    /// Whether this hunter is still waiting inside the pen.
    pub penned: bool,
    /// Hunting, frightened, or eyes — what the shell draws it as.
    pub mode: HunterMode,
}

/// A hunter's live state: where it is, the way it heads, its fractional-speed
/// accumulator, which mind it has, and whether it is still penned. Unlike the eater
/// it never buffers a turn or corners — it decides afresh at each tile centre by its
/// target.
#[derive(Clone, Copy)]
struct HunterState {
    x: i32,
    y: i32,
    dir: Dir,
    accum: i32,
    kind: HunterKind,
    /// Still in the pen, immobile, until the release ticket lets it out.
    penned: bool,
    /// Released but still travelling through the gate to join the hunt.
    leaving_pen: bool,
    /// Hunting, frightened, or eyes.
    mode: HunterMode,
}

/// The maze: its fixed walls and gate, and the mutable field of pickups that empties
/// as the eater feeds. Built once from [`LAYOUT`] and thereafter only eaten from.
#[derive(Clone)]
struct Maze {
    tiles: [[Tile; COLS]; ROWS],
    pickups: [[Pickup; COLS]; ROWS],
    /// How many pickups (dots and power pellets) are still on the board.
    remaining: u32,
    /// How many pickups the full maze holds — the count to clear a level.
    total: u32,
}

impl Maze {
    /// Reads [`LAYOUT`] into walls, gate and the initial pickups, counting the total.
    fn new() -> Self {
        let mut tiles = [[Tile::Wall; COLS]; ROWS];
        let mut pickups = [[Pickup::None; COLS]; ROWS];
        let mut total = 0;
        for (r, row) in LAYOUT.iter().enumerate() {
            for (c, ch) in row.bytes().enumerate() {
                let (tile, pickup) = match ch {
                    b'#' => (Tile::Wall, Pickup::None),
                    b'-' => (Tile::Gate, Pickup::None),
                    b' ' => (Tile::Path, Pickup::None),
                    b'.' => (Tile::Path, Pickup::Dot),
                    b'o' => (Tile::Path, Pickup::PowerPellet),
                    other => panic!("unexpected maze glyph {:?} at ({c}, {r})", other as char),
                };
                tiles[r][c] = tile;
                pickups[r][c] = pickup;
                if pickup != Pickup::None {
                    total += 1;
                }
            }
        }
        Self {
            tiles,
            pickups,
            remaining: total,
            total,
        }
    }

    /// The tile at `(col, row)`; out-of-bounds reads are [`Tile::Wall`] except along
    /// the tunnel row, which is open past the horizontal edges so the wrap is legal.
    fn tile(&self, col: i32, row: i32) -> Tile {
        if row == TUNNEL_ROW as i32 && (col < 0 || col >= COLS as i32) {
            return Tile::Path;
        }
        if !in_bounds(col, row) {
            return Tile::Wall;
        }
        self.tiles[row as usize][col as usize]
    }

    /// The pickup on `(col, row)` as it stands now; out-of-bounds is [`Pickup::None`].
    fn pickup(&self, col: i32, row: i32) -> Pickup {
        if !in_bounds(col, row) {
            return Pickup::None;
        }
        self.pickups[row as usize][col as usize]
    }

    /// Takes the pickup off `(col, row)` — clearing it and decrementing the remaining
    /// count — and returns what was there (or [`Pickup::None`] if the tile was empty).
    fn take(&mut self, col: i32, row: i32) -> Pickup {
        let pickup = self.pickup(col, row);
        if pickup != Pickup::None {
            self.pickups[row as usize][col as usize] = Pickup::None;
            self.remaining -= 1;
        }
        pickup
    }
}

/// What the player pressed this step — a desired heading, latched by the core until
/// a turn can honour it. All-false means "no new intent".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl Input {
    /// The single desired heading this step, or `None` if nothing (or an opposing
    /// pair) is pressed. Vertical wins a diagonal tie, arbitrarily but consistently.
    pub fn dir(self) -> Option<Dir> {
        match (self.up, self.down, self.left, self.right) {
            (true, false, _, _) => Some(Dir::Up),
            (false, true, _, _) => Some(Dir::Down),
            (_, _, true, false) => Some(Dir::Left),
            (_, _, false, true) => Some(Dir::Right),
            _ => None,
        }
    }
}

/// What happened during a single [`Game::step`], for the shell to react to. The
/// authoritative score and counts are read from the accessors; these are one-step
/// cues for sound and juice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The eater ate a dot this step.
    pub dot_eaten: bool,
    /// The eater ate a power pellet this step (T7 flips the hunt on it).
    pub power_pellet_eaten: bool,
    /// The maze was cleared of every pickup this step (a later ticket advances the
    /// level on it).
    pub maze_cleared: bool,
    /// A hunter caught the eater this step (lives and respawn are a later ticket).
    pub life_lost: bool,
    /// The hunt switched between scatter and chase this step.
    pub hunt_phase_changed: bool,
    /// A waiting hunter left the pen this step, if one did.
    pub hunter_released: Option<HunterKind>,
    /// A power pellet flipped the hunt to frightened this step. (At the deep levels
    /// the window is zero: the pellet is eaten but this never fires.)
    pub frightened_started: bool,
    /// The frightened window ran out this step and the schedule resumed.
    pub frightened_ended: bool,
    /// The eater caught a frightened hunter this step, and what the catch scored.
    /// If two hunters are somehow caught in one step the later overwrites — the
    /// authoritative score is [`Game::score`]; this is a one-shot cue for juice.
    pub hunter_caught: Option<(HunterKind, u32)>,
    /// A pair of eyes reached the pen and the hunter regenerated this step. Same
    /// last-writer-wins caveat as `hunter_caught`, and just as rare.
    pub hunter_regenerated: Option<HunterKind>,
}

/// Where a game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The game is being played.
    Playing,
    /// Every life has been spent (a later ticket).
    GameOver,
}

/// The whole game: the maze and the eater threading it. Advanced only through
/// [`Game::step`]; everything else is read-only.
pub struct Game {
    maze: Maze,
    eater: MoverState,
    /// The four hunters. The Canine starts loose; the other three wait for their
    /// personal dot thresholds or the post-death global counter.
    hunters: Vec<HunterState>,
    /// Whether a hunter has caught the eater (latched; lives and respawn are a later
    /// ticket).
    caught: bool,
    pickups_eaten: u32,
    post_death_pickups: u32,
    global_release: bool,
    frames_since_pickup: u32,
    hunt_phase: HuntPhase,
    hunt_phase_index: usize,
    hunt_phase_frames: u32,
    /// The level being played, driving which schedule tier the hunt runs on. Fixed
    /// at 1 until T8 advances it on a maze clear.
    level: u32,
    /// Frames left on the frightened window; zero means the hunt is running. While
    /// nonzero the scatter/chase clock holds its breath.
    frightened_frames: u32,
    /// Catches on the current pellet, indexing the doubling score ladder. Reset by
    /// the next pellet.
    catch_streak: usize,
    /// The frightened wander's randomness — the game's only nondeterminism, seeded.
    rng: Rng,
    /// A seam for T8's savage Canine, which keeps chasing during scatter.
    canine_chases_in_scatter: bool,
    score: u32,
    phase: Phase,
    /// Steps taken so far.
    steps: u64,
    /// The seed the game began on, so a restart replays it exactly. (The movers that
    /// later tickets add are what it will seed.)
    seed: u64,
}

impl Game {
    /// Starts a new game. The same seed always produces the same game: the maze is
    /// laid out full, and the eater waits centred on its start tile facing left.
    pub fn new(seed: u64) -> Self {
        let (sx, sy) = tile_center(EATER_START.0 as i32, EATER_START.1 as i32);
        let level = 1;
        let hunters = vec![
            new_hunter(HunterKind::Canine, CANINE_START, Dir::Left, false),
            new_hunter(HunterKind::Incisor, INCISOR_START, Dir::Down, true),
            new_hunter(HunterKind::Wisdom, WISDOM_START, Dir::Up, true),
            new_hunter(HunterKind::Molar, MOLAR_START, Dir::Up, true),
        ];
        Self {
            maze: Maze::new(),
            eater: MoverState {
                x: sx,
                y: sy,
                dir: Dir::Left,
                want: None,
                turning: None,
                accum: 0,
                stall: 0,
            },
            hunters,
            caught: false,
            pickups_eaten: 0,
            post_death_pickups: 0,
            global_release: false,
            frames_since_pickup: 0,
            hunt_phase: hunt_schedule(level)[0].0,
            hunt_phase_index: 0,
            hunt_phase_frames: 0,
            level,
            frightened_frames: 0,
            catch_streak: 0,
            rng: Rng::new(seed),
            canine_chases_in_scatter: false,
            score: 0,
            phase: Phase::Playing,
            steps: 0,
            seed,
        }
    }

    /// Advances the game one fixed timestep, returning what happened for the shell
    /// to react to. The eater threads the maze; the hunters that later tickets add
    /// hang off this same seam.
    pub fn step(&mut self, input: Input) -> Events {
        self.steps += 1;
        let mut events = Events::default();
        if self.phase == Phase::GameOver {
            return events;
        }
        self.frames_since_pickup = self.frames_since_pickup.saturating_add(1);
        if let Some(d) = input.dir() {
            self.eater.want = Some(d);
        }
        self.advance_eater(&mut events);
        if events.power_pellet_eaten {
            self.flip_hunt(&mut events);
        }
        // The original checks a catch both after the eater moves and after the
        // hunters move, so a head-on pass counts as a catch either way.
        self.resolve_contact(&mut events);
        self.advance_hunters(&mut events);
        self.resolve_contact(&mut events);
        self.advance_pen_release(&mut events);
        self.advance_hunt_schedule(&mut events);
        events
    }

    /// A power pellet flips the hunt: every hunter out on the maze reverses, slows
    /// and turns frightened for the level's window, and the catch ladder resets.
    /// At the deep levels the window is zero — the pellet scores, and nothing else
    /// happens at all.
    fn flip_hunt(&mut self, events: &mut Events) {
        let (frames, _) = fright_for(self.level);
        if frames == 0 {
            return;
        }
        self.frightened_frames = frames;
        self.catch_streak = 0;
        events.frightened_started = true;
        for hunter in &mut self.hunters {
            // A second pellet re-frightens and re-reverses a still-blue hunter;
            // only eyes are past caring.
            if !hunter.penned && hunter.mode != HunterMode::Eyes {
                hunter.mode = HunterMode::Frightened;
                // But a hunter mid-doorway keeps its heading: the original never
                // reverses one in or leaving the house — it turns blue and keeps
                // climbing out.
                if !hunter.leaving_pen {
                    hunter.dir = hunter.dir.opposite();
                }
            }
        }
    }

    /// Advances the eater one frame: honour a reversal at once, then spend the frame's
    /// fractional-speed budget one pixel at a time — unless it is frozen mid-feed.
    fn advance_eater(&mut self, events: &mut Events) {
        if self.eater.stall > 0 {
            self.eater.stall -= 1;
            return;
        }
        // The eater alone may reverse on the spot — the corridor behind is open by
        // definition, so no junction is needed. A reversal abandons any corner-cut.
        if self.eater.want == Some(self.eater.dir.opposite()) {
            self.eater.dir = self.eater.dir.opposite();
            self.eater.turning = None;
        }
        self.eater.accum += EATER_SPEED;
        while self.eater.accum >= SPEED_DEN {
            self.eater.accum -= SPEED_DEN;
            if self.advance_eater_pixel(events) {
                break; // ate this pixel — the feeding stall freezes the rest of the frame
            }
        }
    }

    /// Advances the eater a single pixel: eat what is under it, then steer and move.
    /// Returns whether it ate (so the caller freezes the rest of the frame).
    fn advance_eater_pixel(&mut self, events: &mut Events) -> bool {
        let (tc, tr) = tile_at(self.eater.x, self.eater.y);
        if self.eat_at(tc, tr, events) {
            return true;
        }
        let ox = self.eater.x.rem_euclid(TILE);
        let oy = self.eater.y.rem_euclid(TILE);

        // Cornering: a corner-cut already under way keeps easing diagonally; a fresh
        // one starts when a buffered perpendicular turn is open and the eater is in
        // the last few pixels before the tile centre, squarely on the corridor line.
        let corner = self.eater.turning.or_else(|| {
            self.eater.want.filter(|&w| {
                let (nc, nr) = w.neighbor(tc, tr);
                w.perpendicular(self.eater.dir)
                    && self.eater_can_enter(nc, nr)
                    && in_corner_window(ox, oy, self.eater.dir)
            })
        });
        if let Some(w) = corner {
            self.eater.turning = Some(w);
            let d = self.eater.dir;
            // A diagonal pixel — toward the centre along the travel axis, and into the
            // new corridor — until the travel axis re-centres, when the turn is done.
            self.move_eater_pixel(d);
            self.move_eater_pixel(w);
            let centered = if d.horizontal() {
                self.eater.x.rem_euclid(TILE) == HALF
            } else {
                self.eater.y.rem_euclid(TILE) == HALF
            };
            if centered {
                self.eater.dir = w;
                self.eater.turning = None;
                self.eater.want = None; // the buffered turn is spent
            }
            return false;
        }

        if ox == HALF && oy == HALF {
            // Squarely centred: take a buffered turn if its corridor is open, and spend
            // it — so a single press turns once at the first opening, not at every
            // junction thereafter.
            if let Some(w) = self.eater.want {
                let (nc, nr) = w.neighbor(tc, tr);
                if self.eater_can_enter(nc, nr) {
                    self.eater.dir = w;
                    self.eater.want = None;
                }
            }
            // ...then press on if the way ahead is open, else stall against the wall.
            let (nc, nr) = self.eater.dir.neighbor(tc, tr);
            if self.eater_can_enter(nc, nr) {
                self.move_eater_pixel(self.eater.dir);
            }
            return false;
        }

        // Mid-tile with nothing to turn onto: carry straight on toward the next centre.
        self.move_eater_pixel(self.eater.dir);
        false
    }

    /// Eats the pickup on `(col, row)` if there is one: scores it, sets the feeding
    /// stall, and flags a cleared maze. Returns whether anything was eaten.
    fn eat_at(&mut self, col: i32, row: i32, events: &mut Events) -> bool {
        match self.maze.take(col, row) {
            Pickup::None => return false,
            Pickup::Dot => {
                self.score += DOT_SCORE;
                self.eater.stall = DOT_STALL;
                events.dot_eaten = true;
            }
            Pickup::PowerPellet => {
                self.score += POWER_PELLET_SCORE;
                self.eater.stall = POWER_PELLET_STALL;
                events.power_pellet_eaten = true;
            }
        }
        self.pickups_eaten += 1;
        if self.global_release {
            self.post_death_pickups += 1;
        }
        self.frames_since_pickup = 0;
        if self.maze.remaining == 0 {
            events.maze_cleared = true;
        }
        true
    }

    /// Whether the eater may enter tile `(col, row)` — an open corridor, but never a
    /// wall or the pen's gate (which only hunters pass).
    fn eater_can_enter(&self, col: i32, row: i32) -> bool {
        self.maze.tile(col, row) == Tile::Path
    }

    /// Moves the eater one pixel along `dir`, wrapping it through the side tunnel.
    fn move_eater_pixel(&mut self, dir: Dir) {
        (self.eater.x, self.eater.y) = step_pixel(self.eater.x, self.eater.y, dir);
    }

    /// Advances every loose hunter one frame toward its target. Penned hunters hold
    /// still until the release rules let them out.
    fn advance_hunters(&mut self, events: &mut Events) {
        for i in 0..self.hunters.len() {
            if self.hunters[i].penned {
                continue;
            }
            let target = self.hunter_target(i);
            self.advance_hunter(i, target);
            // Eyes that have made it back inside the pen regenerate: penned again,
            // hunting again, and the release machinery re-releases them the way it
            // released them the first time.
            if self.hunters[i].mode == HunterMode::Eyes {
                let tile = tile_at(self.hunters[i].x, self.hunters[i].y);
                if in_pen(tile.0, tile.1) {
                    self.hunters[i].mode = HunterMode::Hunting;
                    self.hunters[i].penned = true;
                    self.hunters[i].leaving_pen = false;
                    events.hunter_regenerated = Some(self.hunters[i].kind);
                }
            }
        }
    }

    /// Releases at most one waiting hunter per frame. Personal thresholds govern a
    /// fresh run; after a death, the seven-pickup global counter takes over. The
    /// timeout guarantees that a stalled eater cannot hold the pen forever.
    fn advance_pen_release(&mut self, events: &mut Events) {
        let next = self.hunters.iter().position(|hunter| hunter.penned);
        let Some(i) = next else { return };

        let threshold_reached = if self.global_release {
            self.post_death_pickups >= GLOBAL_RELEASE_DOTS
        } else {
            self.pickups_eaten >= release_threshold(self.hunters[i].kind)
        };
        let timeout = self.frames_since_pickup >= RELEASE_TIMEOUT_FRAMES;
        if threshold_reached || timeout {
            let kind = self.hunters[i].kind;
            self.hunters[i].penned = false;
            self.hunters[i].leaving_pen = true;
            self.hunters[i].dir = Dir::Up;
            self.hunters[i].accum = SPEED_DEN;
            if self.global_release {
                self.post_death_pickups = 0;
            }
            self.frames_since_pickup = 0;
            events.hunter_released = Some(kind);
        }
    }

    /// Advances the level's scatter/chase clock — unless the hunt is frightened, in
    /// which case the clock holds its breath until the window runs out, and the
    /// still-blue hunters revert where they stand (no reversal on recovery).
    fn advance_hunt_schedule(&mut self, events: &mut Events) {
        if self.frightened_frames > 0 {
            self.frightened_frames -= 1;
            if self.frightened_frames == 0 {
                for hunter in &mut self.hunters {
                    if hunter.mode == HunterMode::Frightened {
                        hunter.mode = HunterMode::Hunting;
                    }
                }
                events.frightened_ended = true;
            }
            return;
        }
        let schedule = hunt_schedule(self.level);
        self.hunt_phase_frames = self.hunt_phase_frames.saturating_add(1);
        let duration = schedule[self.hunt_phase_index].1;
        if self.hunt_phase_frames < duration || self.hunt_phase_index + 1 >= schedule.len() {
            return;
        }
        self.hunt_phase_index += 1;
        self.hunt_phase = schedule[self.hunt_phase_index].0;
        self.hunt_phase_frames = 0;
        for hunter in &mut self.hunters {
            if !hunter.penned {
                hunter.dir = hunter.dir.opposite();
            }
        }
        events.hunt_phase_changed = true;
    }

    /// The tile hunter `i` steers toward, by its mind and the active hunt phase.
    /// (A frightened hunter never consults this — it wanders.)
    fn hunter_target(&self, i: usize) -> (i32, i32) {
        let hunter = self.hunters[i];
        if hunter.mode == HunterMode::Eyes {
            return EYES_HOME;
        }
        if hunter.leaving_pen {
            return (13, 12);
        }
        let eater = tile_at(self.eater.x, self.eater.y);
        if self.hunt_phase == HuntPhase::Scatter
            && !(hunter.kind == HunterKind::Canine && self.canine_chases_in_scatter)
        {
            return hunter.kind.scatter_corner();
        }
        match hunter.kind {
            // The Canine bears straight down on the eater.
            HunterKind::Canine => eater,
            // The Incisor aims a few tiles ahead of where the eater is heading.
            HunterKind::Incisor => ahead_of_eater(eater, self.eater.dir, INCISOR_LOOKAHEAD),
            // The Wisdom doubles the vector from the Canine through a point ahead of the
            // eater — a pincer that swings as the pair move.
            HunterKind::Wisdom => {
                let pivot = ahead_of_eater(eater, self.eater.dir, WISDOM_PIVOT);
                let canine = self.canine_tile();
                (2 * pivot.0 - canine.0, 2 * pivot.1 - canine.1)
            }
            // The Molar chases while far, but breaks for its corner when the eater is near.
            HunterKind::Molar => {
                let own = tile_at(hunter.x, hunter.y);
                if tile_dist_sq(own, eater) > MOLAR_FLEE_TILES * MOLAR_FLEE_TILES {
                    eater
                } else {
                    HunterKind::Molar.scatter_corner()
                }
            }
        }
    }

    /// The Canine's current tile — the Wisdom steers off it. Falls back to the eater's
    /// tile if somehow no Canine is present.
    fn canine_tile(&self) -> (i32, i32) {
        self.hunters
            .iter()
            .find(|h| h.kind == HunterKind::Canine)
            .map_or_else(
                || tile_at(self.eater.x, self.eater.y),
                |h| tile_at(h.x, h.y),
            )
    }

    /// Advances one hunter a frame: spend its fractional-speed budget — slowed when
    /// frightened, racing as eyes, a crawl while crossing the tunnel — one pixel at
    /// a time.
    fn advance_hunter(&mut self, i: usize, target: (i32, i32)) {
        let (_, row) = tile_at(self.hunters[i].x, self.hunters[i].y);
        let base = match self.hunters[i].mode {
            HunterMode::Hunting => HUNTER_SPEED,
            HunterMode::Frightened => FRIGHTENED_SPEED,
            HunterMode::Eyes => EYES_SPEED,
        };
        let speed = if row == TUNNEL_ROW as i32 && self.hunters[i].mode != HunterMode::Eyes {
            base.min(HUNTER_TUNNEL_SPEED)
        } else {
            base
        };
        self.hunters[i].accum += speed;
        while self.hunters[i].accum >= SPEED_DEN {
            self.hunters[i].accum -= SPEED_DEN;
            self.step_hunter_pixel(i, target);
        }
    }

    /// Moves one hunter a single pixel: at a tile centre it picks the exit nearest its
    /// target — or a random one while frightened — then it steps along its heading,
    /// wrapping through the tunnel.
    fn step_hunter_pixel(&mut self, i: usize, target: (i32, i32)) {
        let (tc, tr) = tile_at(self.hunters[i].x, self.hunters[i].y);
        let ox = self.hunters[i].x.rem_euclid(TILE);
        let oy = self.hunters[i].y.rem_euclid(TILE);
        if ox == HALF && oy == HALF {
            let through_gate =
                self.hunters[i].leaving_pen || self.hunters[i].mode == HunterMode::Eyes;
            // A frightened hunter still mid-doorway steers for the exit like anyone
            // leaving the pen; the wander begins once it is out on the maze.
            let wanders =
                self.hunters[i].mode == HunterMode::Frightened && !self.hunters[i].leaving_pen;
            self.hunters[i].dir = if wanders {
                self.choose_frightened_dir(tc, tr, self.hunters[i].dir, through_gate)
            } else {
                self.choose_hunter_dir(tc, tr, self.hunters[i].dir, target, through_gate)
            };
        }
        (self.hunters[i].x, self.hunters[i].y) =
            step_pixel(self.hunters[i].x, self.hunters[i].y, self.hunters[i].dir);
        if self.hunters[i].leaving_pen {
            let tile = tile_at(self.hunters[i].x, self.hunters[i].y);
            if !in_pen(tile.0, tile.1) && self.maze.tile(tile.0, tile.1) != Tile::Gate {
                self.hunters[i].leaving_pen = false;
            }
        }
    }

    /// Chooses a hunter's heading out of tile `(tc, tr)`: among the open exits — never
    /// the reverse of `current`, never up at a no-up junction — the one whose next tile
    /// is nearest `target` in straight-line distance, ties broken up → left → down →
    /// right. A dead end (no exit) forces a reversal.
    fn choose_hunter_dir(
        &self,
        tc: i32,
        tr: i32,
        current: Dir,
        target: (i32, i32),
        through_gate: bool,
    ) -> Dir {
        let reverse = current.opposite();
        let mut best: Option<(Dir, i32)> = None;
        // The tie-break order is the iteration order: with strict-less-than, the first
        // exit at the minimum distance is the one kept.
        for dir in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
            if dir == reverse {
                continue;
            }
            if dir == Dir::Up && is_no_up(tc, tr) {
                continue;
            }
            let (nc, nr) = dir.neighbor(tc, tr);
            if !self.hunter_can_enter(nc, nr, through_gate) {
                continue;
            }
            let dist = tile_dist_sq((nc, nr), target);
            if best.is_none_or(|(_, b)| dist < b) {
                best = Some((dir, dist));
            }
        }
        best.map_or(reverse, |(dir, _)| dir)
    }

    /// Whether a hunter may enter tile `(col, row)`. The gate opens only to a hunter
    /// with business there — leaving the pen, or racing home as eyes; a hunter out on
    /// the maze never walks back in alive. The eater treats it as a wall outright.
    fn hunter_can_enter(&self, col: i32, row: i32, through_gate: bool) -> bool {
        match self.maze.tile(col, row) {
            Tile::Path => true,
            Tile::Gate => through_gate,
            Tile::Wall => false,
        }
    }

    /// Chooses a frightened hunter's heading out of tile `(tc, tr)`: a pseudo-random
    /// pick among the open exits, under the same rules as the hunt — never reversing,
    /// never up at a no-up junction. A dead end still forces the reversal.
    fn choose_frightened_dir(&mut self, tc: i32, tr: i32, current: Dir, through_gate: bool) -> Dir {
        let reverse = current.opposite();
        let mut open = [Dir::Up; 4];
        let mut count = 0;
        for dir in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
            if dir == reverse || (dir == Dir::Up && is_no_up(tc, tr)) {
                continue;
            }
            let (nc, nr) = dir.neighbor(tc, tr);
            if self.hunter_can_enter(nc, nr, through_gate) {
                open[count] = dir;
                count += 1;
            }
        }
        if count == 0 {
            return reverse;
        }
        // Reduce in u64: `as usize` first would truncate on 32-bit targets (wasm)
        // and let the same seed replay differently on web and desktop.
        open[(self.rng.next() % count as u64) as usize]
    }

    /// Resolves the eater sharing a tile with a loose hunter, by the hunter's mode:
    /// a hunting one costs a life (latched; T8 turns the latch into lives and a
    /// reset), a frightened one is caught for the ladder and becomes eyes, and eyes
    /// pass straight through.
    fn resolve_contact(&mut self, events: &mut Events) {
        let eater_tile = tile_at(self.eater.x, self.eater.y);
        for i in 0..self.hunters.len() {
            let hunter = self.hunters[i];
            if hunter.penned || tile_at(hunter.x, hunter.y) != eater_tile {
                continue;
            }
            match hunter.mode {
                HunterMode::Hunting => {
                    if !self.caught {
                        self.caught = true;
                        events.life_lost = true;
                        self.global_release = true;
                        self.post_death_pickups = 0;
                    }
                }
                HunterMode::Frightened => {
                    let score = CATCH_SCORES[self.catch_streak.min(CATCH_SCORES.len() - 1)];
                    self.catch_streak += 1;
                    self.score += score;
                    self.hunters[i].mode = HunterMode::Eyes;
                    events.hunter_caught = Some((hunter.kind, score));
                }
                HunterMode::Eyes => {}
            }
        }
    }

    /// The structural tile at `(col, row)` — what the shell draws and the movers read
    /// as wall or corridor. Out-of-bounds is [`Tile::Wall`] except along the tunnel
    /// row, where it is open so the wrap is legal.
    pub fn tile(&self, col: i32, row: i32) -> Tile {
        self.maze.tile(col, row)
    }

    /// The pickup on `(col, row)` — dot, power pellet, or nothing — as it stands now,
    /// emptying as the eater feeds. Out-of-bounds is [`Pickup::None`].
    pub fn pickup(&self, col: i32, row: i32) -> Pickup {
        self.maze.pickup(col, row)
    }

    /// The eater, as the shell should draw it.
    pub fn eater(&self) -> Eater {
        Eater {
            x: self.eater.x,
            y: self.eater.y,
            dir: self.eater.dir,
        }
    }

    /// The hunters, as the shell should draw them.
    pub fn hunters(&self) -> impl Iterator<Item = Hunter> + '_ {
        self.hunters.iter().map(|h| Hunter {
            x: h.x,
            y: h.y,
            dir: h.dir,
            kind: h.kind,
            penned: h.penned,
            mode: h.mode,
        })
    }

    /// The current scatter/chase phase of the hunt.
    pub fn hunt_phase(&self) -> HuntPhase {
        self.hunt_phase
    }

    /// Frames elapsed in the current scatter/chase phase.
    pub fn hunt_phase_frames(&self) -> u32 {
        self.hunt_phase_frames
    }

    /// The level being played, from 1. (Advancing it on a maze clear is T8's.)
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Frames left on the frightened window — zero when the hunt is running. The
    /// shell reads this against [`Game::frightened_flashes`] to blink the warning.
    pub fn frightened_frames_left(&self) -> u32 {
        self.frightened_frames
    }

    /// How many warning flashes this level's frightened window ends on.
    pub fn frightened_flashes(&self) -> u32 {
        fright_for(self.level).1
    }

    /// The number of pickups eaten so far, useful for the HUD and release tests.
    pub fn pickups_eaten(&self) -> u32 {
        self.pickups_eaten
    }

    /// Number of hunters still waiting in the pen.
    pub fn penned_hunters(&self) -> usize {
        self.hunters.iter().filter(|hunter| hunter.penned).count()
    }

    /// Enables the T8 savage-Canine seam: Canine keeps its chase target during
    /// scatter while the other active hunters still use their corners.
    pub fn set_canine_chases_in_scatter(&mut self, enabled: bool) {
        self.canine_chases_in_scatter = enabled;
    }

    /// Whether a hunter has caught the eater — latched, until a later ticket adds
    /// lives and respawn.
    pub fn caught(&self) -> bool {
        self.caught
    }

    /// How many pickups (dots and power pellets) are still on the board.
    pub fn pickups_remaining(&self) -> u32 {
        self.maze.remaining
    }

    /// How many pickups the full maze holds — the count to clear a level.
    pub fn pickups_total(&self) -> u32 {
        self.maze.total
    }

    /// The running score.
    pub fn score(&self) -> u32 {
        self.score
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

/// The centre pixel of tile `(col, row)` — the point a mover is aligned to when it
/// sits squarely in that tile. Tiles are 8px, so the centre sits at offset 4.
pub fn tile_center(col: i32, row: i32) -> (i32, i32) {
    (col * TILE + TILE / 2, row * TILE + TILE / 2)
}

/// The tile containing pixel `(x, y)`.
pub fn tile_at(x: i32, y: i32) -> (i32, i32) {
    (x.div_euclid(TILE), y.div_euclid(TILE))
}

/// One pixel of travel from `(x, y)` along `dir`, wrapping x through the side tunnel.
/// Both the eater and the hunters step by this.
fn step_pixel(x: i32, y: i32, dir: Dir) -> (i32, i32) {
    let (dx, dy) = dir.delta();
    ((x + dx).rem_euclid(LOGICAL_WIDTH), y + dy)
}

/// Whether a mover heading `dir` is within the cornering window on its approach to a
/// tile centre — the last [`CORNER`] pixels before the centre, and squarely on the
/// corridor's centre-line across the travel axis, so an early perpendicular turn is
/// a clean corner-cut rather than a clip into a wall.
fn in_corner_window(ox: i32, oy: i32, dir: Dir) -> bool {
    match dir {
        Dir::Left => oy == HALF && (HALF + 1..=HALF + CORNER).contains(&ox),
        Dir::Right => oy == HALF && (HALF - CORNER..=HALF - 1).contains(&ox),
        Dir::Up => ox == HALF && (HALF + 1..=HALF + CORNER).contains(&oy),
        Dir::Down => ox == HALF && (HALF - CORNER..=HALF - 1).contains(&oy),
    }
}

/// Whether `(col, row)` is a tile inside the maze grid.
fn in_bounds(col: i32, row: i32) -> bool {
    (0..COLS as i32).contains(&col) && (0..ROWS as i32).contains(&row)
}

/// Whether `(col, row)` is inside the pen's interior — where the hunters begin and
/// the eater never goes. The seam the pursuit tickets read pen-membership from.
pub fn in_pen(col: i32, row: i32) -> bool {
    (PEN_COLS.0..=PEN_COLS.1).contains(&col) && (PEN_ROWS.0..=PEN_ROWS.1).contains(&row)
}

/// The squared straight-line distance between two tiles — the quantity the pursuit AI
/// minimises. Squared, because ranking needs no square root.
fn tile_dist_sq(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}

/// A fresh hunter of `kind`, centred on its start `tile`, facing `dir`.
fn new_hunter(kind: HunterKind, tile: (usize, usize), dir: Dir, penned: bool) -> HunterState {
    let (x, y) = tile_center(tile.0 as i32, tile.1 as i32);
    HunterState {
        x,
        y,
        dir,
        accum: 0,
        kind,
        penned,
        leaving_pen: false,
        mode: HunterMode::Hunting,
    }
}

/// The personal release threshold for a hunter on a fresh run.
fn release_threshold(kind: HunterKind) -> u32 {
    match kind {
        HunterKind::Canine => 0,
        HunterKind::Incisor => INCISOR_RELEASE_DOTS,
        HunterKind::Wisdom => WISDOM_RELEASE_DOTS,
        HunterKind::Molar => MOLAR_RELEASE_DOTS,
    }
}

/// The tile `n` ahead of the eater's `tile` along `dir`. Facing up it is also `n` to
/// the left — the original's overflow quirk, which both the Incisor (n=4) and the
/// Wisdom's pivot (n=2) inherit.
fn ahead_of_eater(tile: (i32, i32), dir: Dir, n: i32) -> (i32, i32) {
    let (dx, dy) = dir.delta();
    let mut ahead = (tile.0 + n * dx, tile.1 + n * dy);
    if dir == Dir::Up {
        ahead.0 -= n;
    }
    ahead
}

/// Whether `(col, row)` is one of the marked no-up junctions, where a hunter may not
/// choose to turn upward.
fn is_no_up(col: i32, row: i32) -> bool {
    col >= 0 && row >= 0 && NO_UP_TILES.contains(&(col as usize, row as usize))
}

#[cfg(test)]
mod tests {
    //! Board-shape invariants (the maze parses to the right size, is symmetric, holds
    //! 240 dots and four corner power pellets, is fully connected, and has the pen,
    //! gate and tunnel it needs) and the eater's movement (drift, buffered turns,
    //! cornering, the wall stall, tunnel wrap and eating) — planted through the crate
    //! internals; honest play and determinism live in `tests/`.
    use super::*;

    fn glyph(col: usize, row: usize) -> u8 {
        LAYOUT[row].as_bytes()[col]
    }

    #[test]
    fn the_maze_is_28_by_31() {
        assert_eq!(LAYOUT.len(), ROWS);
        for (r, row) in LAYOUT.iter().enumerate() {
            assert_eq!(row.chars().count(), COLS, "row {r} is the wrong width");
        }
    }

    #[test]
    fn the_maze_is_left_right_symmetric() {
        for row in 0..ROWS {
            for col in 0..COLS {
                assert_eq!(
                    glyph(col, row),
                    glyph(COLS - 1 - col, row),
                    "row {row} is not mirrored at column {col}"
                );
            }
        }
    }

    #[test]
    fn there_are_four_power_pellets_one_per_corner() {
        let energizers: Vec<(usize, usize)> = (0..ROWS)
            .flat_map(|r| (0..COLS).map(move |c| (c, r)))
            .filter(|&(c, r)| glyph(c, r) == b'o')
            .collect();
        assert_eq!(energizers.len(), 4, "four power pellets");
        // One in each quadrant.
        let quadrant = |c: usize, r: usize| (c >= COLS / 2, r >= ROWS / 2);
        let mut seen = std::collections::HashSet::new();
        for &(c, r) in &energizers {
            assert!(
                seen.insert(quadrant(c, r)),
                "a quadrant has two power pellets"
            );
        }
        assert_eq!(seen.len(), 4, "every quadrant has one");
    }

    #[test]
    fn every_pickup_is_reachable_from_the_start() {
        // Flood-fill the open tiles (path or gate) from the eater's start; every dot
        // and power pellet must be reached, so nothing is walled off.
        let game = Game::new(1);
        let mut seen = [[false; COLS]; ROWS];
        let start = (EATER_START.0 as i32, EATER_START.1 as i32);
        let mut stack = vec![start];
        seen[start.1 as usize][start.0 as usize] = true;
        while let Some((c, r)) = stack.pop() {
            for dir in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
                let (dc, dr) = dir.delta();
                let (mut nc, nr) = (c + dc, r + dr);
                // The tunnel wraps horizontally on its row.
                if nr == TUNNEL_ROW as i32 {
                    if nc < 0 {
                        nc = COLS as i32 - 1;
                    } else if nc >= COLS as i32 {
                        nc = 0;
                    }
                }
                if nc < 0 || nc >= COLS as i32 || nr < 0 || nr >= ROWS as i32 {
                    continue;
                }
                if game.tile(nc, nr) == Tile::Wall || seen[nr as usize][nc as usize] {
                    continue;
                }
                seen[nr as usize][nc as usize] = true;
                stack.push((nc, nr));
            }
        }
        for (r, seen_row) in seen.iter().enumerate() {
            for (c, &visited) in seen_row.iter().enumerate() {
                if game.pickup(c as i32, r as i32) != Pickup::None {
                    assert!(visited, "the pickup at ({c}, {r}) is walled off");
                }
            }
        }
    }

    #[test]
    fn the_maze_holds_240_dots_and_4_power_pellets() {
        let game = Game::new(1);
        let count = |target: u8| -> u32 {
            (0..ROWS)
                .flat_map(|r| (0..COLS).map(move |c| (c, r)))
                .filter(|&(c, r)| glyph(c, r) == target)
                .count() as u32
        };
        // The original's exact tally — 240 dots and 4 power pellets — on our own layout.
        assert_eq!(count(b'.'), 240, "240 dots");
        assert_eq!(count(b'o'), 4, "4 power pellets");
        assert_eq!(game.pickups_total(), 244, "244 pickups in all");
        assert_eq!(game.pickups_remaining(), 244, "a fresh maze is full");
    }

    #[test]
    fn the_pen_has_a_gate_and_an_enclosed_interior() {
        let game = Game::new(1);
        let gates: Vec<(i32, i32)> = (0..ROWS)
            .flat_map(|r| (0..COLS).map(move |c| (c as i32, r as i32)))
            .filter(|&(c, r)| game.tile(c, r) == Tile::Gate)
            .collect();
        assert!(!gates.is_empty(), "the pen has a gate");
        // Every pen-interior tile is open path carrying no pickup, and `in_pen`
        // reports it — the seam the hunter tickets read pen-membership from.
        for row in PEN_ROWS.0..=PEN_ROWS.1 {
            for col in PEN_COLS.0..=PEN_COLS.1 {
                assert!(in_pen(col, row), "({col}, {row}) is inside the pen");
                assert_eq!(game.tile(col, row), Tile::Path, "the pen interior is open");
                assert_eq!(game.pickup(col, row), Pickup::None, "the pen holds no dots");
            }
        }
        // A corridor tile outside the pen is not reported as pen.
        assert!(!in_pen(EATER_START.0 as i32, EATER_START.1 as i32));
    }

    #[test]
    fn the_tunnel_row_is_the_open_dotless_crossing() {
        let game = Game::new(1);
        assert_eq!(game.tile(-1, TUNNEL_ROW as i32), Tile::Path);
        assert_eq!(game.tile(COLS as i32, TUNNEL_ROW as i32), Tile::Path);
        // Off the tunnel row, out-of-bounds is solid.
        assert_eq!(game.tile(-1, 5), Tile::Wall);
        // The tunnel row itself carries no pickups — so redrawing the maze can't
        // silently drop dots into the crossing.
        for col in 0..COLS as i32 {
            assert_eq!(
                game.pickup(col, TUNNEL_ROW as i32),
                Pickup::None,
                "the tunnel row is dotless at column {col}"
            );
        }
    }

    #[test]
    fn the_no_up_tiles_are_real_up_junctions() {
        let game = Game::new(1);
        for (c, r) in NO_UP_TILES {
            assert_ne!(
                game.tile(c as i32, r as i32),
                Tile::Wall,
                "no-up tile ({c}, {r}) must be a corridor"
            );
            assert_ne!(
                game.tile(c as i32, r as i32 - 1),
                Tile::Wall,
                "no-up tile ({c}, {r}) must have an open tile above to forbid"
            );
        }
    }

    #[test]
    fn the_eater_start_is_an_empty_corridor() {
        let game = Game::new(1);
        let (c, r) = (EATER_START.0 as i32, EATER_START.1 as i32);
        assert_eq!(game.tile(c, r), Tile::Path);
        assert_eq!(game.pickup(c, r), Pickup::None, "the start carries no dot");
    }

    #[test]
    fn the_board_advances_deterministically() {
        let mut a = Game::new(7);
        let mut b = Game::new(7);
        for _ in 0..600 {
            assert_eq!(a.step(Input::default()), b.step(Input::default()));
        }
        assert_eq!(a.pickups_remaining(), b.pickups_remaining());
    }

    /// Centres the eater on `(col, row)` facing `dir`, ready to move on the next step.
    fn plant_eater(game: &mut Game, col: i32, row: i32, dir: Dir) {
        let (x, y) = tile_center(col, row);
        game.eater = MoverState {
            x,
            y,
            dir,
            want: None,
            turning: None,
            // Primed so the first step spends a pixel (and so eats, turns or moves)
            // rather than only banking speed.
            accum: SPEED_DEN,
            stall: 0,
        };
    }

    fn press(dir: Dir) -> Input {
        match dir {
            Dir::Up => Input {
                up: true,
                ..Default::default()
            },
            Dir::Down => Input {
                down: true,
                ..Default::default()
            },
            Dir::Left => Input {
                left: true,
                ..Default::default()
            },
            Dir::Right => Input {
                right: true,
                ..Default::default()
            },
        }
    }

    #[test]
    fn the_eater_drifts_along_its_facing_and_eats() {
        let mut game = Game::new(1);
        let start_x = game.eater().x;
        for _ in 0..100 {
            game.step(Input::default());
        }
        assert!(game.eater().x < start_x, "it drifts left along its facing");
        assert!(
            game.score() >= 30,
            "and eats the dots it passes, got {}",
            game.score()
        );
    }

    #[test]
    fn a_buffered_turn_is_taken_at_the_first_opening() {
        // Heading left along row 23, up is walled until column 6. Buffer the turn now;
        // it must be remembered and taken at that first opening.
        let mut game = Game::new(1);
        plant_eater(&mut game, 10, 23, Dir::Left);
        for _ in 0..140 {
            game.step(press(Dir::Up));
        }
        assert_eq!(game.eater().dir, Dir::Up, "the buffered turn is honoured");
        let (col, _) = tile_at(game.eater().x, game.eater().y);
        assert_eq!(col, 6, "up the first open corridor");
    }

    #[test]
    fn the_eater_stalls_against_a_wall() {
        // Column 5 of row 23 is a wall; pressed into it, the eater holds at the centre.
        let mut game = Game::new(1);
        plant_eater(&mut game, 6, 23, Dir::Left);
        let x0 = game.eater().x;
        for _ in 0..20 {
            game.step(press(Dir::Left));
        }
        assert_eq!(
            game.eater().x,
            x0,
            "a wall holds the eater at the tile centre"
        );
    }

    #[test]
    fn the_tunnel_wraps_the_eater() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 2, TUNNEL_ROW as i32, Dir::Left);
        for _ in 0..60 {
            game.step(press(Dir::Left));
        }
        let (col, _) = tile_at(game.eater().x, game.eater().y);
        assert!(
            col > 14,
            "leaving the left tunnel re-enters on the right, got col {col}"
        );
    }

    #[test]
    fn eating_a_dot_scores_ten_and_stalls_a_frame() {
        let mut game = Game::new(1);
        let remaining = game.pickups_remaining();
        plant_eater(&mut game, 12, 23, Dir::Left); // (12, 23) carries a dot
        let events = game.step(Input::default());
        assert!(events.dot_eaten);
        assert_eq!(game.score(), 10);
        assert_eq!(game.pickups_remaining(), remaining - 1);

        // The dot's one-frame stall freezes it, then it moves again.
        let frozen = game.eater();
        game.step(Input::default());
        assert_eq!(game.eater(), frozen, "a dot freezes the eater for a frame");
        for _ in 0..3 {
            game.step(Input::default());
        }
        assert_ne!(game.eater(), frozen, "then it feeds on and moves");
    }

    #[test]
    fn eating_a_power_pellet_scores_fifty_and_stalls_longer() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 1, 23, Dir::Right); // (1, 23) carries a power pellet
        let events = game.step(Input::default());
        assert!(events.power_pellet_eaten);
        assert_eq!(game.score(), 50);

        // The pellet's three-frame stall freezes it longer than a dot would.
        let frozen = game.eater();
        for _ in 0..3 {
            game.step(Input::default());
            assert_eq!(
                game.eater(),
                frozen,
                "the pellet freezes the eater three frames"
            );
        }
        game.step(Input::default());
        assert_ne!(game.eater(), frozen, "then it moves on");
    }

    #[test]
    fn clearing_the_last_pickup_raises_maze_cleared() {
        let mut game = Game::new(1);
        for row in &mut game.maze.pickups {
            for cell in row {
                *cell = Pickup::None;
            }
        }
        game.maze.pickups[23][12] = Pickup::Dot;
        game.maze.remaining = 1;
        plant_eater(&mut game, 12, 23, Dir::Left);
        let events = game.step(Input::default());
        assert!(events.maze_cleared, "the last pickup clears the maze");
        assert_eq!(game.pickups_remaining(), 0);
    }

    #[test]
    fn cornering_cuts_into_the_new_corridor_early() {
        // Approaching the centre of (6, 20) heading right, with up open there and the
        // turn buffered. Cornering eases the eater upward before it fully centres.
        let mut game = Game::new(1);
        game.maze.pickups[20][6] = Pickup::None; // clear the corner dot so it doesn't interrupt
        let (cx, cy) = tile_center(6, 20);
        game.eater = MoverState {
            x: cx - 2, // two pixels shy of the centre — inside the cornering window
            y: cy,
            dir: Dir::Right,
            want: Some(Dir::Up),
            turning: None,
            accum: SPEED_DEN,
            stall: 0,
        };
        game.step(Input::default());
        assert!(
            game.eater().y < cy,
            "the eater eases up before the centre — a corner-cut"
        );
        assert_eq!(
            game.eater().dir,
            Dir::Right,
            "still easing through the corner"
        );

        for _ in 0..4 {
            game.step(Input::default());
        }
        assert_eq!(
            game.eater().dir,
            Dir::Up,
            "the corner completes onto the new heading"
        );
        assert_eq!(
            game.eater().x,
            cx,
            "and the eater is aligned in the new corridor"
        );
    }

    /// Replaces the hunters with a single loose Canine centred on `(col, row)` facing
    /// `dir`, primed to move on the next step.
    fn plant_hunter(game: &mut Game, col: i32, row: i32, dir: Dir) {
        let (x, y) = tile_center(col, row);
        game.hunters = vec![HunterState {
            x,
            y,
            dir,
            accum: SPEED_DEN,
            kind: HunterKind::Canine,
            penned: false,
            leaving_pen: false,
            mode: HunterMode::Hunting,
        }];
        game.caught = false;
    }

    #[test]
    fn the_canine_runs_the_eater_down() {
        // The eater, stalled against the wall left of (6, 23), sits still; the Canine,
        // planted three tiles up the same corridor, must chase down and catch it.
        let mut game = Game::new(1);
        game.hunt_phase = HuntPhase::Chase;
        plant_eater(&mut game, 6, 23, Dir::Left);
        plant_hunter(&mut game, 6, 20, Dir::Down);
        let mut caught = false;
        for _ in 0..200 {
            caught |= game.step(Input::default()).life_lost;
        }
        assert!(caught, "the Canine closes on and catches the eater");
        assert!(game.caught());
    }

    #[test]
    fn a_hunter_never_reverses() {
        // Over a long chase the hunter only ever turns at right angles or holds on —
        // it never flips to its opposite heading.
        let mut game = Game::new(2);
        game.hunt_phase = HuntPhase::Chase;
        game.hunt_phase_index = 1;
        game.hunt_phase_frames = 0;
        let mut prev = game.hunters[0].dir;
        for _ in 0..1000 {
            game.step(Input::default());
            let now = game.hunters[0].dir;
            assert_ne!(now, prev.opposite(), "a hunter never reverses on the spot");
            prev = now;
        }
    }

    #[test]
    fn a_hunter_will_not_turn_up_at_a_no_up_junction() {
        // On the no-up tile (12, 11), with the eater straight above, up is the nearest
        // exit — but forbidden here, so the hunter takes the next-best instead.
        let mut game = Game::new(1);
        plant_eater(&mut game, 12, 3, Dir::Left); // parked directly above the junction
        plant_hunter(&mut game, 12, 11, Dir::Left);
        assert!(
            is_no_up(12, 11),
            "the fixture tile really is a no-up junction"
        );
        game.step(Input::default());
        assert_ne!(
            game.hunters[0].dir,
            Dir::Up,
            "no hunter turns up at a no-up junction"
        );
    }

    #[test]
    fn a_catch_costs_a_life() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 13, 23, Dir::Left);
        plant_hunter(&mut game, 13, 23, Dir::Left); // planted on the eater's tile
        let events = game.step(Input::default());
        assert!(events.life_lost, "sharing the eater's tile is a catch");
        assert!(game.caught(), "and the catch is latched");
    }

    #[test]
    fn the_incisor_aims_four_ahead() {
        let mut game = Game::new(1);
        game.hunt_phase = HuntPhase::Chase;
        plant_eater(&mut game, 10, 20, Dir::Right);
        assert_eq!(game.hunters[1].kind, HunterKind::Incisor);
        assert_eq!(
            game.hunter_target(1),
            (14, 20),
            "four tiles ahead of the right-facing eater"
        );
    }

    #[test]
    fn the_incisor_up_quirk_aims_ahead_and_aside() {
        let mut game = Game::new(1);
        game.hunt_phase = HuntPhase::Chase;
        plant_eater(&mut game, 10, 20, Dir::Up);
        assert_eq!(
            game.hunter_target(1),
            (6, 16),
            "facing up: four ahead and four to the left, the original's overflow"
        );
    }

    #[test]
    fn the_wisdom_pincers_off_the_canine() {
        let mut game = Game::new(1);
        game.hunt_phase = HuntPhase::Chase;
        plant_eater(&mut game, 10, 20, Dir::Right);
        // Park the Canine (hunters[0]) at a known tile the Wisdom steers off.
        (game.hunters[0].x, game.hunters[0].y) = tile_center(10, 10);
        assert_eq!(game.hunters[2].kind, HunterKind::Wisdom);
        // pivot = two ahead of the eater = (12, 20); target = 2*pivot - canine = (14, 30).
        assert_eq!(game.hunter_target(2), (14, 30));
    }

    #[test]
    fn the_molar_chases_when_far_and_flees_when_near() {
        let mut game = Game::new(1);
        game.hunt_phase = HuntPhase::Chase;
        plant_eater(&mut game, 10, 20, Dir::Left);
        assert_eq!(game.hunters[3].kind, HunterKind::Molar);
        // Fifteen tiles up — far — so it targets the eater.
        (game.hunters[3].x, game.hunters[3].y) = tile_center(10, 5);
        assert_eq!(
            game.hunter_target(3),
            (10, 20),
            "far: the Molar targets the eater"
        );
        // On the eater's tile — near — so it breaks for its own corner.
        (game.hunters[3].x, game.hunters[3].y) = tile_center(10, 20);
        assert_eq!(
            game.hunter_target(3),
            HunterKind::Molar.scatter_corner(),
            "near: the Molar breaks for its corner"
        );
    }

    #[test]
    fn each_mind_has_a_distinct_scatter_corner() {
        let corners = [
            HunterKind::Canine.scatter_corner(),
            HunterKind::Incisor.scatter_corner(),
            HunterKind::Wisdom.scatter_corner(),
            HunterKind::Molar.scatter_corner(),
        ];
        for (i, ci) in corners.iter().enumerate() {
            for cj in &corners[i + 1..] {
                assert_ne!(ci, cj, "the four scatter corners are distinct");
            }
        }
    }

    #[test]
    fn the_penned_hunters_release_on_personal_thresholds() {
        let mut game = Game::new(1);
        let positions = |g: &Game| -> Vec<(i32, i32)> {
            g.hunters()
                .filter(|h| h.kind != HunterKind::Canine)
                .map(|h| (h.x, h.y))
                .collect()
        };
        assert_eq!(game.penned_hunters(), 3);
        let first = (0..300)
            .map(|_| game.step(Input::default()))
            .find_map(|events| events.hunter_released);
        assert_eq!(first, Some(HunterKind::Incisor));
        assert_eq!(game.penned_hunters(), 2);
        assert!(
            positions(&game).len() == 3,
            "all hunters remain observable after the first release"
        );
    }

    #[test]
    fn the_hunt_switches_from_scatter_to_chase_on_schedule() {
        let mut game = Game::new(1);
        assert_eq!(game.hunt_phase(), HuntPhase::Scatter);
        for _ in 0..(7 * 60 - 1) {
            assert!(!game.step(Input::default()).hunt_phase_changed);
        }
        let events = game.step(Input::default());
        assert!(events.hunt_phase_changed);
        assert_eq!(game.hunt_phase(), HuntPhase::Chase);
        assert_eq!(game.hunt_phase_frames(), 0);
    }

    #[test]
    fn a_phase_switch_reverses_each_active_hunter_once() {
        let mut game = Game::new(1);
        game.hunters[0].x = tile_center(6, 20).0;
        game.hunters[0].y = tile_center(6, 20).1;
        game.hunters[0].dir = Dir::Left;
        game.hunt_phase_frames = 7 * 60 - 1;
        let before = game.hunters[0].dir;
        let mut events = Events::default();
        game.advance_hunt_schedule(&mut events);
        assert!(events.hunt_phase_changed);
        assert_eq!(game.hunt_phase(), HuntPhase::Chase);
        assert_eq!(game.hunters[0].dir, before.opposite());
    }

    #[test]
    fn the_schedule_tiers_follow_the_original() {
        // Tier selection: 1 / 2–4 / 5+.
        assert_eq!(hunt_schedule(1), &HUNT_SCHEDULE_L1);
        assert_eq!(hunt_schedule(2), &HUNT_SCHEDULE_L2_4);
        assert_eq!(hunt_schedule(4), &HUNT_SCHEDULE_L2_4);
        assert_eq!(hunt_schedule(5), &HUNT_SCHEDULE_L5);
        assert_eq!(hunt_schedule(99), &HUNT_SCHEDULE_L5);
        // The signature values, pinned against the original's tables rather than
        // the consts themselves: the openers, the marathon third chases, the
        // one-frame deep scatters, and the chase that never expires.
        assert_eq!(hunt_schedule(1)[0], (HuntPhase::Scatter, 7 * 60));
        assert_eq!(hunt_schedule(1)[5], (HuntPhase::Chase, 20 * 60));
        assert_eq!(hunt_schedule(2)[5], (HuntPhase::Chase, 1033 * 60));
        assert_eq!(hunt_schedule(2)[6], (HuntPhase::Scatter, 1));
        assert_eq!(hunt_schedule(5)[0], (HuntPhase::Scatter, 5 * 60));
        assert_eq!(hunt_schedule(5)[5], (HuntPhase::Chase, 1037 * 60));
        assert_eq!(hunt_schedule(5)[6], (HuntPhase::Scatter, 1));
        for level in [1, 2, 5] {
            assert_eq!(hunt_schedule(level)[7], (HuntPhase::Chase, u32::MAX));
        }
    }

    #[test]
    fn deep_levels_open_with_the_shorter_scatter() {
        let mut game = Game::new(1);
        game.level = 5;
        // At level 5 the first scatter runs five seconds, not seven: the switch
        // lands on frame 300, and frame 299 is still scatter.
        game.hunt_phase_frames = 5 * 60 - 2;
        let mut events = Events::default();
        game.advance_hunt_schedule(&mut events);
        assert!(!events.hunt_phase_changed);
        game.advance_hunt_schedule(&mut events);
        assert!(events.hunt_phase_changed);
        assert_eq!(game.hunt_phase(), HuntPhase::Chase);
    }

    #[test]
    fn the_one_frame_scatter_is_a_bare_reversal() {
        // Deep tiers shrink the last scatter to a single frame — the original's
        // quirk where the hunters reverse without ever visibly relaxing.
        let mut game = Game::new(1);
        game.level = 2;
        game.hunt_phase_index = 6;
        game.hunt_phase = HuntPhase::Scatter;
        game.hunt_phase_frames = 0;
        let before = game.hunters[0].dir;
        let mut events = Events::default();
        game.advance_hunt_schedule(&mut events);
        assert!(events.hunt_phase_changed, "one frame in, scatter is over");
        assert_eq!(game.hunt_phase(), HuntPhase::Chase);
        assert_eq!(game.hunters[0].dir, before.opposite());
    }

    #[test]
    fn a_released_hunter_exits_through_the_gate() {
        let mut game = Game::new(1);
        let event = game.step(Input::default());
        assert_eq!(event.hunter_released, Some(HunterKind::Incisor));
        for _ in 0..240 {
            game.step(Input::default());
        }
        let incisor = game.hunters().nth(1).unwrap();
        let tile = tile_at(incisor.x, incisor.y);
        assert!(!incisor.penned);
        assert!(!in_pen(tile.0, tile.1), "the Incisor cleared the pen");
    }

    #[test]
    fn the_release_timeout_forces_the_next_hunter() {
        let mut game = Game::new(1);
        for row in &mut game.maze.pickups {
            for pickup in row {
                *pickup = Pickup::None;
            }
        }
        game.maze.remaining = 0;
        game.caught = true; // keep this release-only scenario free of contact noise
        assert_eq!(
            game.step(Input::default()).hunter_released,
            Some(HunterKind::Incisor)
        );
        let mut released = None;
        for _ in 0..RELEASE_TIMEOUT_FRAMES {
            released = game.step(Input::default()).hunter_released;
            if released.is_some() {
                break;
            }
        }
        assert_eq!(released, Some(HunterKind::Wisdom));
    }

    #[test]
    fn a_death_switches_pen_release_to_the_global_counter() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 13, 23, Dir::Left);
        plant_hunter(&mut game, 13, 23, Dir::Left);
        game.hunters.push(new_hunter(
            HunterKind::Incisor,
            INCISOR_START,
            Dir::Down,
            true,
        ));
        assert!(game.step(Input::default()).life_lost);
        assert!(game.global_release);
        game.caught = true;
        game.maze.remaining = 0;
        for row in &mut game.maze.pickups {
            for pickup in row {
                *pickup = Pickup::None;
            }
        }
        game.post_death_pickups = GLOBAL_RELEASE_DOTS - 1;
        game.eater.x = tile_center(1, 23).0;
        game.eater.y = tile_center(1, 23).1;
        game.eater.dir = Dir::Left;
        game.eater.accum = SPEED_DEN;
        game.maze.pickups[23][1] = Pickup::Dot;
        game.maze.remaining = 1;
        assert!(game.step(Input::default()).dot_eaten);
        assert_eq!(game.post_death_pickups, 0);
        assert_eq!(game.hunters.iter().filter(|h| h.penned).count(), 0);
    }

    #[test]
    fn a_released_mind_moves_and_replays_identically() {
        // T6 releases the penned minds on a schedule; here we un-pen the Incisor
        // directly to exercise its targeting driving movement through the step seam,
        // and confirm it replays identically across two runs on one seed.
        let run = || {
            let mut game = Game::new(9);
            game.hunters[1].penned = false; // the Incisor
            let mut path = Vec::new();
            for _ in 0..1500 {
                game.step(Input::default());
                let a = game.hunters().nth(1).expect("the Incisor");
                path.push((a.x, a.y));
            }
            path
        };
        let a = run();
        let b = run();
        assert_eq!(a, b, "a released mind replays identically");
        let distinct = a.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(
            distinct > 1,
            "its targeting drove it to move, visiting {distinct} tiles"
        );
    }

    /// Plants a power pellet on the eater's tile and steps until it is eaten,
    /// so a test can trigger the flip mid-scene without steering to a corner.
    /// (A few steps may pass first if the eater is still stalled from feeding.)
    fn feed_pellet(game: &mut Game) -> Events {
        let (tc, tr) = tile_at(game.eater.x, game.eater.y);
        if game.maze.pickups[tr as usize][tc as usize] == Pickup::None {
            game.maze.remaining += 1;
        }
        game.maze.pickups[tr as usize][tc as usize] = Pickup::PowerPellet;
        for _ in 0..10 {
            let events = game.step(Input::default());
            if events.power_pellet_eaten {
                return events;
            }
        }
        panic!("the planted pellet was never eaten");
    }

    #[test]
    fn a_power_pellet_flips_the_loose_hunters() {
        let mut game = Game::new(3);
        // Plant the pellet by hand so the pre-flip heading can be read on the very
        // frame it is eaten, however many stalled frames precede it.
        let (tc, tr) = tile_at(game.eater.x, game.eater.y);
        game.maze.pickups[tr as usize][tc as usize] = Pickup::PowerPellet;
        game.maze.remaining += 1;
        let mut before = game.hunters[0].dir;
        let mut events = Events::default();
        for _ in 0..10 {
            before = game.hunters[0].dir;
            events = game.step(Input::default());
            if events.power_pellet_eaten {
                break;
            }
        }
        assert!(events.power_pellet_eaten);
        assert!(events.frightened_started);
        assert_eq!(game.hunters[0].mode, HunterMode::Frightened);
        assert_eq!(game.hunters[0].dir, before.opposite(), "the flip reverses");
        assert!(
            game.hunters[3].penned && game.hunters[3].mode == HunterMode::Hunting,
            "a penned hunter is not frightened"
        );
        assert!(game.frightened_frames_left() > 0);
        let clock = game.hunt_phase_frames();
        for _ in 0..5 {
            game.step(Input::default());
        }
        assert_eq!(
            game.hunt_phase_frames(),
            clock,
            "the scatter/chase clock holds its breath"
        );
    }

    #[test]
    fn catches_score_the_doubling_ladder_and_reset_on_the_next_pellet() {
        let mut game = Game::new(3);
        feed_pellet(&mut game);
        let eater_tile = tile_at(game.eater.x, game.eater.y);
        let mut scores = Vec::new();
        for _ in 0..4 {
            // Park a fresh frightened hunter on the eater and resolve the touch.
            plant_hunter(&mut game, eater_tile.0, eater_tile.1, Dir::Left);
            game.hunters[0].mode = HunterMode::Frightened;
            let mut events = Events::default();
            game.resolve_contact(&mut events);
            let (_, score) = events.hunter_caught.expect("a catch");
            scores.push(score);
        }
        assert_eq!(scores, vec![200, 400, 800, 1600]);
        // The next pellet resets the ladder — and leaves the caught one as eyes.
        feed_pellet(&mut game);
        assert_eq!(
            game.hunters[0].mode,
            HunterMode::Eyes,
            "a second pellet does not re-frighten eyes"
        );
        plant_hunter(&mut game, eater_tile.0, eater_tile.1, Dir::Left);
        game.hunters[0].mode = HunterMode::Frightened;
        let mut events = Events::default();
        game.resolve_contact(&mut events);
        assert_eq!(events.hunter_caught, Some((HunterKind::Canine, 200)));
    }

    #[test]
    fn frightened_and_eyes_hunters_do_not_harm_the_eater() {
        let mut game = Game::new(3);
        let eater_tile = tile_at(game.eater.x, game.eater.y);
        plant_hunter(&mut game, eater_tile.0, eater_tile.1, Dir::Left);
        game.hunters[0].mode = HunterMode::Frightened;
        let mut events = Events::default();
        game.resolve_contact(&mut events);
        assert!(!events.life_lost, "a frightened hunter is prey, not peril");
        assert_eq!(game.hunters[0].mode, HunterMode::Eyes, "and is now eyes");
        let mut events = Events::default();
        game.resolve_contact(&mut events);
        assert!(!events.life_lost, "eyes pass straight through");
        assert_eq!(events.hunter_caught, None);
    }

    #[test]
    fn eyes_race_home_regenerate_and_re_release() {
        let mut game = Game::new(3);
        // Turn the loose Canine to eyes out on the maze.
        game.hunters[0].mode = HunterMode::Eyes;
        let mut regenerated = false;
        let mut re_released = false;
        for _ in 0..1800 {
            let events = game.step(Input::default());
            if events.hunter_regenerated == Some(HunterKind::Canine) {
                regenerated = true;
                assert_eq!(game.hunters[0].mode, HunterMode::Hunting);
                // The release can fire in this same step — the Canine's threshold
                // is zero, so it barely pauses in the pen. Faithful, not a bug.
            }
            if regenerated && events.hunter_released == Some(HunterKind::Canine) {
                re_released = true;
                break;
            }
        }
        assert!(regenerated, "the eyes made it home");
        assert!(re_released, "and the release rules let the hunter back out");
    }

    #[test]
    fn the_window_expires_back_into_the_schedule() {
        let mut game = Game::new(3);
        feed_pellet(&mut game);
        game.frightened_frames = 2;
        let events = game.step(Input::default());
        assert!(!events.frightened_ended);
        let clock = game.hunt_phase_frames();
        let events = game.step(Input::default());
        assert!(events.frightened_ended);
        assert_eq!(game.hunters[0].mode, HunterMode::Hunting, "blue is over");
        game.step(Input::default());
        assert!(
            game.hunt_phase_frames() > clock,
            "the scatter/chase clock breathes again"
        );
    }

    #[test]
    fn a_live_hunter_never_walks_back_through_the_gate() {
        // Park hunters at the tile straight above the gate, where Down leads into
        // the pen. Neither a hunting nor a wandering hunter may take it — only
        // eyes and leavers have gate business. Pins the through_gate rule.
        for frightened in [false, true] {
            let mut game = Game::new(7);
            plant_hunter(&mut game, 13, 11, Dir::Right);
            if frightened {
                game.hunters[0].mode = HunterMode::Frightened;
                game.frightened_frames = u32::MAX;
            }
            for _ in 0..600 {
                game.step(Input::default());
                let hunter = game.hunters().next().expect("the hunter");
                let tile = tile_at(hunter.x, hunter.y);
                assert!(
                    !in_pen(tile.0, tile.1) && game.tile(tile.0, tile.1) != Tile::Gate,
                    "a live hunter (frightened: {frightened}) re-entered the pen"
                );
            }
        }
    }

    #[test]
    fn the_fright_table_follows_the_original() {
        assert_eq!(fright_for(1), (6 * 60, 5));
        assert_eq!(fright_for(9), (60, 3));
        assert_eq!(fright_for(14), (3 * 60, 5));
        assert_eq!(fright_for(17), (0, 0));
        assert_eq!(fright_for(18), (60, 3), "the one late reprieve");
        assert_eq!(fright_for(19), (0, 0));
        assert_eq!(fright_for(255), (0, 0));
    }

    #[test]
    fn deep_levels_get_no_blue_time_at_all() {
        let mut game = Game::new(3);
        game.level = 17;
        let score = game.score();
        let events = feed_pellet(&mut game);
        assert!(events.power_pellet_eaten, "the pellet is still eaten");
        assert!(!events.frightened_started, "but buys no blue time");
        assert_eq!(game.hunters[0].mode, HunterMode::Hunting);
        assert_eq!(game.score(), score + POWER_PELLET_SCORE);
        assert_eq!(game.frightened_frames_left(), 0);
    }

    #[test]
    fn a_frightened_wander_stays_legal_and_replays() {
        let run = || {
            let mut game = Game::new(11);
            game.hunters[0].mode = HunterMode::Frightened;
            game.frightened_frames = u32::MAX; // hold the window open artificially
            let mut path = Vec::new();
            for _ in 0..2000 {
                game.step(Input::default());
                let hunter = game.hunters().next().expect("the Canine");
                let tile = tile_at(hunter.x, hunter.y);
                assert_ne!(
                    game.tile(tile.0, tile.1),
                    Tile::Wall,
                    "a wandering hunter never leaves the corridors"
                );
                path.push((hunter.x, hunter.y));
            }
            path
        };
        assert_eq!(run(), run(), "the wander is seeded, so it replays");
    }
}
