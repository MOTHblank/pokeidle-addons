# Native Moth Watch rule buying

Moth Controller's **Automation → Market rule buying · Moth Watch** panel supports native, per-account market buying without Violentmonkey or a browser userscript. It is independent of native Auto Catch, shop restocking and server automation.

## Configuring

1. Select the intended account. Market settings are saved separately for Game1–Game4, including when that account is offline.
2. Add one or more specific-item, any-item or Pokémon rules, set currency (Gold or Gems) and a **maximum unit price**, and enable the individual rules. For Pokémon, select **Any**, **P5** or **Shiny**. Shiny requires Gems.
3. Optionally specify account Gold/Gem reserves and global/per-rule spending, purchase-count or quantity limits. A zero limit means *unlimited*, not zero spend.
4. Enable **native market buying** and click **Save market rules**. The engine is disabled by default. Pausing or disabling only affects this market buyer; other native automation stays active.

The per-account order journal is saved as `market-orders-GameN.json` under Moth's app-data directory. The configuration is `market-rules-GameN.json`. Only rules and transaction exposure are stored; authentication credentials are not copied into these files.

## Importing existing Moth Watch settings

From a Firefox profile where browser Moth Watch was configured, copy the JSON value returned by:

```js
localStorage.getItem('moth-pokeidle-market-bot-config-v1')
```

Paste it into **Import existing Moth Watch rules**. The imported settings preserve available rule types, prices and limits, but leave the global native buyer **OFF** until reviewed and saved. Existing browser scripts are not modified, and their journal is not imported.

**Avoid running the browser Moth Watch buyer and native buyer at the same time for the same account**. Two independent spending engines cannot coordinate transaction exposure.

## Market request and confirmation model

The authenticated native game WebSocket issues `market.itens` summaries (only for broad item rules), `market.item` exact offer queries, and `market.listar` Pokémon pages. Quotes are rejected when stale, held, missing required identity/price data, or offered by the account itself. Only actual item/Pokémon listing replies authorize `market.comprar`, never a summary minimum or historical average. For a qualified item listing, the buyer requests the *maximum quantity available and affordable* in one purchase.

**Moth Watch aggressive behavior (native .27):** up to five in-flight purchase attempts per account and three per item listing, 120 ms nominal spacing, 400 ms item retry gap and a five-second timeout. The native game socket uses a 60 ms blocking read timeout so commands are not artificially serialized by a 250 ms reader wait. Every item order requests the full *available, affordable* listing quantity; the NPC-only 5,000 quantity ceiling is not imposed on Community Market buys. Held listings are not ordered until the server-reported release, and price/currency/scope changes invalidate their quotes.

Matching `marketComprado` receipts are reconciled by listing ID (including nested ID aliases), or by unambiguous currency/total evidence where the server omits an ID. Conflicting or ambiguous acknowledgements do not confirm an unrelated order. Explicit listing-specific refusals clear that order; five-second timeouts remain **unverified** and reserve quoted exposure but do **not** prevent fresh-quote item retries. Pokémon are not retried after ambiguous outcomes to avoid duplicated unique purchases. Matching receipts trigger targeted item-list refresh, which enables another maximum-quantity purchase on a partially filled lot without relying on a stale quantity.

Market scanning uses the shared authenticated WebSocket and retains two-second background request separation for ordinary rules. With enabled Pokémon buying, request separation is 1.25 seconds, cheapest-first Pokémon page zero is refreshed after six seconds, and deeper pages are checked less frequently (40 seconds) without delaying the front page. Matched nonempty Pokémon replies with row-verified scope can be processed even when the server omits redundant top-level discriminators; ambiguous empty pages are never treated as valid. Purchases from newly received valid quotes are dispatched in the read path without an extra 60 ms worker wake. One outstanding market scan and the fifteen-second server rate-limit backoff remain in force; after a rate-limit error, scan spacing falls back to three seconds for three minutes. These are client pacing settings, not verified server throughput guarantees. Purchase history is compacted into exact per-rule, currency, UTC-day, and session totals, so high-volume trading does not permanently stop at the original 20,000-entry journal ceiling.

### Upstream comparison

The original [DiuleJR/pokeidle-manager](https://github.com/DiuleJR/pokeidle-manager/blob/main/src-tauri/src/market.rs) confirms market listing state, explicit type/item/currency scope, reserve-aware lot quantity, and protects ambiguous purchases with a transaction quarantine. These correctness checks are retained **when those optional listing fields are present**. Its per-account purchase serialization, 15-second confirmation wait, and 30-second uncertain-outcome quarantine are **not** copied into rule-buy dispatch. The more aggressive [Moth Watch](https://github.com/MOTHblank/pokeidle-addons-premium/tree/main/addons/moth-watch) semantics remain authoritative for pacing and same-listing retries.

**Verification:** Unit tests check quote identity, scope/state rejection, reserve-aware full-lot quantities, multi-order spending reservations and journal compaction. These do not replace live authenticated market tests. Order outcomes remain distinguishable from mere send success.

## Mission automation status

The game-confirmed server automation keys currently managed by the controller cover potions, revives, auto selling, Pokémon locking and hunt return. A mission/quest write-and-acknowledge command has not been verified. The **Mission automation · observed server fields** section is intentionally **read-only**: it exposes mission-like fields *only if the authenticated server snapshot actually supplies them*. No guessed mission commands are sent. Live protocol evidence is required before enabling mission mutations.

## Native Moth Watch Snatch (0.1.7-native.28)

**Snatch is a separate, high-priority market path**, not merely normal rule buying with shorter delays. The Automation → Market rule buying → **Snatch** section saves an account-specific queue of up to **40 individual listing targets** alongside the market rules. A player can arm a known listing by its ID, type, currency, item ID (for an item lot) and **authorized maximum unit price**, or click **Snatch** on a held offer already observed by the native scanner. Queue edits are saved immediately. Global **Enable native market buying** must be on, and Pause purchases must be off; the manual queue does *not* require any normal buying rule.

The buyer uses `market.anuncio {t,id}` to recover and revalidate an individual listing (including after reconnect). A listing must provide actual open state, real seller, positive price, stock, exact ID/currency/type/item scope and match the player's maximum price. A sold, vanished, own or conflicting listing is not bought and its status is displayed as an error; it can be cancelled and re-armed after inspection. A manually entered listing is *never bought solely from user-supplied price/ID* without a server quote.

Held listings matching enabled native item/Pokémon rules become **automatic Snatch targets** when encountered in authenticated `market.item` or `market.listar` replies. Before release, the engine retains the server-dated quote, enters a **45-second preparation window** for exact-ID verification, and uses the server time offset from the authenticated state. During the **four seconds after release**, Snatch has priority over normal rule buys/scans. It makes each eligible listing's **first attempt before giving the same listing more attempts**; up to **eight** authorized `market.comprar` sends can be committed per worker wake, with a nominal **150 ms per-listing retry**. This path **does not use** the normal rule buyer's five-pending or three-per-listing throttle. No additional lottery entries are claimed—the game server controls the outcome.

After the four-second burst, further purchases require a **fresh, exact listing verification** (within 15 seconds), rather than inferring that a listing still exists from truncated page scans. An item listing stays eligible for more **maximum-affordable-quantity** sends while verified; a confirmed unique Pokémon cannot be retried. Server price/currency and seller identity are rechecked; global/reserve/per-rule/day/session spending caps and the order journal still apply. Repeated attempts on the *same* competitive listing reuse that listing's balance reservation while independent listings reserve separately; all sends are journaled as exposure until authoritatively settled. The full burst journal is written atomically before dispatch instead of synchronously rewriting the file for each send.

This is a **source-level native port**, not a claim of live-market or Rust-build verification. Synthetic tests cover manual authorization, rule-held observation and exact-ID positive/negative lookup. Live authenticated timing, server rate feedback and idless `marketComprado` outcome correlation still need actual-game validation.
