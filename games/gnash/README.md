# GNASH

After Namco's 1980 arcade maze-chase original — the Collection's fifth Game, its
first on a tile maze, and its first with pursuit AI. Namco is a flagship,
actively-enforced franchise, so the per-title re-check
([ADR 0005](../../docs/adr/0005-pac-man-ip-recheck.md)) lands against the real
name, the maze *and* the cast: GNASH ships under an **invented name**, threads an
**original maze**, and fields an **original cast**. Only the rules and feel are
faithful. It is drawn entirely in code
([ADR 0003](../../docs/adr/0003-code-drawn-visuals.md)) and its sound is
synthesized, nothing sampled or traced. No affiliation with, or endorsement by,
the rights holder is implied; this page names plainly what it recreates.

The identity sentence, from the spec: *the 1980 hunt, exact to the frame, inside a
mouth of our own — four teeth for hunters, a bare pair of jaws for an eater,
sweets for bait.*

**▶ Play: https://geox123.github.io/minigames/gnash/**

---

## The Faithful

You steer the **jaws** — a bare pair of them, all bite and nothing behind —
through a 28×31 corridor grid of 240 dots and four power pellets, always moving,
buffering your next turn, cutting corners for the sliver of ground that keeps you
ahead. Eating stalls you a beat; the hunt closes in while you feed.

The hunters are the mouth's four kinds of **tooth**, each steering at its own
target, and together they corner you:

- **The Canine** (red) — the fang: your own tile, relentlessly. As the maze
  empties it turns savage — faster, and deaf to scatter.
- **The Incisor** (pink) — the front tooth, first to cut: four tiles ahead of your
  facing, closing the door you were about to use.
- **The Wisdom** (cyan) — the crooked latecomer: a point doubled through the space
  ahead of you off the Canine's position, swinging wildly, pincering.
- **The Molar** (orange) — the back tooth: bold at a distance, breaking for its
  corner within eight tiles — it lopes in, loses nerve, and comes again.

The hunt breathes between **scatter** and **chase** on the original's per-level
schedule, every switch reversing the pack. A **power pellet** flips it: the teeth
turn midnight and flee, worth **200 → 400 → 800 → 1600** on one pellet, racing
home as bare eyes to regenerate — and the window shrinks level by level to
**nothing at all** in the deep game. **Sweets** appear below the pen at 70 and 170
dots, drawn grander at each rung of the original's ladder. Three lives, one more
at ten thousand, and a maze that refills meaner each time you clear it: faster,
tighter, crueller, with no end but the last life.

Controls: arrows / WASD to steer, **P** pause, **R** restart, **F** fullscreen.

## The shape of the code

The rules live in the pure, deterministic `gnash-core` crate — a 60 Hz
fixed-timestep `step(input) -> Events` seam, integer-pixel positions on the tile
grid, and every one of the original's tables (speeds, schedule tiers, fright
windows, savage thresholds, the sweet ladder) encoded as data. Same seed, same
inputs, same game, native or browser. The `gnash` shell owns the window, the
code-drawn look and the synthesized sound; it can hold the clock for its beats
(READY, the catch freeze, the death scatter) but moves no rule.

## The Remix

Designed only now that the Faithful has shipped, with a spec and an invented name
of its own — as every Remix before it.
