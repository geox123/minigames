//! The game's sound, composed from the shared synthesizer so the repo ships no
//! ripped or sampled audio (ADR 0002 / 0003). Every voice is a square-wave blip, a
//! slide or a warble, built as an in-memory WAV; the same bytes drive the native
//! and browser builds.
//!
//! The signature is the **heartbeat**: two low thumps the shell alternates on a
//! timer whose tempo tracks how much of the field remains — slow when a field is
//! full, quickening as it thins, like the original's. The *idea* (tempo tracks the
//! count) is the original's; these oscillators are our own.

use macroquad::audio::{PlaySoundParams, Sound, play_sound, play_sound_once, stop_sound};
use shell_kit::synth::{blip, chirp, warble};

use asteroids_core::Events;

/// Every voice the Asteroids shell can play.
pub struct Audio {
    /// The two heartbeat thumps, alternated on the field's tempo.
    beat_low: Sound,
    beat_high: Sound,
    /// The ship's shot.
    fire: Sound,
    /// A rock breaking apart.
    rock_break: Sound,
    /// The ship blown apart.
    ship_die: Sound,
    /// The saucer felled.
    saucer_die: Sound,
    /// The whoosh of a hyperspace jump.
    hyperspace: Sound,
    /// The chime of an earned ship.
    extra_life: Sound,
    /// The ship's thrust rumble — looped while thrusting.
    thrust: Sound,
    /// The saucer's warble while it crosses — looped on and off.
    saucer: Sound,
}

impl Audio {
    /// Synthesizes and loads every voice. Awaited once, before play.
    pub async fn load() -> Self {
        Self {
            beat_low: blip(55.0, 0.14).await,
            beat_high: blip(73.0, 0.14).await,
            fire: chirp(900.0, 480.0, 0.08).await,
            rock_break: chirp(210.0, 55.0, 0.2).await,
            ship_die: chirp(300.0, 40.0, 0.5).await,
            saucer_die: chirp(520.0, 1150.0, 0.28).await,
            hyperspace: chirp(300.0, 1500.0, 0.22).await,
            extra_life: chirp(520.0, 1040.0, 0.3).await,
            thrust: warble(55.0, 90.0, 8, 0.4).await,
            saucer: warble(560.0, 860.0, 6, 0.5).await,
        }
    }

    /// Plays the one-shot voices for what a step produced — most urgent first, so a
    /// single step never stacks two of them.
    pub fn play(&self, events: &Events) {
        if events.ship_destroyed {
            play_sound_once(&self.ship_die);
        } else if events.saucer_destroyed {
            play_sound_once(&self.saucer_die);
        } else if events.rock_destroyed {
            play_sound_once(&self.rock_break);
        } else if events.hyperspaced {
            play_sound_once(&self.hyperspace);
        } else if events.fired {
            play_sound_once(&self.fire);
        }
        // The extra ship chimes over whatever else happened.
        if events.extra_life {
            play_sound_once(&self.extra_life);
        }
    }

    /// Plays one heartbeat thump — the low one or the high one.
    pub fn beat(&self, high: bool) {
        play_sound_once(if high {
            &self.beat_high
        } else {
            &self.beat_low
        });
    }

    /// Starts or stops the looping thrust rumble.
    pub fn set_thrust(&self, sounding: bool) {
        set_loop(&self.thrust, sounding);
    }

    /// Starts or stops the saucer's looping warble.
    pub fn set_saucer(&self, sounding: bool) {
        set_loop(&self.saucer, sounding);
    }
}

/// Loops `sound` while `sounding`, else stops it.
fn set_loop(sound: &Sound, sounding: bool) {
    if sounding {
        play_sound(
            sound,
            PlaySoundParams {
                looped: true,
                volume: 1.0,
            },
        );
    } else {
        stop_sound(sound);
    }
}
