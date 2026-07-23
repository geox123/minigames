//! The shell's front-end: the screens in front of a match and the flow between
//! them. Everything here is window, input and rendering glue around the pure
//! core, which is why it lives in the shell rather than in `pong_core`.

use macroquad::prelude::*;
use pong_core::{Game, Players, TIMESTEP};
use pong_remix_core::{self as pulse, Ball as PulseBall, Game as PulseGame};

use shell_kit::timestep::Accumulator;

use crate::audio::Audio;
use crate::fx::Fx;
use crate::render;

/// How much real time a single frame may contribute to the simulation. Without
/// this cap, one long stall (a dragged window, a backgrounded tab) would make
/// the game try to catch up by simulating minutes at once.
const MAX_FRAME_TIME: f32 = 0.25;

/// The two takes every Game in the Collection ships. Both of Pong's are now
/// playable: the Faithful, and PULSE — its Remix.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The faithful recreation.
    Faithful,
    /// PULSE, the reimagined version.
    Remix,
}

/// The modes PULSE itself offers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PulseMode {
    /// Head to head, single game.
    Versus,
    /// Best-of-five match.
    Duel,
    /// Solo survival.
    Gauntlet,
}

impl PulseMode {
    fn next(self) -> Self {
        match self {
            PulseMode::Versus => PulseMode::Duel,
            PulseMode::Duel => PulseMode::Gauntlet,
            PulseMode::Gauntlet => PulseMode::Versus,
        }
    }

    fn prev(self) -> Self {
        self.next().next()
    }
}

/// Which screen the player is looking at.
enum Screen {
    /// The Collection's two-takes screen: Faithful or PULSE.
    ModeSelect { highlight: Mode },
    /// Choosing one or two players before a Faithful match.
    PlayerSelect { highlight: Players },
    /// Choosing which PULSE mode to play.
    PulseModeSelect { highlight: PulseMode },
    /// Choosing one or two players before a PULSE Versus or Duel match.
    PulsePlayerSelect { highlight: Players, duel: bool },
    /// A Faithful match in progress.
    FaithfulMatch {
        game: Game,
        /// Left-over real time not yet folded into a fixed step.
        accumulator: Accumulator,
        /// Whether the match is paused.
        paused: bool,
    },
    /// A PULSE match in progress (Versus or Gauntlet).
    PulseMatch {
        game: PulseGame,
        accumulator: Accumulator,
        paused: bool,
        /// Whether this is a Gauntlet run rather than a Versus match.
        gauntlet: bool,
        /// The neon feel — trails, particles, shake, hit-stop.
        fx: Fx,
    },
}

/// The whole shell: the current screen, the seed source for new matches, the
/// sounds, the best Gauntlet score, and whether the window is fullscreen.
pub struct App {
    screen: Screen,
    next_seed: u64,
    audio: Audio,
    best_gauntlet: u32,
    fullscreen: bool,
    /// The screen-shake offset to blit by this frame, in logical units.
    blit_shake: Vec2,
}

impl App {
    /// Opens the shell on the mode-select screen.
    pub fn new(audio: Audio) -> Self {
        Self {
            screen: Screen::ModeSelect {
                highlight: Mode::Faithful,
            },
            next_seed: seed_from_clock(),
            audio,
            best_gauntlet: pong_storage::best_gauntlet(),
            fullscreen: false,
            blit_shake: Vec2::ZERO,
        }
    }

    /// The screen-shake offset the window should blit the canvas by this frame.
    pub fn blit_shake(&self) -> Vec2 {
        self.blit_shake
    }

    /// Advances the shell by one real frame: reads input, runs whatever the
    /// current screen does, and draws it to the logical canvas.
    pub fn frame(&mut self) {
        // Fullscreen can be toggled from anywhere in the shell.
        if is_key_pressed(KeyCode::F) {
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
        }
        // Reset the shake each frame; a PULSE match sets it below.
        self.blit_shake = Vec2::ZERO;

        match &mut self.screen {
            Screen::ModeSelect { highlight } => {
                if mode_select_input(highlight) {
                    let next = match *highlight {
                        Mode::Faithful => Screen::PlayerSelect {
                            highlight: Players::Two,
                        },
                        Mode::Remix => Screen::PulseModeSelect {
                            highlight: PulseMode::Versus,
                        },
                    };
                    self.screen = next;
                } else {
                    render::mode_select(*highlight);
                }
            }
            Screen::PlayerSelect { highlight } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                } else if let Some(chosen) = player_select_input(highlight) {
                    self.start_match(chosen);
                } else {
                    render::player_select(*highlight);
                }
            }
            Screen::PulseModeSelect { highlight } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                } else if pulse_mode_input(highlight) {
                    match *highlight {
                        PulseMode::Versus => {
                            self.screen = Screen::PulsePlayerSelect {
                                highlight: Players::One,
                                duel: false,
                            };
                        }
                        PulseMode::Duel => {
                            self.screen = Screen::PulsePlayerSelect {
                                highlight: Players::One,
                                duel: true,
                            };
                        }
                        PulseMode::Gauntlet => self.start_gauntlet(),
                    }
                } else {
                    render::pulse_mode_select(*highlight);
                }
            }
            Screen::PulsePlayerSelect { highlight, duel } => {
                let duel = *duel;
                if is_key_pressed(KeyCode::Escape) {
                    self.screen = Screen::PulseModeSelect {
                        highlight: PulseMode::Versus,
                    };
                } else if let Some(chosen) = player_select_input(highlight) {
                    self.start_pulse(chosen, duel);
                } else {
                    render::pulse_player_select(*highlight, duel);
                }
            }
            Screen::FaithfulMatch {
                game,
                accumulator,
                paused,
            } => {
                // Backing out of a match returns to the Collection's mode-select.
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                if is_key_pressed(KeyCode::P) {
                    *paused = !*paused;
                }
                if is_key_pressed(KeyCode::R) {
                    game.restart();
                    *paused = false;
                }

                if !*paused {
                    let input = crate::read_input();
                    for _ in 0..accumulator.steps(get_frame_time()) {
                        self.audio.play(game.step(input));
                    }
                } else {
                    // Don't let paused wall-time pile up and fast-forward on resume.
                    accumulator.reset();
                }

                render::draw(game);
                if *paused {
                    render::paused_overlay();
                }
            }
            Screen::PulseMatch {
                game,
                accumulator,
                paused,
                gauntlet,
                fx,
            } => {
                if is_key_pressed(KeyCode::Escape) {
                    self.return_to_mode_select();
                    return;
                }
                if is_key_pressed(KeyCode::P) {
                    *paused = !*paused;
                }
                if is_key_pressed(KeyCode::R) {
                    game.restart();
                    *paused = false;
                }

                if !*paused {
                    let dt = get_frame_time();
                    fx.update(dt);
                    // Hit-stop freezes the simulation, not the effects.
                    if fx.frozen() {
                        accumulator.reset();
                    } else {
                        let input = read_pulse_input();
                        for _ in 0..accumulator.steps(dt) {
                            let events = game.step(input);
                            let balls: Vec<PulseBall> = game.balls().collect();
                            fx.on_step(events, &balls);
                            let fastest = balls
                                .iter()
                                .map(|b| b.vx.hypot(b.vy))
                                .fold(0.0_f32, f32::max);
                            self.audio.play_pulse(events, fastest / pulse::POWER_SPEED);
                        }
                    }
                } else {
                    accumulator.reset();
                }
                self.blit_shake = Vec2::from(fx.shake_offset());

                if *gauntlet {
                    // Bank a new best score the moment the run ends.
                    if game.phase() == pulse::Phase::RunOver
                        && game.gauntlet_score() > self.best_gauntlet
                    {
                        self.best_gauntlet = game.gauntlet_score();
                        pong_storage::set_best_gauntlet(self.best_gauntlet);
                    }
                    render::draw_gauntlet(game, self.best_gauntlet);
                } else {
                    render::draw_pulse(game);
                }
                fx.draw(pulse::BALL_SIZE);
                if *paused {
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

    /// Consumes the next seed, advancing it so the next match differs.
    fn take_seed(&mut self) -> u64 {
        let seed = self.next_seed;
        self.next_seed = self.next_seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        seed
    }

    fn start_match(&mut self, players: Players) {
        let game = Game::new(players, self.take_seed());
        self.screen = Screen::FaithfulMatch {
            game,
            accumulator: Accumulator::new(TIMESTEP, MAX_FRAME_TIME),
            paused: false,
        };
    }

    fn start_pulse(&mut self, players: Players, duel: bool) {
        let seed = self.take_seed();
        let game = match (duel, players) {
            (false, Players::One) => PulseGame::new_versus_cpu(seed),
            (false, Players::Two) => PulseGame::new(seed),
            (true, Players::One) => PulseGame::new_duel_cpu(seed),
            (true, Players::Two) => PulseGame::new_duel(seed),
        };
        self.screen = Screen::PulseMatch {
            game,
            accumulator: Accumulator::new(pulse::TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            gauntlet: false,
            fx: Fx::default(),
        };
    }

    fn start_gauntlet(&mut self) {
        let game = PulseGame::new_gauntlet(self.take_seed());
        self.screen = Screen::PulseMatch {
            game,
            accumulator: Accumulator::new(pulse::TIMESTEP, MAX_FRAME_TIME),
            paused: false,
            gauntlet: true,
            fx: Fx::default(),
        };
    }
}

/// Reads the PULSE mode-select, moving the highlight through the three modes
/// and reporting a commit.
fn pulse_mode_input(highlight: &mut PulseMode) -> bool {
    if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
        *highlight = highlight.prev();
    }
    if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
        *highlight = highlight.next();
    }
    is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)
}

/// Reads both players off one keyboard for a PULSE match: W/S and Left-Shift to
/// charge on the left, the arrows and Right-Shift (or Enter) on the right.
fn read_pulse_input() -> pulse::Input {
    pulse::Input {
        left: pulse_axis(KeyCode::W, KeyCode::S),
        right: pulse_axis(KeyCode::Up, KeyCode::Down),
        charge_left: is_key_down(KeyCode::LeftShift),
        charge_right: is_key_down(KeyCode::RightShift) || is_key_down(KeyCode::Enter),
    }
}

fn pulse_axis(up: KeyCode, down: KeyCode) -> pulse::Axis {
    match (is_key_down(up), is_key_down(down)) {
        (true, false) => pulse::Axis::Up,
        (false, true) => pulse::Axis::Down,
        _ => pulse::Axis::Hold,
    }
}

/// Reads the mode-select screen, moving the highlight between the two takes.
/// Returns whether the player committed to the highlighted one.
fn mode_select_input(highlight: &mut Mode) -> bool {
    if pressed_menu_move() {
        *highlight = match *highlight {
            Mode::Faithful => Mode::Remix,
            Mode::Remix => Mode::Faithful,
        };
    }
    is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)
}

/// Reads the player-select screen. Returns the chosen player count once the
/// player commits, or `None` while they are still choosing.
fn player_select_input(highlight: &mut Players) -> Option<Players> {
    if pressed_menu_move() {
        *highlight = match *highlight {
            Players::One => Players::Two,
            Players::Two => Players::One,
        };
    }
    if is_key_pressed(KeyCode::Key1) {
        *highlight = Players::One;
    }
    if is_key_pressed(KeyCode::Key2) {
        *highlight = Players::Two;
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
        return Some(*highlight);
    }
    None
}

/// Whether the player nudged a menu highlight this frame.
fn pressed_menu_move() -> bool {
    is_key_pressed(KeyCode::Up)
        || is_key_pressed(KeyCode::Down)
        || is_key_pressed(KeyCode::Left)
        || is_key_pressed(KeyCode::Right)
        || is_key_pressed(KeyCode::W)
        || is_key_pressed(KeyCode::S)
}

/// A seed for a match. The core is deterministic by design, so the only
/// nondeterminism in the game is this one number, read from the clock.
fn seed_from_clock() -> u64 {
    (miniquad::date::now() * 1_000.0) as u64
}
