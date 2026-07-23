# PULSE — Pong's Remix

PULSE is the **Remix** half of Pong: the Faithful's tight two-paddle core
reimagined with modern, skill-expressive mechanics, three modes and heavy juice,
under its own name (per [ADR 0002](../../docs/adr/0002-naming-and-ip-policy.md)).
Choose it from Pong's mode-select.

**▶ Play: https://geox123.github.io/minigames/pong/** — pick PULSE on the
opening screen.

## Controls

| Key | Action |
| --- | --- |
| **W / S** | Left paddle up / down |
| **Left Shift** | Left player charges a power shot |
| **↑ / ↓** | Right paddle up / down (two-player) |
| **Right Shift / Enter** | Right player charges a power shot |
| **← → ↑ ↓** | Move a menu highlight |
| **Enter / Space** | Select |
| **P** | Pause / resume |
| **R** | Restart the match or run |
| **F** | Toggle fullscreen |
| **Esc** | Back out to mode-select |

## The mechanics

PULSE keeps the Faithful's contact-point deflection and layers three skill
mechanics on top, all living in the pure [`remix-core`](remix-core/src/lib.rs):

- **Spin.** Move your paddle as you strike and the ball's flight bends in the
  direction of that motion, decaying over the shot. A sliding hit sends a banana
  shot around your opponent; meeting the ball flat keeps it true. It's a direct
  deepening of the Faithful's aiming — same instinct, higher ceiling.
- **Power shots.** Hold your charge key to build a power shot; charging slows
  your paddle (the risk). Return the ball fully charged and it flies out fast
  and bright, then your paddle is briefly sluggish (the cost). Let go early and
  the charge is spent for nothing.
- **Pickups.** Targets spawn on the net during a rally — steer a ball through
  one to collect it. **Multiball** splits an extra ball in; **Shield** saves your
  goal once; **Widen** enlarges your paddle for a while; **Slow-mo** briefly
  slows every ball so you can react. They go to whoever last hit the ball.

## The modes

- **Versus** — first to 7, one player against the computer or two at one
  keyboard. (PULSE runs first-to-7 rather than the Faithful's 11 — power shots
  and pickups make points come faster.)
- **Duel** — a best-of-five match: win three games to take it, with a short
  scorecard beat between games.
- **Gauntlet** — solo survival. You defend the left goal while the barrage
  speeds up and multiplies, until one gets past you. Score is ten a return plus
  a point a second survived; the run is seeded (repeatable and fair) and your
  best is saved locally.

## The feel

PULSE wears a neon palette — cyan and magenta paddles, a yellow ball on a dark
field — deliberately unlike the Faithful's stark white-on-black, so the two
takes read apart at a glance. On top of that: ball trails, particle bursts on
every contact, screen shake and a brief hit-stop on power shots and scores, and
a rally sound that rises in pitch as the ball speeds up. All art and audio are
original and synthesized — nothing ripped ([ADR 0002](../../docs/adr/0002-naming-and-ip-policy.md)).

## Under the hood

- The **core is pure and deterministic**, advancing in fixed 120 Hz steps: the
  same mode, seed and inputs always replay the same game. That's what makes the
  rules testable, and it's deliberate groundwork for a future online-ranked
  phase — rollback netcode wants exactly this shape.
- Every rule is unit-tested through the core's single seam — feed input and
  timesteps, assert on state. See [`remix-core/tests`](remix-core/tests): spin,
  power shots, multiball and the other pickups, the opponent, Duel match
  progression, Gauntlet escalation, and determinism.
- Best-score persistence lives in the tiny [`pong-storage`](storage/src/lib.rs)
  crate — a file natively, `localStorage` in the browser — the one place the
  project uses `unsafe`, kept out of the game and shell.

```sh
cargo run  -p pong             # play it natively (choose PULSE)
cargo test -p pong-remix-core  # PULSE's rules
```
