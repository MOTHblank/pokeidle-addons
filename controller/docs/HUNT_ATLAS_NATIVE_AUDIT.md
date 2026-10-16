# Hunt Atlas native interface — implementation audit

## Scope and architectural decisions

The controller uses **eframe / egui 0.36**, not React or a browser WebView.
`controller/src/app.rs` handles the global dashboard/sidebar and stores user-selected
account, sorting, query, region, type, subtab, pagination and transient command errors.
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
| Recommendations and ranking | Implemented with caveat | Three ranking criteria, numerical provenance, no fabricated zero |
| Hunt filters and search | Implemented | Region, reference-derived type, hunt/name/area/species search |
| Spawn composition | Implemented | Native server weights, optional guide species name/type |
| Reference drop probabilities | Implemented, reference-only | Per-species guide item ID, chance, quantity and guide valuation, with no claim of observed odds |
| Hunt switch | Implemented | Native game command; errors, pending confirmation, disabled if not live |
| Session XP, gold, drops | Implemented | Server receipts, session reset and item counts |
| Session ball tracking | Implemented | Ball counts and empirical per-ball success, not predicted capture odds |
| Persistent battle history | Implemented | Native history panel, 90-day detail and unpruned lifetime totals |
| Disconnected/stale/loading/error | Implemented | Connection phase, last-packet age, read-only stale view, retries |
| Resize and long tables | Implemented | Single main scroll, compact responsive rows, 35-result pages |
| Forecast calculation cost | Improved | Eight-second scoped cache, invalidated on catalog/account/Pokémon/reference/connection change |
| Species-specific capture odds | Not ported | Userscript capture-ball models need verified native rarity/efficiency inputs |
| Shiny capture / egg outcomes | Not ported | Userscript probability engine and complete native sources are not yet available |
| Rare loot/outland forecasts | Not ported | Userscript rare tiers, horizons and source probabilities absent from native reference |
| Market profit and farming economics | Not ported | Native reference has only partial drop pricing and no verified live market signal |
| Historical forecast residual charts | Not ported | Native history persists reward events, not model/observation pairs |
| Atlas global map and encyclopedia | Not ported | Reference species exists, but map and captured-species screens are not in this UI |

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
