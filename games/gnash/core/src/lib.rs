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
//! the **Shadow**, the direct chaser — navigates the maze by the original's
//! one-tile-lookahead rule: at each tile centre it takes the exit whose next tile is
//! nearest its **target** (never reversing, ties broken up-left-down-right, and never
//! turning up at the marked junctions), targeting the eater's own tile. Contact costs
//! a life.
//!
//! The **four minds** ([T5](https://github.com/geox123/minigames/issues/163)) give
//! the hunt its character: the **Shadow** targets the eater's tile; the **Ambusher**
//! four tiles ahead of its facing (with the original's up-facing overflow quirk); the
//! **Fickle** a point pincering off the Shadow (doubled through two-ahead); the
//! **Shy** the eater when far but its own corner when within eight tiles. Each also
//! has a scatter corner it heads for. Only the Shadow is loose so far — the other
//! three wait **penned** until the release-and-schedule ticket that follows lets them
//! out and starts the scatter/chase cycle.

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

/// The Shadow's start tile — just outside the pen, above the gate, where the direct
/// chaser begins already loose on the maze (the staggered pen release of the other
/// hunters is a later ticket). It heads left from here.
pub const HUNTER_START: (usize, usize) = (13, 11);
/// The other three hunters' start tiles, inside the pen, where they wait until the
/// release ticket lets them out.
pub const AMBUSHER_START: (usize, usize) = (13, 14);
pub const FICKLE_START: (usize, usize) = (11, 14);
pub const SHY_START: (usize, usize) = (16, 14);
/// A hunter's speed at level 1, as a percentage of the base rate — a touch under the
/// eater's, so a clean run stays ahead. (Per-level speeds are a later ticket.)
const HUNTER_SPEED: i32 = 75;
/// A hunter's speed while crossing the tunnel — it crawls there, the original's
/// let-off that a cornered player can exploit.
const HUNTER_TUNNEL_SPEED: i32 = 40;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HunterKind {
    /// Targets the eater's own tile — the relentless direct chaser.
    Shadow,
    /// Targets four tiles ahead of the eater — cutting off where it is going.
    Ambusher,
    /// Targets a point pincering off the Shadow — swinging wildly as the pair move.
    Fickle,
    /// Targets the eater when far, but breaks for its own corner when within eight
    /// tiles — it lopes in, loses nerve, and comes again.
    Shy,
}

/// A hunter, as the shell should draw it: its centre in logical pixels, the way it
/// heads (which the shell points its eyes along), and which mind it is (its colour).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hunter {
    pub x: i32,
    pub y: i32,
    pub dir: Dir,
    pub kind: HunterKind,
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
    /// The eater ate a power pellet this step (a later ticket flips the hunt on it).
    pub power_pellet_eaten: bool,
    /// The maze was cleared of every pickup this step (a later ticket advances the
    /// level on it).
    pub maze_cleared: bool,
    /// A hunter caught the eater this step (lives and respawn are a later ticket).
    pub life_lost: bool,
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
    /// The four hunters. The Shadow starts loose; the other three wait penned until
    /// the release ticket lets them out.
    hunters: Vec<HunterState>,
    /// Whether a hunter has caught the eater (latched; lives and respawn are a later
    /// ticket).
    caught: bool,
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
        let hunters = vec![
            new_hunter(HunterKind::Shadow, HUNTER_START, Dir::Left, false),
            new_hunter(HunterKind::Ambusher, AMBUSHER_START, Dir::Down, true),
            new_hunter(HunterKind::Fickle, FICKLE_START, Dir::Up, true),
            new_hunter(HunterKind::Shy, SHY_START, Dir::Up, true),
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
        if let Some(d) = input.dir() {
            self.eater.want = Some(d);
        }
        self.advance_eater(&mut events);
        // The original checks a catch both after the eater moves and after the
        // hunters move, so a head-on pass counts as a catch either way.
        self.resolve_contact(&mut events);
        self.advance_hunters();
        self.resolve_contact(&mut events);
        events
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
    /// still until the release ticket lets them out.
    fn advance_hunters(&mut self) {
        for i in 0..self.hunters.len() {
            if self.hunters[i].penned {
                continue;
            }
            let target = self.hunter_target(i);
            self.advance_hunter(i, target);
        }
    }

    /// The tile hunter `i` steers toward, by its mind. (In scatter mode a later ticket
    /// will send each to its [`scatter_corner`] instead.)
    fn hunter_target(&self, i: usize) -> (i32, i32) {
        let hunter = self.hunters[i];
        let eater = tile_at(self.eater.x, self.eater.y);
        match hunter.kind {
            // The Shadow bears straight down on the eater.
            HunterKind::Shadow => eater,
            // The Ambusher aims four tiles ahead of where the eater is heading.
            HunterKind::Ambusher => ahead_of_eater(eater, self.eater.dir, 4),
            // The Fickle doubles the vector from the Shadow through the point two ahead
            // of the eater — a pincer that swings as the pair move.
            HunterKind::Fickle => {
                let pivot = ahead_of_eater(eater, self.eater.dir, 2);
                let shadow = self.shadow_tile();
                (2 * pivot.0 - shadow.0, 2 * pivot.1 - shadow.1)
            }
            // The Shy chases while far, but breaks for its corner within eight tiles.
            HunterKind::Shy => {
                let own = tile_at(hunter.x, hunter.y);
                if tile_dist_sq(own, eater) > 8 * 8 {
                    eater
                } else {
                    scatter_corner(HunterKind::Shy)
                }
            }
        }
    }

    /// The Shadow's current tile — the Fickle steers off it. Falls back to the eater's
    /// tile if somehow no Shadow is present.
    fn shadow_tile(&self) -> (i32, i32) {
        self.hunters
            .iter()
            .find(|h| h.kind == HunterKind::Shadow)
            .map_or_else(
                || tile_at(self.eater.x, self.eater.y),
                |h| tile_at(h.x, h.y),
            )
    }

    /// Advances one hunter a frame: spend its fractional-speed budget — a crawl while
    /// crossing the tunnel — one pixel at a time.
    fn advance_hunter(&mut self, i: usize, target: (i32, i32)) {
        let (_, row) = tile_at(self.hunters[i].x, self.hunters[i].y);
        let speed = if row == TUNNEL_ROW as i32 {
            HUNTER_TUNNEL_SPEED
        } else {
            HUNTER_SPEED
        };
        self.hunters[i].accum += speed;
        while self.hunters[i].accum >= SPEED_DEN {
            self.hunters[i].accum -= SPEED_DEN;
            self.step_hunter_pixel(i, target);
        }
    }

    /// Moves one hunter a single pixel: at a tile centre it picks the exit nearest its
    /// target, then it steps along its heading, wrapping through the tunnel.
    fn step_hunter_pixel(&mut self, i: usize, target: (i32, i32)) {
        let (tc, tr) = tile_at(self.hunters[i].x, self.hunters[i].y);
        let ox = self.hunters[i].x.rem_euclid(TILE);
        let oy = self.hunters[i].y.rem_euclid(TILE);
        if ox == HALF && oy == HALF {
            self.hunters[i].dir = self.choose_hunter_dir(tc, tr, self.hunters[i].dir, target);
        }
        (self.hunters[i].x, self.hunters[i].y) =
            step_pixel(self.hunters[i].x, self.hunters[i].y, self.hunters[i].dir);
    }

    /// Chooses a hunter's heading out of tile `(tc, tr)`: among the open exits — never
    /// the reverse of `current`, never up at a no-up junction — the one whose next tile
    /// is nearest `target` in straight-line distance, ties broken up → left → down →
    /// right. A dead end (no exit) forces a reversal.
    fn choose_hunter_dir(&self, tc: i32, tr: i32, current: Dir, target: (i32, i32)) -> Dir {
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
            if !self.hunter_can_enter(nc, nr) {
                continue;
            }
            let dist = tile_dist_sq((nc, nr), target);
            if best.is_none_or(|(_, b)| dist < b) {
                best = Some((dir, dist));
            }
        }
        best.map_or(reverse, |(dir, _)| dir)
    }

    /// Whether a hunter may enter tile `(col, row)` — an open corridor. The gate and
    /// pen interior open to hunters only in the release/eyes ticket that follows; for
    /// now the Shadow, already loose, keeps to the corridors.
    fn hunter_can_enter(&self, col: i32, row: i32) -> bool {
        self.maze.tile(col, row) == Tile::Path
    }

    /// Flags a catch if any loose hunter shares the eater's tile.
    fn resolve_contact(&mut self, events: &mut Events) {
        let eater_tile = tile_at(self.eater.x, self.eater.y);
        if self
            .hunters
            .iter()
            .any(|h| !h.penned && tile_at(h.x, h.y) == eater_tile)
        {
            self.caught = true;
            events.life_lost = true;
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
        })
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
    }
}

/// The tile `n` ahead of the eater's `tile` along `dir`. Facing up it is also `n` to
/// the left — the original's overflow quirk, which both the Ambusher (n=4) and the
/// Fickle's pivot (n=2) inherit.
fn ahead_of_eater(tile: (i32, i32), dir: Dir, n: i32) -> (i32, i32) {
    let (dx, dy) = dir.delta();
    let mut ahead = (tile.0 + n * dx, tile.1 + n * dy);
    if dir == Dir::Up {
        ahead.0 -= n;
    }
    ahead
}

/// The off-maze corner a hunter heads for in scatter mode (and the Shy breaks for when
/// the eater comes close) — one per quadrant, so the four scatter apart.
fn scatter_corner(kind: HunterKind) -> (i32, i32) {
    match kind {
        HunterKind::Shadow => (COLS as i32 - 3, 0), // top-right
        HunterKind::Ambusher => (2, 0),             // top-left
        HunterKind::Fickle => (COLS as i32 - 1, ROWS as i32 - 1), // bottom-right
        HunterKind::Shy => (0, ROWS as i32 - 1),    // bottom-left
    }
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

    /// Replaces the hunters with a single loose Shadow centred on `(col, row)` facing
    /// `dir`, primed to move on the next step.
    fn plant_hunter(game: &mut Game, col: i32, row: i32, dir: Dir) {
        let (x, y) = tile_center(col, row);
        game.hunters = vec![HunterState {
            x,
            y,
            dir,
            accum: SPEED_DEN,
            kind: HunterKind::Shadow,
            penned: false,
        }];
        game.caught = false;
    }

    #[test]
    fn the_shadow_runs_the_eater_down() {
        // The eater, stalled against the wall left of (6, 23), sits still; the Shadow,
        // planted three tiles up the same corridor, must chase down and catch it.
        let mut game = Game::new(1);
        plant_eater(&mut game, 6, 23, Dir::Left);
        plant_hunter(&mut game, 6, 20, Dir::Down);
        let mut caught = false;
        for _ in 0..200 {
            caught |= game.step(Input::default()).life_lost;
        }
        assert!(caught, "the Shadow closes on and catches the eater");
        assert!(game.caught());
    }

    #[test]
    fn a_hunter_never_reverses() {
        // Over a long chase the hunter only ever turns at right angles or holds on —
        // it never flips to its opposite heading.
        let mut game = Game::new(2);
        let mut prev = game.hunters[0].dir;
        for _ in 0..4000 {
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
    fn the_ambusher_aims_four_ahead() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 10, 20, Dir::Right);
        assert_eq!(game.hunters[1].kind, HunterKind::Ambusher);
        assert_eq!(
            game.hunter_target(1),
            (14, 20),
            "four tiles ahead of the right-facing eater"
        );
    }

    #[test]
    fn the_ambusher_up_quirk_aims_ahead_and_aside() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 10, 20, Dir::Up);
        assert_eq!(
            game.hunter_target(1),
            (6, 16),
            "facing up: four ahead and four to the left, the original's overflow"
        );
    }

    #[test]
    fn the_fickle_pincers_off_the_shadow() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 10, 20, Dir::Right);
        // Park the Shadow (hunters[0]) at a known tile the Fickle steers off.
        (game.hunters[0].x, game.hunters[0].y) = tile_center(10, 10);
        assert_eq!(game.hunters[2].kind, HunterKind::Fickle);
        // pivot = two ahead of the eater = (12, 20); target = 2*pivot - shadow = (14, 30).
        assert_eq!(game.hunter_target(2), (14, 30));
    }

    #[test]
    fn the_shy_chases_when_far_and_flees_when_near() {
        let mut game = Game::new(1);
        plant_eater(&mut game, 10, 20, Dir::Left);
        assert_eq!(game.hunters[3].kind, HunterKind::Shy);
        // Fifteen tiles up — far — so it targets the eater.
        (game.hunters[3].x, game.hunters[3].y) = tile_center(10, 5);
        assert_eq!(
            game.hunter_target(3),
            (10, 20),
            "far: the Shy targets the eater"
        );
        // On the eater's tile — near — so it breaks for its own corner.
        (game.hunters[3].x, game.hunters[3].y) = tile_center(10, 20);
        assert_eq!(
            game.hunter_target(3),
            scatter_corner(HunterKind::Shy),
            "near: the Shy breaks for its corner"
        );
    }

    #[test]
    fn each_mind_has_a_distinct_scatter_corner() {
        let corners = [
            scatter_corner(HunterKind::Shadow),
            scatter_corner(HunterKind::Ambusher),
            scatter_corner(HunterKind::Fickle),
            scatter_corner(HunterKind::Shy),
        ];
        for (i, ci) in corners.iter().enumerate() {
            for cj in &corners[i + 1..] {
                assert_ne!(ci, cj, "the four scatter corners are distinct");
            }
        }
    }

    #[test]
    fn the_penned_hunters_hold_until_released() {
        let mut game = Game::new(1);
        let positions = |g: &Game| -> Vec<(i32, i32)> {
            g.hunters()
                .filter(|h| h.kind != HunterKind::Shadow)
                .map(|h| (h.x, h.y))
                .collect()
        };
        let before = positions(&game);
        for _ in 0..300 {
            game.step(Input::default());
        }
        assert_eq!(
            before,
            positions(&game),
            "the penned hunters stay put until released"
        );
    }
}
