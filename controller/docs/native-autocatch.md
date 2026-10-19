# Native Auto Catch — protocol and timing

## Scope

The native controller uses the already authenticated game WebSocket to select from the server-reported `catalogoBolas` and issue `ball.throw {t, ballId, slot}` against server-reported wild death slots. It does **not** depend on DOM buttons, Firefox, a userscript, the Twitch socket, or a guess at game inventory.

## Cadence and ordering (0.1.7-native.29)

- Configurable native throw interval: 1200–3000 ms, default 1200 ms. Legacy imported intervals below 1200 ms are upgraded on load. The reference Manager also uses a 1.2-second ball cooldown. Only one native throw can be outstanding at once; actual pacing also depends on the shared game socket.
- For each `batalha.ev` `morte` with `quem = selvagem`, the slot is tracked for at most 30 seconds. When multiple corpses are eligible, the oldest observed death is served first (earliest estimated expiration). A reused slot is a new corpse lifecycle.
- A server `bola` success closes that slot; a failed result releases it for an immediate *configured-interval* retry. An unanswered send remains pending until 1,000 ms have elapsed, then the same slot may be retried. A later failure must not reopen an already captured/removed slot.
- A subsequent hunt/center transition and a disconnected/reconnected session invalidate all tracked corpses. `next_capture` checks expiry even without incoming battle frames, preventing stale sends.
- Preferred ball is used until server stock *minus locally outstanding throw reservations* would reach the player's configured reserve; fallback ball is next. Master Ball is excluded. Actual stock snapshots reconcile the local reservations only after the server reflects their consumption.

## Accounting

Native sends count on actual dispatch rather than on all received `bola` events. Matched successes, explicit failure receipts and timeout retries are separate counters. Per-ball attempts and successes are shown in **Automation → Auto Catch → ball statistics**. The dashboard's total captures can include other game actions and must not be mistaken for native Auto Catch successes.

## Capture readiness and recovery

The UI no longer equates an enabled Auto Catch preference with a worker that can throw. Each account reports a waiting reason without equating lack of an immediate eligible throw to failure when it is at center, has no game ball catalog, has no selected ball above its reserve, or the latest wild death has no server capture slot. A separate warning detects the common mismatch between restocking Poké Balls and configuring Auto Catch to throw Beast/Ultra Balls. Captures still require actual server-identified downed slots; no slot is invented.

Slot 0 is accepted as a legitimate server identifier. When the server includes a complete `estado.caidos` list, the worker can recover targets even if an earlier `morte` event omitted its slot. A fresh increased server stock supersedes old provisional local ball debits, avoiding indefinite capture stalls following replenishment while unchanged stale stock still protects configured reserves.

## Ultra Ball defaults and account isolation

Fresh native Auto Catch settings now select only Ultra Ball (catalog ID 4)
and native restock also selects Ultra Ball (ID 4). Explicit saved choices
remain unchanged. The optional Auto Catch+ userscript uses the same defaults.

Each account uses its own Game1–Game4 credentials and settings vault entry;
Game 4 is disabled in the account launcher by default, but has the same native
capture algorithm once enabled. When Firefox reauthentication refreshes an
existing native session, the controller preserves that profile's stored native
Auto Catch/restock preferences instead of replacing them with older browser
localStorage values. The merged configuration is used immediately and saved
for subsequent native launches. First-time accounts still import browser
settings as before.

If one account stops capturing, compare its selected ball stock and reserve,
Auto Catch blocked reason, restock phase, and death-event slot diagnostics.
An account-specific failure cannot be definitively attributed to a root cause
without its authenticated runtime observations.

## Server captures, account-specific stalls, and restock recovery

The controller now separates native-attributed catches from **server-confirmed
capture receipts**. The Automation counters and per-ball receipt totals report
the latter, even when an event lacks a slot or the original corpse has already
expired. A missing-slot receipt is attributed to the native throw only when
exactly one compatible pending throw exists; ambiguous receipts are counted as
server confirmations without falsely assigning them to a target. This also
avoids displaying a misleading zero captures after successful catches.

The account detail shows the selected restock ball's stock and target, the
restock phase, outstanding unreconciled sends, and an explicit readiness reason
when the queue keeps changing but no new throw can be sent. The numbers are
per native session and require rebuilding/rerunning the controller.

A confirmed `bola` receipt updates local restock ball consumption ahead of
the next `estado.balls` snapshot. Native-matched receipts also reconcile their
provisional ball reservations. A confirmed shop purchase must still wait for an
authoritative inventory snapshot before another purchase. If no such snapshot
arrives within 20 seconds, only that account's authenticated game socket
reconnects to obtain a fresh server state rather than indefinitely blocking
restock or blindly making duplicate purchases.

## Manager parity and receipt limitations

The desktop Auto Catch worker now follows the reference
`DiuleJR/pokeidle-manager`'s conservative 1.2-second send cadence.
It also allows only **one unanswered native `ball.throw` at a time**.
Previously the 100-ms loop could reserve many unacknowledged Ultra Balls
while waiting for server results, exhausting its local usable stock and
making an account appear frozen even as new wild deaths entered its queue.

The shop's descriptive `catalogoBolas` is optional. The worker uses
stable PokéIdle inventory ball IDs (Poké 1, Great 2, Super 3, Ultra 4,
Beast 5), the selected priority and authoritative `estado.balls` counts.
Master Balls are excluded. A received empty inventory is distinguished
from an inventory snapshot that has not yet arrived.

If an already-sent ball never receives a result for eight seconds,
the affected native game socket reconnects for fresh server state
instead of sending blind retries. The Automation view shows the
per-account recovery count and any capture-related server notice.

The capture statistic labeled **Ball-event captures** counts observed
`batalha.ev[k="bola"]` success receipts. It does **not** claim to count
every Pokémon obtained by the game's built-in VIP capture automation,
which may not emit those events. Native-attributed catches are a
separate subset. An enabled preference with no currently eligible
throw shows *Waiting*, not *Blocked*; waiting reasons still identify
zero stock, missing inventory, center, and pending reservations.

## Verification

The Rust tests in `native_gameplay.rs` cover fast two-target scheduling, 1-second unanswered retries, explicit-failure retries, strict reserve behavior during stale stock patches, deduplicated/matched results and expiry/reconnect. `native_capture_config.rs` covers importing/saving interval settings and bounds. Compile and live game verification are required before claiming production-tested parity; neither was executable in the editing environment.
