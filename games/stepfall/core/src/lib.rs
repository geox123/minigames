//! The pure, deterministic core of **STEPFALL** — the Collection's faithful
//! recreation of the 1978 arcade alien-invasion original (Taito's *Space
//! Invaders*), shipped under a name of its own per
//! [ADR 0004](../../../docs/adr/0004-space-invaders-ip-recheck.md). The name is
//! the game's signature motion: the formation steps sideways, then falls a row.
//!
//! Like the Collection's other cores it owns every rule and knows nothing about
//! rendering, audio, windows or wall-clock time, and advances in fixed timesteps
//! so a seed and a sequence of inputs always replay the same game.
//!
//! It plays out on the original's 224×256 portrait field: a cannon the player
//! slides along the bottom, and a five-by-eleven formation of invaders that
//! marches sideways, reverses at the edges and grinds downward.
//!
//! # Why the march speeds up
//!
//! The original advanced **one invader per screen interrupt**, cycling through
//! the formation, so a step of the *whole* formation took as many interrupts as
//! there were invaders left — 55 at the start, one at the end. Its famous
//! acceleration was therefore not a difficulty curve anyone tuned; it fell out of
//! how the machine drew. This core is built the same way, so the same
//! acceleration falls out here — including the near-frantic last survivor.

/// Width of the portrait play field, in logical units — the original's.
pub const LOGICAL_WIDTH: f32 = 224.0;
/// Height of the portrait play field, in logical units — the original's.
pub const LOGICAL_HEIGHT: f32 = 256.0;

/// Length of a single simulation step, in seconds.
pub const TIMESTEP: f32 = 1.0 / 120.0;
/// Simulation steps per machine interrupt. The Collection's cores all run at
/// 120 Hz; the original acted once per 60 Hz interrupt, so it acts every second
/// step — which makes the one-invader-per-interrupt march exact, not an
/// approximation.
const STEPS_PER_INTERRUPT: u64 = 2;

/// The formation: five rows of eleven.
pub const ROWS: usize = 5;
pub const COLS: usize = 11;
/// Invaders in a full formation.
pub const INVADERS: usize = ROWS * COLS;

/// How big an invader is, and how far apart they sit.
pub const INVADER_WIDTH: f32 = 12.0;
pub const INVADER_HEIGHT: f32 = 8.0;
const CELL_WIDTH: f32 = 16.0;
const CELL_HEIGHT: f32 = 16.0;

/// Where a fresh formation stands: centred, with its top row here.
const FORMATION_TOP: f32 = 64.0;
/// How far the formation steps sideways per move, and down at an edge.
const MARCH_STEP: f32 = 2.0;
const DROP: f32 = 8.0;
/// The last invader alive presses to the right faster than to the left, exactly
/// as the original's did.
const LAST_INVADER_RIGHT_STEP: f32 = 3.0;
/// How close to the field edge the formation may come before turning.
const EDGE_MARGIN: f32 = 8.0;

/// The cannon: its size, where it sits, and how fast it slides.
pub const CANNON_WIDTH: f32 = 13.0;
pub const CANNON_HEIGHT: f32 = 8.0;
const CANNON_TOP: f32 = LOGICAL_HEIGHT - 32.0;
const CANNON_SPEED: f32 = 120.0;
/// How far from the side walls the cannon may travel.
const CANNON_MARGIN: f32 = 8.0;

/// Which way the player is pushing the cannon this step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Move {
    /// Towards the left wall.
    Left,
    /// Not moving.
    #[default]
    Hold,
    /// Towards the right wall.
    Right,
}

/// What the player is doing this step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// The cannon's direction.
    pub cannon: Move,
}

/// An invader still standing, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Invader {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Which row of the formation it belongs to, 0 (top) to [`ROWS`] − 1. The
    /// row decides its shape and, later, what it scores.
    pub row: usize,
}

/// The cannon, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cannon {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
}

/// What happened during a single [`Game::step`], for the shell to react to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// The march advanced an invader this step.
    pub marched: bool,
    /// The formation reversed and began stepping down.
    pub turned: bool,
}

/// Where a game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The game is being played.
    Playing,
}

/// One invader's position. Each keeps its own, because the march moves them one
/// at a time — which is what gives the formation its rippling shuffle.
#[derive(Clone, Copy)]
struct Pos {
    x: f32,
    y: f32,
}

/// A game of Space Invaders.
pub struct Game {
    /// Left edge of the cannon.
    cannon_x: f32,
    /// Each cell's invader, or `None` where one has been destroyed. Indexed
    /// `row * COLS + col`, with row 0 the top.
    invaders: Vec<Option<Pos>>,
    /// Invaders still standing.
    alive: u32,
    /// The cell the march advances next; a full pass over the array is one step
    /// of the whole formation.
    cursor: usize,
    /// Which way the formation marches: +1 right, −1 left.
    dir: f32,
    /// An invader touched a field edge during this pass.
    edge_hit: bool,
    /// This pass steps the formation down instead of sideways.
    dropping: bool,
    /// Steps taken, to derive the machine interrupt from the timestep.
    steps: u64,
    phase: Phase,
    /// The seed the game began on, so a restart replays it exactly. Nothing is
    /// random yet — return fire brings the first of it.
    seed: u64,
}

impl Game {
    /// Starts a new game. The same seed always produces the same game.
    pub fn new(seed: u64) -> Self {
        let mut invaders = Vec::with_capacity(INVADERS);
        let left = (LOGICAL_WIDTH - COLS as f32 * CELL_WIDTH) / 2.0;
        for row in 0..ROWS {
            for col in 0..COLS {
                invaders.push(Some(Pos {
                    x: left + col as f32 * CELL_WIDTH,
                    y: FORMATION_TOP + row as f32 * CELL_HEIGHT,
                }));
            }
        }
        Self {
            cannon_x: (LOGICAL_WIDTH - CANNON_WIDTH) / 2.0,
            invaders,
            alive: INVADERS as u32,
            cursor: 0,
            dir: 1.0,
            edge_hit: false,
            dropping: false,
            steps: 0,
            phase: Phase::Playing,
            seed,
        }
    }

    /// The invaders still standing, as the shell should draw them.
    pub fn invaders(&self) -> impl Iterator<Item = Invader> + '_ {
        self.invaders.iter().enumerate().filter_map(|(i, cell)| {
            let pos = (*cell)?;
            Some(Invader {
                x: pos.x,
                y: pos.y,
                row: i / COLS,
            })
        })
    }

    /// Invaders still standing.
    pub fn alive(&self) -> u32 {
        self.alive
    }

    /// The cannon, as the shell should draw it.
    pub fn cannon(&self) -> Cannon {
        Cannon {
            x: self.cannon_x,
            y: CANNON_TOP,
        }
    }

    /// Where the game is.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Starts the game over from the beginning; the same seed replays it.
    pub fn restart(&mut self) {
        *self = Self::new(self.seed);
    }

    /// Advances the game by exactly one [`TIMESTEP`].
    pub fn step(&mut self, input: Input) -> Events {
        self.steps += 1;
        self.move_cannon(input.cannon);

        // The formation only stirs on a machine interrupt.
        if self.steps.is_multiple_of(STEPS_PER_INTERRUPT) {
            self.advance_march()
        } else {
            Events::default()
        }
    }

    fn move_cannon(&mut self, mv: Move) {
        let travel = CANNON_SPEED * TIMESTEP;
        match mv {
            Move::Left => self.cannon_x -= travel,
            Move::Right => self.cannon_x += travel,
            Move::Hold => {}
        }
        self.cannon_x = self
            .cannon_x
            .clamp(CANNON_MARGIN, LOGICAL_WIDTH - CANNON_MARGIN - CANNON_WIDTH);
    }

    /// Advances exactly one invader — the whole trick. A pass over the formation
    /// therefore costs one interrupt per surviving invader, so the fewer are
    /// left, the sooner the formation steps again.
    fn advance_march(&mut self) -> Events {
        let Some(index) = self.next_standing() else {
            return Events::default();
        };

        let pos = self.invaders[index].expect("next_standing yields a standing invader");
        let moved = if self.dropping {
            Pos {
                x: pos.x,
                y: pos.y + DROP,
            }
        } else {
            Pos {
                x: pos.x + self.march_step(),
                y: pos.y,
            }
        };
        self.invaders[index] = Some(moved);

        if moved.x < EDGE_MARGIN || moved.x + INVADER_WIDTH > LOGICAL_WIDTH - EDGE_MARGIN {
            self.edge_hit = true;
        }

        // Move the cursor on; running off the end completes a pass.
        self.cursor = index + 1;
        let turned = if self.cursor >= INVADERS {
            self.cursor = 0;
            self.finish_pass()
        } else {
            false
        };

        Events {
            marched: true,
            turned,
        }
    }

    /// The next standing invader at or after the cursor, if any.
    fn next_standing(&self) -> Option<usize> {
        (self.cursor..INVADERS).find(|i| self.invaders[*i].is_some())
    }

    /// How far the next invader steps sideways. The last one alive presses right
    /// harder than it presses left, as the original's did.
    fn march_step(&self) -> f32 {
        let magnitude = if self.alive == 1 && self.dir > 0.0 {
            LAST_INVADER_RIGHT_STEP
        } else {
            MARCH_STEP
        };
        magnitude * self.dir
    }

    /// Ends a pass over the formation: a pass that touched an edge turns the
    /// formation round and sends the next one downward. Reports whether the
    /// formation just turned.
    fn finish_pass(&mut self) -> bool {
        let mut turned = false;
        if self.dropping {
            // The downward pass is done; carry on sideways the new way.
            self.dropping = false;
        } else if self.edge_hit {
            self.dir = -self.dir;
            self.dropping = true;
            turned = true;
        }
        self.edge_hit = false;
        turned
    }
}

#[cfg(test)]
mod tests {
    //! The march's acceleration is the one thing that cannot be driven honestly
    //! yet: it only shows once invaders are destroyed, and shooting arrives in
    //! the next ticket. These tests remove invaders directly and then let the
    //! real `step` path do the marching, so what is measured is the genuine
    //! article. Everything else is driven through the seam in `tests/`.
    use super::*;

    /// Leaves only `keep` invaders standing, taking them from the bottom row up.
    fn thin_to(game: &mut Game, keep: usize) {
        let mut left = keep;
        for i in (0..INVADERS).rev() {
            if left > 0 {
                left -= 1;
            } else {
                game.invaders[i] = None;
            }
        }
        game.alive = keep as u32;
        game.cursor = 0;
    }

    /// Interrupts taken for the formation to advance every standing invader once.
    fn interrupts_per_pass(game: &mut Game) -> u32 {
        let mut interrupts = 0;
        loop {
            let events = game.step(Input::default());
            if events.marched {
                interrupts += 1;
            }
            // A pass ends when the cursor wraps back to the start.
            if events.marched && game.cursor == 0 {
                return interrupts;
            }
            if interrupts > 10_000 {
                panic!("the formation never completed a pass");
            }
        }
    }

    #[test]
    fn a_thinner_formation_marches_sooner() {
        let mut full = Game::new(1);
        let full_pass = interrupts_per_pass(&mut full);
        assert_eq!(
            full_pass, INVADERS as u32,
            "a full formation costs one interrupt per invader"
        );

        let mut thinned = Game::new(1);
        thin_to(&mut thinned, 5);
        let thin_pass = interrupts_per_pass(&mut thinned);
        assert_eq!(thin_pass, 5, "five invaders cost five interrupts");

        assert!(
            thin_pass < full_pass,
            "the march accelerates as the formation thins"
        );
    }

    #[test]
    fn the_last_invader_presses_right_harder_than_left() {
        let mut game = Game::new(1);
        thin_to(&mut game, 1);

        // Marching right: the survivor takes the longer stride.
        game.dir = 1.0;
        let before = game.invaders().next().unwrap().x;
        interrupts_per_pass(&mut game);
        let right_stride = game.invaders().next().unwrap().x - before;

        // Marching left: the ordinary stride.
        game.dir = -1.0;
        let before = game.invaders().next().unwrap().x;
        interrupts_per_pass(&mut game);
        let left_stride = before - game.invaders().next().unwrap().x;

        assert_eq!(right_stride, LAST_INVADER_RIGHT_STEP);
        assert_eq!(left_stride, MARCH_STEP);
        assert!(right_stride > left_stride);
    }
}
