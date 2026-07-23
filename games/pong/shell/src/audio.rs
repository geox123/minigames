//! The match's sound, composed from the shared synthesizer so the repo ships no
//! ripped or third-party audio (ADR 0002). Each voice is a short square wave,
//! built as an in-memory WAV; the same bytes drive the native and browser
//! builds.

use macroquad::audio::{Sound, play_sound_once};
use pong_core::Events;
use pong_remix_core::Events as PulseEvents;
use shell_kit::synth::{blip, chirp};

/// How many pitches the PULSE paddle sound is synthesized at; a faster rally
/// picks a higher one, so the sound rises as the ball speeds up.
const PULSE_PADDLE_STEPS: usize = 6;

/// Every voice the shell can play — the Faithful's three, plus PULSE's.
pub struct Audio {
    paddle: Sound,
    wall: Sound,
    score: Sound,
    /// PULSE paddle hits, low to high pitch.
    pulse_paddle: Vec<Sound>,
    pulse_wall: Sound,
    pulse_power: Sound,
    pulse_pickup: Sound,
    pulse_score: Sound,
}

impl Audio {
    /// Synthesizes and loads every sound. Awaited once, before play.
    pub async fn load() -> Self {
        let mut pulse_paddle = Vec::with_capacity(PULSE_PADDLE_STEPS);
        for i in 0..PULSE_PADDLE_STEPS {
            let freq = 420.0 + 110.0 * i as f32;
            pulse_paddle.push(blip(freq, 0.045).await);
        }
        Self {
            paddle: blip(480.0, 0.05).await,
            wall: blip(240.0, 0.05).await,
            score: blip(120.0, 0.25).await,
            pulse_paddle,
            pulse_wall: blip(300.0, 0.04).await,
            pulse_power: chirp(200.0, 520.0, 0.14).await,
            pulse_pickup: chirp(520.0, 900.0, 0.12).await,
            pulse_score: chirp(300.0, 110.0, 0.3).await,
        }
    }

    /// Plays whatever a Faithful step produced. A score takes precedence over
    /// the paddle hit that may have set it up in the same step.
    pub fn play(&self, events: Events) {
        if events.scored.is_some() {
            play_sound_once(&self.score);
        } else if events.paddle_hit {
            play_sound_once(&self.paddle);
        }
        if events.wall_bounce {
            play_sound_once(&self.wall);
        }
    }

    /// Plays whatever a PULSE step produced. `speed_fraction` (0..1) is how fast
    /// the struck ball is going, so a paddle hit rises in pitch with the rally.
    pub fn play_pulse(&self, events: PulseEvents, speed_fraction: f32) {
        if events.scored.is_some() {
            play_sound_once(&self.pulse_score);
        } else if events.power_hit {
            play_sound_once(&self.pulse_power);
        } else if events.paddle_hit {
            let last = self.pulse_paddle.len() - 1;
            let index = (speed_fraction.clamp(0.0, 1.0) * last as f32).round() as usize;
            play_sound_once(&self.pulse_paddle[index.min(last)]);
        }
        if events.pickup {
            play_sound_once(&self.pulse_pickup);
        }
        if events.wall_bounce && !events.paddle_hit {
            play_sound_once(&self.pulse_wall);
        }
    }
}
