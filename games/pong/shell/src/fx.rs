//! The Remix's feel: ball trails, contact particles, screen shake and hit-stop.
//!
//! This is pure presentation — it reacts to the events the core reports and to
//! where the balls are, and draws into the neon canvas. It never touches the
//! simulation, so none of it is tested; it is judged by eye.

use macroquad::prelude::*;
use pong_remix_core::{Ball, Events, LOGICAL_HEIGHT, LOGICAL_WIDTH, PADDLE_INSET, Side};

/// How many recent frames of ball positions the trail remembers.
const TRAIL_LEN: usize = 9;
/// Cap on live particles, so a wild multiball rally can't run away.
const MAX_PARTICLES: usize = 240;
/// Peak screen-shake amplitude, in logical units.
const SHAKE_MAX: f32 = 6.0;

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

/// All of a PULSE view's live effects.
pub struct Fx {
    /// Recent ball positions, newest last; each entry a frame's worth.
    trail: Vec<Vec<(f32, f32)>>,
    particles: Vec<Particle>,
    shake: f32,
    /// Seconds of hit-stop remaining; while positive the sim is frozen.
    hitstop: f32,
    /// A tiny deterministic generator so shake/particles need no real RNG.
    seed: u32,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            trail: Vec::with_capacity(TRAIL_LEN),
            particles: Vec::new(),
            shake: 0.0,
            hitstop: 0.0,
            seed: 0x1234_5678,
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

    /// Reacts to one simulation step: records the trail and throws particles,
    /// shake and hit-stop off whatever just happened.
    pub fn on_step(&mut self, events: Events, balls: &[Ball]) {
        // Trail: remember this frame's ball positions.
        self.trail.push(balls.iter().map(|b| (b.x, b.y)).collect());
        if self.trail.len() > TRAIL_LEN {
            self.trail.remove(0);
        }

        if events.paddle_hit
            && let Some(ball) = nearest_to_paddle(balls)
        {
            let colour = if ball.x < LOGICAL_WIDTH / 2.0 {
                color_u8!(60, 240, 255, 255)
            } else {
                color_u8!(255, 70, 200, 255)
            };
            let count = if events.power_hit { 22 } else { 9 };
            self.burst(
                ball.x,
                ball.y,
                count,
                colour,
                if events.power_hit { 130.0 } else { 70.0 },
            );
        }
        if events.wall_bounce
            && let Some(ball) = balls.first()
        {
            self.burst(ball.x, ball.y, 5, color_u8!(180, 160, 255, 255), 55.0);
        }
        if events.pickup {
            self.burst(
                LOGICAL_WIDTH / 2.0,
                balls.first().map(|b| b.y).unwrap_or(LOGICAL_HEIGHT / 2.0),
                14,
                color_u8!(255, 245, 120, 255),
                90.0,
            );
        }
        if events.power_hit {
            self.shake = (self.shake + SHAKE_MAX).min(SHAKE_MAX);
            self.hitstop = self.hitstop.max(0.05);
        }
        if let Some(scorer) = events.scored {
            // A burst at the breached goal, and a solid jolt.
            let x = match scorer {
                Side::Left => LOGICAL_WIDTH - 2.0,
                Side::Right => 2.0,
            };
            let y = balls.first().map(|b| b.y).unwrap_or(LOGICAL_HEIGHT / 2.0);
            self.burst(x, y, 26, color_u8!(255, 245, 120, 255), 120.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.08);
        }
    }

    fn burst(&mut self, x: f32, y: f32, count: u32, colour: Color, speed: f32) {
        for _ in 0..count {
            if self.particles.len() >= MAX_PARTICLES {
                break;
            }
            let (a, b) = (self.rand(), self.rand());
            let life = 0.25 + 0.25 * (b.abs());
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

    /// Draws the trails behind the balls, then the particles over the field.
    pub fn draw(&self, ball_size: f32) {
        // Trails: older frames fainter and smaller.
        for (age, frame) in self.trail.iter().enumerate() {
            let fade = (age as f32 + 1.0) / (self.trail.len() as f32 + 1.0);
            let alpha = fade * 0.35;
            let size = ball_size * fade;
            for &(x, y) in frame {
                draw_rectangle(
                    x - size / 2.0,
                    y - size / 2.0,
                    size,
                    size,
                    Color::new(1.0, 0.96, 0.47, alpha),
                );
            }
        }
        for p in &self.particles {
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            let mut c = p.colour;
            c.a = alpha;
            draw_rectangle(p.x - 1.0, p.y - 1.0, 2.0, 2.0, c);
        }
    }
}

/// The ball closest to either paddle's face — where a paddle hit landed.
fn nearest_to_paddle(balls: &[Ball]) -> Option<Ball> {
    balls
        .iter()
        .copied()
        .min_by(|a, b| dist_to_paddle(a).partial_cmp(&dist_to_paddle(b)).unwrap())
}

fn dist_to_paddle(ball: &Ball) -> f32 {
    let left = (ball.x - PADDLE_INSET).abs();
    let right = (ball.x - (LOGICAL_WIDTH - PADDLE_INSET)).abs();
    left.min(right)
}
