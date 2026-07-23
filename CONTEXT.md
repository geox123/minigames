# Minigames

A continuously growing collection of recreations of games from the first generation of videogames onward, all written in Rust, playable on desktop and in the browser.

## Language

**Game**:
One recreated title in the collection (e.g. Pong). Every Game ships two versions: a Faithful and a Remix.
_Avoid_: minigame (in code/docs — the repo name is enough), title, project

**Faithful**:
The version of a Game that preserves the original's rules, mechanics, difficulty, and feel at its original logical resolution, wrapped in a modern shell (crisp scaling, modern input, pause/restart/fullscreen).
_Avoid_: clone (ambiguous — could mean either version), port, emulation

**Remix**:
The reimagined version of a Game, grown out of its Faithful core after the Faithful ships — modernized mechanics, juice, twists, or new modes. Players choose Faithful or Remix from the same Game.
_Avoid_: reimagining, plus-version, sequel

**Done**:
The state of a Game whose Faithful and Remix have both shipped. Only a Done Game unlocks work on the next Game.
_Avoid_: finished, complete

**Collection**:
The full set of Games in this repo, developed one Game at a time in roughly chronological order of the originals.
_Avoid_: arcade, library
