# The 1980 maze game ships as GNASH — a name of its own, and an original maze

[ADR 0002](0002-naming-and-ip-policy.md) sets the Collection's posture: Faithfuls
reference the original name plainly, Remixes ship under invented names, and no
asset is ever taken from an original. It asks that titles from more-defended
franchises be revisited **per title before shipping**, and both
[the roadmap](../roadmap.md) and [ADR 0004](0004-space-invaders-ip-recheck.md)
flag this one by name — Namco sits a clear rung above Taito, and 0004 told us to
"budget an invented name from the start" when the Collection reached it. This is
that re-check. Like STEPFALL's, it resolves **against** using the original name —
and, unlike any re-check before it, it constrains the **maze and the cast** too,
not only the tin.

**Decision: the Game ships under an invented name — GNASH.** Its Faithful is still
faithful to the 1980 original's rules, difficulty and feel; its Remix, when it
comes, gets an invented name of its own as every Remix does.

## Why this one is stricter than STEPFALL's

STEPFALL's re-check landed on the name and left the assets where
[ADR 0003](0003-code-drawn-visuals.md) already had them — hand-authored bitmaps
that evoke the era. Here the same logic bites harder, for two reasons specific to
this title.

First, **the rights-holder.** Namco's 1980 maze game is not an arcade-era
also-ran; it is a flagship, still actively merchandised and enforced forty-plus
years on. The wedge-shaped eater and the four coloured pursuers are among the most
recognisable characters in the medium, and their shapes function as trade dress,
not just the word-mark. The benefit of using the real name — instant recognition —
is, as in 0004, almost entirely recoverable through an honest one-line
description; the cost of using someone's flagship mark *as the name of your
product* is at its highest here. So we keep the description and drop the name.

Second, **where a maze game's identity lives.** A shooter's exposure is mostly its
sprites, which 0003 already handles. A maze game's identity is also its **maze** —
the exact wall layout, and the specific cast of characters that inhabit it, are as
iconic as any sprite. So this re-check extends the original-but-evocative rule
past the sprite sheet to two more surfaces:

- **The maze is an original layout.** It keeps the *structure* that makes the
  genre work and the rules legible — a rectangular tile maze, four corner power
  pellets, a central pen the pursuers start in, and side tunnels that wrap — but
  the actual wall arrangement is our own design, not a re-drawing of Namco's.
- **The cast is original-but-evocative.** One hungry eater and four distinct
  pursuers, drawn as our own shapes in our own palette, never a reproduction of
  the famous wedge or the scalloped ghosts. The bonus items are likewise our own,
  keeping only the escalating **score ladder**, not Namco's specific fruit.

## What is faithfully recreated — the rules, not the look

What makes this Game worth doing is its **behaviour**, and behaviour is a game
mechanic, freely recreatable. We reproduce, faithfully, the 1980 original's rules
and feel: the four pursuers each hunt by their own **distinct targeting rule**,
the world cycles between **scatter** and **chase**, a power pellet flips the hunt
and turns the pursuers edible for a shrinking window, they are released from the
pen on the original's dot-count logic, the eater corners a touch faster than it
runs the straights, and the difficulty ramps level over level exactly as the
original's did. This is the roadmap's intended new engine muscle — a **tile maze
with pursuit AI** — and none of it is anyone's property. We say plainly what it
recreates ("a faithful recreation of the 1980 arcade maze-chase original"),
naming Namco's *Pac-Man* once as the thing it descends from, alongside a note of
no affiliation. What we no longer do is put the mark on the tin, copy the maze, or
copy the cast.

## The name

**GNASH** is coined rather than borrowed, so it is ours to use, and it names the
game's signature action: the relentless chomping bite that carries the eater
through the maze, with a fitting note of menace from the pursuit. It was checked
for obvious collisions before adoption — the near neighbours *GNAW* (a 2026 title)
and *GOBBLE* (a period Pac-Man clone) were both taken and rejected on that
ground. The crate and paths are `gnash`; the Collection lists the Game as GNASH,
after the 1980 original.

## What has not changed

Everything the earlier ADRs settled still holds. Art remains code-drawn
([ADR 0003](0003-code-drawn-visuals.md)): the maze, the eater's chomp, the
pursuers and the bonus items are all authored directly in code, and audio is
synthesized from our own oscillators — including the siren that tracks the chase.
Nothing — layout, sprites, tables, audio — comes from the original binary.

## Consequences and scope

This continues the pattern 0004 predicted and tightens it for a maze game: when
the Collection reaches a franchise whose *level design or cast* is itself the
icon (as here, and as the Nintendo-era titles will be), the re-check constrains
those surfaces too, not only the name — and, as always, budget it from the start
rather than retrofitting late.

Recorded as an engineering-risk judgement for the project, not legal advice.
Revisit on any contact from a rights holder, or if the Collection ever becomes
commercial or is distributed through a storefront rather than GitHub Pages.
