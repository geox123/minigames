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
//! maze-cleared event. The hunters land in the tickets that follow.

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
const ENERGIZER_STALL: u32 = 3;
/// What a dot and a power pellet score.
const DOT_SCORE: u32 = 10;
const ENERGIZER_SCORE: u32 = 50;

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
    "######.##          ##.######", // 17
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
    Energizer,
}

/// The eater, as the shell should draw it: its centre in logical pixels and the way
/// it faces (which the shell animates the chomp along).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eater {
    pub x: i32,
    pub y: i32,
    pub dir: Dir,
}

/// A tile mover's live state, in logical pixels: where it is, the way it heads, the
/// turn it wants next (buffered until an opening takes it), its fractional-speed
/// accumulator, and any eating stall freezing it. The eater is one; the hunters
/// (later tickets) reuse the same shape.
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
                    b'o' => (Tile::Path, Pickup::Energizer),
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
        if col < 0 || col >= COLS as i32 || row < 0 || row >= ROWS as i32 {
            return Tile::Wall;
        }
        self.tiles[row as usize][col as usize]
    }
}

/// What the player pressed this step — a desired heading, latched by the core until
/// a turn can honour it (a later ticket). All-false means "no new intent".
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
    pub energizer_eaten: bool,
    /// The maze was cleared of every pickup this step (a later ticket advances the
    /// level on it).
    pub maze_cleared: bool,
}

/// Where a game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The game is being played.
    Playing,
    /// Every life has been spent (a later ticket).
    GameOver,
}

/// The whole game: the maze and the seeded RNG that later movers draw on. Advanced
/// only through [`Game::step`]; everything else is read-only.
pub struct Game {
    maze: Maze,
    eater: MoverState,
    score: u32,
    phase: Phase,
    /// Steps taken so far.
    steps: u64,
    /// The seed the game began on, so a restart replays it exactly. Unused by the
    /// board alone; the movers that follow are what it seeds.
    seed: u64,
    #[allow(dead_code)]
    rng: Rng,
}

impl Game {
    /// Starts a new game. The same seed always produces the same game: the maze is
    /// laid out full, and the eater waits centred on its start tile facing left.
    pub fn new(seed: u64) -> Self {
        let (sx, sy) = tile_center(EATER_START.0 as i32, EATER_START.1 as i32);
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
            score: 0,
            phase: Phase::Playing,
            steps: 0,
            seed,
            rng: Rng::new(seed),
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
                w.perpendicular(self.eater.dir)
                    && self.eater_can_enter(tc + w.delta().0, tr + w.delta().1)
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
            }
            return false;
        }

        if ox == HALF && oy == HALF {
            // Squarely centred: take a buffered turn if its corridor is open...
            if let Some(w) = self.eater.want
                && self.eater_can_enter(tc + w.delta().0, tr + w.delta().1)
            {
                self.eater.dir = w;
            }
            // ...then press on if the way ahead is open, else stall against the wall.
            let (dx, dy) = self.eater.dir.delta();
            if self.eater_can_enter(tc + dx, tr + dy) {
                self.move_eater_pixel(self.eater.dir);
            }
            return false;
        }

        // Mid-tile with nothing to turn onto: carry straight on toward the next centre.
        self.move_eater_pixel(self.eater.dir);
        false
    }

    /// Eats the pickup on `(col, row)` if there is one: scores it, clears it, sets the
    /// feeding stall, and flags the cleared maze. Returns whether anything was eaten.
    fn eat_at(&mut self, col: i32, row: i32, events: &mut Events) -> bool {
        if col < 0 || col >= COLS as i32 || row < 0 || row >= ROWS as i32 {
            return false;
        }
        let pickup = self.maze.pickups[row as usize][col as usize];
        if pickup == Pickup::None {
            return false;
        }
        self.maze.pickups[row as usize][col as usize] = Pickup::None;
        self.maze.remaining -= 1;
        match pickup {
            Pickup::Dot => {
                self.score += DOT_SCORE;
                self.eater.stall = DOT_STALL;
                events.dot_eaten = true;
            }
            Pickup::Energizer => {
                self.score += ENERGIZER_SCORE;
                self.eater.stall = ENERGIZER_STALL;
                events.energizer_eaten = true;
            }
            Pickup::None => unreachable!(),
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
        let (dx, dy) = dir.delta();
        self.eater.x = (self.eater.x + dx).rem_euclid(LOGICAL_WIDTH);
        self.eater.y += dy;
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
        if col < 0 || col >= COLS as i32 || row < 0 || row >= ROWS as i32 {
            return Pickup::None;
        }
        self.maze.pickups[row as usize][col as usize]
    }

    /// The eater, as the shell should draw it.
    pub fn eater(&self) -> Eater {
        Eater {
            x: self.eater.x,
            y: self.eater.y,
            dir: self.eater.dir,
        }
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

/// The Collection's small deterministic PRNG: splitmix64 to spread the seed, then
/// xorshift for the stream. Shared in spirit with the other cores' `Rng`; kept local
/// so the core has no dependency. Unused by the board alone — the movers seed on it.
#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        Self(z | 1)
    }

    #[allow(dead_code)]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

#[cfg(test)]
mod tests {
    //! Board-shape invariants: the maze parses to the right size, is symmetric, holds
    //! four power pellets one near each corner, is fully connected (no pickup walled
    //! off), and has the pen, gate and tunnel it needs. Movement is tested with the
    //! movers, in later tickets.
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
    fn the_pickup_total_matches_the_layout() {
        let game = Game::new(1);
        let counted: u32 = (0..ROWS)
            .flat_map(|r| (0..COLS).map(move |c| (c, r)))
            .filter(|&(c, r)| matches!(glyph(c, r), b'.' | b'o'))
            .count() as u32;
        assert_eq!(game.pickups_total(), counted);
        assert_eq!(game.pickups_remaining(), counted, "a fresh maze is full");
        // A faithful maze is packed with dots — the original held 244 pickups; ours
        // sits in the same neighbourhood.
        assert!(
            (230..=250).contains(&counted),
            "the maze holds a faithful pickup count, got {counted}"
        );
    }

    #[test]
    fn the_pen_has_a_gate_and_an_enclosed_interior() {
        let game = Game::new(1);
        let gates: Vec<(i32, i32)> = (0..ROWS)
            .flat_map(|r| (0..COLS).map(move |c| (c as i32, r as i32)))
            .filter(|&(c, r)| game.tile(c, r) == Tile::Gate)
            .collect();
        assert!(!gates.is_empty(), "the pen has a gate");
        // The interior tiles (open path enclosed by walls/gate around the pen centre)
        // exist and carry no pickups.
        assert_eq!(game.tile(13, 13), Tile::Path);
        assert_eq!(game.pickup(13, 13), Pickup::None, "the pen holds no dots");
    }

    #[test]
    fn the_tunnel_row_is_open_past_both_edges() {
        let game = Game::new(1);
        assert_eq!(game.tile(-1, TUNNEL_ROW as i32), Tile::Path);
        assert_eq!(game.tile(COLS as i32, TUNNEL_ROW as i32), Tile::Path);
        // Off the tunnel row, out-of-bounds is solid.
        assert_eq!(game.tile(-1, 5), Tile::Wall);
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
        assert!(events.energizer_eaten);
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
}
