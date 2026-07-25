# Asteroids

Atari's 1979 vector rock-shooter — the Collection's fourth Game. Atari, arcade-era
and low IP risk, so it ships under **its real name**, the same posture as Pong and
Breakout ([ADR 0002](../../docs/adr/0002-naming-and-ip-policy.md)). It is drawn
entirely in code as programmatic **vector polygons**
([ADR 0003](../../docs/adr/0003-code-drawn-visuals.md)) and its sound is
synthesized, nothing sampled or traced.

- **Faithful** — the vector original recreated: a ship that rotates, thrusts and
  drifts with real inertia on a screen-wrapped field, rocks that split as you break
  them, two mystery saucers, hyperspace, and escalating waves. Documented below.
- **Remix** — still to come. A Game is **Done** only once both takes ship; the
  Remix gets its own invented name and its own spec once the Faithful is out.

**▶ Play: https://geox123.github.io/minigames/asteroids/**

---

# Asteroids — Faithful

A faithful recreation of the 1979 arcade original. Fly a ship adrift in a field of
drifting rocks: rotate, thrust and coast on momentum, and shoot the rocks down —
each large one breaking into faster, smaller pieces — while a mystery saucer
crosses to hunt you. Clear the field to bring the next, bigger one.

## Controls

| Key | Action |
| --- | --- |
| **← / →** or **A / D** | Rotate the ship |
| **↑** or **W** | Thrust |
| **Space** | Fire |
| **↓** or **Shift** | Hyperspace |
| **Enter / Space** | Select on the mode-select |
| **P** | Pause / resume |
| **R** | Restart the game |
| **F** | Toggle fullscreen |
| **Esc** | Back out to mode-select |

The Game opens on a **mode-select** screen — the Faithful, or its Remix (locked
until it ships) — then drops you into the first field.

## What makes it faithful

The rules live entirely in the game's [pure core](core/src/lib.rs), which knows
nothing about rendering, audio or the clock. The details that make it play like
1979 rather than a generic shooter:

- **Newtonian flight.** The ship rotates in place, thrusts along its facing, and
  then *coasts* — momentum carries it, a gentle space-friction bleeds speed off,
  and a top speed caps it. Everything — ship, rocks, shots — **wraps** around every
  edge, and so does collision.
- **Fixed-speed fire.** The ship fires straight ahead, at most **four shots** on
  screen at once, each travelling a fixed distance before it expires. A shot flies
  at a fixed speed *through the world*, not relative to the ship — so a ship at full
  tilt can outrun and even fly into its own fire, exactly the original's quirk.
- **Splitting rocks.** A shot on a **large** rock yields two **mediums**, a medium
  two **smalls**, a small nothing — the fragments flying off faster and in a spread,
  so a cleared field grows more dangerous as it thins. Rocks score **20 / 50 / 100**
  by size.
- **The saucers.** A **large saucer** (200 points) that fires in random directions
  and a **small saucer** (1000 points) that fires *aimed* shots whose accuracy
  sharpens as your score climbs — with only the deadly small one appearing past a
  high score. A saucer crosses the field and leaves; its shots or its hull end a
  life.
- **Hyperspace.** A panic teleport: the ship blinks out and reappears somewhere
  random, arriving **unprotected** — it may materialise onto a rock and be
  destroyed, and on a small flat chance the jump malfunctions.
- **Lives, the bonus ship, and safe respawn.** Three ships, an extra every
  **10,000** points, and a ship lost to any rock, saucer or saucer shot — reappearing
  in the centre only once it is clear. The game ends when the last ship is gone.
- **Waves.** A field opens with **four** large rocks; each field cleared brings a
  fresh one with **two more**, up to a dozen, so the screen fills and the pressure
  rises.

## The modern shell

- Played in the original's **1024×768 vector coordinate space** — the field its math
  ran in — rendered to an offscreen target and scaled to the window by a whole number
  with the aspect ratio preserved: crisp on any display.
- A **fixed 120 Hz simulation**: the shell accumulates real time into fixed steps, so
  the core stays deterministic — the same seed and inputs always replay the same game.
- The era's **vector look**: bright thin outlines on black with a soft glow — the
  triangular ship with its thrust flame, an irregular silhouette per rock size, the
  saucer, blip shots, and line-shard explosions — all authored in code, none traced
  from the original.
- A **HUD** with the score, the session's best, the wave, and the ships left as
  little icons.
- **Synthesized sound**: the two-tone **heartbeat** whose tempo tracks how much of
  the field remains — quickening as it thins — plus the saucer's warble, the thrust
  rumble, and voices for firing, a rock breaking, a saucer felled, hyperspace, the
  ship's death and an earned ship, all generated at runtime so nothing is ripped.
- Pause, restart and fullscreen, natively and in the browser.

## Testing

The core is driven through its public seam — construct a game, feed input and fixed
timesteps, assert on what a player could see. See [`core/tests`](core/tests): the
ship's turning, thrust, friction, top speed and wrap; the field's layout and drift;
firing cadence and the shot's world-fixed speed; and determinism. The transitions
honest play can't cleanly stage — aiming a shot at a chosen rock to test splitting
and scoring, steering the ship onto one to test dying, summoning a saucer, landing a
hyperspace jump onto a rock, clearing a field to bring the next wave — are white-box
unit tests inside the core that set the state up and let the real step path run, the
same approach the Collection's other cores use.

```sh
cargo test -p asteroids-core     # the rules
cargo run  -p asteroids          # play it natively
```
