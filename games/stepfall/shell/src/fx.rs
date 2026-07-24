//! HAILFALL's feel: a ship trail, per-event particles, screen shake and hit-stop.
//!
//! This is pure presentation — it reacts to the events the core reports and to
//! where the ship and the mothership are, and draws into the canvas. It never
//! touches the simulation, so none of it is tested; it is judged by eye.

use macroquad::prelude::*;
use stepfall_remix_core::{Events, LOGICAL_HEIGHT, LOGICAL_WIDTH};

/// How many recent frames of ship position the trail remembers.
const TRAIL_LEN: usize = 12;
/// Cap on live particles, so a dense storm can't run away.
const MAX_PARTICLES: usize = 280;
/// Peak screen-shake amplitude, in logical units.
const SHAKE_MAX: f32 = 5.0;

const GRAZE_SPARK: Color = color_u8!(120, 225, 255, 255);
const POWER_SPARK: Color = color_u8!(255, 240, 150, 255);
const NOVA_SPARK: Color = color_u8!(235, 245, 255, 255);
const BOSS_SPARK: Color = color_u8!(255, 110, 200, 255);
const HIT_SPARK: Color = color_u8!(255, 100, 70, 255);
const PHASE_SPARK: Color = color_u8!(255, 160, 90, 255);
const WIN_SPARK: Color = color_u8!(120, 240, 230, 255);

/// A speck thrown off an event, fading as it flies.
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    colour: Color,
}

/// All of a HAILFALL view's live effects.
pub struct Fx {
    /// Recent ship positions, newest last.
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
            seed: 0x51ed_2b3c,
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

    /// Reacts to one simulation step: records the trail while the ship is `live`,
    /// and throws particles, shake and hit-stop off whatever just happened at the
    /// ship, the mothership (`boss`, its centre) or the whole field.
    pub fn on_step(
        &mut self,
        events: Events,
        ship: (f32, f32),
        boss: Option<(f32, f32)>,
        live: bool,
    ) {
        if live {
            self.trail.push(ship);
            if self.trail.len() > TRAIL_LEN {
                self.trail.remove(0);
            }
        } else {
            self.trail.clear();
        }
        let centre = (LOGICAL_WIDTH / 2.0, LOGICAL_HEIGHT / 2.0);

        if events.grazed {
            self.burst(ship, 2, GRAZE_SPARK, 55.0);
        }
        if events.power_up_taken {
            self.burst(ship, 12, POWER_SPARK, 90.0);
            self.shake = (self.shake + 2.0).min(SHAKE_MAX);
        }
        if events.enemy_killed {
            // No position to place, but a small punch keeps clears feeling solid.
            self.shake = (self.shake + 1.2).min(SHAKE_MAX);
        }
        if events.boss_hit {
            self.burst(boss.unwrap_or(centre), 4, BOSS_SPARK, 60.0);
        }
        if events.boss_phase_changed {
            self.burst(boss.unwrap_or(centre), 20, PHASE_SPARK, 110.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.05);
        }
        if events.shield_broke {
            self.burst(ship, 14, GRAZE_SPARK, 100.0);
            self.shake = (self.shake + 3.0).min(SHAKE_MAX);
        }
        if events.overdrive_fired {
            self.burst(centre, 40, NOVA_SPARK, 165.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.08);
        }
        if events.player_hit {
            self.burst(ship, 22, HIT_SPARK, 130.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.07);
        }
        if events.boss_cleared {
            self.burst(boss.unwrap_or(centre), 34, BOSS_SPARK, 150.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.09);
        }
        if events.run_won {
            self.burst(centre, 40, WIN_SPARK, 150.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.10);
        }
        if events.run_over {
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.08);
        }
    }

    fn burst(&mut self, (x, y): (f32, f32), count: u32, colour: Color, speed: f32) {
        for _ in 0..count {
            if self.particles.len() >= MAX_PARTICLES {
                break;
            }
            let (a, b) = (self.rand(), self.rand());
            let life = 0.22 + 0.30 * b.abs();
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

    /// Draws the ship's trail and the live particles over the field.
    pub fn draw(&self) {
        for (age, &(x, y)) in self.trail.iter().enumerate() {
            let fade = (age as f32 + 1.0) / (self.trail.len() as f32 + 1.0);
            let alpha = (fade * 0.4).min(0.5);
            let size = 3.0 * fade;
            draw_rectangle(
                x - size / 2.0,
                y - size / 2.0,
                size,
                size,
                Color::new(0.35, 0.86, 1.0, alpha),
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
