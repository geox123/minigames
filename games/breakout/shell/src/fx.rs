//! RIFT's feel: a speed-scaled ball trail, per-event contact particles, screen
//! shake and hit-stop.
//!
//! This is pure presentation — it reacts to the events the core reports and to
//! where the ball is, and draws into the canvas. It never touches the
//! simulation, so none of it is tested; it is judged by eye.

use breakout_remix_core::{BALL_SPEED, Ball, Events, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;

/// How many recent frames of ball position the trail remembers.
const TRAIL_LEN: usize = 10;
/// Cap on live particles, so a wild rally can't run away.
const MAX_PARTICLES: usize = 220;
/// Peak screen-shake amplitude, in logical units.
const SHAKE_MAX: f32 = 5.0;

/// The RIFT brick-band particle colours (teal, azure, violet, magenta).
const BANDS: [Color; 4] = [
    color_u8!(70, 200, 180, 255),
    color_u8!(80, 160, 240, 255),
    color_u8!(150, 110, 240, 255),
    color_u8!(230, 90, 200, 255),
];
const PADDLE_SPARK: Color = color_u8!(90, 230, 220, 255);
const WALL_SPARK: Color = color_u8!(150, 130, 230, 255);
const STRUCK_SPARK: Color = color_u8!(212, 226, 240, 255);
const BLAST_SPARK: Color = color_u8!(255, 170, 90, 255);
const GUARDIAN_SPARK: Color = color_u8!(240, 110, 210, 255);
const WIN_SPARK: Color = color_u8!(120, 240, 230, 255);

/// A speck thrown off a contact, fading as it flies.
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    colour: Color,
}

/// All of a RIFT view's live effects.
pub struct Fx {
    /// Recent ball positions, newest last.
    trail: Vec<(f32, f32)>,
    particles: Vec<Particle>,
    shake: f32,
    /// Seconds of hit-stop remaining; while positive the sim holds still.
    hitstop: f32,
    /// A tiny deterministic generator so shake and particles need no real RNG.
    seed: u32,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            trail: Vec::with_capacity(TRAIL_LEN),
            particles: Vec::new(),
            shake: 0.0,
            hitstop: 0.0,
            seed: 0x2b3c_4d5e,
        }
    }
}

impl Fx {
    fn rand(&mut self) -> f32 {
        // xorshift, mapped to -1..1.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Whether the simulation should hold still this frame (hit-stop).
    pub fn frozen(&self) -> bool {
        self.hitstop > 0.0
    }

    /// A small beat when a boon is drafted, felt as a nudge.
    pub fn beat(&mut self) {
        self.shake = (self.shake + 2.5).min(SHAKE_MAX);
    }

    /// Reacts to one simulation step: records the trail while the ball is `live`,
    /// and throws particles, shake and hit-stop off whatever just happened.
    pub fn on_step(&mut self, events: Events, ball: Ball, live: bool) {
        if live {
            self.trail.push((ball.x, ball.y));
            if self.trail.len() > TRAIL_LEN {
                self.trail.remove(0);
            }
        } else {
            self.trail.clear();
        }

        if events.paddle_hit {
            self.burst(ball.x, ball.y, 8, PADDLE_SPARK, 72.0);
        }
        if events.wall_bounce {
            self.burst(ball.x, ball.y, 4, WALL_SPARK, 52.0);
        }
        if let Some(band) = events.brick_broken {
            self.burst(ball.x, ball.y, 10, BANDS[(band as usize).min(3)], 82.0);
        }
        if events.brick_hit {
            self.burst(ball.x, ball.y, 5, STRUCK_SPARK, 60.0);
        }
        if events.exploded {
            self.burst(ball.x, ball.y, 24, BLAST_SPARK, 135.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.05);
        }
        if events.guardian_cleared {
            self.burst(ball.x, ball.y, 22, GUARDIAN_SPARK, 120.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.06);
        }
        if events.wall_cleared {
            self.burst(ball.x, ball.y, 12, WIN_SPARK, 96.0);
        }
        if events.lost_ball {
            self.shake = (self.shake + 3.0).min(SHAKE_MAX);
        }
        if events.won {
            self.burst(
                LOGICAL_WIDTH / 2.0,
                LOGICAL_HEIGHT / 2.0,
                32,
                WIN_SPARK,
                150.0,
            );
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.08);
        }
        if events.lost {
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.06);
        }
    }

    fn burst(&mut self, x: f32, y: f32, count: u32, colour: Color, speed: f32) {
        for _ in 0..count {
            if self.particles.len() >= MAX_PARTICLES {
                break;
            }
            let (a, b) = (self.rand(), self.rand());
            let life = 0.22 + 0.28 * b.abs();
            self.particles.push(Particle {
                x,
                y,
                vx: a * speed,
                vy: b * speed,
                life,
                max_life: life,
                colour,
            });
        }
    }

    /// Advances the effects by `dt` real seconds: particles fly and fade, the
    /// shake and hit-stop wind down.
    pub fn update(&mut self, dt: f32) {
        self.hitstop = (self.hitstop - dt).max(0.0);
        self.shake = (self.shake - dt * SHAKE_MAX * 4.0).max(0.0);
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    /// The current shake offset to blit the whole field by.
    pub fn shake_offset(&mut self) -> (f32, f32) {
        if self.shake <= 0.0 {
            return (0.0, 0.0);
        }
        (self.rand() * self.shake, self.rand() * self.shake)
    }

    /// Draws the ball's trail (brighter and longer the faster it flies), then the
    /// particles over the field.
    pub fn draw(&self, ball_size: f32, ball_speed: f32) {
        let intensity = (ball_speed / BALL_SPEED).clamp(0.7, 1.9);
        for (age, &(x, y)) in self.trail.iter().enumerate() {
            let fade = (age as f32 + 1.0) / (self.trail.len() as f32 + 1.0);
            let alpha = (fade * 0.4 * intensity).min(0.6);
            let size = ball_size * fade;
            draw_rectangle(
                x - size / 2.0,
                y - size / 2.0,
                size,
                size,
                Color::new(0.66, 0.92, 1.0, alpha),
            );
        }
        for p in &self.particles {
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            let mut c = p.colour;
            c.a = alpha;
            draw_rectangle(p.x - 1.0, p.y - 1.0, 2.0, 2.0, c);
        }
    }
}
