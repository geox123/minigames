# The 1978 invasion game ships as STEPFALL — a name of its own

[ADR 0002](0002-naming-and-ip-policy.md) sets the Collection's posture: Faithfuls
reference the original name plainly, Remixes ship under invented names, and no
asset is ever taken from an original. It also asks that titles from
more-defended franchises be revisited **per title before shipping**, and
[the roadmap](../roadmap.md) flags this one explicitly — Taito is more protective
than Atari. This is that re-check, and unlike Pong's and Breakout's it resolves
**against** using the original name.

**Decision: the Game ships under an invented name — STEPFALL.** Its Faithful is
still faithful to the 1978 original's rules, difficulty and feel; its Remix, when
it comes, gets an invented name of its own as every Remix does.

## Why this one deviates

ADR 0002's real-name posture is a judgement about *risk versus honesty*, and it
already anticipated that the balance shifts per title. Here it does. The benefit
of using the mark is that people immediately know what the thing is — and that
benefit is almost entirely recoverable through an honest one-line description.
The cost is concentrated in exactly one place: using someone's trademark **as the
name of your product**, which is the use that implies origin or endorsement. So
we keep the description and drop the name.

Concretely, we still say plainly what this recreates — "a faithful recreation of
the 1978 arcade alien-invasion original", naming Taito's *Space Invaders* once as
the thing it descends from, alongside a note of no affiliation. Referring to a
trademark to describe what your work relates to is ordinary nominative use, and
the Collection's whole premise is being honest about what it recreates. What we
no longer do is put the mark on the tin.

## What has not changed

The assets were always the larger exposure and they remain governed by
[ADR 0003](0003-code-drawn-visuals.md): every sprite is a hand-authored bitmap
that evokes the era without reproducing Taito's shapes, all audio is synthesized
from our own oscillators (including the accelerating march, which implements the
*idea* that tempo tracks the alien count), and nothing — sprites, tables, audio —
comes from the original binary.

## The name

**STEPFALL** is coined rather than borrowed, so it is ours to use, and it names
the game's signature motion: the formation steps sideways, then falls a row. It
was checked for obvious collisions before adoption; an earlier candidate was
dropped for colliding with an existing arcade platform. The crate and paths are
`stepfall`; the Collection lists the Game as STEPFALL, after the 1978 original.

## Consequences and scope

This supersedes the first draft of this ADR, which proposed shipping under the
real name; the author chose the invented name instead. It also sets the pattern
for the rest of the roadmap: when the Collection reaches Namco (Pac-Man) and
Nintendo, expect the per-title re-check to land here too, and budget an invented
name from the start rather than renaming late.

Recorded as an engineering-risk judgement for the project, not legal advice.
Revisit on any contact from a rights holder, or if the Collection ever becomes
commercial or is distributed through a storefront rather than GitHub Pages.
