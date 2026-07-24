//! The game's sound, composed from the shared synthesizer so the repo ships no
//! ripped or sampled audio (ADR 0003, ADR 0004). Every voice is a square wave, a
//! slide or a warble, built as an in-memory WAV; the same bytes drive the native
//! and browser builds.
//!
//! Above all it owns the march: four descending notes the shell plays one per
//! formation step, so the tempo is the march's own — winding tighter as the
//! formation thins and frantic for the last invader. The march *idea* (tempo
//! tracks the count) is the original's; these oscillators are our own.

use macroquad::audio::{PlaySoundParams, Sound, play_sound, play_sound_once, stop_sound};
use stepfall_core::Events;
use stepfall_remix_core::Events as RemixEvents;

use shell_kit::synth::{blip, chirp, warble};

/// The four march notes, descending.
const MARCH_NOTES: usize = 4;

/// Every voice the STEPFALL shell can play.
pub struct Audio {
    /// The four descending march notes, played one per formation step.
    march: Vec<Sound>,
    /// The cannon's shot leaving the barrel.
    fire: Sound,
    /// An invader destroyed.
    invader_die: Sound,
    /// The saucer's warble while it crosses — looped on and off.
    saucer: Sound,
    /// The bonus when the saucer is shot.
    saucer_hit: Sound,
    /// The cannon blown apart.
    cannon_die: Sound,
    /// The tone for an earned extra life.
    extra_life: Sound,

    // HAILFALL's voices — the Remix's own synth palette.
    /// The ship's rapid pew.
    remix_fire: Sound,
    /// An enemy downed.
    remix_kill: Sound,
    /// A bullet grazed — a bright skim.
    remix_graze: Sound,
    /// A power-up caught.
    remix_power: Sound,
    /// The screen-clearing nova.
    remix_nova: Sound,
    /// A shot into the mothership's core.
    remix_boss_hit: Sound,
    /// The mothership shifting phase.
    remix_boss_phase: Sound,
    /// The mothership felled.
    remix_boss_clear: Sound,
    /// The ship taking a hit.
    remix_hit: Sound,
    /// A run won.
    remix_win: Sound,
    /// A shield soaking a hit.
    remix_shield: Sound,
}

impl Audio {
    /// Synthesizes and loads every voice. Awaited once, before play.
    pub async fn load() -> Self {
        // A low, ominous descending run — A2, G2, F2, E2.
        let notes = [110.0, 98.0, 87.0, 82.0];
        let mut march = Vec::with_capacity(MARCH_NOTES);
        for &freq in &notes {
            march.push(blip(freq, 0.11).await);
        }
        Self {
            march,
            fire: chirp(320.0, 900.0, 0.09).await,
            invader_die: chirp(420.0, 120.0, 0.14).await,
            saucer: warble(560.0, 860.0, 6, 0.5).await,
            saucer_hit: chirp(500.0, 1150.0, 0.28).await,
            cannon_die: chirp(300.0, 55.0, 0.45).await,
            extra_life: chirp(520.0, 1040.0, 0.3).await,

            remix_fire: chirp(620.0, 1180.0, 0.05).await,
            remix_kill: chirp(520.0, 180.0, 0.10).await,
            remix_graze: chirp(1500.0, 2100.0, 0.05).await,
            remix_power: chirp(500.0, 1200.0, 0.18).await,
            remix_nova: chirp(1300.0, 70.0, 0.5).await,
            remix_boss_hit: blip(240.0, 0.04).await,
            remix_boss_phase: chirp(200.0, 520.0, 0.28).await,
            remix_boss_clear: chirp(420.0, 60.0, 0.5).await,
            remix_hit: chirp(300.0, 50.0, 0.4).await,
            remix_win: chirp(400.0, 1200.0, 0.5).await,
            remix_shield: chirp(820.0, 320.0, 0.12).await,
        }
    }

    /// Plays HAILFALL's voice for what a step produced — one beat per step, most
    /// urgent first, so a single step never stacks two.
    pub fn play_remix(&self, events: &RemixEvents) {
        if events.run_won {
            play_sound_once(&self.remix_win);
        } else if events.player_hit || events.run_over {
            play_sound_once(&self.remix_hit);
        } else if events.boss_cleared {
            play_sound_once(&self.remix_boss_clear);
        } else if events.overdrive_fired {
            play_sound_once(&self.remix_nova);
        } else if events.boss_phase_changed {
            play_sound_once(&self.remix_boss_phase);
        } else if events.shield_broke {
            play_sound_once(&self.remix_shield);
        } else if events.power_up_taken {
            play_sound_once(&self.remix_power);
        } else if events.enemy_killed {
            play_sound_once(&self.remix_kill);
        } else if events.boss_hit {
            play_sound_once(&self.remix_boss_hit);
        } else if events.grazed {
            play_sound_once(&self.remix_graze);
        } else if events.shot_fired {
            play_sound_once(&self.remix_fire);
        }
    }

    /// Plays the one-shot voices for what a step produced — most urgent first, so
    /// a single step never stacks two of them.
    pub fn play(&self, events: &Events) {
        if events.player_hit {
            play_sound_once(&self.cannon_die);
        } else if events.saucer_hit.is_some() {
            play_sound_once(&self.saucer_hit);
        } else if events.invader_killed.is_some() {
            play_sound_once(&self.invader_die);
        } else if events.shot_fired {
            play_sound_once(&self.fire);
        }
        // The extra life chimes over whatever else happened.
        if events.extra_life {
            play_sound_once(&self.extra_life);
        }
    }

    /// Plays march note `note` (taken modulo the four).
    pub fn march_note(&self, note: usize) {
        play_sound_once(&self.march[note % MARCH_NOTES]);
    }

    /// Starts or stops the saucer's looping warble.
    pub fn set_saucer(&self, sounding: bool) {
        if sounding {
            play_sound(
                &self.saucer,
                PlaySoundParams {
                    looped: true,
                    volume: 1.0,
                },
            );
        } else {
            stop_sound(&self.saucer);
        }
    }
}
