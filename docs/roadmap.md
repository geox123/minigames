# Roadmap

How the Collection grows. This is a planning document — it changes as Games ship
and appetite shifts. Durable decisions live in [`docs/adr/`](adr); the
vocabulary (Game, Faithful, Remix, Done) lives in [`CONTEXT.md`](../CONTEXT.md).

## How Games are chosen

- **One Game at a time.** A Game must be **Done** — both its Faithful and its
  Remix shipped — before the next starts. Plans may look a batch ahead, but work
  never runs Games in parallel.
- **Iconic and capability-led.** Pick the genuinely distinct, iconic titles in
  rough chronological order, skipping minor clones, and let each one grow a new
  engine muscle. This honours the charter's "roughly chronological / by
  appetite" while keeping each Game a real step forward and the shared code
  growing sensibly.

## The next batch

Post-Pong, three Games form a deliberate capability ladder:

| # | Game | Year · IP | New engine muscle |
| - | ---- | --------- | ----------------- |
| 1 | **Breakout** | 1976 · Atari | Brick grid; single-player level structure; clear and scoring. The gentlest step from Pong (its direct descendant). |
| 2 | **Space Invaders** | 1978 · Taito | Hand-coded sprites; player shooting; a lock-step enemy formation that speeds up as its ranks thin; destructible bunkers; waves. |
| 3 | **Asteroids** | 1979 · Atari | Rotation with thrust and inertia; screen-wrap; vector polygons; asteroids that split as they're shot. |

**Pac-Man** (1980 · Namco — tile maze + ghost pathfinding AI) is the intended
opener of the *next* batch: a bigger capability leap, and Namco is a
more-defended franchise (see IP below), so it is deliberately held back.

Each Game ships its **Faithful** first, then its **Remix** (which is designed
only once the Faithful ships and gets its own spec), then it is **Done** and the
next Game unlocks.

## Policies for this batch

- **Remix ambition — matches PULSE.** Every Remix is a full statement:
  strong central mechanic(s), multiple modes, full juice — the kitchen-sink bar
  PULSE set. Each is designed post-Faithful, gets its own spec and tickets, and
  ships under an invented name ([ADR 0002](adr/0002-naming-and-ip-policy.md)).
  Honest cost: at roughly a 9-ticket Faithful plus a 10-ticket Remix per Game,
  the batch is ~55+ tickets — a large, sustained commitment, taken on
  deliberately.

- **Art — code-drawn, no asset pipeline** ([ADR 0003](adr/0003-code-drawn-visuals.md)).
  Everything is drawn in code: rectangles and the existing 5×7 pixel font and
  seven-segment digits, extended to hand-coded pixel bitmaps for Invaders'
  sprites and programmatic vector polygons for Asteroids. Sprite designs are
  original-but-evocative, never copied from the originals (ADR 0002). No
  AI-art or external-asset pipeline for this era.

- **Shared code — extract stable primitives as we go.** When Breakout first
  needs the pixel font, the logical-canvas + integer scaling, the WAV audio
  synth, or the fixed-timestep accumulator, lift those (genuinely identical,
  stable) into a shared crate instead of copying. Keep each Game's front-end /
  mode-select shell per-Game until the common shape settles across 2–3 Games —
  no framework designed off a sample size of one.

- **De-risking — build straight from spec.** TDD the pure deterministic core
  from the spec, as Pong and PULSE were built. Reach for a throwaway prototype
  only when a specific *feel* question blocks a design — most likely Asteroids'
  thrust-and-rotation handling — decided per-Game at design time.

## IP re-check points

Faithfuls reference the original name plainly; Remixes ship under invented names
(ADR 0002). Within this batch:

- **Breakout, Asteroids** — Atari, arcade-era, low risk. Real names, same
  posture as Pong.
- **Space Invaders** — Taito is more protective than Atari, though a rung below
  the Nintendo-tier ADR 0002 flags. The required per-title re-check is done and
  recorded in [ADR 0004](adr/0004-space-invaders-ip-recheck.md), and unlike
  Pong's and Breakout's it resolved *against* the real name: the Game ships as
  **STEPFALL**, an invented name, still faithful to the 1978 original's rules and
  still describing plainly what it recreates. Expect the same for Pac-Man and the
  Nintendo-era titles — budget an invented name from the start.
- **Pac-Man** (next batch) — Namco; do the per-title re-check when it comes up.

## Working notes

- Specs and tickets are written **just-in-time**, when each Game starts — a
  Faithful spec at kickoff, a Remix spec once the Faithful ships. Nothing is
  pre-published for the whole batch.
- The core of every Game stays pure, deterministic and fixed-timestep — the
  pattern that makes the rules testable through one seam, and (for Games that
  want it later) keeps the door open to rollback netcode.
