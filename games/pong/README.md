# Pong — Faithful

A faithful recreation of Atari's 1972 *Pong*, the game that started the
Collection. Two paddles, one ball, first to eleven — with the original's feel
kept intact and a modern shell around it.

**▶ Play: https://geox123.github.io/minigames/pong/**

## Controls

| Key | Action |
| --- | --- |
| **W / S** | Left paddle up / down |
| **↑ / ↓** | Right paddle up / down (two-player) |
| **← → ↑ ↓** | Move the menu highlight |
| **Enter / Space** | Select |
| **1 / 2** | Pick one or two players |
| **P** | Pause / resume |
| **R** | Restart the match |
| **F** | Toggle fullscreen |
| **Esc** | Back out to mode-select |

The Game opens on a **mode-select** screen — Faithful (playable) or Remix
(coming soon) — then asks for **one or two players**. One player puts you
against the computer on the right paddle.

## What makes it faithful

The rules live entirely in the game's [pure core](core/src/lib.rs), which knows
nothing about rendering, audio or the clock. The details that make it play like
1972 rather than a generic ball-and-paddle game:

- **Paddle-angle deflection.** The paddle is read in eight segments; where the
  ball strikes it sets the angle it leaves at, so you aim by *where* on the
  paddle you make contact, not by holding a direction.
- **Rally speed-up.** The ball steps up in speed twice during a rally — after 4
  returns, then after 12 — and drops back to the opening speed on every new
  point. Long exchanges get tense.
- **First to eleven,** with serves that pause in the middle of the field and
  alternate between the players.
- **The top-of-screen gap.** The original's paddles couldn't quite reach the top
  of the screen — a limit of the hardware Atari shipped. The Faithful keeps it.
- **A beatable computer.** The one-player opponent tracks the ball with a
  person's limits: it re-reads the ball only a few times a second, is a shade
  slower than a player, and plays the ball where it is rather than solving the
  bounce. A ball struck off the end of a paddle at full speed turns faster than
  it can react — aim for the corners it's furthest from and you'll score.

## The modern shell

- Rendered at Pong's original **320×240 logical resolution** to an offscreen
  canvas, then scaled to the window by a whole number with the aspect ratio
  preserved — crisp on any display, no smearing.
- A **fixed 120 Hz simulation**: the shell accumulates real time into fixed
  steps, so behaviour is identical at any frame rate and the core stays
  deterministic and testable.
- **Synthesized sound** — three square-wave blips at different pitches for
  paddle hit, wall bounce and score, generated at runtime so nothing is ripped.
- Pause, restart and fullscreen, natively and in the browser.

## Testing

The core is driven entirely through its public seam — construct a game, feed
input and fixed timesteps, assert on what a player could see. There are no
rendering, audio or window tests. See [`core/tests`](core/tests): paddle
movement and clamping, deflection by contact point, rally speed-up, scoring,
serve alternation, the win condition, restart, the computer opponent, and a
determinism check that replays a full match twice and compares.

```sh
cargo test -p pong-core     # the rules
cargo run  -p pong          # play it natively
```
