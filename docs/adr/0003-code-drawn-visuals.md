# Visuals are code-drawn; no external-asset pipeline for the 2D era

Every Game in the Collection draws its visuals in code — rectangles, lines, a
hand-rolled pixel font, seven-segment digits, hand-coded pixel bitmaps for
sprites, and programmatic vector polygons — with no bitmap files, sprite sheets,
fonts, or AI-generated art loaded as assets. Pong and its Remix PULSE already
work this way; every 2D-era Game follows.

The reasons compound. It keeps the repo **asset-free**: nothing to license,
embed, host, or accidentally rip, which keeps the IP posture in
[ADR 0002](0002-naming-and-ip-policy.md) simple — original-but-evocative shapes
are authored directly, never extracted from an original. It stays **on
aesthetic**: crisp, low-resolution, code-drawn shapes read as one coherent
retro system across the whole Collection, where mixed-in generated art would
clash with the code-drawn UI. And it stays **cheap and self-contained**: no art
pipeline, no generation credits, no PNG-embedding step, no build-time asset
plumbing in the WASM bundle. The arcade-era Games' imagery — bricks, aliens,
ships, rocks, mazes — is simple enough that code-drawing is not a compromise but
the natural fit.

This is scoped to the **2D era** (arcade through 16-bit), the same span
[ADR 0001](0001-macroquad-for-2d-era.md) standardizes on macroquad. Any AI-art
capability the author has (e.g. PixelLab) serves other projects, not this
Collection. Revisit only if a future 2D Game genuinely cannot be expressed in
code-drawn form, or when the roadmap reaches an era whose look demands authored
art — decide that per Game, as with the framework-decision deferral in ADR 0001.

Consequence: shared drawing primitives (the pixel font, canvas scaling, sprite-
and vector-drawing helpers) accumulate as reusable code and are lifted into a
shared crate as Games need them — not shipped as data.
