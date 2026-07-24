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

/// The player's shot: its size, and how far it climbs per interrupt.
pub const SHOT_WIDTH: f32 = 1.0;
pub const SHOT_HEIGHT: f32 = 6.0;
const SHOT_SPEED: f32 = 4.0;

/// Invader bombs: their size, and how far they fall per interrupt — a touch
/// faster once the formation has thinned to a few.
pub const BOMB_WIDTH: f32 = 3.0;
pub const BOMB_HEIGHT: f32 = 7.0;
const BOMB_SPEED: f32 = 4.0 / 3.0;
const BOMB_SPEED_FAST: f32 = 5.0 / 3.0;
/// At or below this many invaders, the bombs speed up.
const FEW_INVADERS: u32 = 8;
/// The most bombs falling at once, and the interrupts between drops.
const MAX_BOMBS: usize = 3;
const BOMB_SPAWN_INTERVAL: u32 = 40;
/// The columns the two patterned bombs cycle through (0-based).
const SQUIGGLY_COLS: [usize; 4] = [10, 0, 5, 3];
const PLUNGER_COLS: [usize; 5] = [1, 7, 2, 8, 4];

/// The line along the bottom the cannon rides and bombs expire at.
const GROUND_Y: f32 = CANNON_TOP + CANNON_HEIGHT;

/// Lives a game starts with, the score that earns an extra, and how long the
/// game holds after the cannon is hit.
pub const LIVES_START: u32 = 3;
const EXTRA_LIFE_AT: u32 = 1500;
const DEATH_PAUSE: f32 = 1.0;

/// The four bunkers: how they are built, where they stand, and how a hit bites
/// into them. Each is a small grid of blocks that wears away a block at a time,
/// so it takes holes and degrades rather than vanishing whole.
pub const BUNKERS: usize = 4;
/// One bunker block, in logical units — a single "pixel" of the shield.
pub const BUNKER_CELL: f32 = 2.0;
/// A bunker's grid, in blocks.
pub const BUNKER_COLS: usize = 11;
pub const BUNKER_ROWS: usize = 8;
/// A bunker's size, in logical units.
pub const BUNKER_WIDTH: f32 = BUNKER_COLS as f32 * BUNKER_CELL;
pub const BUNKER_HEIGHT: f32 = BUNKER_ROWS as f32 * BUNKER_CELL;
/// The row the bunkers stand on: below the formation's descent start, above the
/// cannon, with room for the cannon to shelter under them.
const BUNKER_TOP: f32 = 176.0;
/// How far a hit's bite reaches from the block it strikes, in blocks — a radius
/// of one clears a small cluster, so cover wears away in chunks.
const BITE_RADIUS: i32 = 1;

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
    /// Whether the fire button is held. The cannon fires only when it has no
    /// shot in flight, so holding fire simply shoots again the moment the last
    /// shot clears — one shot on screen at a time, as the original allowed.
    pub fire: bool,
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

/// Which behaviour a bomb follows on the way down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BombKind {
    /// Dropped from the column above the cannon — it comes for you.
    Rolling,
    /// Dropped from a rotating pattern of columns.
    Squiggly,
    /// Dropped from another fixed pattern; held back once one invader remains.
    Plunger,
}

/// The player's shot in flight, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
}

/// A falling invader bomb, as the shell should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bomb {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Which behaviour it follows.
    pub kind: BombKind,
}

/// A single intact block of a bunker, as the shell should draw it — a
/// [`BUNKER_CELL`]-sized square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BunkerBlock {
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
    /// The cannon fired a shot this step.
    pub shot_fired: bool,
    /// A shot destroyed an invader this step, and which row it was in.
    pub invader_killed: Option<u8>,
    /// A bomb reached the cannon this step and cost a life.
    pub player_hit: bool,
    /// An extra life was earned this step.
    pub extra_life: bool,
    /// The last life was spent this step — the game is over.
    pub game_over: bool,
}

/// Where a game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The game is being played.
    Playing,
    /// Every life has been spent.
    GameOver,
}

/// One invader's position. Each keeps its own, because the march moves them one
/// at a time — which is what gives the formation its rippling shuffle.
#[derive(Clone, Copy)]
struct Pos {
    x: f32,
    y: f32,
}

/// A bomb's position and behaviour.
#[derive(Clone, Copy)]
struct BombState {
    x: f32,
    y: f32,
    kind: BombKind,
}

/// One bunker: a grid of blocks, each intact or eaten away. Indexed
/// `row * BUNKER_COLS + col`, row 0 the top.
#[derive(Clone)]
struct Bunker {
    /// Left edge.
    x: f32,
    /// Top edge.
    y: f32,
    cells: Vec<bool>,
}

impl Bunker {
    /// A fresh bunker at `x`: solid but for the shaved top corners and the arch
    /// cut up into the bottom middle — the silhouette the original's shields wore.
    fn fresh(x: f32) -> Self {
        let mut cells = vec![true; BUNKER_COLS * BUNKER_ROWS];
        let idx = |r: usize, c: usize| r * BUNKER_COLS + c;
        // The shaved top corners.
        cells[idx(0, 0)] = false;
        cells[idx(0, BUNKER_COLS - 1)] = false;
        // The arch cut up into the bottom middle.
        cells[idx(BUNKER_ROWS - 3, 5)] = false;
        for c in 4..=6 {
            cells[idx(BUNKER_ROWS - 2, c)] = false;
            cells[idx(BUNKER_ROWS - 1, c)] = false;
        }
        Self {
            x,
            y: BUNKER_TOP,
            cells,
        }
    }

    fn intact(&self, r: usize, c: usize) -> bool {
        self.cells[r * BUNKER_COLS + c]
    }

    /// The rectangle a block occupies, in logical units.
    fn block_rect(&self, r: usize, c: usize) -> (f32, f32, f32, f32) {
        (
            self.x + c as f32 * BUNKER_CELL,
            self.y + r as f32 * BUNKER_CELL,
            BUNKER_CELL,
            BUNKER_CELL,
        )
    }

    /// The block a projectile strikes, if it overlaps any intact one: the lowest
    /// when the fire comes from below (a shot), the highest when it comes from
    /// above (a bomb) — so cover erodes from the side it was fired at.
    fn impact(&self, rect: (f32, f32, f32, f32), from_below: bool) -> Option<(usize, usize)> {
        let mut hit: Option<(usize, usize)> = None;
        for r in 0..BUNKER_ROWS {
            for c in 0..BUNKER_COLS {
                if self.intact(r, c) && overlaps(rect, self.block_rect(r, c)) {
                    let better = match hit {
                        None => true,
                        Some((hr, _)) if from_below => r > hr,
                        Some((hr, _)) => r < hr,
                    };
                    if better {
                        hit = Some((r, c));
                    }
                }
            }
        }
        hit
    }

    /// Bites a cluster out of the shield, centred on `(r, c)`.
    fn bite(&mut self, r: usize, c: usize) {
        for dr in -BITE_RADIUS..=BITE_RADIUS {
            for dc in -BITE_RADIUS..=BITE_RADIUS {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if (0..BUNKER_ROWS as i32).contains(&nr) && (0..BUNKER_COLS as i32).contains(&nc) {
                    self.cells[nr as usize * BUNKER_COLS + nc as usize] = false;
                }
            }
        }
    }

    /// Scrapes away every block a descending invader has reached.
    fn scrape(&mut self, rect: (f32, f32, f32, f32)) {
        for r in 0..BUNKER_ROWS {
            for c in 0..BUNKER_COLS {
                if self.intact(r, c) && overlaps(rect, self.block_rect(r, c)) {
                    self.cells[r * BUNKER_COLS + c] = false;
                }
            }
        }
    }

    /// The intact blocks, for the shell to draw.
    fn blocks(&self) -> impl Iterator<Item = BunkerBlock> + '_ {
        let (x, y) = (self.x, self.y);
        self.cells
            .iter()
            .enumerate()
            .filter(|&(_, &intact)| intact)
            .map(move |(i, _)| BunkerBlock {
                x: x + (i % BUNKER_COLS) as f32 * BUNKER_CELL,
                y: y + (i / BUNKER_COLS) as f32 * BUNKER_CELL,
            })
    }
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
    /// The player's shot, if one is in flight.
    shot: Option<Pos>,
    /// The bombs the formation has in the air.
    bombs: Vec<BombState>,
    /// Interrupts since the last bomb was dropped.
    bomb_spawn_tick: u32,
    /// Bombs dropped so far, driving the kind and column rotation.
    spawns: u32,
    /// The four bunkers, eroding from both sides as the game is played.
    bunkers: Vec<Bunker>,
    score: u32,
    /// Lives left; the game ends when this reaches zero.
    lives: u32,
    /// Whether the extra life has been awarded yet.
    extra_awarded: bool,
    /// Seconds the game holds after a hit, before the cannon returns.
    dead: f32,
    /// Steps taken, to derive the machine interrupt from the timestep.
    steps: u64,
    phase: Phase,
    /// The seed the game began on, so a restart replays it exactly.
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
            shot: None,
            bombs: Vec::new(),
            bomb_spawn_tick: 0,
            spawns: 0,
            bunkers: (0..BUNKERS)
                .map(|i| {
                    let centre = (i as f32 + 0.5) / BUNKERS as f32 * LOGICAL_WIDTH;
                    Bunker::fresh(centre - BUNKER_WIDTH / 2.0)
                })
                .collect(),
            score: 0,
            lives: LIVES_START,
            extra_awarded: false,
            dead: 0.0,
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

    /// The player's shot in flight, if any.
    pub fn shot(&self) -> Option<Shot> {
        self.shot.map(|p| Shot { x: p.x, y: p.y })
    }

    /// The bombs the formation has in the air.
    pub fn bombs(&self) -> impl Iterator<Item = Bomb> + '_ {
        self.bombs.iter().map(|b| Bomb {
            x: b.x,
            y: b.y,
            kind: b.kind,
        })
    }

    /// The intact blocks of every bunker, for the shell to draw.
    pub fn bunker_blocks(&self) -> impl Iterator<Item = BunkerBlock> + '_ {
        self.bunkers.iter().flat_map(|b| b.blocks())
    }

    /// The score so far.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Lives left.
    pub fn lives(&self) -> u32 {
        self.lives
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
        let mut events = Events::default();

        if self.phase == Phase::GameOver {
            return events;
        }
        // After a hit the game holds for a beat before the cannon returns.
        if self.dead > 0.0 {
            self.dead -= TIMESTEP;
            if self.dead <= 0.0 {
                self.respawn();
            }
            return events;
        }

        self.move_cannon(input.cannon);
        if input.fire && self.shot.is_none() {
            self.shot = Some(Pos {
                x: self.cannon_x + CANNON_WIDTH / 2.0 - SHOT_WIDTH / 2.0,
                y: CANNON_TOP,
            });
            events.shot_fired = true;
        }

        // Everything but the cannon stirs only on a machine interrupt.
        if self.steps.is_multiple_of(STEPS_PER_INTERRUPT) {
            self.advance_march(&mut events);
            self.advance_shot(&mut events);
            self.advance_bombs(&mut events);
            self.spawn_bomb();
        }
        events
    }

    /// Returns the cannon to the middle and clears the sky after a hit.
    fn respawn(&mut self) {
        self.cannon_x = (LOGICAL_WIDTH - CANNON_WIDTH) / 2.0;
        self.bombs.clear();
        self.shot = None;
        self.dead = 0.0;
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
    fn advance_march(&mut self, events: &mut Events) {
        let Some(index) = self.next_standing() else {
            return;
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
        events.marched = true;

        // An invader that has descended into a bunker scrapes the cover away.
        let scraped = (moved.x, moved.y, INVADER_WIDTH, INVADER_HEIGHT);
        for bunker in &mut self.bunkers {
            bunker.scrape(scraped);
        }

        if moved.x < EDGE_MARGIN || moved.x + INVADER_WIDTH > LOGICAL_WIDTH - EDGE_MARGIN {
            self.edge_hit = true;
        }

        // Move the cursor on; running off the end completes a pass.
        self.cursor = index + 1;
        if self.cursor >= INVADERS {
            self.cursor = 0;
            events.turned = self.finish_pass();
        }
    }

    /// Climbs the player's shot, and destroys the first invader it reaches.
    fn advance_shot(&mut self, events: &mut Events) {
        let Some(mut shot) = self.shot else {
            return;
        };
        shot.y -= SHOT_SPEED;
        if shot.y + SHOT_HEIGHT < 0.0 {
            self.shot = None;
            return;
        }
        self.shot = Some(shot);

        let shot_rect = (shot.x, shot.y, SHOT_WIDTH, SHOT_HEIGHT);
        // Cover comes first: a shot is spent on the bunker it grazes, biting up
        // into it from below.
        if self.strike_bunkers(shot_rect, true) {
            self.shot = None;
            return;
        }
        for i in 0..INVADERS {
            if let Some(pos) = self.invaders[i]
                && overlaps(shot_rect, (pos.x, pos.y, INVADER_WIDTH, INVADER_HEIGHT))
            {
                self.destroy(i, events);
                self.shot = None;
                break;
            }
        }
    }

    /// Destroys the invader at `index`, scoring its row and granting the extra
    /// life if this is the score that earns it.
    fn destroy(&mut self, index: usize, events: &mut Events) {
        let row = index / COLS;
        self.invaders[index] = None;
        self.alive -= 1;
        self.score += row_score(row);
        events.invader_killed = Some(row as u8);
        if !self.extra_awarded && self.score >= EXTRA_LIFE_AT {
            self.extra_awarded = true;
            self.lives += 1;
            events.extra_life = true;
        }
    }

    /// Falls every bomb; a bomb that reaches the cannon costs a life, and one
    /// that reaches the ground simply expires.
    fn advance_bombs(&mut self, events: &mut Events) {
        let speed = if self.alive <= FEW_INVADERS {
            BOMB_SPEED_FAST
        } else {
            BOMB_SPEED
        };
        let cannon = self.cannon();
        let cannon_rect = (cannon.x, cannon.y, CANNON_WIDTH, CANNON_HEIGHT);

        let mut survivors = Vec::with_capacity(self.bombs.len());
        let mut hit = false;
        for mut bomb in std::mem::take(&mut self.bombs) {
            bomb.y += speed;
            let rect = (bomb.x, bomb.y, BOMB_WIDTH, BOMB_HEIGHT);
            // A bomb is spent on any bunker it reaches, biting down into it from
            // above — cover the cannon might have been sheltering under.
            if self.strike_bunkers(rect, false) {
                continue;
            }
            if overlaps(rect, cannon_rect) {
                hit = true;
                break;
            }
            if bomb.y <= GROUND_Y {
                survivors.push(bomb);
            }
        }
        if hit {
            self.lose_life(events);
        } else {
            self.bombs = survivors;
        }
    }

    /// Strikes the bunkers with a projectile, biting into the first one it
    /// grazes. Returns whether the projectile was spent on a bunker.
    fn strike_bunkers(&mut self, rect: (f32, f32, f32, f32), from_below: bool) -> bool {
        for bunker in &mut self.bunkers {
            if let Some((r, c)) = bunker.impact(rect, from_below) {
                bunker.bite(r, c);
                return true;
            }
        }
        false
    }

    /// Spends a life to a bomb: clears the sky, and either holds for the cannon
    /// to return or ends the game if that was the last life.
    fn lose_life(&mut self, events: &mut Events) {
        events.player_hit = true;
        self.bombs.clear();
        self.shot = None;
        self.lives -= 1;
        if self.lives == 0 {
            self.phase = Phase::GameOver;
            events.game_over = true;
        } else {
            self.dead = DEATH_PAUSE;
        }
    }

    /// Drops a new bomb on its cadence, from one of the three column rules — the
    /// rolling bomb from the column above the cannon, the other two from fixed
    /// patterns (the plunger held back once a single invader remains).
    fn spawn_bomb(&mut self) {
        if self.bombs.len() >= MAX_BOMBS {
            return;
        }
        self.bomb_spawn_tick += 1;
        if self.bomb_spawn_tick < BOMB_SPAWN_INTERVAL {
            return;
        }

        let n = self.spawns as usize;
        let mut kind = [BombKind::Rolling, BombKind::Squiggly, BombKind::Plunger][n % 3];
        if kind == BombKind::Plunger && self.alive <= 1 {
            kind = BombKind::Rolling;
        }
        let column = match kind {
            BombKind::Rolling => self.column_nearest_cannon(),
            BombKind::Squiggly => Some(SQUIGGLY_COLS[n % SQUIGGLY_COLS.len()]),
            BombKind::Plunger => Some(PLUNGER_COLS[n % PLUNGER_COLS.len()]),
        };
        let source = column
            .and_then(|c| self.bottom_of_column(c))
            .or_else(|| self.lowest_invader());
        if let Some(pos) = source {
            self.bombs.push(BombState {
                x: pos.x + INVADER_WIDTH / 2.0 - BOMB_WIDTH / 2.0,
                y: pos.y + INVADER_HEIGHT,
                kind,
            });
            self.spawns += 1;
            self.bomb_spawn_tick = 0;
        }
    }

    /// The column whose bottom invader sits nearest the cannon.
    fn column_nearest_cannon(&self) -> Option<usize> {
        let centre = self.cannon_x + CANNON_WIDTH / 2.0;
        (0..COLS)
            .filter_map(|c| {
                self.bottom_of_column(c)
                    .map(|p| (c, (p.x + INVADER_WIDTH / 2.0 - centre).abs()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(c, _)| c)
    }

    /// The lowest standing invader in column `col`, if any.
    fn bottom_of_column(&self, col: usize) -> Option<Pos> {
        (0..ROWS)
            .filter_map(|r| self.invaders[r * COLS + col])
            .max_by(|a, b| a.y.total_cmp(&b.y))
    }

    /// The lowest standing invader anywhere, if any.
    fn lowest_invader(&self) -> Option<Pos> {
        self.invaders
            .iter()
            .flatten()
            .copied()
            .max_by(|a, b| a.y.total_cmp(&b.y))
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

/// What an invader in `row` (0 the top) scores: the top row is worth 30, the
/// next two 20, the bottom two 10 — the original's values.
fn row_score(row: usize) -> u32 {
    match row {
        0 => 30,
        1 | 2 => 20,
        _ => 10,
    }
}

/// Whether two rectangles, each `(x, y, width, height)`, overlap.
fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

#[cfg(test)]
mod tests {
    //! White-box tests for the things honest play cannot cleanly stage: the
    //! march's acceleration (which needs invaders removed), and the lives and
    //! deaths (which need a bomb placed on the cannon). Each sets state up
    //! directly and then lets the real `step` path do the work, so what is
    //! measured is the genuine article. Firing, and everything reachable by
    //! playing, is driven through the seam in `tests/`.
    use super::*;

    /// A bomb about to land on the cannon's head this interrupt.
    fn bomb_on_cannon(game: &mut Game) {
        let cannon = game.cannon();
        game.bombs.push(BombState {
            x: cannon.x + CANNON_WIDTH / 2.0,
            y: cannon.y - 1.0,
            kind: BombKind::Rolling,
        });
    }

    /// Steps one whole machine interrupt (two simulation steps).
    fn interrupt(game: &mut Game) {
        game.step(Input::default());
        game.step(Input::default());
    }

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
    /// Bombs are swept each step so return fire can't kill the static cannon and
    /// stall the march we're measuring.
    fn interrupts_per_pass(game: &mut Game) -> u32 {
        let mut interrupts = 0;
        loop {
            let events = game.step(Input::default());
            game.bombs.clear();
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

    #[test]
    fn a_bomb_that_reaches_the_cannon_costs_a_life() {
        let mut game = Game::new(1);
        let lives = game.lives();
        bomb_on_cannon(&mut game);
        interrupt(&mut game);

        assert_eq!(game.lives(), lives - 1, "the bomb cost a life");
        assert_eq!(game.bombs().count(), 0, "the hit clears the sky");
    }

    #[test]
    fn spending_the_last_life_ends_the_game() {
        let mut game = Game::new(1);
        game.lives = 1;
        bomb_on_cannon(&mut game);
        interrupt(&mut game);

        assert_eq!(game.lives(), 0);
        assert_eq!(game.phase(), Phase::GameOver, "the last life ends the game");
    }

    #[test]
    fn crossing_the_threshold_grants_an_extra_life() {
        let mut game = Game::new(1);
        game.score = EXTRA_LIFE_AT - 5;
        let lives = game.lives();

        // A shot just under the top-left invader (worth 30) will cross 1500.
        let target = game.invaders[0].unwrap();
        game.shot = Some(Pos {
            x: target.x + INVADER_WIDTH / 2.0,
            y: target.y + INVADER_HEIGHT + 1.0,
        });
        interrupt(&mut game);

        assert!(
            game.score() >= EXTRA_LIFE_AT,
            "the kill crossed the threshold"
        );
        assert_eq!(game.lives(), lives + 1, "crossing it grants a life");
    }

    #[test]
    fn bombs_fall_faster_once_few_invaders_remain() {
        // The same bomb, dropped a step, with a full formation and with a thin
        // one — the thin one falls further.
        let drop = |alive: u32| -> f32 {
            let mut game = Game::new(1);
            game.alive = alive;
            game.bombs.push(BombState {
                x: 100.0,
                y: 100.0,
                kind: BombKind::Rolling,
            });
            let before = game.bombs().next().unwrap().y;
            interrupt(&mut game);
            game.bombs().next().unwrap().y - before
        };
        assert!(
            drop(FEW_INVADERS) > drop(INVADERS as u32),
            "a thinned formation drops faster bombs"
        );
    }

    #[test]
    fn a_descending_invader_scrapes_a_bunker_away() {
        let mut game = Game::new(1);
        let before = game.bunker_blocks().count();

        // Drop the marching invader straight onto the nearest cover; the march
        // then carries it through, and what it reaches is scraped away.
        let block = game.bunker_blocks().next().expect("a bunker stands");
        game.invaders[0] = Some(Pos {
            x: block.x,
            y: block.y,
        });
        game.cursor = 0;
        interrupt(&mut game);

        assert!(
            game.bunker_blocks().count() < before,
            "the invader scraped cover away"
        );
    }

    #[test]
    fn return_fire_uses_all_three_bomb_kinds() {
        use std::collections::HashSet;
        let mut game = Game::new(1);
        let mut seen = HashSet::new();
        for _ in 0..2_000 {
            interrupt(&mut game);
            seen.extend(game.bombs().map(|b| b.kind));
            // Sweep the sky each round so fresh bombs keep dropping and the
            // static cannon is never hit.
            game.bombs.clear();
            if seen.len() == 3 {
                break;
            }
        }
        assert_eq!(seen.len(), 3, "all three bomb kinds are dropped");
    }
}
