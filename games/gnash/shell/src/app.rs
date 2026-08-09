//! The GNASH shell's front-end: it boots straight into the Faithful and wraps the
//! pure core in the Collection's standard pause / restart / fullscreen chrome, plus
//! T9's beats — the READY hold, the catch freeze (the original's pause where the
//! score hangs in the air), and the death scatter played where the jaws fell. All
//! of these are shell-side holds on the clock; no core rule moves.

use gnash_core::{Game, Phase, TIMESTEP};
use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;

use crate::{audio::Audio, read_input, render};

/// How much real time a single frame may contribute to the simulation. Without this
/// cap, one long stall (a dragged window, a backgrounded tab) would make the game try
/// to catch up by simulating seconds at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The READY beat before play, in real frames.
const READY_FRAMES: u32 = 90;
/// The catch freeze — the beat the score hangs over a caught hunter, everything
/// held but the popup.
const CATCH_FRAMES: u32 = 36;
/// How long the death scatter plays, matching the core's own frozen beat.
const SCATTER_FRAMES: u32 = 60;

/// What the shell is holding over the game right now. At most one at a time; the
/// game clock stops for every variant but `None`.
pub enum Overlay {
    None,
    Paused,
    /// The pre-play hold, counting down.
    Ready(u32),
    /// A caught hunter's score, hanging where the catch happened.
    CatchPopup {
        x: f32,
        y: f32,
        value: u32,
        t: u32,
    },
    /// The jaws bursting apart where they fell.
    DeathScatter {
        x: f32,
        y: f32,
        t: u32,
    },
}

/// The whole shell: the game in play, the fixed-timestep accumulator banking real
/// time into 60 Hz steps, the current overlay beat, and the synth voices.
pub struct App {
    game: Game,
    accumulator: Accumulator,
    audio: Audio,
    overlay: Overlay,
    fullscreen: bool,
    /// Real frames since boot — the animation clock for blinks and flashes.
    tick: u32,
}

impl App {
    /// Opens the shell on a fresh game, seeded from the clock (the core's only
    /// nondeterminism), holding on READY.
    pub fn new(audio: Audio) -> Self {
        Self {
            game: Game::new(seed_from_clock()),
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            audio,
            overlay: Overlay::Ready(0),
            fullscreen: false,
            tick: 0,
        }
    }

    /// One real frame: honour the chrome keys, advance whichever clock is running —
    /// the overlay's or the game's — and draw.
    pub fn frame(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.chrome_keys();
        self.advance();
        let hushed = !matches!(self.overlay, Overlay::None)
            || self.game.dying()
            || self.game.phase() == Phase::GameOver;
        self.audio.update_backing(&self.game, hushed);
        render::draw(&self.game, self.tick, &self.overlay);
    }

    /// Fullscreen, restart, pause.
    fn chrome_keys(&mut self) {
        if is_key_pressed(KeyCode::F) {
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
        }
        if is_key_pressed(KeyCode::R) {
            self.game.restart();
            self.overlay = Overlay::Ready(0);
            self.accumulator.reset();
        }
        if is_key_pressed(KeyCode::P) {
            self.overlay = match self.overlay {
                Overlay::Paused => Overlay::None,
                // Pause trumps any beat; resuming returns to play directly — the
                // beats are flourishes, not owed time.
                _ => Overlay::Paused,
            };
        }
    }

    /// Advances the overlay if one is holding the clock, else the game itself.
    fn advance(&mut self) {
        match &mut self.overlay {
            Overlay::Paused => {
                self.accumulator.reset();
            }
            Overlay::Ready(t) | Overlay::CatchPopup { t, .. } | Overlay::DeathScatter { t, .. } => {
                *t += 1;
                let done = match self.overlay {
                    Overlay::Ready(t) => t >= READY_FRAMES,
                    Overlay::CatchPopup { t, .. } => t >= CATCH_FRAMES,
                    Overlay::DeathScatter { t, .. } => t >= SCATTER_FRAMES,
                    _ => unreachable!(),
                };
                if done {
                    self.overlay = Overlay::None;
                }
                self.accumulator.reset();
            }
            Overlay::None => self.step_game(),
        }
    }

    /// Runs the fixed steps now due, watching the events for the beats the shell
    /// plays over the top.
    fn step_game(&mut self) {
        let input = read_input();
        for _ in 0..self.accumulator.steps(get_frame_time()) {
            // The death scatter needs the spot the jaws fell, and the core walks
            // everyone home inside the fatal step — so remember where the eater
            // stood before each step.
            let eater = self.game.eater();
            let (before_x, before_y) = (eater.x as f32, eater.y as f32);
            let events = self.game.step(input);
            self.audio.play(&events);
            if events.life_lost {
                self.overlay = Overlay::DeathScatter {
                    x: before_x,
                    y: before_y,
                    t: 0,
                };
                break;
            }
            if let Some((_, value)) = events.hunter_caught {
                let eater = self.game.eater();
                self.overlay = Overlay::CatchPopup {
                    x: eater.x as f32,
                    y: eater.y as f32,
                    value,
                    t: 0,
                };
                break;
            }
            if events.maze_cleared {
                self.overlay = Overlay::Ready(0);
                break;
            }
        }
    }
}

/// A seed for a game, read from the clock — the core is deterministic, so this one
/// number is the only nondeterminism in a run.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
