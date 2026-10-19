# Native Auto Catch — protocol, 600-ms dispatch and stock accounting

## Transport and targets

The native controller sends `ball.throw {t, ballId, slot}` through each account's
authenticated game WebSocket. It does not need the browser, DOM or userscript.
Wild `batalha.ev[k="morte", quem="selvagem"]` slots are tracked for up to
30 seconds; a subsequent hunt/center transition or a reconnect clears old
slots. A full `estado.caidos` list may repair a death event with no slot.
Slot 0 is valid and Master Balls are never automated.

The shop's descriptive `catalogoBolas` is optional. Ball selection uses
server `estado.balls` inventory and stable game IDs (Poké 1, Great 2,
Super 3, Ultra 4, Beast 5). Ultra Ball is the default capture and restock
selection; existing explicit alternatives remain available.

## Cadence and AoE ordering

The native interval is **600–3000 ms**, default **600 ms**. Older native
configurations carrying the former 1200-ms default are migrated once;
newly saved explicit 1200-ms settings retain their chosen value. The
separate Auto Catch+ userscript has its own timing.

The worker sends at most **three unanswered throws** concurrently and
never more than one new throw per configured interval. This keeps high-AoE
hunts moving while preventing the old unbounded 100-ms reservation backlog.
Oldest corpses are served first; equally old untouched targets precede
retries. Explicit failure receipts make a target retryable after the
configured interval. A pending throw with no server response for eight
seconds triggers a reconnect of just that account to obtain fresh state,
rather than blindly retrying indefinitely.

VIP or manual `bola` receipts may appear on the same game connection.
They do not reset the **native** dispatch cooldown; otherwise busy VIP
accounts can indefinitely postpone native throws.

## Stock and restock

Inventory is rooted in the authoritative `estado.balls` snapshot; valid
`bola` receipts can reflect consumption before the next snapshot. The
controller no longer maintains a cumulative, historical `unreflected`
debit ledger. Provisional ball holds belong **only** to currently pending
target slots, and are released by a matching receipt, reflected stock
decrease, target expiry/removal, or reconnect.

When native restock is **enabled for the same selected ball ID**, pending
local throws are **not subtracted** from usable ball stock. This prevents
a burst of provisional reservations from stalling a replenishable
10-to-30/50-ball stock. Capture still respects the player's explicit
ball reserve and requires **positive server-observed inventory**: enabling
restock does not literally create infinite balls or guarantee the shop
has accepted an order. The three-pending-throw cap and eight-second
recovery prevent unbounded speculative sends.

When restock is disabled or targets a different ball, live pending throws
are deducted from the available inventory. Replenishment uses confirmed
shop purchases and inventory reconciliation. If a confirmed purchase is
not reflected by authoritative inventory within 20 seconds, the
affected account reconnects without issuing an unsafe duplicate order.

The user can configure restock independently (for example, buy Ultra Balls
when stock reaches 10 and restore 30–50); no fixed 500/1000-ball stock is
required for reliable native capture.

## Statistics and diagnostics

The Automation panel separates:
- Native throws and actual subsequent native-matched capture results
- Observed `batalha.ev[k="bola"]` successes and failures
- Downed target slots, pending unreconciled sends, restock state and recovery count

Observed `bola` successes are not a guaranteed count of all Pokémon
obtained through the game's built-in VIP Auto Catch; VIP automation can
have a different reporting path. A configured worker currently waiting
for inventory, center exit, or allowed stock shows **Aguardando**, not
an unsupported claim that Auto Catch itself is broken.

Each Game1–Game4 native profile retains its own capture/restock settings
through Firefox reauthentication. These counters are per current native
session and require a new desktop build to reflect these changes.

## Verification

Rust regression coverage in `native_gameplay.rs` exercises the 600-ms
two-target cadence, three-pending-throw cap, VIP receipts not delaying
native sends, small restock inventory replenishment, configured reserves,
no phantom stock at zero, reconciliation, and timeout recovery. Settings
tests verify migration and explicit-interval persistence.

Code-level validation is not live server confirmation: no Rust compiler
or authenticated PokéIdle test session was available here.
