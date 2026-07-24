//! The game's sound, composed from the shared synthesizer so the repo ships no
//! ripped or third-party audio (ADR 0002). Each voice is a short square wave (or
//! a slide), built as an in-memory WAV; the same bytes drive the native and
//! browser builds.

use breakout_core::Events;
use breakout_remix_core::Events as RiftEvents;
use macroquad::audio::{Sound, play_sound_once};
use shell_kit::synth::{blip, chirp};

/// One brick voice per colour band, low band to high.
const BANDS: usize = 4;

/// Every voice the Faithful shell can play.
pub struct Audio {
    paddle: Sound,
    wall: Sound,
    /// Brick breaks, one pitch per band — higher bands ring higher.
    brick: Vec<Sound>,
    /// The flourish when a wall is cleared (or the game won).
    cleared: Sound,
    /// The low tone when the ball is lost.
    lost: Sound,
    /// A hard tick when a brick is struck but not broken (armoured, mirror).
    clink: Sound,
    /// A low boom when an explosive brick chains.
    boom: Sound,
}

impl Audio {
    /// Synthesizes and loads every sound. Awaited once, before play.
    pub async fn load() -> Self {
        let mut brick = Vec::with_capacity(BANDS);
        for band in 0..BANDS {
            // Bottom band lowest, top band highest.
            let freq = 300.0 + 120.0 * band as f32;
            brick.push(blip(freq, 0.03).await);
        }
        Self {
            paddle: blip(220.0, 0.045).await,
            wall: blip(160.0, 0.03).await,
            brick,
            cleared: chirp(300.0, 820.0, 0.25).await,
            lost: chirp(300.0, 90.0, 0.35).await,
            clink: blip(540.0, 0.022).await,
            boom: chirp(220.0, 60.0, 0.18).await,
        }
    }

    /// Plays whatever a step produced. One contact voice sounds per step, chosen
    /// by what matters most — a lost ball, a cleared wall, a broken brick, then a
    /// plain paddle return — with a wall bounce chirping underneath.
    pub fn play(&self, events: Events) {
        if events.lost_turn {
            play_sound_once(&self.lost);
        } else if events.wall_cleared {
            play_sound_once(&self.cleared);
        } else if let Some(band) = events.brick_broken {
            play_sound_once(&self.brick[(band as usize).min(BANDS - 1)]);
        } else if events.paddle_hit {
            play_sound_once(&self.paddle);
        }
        if events.wall_bounce && !events.lost_turn {
            play_sound_once(&self.wall);
        }
    }

    /// Plays whatever a RIFT step produced. One contact voice sounds per step,
    /// chosen by what matters most — the run ending, a depth or wall cleared, a
    /// ball lost, a broken brick, a struck-but-unbroken brick, then a plain
    /// paddle return — with a wall bounce ticking underneath. RIFT reuses the
    /// Faithful's voices for now; its own voice character is the juice pass.
    pub fn play_rift(&self, events: RiftEvents) {
        if events.lost {
            play_sound_once(&self.lost);
        } else if events.won || events.guardian_cleared || events.wall_cleared {
            play_sound_once(&self.cleared);
        } else if events.lost_ball {
            play_sound_once(&self.lost);
        } else if events.exploded {
            play_sound_once(&self.boom);
        } else if let Some(band) = events.brick_broken {
            play_sound_once(&self.brick[(band as usize).min(BANDS - 1)]);
        } else if events.brick_hit {
            play_sound_once(&self.clink);
        } else if events.paddle_hit {
            play_sound_once(&self.paddle);
        }
        if events.wall_bounce && !events.lost_ball && !events.lost {
            play_sound_once(&self.wall);
        }
    }
}
