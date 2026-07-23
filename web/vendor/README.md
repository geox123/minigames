# Vendored: `mq_js_bundle.js`

This is macroquad's JavaScript loader — the runtime half of a macroquad WASM
build. It implements the GL calls, windowing, input, audio and clipboard hooks
that the `.wasm` imports, and boots the game via `load("pong.wasm")`.

It is vendored verbatim so the deployed site depends on nothing but its own
files — no CDN, no third-party host at runtime.

- **Source:** the `js/mq_js_bundle.js` shipped inside the `macroquad` crate.
- **Version:** matches the `macroquad` version pinned in the workspace
  `Cargo.toml` (currently 0.4.15). The file declares `version = 2` internally.
- **Licence:** macroquad is MIT OR Apache-2.0.

When the pinned `macroquad` version changes, re-copy this file from the matching
crate source so the loader and the `.wasm` stay in step.

## `pulse-storage.js`

A tiny, self-contained macroquad plugin that gives the WASM two functions to
read and write one number — PULSE's best Gauntlet score — in the browser's
`localStorage`. It passes plain numbers, so it needs no `sapp_jsutils` and can't
fall out of step with the bundle above. The native build persists the same value
to a file instead. Both sides are driven by the `pong-storage` crate, which is
the only place in the project that uses `unsafe` (the two FFI calls). Loaded
after `mq_js_bundle.js` and before the `.wasm` in each Game's page.
