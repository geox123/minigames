# RIFT — Breakout's Remix

RIFT is the **Remix** half of Breakout: the Faithful's paddle-and-wall core
reimagined as a **roguelike descent** — a finite, winnable dive through
deepening walls, build-crafting boons between them, under its own name (per
[ADR 0002](../../docs/adr/0002-naming-and-ip-policy.md)). Choose it from
Breakout's mode-select.

**▶ Play: https://geox123.github.io/minigames/breakout/** — pick RIFT on the
opening screen.

## Controls

| Key | Action |
| --- | --- |
| **← / →** or **A / D** | Move the paddle (in a draft, move the highlight) |
| **Enter / Space** | Take the highlighted boon · select |
| **Tab** | Re-roll the boon draft |
| **P** | Pause / resume |
| **R** | Restart the run |
| **F** | Toggle fullscreen |
| **Esc** | Back out to RIFT's mode menu |

## The descent

A run dives through **three depths**. Each depth is a few walls capped by a
**guardian** — a designed set-piece (the Bastion, the Works, the Vault, each
tougher than the last) built from the brick zoo. Clear the final guardian and the
run is **won**. A run-long pool of **lives** replaces per-screen balls: drop the
ball and spend one; clear a guardian and bank one back; run out and the run ends.

## The brick zoo

Walls are mostly normal bricks, sprinkled with kinds that behave:

- **Armoured** — two hits; the first cracks it.
- **Mirror** — indestructible; sends the ball straight back. Doesn't count toward
  the clear — an obstacle to play around.
- **Explosive** — breaks and chains the blast to its neighbours.
- **Mover** — slides along its row while it stands.
- **Spawner** — refills an adjacent empty cell, regrowing the wall until broken.

## The boons

Between walls, **draft one of three boons** — with a re-roll — and they **stack**
across the run:

- **Pierce** (ball plows through breaks) · **Blast** (your breaks explode)
- **Widen** · **Swift** (a wider / faster paddle)
- **Extra Life** · **Fortune** (a life · bricks score more)

## The modes

- **Run** — a fresh, seeded descent; your best depth is kept.
- **Daily** — the calendar day's **shared seed**, so everyone plays the same run
  that day; the day's best is kept.
- **Ascension** — a mastery ladder: **win to unlock the next tier**, each layering
  difficulty modifiers (a faster ball, fewer lives, denser walls). Your highest
  tier is saved and resumes.

## The collection

RIFT opens up as it is played. A new player starts with a small kit — armoured
bricks, and the Widen, Extra Life and Fortune boons — and **earns the rest by
playing**: reaching depth 2 and 3, scoring 300 and 600 in a run, winning, and
winning at Ascension tiers 1 and 2. A run only ever draws on what has been
earned, so the walls and the drafts grow richer the further you get.

What a run unlocks is announced on its summary card, and the **collection**
screen — from RIFT's menu — lists everything: earned content in gold, and what is
still out there with the condition that unlocks it. One collection is shared
across all three modes.

## The feel

RIFT wears cool violets and cyans on a deep indigo field, deliberately apart from
the Faithful's stark banded wall so the two takes read apart at a glance. On top:
a speed-scaled ball trail, per-type particle bursts, screen shake and brief
hit-stop on explosive chains, guardian breaks and wins, and a rising chime when a
boon is drafted. All art and audio are original and synthesized — nothing ripped
([ADR 0002](../../docs/adr/0002-naming-and-ip-policy.md),
[ADR 0003](../../docs/adr/0003-code-drawn-visuals.md)).

## Under the hood

- The **core is pure and deterministic**, advancing in fixed 120 Hz steps: the
  same seed and inputs always replay the same run. The available **pool** of
  brick and boon types is passed *in* at construction, so the core never knows
  the concept of "unlocks" — it only ever draws on the pool it is handed. The
  cross-run meta (which content is earned, and what a run's outcome unlocks)
  lives in a pure `meta` module beside the rules and simply builds that pool, so
  the whole unlock layer was added without moving a single rule.
- Every rule is unit-tested through the core's single `step` seam. The
  wall/guardian/win transitions can't be reached by an auto-paddle (a perfect
  paddle digs a channel the ball bounces in forever), so those use white-box
  tests; everything else is driven through the seam.
- Best depth, the Daily result, the Ascension tier and the unlocked-content set
  persist via the tiny [`breakout-storage`](storage/src/lib.rs) crate — a file
  natively, `localStorage` in the browser — the one place the game uses `unsafe`.

```sh
cargo run  -p breakout               # play it natively (choose RIFT)
cargo test -p breakout-remix-core    # RIFT's rules
```
