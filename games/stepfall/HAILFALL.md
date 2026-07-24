# HAILFALL — STEPFALL's Remix

HAILFALL is the **Remix** half of STEPFALL: the Faithful's lock-step invasion
reimagined as a **bullet-hell** — the swarm cut loose into flying squadrons that
fill the screen with fire, and a nimble ship threading it — under its own name
(per [ADR 0002](../../docs/adr/0002-naming-and-ip-policy.md),
[ADR 0004](../../docs/adr/0004-space-invaders-ip-recheck.md)). It shares the
Faithful's *–FALL* DNA so the pair read as siblings. Choose it from STEPFALL's
mode-select.

**▶ Play: https://geox123.github.io/minigames/stepfall/** — pick HAILFALL on the
opening screen.

## Controls

| Key | Action |
| --- | --- |
| **Arrows** or **WASD** | Fly the ship within the lower band |
| **Space / Z** | Hold to fire |
| **Shift** | Focus — move slow and precise, hitbox revealed, fire concentrated |
| **X** | Dash — a quick dodge with a burst of invulnerability |
| **C** | Nova — spend a full overdrive to clear the screen |
| **P** | Pause / resume |
| **R** | Restart the run |
| **F** | Toggle fullscreen |
| **Esc** | Back out to HAILFALL's mode menu |

On the mode picker, **↑ / ↓** move the highlight and **Enter / Space** starts the
run.

## The ship

You fly freely within the lower band with three tools STEPFALL's cannon never
had, and a tiny true hitbox at your heart — only that pip can be hit, so a bullet
can pass through your hull and miss:

- **Dash** — a fast dodge on a cooldown, invulnerable for its burst; punch through
  a wall of fire when threading fails.
- **Focus** — move slow and precise, your true hitbox shown, your fire tightened
  into a concentrated twin.
- **Graze** — skimming enemy fire without being hit charges an **overdrive** meter;
  flirting with danger is rewarded. A full meter spends on a **nova** — a
  screen-clearing wipe that also damages everything on the field.

Downed enemies sometimes drop **power-ups**: a weapon that steps up a ladder
(spread → pierce → rapid → side drones), a **shield** that soaks one hit, and an
**overdrive** charge.

## The swarm

Alien **squadrons** sweep in on their own entry paths and open up with distinct
patterns — the **pattern zoo**:

- **Darts** aim single shots · **Weavers** bloom spreads
- **Turrets** fire full **rings** · **Spinners** sweep **spirals**
- **Wall gunners** drop a full-width sheet with one gap to thread
- **Bombers** drop STEPFALL's **rolling, squiggly and plunger** bombs — a callback
  to the Faithful's return fire

The pressure keeps rising like the Faithful's march: more enemies, denser
patterns, faster bullets the deeper a run goes.

## The mothership

Each stage builds to a **mothership** — a boss grown from the Faithful's saucer.
Its hull armours off your fire; only a shot that finds a **weak-point core** bites.
It runs **multiple phases** as it wears down — calm, then pressing, then enraged —
each with its own screen-filling pattern, and a nova takes a heavy bite. Fell it
to clear the stage.

## The modes

- **Sortie** — a finite, winnable ladder of stages, each a set of waves capped by
  a mothership, on a run-long pool of lives. Fell the final mothership and the run
  is **won**.
- **Onslaught** — endless, ever-deepening waves and motherships, scored for
  survival; your **best score** is kept.
- **Daily** — the calendar day's **shared seed**, so everyone plays the same run
  that day; the day's best is kept.

A run resolves on a **summary** card — won or lost, the score and stage reached,
and the mode's best.

## The feel

HAILFALL wears bright neon on a near-black field, deliberately apart from the
Faithful's stark banded mono so the two takes read apart at a glance: each
squadron kind in its own colour and silhouette, the bombs tinted by kind, the
mothership's phase-coloured hull and pulsing cores. On top: a ship trail,
particle bursts on grazes, power-ups, boss hits and the nova, screen shake and a
beat of hit-stop on the big impacts, and a synth voice for every event. All art
and audio are original and synthesized — nothing ripped
([ADR 0003](../../docs/adr/0003-code-drawn-visuals.md)).

## Under the hood

- The **core is pure and deterministic**, advancing in fixed 120 Hz steps: the
  same seed and inputs always replay the same run (a Daily depends on it). The
  ship's **loadout** is passed *in* at construction, so the core never knows the
  concept of "unlocks" — it only flies whatever it is handed, exactly as RIFT's
  core is handed a pool.
- Every rule is unit-tested through the core's single `step` seam — movement and
  its bounds, the dash and focus, enemy fire against the tiny hitbox, graze and
  the nova, the pattern zoo, power-ups, and mode flow. Reaching and felling a
  **mothership** and a full **Sortie** victory can't be cleanly staged by honest
  play, so those use white-box tests that run the real step path.
- The best Onslaught score and today's Daily persist via the tiny
  [`stepfall-storage`](storage/src/lib.rs) crate — a file natively, `localStorage`
  in the browser — the one place STEPFALL uses `unsafe`.

Cross-run **unlocks** (weapons and ship options earned by playing) and
**Ascension** modifier tiers are a planned Phase B, the same shape RIFT's meta
took; the core already takes its loadout in, so they land without touching a
Phase-A rule.

```sh
cargo run  -p stepfall                # play it natively (choose HAILFALL)
cargo test -p stepfall-remix-core     # HAILFALL's rules
```
