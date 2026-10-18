# Native Hunts / Hunt Atlas — experimental architecture

**Branch:** `experiment/native-background-sessions`  
**Preview:** `0.1.7-native.25`

The **Dashboard → Hunt Atlas** workspace is a Rust-native integration of Pokeidle Manager's account-protocol design and the original Hunt Atlas's hunt discovery and combat-estimation goals. It does **not** execute Pokeidle Manager's GPL source or require either Violentmonkey addon after login. The stable main branch is unaffected.

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

Under **Dashboard → Hunt Atlas**, use the app-wide account switcher or open **Compare all accounts** for simultaneous current-rates and best modeled hunts. **Explore Pokémon** is now the default species-first view; it groups destinations, including non-executable public-guide previews of locked hunts. Each account preserves independent explorer filters and result pagination. Filter by location, level range, availability, type, weaknesses (any/all), depot ownership, species/name and objective. Expand details for source-labeled combat and loot estimates or switch immediately to an actual server-unlocked hunt. The existing **Hunt rankings**, **Current hunt** and **Session & rewards** subtabs remain. No background browser tab or userscript is required.

The reference is lazy-loaded once per controller process, rather than polled; normal native gameplay and Twitch run unchanged. Estimated hunt rates are calculated on account-scoped background workers, no more often than a 20-second cache refresh when viewing the panel. Measured rates remain live. Collection means *present in the fully server-confirmed depot*, not historical Pokédex completion.

## Native.23 capture, Outland, economy and comparison models

The **Capture · Outland · Economy** tab now provides per-account Ultra/Beast
capture modeling from bounded public creature catalogs (sell-value-derived
rarity) and authenticated ball efficiency and price quotes where present.
Without the creature rarity catalog, a capture probability remains
**unavailable**. Wiki fallback efficiencies are explicitly labeled.
Predicted captures and shiny catches use weighted spawn composition and the
experimental lead kill rate. Default all-corpses-targeted assumptions are
**upper bounds**; user-configured attempt fractions are optional.

Outland 1–8 uses the userscript's actual level thresholds and tier
multipliers. Bronze token, key/stone/bicycle fragment probabilities,
expected hours, 24h chances and crafting times are shown separately from
ordinary shiny egg thresholds. Tier choice is a *preview*, not a command
confirmation. The Bicycle Fragment source is not verified against a live
server hook. Capture, shiny, Kick, event, loot and guild boost factors are
opt-in what-if controls, **never guessed as active**.

Economics shows independently: modeled combat gold, guide reference NPC
drops, optional expected named-item units/hour, manual player-market
gold/unit assumption, and catalog-known ball costs in their actual
currency. Only a fully sourced, gold-denominated scenario produces the
conditional balance. Realized sales, market price histories, shop stocks
or a complete tax/potion/revive ledger are not available here; no live
profit claim is made. Capture/shiny, Outland rare and market scenarios can
also sort the species explorer. The former sprawling advanced filter block
now starts collapsed, with search and objective selection immediately
visible and responsive model rows at narrow widths.

Historical **current-model vs observed** comparisons use same-hunt
time-attributed kill intervals, rejecting elapsed idle gaps over 120s
and missing trainer/Pokémon reward keys. Past totals without a valid
denominator are never converted into rates. Additionally, the authenticated
native worker archives point-in-time forecasts at most once per 15 minutes
while Atlas is visible, in the existing account-isolated 90-day history.
The UI shows archived model checkpoints against same-day observed rates,
with a clear warning that the observation and forecast hours may differ.
Old history files remain readable without fabricating archived models.

Added unit tests for public creature rarity, ball currency alternatives,
all eight tier locking and rare probabilities, independent XP channels,
capture edge cases, market input validation, time-attributed history
serialization and archived model history. **Rust fmt/check/test/Clippy
and actual desktop runtime checks were not executable in this connector
environment**; static structural inspections alone do not establish a
passing build. These limitations should be addressed before claiming
production-verified, exact live-model parity.

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


## Gold/hour correction (17 October 2026)

Following the 15 October upstream change in `shared/recompensa-hunt.mjs`,
ordinary NPC-resellable loot no longer drops. Instead, each wild kill deposits
base gold plus its converted loot bonus. The native forecast uses the upstream
gold cap at level 25,000, a 15% converted-loot target below the cap, and the
0.30 high-level target exponent above the cap. Known guild loot bonuses apply
only to the converted-loot portion; missing store and event boosts are not guessed.

**Measured gross combat Coins/hour** comes from actual `batalha.ev`
`k=morte, quem=selvagem, ouro` receipts and excludes `ouroVendaAuto`.
It is valid only when all kills in the observation window have receipts.
**Wallet net Coins/hour** instead uses `estado.gold` deltas and may be negative
because it includes purchases, capture costs, and auto-sales. Do not compare
wallet net directly to projected gross gold. Gold-sorted Hunt Atlas now uses
measured gross for the current hunt, calibrated forecasts where possible,
and clearly identified same-pace scenarios otherwise. Without reliable combat
measurements, uncalibrated gross gold estimates are hidden instead of ranked
as realistic earnings.

Regression tests cover payout anchors, >25,000 scaling, gross-versus-net
ledger separation, missing receipts, and Explorer cache invalidation.
