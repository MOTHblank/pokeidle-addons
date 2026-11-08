# /hunt for PokeIdle — third-party reference

This directory keeps a source snapshot of the Chrome extension **/hunt for PokeIdle** by Liang for research, regression comparison, and Hunt Atlas calibration work.

- Chrome Web Store ID: `mhnplloljgchpfpcpbhmcokmnefodjnh`
- Vendored source snapshot: **1.0.12**
- Snapshot source: `GabrielGamaBaptista/IdleStream`
- Source commit: `c5a9a1ee4bef67695ab051b0595354eb214e9644`
- Mirrored file: `src-tauri/src/games/hunt_extension.js`
- Added to this repository: 2026-11-08

The Chrome Web Store listing reported **1.0.17** on 2026-11-08. Chrome-Stats also exposes an archived **1.0.13** manifest/package listing, but its full source explorer is gated. This directory therefore keeps the newest complete public source snapshot that could be independently verified rather than labeling an incomplete or reconstructed package as current.

## Scope

This code is **reference-only**. It is not imported by Moth Controller, bundled into releases, or treated as an implementation dependency. The purpose is to make comparisons of pacing, geometry, combat, XP/h, bicycle handling, and related hunt estimation logic reproducible inside this repository.

## Attribution and licensing

The extension is third-party work by Liang. No license file was present in the mirrored snapshot used here. Do not assume this code is covered by the license of `pokeidle-addons`; preserve attribution and treat reuse separately from reference/audit use.
