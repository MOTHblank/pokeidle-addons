# Native Hunts / Hunt Atlas — experimental architecture

**Branch:** `experiment/native-background-sessions`  
**Preview:** `0.1.7-native.12`

The **Panel → Hunts · Atlas** tab is a Rust-native integration of Pokeidle Manager's account-protocol design and the original Hunt Atlas's hunt discovery and combat-estimation goals. It does **not** execute Pokeidle Manager's GPL source or require either Violentmonkey addon after login. The stable main branch is unaffected.

## Server as source of truth

`native_hunts.rs` consumes actual authenticated PokéIdle game frames, not DOM text:

- `welcome.hunts`: unlocked hunts, slugs, areas, regions, levels, total spawn count, species IDs and weights.
- `welcome.estado` and incremental `estado`: trainer and Pokémon XP, gold/diamonds/orbs, active Pokémon and roster (`pokemons`, `pkMud`), held items, Pokémon IVs/quality/potency/types/power, inventory (`balls`, `items`), wild enemy, automation fields, server clock, VIP, Twitch/guild/event bonuses, center state.
- `batalha.ev`: server-confirmed hunt navigation, center transitions, wild deaths and capture results.
- `hunt.select` is sent using the existing native command queue. Hunt availability is validated against the actual `welcome.hunts` catalog and displayed as pending until confirmed by the server.

Full Pokémon roster/inventory domains are retained in read-only shared snapshots using `Arc` so the lightweight game/Twitch worker does not repeatedly clone every item. Incremental patches are merged instead of overwriting missing properties.

## Actual measurements versus Atlas projections

The live panel shows **separate trainer XP/h and Pokémon XP/h**, net gold/h and kills/h, observed from server deltas over a rolling ten-minute window (minimum 20 seconds). The two XP streams are **never added together**. Negative net-gold rates remain visible when purchases consume funds. A changed active Pokémon, changed hunt or disconnected session invalidates stale measurements.

The public, read-only reference used by Pokeidle Manager (`https://guiapokeidlehardtocapture.site/data.js?v=32`) is fetched **only when the Hunts tab opens**, with a timeout and 4 MiB body limit. Its `window.PI_DATA =` JSON data is parsed as data, **never executed**. No credentials, cookies, game messages or account identifiers are sent to the guide. The tab continues to work with the account's native hunt catalog if the reference host is offline.

`native_atlas.rs` models available wild species using reference stats, types/moves, move cooldowns, active Pokémon IV/quality/potency and spawn weights. It offers EXP/h, gold/h, kills/h and time/kill scenarios; the active hunt's real measured rates always take precedence. When a meaningful number of real kills has been observed, a per-account kill-rate calibration scales the speculative guide model. Calibrated estimates **are not guarantees**: the reference may lack server changes, buffs, precise attack scheduling, deaths, movement, and realistic shop/capture proceeds. **Other-hunt estimates remain experimental** and are labeled. A fallback without species reference shows only the scenario "at the current measured kill pace" rather than inventing another hunt's actual kills/hour.

The original **premium Hunt Atlas** has substantially more economics, catch/shiny/rare probabilities, outland modifiers, encounter simulation and historical performance data than this tab currently provides. Those specialized models are **not yet fully ported**; the source repository remains the reference for future parity work. This port establishes the native source-of-truth architecture first to avoid past XP/h mispredictions.

## UI

Under **Panel → Hunts · Atlas**, select one of the four accounts. The tab shows the active Pokémon/IVs, game-reported bonuses, current hunt, independently measured rates, and the unlocked hunt ranking. Filter by hunt, region, target species name/ID and type. Expand a hunt to inspect its weighted spawn composition. Switch directly using `hunt.select` without reopening Firefox. No background browser tab or userscript is required.

The reference is lazy-loaded once per controller process, rather than polled; normal native gameplay and Twitch run unchanged.

## Validation

From this branch, run:

```powershell
cd controller
cargo test
cargo build --release
.\target\release\moth-controller.exe
```

Verify: `Panel → Hunts · Atlas` loads the account's hunts immediately after authenticated `welcome`; the guide data enriches species names when reachable; real XP/h becomes available after enough continuous play; switching to another hunt is acknowledged; returning from Firefox recovery or changing Pokémon resets stale measurements; native Twitch and capture remain online.

The code was committed without a local Rust compiler or authenticated game session. GitHub Actions are not a substitute for confirming the live rates and actual hunt-switch behavior on your account. In particular, **do not treat speculative guide forecasts as measured earnings**.
