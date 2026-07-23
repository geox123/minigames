//! The match's sound, synthesized from scratch so the repo ships no ripped or
//! third-party audio (ADR 0002).
//!
//! Each sound is a short square-wave blip — the voice the original's simple
//! circuitry had — built as an in-memory WAV and handed to macroquad. The same
//! bytes drive the native and browser builds.

use macroquad::audio::{Sound, load_sound_from_bytes, play_sound_once};
use pong_core::Events;

const SAMPLE_RATE: u32 = 44_100;

/// The three voices of a match. A different pitch for each, so a player can tell
/// a wall bounce from a paddle hit without looking.
pub struct Audio {
    paddle: Sound,
    wall: Sound,
    score: Sound,
}

impl Audio {
    /// Synthesizes and loads the three sounds. Awaited once, before play.
    pub async fn load() -> Self {
        Self {
            paddle: blip(480.0, 0.05).await,
            wall: blip(240.0, 0.05).await,
            score: blip(120.0, 0.25).await,
        }
    }

    /// Plays whatever the given step produced. A score takes precedence over
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
}

/// A square-wave tone at `freq` Hz lasting `seconds`, with a linear decay so it
/// ends without a click, loaded as a macroquad sound.
async fn blip(freq: f32, seconds: f32) -> Sound {
    let count = (SAMPLE_RATE as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let square = if (freq * t).fract() < 0.5 { 1.0 } else { -1.0 };
        let decay = 1.0 - i as f32 / count as f32;
        samples.push(square * decay * 0.25);
    }

    load_sound_from_bytes(&wav_mono_16(&samples))
        .await
        .expect("synthesized WAV should always load")
}

/// Packs f32 samples in `-1.0..1.0` into a 16-bit PCM mono WAV file.
fn wav_mono_16(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // sub-chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for &sample in samples {
        let clamped = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        wav.extend_from_slice(&clamped.to_le_bytes());
    }

    wav
}
