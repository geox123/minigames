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

/// Thickness of a paddle, in logical units.
pub const PADDLE_WIDTH: f32 = 4.0;
/// Length of a paddle, in logical units — a sixth of the field, as the
/// original's was, and a whole number of segments (eight of five units each).
pub const PADDLE_HEIGHT: f32 = 40.0;
/// Gap between a paddle and the side wall behind it.
pub const PADDLE_INSET: f32 = 16.0;
/// How fast a paddle travels, in logical units per second.
pub const PADDLE_SPEED: f32 = 180.0;
/// How much of the top of the field a paddle can never cover.
///
/// The 1972 original's paddles could not reach the top of the screen — a
/// limitation of the hardware that Atari shipped and players learned to use.
/// The Faithful keeps it.
pub const PADDLE_TOP_GAP: f32 = 8.0;

/// Which player, and which end of the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The player on the left of the field.
    Left,
    /// The player on the right of the field.
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

    /// Which way this side lies along the field: -1 for the left, +1 for the
    /// right. A serve travels towards `side.sign()`; a ball leaves a paddle
    /// away from it, at `-side.sign()`.
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

/// A paddle, as the shell should draw it: the top-left corner of a
/// [`PADDLE_WIDTH`] by [`PADDLE_HEIGHT`] rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paddle {
    /// Left edge of the paddle.
    pub x: f32,
    /// Top edge of the paddle.
    pub y: f32,
}

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
    /// A paddle returned the ball.
    pub paddle_hit: bool,
    /// The ball rebounded off a wall.
    pub wall_bounce: bool,
    /// A player won the point.
    pub scored: Option<Side>,
}

/// Where a match is: waiting on a serve, in play, or over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The ball is waiting in the middle of the field for the next serve.
    Serving,
    /// The ball is in play.
    Rally,
    /// One player has reached [`WIN_SCORE`].
    GameOver {
        /// The player who won.
        winner: Side,
    },
}

/// Who is playing the right paddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Players {
    /// One player, with the computer on the right paddle.
    One,
    /// Two players at one keyboard.
    Two,
}

/// The score a player has to reach to win, as in the original.
pub const WIN_SCORE: u32 = 11;

/// How long the ball waits in the middle of the field before each serve.
pub const SERVE_PAUSE: f32 = 1.0;

/// The number of segments a paddle is read in. The original resolved the paddle
/// this coarsely, and playing to the segments is how a player aims.
const SEGMENTS: usize = 8;

/// The widest angle, in degrees, that the ends of a paddle send the ball out at.
const WIDEST_ANGLE: f32 = 45.0;

/// The speed the ball settles at as a rally goes on, and the number of returns
/// it takes to reach each — the original stepped the ball up twice.
const RALLY_SPEEDS: [f32; 3] = [BALL_SPEED, 175.0, 220.0];
const SPEED_UP_AFTER_HITS: [u32; 2] = [4, 12];

/// How the computer opponent plays.
///
/// It is a shade slower than a player; it only takes a fresh look at the ball
/// every [`OPPONENT_REACTION`] seconds and keeps moving on what it last saw in
/// between; it waits until the ball is genuinely coming at it before
/// committing; it plays the ball where it is rather than working out where a
/// bounce will put it; and it is content to be roughly right. Those add up to a
/// human's weaknesses — a fast ball struck off the end of a paddle changes
/// direction faster than the opponent notices, which is how a player beats it.
const OPPONENT_SPEED: f32 = PADDLE_SPEED * 0.9;
const OPPONENT_REACTION: f32 = 0.16;
const OPPONENT_REACTS_AT: f32 = LOGICAL_WIDTH * 0.35;
const OPPONENT_DEADZONE: f32 = 6.0;
/// How far off the ball the opponent aims, re-drawn on every serve.
const OPPONENT_AIM_DRIFT: f32 = 10.0;

/// The angle the ball leaves at when it lands on `segment`, counting from the
/// top of the paddle down.
fn segment_angle(segment: usize) -> f32 {
    let across = segment as f32 / (SEGMENTS - 1) as f32;
    (-WIDEST_ANGLE + 2.0 * WIDEST_ANGLE * across).to_radians()
}

/// How fast the ball travels once it has been returned `hits` times this point.
fn rally_speed(hits: u32) -> f32 {
    if hits >= SPEED_UP_AFTER_HITS[1] {
        RALLY_SPEEDS[2]
    } else if hits >= SPEED_UP_AFTER_HITS[0] {
        RALLY_SPEEDS[1]
    } else {
        RALLY_SPEEDS[0]
    }
}

/// A game of Pong.
pub struct Game {
    ball: Ball,
    /// Top edge of each paddle, indexed by [`Side::index`].
    paddles: [f32; 2],
    /// How many times the ball has been returned during this point.
    rally_hits: u32,
    /// Each player's score, indexed by [`Side::index`].
    scores: [u32; 2],
    phase: Phase,
    /// Seconds left of the pause before the next serve.
    serve_countdown: f32,
    /// Which player the next serve goes to. Serves alternate.
    serve_towards: Side,
    players: Players,
    /// How far off the ball the computer opponent is aiming this point.
    opponent_aim: f32,
    /// Where the opponent last decided to move its paddle to.
    opponent_target: f32,
    /// Seconds until the opponent next takes a fresh look at the ball.
    opponent_look_due: f32,
    rng: Rng,
}

/// The ball at rest in the middle of the field, waiting to be served.
const PARKED_BALL: Ball = Ball {
    x: LOGICAL_WIDTH / 2.0,
    y: LOGICAL_HEIGHT / 2.0,
    vx: 0.0,
    vy: 0.0,
};

impl Game {
    /// Starts a new game. The same seed always produces the same game.
    pub fn new(players: Players, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let serve_towards = if rng.flip() { Side::Left } else { Side::Right };
        Self {
            ball: PARKED_BALL,
            paddles: [(LOGICAL_HEIGHT - PADDLE_HEIGHT) / 2.0; 2],
            rally_hits: 0,
            scores: [0; 2],
            phase: Phase::Serving,
            serve_countdown: SERVE_PAUSE,
            serve_towards,
            players,
            opponent_aim: 0.0,
            opponent_target: LOGICAL_HEIGHT / 2.0,
            opponent_look_due: 0.0,
            rng,
        }
    }

    /// Clears the scores and starts the match again from the opening serve.
    pub fn restart(&mut self) {
        self.scores = [0; 2];
        self.paddles = [(LOGICAL_HEIGHT - PADDLE_HEIGHT) / 2.0; 2];
        self.phase = Phase::Serving;
        self.begin_serve();
    }

    /// The ball, as the shell should draw it. Between points it waits in the
    /// middle of the field, at rest.
    pub fn ball(&self) -> Ball {
        self.ball
    }

    /// One player's score.
    pub fn score(&self, side: Side) -> u32 {
        self.scores[side.index()]
    }

    /// Where the match is: waiting on a serve, in play, or over.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// One player's paddle, as the shell should draw it.
    pub fn paddle(&self, side: Side) -> Paddle {
        Paddle {
            x: match side {
                Side::Left => PADDLE_INSET,
                Side::Right => LOGICAL_WIDTH - PADDLE_INSET - PADDLE_WIDTH,
            },
            y: self.paddles[side.index()],
        }
    }

    /// Advances the game by exactly one [`TIMESTEP`].
    pub fn step(&mut self, input: Input) -> Events {
        // Everyone keeps hold of their paddle whatever the ball is doing. In a
        // one-player game the computer has the right paddle, and the right
        // player's keys do nothing.
        let (right_axis, right_speed) = match self.players {
            Players::Two => (input.right, PADDLE_SPEED),
            Players::One => {
                self.opponent_looks();
                (self.opponent_axis(), OPPONENT_SPEED)
            }
        };
        move_paddle(
            &mut self.paddles[Side::Left.index()],
            input.left,
            PADDLE_SPEED,
        );
        move_paddle(
            &mut self.paddles[Side::Right.index()],
            right_axis,
            right_speed,
        );

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

    /// Lets the computer opponent take a fresh look at the ball, if it is due
    /// one. Between looks it keeps playing what it last saw.
    fn opponent_looks(&mut self) {
        self.opponent_look_due -= TIMESTEP;
        if self.opponent_look_due > 0.0 {
            return;
        }
        self.opponent_look_due = OPPONENT_REACTION;

        // It commits only once the ball is coming its way and has crossed into
        // its half, and it plays the ball where it is — it does not work out
        // where a bounce will put it. Otherwise it drifts back to the middle,
        // which is where a player waits too.
        let watching = self.ball.vx > 0.0 && self.ball.x > OPPONENT_REACTS_AT;
        self.opponent_target = if watching {
            self.ball.y + self.opponent_aim
        } else {
            LOGICAL_HEIGHT / 2.0
        };
    }

    /// Which way the computer opponent pushes its paddle this step.
    fn opponent_axis(&self) -> Axis {
        let centre = self.paddles[Side::Right.index()] + PADDLE_HEIGHT / 2.0;
        if centre < self.opponent_target - OPPONENT_DEADZONE {
            Axis::Down
        } else if centre > self.opponent_target + OPPONENT_DEADZONE {
            Axis::Up
        } else {
            Axis::Hold
        }
    }

    /// Moves the ball for one step, and settles anything it ran into.
    fn advance_ball(&mut self) -> Events {
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

    /// The player who wins the point, if the ball has left the field behind
    /// their opponent.
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
            self.rally_hits = 0;
        } else {
            self.phase = Phase::Serving;
            self.begin_serve();
        }
    }

    /// Parks the ball in the middle of the field and starts the pause before it
    /// is served.
    fn begin_serve(&mut self) {
        self.ball = PARKED_BALL;
        self.rally_hits = 0;
        self.serve_countdown = SERVE_PAUSE;
    }

    /// Puts the ball back in play, at the opening speed and a shallow angle,
    /// towards whichever player is due to receive.
    fn serve(&mut self) {
        // The opponent takes a fresh view of where on the paddle it wants the
        // ball each point, so it does not play every rally identically.
        let drift = 2.0 * OPPONENT_AIM_DRIFT;
        self.opponent_aim = self.rng.below(drift as u32 + 1) as f32 - OPPONENT_AIM_DRIFT;

        // Only the middle four segments' angles, so a serve is always playable.
        let angle = segment_angle(SEGMENTS / 2 - 2 + self.rng.below(4) as usize);
        let towards = self.serve_towards.sign();

        self.ball.vx = RALLY_SPEEDS[0] * angle.cos() * towards;
        self.ball.vy = RALLY_SPEEDS[0] * angle.sin();
        self.phase = Phase::Rally;
    }

    /// Returns the ball if it reached `side`'s paddle during this step.
    ///
    /// The test is against the path the ball travelled, not just where it ended
    /// up, so a fast ball cannot slip through a paddle between two steps.
    fn strike_paddle(&mut self, side: Side, previous: Ball) -> bool {
        let half = BALL_SIZE / 2.0;
        let paddle = self.paddle(side);

        // The face the ball arrives at, and the ball's leading edge either side
        // of this step's move.
        let (face, before, after) = match side {
            Side::Left => (
                paddle.x + PADDLE_WIDTH,
                previous.x - half,
                self.ball.x - half,
            ),
            Side::Right => (paddle.x, previous.x + half, self.ball.x + half),
        };
        let reached_the_face = match side {
            Side::Left => previous.vx < 0.0 && before >= face && after <= face,
            Side::Right => previous.vx > 0.0 && before <= face && after >= face,
        };
        if !reached_the_face {
            return false;
        }

        // Where the ball was at the moment it reached the face.
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

        self.rally_hits += 1;
        let speed = rally_speed(self.rally_hits);
        let segment = ((contact - paddle.y) / (PADDLE_HEIGHT / SEGMENTS as f32)) as isize;
        let angle = segment_angle(segment.clamp(0, SEGMENTS as isize - 1) as usize);
        let away = -side.sign();

        self.ball.x = match side {
            Side::Left => face + half,
            Side::Right => face - half,
        };
        self.ball.y = contact;
        self.ball.vx = speed * angle.cos() * away;
        self.ball.vy = speed * angle.sin();
        true
    }
}

/// Moves one paddle for a step and keeps it within the reach the field allows.
fn move_paddle(y: &mut f32, axis: Axis, speed: f32) {
    let travel = speed * TIMESTEP;
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

    /// A whole number in `0..limit`.
    fn below(&mut self, limit: u32) -> u32 {
        (self.next_u64() >> 32) as u32 % limit
    }

    fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }
}
