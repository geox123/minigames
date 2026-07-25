//! The shell's front-end: the mode-select in front of a game and the flow between
//! it and play. Everything here is window, input and rendering glue around the
//! pure core, which is why it lives in the shell, not `asteroids_core`.

use asteroids_core::{Game, Phase, TIMESTEP};
use asteroids_remix_core::{
    Game as RemixGame, Mode as RunMode, Outcome as RunOutcome, Phase as RemixPhase, meta,
};
use macroquad::prelude::*;
use shell_kit::timestep::Accumulator;

use crate::fx::Fx;
use crate::{Audio, read_input, read_remix_input, render};

/// ACCRETE's three modes, top to bottom on its picker.
pub const REMIX_MODES: [RunMode; 3] = [RunMode::Orbit, RunMode::Maelstrom, RunMode::Daily];

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make the
/// game try to catch up by simulating seconds at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The two takes every Game in the Collection ships — both playable: the Faithful,
/// and ACCRETE, its gravity Remix.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The faithful recreation.
    Faithful,
    /// The reimagined version — ACCRETE.
    Remix,
}

/// Which screen the player is looking at.
enum Screen {
    /// The Collection's two-takes screen: the Faithful or the locked Remix.
    ModeSelect { highlight: Mode },
    /// A Faithful game in progress. The game is boxed: it dwarfs the mode-select
    /// variant, so keeping it behind a pointer keeps `Screen` small.
    Match {
        game: Box<Game>,
        /// Left-over real time not yet folded into a fixed step.
        accumulator: Accumulator,
        /// Whether the game is paused.
        paused: bool,
        /// Whether the thrust rumble and saucer warble loops are sounding, so each
        /// toggles only on change.
        thrust_sounding: bool,
        saucer_sounding: bool,
        /// The heartbeat: seconds until the next thump, and which of the two it is.
        beat_timer: f32,
        beat_high: bool,
    },
    /// ACCRETE's mode picker — Orbit, Maelstrom or Daily.
    RemixSelect { highlight: RunMode },
    /// An ACCRETE run in progress — the gravity Remix. Boxed like the Faithful's game.
    RemixMatch {
        game: Box<RemixGame>,
        accumulator: Accumulator,
        paused: bool,
        /// Which mode this run is.
        mode: RunMode,
        /// The calendar day a Daily run belongs to (its saved key; 0 otherwise).
        day: u32,
        /// The best score to beat and show for this mode (0 for Orbit).
        best: u32,
        /// Whether this run's result has been saved yet (saved once, on run-over).
        saved: bool,
        /// The run's feel: trails, particles, collapse rings, shake and hit-stop.
        fx: Fx,
        /// The gravity-hum depth currently sounding, so it re-pitches only on a change.
        hum_level: Option<usize>,
        /// The options unlocked so far — this run's loadout was built from it, and its
        /// outcome is recorded back into it.
        unlocked: meta::Unlocked,
        /// Options newly unlocked by this run, for the summary to call out.
        earned: Vec<meta::Content>,
    },
}

/// The whole shell: the current screen, the seed source for new games, whether the
/// window is fullscreen, and the best score this session (not persisted — it resets
/// with the session, as the original's did with the cabinet).
pub struct App {
    screen: Screen,
    next_seed: u64,
    fullscreen: bool,
    /// The best score this session, carried across games and restarts; not saved.
    best: u32,
    /// How far to nudge the blit this frame — ACCRETE's screen shake, else zero.
    blit_shake: Vec2,
    audio: Audio,
}

impl App {
    /// Opens the shell on the mode-select screen.
    pub fn new(audio: Audio) -> Self {
        Self {
            screen: Screen::ModeSelect {
                highlight: Mode::Faithful,
            },
            next_seed: seed_from_clock(),
            fullscreen: false,
            best: 0,
            blit_shake: Vec2::ZERO,
            audio,
        }
    }

    /// How far to nudge the canvas when it is blitted this frame.
    pub fn blit_shake(&self) -> Vec2 {
        self.blit_shake
    }

    /// Advances the shell by one real frame: reads input, runs whatever the
    /// current screen does, and draws it to the logical canvas.
    pub fn frame(&mut self) {
        // Any shake is set anew each frame by the run that is playing.
        self.blit_shake = Vec2::ZERO;

        // Fullscreen can be toggled from anywhere in the shell.
        if is_key_pressed(KeyCode::F) {
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
        }

        match &mut self.screen {
            Screen::ModeSelect { highlight } => {
                if pressed_menu_move() {
                    *highlight = match *highlight {
                        Mode::Faithful => Mode::Remix,
                        Mode::Remix => Mode::Faithful,
                    };
                }
                if committed() {
                    match *highlight {
                        Mode::Faithful => self.start_match(),
                        // The Remix opens its own mode picker first.
                        Mode::Remix => self.open_remix_menu(),
                    }
                } else {
                    render::mode_select(*highlight);
                }
            }
            Screen::RemixSelect { highlight } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                if let Some(mode) = remix_menu_input(highlight) {
                    self.start_remix_match(mode);
                } else {
                    render::remix_select(*highlight);
                }
            }
            Screen::Match {
                game,
                accumulator,
                paused,
                thrust_sounding,
                saucer_sounding,
                beat_timer,
                beat_high,
            } => {
                // Backing out of a game returns to the Collection's mode-select.
                if is_key_pressed(KeyCode::Escape) {
                    self.audio.set_thrust(false);
                    self.audio.set_saucer(false);
                    self.return_to_mode_select();
                    return;
                }
                if is_key_pressed(KeyCode::P) {
                    *paused = !*paused;
                }
                if is_key_pressed(KeyCode::R) {
                    game.restart();
                    *paused = false;
                    *beat_timer = 0.0;
                    *beat_high = false;
                }

                let dt = get_frame_time();
                if !*paused {
                    let input = read_input();
                    for _ in 0..accumulator.steps(dt) {
                        self.audio.play(&game.step(input));
                    }
                } else {
                    // Don't let paused wall-time pile up and fast-forward on resume.
                    accumulator.reset();
                }

                let playing = !*paused && game.phase() == Phase::Playing;

                // The thrust rumble and the saucer warble loop while they apply.
                let want_thrust = playing && game.ship_alive() && game.ship().thrusting;
                if want_thrust != *thrust_sounding {
                    *thrust_sounding = want_thrust;
                    self.audio.set_thrust(want_thrust);
                }
                let want_saucer = playing && game.saucer().is_some();
                if want_saucer != *saucer_sounding {
                    *saucer_sounding = want_saucer;
                    self.audio.set_saucer(want_saucer);
                }

                // The heartbeat: two thumps alternating on a tempo that quickens as
                // the field thins; it rests between fields and when not playing.
                let rocks = game.asteroid_count();
                if playing && rocks > 0 {
                    *beat_timer -= dt;
                    if *beat_timer <= 0.0 {
                        self.audio.beat(*beat_high);
                        *beat_high = !*beat_high;
                        *beat_timer = heartbeat_interval(rocks);
                    }
                } else {
                    *beat_timer = 0.0;
                }

                self.best = self.best.max(game.score());
                render::draw(game, self.best);
                if game.phase() == Phase::GameOver {
                    render::game_over(game, self.best);
                } else if *paused {
                    render::paused_overlay();
                }
            }
            Screen::RemixMatch {
                game,
                accumulator,
                paused,
                mode,
                day,
                best,
                saved,
                fx,
                hum_level,
                unlocked,
                earned,
            } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.audio.set_gravity_hum(None);
                    self.return_to_mode_select();
                    return;
                }
                // A fresh run of the same mode, from the summary or mid-run.
                if is_key_pressed(KeyCode::R) {
                    let mode = *mode;
                    self.audio.set_gravity_hum(None);
                    self.start_remix_match(mode);
                    return;
                }

                let dt = get_frame_time();
                fx.update(dt);

                let over = game.phase() == RemixPhase::Over;
                if !over {
                    if is_key_pressed(KeyCode::P) {
                        *paused = !*paused;
                    }
                    // Hit-stop holds the sim still for a beat on the big impacts.
                    if !*paused && !fx.frozen() {
                        let input = read_remix_input();
                        for _ in 0..accumulator.steps(dt) {
                            let events = game.step(input);
                            self.audio.play_remix(&events);
                            let ship = game.ship();
                            let boss = game.boss().map(|b| (b.x, b.y));
                            fx.on_step(
                                events,
                                (ship.x, ship.y),
                                boss,
                                game.phase() == RemixPhase::Playing,
                            );
                        }
                    } else {
                        accumulator.reset();
                    }
                }

                // The gravity hum deepens a band each system; toggle only on a change so
                // the drone does not restart every frame, and rest it once the run ends.
                let want_hum = if over || *paused {
                    None
                } else {
                    Some((game.stage().saturating_sub(1) as usize).min(2))
                };
                if want_hum != *hum_level {
                    *hum_level = want_hum;
                    self.audio.set_gravity_hum(want_hum);
                }

                // On run-over, save the mode's best once, only when beaten. The guard
                // lives inside each arm so Orbit — which persists nothing — never even
                // touches `*best` (its best stays 0, so no BEST line shows for it).
                if over && !*saved {
                    *saved = true;
                    let score = game.score();
                    match mode {
                        RunMode::Maelstrom if score > *best => {
                            *best = score;
                            asteroids_storage::set_maelstrom_best(score);
                        }
                        RunMode::Daily if score > *best => {
                            *best = score;
                            asteroids_storage::set_daily_best(*day, score);
                        }
                        _ => {}
                    }

                    // Record the run into the unlock set; save and call out anything
                    // newly earned. The core never sees this — it only flew the loadout.
                    let outcome = meta::Outcome {
                        won: game.outcome() == Some(RunOutcome::Won),
                        systems_cleared: game.stage().saturating_sub(1),
                        score,
                        tier: 0,
                    };
                    let newly = unlocked.record(outcome);
                    if !newly.is_empty() {
                        asteroids_storage::set_unlocked_bits(unlocked.bits());
                        *earned = newly;
                    }
                }

                self.blit_shake = Vec2::from(fx.shake_offset());
                render::draw_remix(game, *best);
                fx.draw();
                if over {
                    render::remix_summary(game, *best, earned);
                } else if *paused {
                    render::paused_overlay();
                }
            }
        }
    }

    fn return_to_mode_select(&mut self) {
        self.screen = Screen::ModeSelect {
            highlight: Mode::Faithful,
        };
    }

    /// Opens ACCRETE's mode picker on its first mode.
    fn open_remix_menu(&mut self) {
        self.screen = Screen::RemixSelect {
            highlight: RunMode::Orbit,
        };
    }

    /// Consumes the next seed, advancing it so the next game differs.
    fn take_seed(&mut self) -> u64 {
        let seed = self.next_seed;
        self.next_seed = self.next_seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        seed
    }

    fn start_match(&mut self) {
        let game = Box::new(Game::new(self.take_seed()));
        self.screen = Screen::Match {
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            thrust_sounding: false,
            saucer_sounding: false,
            beat_timer: 0.0,
            beat_high: false,
        };
    }

    /// Starts an ACCRETE run in `mode`. Orbit and Maelstrom take a fresh seed from the
    /// clock; a Daily takes the day's shared seed so everyone plays the same run, and
    /// both endless modes draw the best to beat from the save.
    fn start_remix_match(&mut self, mode: RunMode) {
        let (seed, day, best) = match mode {
            RunMode::Orbit => (self.take_seed(), 0, 0),
            RunMode::Maelstrom => (self.take_seed(), 0, asteroids_storage::maelstrom_best()),
            RunMode::Daily => {
                let day = today();
                (u64::from(day), day, asteroids_storage::daily_best(day))
            }
        };
        // The run flies whatever the player has earned — the core only ever sees the
        // loadout the meta builds; it never knows the word "unlock".
        let unlocked = meta::Unlocked::from_bits(asteroids_storage::unlocked_bits());
        let game = Box::new(RemixGame::new(seed, mode, unlocked.loadout()));
        self.screen = Screen::RemixMatch {
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            mode,
            day,
            best,
            saved: false,
            fx: Fx::default(),
            hum_level: None,
            unlocked,
            earned: Vec::new(),
        };
    }
}

/// The seconds between heartbeat thumps for a field of `rocks` — a fuller field
/// beats slower, a thinning one quicker.
fn heartbeat_interval(rocks: usize) -> f32 {
    (0.22 + 0.045 * rocks as f32).min(0.7)
}

/// Whether the player committed to the highlighted option this frame.
fn committed() -> bool {
    is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)
}

/// Reads ACCRETE's mode picker, cycling the highlight through its modes. Returns the
/// chosen mode once the player commits to it.
fn remix_menu_input(highlight: &mut RunMode) -> Option<RunMode> {
    let i = REMIX_MODES.iter().position(|m| m == highlight).unwrap_or(0);
    if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
        *highlight = REMIX_MODES[(i + 1) % REMIX_MODES.len()];
    } else if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
        *highlight = REMIX_MODES[(i + REMIX_MODES.len() - 1) % REMIX_MODES.len()];
    }
    committed().then_some(*highlight)
}

/// Today's calendar day, as whole days since the Unix epoch. The core stays clock-free;
/// only the shell reads the clock, so a Daily's seed is shared by everyone playing on
/// the same day.
fn today() -> u32 {
    (miniquad::date::now() / 86_400.0) as u32
}

/// Whether the player nudged the menu highlight this frame.
fn pressed_menu_move() -> bool {
    is_key_pressed(KeyCode::Up)
        || is_key_pressed(KeyCode::Down)
        || is_key_pressed(KeyCode::Left)
        || is_key_pressed(KeyCode::Right)
        || is_key_pressed(KeyCode::W)
        || is_key_pressed(KeyCode::A)
        || is_key_pressed(KeyCode::S)
        || is_key_pressed(KeyCode::D)
}

/// A seed for a game. The core is deterministic by design, so the only
/// nondeterminism in the game is this one number, read from the clock.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
