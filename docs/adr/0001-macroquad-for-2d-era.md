# macroquad for 2D-era games; 3D framework deferred

The collection recreates games chronologically from Pong onward, targeting native desktop and browser (WASM). All 2D-era games (arcade through 16-bit, including sprite-based fighters) standardize on **macroquad**: its ceremony-to-game-logic ratio fits arcade-scale games, WASM builds work with zero config, and quality shows in the game code rather than framework plumbing. Bevy was considered and rejected for this era — its ECS ceremony, compile times, WASM bundle size, and 3-4-monthly breaking releases are costs paid on every small game while its strengths only pay off at much larger scale.

The framework for 3D-era games (Mario Kart onward) is **explicitly deferred** until the first 3D game is actually next — deciding today would buy nothing but today's costs. Known consequence: any shared code accumulated by then (input, high scores, shell) will be macroquad-coupled and need porting or bridging.
