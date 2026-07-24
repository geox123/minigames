# STEPFALL

The 1978 arcade invasion game that defined a genre — the Collection's third
Game, shipped under **a name of its own**. Rather than the original's trademark,
STEPFALL takes an invented name (its signature motion: the formation *steps*
sideways, then *falls* a row), keeping only an honest description of what it
recreates. The reasoning is in [ADR 0004](../../docs/adr/0004-space-invaders-ip-recheck.md);
every sprite and sound is authored from scratch, nothing sampled or traced.

- **Faithful** — the arcade invasion game recreated: a lock-step formation that
  marches down at you, four bunkers to hide behind, a mystery saucer, and the
  march that winds tighter as you thin the formation. Documented below.
- **Remix** — still to come. It grows out of the Faithful, as every Remix does,
  and gets its own name and spec once it ships.

**▶ Play: https://geox123.github.io/minigames/stepfall/**

---

# STEPFALL — Faithful

A faithful recreation of the 1978 arcade original. Slide your cannon along the
bottom and shoot the descending formation before it reaches you — take cover
behind the bunkers, pick off the saucer for a bonus, and clear the screen to
bring the next, lower wave.

## Controls

| Key | Action |
| --- | --- |
| **← / →** or **A / D** | Move the cannon |
| **Space** or **↑** | Fire |
| **← → ↑ ↓** | Move the menu highlight |
| **Enter / Space** | Select |
| **P** | Pause / resume |
| **R** | Restart the game |
| **F** | Toggle fullscreen |
| **Esc** | Back out to mode-select |

The Game opens on a **mode-select** screen — Faithful (playable) or Remix
(coming soon) — then drops you into the first wave.

## What makes it faithful

The rules live entirely in the game's [pure core](core/src/lib.rs), which knows
nothing about rendering, audio or the clock. The details that make it play like
1978 rather than a generic shooter:

- **The march, and why it speeds up.** The original advanced **one invader per
  screen interrupt**, cycling through the formation, so a step of the *whole*
  formation took as many interrupts as there were invaders left — 55 at the
  start, one at the end. Its famous acceleration was never a tuned difficulty
  curve; it fell out of how the machine drew. The core is built the same way, so
  the same acceleration **emerges** — including the near-frantic last survivor,
  which also presses to the right harder than to the left, exactly as the
  original's did. The formation steps two pixels sideways, reverses and drops a
  row at the edges.
- **Scoring by row.** The top row scores **30**, the middle two **20**, the
  bottom two **10** — a cleared screen is worth **990**.
- **One shot at a time.** The cannon has a single shot on screen; hold fire and
  it shoots again the moment the last one clears.
- **Return fire.** Three kinds of bomb — rolling, squiggly and plunger — fall
  from the formation on a cadence, and speed up once only a few invaders remain.
- **The bunkers.** Four shields stand between you and the formation, and they
  erode **from both directions** — the invaders' bombs chew them from above,
  your own shots from below — so hiding costs the very cover you hide behind.
  Invaders that descend into a bunker scrape it away.
- **The mystery saucer.** It crosses the top at intervals while at least eight
  invaders remain; its heading follows the parity of your shot count, and
  shooting it scores from the original's table (50–300), including the quirk that
  makes the **23rd** shot — and every fifteenth after it — worth the full 300.
- **Lives and the descent.** Three lives, an extra at **1500**, a beat of pause
  on a death. But if the formation ever grinds down to the cannon's row, the game
  is over on the spot, **however many lives remain** — the march is a threat, not
  a timer.
- **Waves.** Clear the formation and a fresh one starts **lower**, on the
  original's ladder of starting heights (levelling off from the sixth), so the
  game escalates. The bunkers are rebuilt each wave; your score and lives carry
  across.

## The modern shell

- Rendered at a fixed **224×256 portrait logical resolution** — the original's —
  to an offscreen target, then scaled to the window by a whole number with the
  aspect ratio preserved: crisp on any display, no smearing.
- A **fixed 120 Hz simulation**: the shell accumulates real time into fixed
  steps, and the original's 60 Hz interrupt is exactly every second step, so the
  one-invader-per-interrupt march is precise and the core stays deterministic.
- **Hand-authored sprites** — STEPFALL's own three invader shapes (two frames
  each, alternating with the march), the cannon, the saucer, the bombs and the
  explosions — drawn as const bitmaps in code, under the cabinet's **colour
  bands** (red high, green low) over an otherwise monochrome field.
- A **HUD** with the score, the session's best, and the lives left as cannon
  icons.
- **Synthesized sound** — above all the four-note descending **march**, played
  one note per formation step so its tempo is the march's own, winding tighter as
  the formation thins; plus voices for firing, an invader dying, the saucer's
  warble and its bonus, the cannon's death and an extra life, all generated at
  runtime so nothing is ripped.
- Pause, restart and fullscreen, natively and in the browser.

## Testing

The core is driven through its public seam — construct a game, feed input and
fixed timesteps, assert on what a player could see. See [`core/tests`](core/tests):
the march and its acceleration, firing and scoring, the bunkers eroding from both
sides, the saucer's presence and its shot-count heading, and the descent grinding
down to the cannon's row to end the game. The transitions honest play can't
cleanly stage — the exact saucer prize table, the wave turn on a full clear, the
reach-the-line ending — are white-box unit tests inside the core that set the
state up and let the real step path run, the same approach Breakout's win uses.

```sh
cargo test -p stepfall-core     # the rules
cargo run  -p stepfall          # play it natively
```
