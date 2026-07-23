//! The pure, deterministic core of PULSE — Pong's Remix.
//!
//! Like the Faithful's core it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and it advances in fixed
//! timesteps so a seed and a sequence of inputs always replay the same game.
//! This is the Versus baseline with spin, power shots and multiball; the other
//! modes build on top of it.

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

/// A pickup is a square this many logical units on a side.
pub const PICKUP_SIZE: f32 = 16.0;

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

/// Power shots — hold to charge, then commit. Charging and the cooldown after a
/// power shot both slow the paddle (the risk); a charged return flies faster
/// (the reward).
const CHARGE_TIME: f32 = 0.6;
const CHARGE_PADDLE_FACTOR: f32 = 0.5;
const COOLDOWN_TIME: f32 = 0.5;
const COOLDOWN_PADDLE_FACTOR: f32 = 0.55;
/// Speed of a charged return, in logical units per second.
const POWER_SPEED: f32 = 250.0;

/// Pickups spawn on the net at this cadence, and never nearer the top or bottom
/// than [`PICKUP_MARGIN`]. A collected Multiball splits an extra ball off at
/// [`SPLIT_ANGLE`] radians from the one that took it.
const SPAWN_INTERVAL: f32 = 3.5;
const PICKUP_MARGIN: f32 = 36.0;
const SPLIT_ANGLE: f32 = 0.5;

/// How long a Widen enlarges a paddle, and by how much of its length.
pub const WIDEN_TIME: f32 = 6.0;
const WIDEN_EXTRA: f32 = 0.6;
/// How long a Slow-mo slows the balls, and to what fraction of their speed.
pub const SLOWMO_TIME: f32 = 3.0;
const SLOWMO_FACTOR: f32 = 0.5;

/// The computer opponent. It plays the right paddle with a person's limits: a
/// touch slower than a player, it re-reads the balls only a few times a second
/// and keeps moving on what it last saw, it tracks whichever ball will reach it
/// soonest, and it plays that ball where it is rather than solving its curve.
/// Those add up to a beatable opponent — a fast ball struck off a paddle's edge
/// turns faster than it notices.
const OPPONENT_SPEED_FACTOR: f32 = 0.88;
const OPPONENT_REACTION: f32 = 0.18;
const OPPONENT_DEADZONE: f32 = 6.0;
const OPPONENT_AIM_DRIFT: f32 = 12.0;

/// Gauntlet — solo survival. You defend the left goal; the right side is a
/// wall. The balls speed up and multiply over time until one gets past you.
const GAUNTLET_ADD_BALL_EVERY: f32 = 9.0;
const GAUNTLET_MAX_BALLS: usize = 6;
/// How fast the balls ramp up, per second, and the ceiling on the multiplier.
const GAUNTLET_SPEEDUP_PER_SEC: f32 = 0.03;
const GAUNTLET_MAX_SPEED_MULT: f32 = 2.4;
/// Score is ten a return plus one a second survived.
const GAUNTLET_RETURN_POINTS: u32 = 10;

/// Which way a PULSE game is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Head to head, first to [`WIN_SCORE`].
    Versus,
    /// Solo survival against an escalating barrage.
    Gauntlet,
}

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
    /// The left player's paddle direction.
    pub left: Axis,
    /// The right player's paddle direction.
    pub right: Axis,
    /// Whether the left player is charging a power shot.
    pub charge_left: bool,
    /// Whether the right player is charging a power shot.
    pub charge_right: bool,
}

impl Input {
    fn charging(&self, side: Side) -> bool {
        match side {
            Side::Left => self.charge_left,
            Side::Right => self.charge_right,
        }
    }
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

/// What kind of pickup is on the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    /// Splits an extra ball into play.
    Multiball,
    /// A one-time barrier that saves the collector's goal once.
    Shield,
    /// Enlarges the collector's paddle for a while.
    Widen,
    /// Slows every ball for a while.
    SlowMo,
}

impl PickupKind {
    /// The four kinds, for a seeded spawn.
    const ALL: [PickupKind; 4] = [
        PickupKind::Multiball,
        PickupKind::Shield,
        PickupKind::Widen,
        PickupKind::SlowMo,
    ];
}

/// A pickup waiting to be collected, centred at `(x, y)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pickup {
    /// Horizontal centre.
    pub x: f32,
    /// Vertical centre.
    pub y: f32,
    /// What collecting it does.
    pub kind: PickupKind,
}

/// What happened during a single [`Game::step`], for the shell to react to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// A paddle returned a ball.
    pub paddle_hit: bool,
    /// The return was a charged power shot.
    pub power_hit: bool,
    /// A ball rebounded off the top or bottom wall.
    pub wall_bounce: bool,
    /// A ball was collected into a pickup.
    pub pickup: bool,
    /// A shield saved a goal this step.
    pub shield_saved: bool,
    /// A player won the point (a ball left the opposite goal).
    pub scored: Option<Side>,
}

/// Where a match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The ball waits in the middle for the next serve.
    Serving,
    /// A ball is in play.
    Rally,
    /// One player has reached [`WIN_SCORE`] (Versus).
    GameOver {
        /// The winner.
        winner: Side,
    },
    /// A Gauntlet run has ended — a ball got past the player.
    RunOver,
}

/// A ball in flight, together with the spin curving it and who last hit it.
#[derive(Clone, Copy)]
struct LiveBall {
    pos: Ball,
    spin: f32,
    /// The player who last returned this ball; a pickup it collects benefits
    /// them. `None` on a fresh serve.
    last_hit: Option<Side>,
}

/// The ball at rest in the middle, waiting to be served.
const PARKED_BALL: LiveBall = LiveBall {
    pos: Ball {
        x: LOGICAL_WIDTH / 2.0,
        y: LOGICAL_HEIGHT / 2.0,
        vx: 0.0,
        vy: 0.0,
    },
    spin: 0.0,
    last_hit: None,
};

/// A game of PULSE Versus.
pub struct Game {
    /// Every ball currently in play; always at least one.
    balls: Vec<LiveBall>,
    /// Top edge of each paddle, indexed by [`Side::index`].
    paddles: [f32; 2],
    /// Each paddle's vertical velocity this step, for spin at contact.
    paddle_vel: [f32; 2],
    /// Each player's power-shot charge, 0 to 1.
    charge: [f32; 2],
    /// Whether each player is charging this step.
    charging: [bool; 2],
    /// Seconds of post-power-shot cooldown left for each player.
    cooldown: [f32; 2],
    /// Whether each player holds a one-time goal shield.
    shield: [bool; 2],
    /// Seconds each player's paddle stays widened.
    widen_timer: [f32; 2],
    /// Seconds every ball stays slowed.
    slowmo_timer: f32,
    /// The pickup on the field, if any.
    pickup: Option<Pickup>,
    /// Seconds until the next pickup may spawn.
    spawn_timer: f32,
    /// Each player's score.
    scores: [u32; 2],
    phase: Phase,
    /// Seconds left of the pause before the next serve.
    serve_countdown: f32,
    /// Which player the next serve goes to.
    serve_towards: Side,
    /// Whether the computer plays the right paddle.
    opponent: bool,
    /// Where the opponent last decided to move its paddle.
    opp_target: f32,
    /// Seconds until the opponent next re-reads the balls.
    opp_look_due: f32,
    /// How far off the ball the opponent aims this point.
    opp_aim: f32,
    /// Whether this is a Versus or a Gauntlet game.
    mode: Mode,
    /// Seconds the current Gauntlet run has lasted.
    gauntlet_elapsed: f32,
    /// Returns the player has made this Gauntlet run.
    gauntlet_returns: u32,
    /// How much faster than base the Gauntlet balls fly.
    speed_mult: f32,
    /// Seconds until the Gauntlet adds another ball.
    add_ball_timer: f32,
    rng: Rng,
}

impl Game {
    /// Starts a new two-player Versus game. The same seed always replays.
    pub fn new(seed: u64) -> Self {
        Self::with_opponent(seed, false)
    }

    /// Starts a one-player Versus game: the computer plays the right paddle.
    pub fn new_versus_cpu(seed: u64) -> Self {
        Self::with_mode(seed, Mode::Versus, true)
    }

    /// Starts a Gauntlet run: solo survival against an escalating barrage.
    pub fn new_gauntlet(seed: u64) -> Self {
        Self::with_mode(seed, Mode::Gauntlet, false)
    }

    fn with_opponent(seed: u64, opponent: bool) -> Self {
        Self::with_mode(seed, Mode::Versus, opponent)
    }

    fn with_mode(seed: u64, mode: Mode, opponent: bool) -> Self {
        let mut rng = Rng::new(seed);
        // Gauntlet always serves at the player on the left.
        let serve_towards = match mode {
            Mode::Gauntlet => Side::Left,
            Mode::Versus if rng.flip() => Side::Left,
            Mode::Versus => Side::Right,
        };
        let mut game = Self {
            balls: vec![PARKED_BALL],
            paddles: [(LOGICAL_HEIGHT - PADDLE_HEIGHT) / 2.0; 2],
            paddle_vel: [0.0; 2],
            charge: [0.0; 2],
            charging: [false; 2],
            cooldown: [0.0; 2],
            shield: [false; 2],
            widen_timer: [0.0; 2],
            slowmo_timer: 0.0,
            pickup: None,
            spawn_timer: SPAWN_INTERVAL,
            scores: [0; 2],
            phase: Phase::Serving,
            serve_countdown: SERVE_PAUSE,
            serve_towards,
            opponent,
            opp_target: LOGICAL_HEIGHT / 2.0,
            opp_look_due: 0.0,
            opp_aim: 0.0,
            mode,
            gauntlet_elapsed: 0.0,
            gauntlet_returns: 0,
            speed_mult: 1.0,
            add_ball_timer: GAUNTLET_ADD_BALL_EVERY,
            rng,
        };
        game.begin_serve();
        game
    }

    /// Restarts the game (Versus) or the run (Gauntlet) from the beginning.
    pub fn restart(&mut self) {
        self.scores = [0; 2];
        self.paddles = [(LOGICAL_HEIGHT - PADDLE_HEIGHT) / 2.0; 2];
        self.charge = [0.0; 2];
        self.cooldown = [0.0; 2];
        self.shield = [false; 2];
        self.widen_timer = [0.0; 2];
        self.slowmo_timer = 0.0;
        self.gauntlet_elapsed = 0.0;
        self.gauntlet_returns = 0;
        self.speed_mult = 1.0;
        self.add_ball_timer = GAUNTLET_ADD_BALL_EVERY;
        self.begin_serve();
    }

    /// The current Gauntlet score: ten a return plus one a second survived.
    pub fn gauntlet_score(&self) -> u32 {
        self.gauntlet_returns * GAUNTLET_RETURN_POINTS + self.gauntlet_elapsed as u32
    }

    /// The primary ball. Between points it rests in the middle; with multiball
    /// in play this is simply the first of several — use [`Game::balls`] to draw
    /// them all.
    pub fn ball(&self) -> Ball {
        self.balls[0].pos
    }

    /// Every ball currently in play.
    pub fn balls(&self) -> impl Iterator<Item = Ball> + '_ {
        self.balls.iter().map(|b| b.pos)
    }

    /// The pickup on the field, if any.
    pub fn pickup(&self) -> Option<Pickup> {
        self.pickup
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

    /// A player's power-shot charge, from 0 (none) to 1 (ready).
    pub fn charge(&self, side: Side) -> f32 {
        self.charge[side.index()]
    }

    /// Whether a player's paddle is in its post-power-shot cooldown.
    pub fn cooling(&self, side: Side) -> bool {
        self.cooldown[side.index()] > 0.0
    }

    /// A player's paddle length now — longer while a Widen is active.
    pub fn paddle_height(&self, side: Side) -> f32 {
        let extra = if self.widen_timer[side.index()] > 0.0 {
            WIDEN_EXTRA
        } else {
            0.0
        };
        PADDLE_HEIGHT * (1.0 + extra)
    }

    /// Whether a player is holding an unused goal shield.
    pub fn has_shield(&self, side: Side) -> bool {
        self.shield[side.index()]
    }

    /// Whether the balls are currently slowed by a Slow-mo.
    pub fn slow_motion(&self) -> bool {
        self.slowmo_timer > 0.0
    }

    /// Advances the game by exactly one [`TIMESTEP`].
    pub fn step(&mut self, mut input: Input) -> Events {
        // In one-player games the computer drives the right paddle; the right
        // player's keys are ignored.
        if self.opponent {
            self.opponent_look();
            input.right = self.opponent_axis();
            input.charge_right = false;
        }

        self.update_charge(Side::Left, input.charging(Side::Left));
        self.update_charge(Side::Right, input.charging(Side::Right));
        for timer in &mut self.widen_timer {
            *timer = (*timer - TIMESTEP).max(0.0);
        }
        self.slowmo_timer = (self.slowmo_timer - TIMESTEP).max(0.0);

        let before = self.paddles;
        let (left_speed, mut right_speed) = (
            self.paddle_speed(Side::Left),
            self.paddle_speed(Side::Right),
        );
        if self.opponent {
            right_speed *= OPPONENT_SPEED_FACTOR;
        }
        let (left_height, right_height) = (
            self.paddle_height(Side::Left),
            self.paddle_height(Side::Right),
        );
        move_paddle(
            &mut self.paddles[Side::Left.index()],
            input.left,
            left_speed,
            left_height,
        );
        move_paddle(
            &mut self.paddles[Side::Right.index()],
            input.right,
            right_speed,
            right_height,
        );
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
            Phase::Rally => {
                if self.mode == Mode::Gauntlet {
                    self.escalate_gauntlet();
                }
                self.advance_balls()
            }
            Phase::GameOver { .. } | Phase::RunOver => Events::default(),
        }
    }

    /// Ramps a Gauntlet run: time and speed climb, and now and then another
    /// ball joins from the right wall.
    fn escalate_gauntlet(&mut self) {
        self.gauntlet_elapsed += TIMESTEP;
        self.speed_mult =
            (self.speed_mult + GAUNTLET_SPEEDUP_PER_SEC * TIMESTEP).min(GAUNTLET_MAX_SPEED_MULT);

        self.add_ball_timer -= TIMESTEP;
        if self.add_ball_timer <= 0.0 {
            self.add_ball_timer = GAUNTLET_ADD_BALL_EVERY;
            if self.balls.len() < GAUNTLET_MAX_BALLS {
                let y = self
                    .rng
                    .range(PICKUP_MARGIN, LOGICAL_HEIGHT - PICKUP_MARGIN);
                let angle = segment_angle(SEGMENTS / 2 - 2 + self.rng.below(4) as usize);
                // A fresh ball from the right wall, flying at the player.
                self.balls.push(LiveBall {
                    pos: Ball {
                        x: LOGICAL_WIDTH - BALL_SIZE,
                        y,
                        vx: -BALL_SPEED * angle.cos(),
                        vy: BALL_SPEED * angle.sin(),
                    },
                    spin: 0.0,
                    last_hit: None,
                });
            }
        }
    }

    /// Lets the opponent re-read the balls, if it is due a look. Between looks
    /// it keeps moving on what it last saw.
    fn opponent_look(&mut self) {
        self.opp_look_due -= TIMESTEP;
        if self.opp_look_due > 0.0 {
            return;
        }
        self.opp_look_due = OPPONENT_REACTION;

        // Track whichever ball heading its way will arrive soonest; if none is
        // coming, drift back to the middle where a player would wait too.
        let soonest = self.balls.iter().filter(|b| b.pos.vx > 0.0).min_by(|a, b| {
            arrival_time(&a.pos)
                .partial_cmp(&arrival_time(&b.pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.opp_target = match soonest {
            Some(ball) => ball.pos.y + self.opp_aim,
            None => LOGICAL_HEIGHT / 2.0,
        };
    }

    /// Which way the opponent pushes its paddle this step.
    fn opponent_axis(&self) -> Axis {
        let centre = self.paddles[Side::Right.index()] + self.paddle_height(Side::Right) / 2.0;
        if centre < self.opp_target - OPPONENT_DEADZONE {
            Axis::Down
        } else if centre > self.opp_target + OPPONENT_DEADZONE {
            Axis::Up
        } else {
            Axis::Hold
        }
    }

    /// Builds or drops a player's charge and runs down their cooldown.
    fn update_charge(&mut self, side: Side, charging: bool) {
        let i = side.index();
        self.charging[i] = charging;
        if charging {
            self.charge[i] = (self.charge[i] + TIMESTEP / CHARGE_TIME).min(1.0);
        } else {
            // Letting go before the return spends the charge.
            self.charge[i] = 0.0;
        }
        self.cooldown[i] = (self.cooldown[i] - TIMESTEP).max(0.0);
    }

    /// A paddle's speed this step: reduced while charging or cooling down.
    fn paddle_speed(&self, side: Side) -> f32 {
        let i = side.index();
        let factor = if self.charging[i] {
            CHARGE_PADDLE_FACTOR
        } else if self.cooldown[i] > 0.0 {
            COOLDOWN_PADDLE_FACTOR
        } else {
            1.0
        };
        PADDLE_SPEED * factor
    }

    /// Advances every ball one step: spin, motion, paddle and wall bounces,
    /// pickups, and scoring. In Versus a ball that leaves a goal scores; in
    /// Gauntlet only the left goal is a goal — the right side is a wall — and a
    /// ball getting past it ends the run.
    fn advance_balls(&mut self) -> Events {
        let mut events = Events::default();
        self.maybe_spawn_pickup();

        let gauntlet = self.mode == Mode::Gauntlet;
        let half = BALL_SIZE / 2.0;
        // Slow-mo scales the balls' motion down; the Gauntlet's ramp scales it
        // up. Both leave the paddles at full speed.
        let mut time_scale = if self.slowmo_timer > 0.0 {
            SLOWMO_FACTOR
        } else {
            1.0
        };
        if gauntlet {
            time_scale *= self.speed_mult;
        }
        let mut kept: Vec<LiveBall> = Vec::with_capacity(self.balls.len() + 1);
        let mut spawned: Vec<LiveBall> = Vec::new();
        let mut run_over = false;

        for mut lb in std::mem::take(&mut self.balls) {
            apply_spin(&mut lb, time_scale);
            let previous = lb.pos;
            lb.pos.x += lb.pos.vx * TIMESTEP * time_scale;
            lb.pos.y += lb.pos.vy * TIMESTEP * time_scale;

            // In Gauntlet only the player's (left) paddle is in play.
            let hit = if gauntlet {
                self.strike_ball(&mut lb, Side::Left, previous)
            } else {
                self.strike_ball(&mut lb, Side::Left, previous)
                    .or_else(|| self.strike_ball(&mut lb, Side::Right, previous))
            };
            if let Some(power) = hit {
                events.paddle_hit = true;
                events.power_hit |= power;
                if gauntlet {
                    self.gauntlet_returns += 1;
                }
            }

            if bounce_within(&mut lb.pos.y, &mut lb.pos.vy, half, LOGICAL_HEIGHT) {
                events.wall_bounce = true;
            }
            // The Gauntlet's right side is a wall the barrage bounces off.
            if gauntlet && lb.pos.x + half > LOGICAL_WIDTH {
                lb.pos.x = LOGICAL_WIDTH - half;
                lb.pos.vx = -lb.pos.vx.abs();
                events.wall_bounce = true;
            }

            if let Some(kind) = self.collect_pickup(&lb.pos) {
                events.pickup = true;
                if let Some(extra) = self.apply_pickup(kind, &lb) {
                    spawned.push(extra);
                }
            }

            if gauntlet {
                // Only the left goal can be breached; it ends the run.
                if lb.pos.x - half < 0.0 {
                    if self.shield[Side::Left.index()] {
                        self.shield[Side::Left.index()] = false;
                        events.shield_saved = true;
                        bounce_off_goal(&mut lb.pos, Side::Left);
                        kept.push(lb);
                    } else {
                        run_over = true;
                    }
                } else {
                    kept.push(lb);
                }
            } else {
                match past_the_field(&lb.pos) {
                    Some(scorer) => {
                        let defender = scorer.opposite();
                        if self.shield[defender.index()] {
                            // The shield saves the goal once, then is spent.
                            self.shield[defender.index()] = false;
                            events.shield_saved = true;
                            bounce_off_goal(&mut lb.pos, defender);
                            kept.push(lb);
                        } else {
                            self.scores[scorer.index()] += 1;
                            self.serve_towards = self.serve_towards.opposite();
                            events.scored = Some(scorer);
                        }
                    }
                    None => kept.push(lb),
                }
            }
        }

        kept.append(&mut spawned);
        self.balls = kept;

        if gauntlet {
            if run_over {
                self.phase = Phase::RunOver;
                self.balls = vec![PARKED_BALL];
                self.pickup = None;
            } else if self.balls.is_empty() {
                self.begin_serve();
            }
        } else if let Some(winner) = self.winner() {
            self.phase = Phase::GameOver { winner };
            self.balls = vec![PARKED_BALL];
            self.pickup = None;
        } else if self.balls.is_empty() {
            self.begin_serve();
        }

        events
    }

    fn winner(&self) -> Option<Side> {
        if self.scores[0] >= WIN_SCORE {
            Some(Side::Left)
        } else if self.scores[1] >= WIN_SCORE {
            Some(Side::Right)
        } else {
            None
        }
    }

    /// If `side`'s paddle reached `lb` this step, returns the ball off it and
    /// whether it was a charged power shot; otherwise `None`. Tests the path
    /// travelled, not just where the ball ended, so a fast ball can't tunnel.
    fn strike_ball(&mut self, lb: &mut LiveBall, side: Side, previous: Ball) -> Option<bool> {
        let half = BALL_SIZE / 2.0;
        let paddle = self.paddle(side);
        let height = self.paddle_height(side);

        let (face, before, after) = match side {
            Side::Left => (paddle.x + PADDLE_WIDTH, previous.x - half, lb.pos.x - half),
            Side::Right => (paddle.x, previous.x + half, lb.pos.x + half),
        };
        let reached = match side {
            Side::Left => previous.vx < 0.0 && before >= face && after <= face,
            Side::Right => previous.vx > 0.0 && before <= face && after >= face,
        };
        if !reached {
            return None;
        }

        let travelled = (before - after).abs();
        let contact = if travelled > f32::EPSILON {
            previous.y + (lb.pos.y - previous.y) * ((before - face).abs() / travelled)
        } else {
            lb.pos.y
        };
        let missed = contact + half <= paddle.y || contact - half >= paddle.y + height;
        if missed {
            return None;
        }

        // A fully-charged return is a power shot: faster, and it spends the
        // charge and starts the paddle's cooldown.
        let i = side.index();
        let power = self.charge[i] >= 1.0;
        let speed = if power {
            self.charge[i] = 0.0;
            self.cooldown[i] = COOLDOWN_TIME;
            POWER_SPEED
        } else {
            BALL_SPEED
        };

        let segment = ((contact - paddle.y) / (height / SEGMENTS as f32)) as isize;
        let angle = segment_angle(segment.clamp(0, SEGMENTS as isize - 1) as usize);
        let away = -side.sign();

        lb.pos.x = match side {
            Side::Left => face + half,
            Side::Right => face - half,
        };
        lb.pos.y = contact;
        lb.pos.vx = speed * angle.cos() * away;
        lb.pos.vy = speed * angle.sin();
        // The paddle's motion at contact bends the shot from here on, and the
        // ball now belongs to this player for pickup purposes.
        lb.spin = (self.paddle_vel[i] * SPIN_PER_PADDLE_VELOCITY).clamp(-SPIN_MAX, SPIN_MAX);
        lb.last_hit = Some(side);
        Some(power)
    }

    /// Applies a collected pickup, returning an extra ball if it spawned one.
    /// Effects benefit the player who last hit the collecting ball (or, off a
    /// serve, the side it is heading towards).
    fn apply_pickup(&mut self, kind: PickupKind, lb: &LiveBall) -> Option<LiveBall> {
        let beneficiary = lb.last_hit.unwrap_or(if lb.pos.vx >= 0.0 {
            Side::Right
        } else {
            Side::Left
        });
        match kind {
            PickupKind::Multiball => return Some(split_ball(lb)),
            PickupKind::Shield => self.shield[beneficiary.index()] = true,
            PickupKind::Widen => self.widen_timer[beneficiary.index()] = WIDEN_TIME,
            PickupKind::SlowMo => self.slowmo_timer = SLOWMO_TIME,
        }
        None
    }

    /// Spawns a pickup on the net if the field is clear and its timer is up.
    fn maybe_spawn_pickup(&mut self) {
        if self.pickup.is_some() {
            return;
        }
        self.spawn_timer -= TIMESTEP;
        if self.spawn_timer <= 0.0 {
            self.spawn_timer = SPAWN_INTERVAL;
            let y = self
                .rng
                .range(PICKUP_MARGIN, LOGICAL_HEIGHT - PICKUP_MARGIN);
            let kind = PickupKind::ALL[self.rng.below(PickupKind::ALL.len() as u32) as usize];
            self.pickup = Some(Pickup {
                x: LOGICAL_WIDTH / 2.0,
                y,
                kind,
            });
        }
    }

    /// Collects the pickup if `ball` overlaps it, returning what it was.
    fn collect_pickup(&mut self, ball: &Ball) -> Option<PickupKind> {
        let pickup = self.pickup?;
        let reach = (PICKUP_SIZE + BALL_SIZE) / 2.0;
        if (ball.x - pickup.x).abs() < reach && (ball.y - pickup.y).abs() < reach {
            self.pickup = None;
            Some(pickup.kind)
        } else {
            None
        }
    }

    fn begin_serve(&mut self) {
        self.balls = vec![PARKED_BALL];
        self.pickup = None;
        self.spawn_timer = SPAWN_INTERVAL;
        self.serve_countdown = SERVE_PAUSE;
        self.phase = Phase::Serving;
    }

    fn serve(&mut self) {
        // The opponent takes a fresh view of where on the paddle it wants the
        // ball each point, so it does not play every rally identically.
        if self.opponent {
            self.opp_aim = self.rng.range(-OPPONENT_AIM_DRIFT, OPPONENT_AIM_DRIFT);
            self.opp_look_due = 0.0;
        }

        // One of the middle four segments, so a serve is always playable.
        let angle = segment_angle(SEGMENTS / 2 - 2 + self.rng.below(4) as usize);
        let towards = self.serve_towards.sign();
        let ball = &mut self.balls[0].pos;
        ball.vx = BALL_SPEED * angle.cos() * towards;
        ball.vy = BALL_SPEED * angle.sin();
        self.phase = Phase::Rally;
    }
}

/// The angle the ball leaves at from `segment`, counting from the top down.
fn segment_angle(segment: usize) -> f32 {
    let across = segment as f32 / (SEGMENTS - 1) as f32;
    (-WIDEST_ANGLE + 2.0 * WIDEST_ANGLE * across).to_radians()
}

/// Roughly how long until `ball` reaches the right paddle's face, for the
/// opponent to pick the most urgent ball. Assumes it keeps its heading.
fn arrival_time(ball: &Ball) -> f32 {
    let face = LOGICAL_WIDTH - PADDLE_INSET - PADDLE_WIDTH;
    if ball.vx > 0.0 {
        (face - ball.x) / ball.vx
    } else {
        f32::INFINITY
    }
}

/// The player who wins the point if `ball` has left the opposite goal.
fn past_the_field(ball: &Ball) -> Option<Side> {
    let half = BALL_SIZE / 2.0;
    if ball.x + half < 0.0 {
        Some(Side::Right)
    } else if ball.x - half > LOGICAL_WIDTH {
        Some(Side::Left)
    } else {
        None
    }
}

/// Splits an extra ball off `lb`, sent out at [`SPLIT_ANGLE`] from it.
fn split_ball(lb: &LiveBall) -> LiveBall {
    let (sin, cos) = SPLIT_ANGLE.sin_cos();
    let (vx, vy) = (lb.pos.vx, lb.pos.vy);
    LiveBall {
        pos: Ball {
            vx: vx * cos - vy * sin,
            vy: vx * sin + vy * cos,
            ..lb.pos
        },
        spin: 0.0,
        last_hit: lb.last_hit,
    }
}

/// Bounces a ball back into play off `defender`'s goal, after a shield save.
fn bounce_off_goal(ball: &mut Ball, defender: Side) {
    let half = BALL_SIZE / 2.0;
    match defender {
        Side::Left => {
            ball.x = half;
            ball.vx = ball.vx.abs();
        }
        Side::Right => {
            ball.x = LOGICAL_WIDTH - half;
            ball.vx = -ball.vx.abs();
        }
    }
}

/// Rotates a ball's velocity by its spin for this step (scaled by `time_scale`
/// for slow-mo) and lets the spin decay, keeping the angle within a returnable
/// bound.
fn apply_spin(lb: &mut LiveBall, time_scale: f32) {
    if lb.spin == 0.0 {
        return;
    }
    let (sin, cos) = (lb.spin * TIMESTEP * time_scale).sin_cos();
    let (vx, vy) = (lb.pos.vx, lb.pos.vy);
    lb.pos.vx = vx * cos - vy * sin;
    lb.pos.vy = vx * sin + vy * cos;
    clamp_flight_angle(&mut lb.pos.vx, &mut lb.pos.vy);
    lb.spin *= SPIN_DECAY;
}

/// Keeps a ball's flight within [`MAX_FLIGHT_SIN`] of horizontal, so however
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

/// Moves one paddle of length `height` for a step at `speed` and keeps it within
/// the field's reach.
fn move_paddle(y: &mut f32, axis: Axis, speed: f32, height: f32) {
    let travel = speed * TIMESTEP;
    match axis {
        Axis::Up => *y -= travel,
        Axis::Down => *y += travel,
        Axis::Hold => {}
    }
    *y = y.clamp(PADDLE_TOP_GAP, LOGICAL_HEIGHT - height);
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

    /// A uniform float in `lo..hi`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * unit
    }

    fn below(&mut self, limit: u32) -> u32 {
        (self.next_u64() >> 32) as u32 % limit
    }

    fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }
}
