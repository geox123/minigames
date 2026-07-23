//! The pure, deterministic core of PULSE — Pong's Remix.
//!
//! Like the Faithful's core it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and it advances in fixed
//! timesteps so a seed and a sequence of inputs always replay the same game.
//! This is the Versus baseline; spin, power shots, pickups and the other modes
//! build on top of it.

/// Width of the play field, in logical units.
pub const LOGICAL_WIDTH: f32 = 320.0;
/// Height of the play field, in logical units.
pub const LOGICAL_HEIGHT: f32 = 240.0;

/// Length of a single simulation step, in seconds.
pub const TIMESTEP: f32 = 1.0 / 120.0;

/// The ball is a square this many logical units on a side.
pub const BALL_SIZE: f32 = 4.0;
/// Base speed of the ball, in logical units per second. PULSE runs a touch
/// hotter than the Faithful; power shots (not the rally) provide speed variance.
pub const BALL_SPEED: f32 = 150.0;

/// Thickness of a paddle, in logical units.
pub const PADDLE_WIDTH: f32 = 4.0;
/// Length of a paddle, in logical units.
pub const PADDLE_HEIGHT: f32 = 40.0;
/// Gap between a paddle and the side wall behind it.
pub const PADDLE_INSET: f32 = 16.0;
/// How fast a paddle travels, in logical units per second.
pub const PADDLE_SPEED: f32 = 190.0;
/// How much of the top of the field a paddle can never cover.
pub const PADDLE_TOP_GAP: f32 = 8.0;

/// The score a player must reach to win a Versus game.
pub const WIN_SCORE: u32 = 7;

/// How long the ball waits in the middle before each serve.
pub const SERVE_PAUSE: f32 = 0.8;

/// The number of segments a paddle is read in.
const SEGMENTS: usize = 8;
/// The widest angle, in degrees, the paddle's ends send the ball out at.
const WIDEST_ANGLE: f32 = 45.0;

/// Spin — the Remix's signature. A paddle's vertical velocity at contact bends
/// the ball's flight: the ball's velocity is rotated a little each step, curving
/// its path, and the spin decays over the shot. Speed is preserved (rotation,
/// not acceleration), and the flight angle is clamped so a spun ball never
/// stalls against a wall — it always keeps crossing the field.
const SPIN_MAX: f32 = 2.6;
const SPIN_PER_PADDLE_VELOCITY: f32 = SPIN_MAX / PADDLE_SPEED;
const SPIN_DECAY: f32 = 0.98;
/// The steepest the ball's flight may tilt from horizontal, as a sine.
const MAX_FLIGHT_SIN: f32 = 0.883; // sin(62°)

/// Which player, and which end of the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The player on the left.
    Left,
    /// The player on the right.
    Right,
}

impl Side {
    /// The other player.
    pub fn opposite(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }

    /// -1 for the left, +1 for the right; the field runs left to right.
    fn sign(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }
}

/// Which way a player is pushing their paddle this step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    /// Towards the top of the field.
    Up,
    /// Not moving.
    #[default]
    Hold,
    /// Towards the bottom of the field.
    Down,
}

/// What both players are doing this step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// The left player's paddle.
    pub left: Axis,
    /// The right player's paddle.
    pub right: Axis,
}

/// A paddle, as the shell draws it: the top-left corner of a `PADDLE_WIDTH` by
/// `PADDLE_HEIGHT` rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paddle {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
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
    /// A paddle returned the ball.
    pub paddle_hit: bool,
    /// The ball rebounded off the top or bottom wall.
    pub wall_bounce: bool,
    /// A player won the point.
    pub scored: Option<Side>,
}

/// Where a match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The ball waits in the middle for the next serve.
    Serving,
    /// The ball is in play.
    Rally,
    /// One player has reached [`WIN_SCORE`].
    GameOver {
        /// The winner.
        winner: Side,
    },
}

/// The ball at rest in the middle, waiting to be served.
const PARKED_BALL: Ball = Ball {
    x: LOGICAL_WIDTH / 2.0,
    y: LOGICAL_HEIGHT / 2.0,
    vx: 0.0,
    vy: 0.0,
};

/// A game of PULSE Versus.
pub struct Game {
    ball: Ball,
    /// Angular velocity curving the ball's flight, in radians per second.
    ball_spin: f32,
    /// Top edge of each paddle, indexed by [`Side::index`].
    paddles: [f32; 2],
    /// Each paddle's vertical velocity this step, for spin at contact.
    paddle_vel: [f32; 2],
    /// Each player's score.
    scores: [u32; 2],
    phase: Phase,
    /// Seconds left of the pause before the next serve.
    serve_countdown: f32,
    /// Which player the next serve goes to.
    serve_towards: Side,
    rng: Rng,
}

impl Game {
    /// Starts a new two-player Versus game. The same seed always replays.
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let serve_towards = if rng.flip() { Side::Left } else { Side::Right };
        let mut game = Self {
            ball: PARKED_BALL,
            ball_spin: 0.0,
            paddles: [(LOGICAL_HEIGHT - PADDLE_HEIGHT) / 2.0; 2],
            paddle_vel: [0.0; 2],
            scores: [0; 2],
            phase: Phase::Serving,
            serve_countdown: SERVE_PAUSE,
            serve_towards,
            rng,
        };
        game.begin_serve();
        game
    }

    /// Clears the scores and starts again from the opening serve.
    pub fn restart(&mut self) {
        self.scores = [0; 2];
        self.paddles = [(LOGICAL_HEIGHT - PADDLE_HEIGHT) / 2.0; 2];
        self.phase = Phase::Serving;
        self.begin_serve();
    }

    /// The ball, as the shell draws it. Between points it rests in the middle.
    pub fn ball(&self) -> Ball {
        self.ball
    }

    /// One player's paddle.
    pub fn paddle(&self, side: Side) -> Paddle {
        Paddle {
            x: match side {
                Side::Left => PADDLE_INSET,
                Side::Right => LOGICAL_WIDTH - PADDLE_INSET - PADDLE_WIDTH,
            },
            y: self.paddles[side.index()],
        }
    }

    /// One player's score.
    pub fn score(&self, side: Side) -> u32 {
        self.scores[side.index()]
    }

    /// Where the match is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Advances the game by exactly one [`TIMESTEP`].
    pub fn step(&mut self, input: Input) -> Events {
        let before = self.paddles;
        move_paddle(&mut self.paddles[Side::Left.index()], input.left);
        move_paddle(&mut self.paddles[Side::Right.index()], input.right);
        for (vel, (now, was)) in self
            .paddle_vel
            .iter_mut()
            .zip(self.paddles.iter().zip(before.iter()))
        {
            *vel = (now - was) / TIMESTEP;
        }

        match self.phase {
            Phase::Serving => {
                self.serve_countdown -= TIMESTEP;
                if self.serve_countdown <= 0.0 {
                    self.serve();
                }
                Events::default()
            }
            Phase::Rally => self.advance_ball(),
            Phase::GameOver { .. } => Events::default(),
        }
    }

    fn advance_ball(&mut self) -> Events {
        self.apply_spin();

        let previous = self.ball;
        self.ball.x += self.ball.vx * TIMESTEP;
        self.ball.y += self.ball.vy * TIMESTEP;

        let paddle_hit =
            self.strike_paddle(Side::Left, previous) || self.strike_paddle(Side::Right, previous);

        let half = BALL_SIZE / 2.0;
        let wall_bounce = bounce_within(&mut self.ball.y, &mut self.ball.vy, half, LOGICAL_HEIGHT);

        let scored = self.past_the_field();
        if let Some(scorer) = scored {
            self.award_point(scorer);
        }

        Events {
            paddle_hit,
            wall_bounce,
            scored,
        }
    }

    /// Returns the ball if it reached `side`'s paddle during this step. Tests
    /// the path travelled, not just where it ended, so a fast ball can't tunnel.
    fn strike_paddle(&mut self, side: Side, previous: Ball) -> bool {
        let half = BALL_SIZE / 2.0;
        let paddle = self.paddle(side);

        let (face, before, after) = match side {
            Side::Left => (
                paddle.x + PADDLE_WIDTH,
                previous.x - half,
                self.ball.x - half,
            ),
            Side::Right => (paddle.x, previous.x + half, self.ball.x + half),
        };
        let reached = match side {
            Side::Left => previous.vx < 0.0 && before >= face && after <= face,
            Side::Right => previous.vx > 0.0 && before <= face && after >= face,
        };
        if !reached {
            return false;
        }

        let travelled = (before - after).abs();
        let contact = if travelled > f32::EPSILON {
            previous.y + (self.ball.y - previous.y) * ((before - face).abs() / travelled)
        } else {
            self.ball.y
        };
        let missed = contact + half <= paddle.y || contact - half >= paddle.y + PADDLE_HEIGHT;
        if missed {
            return false;
        }

        let segment = ((contact - paddle.y) / (PADDLE_HEIGHT / SEGMENTS as f32)) as isize;
        let angle = segment_angle(segment.clamp(0, SEGMENTS as isize - 1) as usize);
        let away = -side.sign();

        self.ball.x = match side {
            Side::Left => face + half,
            Side::Right => face - half,
        };
        self.ball.y = contact;
        self.ball.vx = BALL_SPEED * angle.cos() * away;
        self.ball.vy = BALL_SPEED * angle.sin();
        // The paddle's motion at contact bends the shot from here on.
        self.ball_spin =
            (self.paddle_vel[side.index()] * SPIN_PER_PADDLE_VELOCITY).clamp(-SPIN_MAX, SPIN_MAX);
        true
    }

    /// Rotates the ball's velocity by its spin for this step and lets the spin
    /// decay, keeping the flight angle within a returnable bound.
    fn apply_spin(&mut self) {
        if self.ball_spin == 0.0 {
            return;
        }
        let (sin, cos) = (self.ball_spin * TIMESTEP).sin_cos();
        let (vx, vy) = (self.ball.vx, self.ball.vy);
        self.ball.vx = vx * cos - vy * sin;
        self.ball.vy = vx * sin + vy * cos;
        clamp_flight_angle(&mut self.ball.vx, &mut self.ball.vy);
        self.ball_spin *= SPIN_DECAY;
    }

    fn past_the_field(&self) -> Option<Side> {
        let half = BALL_SIZE / 2.0;
        if self.ball.x + half < 0.0 {
            Some(Side::Right)
        } else if self.ball.x - half > LOGICAL_WIDTH {
            Some(Side::Left)
        } else {
            None
        }
    }

    fn award_point(&mut self, scorer: Side) {
        self.scores[scorer.index()] += 1;
        self.serve_towards = self.serve_towards.opposite();

        if self.scores[scorer.index()] >= WIN_SCORE {
            self.phase = Phase::GameOver { winner: scorer };
            self.ball = PARKED_BALL;
        } else {
            self.phase = Phase::Serving;
            self.begin_serve();
        }
    }

    fn begin_serve(&mut self) {
        self.ball = PARKED_BALL;
        self.ball_spin = 0.0;
        self.serve_countdown = SERVE_PAUSE;
    }

    fn serve(&mut self) {
        // One of the middle four segments, so a serve is always playable.
        let angle = segment_angle(SEGMENTS / 2 - 2 + self.rng.below(4) as usize);
        let towards = self.serve_towards.sign();
        self.ball.vx = BALL_SPEED * angle.cos() * towards;
        self.ball.vy = BALL_SPEED * angle.sin();
        self.phase = Phase::Rally;
    }
}

/// The angle the ball leaves at from `segment`, counting from the top down.
fn segment_angle(segment: usize) -> f32 {
    let across = segment as f32 / (SEGMENTS - 1) as f32;
    (-WIDEST_ANGLE + 2.0 * WIDEST_ANGLE * across).to_radians()
}

/// Keeps the ball's flight within [`MAX_FLIGHT_SIN`] of horizontal, so however
/// much spin bends it, it always keeps enough horizontal speed to cross.
fn clamp_flight_angle(vx: &mut f32, vy: &mut f32) {
    let speed = vx.hypot(*vy);
    if speed <= f32::EPSILON {
        return;
    }
    let max_vy = speed * MAX_FLIGHT_SIN;
    if vy.abs() > max_vy {
        *vy = vy.signum() * max_vy;
        let horizontal = (speed * speed - *vy * *vy).max(0.0).sqrt();
        // Preserve the direction of travel; the ball never reverses on spin.
        let dir = if *vx >= 0.0 { 1.0 } else { -1.0 };
        *vx = dir * horizontal;
    }
}

/// Moves one paddle for a step and keeps it within the field's reach.
fn move_paddle(y: &mut f32, axis: Axis) {
    let travel = PADDLE_SPEED * TIMESTEP;
    match axis {
        Axis::Up => *y -= travel,
        Axis::Down => *y += travel,
        Axis::Hold => {}
    }
    *y = y.clamp(PADDLE_TOP_GAP, LOGICAL_HEIGHT - PADDLE_HEIGHT);
}

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
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, limit: u32) -> u32 {
        (self.next_u64() >> 32) as u32 % limit
    }

    fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }
}
