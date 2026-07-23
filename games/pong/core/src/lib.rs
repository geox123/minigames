//! The pure, deterministic core of the Pong Faithful.
//!
//! The core owns every rule of the game and knows nothing about rendering,
//! audio, windows or wall-clock time. It advances in fixed timesteps of
//! [`TIMESTEP`] seconds, so a given starting seed and a given sequence of
//! inputs always produce exactly the same game.

/// Width of the play field, in logical units.
pub const LOGICAL_WIDTH: f32 = 320.0;
/// Height of the play field, in logical units.
pub const LOGICAL_HEIGHT: f32 = 240.0;

/// Length of a single simulation step, in seconds.
pub const TIMESTEP: f32 = 1.0 / 120.0;

/// The ball is a square this many logical units on a side.
pub const BALL_SIZE: f32 = 4.0;
/// Speed of the ball, in logical units per second.
pub const BALL_SPEED: f32 = 130.0;

/// The ball's position and velocity, in logical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    /// Horizontal centre of the ball.
    pub x: f32,
    /// Vertical centre of the ball.
    pub y: f32,
    /// Horizontal velocity, in units per second.
    pub vx: f32,
    /// Vertical velocity, in units per second.
    pub vy: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The ball rebounded off a wall.
    pub wall_bounce: bool,
}

/// A game of Pong.
pub struct Game {
    ball: Ball,
}

impl Game {
    /// Starts a new game. The same seed always produces the same game.
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let angle = rng.range(-0.4, 0.4);
        let direction = if rng.flip() { 1.0 } else { -1.0 };
        Self {
            ball: Ball {
                x: LOGICAL_WIDTH / 2.0,
                y: LOGICAL_HEIGHT / 2.0,
                vx: BALL_SPEED * angle.cos() * direction,
                vy: BALL_SPEED * angle.sin(),
            },
        }
    }

    /// The ball, as the shell should draw it.
    pub fn ball(&self) -> Ball {
        self.ball
    }

    /// Advances the game by exactly one [`TIMESTEP`].
    pub fn step(&mut self) -> Events {
        self.ball.x += self.ball.vx * TIMESTEP;
        self.ball.y += self.ball.vy * TIMESTEP;

        let half = BALL_SIZE / 2.0;
        let mut wall_bounce =
            bounce_within(&mut self.ball.y, &mut self.ball.vy, half, LOGICAL_HEIGHT);
        // The side walls only rebound until scoring arrives; see the Faithful
        // match rules.
        wall_bounce |= bounce_within(&mut self.ball.x, &mut self.ball.vx, half, LOGICAL_WIDTH);

        Events { wall_bounce }
    }
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
        // The zero state is a fixed point of xorshift, so it must be avoided.
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

    /// A uniform float in `lo..hi`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * unit
    }

    fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }
}
