# Breakout

Atari's 1976 wall-breaker, the Collection's second Game. Its first take has
shipped:

- **Faithful** — Atari's *Breakout* recreated: one paddle, one ball, an
  eight-row wall to knock down twice, the original's difficulty and feel kept
  intact under a modern shell. Documented below.
- **Remix** — the reimagining that grows out of the Faithful. Still to come, so
  Breakout is not yet a **Done** Game.

**▶ Play: https://geox123.github.io/minigames/breakout/**

---

# Breakout — Faithful

A faithful recreation of Atari's 1976 *Breakout*. Bounce the ball off your
paddle to break through a wall of bricks — clear it, and a second wall takes its
place. You get three balls.

## Controls

| Key | Action |
| --- | --- |
| **← / →** or **A / D** | Move the paddle |
| **← → ↑ ↓** | Move the menu highlight |
| **Enter / Space** | Select |
| **P** | Pause / resume |
| **R** | Restart the game |
| **F** | Toggle fullscreen |
| **Esc** | Back out to mode-select |

The Game opens on a **mode-select** screen — Faithful (playable) or Remix
(coming soon) — then serves the first ball.

## What makes it faithful

The rules live entirely in the game's [pure core](core/src/lib.rs), which knows
nothing about rendering, audio or the clock. The details that make it play like
1976 rather than a generic ball-and-paddle game:

- **The eight-row wall,** fourteen bricks across, in four colour bands. From the
  bottom up they score **1, 3, 5 and 7** points — yellow, green, orange, red —
  so the higher, harder-to-reach rows are worth the most.
- **Paddle-angle deflection.** The paddle is read in eight segments; where the
  ball strikes it sets the angle it leaves at, so you aim by *where* on the
  paddle you make contact, not by holding a direction.
- **The speed-ups.** The ball steps up in speed after your **4th** and **12th**
  return, and again the first time it breaks into each of the two high bands
  (orange, then red). It only ever gets faster.
- **The shrinking paddle.** The first time the ball punches a column clear and
  reaches the top wall, the paddle **halves in width** for the rest of the game.
- **Two walls.** Clear the wall and a fresh one appears; clear both to win. Your
  score and remaining balls carry across.
- **Three balls,** each served from the paddle after a short pause.

## The modern shell

- Rendered at a fixed low **240×320 portrait logical resolution** — a coarse
  canvas in the spirit of the era's hardware — to an offscreen target, then
  scaled to the window by a whole number with the aspect ratio preserved: crisp
  on any display, no smearing.
- A **fixed 120 Hz simulation**: the shell accumulates real time into fixed
  steps, so behaviour is identical at any frame rate and the core stays
  deterministic and testable.
- A **HUD** with the score, the balls left and which of the two walls is up.
- **Synthesized sound** — square-wave blips for the paddle and walls, four brick
  voices rising in pitch with the higher bands, a flourish on clearing a wall
  and a low slide when a ball is lost, all generated at runtime so nothing is
  ripped.
- Pause, restart and fullscreen, natively and in the browser.

## Testing

The core is driven almost entirely through its public seam — construct a game,
feed input and fixed timesteps, assert on what a player could see. See
[`core/tests`](core/tests): the ball staying in the field and bouncing off the
walls, paddle movement and clamping, deflection by contact point, the serve, the
brick wall's layout and single-brick-per-step collision (so a fast ball never
tunnels through), scoring, the speed-ups and the paddle shrink, and restart. The
one exception is wall progression and the win: a full 112-brick wall can't be
emptied by honest play — a perfect paddle digs a channel and the ball then
bounces in it forever — so those are driven from inside the core in a unit test
that sets up the last standing brick and lets the real step path break it.

```sh
cargo test -p breakout-core     # the rules
cargo run  -p breakout          # play it natively
```
