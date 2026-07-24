#!/usr/bin/env bash
# Assembles the deployable site into dist/: the Collection index and, for every
# Game, its static page plus a freshly built WASM bundle. Run from the repo
# root. Used by CI and reproducible locally.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

dist="dist"
rm -rf "$dist"

# The static site source (index, per-Game pages, vendored JS loader).
cp -r web "$dist"

# One entry per Game: crate name -> its folder under the site.
build_game() {
  local crate="$1" folder="$2"
  echo "building $crate -> $dist/$folder/"
  cargo build --release -p "$crate" --target wasm32-unknown-unknown
  cp "target/wasm32-unknown-unknown/release/${crate}.wasm" "$dist/$folder/${crate}.wasm"
}

build_game pong pong
build_game breakout breakout
build_game stepfall stepfall

echo "done: $dist/"
