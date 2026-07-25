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
use asteroids_remix_core::Events as RemixEvents;

/// The gravity hum's depth bands — a lower, tenser drone each system.
const HUM_LEVELS: usize = 3;

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

    // ACCRETE's voices — the Remix's own synth palette.
    /// The ship's shot.
    remix_fire: Sound,
    /// A bright skim of a well's edge.
    remix_skim: Sound,
    /// The low swell of the well devouring a rock.
    remix_accrete: Sound,
    /// The collapse's shockwave boom.
    remix_collapse: Sound,
    /// An enemy downed.
    remix_enemy: Sound,
    /// A shot into a boss weak point.
    remix_boss_hit: Sound,
    /// A boss shifting phase.
    remix_boss_phase: Sound,
    /// A boss felled.
    remix_boss_clear: Sound,
    /// A power-up caught.
    remix_power: Sound,
    /// A shield soaking a hit.
    remix_shield: Sound,
    /// The ship's death.
    remix_death: Sound,
    /// The gravity hum, one drone per depth band — looped, deepening with the pull.
    gravity_hum: Vec<Sound>,
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

            remix_fire: chirp(880.0, 520.0, 0.06).await,
            remix_skim: chirp(1500.0, 2200.0, 0.05).await,
            remix_accrete: chirp(140.0, 300.0, 0.18).await,
            remix_collapse: chirp(1200.0, 60.0, 0.5).await,
            remix_enemy: chirp(520.0, 170.0, 0.10).await,
            remix_boss_hit: blip(240.0, 0.04).await,
            remix_boss_phase: chirp(200.0, 520.0, 0.28).await,
            remix_boss_clear: chirp(420.0, 60.0, 0.5).await,
            remix_power: chirp(500.0, 1200.0, 0.18).await,
            remix_shield: chirp(820.0, 320.0, 0.12).await,
            remix_death: chirp(300.0, 40.0, 0.5).await,
            // Three deepening drones — a lower, slower warble each band.
            gravity_hum: {
                let mut hums = Vec::with_capacity(HUM_LEVELS);
                for i in 0..HUM_LEVELS {
                    let base = 48.0 - i as f32 * 8.0;
                    hums.push(warble(base, base + 10.0, 4, 0.8).await);
                }
                hums
            },
        }
    }

    /// Plays ACCRETE's voice for what a step produced — one beat per step, most urgent
    /// first, so a single step never stacks two.
    pub fn play_remix(&self, events: &RemixEvents) {
        if events.ship_destroyed || events.game_over {
            play_sound_once(&self.remix_death);
        } else if events.boss_cleared {
            play_sound_once(&self.remix_boss_clear);
        } else if events.collapse_fired {
            play_sound_once(&self.remix_collapse);
        } else if events.boss_phase_changed {
            play_sound_once(&self.remix_boss_phase);
        } else if events.shield_broke {
            play_sound_once(&self.remix_shield);
        } else if events.power_up_taken {
            play_sound_once(&self.remix_power);
        } else if events.accreted {
            play_sound_once(&self.remix_accrete);
        } else if events.enemy_destroyed {
            play_sound_once(&self.remix_enemy);
        } else if events.boss_hit {
            play_sound_once(&self.remix_boss_hit);
        } else if events.skimmed {
            play_sound_once(&self.remix_skim);
        } else if events.fired {
            play_sound_once(&self.remix_fire);
        }
    }

    /// Loops the gravity hum at depth `level` (deeper with the pull), or stops it when
    /// `None`. Call only on a change, so the drone does not restart every frame.
    pub fn set_gravity_hum(&self, level: Option<usize>) {
        for hum in &self.gravity_hum {
            stop_sound(hum);
        }
        if let Some(level) = level {
            play_sound(
                &self.gravity_hum[level.min(HUM_LEVELS - 1)],
                PlaySoundParams {
                    looped: true,
                    volume: 0.5,
                },
            );
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
