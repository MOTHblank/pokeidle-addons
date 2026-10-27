# Hunt Atlas native interface — implementation audit

## Scope and architectural decisions

The controller uses **eframe / egui 0.36**, not React or a browser WebView.
`controller/src/app.rs` handles the global dashboard/sidebar and stores user-selected
account navigation, per-account explorer filters, subtab, pagination, cached forecasts and transient command errors.
`controller/src/app/hunt_panel.rs` owns the native Hunt Atlas presentation and
its bounded forecast cache. It consumes account-scoped `HuntView` snapshots.

Source of truth, in order:

1. `native_background::NativeSession` authenticates to the game using the
   native game socket; `native_hunts::HuntIntel` parses `welcome`, `estado`,
   `campo.init`, and `batalha` into native state and independent battle receipts.
2. The authenticated `welcome.hunts` catalog determines which hunts are
   actually available and which hunt-switch commands are allowed.
3. `native_atlas::Reference` downloads a **read-only**, bounded public guide
   dataset for species, moves, typing and rough battle forecasts. It does not
   execute JavaScript, transmit session credentials or override server unlocks.
4. `native_history::HistoryRecorder` persists server-confirmed counters and
   drops by account. The native UI never infers history from rendered elements.

The independent guide and the logged-in game's state may be incomplete.
The interface distinguishes **measured / calibrated estimate / experimental
estimate / unboosted same-pace scenario / unavailable**. Zero is never substituted
for an unknown recommendation. Trainer and Pokémon XP streams remain separate.

## Feature inventory

| Capability | Native status | Evidence / limitation |
|---|---|---|
| Dashboard navigation | Implemented | Hunt Atlas indented under Dashboard, before Accounts |
| Current hunt and encounter | Implemented | Server hunt slug, available hunt name/level, wild HP and Center status |
| Active Pokémon and IVs | Implemented | Server roster, potency/quality and IVs; unknown attributes shown as dashes |
| XP bonuses | Partial | Live VIP, guild, Twitch, events, boost flags; unknown store boosts are not invented |
| Hunt unlock list | Implemented | `welcome.hunts` only; no guide-only destinations presented as unlocked |
| Trainer/Pokémon XP/h | Implemented | Independent observed reward streams; no double counting |
| Kills/h and net gold/h | Implemented | Measured window; negative net gold may reflect spend |
| Battle forecast | Experimental | Reference move DPS is approximate; not a verified exact combat scheduler |
| Recommendations and ranking | Expanded | Hunt ranking plus species-first explorer and Top 5, with source classification and no fabricated zero |
| Hunt filters and search | Expanded | Per-account query, area, range of hunt levels, authoritative availability, type, multi-type weaknesses any/all, depot ownership and objective/direction |
| Spawn composition | Implemented | Native server weights, optional guide species name/type |
| Reference drop probabilities | Implemented, reference-only | Per-species guide item ID, chance, quantity and guide valuation, with no claim of observed odds |
| Hunt switch | Implemented | Native game command; errors, pending confirmation, disabled if not live |
| Session XP, gold, drops | Implemented | Server receipts, session reset and item counts |
| Session ball tracking | Implemented | Ball counts and empirical per-ball success, not predicted capture odds |
| Persistent battle history | Implemented | Native history panel, 90-day detail and unpruned lifetime totals |
| Disconnected/stale/loading/error | Implemented | Connection phase, last-packet age, read-only stale view, retries |
| Resize and long tables | Implemented | Single main scroll, compact responsive rows, 35-result pages |
| Forecast calculation cost | Improved | Off-frame 20-second per-account cache; invalidated on catalog/Pokémon/reference/connection change; live measurements bypass models |
| Species-specific capture odds | Conditional model | Ultra/Beast from public creature evolutionary stages (20/40/100) and authenticated ball efficiency when present; unknown stages stay unknown |
| Shiny capture / egg outcomes | Conditional model | Configurable lure/event/shiny/catch assumptions; generic Outland shiny capture remains unknown |
| Rare loot/outland forecasts | Conditional model | 1–8 levels, multipliers, rare items and horizons; Bicycle server hook still unverified |
| Market profit and farming economics | Partial | Guide loot, modeled combat, authenticated ball quotes, manual market scenarios; no completed-sale price feed |
| Historical forecast residual charts | Partial | Archived hourly model checkpoints and bounded timed receipts, not exact hour-paired outcomes |
| Atlas global map and encyclopedia | Not ported | Reference species exists, but map and captured-species screens are not in this UI |

## Native.20 feature and UX expansion

The default **Explore Pokémon** view is structured around a species, its known types,
potential weaknesses and all matching destinations, rather than a single hunt row.
Filters can be collapsed. Search restrictions and pagination are retained per
account, with independent cached models. Each species exposes its best match,
unlocked hunt action (game command validated by the server list), spawn composition,
experimental XP/hour, gold/hour, kills/hour and guide-sourced expected item yields.
The comparison screen shows all four accounts in parallel, with live trainer/Pokémon
XP and net gold clearly distinct from each profile's three leading experimental
hunts. The shared account strip above all native data pages shows connection state; switching takes one click and exits Atlas comparison when an account is selected.

The guide's `hunts` catalog is parsed as bounded **read-only preview data**. It
contains destinations which may be locked for the account; these are never
presented as server-unlocked, cannot be selected, and are not written to the native
game state. Only `welcome.hunts` can authorize hunt switching. Collection filtering
means *currently in the server-confirmed Pokémon depot*, **not** ever caught.
Until a full `pokemons` roster is received, both "in depot" and "not in depot"
are unavailable. A reconnect marks the roster unknown again.

The guide's item labels (often strings, not numeric IDs) are retained for expected
drop units/hour. A selected item only yields a value if the reference has a
validated chance/quantity and there is a supported combat forecast. The
conditional shiny encounter model uses the userscript's base 1/24,000 outside
Outland, explicitly **without** claiming capture or shiny-capture odds.
Gross modeled gold is broken into combat and expected NPC loot and is **not**
compared to observed net wallet change. Trainer and Pokémon XP/kill are separate.

**Still missing relative to the 1.9.25 userscript:** verified Ultra/Beast species
catch probabilities and ball-efficiency selection, full Outland 1–8 rare
loot/egg outcomes, event/lure/Kick what-if controls, market valuations,
species-specific historical capture completion, survival-aware combat timing,
market-derived profitability and full forecast residuals. They require additional
native inputs/model ports; generic guide loot probabilities or depot ownership
cannot substitute for those features.

New tests exercise locked-vs-unlocked source separation, no inferred ownership
from partial rosters, selected weakness/ownership filters, named guide loot,
forecast economics and missing conditional projections. **This editing environment
does not have a Rust toolchain or a native game session.** Cargo fmt/check/test/
Clippy and actual desktop scaling/performance checks were not executed here.
Local runtime verification remains a blocker to claiming complete quality or
full userscript parity.

## Native.23 capture, Outland, economy and comparison models

The **Capture · Outland · Economy** tab now provides per-account Ultra/Beast
capture modeling from public creature evolutionary-stage metadata
(20/40/100) and authenticated ball efficiency/price quotes where present.
Ordinary-hunt catch probability does **not** depend on NPC value, region or
hunt level; if the stage is unavailable, the rate remains **unavailable**. Wiki fallback efficiencies are explicitly labeled.
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

## Parity checks

The original Manager's `WhereToHunt.tsx` uses active Pokémon, six IV fields,
bonuses, account-scoped unlocked hunts, three ranking modes, filters, per-hunt
kills/XP/gold/TTK and explicit uncertainty notes. These operations are now
available natively without Tauri/WebView. The MOTH Manager fork uses signature
memoization for forecast stability; the native workspace similarly bounds work
without storing continuously changing cloned game-state snapshots.

The Hunt Atlas userscript is **larger** than a Where-to-Hunt list. It implements
capture/ball rarity probability, shiny/egg, rare outland loot, economics,
residual audits and additional filtering. The existence of a native guide reference
is **not** evidence that these models have been fully ported. Don't label the
native UI feature-complete with the userscript until their game inputs and
probability models are independently verified and tested.

## Verification and manual acceptance

Automated Rust tests in `native_hunts.rs` and `app/hunt_panel.rs` cover
battle stream separation, switching, timed-bonus expiration, missing-rate
semantics, stale-rate expiry and scenario source tagging.
The native reference tests cover calibration, separate XP channels and parsing.

Manual visual acceptance still requires executing the desktop build on
Windows/Linux at narrow and wide widths and 100%/150%/200% scaling.
Check switching during reconnect, error dismissal, sort/filter persistence,
forecast model load failure, page navigation, account changes and session reset.
No visual runtime acceptance is asserted by this document.
