# Minigames

A continuously growing collection of recreations of games from the first
generation of videogames onward — all written in Rust, playable on desktop and
in the browser.

**▶ Play it: https://geox123.github.io/minigames/**

---

## The idea

Every game in the Collection ships **two takes**:

- A **Faithful** — the original's rules, difficulty and feel preserved at its
  original logical resolution, wrapped in a modern shell: crisp integer
  scaling, modern input, pause, restart, fullscreen, sound.
- A **Remix** — a reimagined version that grows out of the Faithful once it
  ships: modern mechanics, juice, new modes. It carries its own invented name.

A Game is **Done** only when both its Faithful and its Remix are live — and only
a Done Game unlocks work on the next. The Collection grows one Game at a time,
in roughly the order the originals shipped.

The full vocabulary lives in [`CONTEXT.md`](CONTEXT.md); the decisions behind it
in [`docs/adr/`](docs/adr).

## What's here

| Game | Year | Faithful | Remix | Status |
| ---- | ---- | -------- | ----- | ------ |
| [Pong](games/pong) | 1972 | ✅ playable | ✅ [PULSE](games/pong/PULSE.md) | **Done** |

**Pong is Done** — the Collection's first: both its Faithful and its Remix,
[PULSE](games/pong/PULSE.md), have shipped. Per the rules, that unlocks work on
the next Game.

## How it's built

- **[macroquad](https://macroquad.rs)** for all 2D-era Games ([ADR 0001](docs/adr/0001-macroquad-for-2d-era.md)):
  a small graphics library that builds to WASM with no ceremony.
- **A Cargo workspace, one crate per Game.** Each Game is split in two:
  - a **pure, deterministic core** — every rule of the game, advanced in fixed
    timesteps, with no dependency on the engine or the clock. This is where the
    tests live.
  - a **thin macroquad shell** — window, real input, rendering, audio, the
    front-end screens. It owns nothing about the rules.
- The split means the interesting logic is testable without a window, and the
  same core runs identically on desktop and in the browser.

```
games/pong/
  core/    # pure rules, no engine — the tested seam
  shell/   # macroquad: window, input, render, audio, menus
```

## The quality bar

Every push and pull request runs [the same gate](.github/workflows/ci.yml):

- `cargo fmt --all --check` — formatting
- `cargo clippy --workspace --all-targets -- -D warnings` — no warnings
- `cargo test --workspace` — the whole suite
- a native **and** a WASM build

`unsafe` is denied at the crate level. Each Game's core is unit-tested through
its public seam only — state in, state out — and is deterministic, so the tests
mean what they say. Once that bar is green, every push to `main`
[rebuilds and redeploys](.github/workflows/ci.yml) the site.

All assets are original — sound is synthesized, nothing is ripped from an
original ([ADR 0002](docs/adr/0002-naming-and-ip-policy.md)).

## Playing and building locally

Play the current build in your browser at the link above, or run it natively:

```sh
# Native desktop build
cargo run -p pong

# Run the core's test suite
cargo test --workspace

# Assemble the web build into dist/ (needs the wasm target)
rustup target add wasm32-unknown-unknown
bash scripts/build-web.sh
```

Per-Game controls and design notes live in each Game's README — see
[games/pong](games/pong/README.md).

## Licence

Dual-licensed under either of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE),
at your option.
