# Native Auto Catch — protocol and timing

## Scope

The native controller uses the already authenticated game WebSocket to select from the server-reported `catalogoBolas` and issue `ball.throw {t, ballId, slot}` against server-reported wild death slots. It does **not** depend on DOM buttons, Firefox, a userscript, the Twitch socket, or a guess at game inventory.

## Cadence and ordering (0.1.7-native.29)

- Configurable throw interval: 100–3000 ms, default 100 ms, imported from Auto Catch+ `intervalMs`. Only one throw is issued per native worker cycle; actual pacing also depends on the shared game socket.
- For each `batalha.ev` `morte` with `quem = selvagem`, the slot is tracked for at most 30 seconds. When multiple corpses are eligible, the oldest observed death is served first (earliest estimated expiration). A reused slot is a new corpse lifecycle.
- A server `bola` success closes that slot; a failed result releases it for an immediate *configured-interval* retry. An unanswered send remains pending until 1,000 ms have elapsed, then the same slot may be retried. A later failure must not reopen an already captured/removed slot.
- A subsequent hunt/center transition and a disconnected/reconnected session invalidate all tracked corpses. `next_capture` checks expiry even without incoming battle frames, preventing stale sends.
- Preferred ball is used until server stock *minus locally outstanding throw reservations* would reach the player's configured reserve; fallback ball is next. Master Ball is excluded. Actual stock snapshots reconcile the local reservations only after the server reflects their consumption.

## Accounting

Native sends count on actual dispatch rather than on all received `bola` events. Matched successes, explicit failure receipts and timeout retries are separate counters. Per-ball attempts and successes are shown in **Automation → Auto Catch → ball statistics**. The dashboard's total captures can include other game actions and must not be mistaken for native Auto Catch successes.

## Capture readiness and recovery

The UI no longer equates an enabled Auto Catch preference with a worker that can throw. Each account reports an actionable blocked reason when it is at center, has no game ball catalog, has no selected ball above its reserve, or the latest wild death has no server capture slot. A separate warning detects the common mismatch between restocking Poké Balls and configuring Auto Catch to throw Beast/Ultra Balls. Captures still require actual server-identified downed slots; no slot is invented.

Slot 0 is accepted as a legitimate server identifier. When the server includes a complete `estado.caidos` list, the worker can recover targets even if an earlier `morte` event omitted its slot. A fresh increased server stock supersedes old provisional local ball debits, avoiding indefinite capture stalls following replenishment while unchanged stale stock still protects configured reserves.

## Verification

The Rust tests in `native_gameplay.rs` cover fast two-target scheduling, 1-second unanswered retries, explicit-failure retries, strict reserve behavior during stale stock patches, deduplicated/matched results and expiry/reconnect. `native_capture_config.rs` covers importing/saving interval settings and bounds. Compile and live game verification are required before claiming production-tested parity; neither was executable in the editing environment.
