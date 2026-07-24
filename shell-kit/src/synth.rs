//! Sound synthesized from scratch, so the Collection ships no ripped or
//! third-party audio (ADR 0002).
//!
//! Each voice is a short square wave — the tone the era's simple circuitry had —
//! built as an in-memory WAV and handed to macroquad. The same bytes drive the
//! native and browser builds. Games compose their own set of sounds from these
//! two generators.

use macroquad::audio::{Sound, load_sound_from_bytes};

const SAMPLE_RATE: u32 = 44_100;

/// A square-wave tone at `freq` Hz lasting `seconds`, with a linear decay so it
/// ends without a click, loaded as a macroquad sound.
pub async fn blip(freq: f32, seconds: f32) -> Sound {
    let count = (SAMPLE_RATE as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let square = if (freq * t).fract() < 0.5 { 1.0 } else { -1.0 };
        let decay = 1.0 - i as f32 / count as f32;
        samples.push(square * decay * 0.25);
    }
    load(&samples).await
}

/// A square-wave tone that sweeps from `from` to `to` Hz over `seconds` — a
/// rising chirp for pickups, a falling one for scores.
pub async fn chirp(from: f32, to: f32, seconds: f32) -> Sound {
    let count = (SAMPLE_RATE as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(count);
    let mut phase = 0.0f32;
    for i in 0..count {
        let progress = i as f32 / count as f32;
        let freq = from + (to - from) * progress;
        phase += freq / SAMPLE_RATE as f32;
        let square = if phase.fract() < 0.5 { 1.0 } else { -1.0 };
        let decay = 1.0 - progress;
        samples.push(square * decay * 0.25);
    }
    load(&samples).await
}

/// A tone that wavers between `low` and `high` Hz, `cycles` whole times across
/// `seconds` — a siren/warble. It holds a steady level (no decay) and starts and
/// ends at the same point, so it loops seamlessly for a continuous voice.
pub async fn warble(low: f32, high: f32, cycles: u32, seconds: f32) -> Sound {
    let count = (SAMPLE_RATE as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(count);
    let mut phase = 0.0f32;
    for i in 0..count {
        let progress = i as f32 / count as f32;
        // A low-frequency oscillator sweeps the pitch up and down.
        let lfo = 0.5 - 0.5 * (progress * cycles as f32 * std::f32::consts::TAU).cos();
        let freq = low + (high - low) * lfo;
        phase += freq / SAMPLE_RATE as f32;
        let square = if phase.fract() < 0.5 { 1.0 } else { -1.0 };
        samples.push(square * 0.16);
    }
    load(&samples).await
}

async fn load(samples: &[f32]) -> Sound {
    load_sound_from_bytes(&wav_mono_16(samples))
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
