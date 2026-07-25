//! ACCRETE's feel: an orbital ship trail, per-event particles, collapse shockwave
//! rings, screen shake and hit-stop — gravity made visible.
//!
//! This is pure presentation — it reacts to the events the core reports and to where
//! the ship, the boss and the central well are, and draws into the canvas. It never
//! touches the simulation, so none of it is tested; it is judged by eye (and the
//! aesthetic is the author's to tune).

use asteroids_remix_core::{Events, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;

/// How many recent frames of ship position the orbital trail remembers.
const TRAIL_LEN: usize = 16;
/// Cap on live particles, so a dense field can't run away.
const MAX_PARTICLES: usize = 320;
/// Peak screen-shake amplitude, in logical units.
const SHAKE_MAX: f32 = 6.0;
/// How fast a collapse shockwave ring expands, in units per second.
const RING_SPEED: f32 = 900.0;

const ACCRETE_SPARK: Color = color_u8!(255, 220, 130, 255);
const SKIM_SPARK: Color = color_u8!(120, 225, 255, 255);
const POWER_SPARK: Color = color_u8!(120, 255, 180, 255);
const SHIELD_SPARK: Color = color_u8!(120, 220, 255, 255);
const BOSS_SPARK: Color = color_u8!(255, 230, 90, 255);
const HIT_SPARK: Color = color_u8!(255, 100, 70, 255);
const COLLAPSE_SPARK: Color = color_u8!(235, 245, 255, 255);

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

/// An expanding shockwave ring — a collapse erupting from a well.
struct Ring {
    x: f32,
    y: f32,
    radius: f32,
    life: f32,
    max_life: f32,
}

/// All of an ACCRETE view's live effects.
pub struct Fx {
    /// Recent ship positions, newest last.
    trail: Vec<(f32, f32)>,
    particles: Vec<Particle>,
    rings: Vec<Ring>,
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
            rings: Vec::new(),
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

    /// Reacts to one simulation step: records the trail while the ship is `live`, and
    /// throws particles, rings, shake and hit-stop off whatever just happened — at the
    /// ship, the boss (`boss`, its centre), or the central well.
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
        let well = (LOGICAL_WIDTH / 2.0, LOGICAL_HEIGHT / 2.0);

        if events.accreted {
            self.burst(well, 3, ACCRETE_SPARK, 70.0);
        }
        if events.skimmed {
            self.burst(ship, 3, SKIM_SPARK, 70.0);
        }
        if events.power_up_taken {
            self.burst(ship, 12, POWER_SPARK, 95.0);
            self.shake = (self.shake + 2.0).min(SHAKE_MAX);
        }
        if events.enemy_destroyed {
            self.shake = (self.shake + 1.2).min(SHAKE_MAX);
        }
        if events.boss_hit {
            self.burst(boss.unwrap_or(well), 4, BOSS_SPARK, 65.0);
        }
        if events.boss_phase_changed {
            self.burst(boss.unwrap_or(well), 20, BOSS_SPARK, 120.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.05);
        }
        if events.shield_broke {
            self.burst(ship, 14, SHIELD_SPARK, 100.0);
            self.shake = (self.shake + 3.0).min(SHAKE_MAX);
        }
        if events.collapse_fired {
            self.rings.push(Ring {
                x: well.0,
                y: well.1,
                radius: 0.0,
                life: 0.6,
                max_life: 0.6,
            });
            self.burst(well, 40, COLLAPSE_SPARK, 170.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.09);
        }
        if events.ship_destroyed {
            self.burst(ship, 24, HIT_SPARK, 135.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.07);
        }
        if events.boss_cleared {
            self.burst(boss.unwrap_or(well), 34, BOSS_SPARK, 155.0);
            self.shake = SHAKE_MAX;
            self.hitstop = self.hitstop.max(0.09);
        }
        if events.game_over {
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

    /// Advances the effects by `dt` real seconds: particles fly and fade, the rings
    /// expand, the shake and hit-stop wind down.
    pub fn update(&mut self, dt: f32) {
        self.hitstop = (self.hitstop - dt).max(0.0);
        self.shake = (self.shake - dt * SHAKE_MAX * 4.0).max(0.0);
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for r in &mut self.rings {
            r.radius += RING_SPEED * dt;
            r.life -= dt;
        }
        self.rings.retain(|r| r.life > 0.0);
    }

    /// The current shake offset to blit the whole field by.
    pub fn shake_offset(&mut self) -> (f32, f32) {
        if self.shake <= 0.0 {
            return (0.0, 0.0);
        }
        (self.rand() * self.shake, self.rand() * self.shake)
    }

    /// Draws the ship's orbital trail, the collapse rings, and the live particles over
    /// the field.
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
                Color::new(1.0, 0.86, 0.5, alpha),
            );
        }
        for r in &self.rings {
            let alpha = (r.life / r.max_life).clamp(0.0, 1.0);
            draw_circle_lines(r.x, r.y, r.radius, 3.0, Color::new(0.92, 0.96, 1.0, alpha));
        }
        for p in &self.particles {
            let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
            let mut c = p.colour;
            c.a = alpha;
            draw_rectangle(p.x - 1.0, p.y - 1.0, 2.0, 2.0, c);
        }
    }
}
