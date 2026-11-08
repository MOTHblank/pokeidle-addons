# /hunt for PokeIdle 1.0.17

Exact extracted Chrome Web Store snapshot of **/hunt for PokeIdle** by Liang, kept as a third-party reference for Hunt Atlas research.

- Extension ID: `mhnplloljgchpfpcpbhmcokmnefodjnh`
- Extension version: **1.0.17**
- Manifest version: 3
- Snapshot added: 2026-11-08
- Signed CRX: `package.crx`
- Original CRX Git blob: `d498efc9021426390f90caf4473eaa07f1dfdc4e`

The extracted files and Chrome's `_metadata/verified_contents.json` are kept alongside the original signed CRX, so this directory can be audited directly without installing the extension.

## Useful files

- `src/engine.js` — hunt simulator and ranking model: combat, pacing/dead-time, XP/gold, capture, seasonal effects, gym podium, AoE and confidence/range handling.
- `src/ritmo.js` — generated per-map/per-hunt pacing dataset. In 1.0.17 it is based on the game's 1.409.0 simulator/export rather than the older October telemetry-only table.
- `src/geo.js` — records actual map identity and observed hunt data; geometry is not the primary ranking model.
- `src/game.js` — reads and normalizes live game state used by the estimator.
- `src/hook.js` — hooks the game's live messages/state updates.
- `src/jogo-bundle.js` — bundled PokeIdle game modules used by the extension under Manifest V3.
- `src/ui.js` / `src/ui.css` — ranking UI and presentation.
- `manifest.json` — load order and host scope.

## Why this is here

This is reference material for comparing Moth Hunt Atlas against another current estimator, especially pacing, map identity, movement/dead-time interaction, bicycle handling, kill rate, XP/h and confidence behavior.

Nothing in this directory is imported or executed by Moth Controller.

## Attribution / licensing

This is third-party work by Liang. Treat its licensing independently from `pokeidle-addons`; do not assume these files inherit this repository's license.
