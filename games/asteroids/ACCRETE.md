# ACCRETE — Asteroids' Remix

ACCRETE is the **Remix** half of Asteroids: the Faithful's Newtonian drift reimagined
around the one force the original left out — **gravity**. A **well** (a star) sits at
the heart of the field and pulls on *everything* — your ship, the rocks, the enemies,
even the shots — so flight becomes the management of an orbit, rocks become matter to
feed a hungry star, and the drift you fought in the Faithful becomes a fall you ride.
It carries an invented, collision-checked name (per
[ADR 0002](../../docs/adr/0002-naming-and-ip-policy.md)) — *accretion* is matter
spiralling inward onto a mass under gravity, exactly this Remix's loop. Choose it from
Asteroids' mode-select.

**▶ Play: https://geox123.github.io/minigames/asteroids/** — pick ACCRETE on the
opening screen.

## Controls

| Key | Action |
| --- | --- |
| **← / →** or **A / D** | Turn the ship |
| **↑** or **W** | Thrust along the facing |
| **Space** | Hold to fire |
| **Shift** or **C** | Collapse — spend a full meter on a gravitational shockwave |
| **P** | Pause / resume |
| **R** | Restart the run |
| **F** | Toggle fullscreen |
| **Esc** | Back out to ACCRETE's mode menu |

On the mode picker, **↑ / ↓** move the highlight and **Enter / Space** starts the run.

## The well, and flight against it

The well pulls on every body with a softened inverse-square force that grows as you
near it, reaching *across* the screen-wrap toward the nearest image of the star. You
fly the Faithful's cannon against that pull:

- **Slingshot.** A close, tangential pass whips the ship up and bends its heading —
  the well is a tool for repositioning, not only a hazard. It falls straight out of
  the gravity model; no special case makes it happen.
- **Feed it.** Shoot rocks and they still split into faster fragments — but now
  everything curves inward, and steering rocks into the well makes it **accrete**
  them for escalating score, the more the larger the rock. A fast, steady feed builds
  a **streak** multiplier; let it lapse and the multiplier falls away.
- **Skim it.** Skimming the well's edge on a close pass — without crossing its core —
  charges a **collapse** meter, once per pass, and only while you are vulnerable:
  flirting with the pull is the reward.
- **Collapse it.** Spend a full meter on a **gravitational shockwave** that erupts
  from the well, flinging the rocks outward and destroying the enemies and fine debris
  it catches — the screen-clearing panic button you earned.
- **Don't fall in.** The well pulls you too. Cross its core and your ship is gone — a
  shield does not save you; the core is absolute.

Downed enemies sometimes drop **power-ups**: a weapon that steps up a ladder
(spread → pierce → rapid), a **shield** that soaks one hit, and a **collapse charge**
that fills the meter.

## The orbital enemy zoo

Alien craft ride the same gravity and threaten differently — the **pattern zoo**:

- **Orbiters** settle into an orbit of the well and fire aimed shots.
- **Divers** enter at an edge and fall across on a gravity-bent path, firing ahead.
- **Mines** drift inert in the pull until you near, then wake and rush you, detonating
  on contact.
- **Shepherds** herd the rock nearest them straight at you.

Waves rotate through the kinds and thicken, speed up and tighten each cycle, plateauing
at a cap.

## The boss

Each **system** builds to a **boss** — a **rival well** whose own pull joins the field,
so you fight in a binary-well gravity. Its hull armours off your fire; only a shot that
finds one of its **rotating weak-point cores** bites. It runs **phases** as it wears
down — calm, then pressing, then enraged — each with its own fire, and a collapse tears
a heavy chunk off it. Fell it to **advance the system**, which deepens the gravity and
the fire.

## The modes

- **Orbit** — a finite, winnable ladder of systems, each capped by a boss, on a
  run-long pool of lives. Fell the final system's boss and the run is **won**.
- **Maelstrom** — endless, the well tightening and the field flooding, scored for
  survival; your **best score** is kept.
- **Daily** — the calendar day's **shared seed**, so everyone plays the same run that
  day; the day's best is kept.

A run resolves on a **summary** card — won or lost, the score, the system reached, the
rocks accreted, and the mode's best.

## The feel

ACCRETE wears bright neon on a near-black field, deliberately apart from the Faithful's
stark white vectors so the two takes read apart at a glance: the well a bright core
inside a warping accretion glow with a lensing shimmer, each enemy kind in its own
colour and silhouette, the boss's hull red with warning-yellow cores. On top: an
orbital ship trail, particle bursts on accretions, skims, power-ups, boss hits and
deaths, an expanding shockwave ring on a collapse, screen shake and a beat of hit-stop
on the big impacts, a synth voice for every event, and a continuous **gravity hum**
that deepens as the well tightens. All art and audio are original and synthesized —
nothing ripped ([ADR 0003](../../docs/adr/0003-code-drawn-visuals.md)).

## Under the hood

- The **core is pure and deterministic**, advancing in fixed 120 Hz steps: the same
  seed and inputs always replay the same run (a Daily depends on it). The gravity acts
  on every body through one shared field; escalation — more enemies, tighter fire,
  deeper gravity — is folded into the same maths, with no per-stage special-casing. The
  ship's **loadout** is passed *in* at construction, so the core never knows the concept
  of "unlocks" — it only flies whatever it is handed, exactly as RIFT's and HAILFALL's
  cores do.
- Every rule is unit-tested through the core's single `step` seam — gravity and the
  slingshot, firing and splitting, accretion and the feed streak, the skim and the
  collapse, the enemy zoo, power-ups, and the modes' run-ends. Reaching and felling a
  **boss** and a full **Orbit** victory can't be cleanly staged by honest play, so
  those use white-box tests that run the real step path.
- The best Maelstrom score and today's Daily persist via the tiny
  [`asteroids-storage`](storage/src/lib.rs) crate — a file natively, `localStorage` in
  the browser — the one place ACCRETE uses `unsafe`.

Cross-run **unlocks** (ship options earned by playing) and **Ascension** modifier tiers
are a planned Phase B, the same shape RIFT's and HAILFALL's metas took; the core already
takes its loadout in, so they land without touching a Phase-A rule.

```sh
cargo run  -p asteroids                 # play it natively (choose ACCRETE)
cargo test -p asteroids-remix-core      # ACCRETE's rules
```
