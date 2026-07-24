//! The pure, deterministic core of RIFT — Breakout's Remix.
//!
//! Like the Collection's other cores it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and advances in fixed timesteps
//! so a seed and a sequence of inputs always replay the same run.
//!
//! A run is a **finite, winnable descent** through [`DEPTHS`] depths. Each depth
//! is a few ordinary walls capped by a **guardian** wall; clearing a guardian
//! descends to the next depth and banks a life, and clearing the final depth's
//! guardian **wins** the run. A run-long pool of [`LIVES_START`] lives replaces
//! per-screen balls: dropping the ball spends a life, and running out ends the
//! run. Deflect the ball off the paddle to break bricks; between walls the ball
//! re-serves. Walls draw a zoo of brick kinds from the pool ([`Kind`]). Guardians
//! are arranged from ordinary walls for now; their set-piece layouts, the boons
//! and the juice are layered on in later work.
//!
//! The available [`Pool`] of brick and boon types is passed *in* at construction,
//! so the core never knows the concept of "unlocks": it only ever draws on the
//! pool it is handed.

/// Width of the portrait play field, in logical units.
pub const LOGICAL_WIDTH: f32 = 240.0;
/// Height of the portrait play field, in logical units.
pub const LOGICAL_HEIGHT: f32 = 320.0;

/// Length of a single simulation step, in seconds.
pub const TIMESTEP: f32 = 1.0 / 120.0;

/// The ball is a square this many logical units on a side.
pub const BALL_SIZE: f32 = 4.0;
/// Speed of the ball, in logical units per second.
pub const BALL_SPEED: f32 = 150.0;

/// Full width of the paddle, in logical units.
pub const PADDLE_WIDTH: f32 = 40.0;
/// The widest the paddle grows, however many Widen boons are taken.
const MAX_PADDLE_WIDTH: f32 = 96.0;
/// Height of the paddle, in logical units.
pub const PADDLE_HEIGHT: f32 = 6.0;
/// Gap between the paddle's bottom and the foot of the field.
pub const PADDLE_MARGIN: f32 = 18.0;
/// How fast the paddle travels, in logical units per second.
pub const PADDLE_SPEED: f32 = 210.0;

/// The brick wall: rows, columns, where it sits, and how tall each brick is.
pub const BRICK_COLS: usize = 14;
pub const BRICK_ROWS: usize = 8;
/// The top of the wall, leaving room above for the run HUD.
pub const BRICK_TOP: f32 = 46.0;
/// Height of one brick, in logical units.
pub const BRICK_HEIGHT: f32 = 9.0;
/// The wall thickness the bricks sit inside.
const WALL: f32 = 2.0;

/// Points for each band, from the low band (index 0, bottom rows) up.
const BAND_POINTS: [u32; 4] = [1, 3, 5, 7];

/// A run descends this many depths; clearing the last one's guardian wins.
pub const DEPTHS: u32 = 3;
/// Ordinary walls in a depth, before the guardian that caps it.
const ORDINARY_WALLS_PER_DEPTH: u32 = 2;
/// Walls in a depth: the ordinary walls plus the guardian.
pub const WALLS_PER_DEPTH: u32 = ORDINARY_WALLS_PER_DEPTH + 1;
/// Lives a run begins with.
pub const LIVES_START: u32 = 3;
/// The most lives a run can bank; clearing a guardian restores one up to here.
pub const LIVES_CAP: u32 = 5;

/// Steps between a mover sliding one cell along its row (half a second).
const MOVER_PERIOD: u32 = 60;
/// Steps between a spawner refilling an adjacent empty cell.
const SPAWN_PERIOD: u32 = 150;

/// Boons offered in a draft.
const DRAFT_OFFERS: usize = 3;
/// Rerolls a run starts with.
const START_REROLLS: u32 = 2;
/// How much a Widen boon adds to the paddle's width.
const WIDEN_AMOUNT: f32 = 8.0;
/// How much a Swift boon adds to the paddle's speed.
const SWIFT_AMOUNT: f32 = 40.0;

/// How long the ball rests on the paddle before each serve.
pub const SERVE_PAUSE: f32 = 0.6;

/// The number of segments the paddle is read in for deflection.
const SEGMENTS: usize = 8;
/// The widest angle, in degrees from vertical, the paddle's ends send the ball.
const WIDEST_ANGLE: f32 = 60.0;

/// The top edge of the paddle, in logical units.
fn paddle_top() -> f32 {
    LOGICAL_HEIGHT - PADDLE_MARGIN - PADDLE_HEIGHT
}

/// Which way the player is pushing the paddle this step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Move {
    /// Towards the left wall.
    Left,
    /// Not moving.
    #[default]
    Hold,
    /// Towards the right wall.
    Right,
}

/// What the player is doing this step. During a draft, `paddle` moves the
/// highlight (one step per press, as the shell only sends a direction on the
/// key's edge), `confirm` takes the highlighted option, and `reroll` re-rolls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// The paddle direction (or, in a draft, the highlight move).
    pub paddle: Move,
    /// Take the highlighted draft option.
    pub confirm: bool,
    /// Re-roll the draft offers.
    pub reroll: bool,
}

/// A kind of brick. Normal bricks break in one hit; the zoo adds bricks that
/// behave.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Breaks in one hit and scores its band.
    Normal,
    /// Takes two hits: the first reflects and cracks it, the second breaks it.
    Armoured,
    /// Indestructible: sends the ball straight back and never breaks, so it does
    /// not count toward clearing the wall — an obstacle to play around.
    Mirror,
    /// Breaks in one hit and blows up, chaining the break to its neighbours.
    Explosive,
    /// Breaks in one hit; while it stands, it slides along its row.
    Mover,
    /// Breaks in one hit; while it stands, it refills an adjacent empty cell.
    Spawner,
}

impl Kind {
    /// Hits this kind takes before breaking (0 for the indestructible mirror).
    fn hits(self) -> u8 {
        match self {
            Kind::Mirror => 0,
            Kind::Armoured => 2,
            _ => 1,
        }
    }

    /// Whether this kind counts toward clearing the wall.
    fn destructible(self) -> bool {
        self != Kind::Mirror
    }

    /// How often a cell rolls this special kind when it is in the pool.
    fn chance(self) -> f32 {
        match self {
            Kind::Armoured => 0.10,
            Kind::Explosive => 0.05,
            Kind::Mirror => 0.04,
            Kind::Mover => 0.04,
            Kind::Spawner => 0.03,
            Kind::Normal => 0.0,
        }
    }
}

/// Where a boon attaches, for grouping it in the draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Home {
    /// Changes the ball.
    Ball,
    /// Changes the paddle.
    Paddle,
    /// Changes the run as a whole.
    Global,
}

impl Home {
    /// A short label for the draft.
    pub fn label(self) -> &'static str {
        match self {
            Home::Ball => "BALL",
            Home::Paddle => "PADDLE",
            Home::Global => "RUN",
        }
    }
}

/// A boon the player drafts between walls. Effects stack across the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boon {
    /// The ball plows through the bricks it breaks instead of rebounding.
    Pierce,
    /// The ball's breaks explode, chaining to neighbours.
    Blast,
    /// A wider paddle.
    Widen,
    /// A faster paddle.
    Swift,
    /// One more life.
    ExtraLife,
    /// Every brick is worth more.
    Fortune,
}

impl Boon {
    /// Where this boon attaches.
    pub fn home(self) -> Home {
        match self {
            Boon::Pierce | Boon::Blast => Home::Ball,
            Boon::Widen | Boon::Swift => Home::Paddle,
            Boon::ExtraLife | Boon::Fortune => Home::Global,
        }
    }

    /// A short name for the draft.
    pub fn title(self) -> &'static str {
        match self {
            Boon::Pierce => "PIERCE",
            Boon::Blast => "BLAST",
            Boon::Widen => "WIDEN",
            Boon::Swift => "SWIFT",
            Boon::ExtraLife => "EXTRA LIFE",
            Boon::Fortune => "FORTUNE",
        }
    }

    /// A one-line description for the draft.
    pub fn description(self) -> &'static str {
        match self {
            Boon::Pierce => "PLOW THROUGH BROKEN BRICKS",
            Boon::Blast => "YOUR BREAKS EXPLODE",
            Boon::Widen => "A WIDER PADDLE",
            Boon::Swift => "A FASTER PADDLE",
            Boon::ExtraLife => "ONE MORE LIFE",
            Boon::Fortune => "BRICKS SCORE MORE",
        }
    }
}

/// The brick and boon types a run may draw on, passed *in* at construction so
/// the core never knows about "unlocks" — only the pool it is handed.
///
/// It carries the special brick kinds a run's walls may include and the boons
/// its drafts may offer; Phase A hands in the full set, and cross-run unlocks
/// will pare it back later. Kept and threaded from the start so those additions
/// don't reshape the seam.
#[derive(Clone, Debug, Default)]
pub struct Pool {
    /// Special (non-[`Kind::Normal`]) brick kinds walls may include.
    specials: Vec<Kind>,
    /// The boons drafts may offer.
    boons: Vec<Boon>,
}

impl Pool {
    /// The base pool: every special brick kind and boon built so far.
    pub fn base() -> Self {
        Self {
            specials: vec![
                Kind::Armoured,
                Kind::Explosive,
                Kind::Mirror,
                Kind::Mover,
                Kind::Spawner,
            ],
            boons: vec![
                Boon::Pierce,
                Boon::Blast,
                Boon::Widen,
                Boon::Swift,
                Boon::ExtraLife,
                Boon::Fortune,
            ],
        }
    }
}

/// The paddle, as the shell should draw it: the top-left corner of a `width` by
/// [`PADDLE_HEIGHT`] rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paddle {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Current width.
    pub width: f32,
}

/// A brick, as the shell should draw it: a rectangle in a colour band, with its
/// kind and whether it is a cracked (part-broken) armoured brick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brick {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
    /// Colour band, 0 (low, bottom) to 3 (high, top).
    pub band: u8,
    /// What kind of brick this is.
    pub kind: Kind,
    /// An armoured brick that has taken its first hit.
    pub damaged: bool,
}

/// The ball's position and velocity, in logical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    /// Horizontal centre.
    pub x: f32,
    /// Vertical centre.
    pub y: f32,
    /// Horizontal velocity, units per second.
    pub vx: f32,
    /// Vertical velocity, units per second.
    pub vy: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The ball rebounded off a wall.
    pub wall_bounce: bool,
    /// The paddle returned the ball.
    pub paddle_hit: bool,
    /// A brick was broken this step, and its band (0 low to 3 high).
    pub brick_broken: Option<u8>,
    /// A brick was struck but not broken this step — an armoured brick's first
    /// hit, or a mirror's bounce.
    pub brick_hit: bool,
    /// An explosive brick blew up this step, chaining to its neighbours.
    pub exploded: bool,
    /// The last brick of an ordinary wall fell, bringing up the next wall.
    pub wall_cleared: bool,
    /// A guardian fell this step, completing a depth (and banking a life).
    pub guardian_cleared: bool,
    /// The ball fell past the paddle this step and a life was spent.
    pub lost_ball: bool,
    /// The final guardian fell this step — the run was won.
    pub won: bool,
    /// The last life was spent this step — the run was lost.
    pub lost: bool,
}

/// Where a run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The ball rests on the paddle, waiting to be served.
    Serving,
    /// The ball is in play.
    Playing,
    /// A wall was cleared; the player is drafting a boon before the next.
    Drafting,
    /// The final guardian has fallen — the run is won.
    Won,
    /// Every life has been spent — the run is lost.
    Lost,
}

/// What emptying a wall did, so [`Game::advance_ball`] reports the right event.
enum WallOutcome {
    /// An ordinary wall fell; the next wall of this depth comes up.
    Ordinary,
    /// A guardian fell; the run descends to the next depth.
    Depth,
    /// The final guardian fell; the run is won.
    Won,
}

/// One standing brick: its kind, the hits it has left (unused for the
/// indestructible mirror), and — for a mover — which way it slides (−1 left,
/// +1 right; 0 for everything else).
#[derive(Clone, Copy)]
struct Cell {
    kind: Kind,
    hits: u8,
    dir: i8,
}

impl Cell {
    /// A fresh brick of `kind`, at full hits and not moving.
    fn of(kind: Kind) -> Self {
        Self {
            kind,
            hits: kind.hits(),
            dir: 0,
        }
    }
}

/// What a ball's contact with the wall did this step.
enum Hit {
    /// No brick was touched.
    None,
    /// A brick was struck but not broken (armoured's first hit, or a mirror).
    Struck,
    /// A brick broke, with its band.
    Broke(u8),
    /// An explosive broke and chained to its neighbours; the band is its own.
    Exploded(u8),
}

/// The brick face the ball came in through, and the geometry needed to rebound
/// off it.
#[derive(Clone, Copy)]
struct Face {
    /// The ball met a horizontal (top or bottom) face rather than a side.
    vertical: bool,
    /// It came in through the top face.
    from_above: bool,
    rx: f32,
    ry: f32,
    rw: f32,
    rh: f32,
    /// The ball's horizontal position before the step, to pick the side face.
    prev_x: f32,
}

/// A run of RIFT.
pub struct Game {
    ball: Ball,
    /// Centre of the paddle, in logical units.
    paddle_x: f32,
    paddle_width: f32,
    /// The wall: each cell's standing brick, or `None`, indexed
    /// `row * BRICK_COLS + col`.
    bricks: Vec<Option<Cell>>,
    /// Standing destructible bricks remaining (mirrors are not counted).
    bricks_left: u32,
    score: u32,
    /// The depth being played, 0-based (0 to [`DEPTHS`] − 1).
    depth: u32,
    /// Which wall within the depth is up, 0-based; the guardian is the last.
    wall_in_depth: u32,
    /// Lives left this run; the run ends when this reaches zero.
    lives: u32,
    /// Steps the current wall has been in play, driving movers and spawners.
    wall_steps: u32,
    /// The ball plows through the bricks it breaks (a Pierce boon).
    pierce: bool,
    /// The ball's breaks explode (a Blast boon).
    blast: bool,
    /// The paddle's current speed; Swift boons raise it.
    paddle_speed: f32,
    /// Fortune boons taken; each raises every brick's score.
    fortunes: u32,
    /// Rerolls left this run.
    rerolls: u32,
    /// The boons on offer in the current draft.
    offers: Vec<Boon>,
    /// Which draft option is highlighted; the reroll option follows the offers.
    draft_highlight: usize,
    /// Every boon taken this run, in order, for the shell to show.
    taken: Vec<Boon>,
    phase: Phase,
    /// Seconds left of the pause before the serve.
    serve_countdown: f32,
    /// The seed the run began on, so a restart replays from the very start.
    seed: u64,
    /// The pool this run draws on, kept so a restart rebuilds the same run.
    pool: Pool,
    rng: Rng,
}

impl Game {
    /// Starts a new run on `seed`, drawing on `pool`. The same seed and pool
    /// always replay the same run.
    pub fn new_run(seed: u64, pool: &Pool) -> Self {
        let mut game = Self {
            ball: PARKED_BALL,
            paddle_x: LOGICAL_WIDTH / 2.0,
            paddle_width: PADDLE_WIDTH,
            bricks: vec![None; BRICK_ROWS * BRICK_COLS],
            bricks_left: 0,
            score: 0,
            depth: 0,
            wall_in_depth: 0,
            lives: LIVES_START,
            wall_steps: 0,
            pierce: false,
            blast: false,
            paddle_speed: PADDLE_SPEED,
            fortunes: 0,
            rerolls: START_REROLLS,
            offers: Vec::new(),
            draft_highlight: 0,
            taken: Vec::new(),
            phase: Phase::Serving,
            serve_countdown: SERVE_PAUSE,
            seed,
            pool: pool.clone(),
            rng: Rng::new(seed),
        };
        game.build_wall();
        game.begin_ball();
        game
    }

    /// The points a brick in `band` (0 low to 3 high) is worth.
    pub fn band_points(&self, band: u8) -> u32 {
        BAND_POINTS[band as usize]
    }

    /// The current score.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// The bricks still standing, as the shell should draw them.
    pub fn bricks(&self) -> impl Iterator<Item = Brick> + '_ {
        self.bricks.iter().enumerate().filter_map(|(i, cell)| {
            let cell = (*cell)?;
            let (row, col) = (i / BRICK_COLS, i % BRICK_COLS);
            let (x, y, width, height) = brick_rect(row, col);
            Some(Brick {
                x,
                y,
                width,
                height,
                band: band_of(row),
                kind: cell.kind,
                damaged: cell.kind == Kind::Armoured && cell.hits == 1,
            })
        })
    }

    /// Standing bricks remaining.
    pub fn bricks_left(&self) -> u32 {
        self.bricks_left
    }

    /// The depth reached, 1-based (1 to [`DEPTHS`]). On a won run this is
    /// [`DEPTHS`]; it is also the run's "depth reached" for the summary.
    pub fn depth(&self) -> u32 {
        self.depth + 1
    }

    /// Lives left this run.
    pub fn lives(&self) -> u32 {
        self.lives
    }

    /// Whether the wall currently up is the depth's guardian.
    pub fn on_guardian(&self) -> bool {
        self.wall_in_depth == ORDINARY_WALLS_PER_DEPTH
    }

    /// The boons on offer in the open draft (empty outside a draft).
    pub fn offers(&self) -> &[Boon] {
        &self.offers
    }

    /// Which offer is highlighted in the open draft.
    pub fn draft_highlight(&self) -> usize {
        self.draft_highlight
    }

    /// Rerolls left this run.
    pub fn rerolls(&self) -> u32 {
        self.rerolls
    }

    /// Every boon taken this run, in the order taken.
    pub fn boons(&self) -> &[Boon] {
        &self.taken
    }

    /// Starts the run over from the beginning: the same seed and pool replay the
    /// same run.
    pub fn restart(&mut self) {
        let pool = self.pool.clone();
        *self = Self::new_run(self.seed, &pool);
    }

    /// The ball, as the shell should draw it. Between balls it rests on the
    /// paddle.
    pub fn ball(&self) -> Ball {
        self.ball
    }

    /// The paddle, as the shell should draw it.
    pub fn paddle(&self) -> Paddle {
        Paddle {
            x: self.paddle_x - self.paddle_width / 2.0,
            y: paddle_top(),
            width: self.paddle_width,
        }
    }

    /// Where the run is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Advances the run by exactly one [`TIMESTEP`] (or, in a draft, applies one
    /// input — the shell steps a draft once per frame, not on the accumulator).
    pub fn step(&mut self, input: Input) -> Events {
        match self.phase {
            Phase::Serving => {
                self.move_paddle(input.paddle);
                // The ball rides the paddle until the pause ends.
                self.ball.x = self.paddle_x;
                self.serve_countdown -= TIMESTEP;
                if self.serve_countdown <= 0.0 {
                    self.serve();
                }
                Events::default()
            }
            Phase::Playing => {
                self.move_paddle(input.paddle);
                self.advance_ball()
            }
            Phase::Drafting => self.draft(input),
            // The run is over; it rests until restarted.
            Phase::Won | Phase::Lost => Events::default(),
        }
    }

    fn move_paddle(&mut self, mv: Move) {
        let travel = self.paddle_speed * TIMESTEP;
        match mv {
            Move::Left => self.paddle_x -= travel,
            Move::Right => self.paddle_x += travel,
            Move::Hold => {}
        }
        let half = self.paddle_width / 2.0;
        self.paddle_x = self.paddle_x.clamp(half, LOGICAL_WIDTH - half);
    }

    fn advance_ball(&mut self) -> Events {
        // The wall lives while the ball is in play: movers slide, spawners fill.
        self.wall_steps += 1;
        self.advance_wall();

        let previous = self.ball;
        self.ball.x += self.ball.vx * TIMESTEP;
        self.ball.y += self.ball.vy * TIMESTEP;

        let half = BALL_SIZE / 2.0;
        let mut wall_bounce =
            bounce_within(&mut self.ball.x, &mut self.ball.vx, half, LOGICAL_WIDTH);
        // Top wall only; the bottom is the paddle's line.
        if self.ball.y - half < 0.0 {
            self.ball.y = half;
            self.ball.vy = self.ball.vy.abs();
            wall_bounce = true;
        }

        let hit = self.collide_bricks(previous);
        let brick_broken = match hit {
            Hit::Broke(band) | Hit::Exploded(band) => Some(band),
            _ => None,
        };
        let brick_hit = matches!(hit, Hit::Struck);
        let exploded = matches!(hit, Hit::Exploded(_));

        // Emptying a wall completes it: an ordinary wall brings up the next, a
        // guardian descends a depth (and re-parks), the final guardian wins.
        let (mut wall_cleared, mut guardian_cleared, mut won) = (false, false, false);
        if brick_broken.is_some() && self.bricks_left == 0 {
            match self.finish_wall() {
                WallOutcome::Ordinary => wall_cleared = true,
                WallOutcome::Depth => guardian_cleared = true,
                WallOutcome::Won => won = true,
            }
        }

        // A completed wall re-parks the ball or ends the run, so only strike and
        // drop while still in play.
        let paddle_hit = self.phase == Phase::Playing && self.strike_paddle(previous);

        let (mut lost_ball, mut lost) = (false, false);
        if self.phase == Phase::Playing && self.ball.y - half > LOGICAL_HEIGHT {
            lost_ball = true;
            self.lives -= 1;
            if self.lives == 0 {
                lost = true;
                self.phase = Phase::Lost;
                self.ball = PARKED_BALL;
            } else {
                self.begin_ball();
            }
        }

        Events {
            wall_bounce,
            paddle_hit,
            brick_broken,
            brick_hit,
            exploded,
            wall_cleared,
            guardian_cleared,
            lost_ball,
            won,
            lost,
        }
    }

    /// Handles a wall being emptied, returning what it did. An ordinary wall
    /// brings up the next wall of the depth; a guardian descends to the next
    /// depth and banks a life; the final depth's guardian wins the run.
    fn finish_wall(&mut self) -> WallOutcome {
        if self.on_guardian() {
            if self.depth + 1 >= DEPTHS {
                self.phase = Phase::Won;
                self.ball = PARKED_BALL;
                WallOutcome::Won
            } else {
                self.depth += 1;
                self.wall_in_depth = 0;
                self.lives = (self.lives + 1).min(LIVES_CAP);
                self.enter_draft();
                WallOutcome::Depth
            }
        } else {
            self.wall_in_depth += 1;
            self.enter_draft();
            WallOutcome::Ordinary
        }
    }

    /// Opens the boon draft held between walls: parks the ball and rolls the
    /// offers. The next wall (already selected by the advanced counters) is built
    /// once the player takes a boon.
    fn enter_draft(&mut self) {
        self.phase = Phase::Drafting;
        self.ball = PARKED_BALL;
        self.draft_highlight = 0;
        self.roll_offers();
    }

    /// Rolls a fresh set of distinct boon offers from the pool.
    fn roll_offers(&mut self) {
        let mut available = self.pool.boons.clone();
        self.offers.clear();
        for _ in 0..DRAFT_OFFERS.min(available.len()) {
            let idx = (self.rng.next_u64() as usize) % available.len();
            self.offers.push(available.swap_remove(idx));
        }
    }

    /// Applies one input to the open draft: the paddle direction moves the
    /// highlight, `confirm` takes the highlighted boon (and moves on to the next
    /// wall), `reroll` spends a reroll for fresh offers.
    fn draft(&mut self, input: Input) -> Events {
        let n = self.offers.len();
        if n == 0 {
            // No boons to offer — just move on.
            self.build_wall();
            self.begin_ball();
            return Events::default();
        }
        match input.paddle {
            Move::Left => self.draft_highlight = (self.draft_highlight + n - 1) % n,
            Move::Right => self.draft_highlight = (self.draft_highlight + 1) % n,
            Move::Hold => {}
        }
        if input.confirm {
            let boon = self.offers[self.draft_highlight];
            self.apply_boon(boon);
            self.taken.push(boon);
            self.offers.clear();
            // The counters already point at the next wall; build it and serve.
            self.build_wall();
            self.begin_ball();
        } else if input.reroll && self.rerolls > 0 {
            self.rerolls -= 1;
            self.roll_offers();
        }
        Events::default()
    }

    /// Applies a taken boon's effect. Effects stack across the run.
    fn apply_boon(&mut self, boon: Boon) {
        match boon {
            Boon::Pierce => self.pierce = true,
            Boon::Blast => self.blast = true,
            Boon::Widen => {
                self.paddle_width = (self.paddle_width + WIDEN_AMOUNT).min(MAX_PADDLE_WIDTH)
            }
            Boon::Swift => self.paddle_speed += SWIFT_AMOUNT,
            Boon::ExtraLife => self.lives += 1,
            Boon::Fortune => self.fortunes += 1,
        }
    }

    /// Builds a fresh wall. A guardian wall is a designed set-piece for the
    /// depth; an ordinary wall draws each cell's kind from the pool — mostly
    /// normal bricks, with the pool's special kinds sprinkled in at their own
    /// rates. The same seed and pool always build the same wall.
    fn build_wall(&mut self) {
        if self.on_guardian() {
            self.build_guardian();
            return;
        }

        let specials = self.pool.specials.clone();
        let mut destructible = 0;
        for i in 0..self.bricks.len() {
            // Give each enabled special a chance at this cell; the first that
            // rolls wins, otherwise the cell is a normal brick.
            let mut kind = Kind::Normal;
            for &special in &specials {
                if self.rng.range(0.0, 1.0) < special.chance() {
                    kind = special;
                    break;
                }
            }
            if kind.destructible() {
                destructible += 1;
            }
            let mut cell = Cell::of(kind);
            if kind == Kind::Mover {
                // Slide left or right, seeded.
                cell.dir = if self.rng.range(0.0, 1.0) < 0.5 {
                    -1
                } else {
                    1
                };
            }
            self.bricks[i] = Some(cell);
        }
        self.bricks_left = destructible;
        self.wall_steps = 0;
    }

    /// Lays out the guardian for the current depth from its designed template.
    /// Deeper guardians are tougher; the deepest is the run's final test.
    fn build_guardian(&mut self) {
        let template = GUARDIANS[(self.depth as usize).min(GUARDIANS.len() - 1)];
        for cell in self.bricks.iter_mut() {
            *cell = None;
        }
        let mut destructible = 0;
        for (row, line) in template.iter().enumerate() {
            for (col, ch) in line.chars().enumerate().take(BRICK_COLS) {
                let Some(kind) = kind_from_char(ch) else {
                    continue;
                };
                if kind.destructible() {
                    destructible += 1;
                }
                let mut cell = Cell::of(kind);
                if kind == Kind::Mover {
                    cell.dir = if self.rng.range(0.0, 1.0) < 0.5 {
                        -1
                    } else {
                        1
                    };
                }
                self.bricks[row * BRICK_COLS + col] = Some(cell);
            }
        }
        self.bricks_left = destructible;
        self.wall_steps = 0;
    }

    /// Chains an explosive break to its neighbours: every adjacent destructible
    /// brick is destroyed and scored, and adjacent explosives chain in turn.
    /// Mirrors, being indestructible, are unaffected.
    fn explode(&mut self, origin: usize) {
        let mut stack = vec![origin];
        while let Some(i) = stack.pop() {
            for n in orthogonal_neighbours(i) {
                let Some(cell) = self.bricks[n] else {
                    continue;
                };
                if !cell.kind.destructible() {
                    continue;
                }
                self.bricks[n] = None;
                self.bricks_left -= 1;
                self.score += self.brick_score(band_of(n / BRICK_COLS));
                if cell.kind == Kind::Explosive {
                    stack.push(n);
                }
            }
        }
    }

    /// Advances the living wall one step: movers slide and spawners fill on their
    /// own cadences.
    fn advance_wall(&mut self) {
        if self.wall_steps.is_multiple_of(MOVER_PERIOD) {
            self.advance_movers();
        }
        if self.wall_steps.is_multiple_of(SPAWN_PERIOD) {
            self.advance_spawners();
        }
    }

    /// Slides each mover one cell along its row into an empty neighbour, or
    /// reverses it where it is blocked or at an edge. Occupancy is read from the
    /// start of the tick so movers neither chase each other nor move twice.
    fn advance_movers(&mut self) {
        let occupied: Vec<bool> = self.bricks.iter().map(Option::is_some).collect();
        let mut filled = vec![false; self.bricks.len()];
        for i in 0..self.bricks.len() {
            // A cell just moved into this tick is not itself re-processed.
            if filled[i] {
                continue;
            }
            let Some(cell) = self.bricks[i] else {
                continue;
            };
            if cell.kind != Kind::Mover {
                continue;
            }
            let col = i % BRICK_COLS;
            let target_col = col as i32 + cell.dir as i32;
            if (0..BRICK_COLS as i32).contains(&target_col) {
                let target = i - col + target_col as usize;
                if !occupied[target] && !filled[target] {
                    self.bricks[target] = Some(cell);
                    self.bricks[i] = None;
                    filled[target] = true;
                    continue;
                }
            }
            // Blocked or at an edge: turn around in place.
            self.bricks[i] = Some(Cell {
                dir: -cell.dir,
                ..cell
            });
        }
    }

    /// Each spawner refills the first empty cell among its neighbours (below,
    /// then the sides, then above) with a normal brick, regrowing the wall until
    /// the spawner itself is broken.
    fn advance_spawners(&mut self) {
        for i in 0..self.bricks.len() {
            let Some(cell) = self.bricks[i] else {
                continue;
            };
            if cell.kind != Kind::Spawner {
                continue;
            }
            for n in orthogonal_neighbours(i) {
                if self.bricks[n].is_none() {
                    self.bricks[n] = Some(Cell::of(Kind::Normal));
                    self.bricks_left += 1;
                    break;
                }
            }
        }
    }

    /// Resolves the ball's contact with the first standing brick it overlaps,
    /// reflecting off the face it came in through. A normal brick breaks (and
    /// scores); an armoured brick cracks on its first hit and breaks on its
    /// second; a mirror sends the ball straight back and never breaks; an
    /// explosive breaks and chains to its neighbours. At most one direct contact
    /// per step — the ball's step is far smaller than a brick, so it can never
    /// pass through the wall.
    fn collide_bricks(&mut self, previous: Ball) -> Hit {
        let half = BALL_SIZE / 2.0;
        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                let i = row * BRICK_COLS + col;
                let Some(cell) = self.bricks[i] else {
                    continue;
                };
                let (rx, ry, rw, rh) = brick_rect(row, col);
                let overlaps = self.ball.x + half > rx
                    && self.ball.x - half < rx + rw
                    && self.ball.y + half > ry
                    && self.ball.y - half < ry + rh;
                if !overlaps {
                    continue;
                }

                // Which face did it come in through? Use where it was before.
                let from_above = previous.y + half <= ry;
                let from_below = previous.y - half >= ry + rh;
                let face = Face {
                    vertical: from_above || from_below,
                    from_above,
                    rx,
                    ry,
                    rw,
                    rh,
                    prev_x: previous.x,
                };

                if cell.kind == Kind::Mirror {
                    // A retroreflector: reflect off the face, then reverse the
                    // other axis too, so the ball heads straight back. Never breaks.
                    self.reflect_face(face);
                    if face.vertical {
                        self.ball.vx = -self.ball.vx;
                    } else {
                        self.ball.vy = -self.ball.vy;
                    }
                    return Hit::Struck;
                }

                // Destructible: the hit either cracks or breaks the brick.
                let hits = cell.hits - 1;
                if hits == 0 {
                    self.bricks[i] = None;
                    self.bricks_left -= 1;
                    let band = band_of(row);
                    self.score += self.brick_score(band);
                    // Pierce plows through; otherwise rebound off the broken brick.
                    if !self.pierce {
                        self.reflect_face(face);
                    }
                    if cell.kind == Kind::Explosive || self.blast {
                        // Blow up, chaining the break to the neighbours.
                        self.explode(i);
                        return Hit::Exploded(band);
                    }
                    return Hit::Broke(band);
                }
                // An armoured brick's first hit: it cracks and rebounds.
                self.reflect_face(face);
                self.bricks[i] = Some(Cell {
                    kind: cell.kind,
                    hits,
                    dir: cell.dir,
                });
                return Hit::Struck;
            }
        }
        Hit::None
    }

    /// Rebounds the ball off the brick face it came in through, nudging it clear.
    fn reflect_face(&mut self, f: Face) {
        let half = BALL_SIZE / 2.0;
        if f.vertical {
            self.ball.vy = -self.ball.vy;
            self.ball.y = if f.from_above {
                f.ry - half
            } else {
                f.ry + f.rh + half
            };
        } else {
            self.ball.vx = -self.ball.vx;
            self.ball.x = if f.prev_x < self.ball.x {
                f.rx - half
            } else {
                f.rx + f.rw + half
            };
        }
    }

    /// A broken brick's score for its band, boosted by any Fortune boons.
    fn brick_score(&self, band: u8) -> u32 {
        BAND_POINTS[band as usize] * (1 + self.fortunes)
    }

    /// Rebounds the ball off the paddle if its path crossed the paddle's top
    /// this step, with an angle set by where it struck.
    fn strike_paddle(&mut self, previous: Ball) -> bool {
        let half = BALL_SIZE / 2.0;
        let top = paddle_top();
        let (before, after) = (previous.y + half, self.ball.y + half);
        let reached = previous.vy > 0.0 && before <= top && after >= top;
        if !reached {
            return false;
        }

        // Where the ball was when it reached the paddle's top.
        let travelled = (after - before).abs();
        let contact_x = if travelled > f32::EPSILON {
            previous.x + (self.ball.x - previous.x) * ((top - before).abs() / travelled)
        } else {
            self.ball.x
        };
        let left = self.paddle_x - self.paddle_width / 2.0;
        let right = self.paddle_x + self.paddle_width / 2.0;
        if contact_x < left - half || contact_x > right + half {
            return false;
        }

        // Contact point across the paddle, -1 (left edge) to +1 (right edge),
        // read in coarse segments as the original's paddle was.
        let across = ((contact_x - self.paddle_x) / (self.paddle_width / 2.0)).clamp(-1.0, 1.0);
        let segment = (across * SEGMENTS as f32).round() / SEGMENTS as f32;
        let angle = (segment * WIDEST_ANGLE).to_radians();

        self.ball.x = contact_x;
        self.ball.y = top - half;
        self.ball.vx = BALL_SPEED * angle.sin();
        self.ball.vy = -BALL_SPEED * angle.cos();
        true
    }

    /// Rests the ball on the paddle and starts the pause before the serve.
    fn begin_ball(&mut self) {
        self.phase = Phase::Serving;
        self.serve_countdown = SERVE_PAUSE;
        self.ball = Ball {
            x: self.paddle_x,
            y: paddle_top() - BALL_SIZE / 2.0,
            vx: 0.0,
            vy: 0.0,
        };
    }

    fn serve(&mut self) {
        // Launch up the field with a slight, seeded lean.
        let lean = self.rng.range(-0.35, 0.35);
        self.ball.vx = BALL_SPEED * lean.sin();
        self.ball.vy = -BALL_SPEED * lean.cos();
        self.phase = Phase::Playing;
    }
}

/// The rectangle of the brick at `(row, col)`: `(x, y, width, height)`.
fn brick_rect(row: usize, col: usize) -> (f32, f32, f32, f32) {
    let brick_w = (LOGICAL_WIDTH - 2.0 * WALL) / BRICK_COLS as f32;
    let x = WALL + col as f32 * brick_w;
    let y = BRICK_TOP + row as f32 * BRICK_HEIGHT;
    (x, y, brick_w, BRICK_HEIGHT)
}

/// The colour band of `row`: two rows per band, high bands (worth more) at the
/// top of the wall.
fn band_of(row: usize) -> u8 {
    // Row 0 is the top; it belongs to the highest band (index 3).
    (BRICK_ROWS / 2 - 1 - row / 2) as u8
}

/// The designed guardian set-pieces, one per depth, deepest last and toughest.
/// Each is [`BRICK_ROWS`] rows of [`BRICK_COLS`] glyphs — `.` empty, `N` normal,
/// `A` armoured, `M` mirror, `E` explosive, `V` mover, `S` spawner. A well-formed
/// test guards their shape; explosives crack open the armoured shells.
const GUARDIANS: [[&str; BRICK_ROWS]; DEPTHS as usize] = [
    // Depth 1 — the Bastion: an armoured border with explosive corners around a
    // mirrored core.
    [
        "AANNNNNNNNNNAA",
        "AENNNNNNNNNNEA",
        "ANNNNNNNNNNNNA",
        "ANNNMMMMMMNNNA",
        "ANNNMMMMMMNNNA",
        "ANNNNNNNNNNNNA",
        "AENNNNNNNNNNEA",
        "AANNNNNNNNNNAA",
    ],
    // Depth 2 — the Works: thicker armour, sliding movers, mirror buttresses.
    [
        "AAAANNNNNNAAAA",
        "AEEANNNNNNAEEA",
        "ANNNVVVVVVNNNA",
        "ANMMNNNNNNMMNA",
        "ANMMNNNNNNMMNA",
        "ANNNVVVVVVNNNA",
        "AEEANNNNNNAEEA",
        "AAAANNNNNNAAAA",
    ],
    // Depth 3 — the Vault: a solid armoured shell, spawners regrowing the walls,
    // and mirror-shielded explosive pockets. The final test.
    [
        "AAAAAAAAAAAAAA",
        "ANSNNNNNNNNSNA",
        "ANMMMMNNMMMMNA",
        "ANMEENNNNEEMNA",
        "ANMEENNNNEEMNA",
        "ANMMMMNNMMMMNA",
        "ANSNNNNNNNNSNA",
        "AAAAAAAAAAAAAA",
    ],
];

/// The brick kind a guardian-template glyph stands for, or `None` for empty.
fn kind_from_char(glyph: char) -> Option<Kind> {
    match glyph {
        'N' => Some(Kind::Normal),
        'A' => Some(Kind::Armoured),
        'M' => Some(Kind::Mirror),
        'E' => Some(Kind::Explosive),
        'V' => Some(Kind::Mover),
        'S' => Some(Kind::Spawner),
        _ => None,
    }
}

/// The wall-cell indices orthogonally adjacent to `i`, in the order below, left,
/// right, above — spawners fill the first empty one in that priority.
fn orthogonal_neighbours(i: usize) -> impl Iterator<Item = usize> {
    let (row, col) = (i / BRICK_COLS, i % BRICK_COLS);
    let mut ns = Vec::with_capacity(4);
    if row + 1 < BRICK_ROWS {
        ns.push((row + 1) * BRICK_COLS + col);
    }
    if col > 0 {
        ns.push(row * BRICK_COLS + col - 1);
    }
    if col + 1 < BRICK_COLS {
        ns.push(row * BRICK_COLS + col + 1);
    }
    if row > 0 {
        ns.push((row - 1) * BRICK_COLS + col);
    }
    ns.into_iter()
}

/// The ball at rest (its resting place is set when a ball begins).
const PARKED_BALL: Ball = Ball {
    x: LOGICAL_WIDTH / 2.0,
    y: LOGICAL_HEIGHT / 2.0,
    vx: 0.0,
    vy: 0.0,
};

/// Keeps a moving square of half-extent `half` inside `0..limit`, reversing its
/// velocity on contact. Returns whether it touched a wall.
fn bounce_within(position: &mut f32, velocity: &mut f32, half: f32, limit: f32) -> bool {
    if *position - half < 0.0 {
        *position = half;
        *velocity = velocity.abs();
        true
    } else if *position + half > limit {
        *position = limit - half;
        *velocity = -velocity.abs();
        true
    } else {
        false
    }
}

/// A tiny xorshift generator, so the core carries no dependencies and stays
/// reproducible across platforms.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Mix the seed through splitmix64 so even adjacent seeds produce
        // well-separated states — a bare `seed ^ K` leaves the top bits (which
        // `range` reads) identical for nearby seeds.
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
    //! Wall, guardian and win transitions. Emptying a wall by honest play is
    //! impractical — a perfect paddle digs a channel the ball bounces in forever
    //! — so these set up the last standing brick (and the run's depth) and let
    //! the real `step` path break it and make the transition. Only the setup
    //! reaches inside; the transition itself runs through the same code the run
    //! does. Lives and losing are reachable by honest play, tested in `tests/`.
    use super::*;

    /// Aims the ball just below `(row, col)`, rising into its underside on the
    /// next step.
    fn aim_below(game: &mut Game, row: usize, col: usize) {
        let (x, y, w, h) = brick_rect(row, col);
        game.ball = Ball {
            x: x + w / 2.0,
            y: y + h + BALL_SIZE / 2.0,
            vx: 0.0,
            vy: -BALL_SPEED,
        };
        game.phase = Phase::Playing;
    }

    /// Leaves a single brick of `kind` standing and aims the ball at it, so a
    /// test can drive that one contact through the real `step` path.
    fn place_brick(game: &mut Game, kind: Kind, row: usize, col: usize) {
        for cell in game.bricks.iter_mut() {
            *cell = None;
        }
        game.bricks[row * BRICK_COLS + col] = Some(Cell::of(kind));
        game.bricks_left = if kind.destructible() { 1 } else { 0 };
        aim_below(game, row, col);
    }

    /// Leaves a single normal brick standing, ready to be broken next step.
    fn one_brick_left(game: &mut Game, row: usize, col: usize) {
        place_brick(game, Kind::Normal, row, col);
    }

    #[test]
    fn clearing_an_ordinary_wall_opens_a_draft_then_brings_up_the_next() {
        let mut game = Game::new_run(1, &Pool::base());
        game.score = 40;
        assert!(!game.on_guardian(), "a depth opens on an ordinary wall");

        one_brick_left(&mut game, 7, 7);
        let events = game.step(Input::default());

        assert!(events.wall_cleared, "an ordinary wall clearing is reported");
        assert!(!events.guardian_cleared && !events.won);
        assert_eq!(game.depth(), 1, "an ordinary wall keeps the same depth");
        assert_eq!(game.phase(), Phase::Drafting, "clearing opens a boon draft");
        assert_eq!(game.score(), 41, "the last brick still scores");

        // Taking a boon brings up the next wall, full, and serves.
        game.step(Input {
            confirm: true,
            ..Default::default()
        });
        assert_eq!(
            game.bricks().count(),
            BRICK_ROWS * BRICK_COLS,
            "the next wall starts full of bricks"
        );
        assert!(
            game.bricks_left() > 0,
            "the next wall has destructible bricks to clear"
        );
        assert_eq!(
            game.phase(),
            Phase::Serving,
            "the next wall waits on a serve"
        );
    }

    #[test]
    fn clearing_a_guardian_descends_a_depth_and_banks_a_life() {
        let mut game = Game::new_run(1, &Pool::base());
        // Advance to this depth's guardian, one life already spent.
        game.wall_in_depth = ORDINARY_WALLS_PER_DEPTH;
        game.lives = LIVES_START - 1;
        assert!(game.on_guardian());

        one_brick_left(&mut game, 7, 7);
        let events = game.step(Input::default());

        assert!(events.guardian_cleared, "a guardian clearing is reported");
        assert!(!events.wall_cleared && !events.won);
        assert_eq!(game.depth(), 2, "clearing a guardian descends a depth");
        assert!(
            !game.on_guardian(),
            "the next depth opens on an ordinary wall"
        );
        assert_eq!(game.lives(), LIVES_START, "a cleared guardian banks a life");
        assert_eq!(game.phase(), Phase::Drafting, "clearing opens a boon draft");
    }

    #[test]
    fn a_banked_life_never_exceeds_the_cap() {
        let mut game = Game::new_run(1, &Pool::base());
        game.wall_in_depth = ORDINARY_WALLS_PER_DEPTH;
        game.lives = LIVES_CAP;

        one_brick_left(&mut game, 7, 7);
        game.step(Input::default());

        assert_eq!(game.lives(), LIVES_CAP, "banked lives never pass the cap");
    }

    #[test]
    fn clearing_the_final_guardian_wins_the_run() {
        let mut game = Game::new_run(1, &Pool::base());
        // The last depth's guardian.
        game.depth = DEPTHS - 1;
        game.wall_in_depth = ORDINARY_WALLS_PER_DEPTH;
        assert!(game.on_guardian());

        one_brick_left(&mut game, 7, 7);
        let events = game.step(Input::default());

        assert!(events.won, "the final guardian wins the run");
        assert!(!events.guardian_cleared);
        assert_eq!(game.phase(), Phase::Won);
        assert_eq!(game.depth(), DEPTHS, "a won run reached the final depth");
    }

    #[test]
    fn an_armoured_brick_cracks_on_the_first_hit_and_breaks_on_the_second() {
        let mut game = Game::new_run(2, &Pool::base());
        place_brick(&mut game, Kind::Armoured, 7, 7);
        // A keeper brick elsewhere, so breaking the armoured one does not empty
        // the wall (which would rebuild it and mask the result).
        game.bricks[0] = Some(Cell::of(Kind::Normal));
        game.bricks_left = 2;

        let first = game.step(Input::default());
        assert!(
            first.brick_hit && first.brick_broken.is_none(),
            "the first hit cracks the armoured brick, not breaks it"
        );
        assert_eq!(
            game.bricks_left(),
            2,
            "a cracked armoured brick has not broken"
        );
        let target = game
            .bricks()
            .find(|b| b.kind == Kind::Armoured)
            .expect("the armoured brick still stands");
        assert!(target.damaged, "a cracked armoured brick reads as damaged");

        aim_below(&mut game, 7, 7);
        let second = game.step(Input::default());
        assert_eq!(
            second.brick_broken,
            Some(band_of(7)),
            "the second hit breaks it and scores its band"
        );
        assert_eq!(
            game.bricks_left(),
            1,
            "breaking it leaves the keeper standing"
        );
        assert!(
            game.bricks().all(|b| b.kind != Kind::Armoured),
            "the armoured brick is gone"
        );
    }

    #[test]
    fn a_mirror_sends_the_ball_straight_back_and_never_breaks() {
        let mut game = Game::new_run(3, &Pool::base());
        place_brick(&mut game, Kind::Mirror, 7, 7);
        // Approach the underside moving up and to the right.
        let (x, y, _w, h) = brick_rect(7, 7);
        game.ball = Ball {
            x,
            y: y + h + BALL_SIZE / 2.0,
            vx: 60.0,
            vy: -BALL_SPEED,
        };
        game.phase = Phase::Playing;

        let events = game.step(Input::default());
        assert!(
            events.brick_hit && events.brick_broken.is_none(),
            "a mirror is struck, never broken"
        );
        assert_eq!(game.bricks().count(), 1, "a mirror stays standing");
        assert_eq!(game.bricks().next().unwrap().kind, Kind::Mirror);
        assert_eq!(
            game.bricks_left(),
            0,
            "a mirror is not counted toward the clear"
        );
        let ball = game.ball();
        assert!(
            ball.vy > 0.0 && ball.vx < 0.0,
            "the mirror reverses both axes, sending the ball straight back"
        );
    }

    #[test]
    fn an_explosive_brick_chains_its_break_to_its_neighbours() {
        let mut game = Game::new_run(2, &Pool::base());
        for cell in game.bricks.iter_mut() {
            *cell = None;
        }
        // An explosive with three neighbours (row 7 is the bottom row), plus a
        // far keeper so the blast doesn't empty the wall.
        game.bricks[7 * BRICK_COLS + 7] = Some(Cell::of(Kind::Explosive));
        game.bricks[7 * BRICK_COLS + 6] = Some(Cell::of(Kind::Normal));
        game.bricks[7 * BRICK_COLS + 8] = Some(Cell::of(Kind::Normal));
        game.bricks[6 * BRICK_COLS + 7] = Some(Cell::of(Kind::Normal));
        game.bricks[0] = Some(Cell::of(Kind::Normal));
        game.bricks_left = 5;
        aim_below(&mut game, 7, 7);

        let events = game.step(Input::default());
        assert!(
            events.exploded,
            "an explosive break is reported as an explosion"
        );
        assert_eq!(events.brick_broken, Some(band_of(7)));
        assert_eq!(
            game.bricks_left(),
            1,
            "the blast takes the explosive and its three neighbours, leaving the keeper"
        );
        assert!(game.bricks().all(|b| b.kind != Kind::Explosive));
    }

    #[test]
    fn a_mover_slides_along_its_row_and_turns_at_an_edge() {
        let mut game = Game::new_run(1, &Pool::base());
        for cell in game.bricks.iter_mut() {
            *cell = None;
        }
        // A right-moving mover with room to its right slides one cell.
        let start = 4 * BRICK_COLS + 4;
        game.bricks[start] = Some(Cell {
            kind: Kind::Mover,
            hits: 1,
            dir: 1,
        });
        game.advance_movers();
        assert!(game.bricks[start].is_none(), "the mover left its cell");
        assert!(
            matches!(game.bricks[start + 1], Some(c) if c.kind == Kind::Mover),
            "it slid one cell to the right"
        );

        // Against the right edge it reverses instead of leaving the row.
        for cell in game.bricks.iter_mut() {
            *cell = None;
        }
        let edge = 4 * BRICK_COLS + (BRICK_COLS - 1);
        game.bricks[edge] = Some(Cell {
            kind: Kind::Mover,
            hits: 1,
            dir: 1,
        });
        game.advance_movers();
        assert!(
            matches!(game.bricks[edge], Some(c) if c.dir == -1),
            "at the edge the mover turns around in place"
        );
    }

    #[test]
    fn a_spawner_refills_an_adjacent_empty_cell() {
        let mut game = Game::new_run(1, &Pool::base());
        for cell in game.bricks.iter_mut() {
            *cell = None;
        }
        let spot = 4 * BRICK_COLS + 4;
        game.bricks[spot] = Some(Cell::of(Kind::Spawner));
        game.bricks_left = 1;
        let below = 5 * BRICK_COLS + 4;
        assert!(game.bricks[below].is_none());

        game.advance_spawners();
        assert!(
            matches!(game.bricks[below], Some(c) if c.kind == Kind::Normal),
            "the spawner refilled the cell below it"
        );
        assert_eq!(
            game.bricks_left(),
            2,
            "the spawned brick counts toward the clear"
        );
    }

    #[test]
    fn guardian_templates_are_well_formed() {
        for template in GUARDIANS {
            assert_eq!(
                template.len(),
                BRICK_ROWS,
                "a guardian has {BRICK_ROWS} rows"
            );
            let mut destructibles = 0;
            for line in template {
                assert_eq!(
                    line.chars().count(),
                    BRICK_COLS,
                    "a guardian row is {BRICK_COLS} wide"
                );
                for ch in line.chars() {
                    assert!(".NAMEVS".contains(ch), "unexpected guardian glyph {ch:?}");
                    if kind_from_char(ch).is_some_and(Kind::destructible) {
                        destructibles += 1;
                    }
                }
            }
            assert!(destructibles > 0, "a guardian must be clearable");
        }
    }

    #[test]
    fn a_guardian_wall_lays_out_its_designed_set_piece() {
        // Depth 0's guardian is laid out exactly from its template, not randomly.
        let mut game = Game::new_run(1, &Pool::base());
        game.wall_in_depth = ORDINARY_WALLS_PER_DEPTH;
        game.build_wall();
        assert!(game.on_guardian());
        let expected: Vec<Option<Kind>> = GUARDIANS[0]
            .iter()
            .flat_map(|line| line.chars())
            .map(kind_from_char)
            .collect();
        let actual: Vec<Option<Kind>> = game
            .bricks
            .iter()
            .map(|c| c.as_ref().map(|cell| cell.kind))
            .collect();
        assert_eq!(actual, expected, "the guardian matches its template");

        // The deepest guardian is distinctly tougher — a solid armoured top row.
        let mut deep = Game::new_run(1, &Pool::base());
        deep.depth = DEPTHS - 1;
        deep.wall_in_depth = ORDINARY_WALLS_PER_DEPTH;
        deep.build_wall();
        for col in 0..BRICK_COLS {
            assert_eq!(
                deep.bricks[col].map(|cell| cell.kind),
                Some(Kind::Armoured),
                "the final guardian's top row is solid armour"
            );
        }
    }

    /// Clears a wall to open the boon draft, leaving the game in `Drafting`.
    fn open_draft(game: &mut Game) {
        one_brick_left(game, 7, 7);
        let events = game.step(Input::default());
        assert!(
            events.wall_cleared || events.guardian_cleared,
            "clearing a wall opens the draft"
        );
        assert_eq!(game.phase(), Phase::Drafting);
    }

    #[test]
    fn clearing_a_wall_opens_a_draft_of_distinct_boons() {
        let mut game = Game::new_run(1, &Pool::base());
        open_draft(&mut game);
        let offers = game.offers();
        assert_eq!(offers.len(), DRAFT_OFFERS, "three boons are offered");
        assert!(
            offers[0] != offers[1] && offers[1] != offers[2] && offers[0] != offers[2],
            "the offers are distinct"
        );
    }

    #[test]
    fn the_draft_highlight_moves_with_the_paddle_keys() {
        let mut game = Game::new_run(1, &Pool::base());
        open_draft(&mut game);
        assert_eq!(game.draft_highlight(), 0);
        game.step(Input {
            paddle: Move::Right,
            ..Default::default()
        });
        assert_eq!(game.draft_highlight(), 1);
        game.step(Input {
            paddle: Move::Left,
            ..Default::default()
        });
        assert_eq!(game.draft_highlight(), 0);
        game.step(Input {
            paddle: Move::Left,
            ..Default::default()
        });
        assert_eq!(
            game.draft_highlight(),
            DRAFT_OFFERS - 1,
            "left wraps around"
        );
    }

    #[test]
    fn taking_a_boon_records_it_and_leaves_the_draft() {
        let mut game = Game::new_run(1, &Pool::base());
        open_draft(&mut game);
        let picked = game.offers()[0];
        game.step(Input {
            confirm: true,
            ..Default::default()
        });
        assert_eq!(game.boons(), &[picked], "the taken boon is recorded");
        assert_ne!(
            game.phase(),
            Phase::Drafting,
            "taking a boon leaves the draft"
        );
    }

    #[test]
    fn a_reroll_spends_a_reroll_and_offers_afresh() {
        let mut game = Game::new_run(1, &Pool::base());
        open_draft(&mut game);
        let rerolls = game.rerolls();
        assert!(rerolls > 0, "a run starts with rerolls");
        game.step(Input {
            reroll: true,
            ..Default::default()
        });
        assert_eq!(game.rerolls(), rerolls - 1, "a reroll is spent");
        assert_eq!(game.phase(), Phase::Drafting, "a reroll stays in the draft");
        assert_eq!(game.offers().len(), DRAFT_OFFERS, "fresh offers are rolled");
    }

    #[test]
    fn an_extra_life_boon_grants_a_life() {
        let pool = Pool {
            specials: vec![],
            boons: vec![Boon::ExtraLife],
        };
        let mut game = Game::new_run(1, &pool);
        let lives = game.lives();
        open_draft(&mut game);
        game.step(Input {
            confirm: true,
            ..Default::default()
        });
        assert_eq!(game.lives(), lives + 1, "Extra Life grants a life");
    }

    #[test]
    fn boons_apply_and_stack_across_the_run() {
        // A single-boon pool so every offer is Widen.
        let pool = Pool {
            specials: vec![],
            boons: vec![Boon::Widen],
        };
        let mut game = Game::new_run(1, &pool);
        let base = game.paddle().width;

        open_draft(&mut game);
        game.step(Input {
            confirm: true,
            ..Default::default()
        });
        let after_one = game.paddle().width;
        assert!(after_one > base, "Widen widens the paddle");

        open_draft(&mut game);
        game.step(Input {
            confirm: true,
            ..Default::default()
        });
        assert!(
            game.paddle().width > after_one,
            "a second Widen stacks on the first"
        );
        assert_eq!(game.boons().len(), 2, "both boons are recorded");
    }
}
