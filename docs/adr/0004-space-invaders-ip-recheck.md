# Space Invaders ships under its real name, with everything original — the per-title re-check

[ADR 0002](0002-naming-and-ip-policy.md) sets the Collection's posture: Faithfuls
reference the original name plainly, Remixes ship under invented names, and no
asset is ever taken from an original. It also asks that titles from
more-defended franchises be revisited **per title before shipping**, and
[the roadmap](../roadmap.md) flags Space Invaders explicitly — Taito is more
protective than Atari, though a rung below the Nintendo tier. This is that
re-check.

**Decision: proceed as with Pong and Breakout.** The Faithful is presented as
"Space Invaders — a faithful recreation", the descriptive use of a title to say
what the thing *is*, in a free, non-commercial, source-open recreation. The
Remix, when it comes, ships under an invented name of its own.

The reasoning is the same shape as ADR 0002's, with the sharper edges named. Game
*mechanics* — a marching formation, a shooting cannon, eroding shields — are not
copyrightable, and recreating them is long-standing practice. What **is**
protectable is the specific expression: the alien sprites, the marching
four-note bass, the cabinet art. So the mitigation is not the name, it is the
assets, and [ADR 0003](0003-code-drawn-visuals.md) already binds us to them:

- **Sprites are authored, never traced.** The invaders, the cannon, the saucer
  and the shields are hand-coded original bitmaps that evoke the era's
  chunky low-resolution look without reproducing Taito's actual shapes. Nothing
  is copied pixel-for-pixel from a screenshot, a ROM or a sprite sheet.
- **Audio is synthesized, never sampled.** The famous accelerating march is
  re-created as an original four-step descending motif whose tempo follows the
  formation — implementing the *idea* (tempo tracks the alien count) from our own
  oscillators, not shipping Taito's recording.
- **No ROM, no extracted data.** Nothing from the original binary — not sprites,
  not tables, not audio — is embedded or distributed.

Residual risk is low but not nil, and it sits almost entirely on the **title**
rather than the code. That is deliberate, because it is also the cheapest thing
to change: only the window title, the menu heading and two README lines reference
it, so if Taito or Square Enix ever objected, renaming the Faithful under an
invented name is a small, contained edit — the Collection already knows how to
ship a game under a name of its own.

This is an engineering-risk judgement recorded for the project, not legal advice;
the call to ship under the real name is the author's. Revisit if the Collection
ever becomes commercial, if it is distributed through a storefront rather than
GitHub Pages, or on any contact from the rights holder — and again per title when
the roadmap reaches Namco (Pac-Man) and Nintendo, as ADR 0002 already requires.
