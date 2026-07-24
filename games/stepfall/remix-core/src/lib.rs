//! The pure, deterministic core of **HAILFALL** — STEPFALL's Remix, a bullet-hell
//! reimagining of the 1978 invasion game.
//!
//! Where the Faithful is a rigid grid grinding down while you shuffle beneath it,
//! HAILFALL cuts the swarm loose: alien squadrons sweep in firing patterns that
//! fill the screen, and you fly a nimble ship through the storm. Like every core
//! in the Collection it owns the rules and knows nothing of rendering, audio,
//! windows or wall-clock time, and advances in fixed timesteps so a seed and a
//! sequence of inputs always replay the same run.
//!
//! It shares the Faithful's portrait field so one shell canvas serves both takes.
//!
//! This slice is the tracer bullet: the seam, and a ship you can fly within the
//! lower band. Fire, enemies, patterns, the tools and the modes arrive in later
//! tickets. The ship's **loadout** is handed *in* at construction, so the core
//! never knows the concept of "unlocks" — it only flies whatever it is given.

/// Width of the portrait play field, in logical units — shared with the Faithful.
pub const LOGICAL_WIDTH: f32 = 224.0;
/// Height of the portrait play field, in logical units — shared with the Faithful.
pub const LOGICAL_HEIGHT: f32 = 256.0;

/// Length of a single simulation step, in seconds — the Collection's 120 Hz.
pub const TIMESTEP: f32 = 1.0 / 120.0;

/// The ship's size, and how fast it flies.
pub const SHIP_WIDTH: f32 = 11.0;
pub const SHIP_HEIGHT: f32 = 8.0;
const SHIP_SPEED: f32 = 130.0;
/// How far from the field's side and foot the ship may travel.
const MARGIN: f32 = 8.0;
/// The ship flies within the lower band of the field: never above this line, so
/// it stays the defender at the bottom even with full freedom to weave.
const BAND_TOP: f32 = LOGICAL_HEIGHT * 0.5;

/// Which run a game is playing. The behaviours differ from a later ticket; here
/// the mode is only recorded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// A finite, winnable ladder of stages.
    #[default]
    Sortie,
    /// Endless, ever-denser waves, scored for survival.
    Onslaught,
    /// A date-seeded fixed run.
    Daily,
}

/// The ship's starting kit — the weapons and options a run flies with. Handed in
/// at construction so the core never knows "unlocks"; empty for now, filled as
/// the weapon and power-up tickets land.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loadout {}

/// What the player is doing this step. Movement is two-dimensional within the
/// band; the action buttons are wired up by later tickets but named now so the
/// input shape does not churn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// Hold to fire (from the firing ticket).
    pub fire: bool,
    /// Hold to move slow and precise (from the tools ticket).
    pub focus: bool,
    /// Tap to dash (from the tools ticket).
    pub dash: bool,
    /// Tap to spend a charged overdrive (from the tools ticket).
    pub bomb: bool,
}

/// The ship, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ship {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to. It
/// grows a field per ticket; empty for the tracer bullet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {}

/// Where a run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The run is being played.
    Playing,
    /// The run is over.
    Over,
}

/// A game of HAILFALL.
pub struct Game {
    /// Left edge of the ship.
    ship_x: f32,
    /// Top edge of the ship.
    ship_y: f32,
    mode: Mode,
    #[allow(dead_code)]
    loadout: Loadout,
    phase: Phase,
    /// Steps taken so far.
    steps: u64,
    /// The seed the run began on, so a restart replays it exactly.
    seed: u64,
}

impl Game {
    /// Starts a new run on `seed`, in `mode`, flying `loadout`. The same seed and
    /// inputs always produce the same run.
    pub fn new(seed: u64, mode: Mode, loadout: Loadout) -> Self {
        Self {
            ship_x: (LOGICAL_WIDTH - SHIP_WIDTH) / 2.0,
            ship_y: LOGICAL_HEIGHT - SHIP_HEIGHT - MARGIN * 3.0,
            mode,
            loadout,
            phase: Phase::Playing,
            steps: 0,
            seed,
        }
    }

    /// The ship, as the shell should draw it.
    pub fn ship(&self) -> Ship {
        Ship {
            x: self.ship_x,
            y: self.ship_y,
        }
    }

    /// Which run this is.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Where the run is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Starts the run over from the beginning; the same seed replays it.
    pub fn restart(&mut self) {
        *self = Self::new(self.seed, self.mode, self.loadout);
    }

    /// Advances the run by exactly one [`TIMESTEP`].
    pub fn step(&mut self, input: Input) -> Events {
        self.steps += 1;
        let events = Events::default();
        if self.phase == Phase::Over {
            return events;
        }
        self.fly(input);
        events
    }

    /// Flies the ship on the player's input, kept within the lower band.
    fn fly(&mut self, input: Input) {
        let travel = SHIP_SPEED * TIMESTEP;
        let dx = f32::from(input.right) - f32::from(input.left);
        let dy = f32::from(input.down) - f32::from(input.up);
        self.ship_x += dx * travel;
        self.ship_y += dy * travel;
        self.ship_x = self
            .ship_x
            .clamp(MARGIN, LOGICAL_WIDTH - MARGIN - SHIP_WIDTH);
        self.ship_y = self
            .ship_y
            .clamp(BAND_TOP, LOGICAL_HEIGHT - MARGIN - SHIP_HEIGHT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        Game::new(1, Mode::Sortie, Loadout::default())
    }

    fn press(input: Input, steps: usize) -> Game {
        let mut game = game();
        for _ in 0..steps {
            game.step(input);
        }
        game
    }

    #[test]
    fn the_ship_starts_low_and_centred() {
        let ship = game().ship();
        assert!((ship.x - (LOGICAL_WIDTH - SHIP_WIDTH) / 2.0).abs() < 0.01);
        assert!(ship.y > BAND_TOP, "the ship starts down in the band");
    }

    #[test]
    fn the_ship_flies_on_input() {
        let start = game().ship();

        let right = press(
            Input {
                right: true,
                ..Default::default()
            },
            30,
        )
        .ship();
        assert!(right.x > start.x, "holding right flies right");

        let up = press(
            Input {
                up: true,
                ..Default::default()
            },
            30,
        )
        .ship();
        assert!(up.y < start.y, "holding up flies up");
    }

    #[test]
    fn the_ship_is_held_within_the_lower_band() {
        // Push hard into every corner; the ship never leaves its bounds.
        let up_left = press(
            Input {
                up: true,
                left: true,
                ..Default::default()
            },
            10_000,
        )
        .ship();
        assert!(up_left.x >= MARGIN - 0.01, "held off the left wall");
        assert!(
            up_left.y >= BAND_TOP - 0.01,
            "held below the band's ceiling"
        );

        let down_right = press(
            Input {
                down: true,
                right: true,
                ..Default::default()
            },
            10_000,
        )
        .ship();
        assert!(
            down_right.x <= LOGICAL_WIDTH - MARGIN - SHIP_WIDTH + 0.01,
            "held off the right wall"
        );
        assert!(
            down_right.y <= LOGICAL_HEIGHT - MARGIN - SHIP_HEIGHT + 0.01,
            "held off the field's foot"
        );
    }

    #[test]
    fn a_restart_returns_the_ship_to_the_start() {
        let mut game = game();
        let start = game.ship();
        press_into(&mut game, 200);
        game.restart();
        assert_eq!(game.ship(), start, "restart replays from the opening");
    }

    fn press_into(game: &mut Game, steps: usize) {
        for _ in 0..steps {
            game.step(Input {
                right: true,
                down: true,
                ..Default::default()
            });
        }
    }
}
