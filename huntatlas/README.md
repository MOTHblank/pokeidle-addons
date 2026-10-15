# PokéIdle Hunt Atlas — compatibility entry point

**Current version: 1.9.25.** The canonical public script is [`addons/hunt-atlas.user.js`](../addons/hunt-atlas.user.js). The older [`huntatlas/hunt-atlas.user.js`](hunt-atlas.user.js) file is now an **identical** compatibility copy, including `@downloadURL` and `@updateURL` directed at the canonical public version. It no longer carries the old v1.9.21 XP/h estimator.

If you previously installed the older `huntatlas/` path (which had no automatic-update URL), **install the current public script once** from the canonical path and ensure only one Hunt Atlas instance is enabled in your userscript manager. Future updates will use the canonical URL.

The model displays **trainer XP/h** = *trainer XP per kill × estimated kills/h*. Pokémon XP/h is separate, never added to the trainer figure. The older upstream Hunt Analyser generic session `xpH` sums both streams; Hunt Atlas does not use that combined number. The old v1.9.21 estimator used an optimistic ~1.5-second single-target kill cadence that could produce the reported 2× overprediction. Current versions simulate sustained move readiness, basic attacks, action pacing, and variable enemy HP.

Regression tests for the **maintained** script: `node --test tests/hunt-atlas-combat.test.cjs` from the repository root. Older experimental milestone test files in `huntatlas/tests/` refer to historical APIs and are not the current combat regression suite.
