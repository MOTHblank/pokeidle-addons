# Native Hunts / Hunt Atlas — experimental architecture

**Branch:** `experiment/native-background-sessions`  
**Preview:** `0.1.7-native.18`

The **Dashboard → Hunt Atlas** tab is a Rust-native integration of Pokeidle Manager's account-protocol design and the original Hunt Atlas's hunt discovery and combat-estimation goals. It does **not** execute Pokeidle Manager's GPL source or require either Violentmonkey addon after login. The stable main branch is unaffected.

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

Under **Dashboard → Hunt Atlas**, select one of the four accounts. The tab shows the active Pokémon/IVs, game-reported bonuses, current hunt, independently measured rates, and the unlocked hunt ranking. Filter by hunt, region, target species name/ID and type. Expand a hunt to inspect its weighted spawn composition. Switch directly using `hunt.select` without reopening Firefox. No background browser tab or userscript is required.

The reference is lazy-loaded once per controller process, rather than polled; normal native gameplay and Twitch run unchanged. Per-hunt forecasts are calculated on a bounded-cadence background worker rather than on the egui render path. List rows have slug-stable expander IDs and pages of 30 entries, and measured/model/scenario/unavailable rates remain visibly distinct. Live metrics are marked stale when the native socket is not online; hunt-switch controls are disabled until reconnect.

## Feature parity audit (native.18)

| Capability | Native status | Source and remaining gap |
|---|---|---|
| Current hunt, wild encounter, active Pokémon, IVs | Supported | Authenticated `welcome/estado/campo.init` and roster patches |
| Account selection and hunt switching | Supported in code | Per-profile native socket and validated `hunt.select`; live confirmation still required |
| EXP/hour, gold/hour, kills/hour forecasts | Partial | Guide-based combat approximation; unknown stats or coverage remain unavailable |
| Measured trainer/Pokémon XP/hour | Supported in code | Distinct reward receipts or state deltas; missing receipt channels are invalid rather than zero |
| Forecast-versus-measured explanation | Supported | Current-hunt breakdown shows channels separately and model/measured ratios |
| Combat efficiency, move timings and survival | Partial | Expected kills/s and broad move DPS; no exact combat timeline or survival/death simulator |
| Location and enemy comparisons | Supported | Search, region/type filters, objective sort and weighted spawn composition |
| Pokémon suitability comparisons | Partial | Forecast assumes *current* lead; alternative roster candidates are not yet ranked |
| Rewards and drops | Partial | Drop odds/quantity/value from guide and actual receipts; no complete catch, rare or shiny model |
| Historical observations | Partial | Native per-account, per-hunt daily totals for 90 days plus lifetime aggregate; no historical time-per-hunt rates |
| Multi-account isolation | Supported in code | Account-scoped view/history and cache key; requires live multi-account regression check |
| Recommendation rationale | Partial | Model coverage, calibration, pace scenario and rates exposed; not full premium Atlas economics |
| Update frequency and background operation | Supported in code | UI preferences and native transport; forecast worker only scheduled while Hunt Atlas is visible |
| Outland 1–8 / rare drops / Ultra-Beast capture / shiny wait | Missing | Present in userscript reference, not in the native model or server-derived calculator |

**Known model limitation:** this implementation does not reproduce the premium Hunt Atlas simulator's exact enemy HP distribution, attack scheduling, capture economics or special modifiers. A calibrated projection is still a hypothesis, *not* an observed rate. Historical XP/h cannot be calculated from saved totals without adding per-hunt observed duration. The new native UI must not be described as full premium/userscript feature parity until those data/calculation paths exist.

**Validation status:** the repository was edited using its GitHub connector, without a local Rust toolchain, live game credentials or a desktop display. The new tests cover provenance, cache account invalidation, incomplete receipt coverage and reconnect boundaries, but **cargo fmt, cargo check/test/clippy and live UI checks have not run here**. A compile, runtime performance profile, and four-account regression pass are mandatory before calling this fully production-verified.

## Validation

From this branch, run:

```powershell
cd controller
cargo test
cargo build --release
.\target\release\moth-controller.exe
```

Verify: `Dashboard → Hunt Atlas` loads the account's hunts immediately after authenticated `welcome`; the guide data enriches species names when reachable; real XP/h becomes available after enough continuous play; switching to another hunt is acknowledged; returning from Firefox recovery or changing Pokémon resets stale measurements; native Twitch and capture remain online.

The code was committed without a local Rust compiler or authenticated game session. GitHub Actions are not a substitute for confirming the live rates and actual hunt-switch behavior on your account. In particular, **do not treat speculative guide forecasts as measured earnings**.

## Native Inventory and Auto Catch settings (native.13)

**Panel → Inventory** shows each account's actual server-reported `estado.balls` and `estado.items` inventory, with an **All accounts** aggregate, name/ID search and category filtering. Pokéball names come from the authenticated `welcome.catalogoBolas`; other names and types come from the public `https://pokeidle.io/assets/items.json`, fetched only when the Inventory tab or Auto Catch editor is opened. No browser or userscript is required, and unknown IDs remain visible without fabricated labels.

**Items acquired** is explicitly separate from stock. The server's `batalha.ev[k=morte].drops` `ganho` (falling back to `qtd`) drives a native per-session tally. Purchases and initial inventory are **never counted as drops**. Manager-style session data also includes kills, captures, XP trainer and Pokémon tracked separately, combat gold, auto-sale gold, ball usage and shinies. Session counters remain intact when switching hunts and reconnecting within the same controller process. The **Hunts · Atlas** current-hunt details include session duration, current wild encounter, live current-hunt information, acquired drops and a confirmable session reset.

**Account details → Automation → Configure native Auto Catch** is now a real Rust-side settings editor: on/off, up to two ball priorities, per-ball reserves, independent restock on/off, gold reserve and ball/potion/revive IDs with buy-at and refill-to thresholds. IDs may be selected from the game's authoritative ball catalog or the optional public item catalog. Edits are validated and saved to the existing encrypted per-account OS credential vault, **then** applied live to the Rust worker without restarting or reopening Firefox. A configuration change preserves unconfirmed restock purchases; resetting a pending order could duplicate buying. The worker now actually enforces per-ball reserves rather than merely displaying them. It does not automate Master Balls.

**Limitations:** only item drops explicitly present in battle events are counted; the session ledger is in memory and begins with the native controller runtime (it does not import past Manager sessions or history from the game). Catalog names are unavailable if the public endpoint is offline, but quantities and item IDs remain correct. The native Auto Catch settings cover capture priority/reserve/restock, not all advanced legacy userscript behavior. The server still controls real purchase availability and prices. The changes have unit tests but have **not** been compiled/live-tested in the editing environment.

Validation: run `cargo test`, then `cargo build --release`. Confirm that selecting Auto Catch preferences while connected changes captured ball choice/reserves, that restocking respects threshold and gold reserve without repeating pending orders, that an inventory purchase changes current stock but does **not** increase acquired drops, and that `morte.drops` events appear in the acquired ledger. Verify all four profiles can coexist natively with Firefox closed.


## Native dashboard data parity in native.14

The account detail drawer now reads one immutable native snapshot per render and
displays active trainer/Pokémon, live currencies, hunt session counters, measured
trainer and Pokémon XP separately, net gold, Pokémon roster with HP and quality,
ball stock and native automation. The compact overview uses that same native
state instead of treating Firefox Bridge data as mandatory. Hunt Atlas and
Inventory remain dedicated tabs reachable from the account drawer.

XP/h now also derives from *server-confirmed wild-death reward receipts* over
the rolling ten-minute window. Missing periodic `estado` messages no longer
leave battle-based trainer/Pokémon XP blank. A 20-second minimum is enforced,
and the rate resets on hunt switch or disconnect. Net gold/h still requires
actual currency snapshots; it is never fabricated from gross battle earnings.

These are native egui improvements, **not a complete React/Tauri Manager
interface port**. No live authenticated game regression test or Rust compiler
was available in the editing environment.
