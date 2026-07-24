//! The pure, deterministic core of RIFT — Breakout's Remix.
//!
//! Like the Collection's other cores it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and advances in fixed timesteps
//! so a seed and a sequence of inputs always replay the same run.
//!
//! This is the **descent skeleton**: a paddle, a ball, and one wall of normal
//! bricks. Deflect the ball off the paddle to break bricks; clear the wall and a
//! fresh one comes up after a serve; drop the ball past the paddle and it is
//! reported and a new ball is served. The depth structure, guardians, lives,
//! boons and the brick zoo are layered on in later work — this establishes the
//! run loop and its single [`Game::step`] seam.
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

/// What the player is doing this step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// The paddle direction.
    pub paddle: Move,
}

/// The brick and boon types a run may draw on, passed *in* at construction so
/// the core never knows about "unlocks" — only the pool it is handed.
///
/// The skeleton run uses only normal bricks and no boons, so the base pool
/// currently carries no options; the brick zoo and the boon set fill it in later
/// work. It is kept and threaded now so those additions don't reshape the seam.
#[derive(Clone, Debug, Default)]
pub struct Pool {}

impl Pool {
    /// The base pool of the plain run: normal bricks, no boons.
    pub fn base() -> Self {
        Self::default()
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

/// A brick, as the shell should draw it: a rectangle in a colour band. Every
/// brick in the skeleton is a normal brick; the brick zoo adds behaving types.
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
    /// The last brick of a wall fell this step, bringing up a fresh wall.
    pub wall_cleared: bool,
    /// The ball fell past the paddle this step and a new ball was served.
    pub lost_ball: bool,
}

/// Where a run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The ball rests on the paddle, waiting to be served.
    Serving,
    /// The ball is in play.
    Playing,
}

/// A run of RIFT.
pub struct Game {
    ball: Ball,
    /// Centre of the paddle, in logical units.
    paddle_x: f32,
    paddle_width: f32,
    /// Whether each brick is still standing, indexed `row * BRICK_COLS + col`.
    bricks: Vec<bool>,
    /// Standing bricks remaining.
    bricks_left: u32,
    score: u32,
    /// Walls fully cleared so far this run.
    walls_cleared: u32,
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
            bricks: vec![true; BRICK_ROWS * BRICK_COLS],
            bricks_left: (BRICK_ROWS * BRICK_COLS) as u32,
            score: 0,
            walls_cleared: 0,
            phase: Phase::Serving,
            serve_countdown: SERVE_PAUSE,
            seed,
            pool: pool.clone(),
            rng: Rng::new(seed),
        };
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
        self.bricks
            .iter()
            .enumerate()
            .filter(|(_, present)| **present)
            .map(|(i, _)| {
                let (row, col) = (i / BRICK_COLS, i % BRICK_COLS);
                let (x, y, width, height) = brick_rect(row, col);
                Brick {
                    x,
                    y,
                    width,
                    height,
                    band: band_of(row),
                }
            })
    }

    /// Standing bricks remaining.
    pub fn bricks_left(&self) -> u32 {
        self.bricks_left
    }

    /// Walls fully cleared so far this run.
    pub fn walls_cleared(&self) -> u32 {
        self.walls_cleared
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

    /// Advances the run by exactly one [`TIMESTEP`].
    pub fn step(&mut self, input: Input) -> Events {
        self.move_paddle(input.paddle);

        match self.phase {
            Phase::Serving => {
                // The ball rides the paddle until the pause ends.
                self.ball.x = self.paddle_x;
                self.serve_countdown -= TIMESTEP;
                if self.serve_countdown <= 0.0 {
                    self.serve();
                }
                Events::default()
            }
            Phase::Playing => self.advance_ball(),
        }
    }

    fn move_paddle(&mut self, mv: Move) {
        let travel = PADDLE_SPEED * TIMESTEP;
        match mv {
            Move::Left => self.paddle_x -= travel,
            Move::Right => self.paddle_x += travel,
            Move::Hold => {}
        }
        let half = self.paddle_width / 2.0;
        self.paddle_x = self.paddle_x.clamp(half, LOGICAL_WIDTH - half);
    }

    fn advance_ball(&mut self) -> Events {
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

        let brick_broken = self.collide_bricks(previous);

        // The last brick of a wall brings up the next after a serve.
        let mut wall_cleared = false;
        if brick_broken.is_some() && self.bricks_left == 0 {
            wall_cleared = true;
            self.finish_wall();
        }

        // A fresh wall re-parks the ball, so only strike and drop while in play.
        let paddle_hit = self.phase == Phase::Playing && self.strike_paddle(previous);

        let mut lost_ball = false;
        if self.phase == Phase::Playing && self.ball.y - half > LOGICAL_HEIGHT {
            lost_ball = true;
            self.begin_ball();
        }

        Events {
            wall_bounce,
            paddle_hit,
            brick_broken,
            wall_cleared,
            lost_ball,
        }
    }

    /// Handles a wall being emptied: a fresh wall comes up and waits on a serve.
    fn finish_wall(&mut self) {
        self.walls_cleared += 1;
        for standing in self.bricks.iter_mut() {
            *standing = true;
        }
        self.bricks_left = (BRICK_ROWS * BRICK_COLS) as u32;
        self.begin_ball();
    }

    /// Breaks the first standing brick the ball is now overlapping, reflecting
    /// the ball off the face it came in through and scoring the brick's band.
    /// At most one brick per step. The ball's step is far smaller than a brick,
    /// so it can never pass through the wall.
    fn collide_bricks(&mut self, previous: Ball) -> Option<u8> {
        let half = BALL_SIZE / 2.0;
        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                let i = row * BRICK_COLS + col;
                if !self.bricks[i] {
                    continue;
                }
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
                if from_above || from_below {
                    self.ball.vy = -self.ball.vy;
                    self.ball.y = if from_above {
                        ry - half
                    } else {
                        ry + rh + half
                    };
                } else {
                    self.ball.vx = -self.ball.vx;
                    self.ball.x = if previous.x < self.ball.x {
                        rx - half
                    } else {
                        rx + rw + half
                    };
                }

                self.bricks[i] = false;
                self.bricks_left -= 1;
                let band = band_of(row);
                self.score += BAND_POINTS[band as usize];
                return Some(band);
            }
        }
        None
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
    //! Clearing a wall by honest play is impractical — a perfect paddle digs a
    //! channel and the ball then bounces in it forever — so this sets up the last
    //! standing brick and lets the real `step` path break it and bring up the
    //! next wall. Only the setup reaches inside; the transition itself runs
    //! through the same code the run does. Everything else is tested through the
    //! seam in `tests/`.
    use super::*;

    /// Leaves a single standing brick and puts the ball on course to break it on
    /// the next step, so a test can drive a wall empty.
    fn one_brick_left(game: &mut Game, row: usize, col: usize) {
        for standing in game.bricks.iter_mut() {
            *standing = false;
        }
        game.bricks[row * BRICK_COLS + col] = true;
        game.bricks_left = 1;

        let (x, y, w, h) = brick_rect(row, col);
        // Just below the brick, rising into its underside this step.
        game.ball = Ball {
            x: x + w / 2.0,
            y: y + h + BALL_SIZE / 2.0,
            vx: 0.0,
            vy: -BALL_SPEED,
        };
        game.phase = Phase::Playing;
    }

    #[test]
    fn clearing_the_wall_brings_up_a_fresh_one_after_a_serve() {
        let mut game = Game::new_run(1, &Pool::base());
        game.score = 40;

        one_brick_left(&mut game, 7, 7);
        let events = game.step(Input::default());

        assert!(events.wall_cleared, "clearing the last brick is reported");
        assert_eq!(game.walls_cleared(), 1);
        assert_eq!(
            game.bricks_left(),
            (BRICK_ROWS * BRICK_COLS) as u32,
            "the next wall starts full"
        );
        assert_eq!(
            game.phase(),
            Phase::Serving,
            "a fresh wall waits on a serve"
        );
        assert_eq!(
            game.score(),
            41,
            "the last brick still scores and score carries"
        );
    }
}
