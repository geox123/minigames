//! The pure, deterministic core of the Breakout Faithful.
//!
//! Like the Collection's other cores it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and advances in fixed timesteps
//! so a seed and a sequence of inputs always replay the same game.
//!
//! So far: the paddle, the serve, contact-point deflection, and turns. Later
//! slices add the bricks, the speed-ups and the win.

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

/// Turns (balls) a game starts with.
pub const TURNS: u32 = 3;

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

/// The paddle, as the shell should draw it: the top-left corner of a `width` by
/// [`PADDLE_HEIGHT`] rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paddle {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Current width (it halves once the ball reaches the top wall).
    pub width: f32,
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
    /// The ball fell past the paddle and a turn was lost.
    pub lost_turn: bool,
}

/// Where a game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The ball rests on the paddle, waiting to be served.
    Serving,
    /// The ball is in play.
    Playing,
    /// Every turn has been spent.
    GameOver,
}

/// A game of Breakout.
pub struct Game {
    ball: Ball,
    /// Centre of the paddle, in logical units.
    paddle_x: f32,
    paddle_width: f32,
    turns: u32,
    phase: Phase,
    /// Seconds left of the pause before the serve.
    serve_countdown: f32,
    rng: Rng,
}

impl Game {
    /// Starts a new game. The same seed always produces the same game.
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            ball: PARKED_BALL,
            paddle_x: LOGICAL_WIDTH / 2.0,
            paddle_width: PADDLE_WIDTH,
            turns: TURNS,
            phase: Phase::Serving,
            serve_countdown: SERVE_PAUSE,
            rng: Rng::new(seed),
        };
        game.begin_turn();
        game
    }

    /// The ball, as the shell should draw it. Between turns it rests on the
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

    /// Turns remaining.
    pub fn turns(&self) -> u32 {
        self.turns
    }

    /// Where the game is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Advances the game by exactly one [`TIMESTEP`].
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
            Phase::GameOver => Events::default(),
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

        let paddle_hit = self.strike_paddle(previous);

        let mut lost_turn = false;
        if self.ball.y - half > LOGICAL_HEIGHT {
            lost_turn = true;
            self.lose_turn();
        }

        Events {
            wall_bounce,
            paddle_hit,
            lost_turn,
        }
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

    fn lose_turn(&mut self) {
        self.turns -= 1;
        if self.turns == 0 {
            self.phase = Phase::GameOver;
            self.ball = PARKED_BALL;
        } else {
            self.begin_turn();
        }
    }

    /// Rests the ball on the paddle and starts the pause before the serve.
    fn begin_turn(&mut self) {
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

/// The ball at rest (its resting place is set when a turn begins).
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
