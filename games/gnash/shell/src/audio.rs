//! GNASH's sound, composed from the shared synthesizer so the repo ships no ripped
//! or sampled audio (ADR 0003, ADR 0005). The voices are the original's *shapes*
//! from our own oscillators: the two-note chomp, a siren that climbs as the maze
//! empties, the frightened warble, the catch arpeggio, and the falling death tune.
//!
//! The juice budget is deliberately uneven (spec #158): the flip, the siren and
//! the death carry the weight. The siren is the centrepiece — a looping warble in
//! four tiers, swapped as the board empties, displaced entirely by the frightened
//! warble and by the thin high whine of eyes racing home.

use gnash_core::{Events, Game, HunterMode};
use macroquad::audio::{PlaySoundParams, Sound, play_sound, play_sound_once, stop_sound};
use shell_kit::synth::{blip, chirp, warble};

/// Which background loop is sounding, if any. At most one at a time: the hunt has
/// one voice, and what that voice is says what the hunt is doing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Backing {
    Silent,
    /// The siren, in a tier picked by how empty the maze is.
    Siren(usize),
    /// The frightened warble — the whole soundscape flips with the hunt.
    Fright,
    /// Eyes somewhere on the maze, racing home.
    Eyes,
}

/// Every voice the GNASH shell can play.
pub struct Audio {
    /// The two-note chomp, alternated as dots go down.
    chomp: [Sound; 2],
    /// A power pellet swallowed.
    pellet: Sound,
    /// A sweet taken.
    sweet: Sound,
    /// A frightened hunter caught — the rising arpeggio.
    caught: Sound,
    /// A pair of eyes regenerating back into the hunt.
    regen: Sound,
    /// The falling death tune.
    death: Sound,
    /// The extra-life chime.
    extra_life: Sound,
    /// The maze cleared — a short rising run.
    level_clear: Sound,
    /// The siren's four tiers, slow to frantic.
    sirens: [Sound; 4],
    /// The frightened warble.
    fright: Sound,
    /// The thin whine of eyes racing home.
    eyes: Sound,
    /// What is looping now.
    backing: Backing,
    /// Which chomp note is next.
    chomp_note: usize,
}

impl Audio {
    /// Synthesizes and loads every voice. Awaited once, before play.
    pub async fn load() -> Self {
        Self {
            chomp: [blip(392.0, 0.05).await, blip(494.0, 0.05).await],
            pellet: chirp(220.0, 440.0, 0.18).await,
            sweet: chirp(660.0, 990.0, 0.22).await,
            caught: chirp(360.0, 1440.0, 0.35).await,
            regen: chirp(880.0, 440.0, 0.15).await,
            death: chirp(520.0, 55.0, 0.9).await,
            extra_life: chirp(523.0, 1046.0, 0.4).await,
            level_clear: chirp(392.0, 1568.0, 0.5).await,
            sirens: [
                warble(300.0, 420.0, 2, 0.6).await,
                warble(360.0, 500.0, 3, 0.55).await,
                warble(430.0, 600.0, 4, 0.5).await,
                warble(520.0, 720.0, 5, 0.45).await,
            ],
            fright: warble(180.0, 320.0, 6, 0.5).await,
            eyes: warble(1200.0, 1500.0, 4, 0.4).await,
            backing: Backing::Silent,
            chomp_note: 0,
        }
    }

    /// Plays the one-shot voices for what a step produced — most urgent first, so a
    /// single step never stacks two. The extra-life chime rides over anything.
    pub fn play(&mut self, events: &Events) {
        if events.life_lost {
            play_sound_once(&self.death);
        } else if let Some((_, _)) = events.hunter_caught {
            play_sound_once(&self.caught);
        } else if events.maze_cleared {
            play_sound_once(&self.level_clear);
        } else if events.sweet_eaten.is_some() {
            play_sound_once(&self.sweet);
        } else if events.power_pellet_eaten {
            play_sound_once(&self.pellet);
        } else if events.hunter_regenerated.is_some() {
            play_sound_once(&self.regen);
        } else if events.dot_eaten {
            play_sound_once(&self.chomp[self.chomp_note]);
            self.chomp_note = 1 - self.chomp_note;
        }
        if events.extra_life {
            play_sound_once(&self.extra_life);
        }
    }

    /// Keeps the backing loop honest against the game's state: eyes outrank the
    /// warble, the warble outranks the siren, and the siren's tier tracks how empty
    /// the maze is. `hushed` (pause, death, ready, game over) silences everything.
    pub fn update_backing(&mut self, game: &Game, hushed: bool) {
        let wanted = if hushed {
            Backing::Silent
        } else if game.hunters().any(|h| h.mode == HunterMode::Eyes) {
            Backing::Eyes
        } else if game.frightened_frames_left() > 0 {
            Backing::Fright
        } else {
            let total = game.pickups_total().max(1) as f32;
            let eaten = 1.0 - game.pickups_remaining() as f32 / total;
            Backing::Siren((eaten * 4.0).min(3.0) as usize)
        };
        if wanted == self.backing {
            return;
        }
        match self.backing {
            Backing::Siren(tier) => stop_sound(&self.sirens[tier]),
            Backing::Fright => stop_sound(&self.fright),
            Backing::Eyes => stop_sound(&self.eyes),
            Backing::Silent => {}
        }
        let looped = PlaySoundParams {
            looped: true,
            volume: 0.45,
        };
        match wanted {
            Backing::Siren(tier) => play_sound(&self.sirens[tier], looped),
            Backing::Fright => play_sound(&self.fright, looped),
            Backing::Eyes => play_sound(&self.eyes, looped),
            Backing::Silent => {}
        }
        self.backing = wanted;
    }
}
