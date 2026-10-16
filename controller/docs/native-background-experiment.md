# Native-first Moth Controller — four-account game + Twitch IRC

**Branch:** [experiment/native-background-sessions](https://github.com/MOTHblank/pokeidle-addons/tree/experiment/native-background-sessions)  
**Preview controller:** `0.1.7-native.12`

This experimental branch is **not an opt-in Firefox mode**. The standard runtime is one native Rust PokéIdle WebSocket **plus** one native Rust Twitch IRC connection per account. Firefox is an authentication helper and must close once both handshakes are captured.

## What changed after the four-profile log report

The user observed four Firefox sessions running indefinitely. The log identified two distinct blockers:

- **Main**: a validation bug rejected capability names that include dots (for example `twitch.tv/tags`). Native IRC validation now accepts the real format.
- **Other three accounts**: the controller requested a new `sessionBootstrap()` function supplied only by the experimental Controller Bridge; their older installed scripts lacked it. The controller now injects an in-memory WebSocket handshake observer through the **existing Firefox BiDi session**. It is automatically available in every profile regardless of userscript version. If the observer missed an earlier handshake, the controller reloads the relevant game or Twitch page **once** and captures the next handshake.
- If no Twitch chat popout is present, the controller opens **one temporary Twitch authentication chat** to obtain the existing session's IRC identity. It is closed with the authentication Firefox.
- Temporary BiDi disconnections are not an instruction to restart Firefox.

## Startup lifecycle

1. **Native first:** on Launch, the controller reads protected game+Twitch session credentials for that profile from the current operating-system user vault. A usable cache starts both Rust connections **without launching Firefox at all**.
2. **Browser only when needed:** if no protected cache exists, or authentication has actually been rejected, Firefox is started to authenticate PokéIdle and Twitch.
3. **Automatic handoff:** the controller observes the real game hello plus Twitch IRC authentication in BiDi, with no manual native-mode toggle and no mandatory addon upgrade per profile. It does not write the raw handshake to logs.
4. **Return to Rust:** Firefox closes. Rust starts a direct TLS-protected WebSocket to PokéIdle and Twitch IRC; `welcome` / IRC `001` confirm server-side authentication.
5. **Persist native:** session credentials are held in memory and saved using **Windows DPAPI** under the account-specific data directory or the **Linux Secret Service** keyring (requires installed, unlocked `secret-tool`). There is no plaintext storage fallback. The cache survives restarting Moth Controller.
6. **Keep connections:** network interruptions reconnect directly in Rust. Firefox is requested again only for authentication failures, not merely because a socket or BiDi probe was interrupted.

Twitch room reconciliation uses PokéIdle's `twitch.lives` and `twitch.assistindoEm`, keeps overlapping rooms intact and sends paced `JOIN` / `PART` commands. **No livestream video is opened or played.** The bonus method is the same chat-only method as the production userscripts.

## How to test the branch

1. Build from the updated experimental branch (Windows/Linux). Alternatively, once GitHub Actions successfully builds, download the **branch-only preview executable** from the workflow's artifacts; it is not a production release.
2. Close any older Moth Controller executable before launching the preview. Updating userscripts **alone** does not update the Rust controller or fix native startup.
3. With four accounts selected, choose Launch enabled. On the first launch, Firefox may open to establish the initial sessions. Profiles using older userscripts may reload the game page once to activate the built-in BiDi hook. No manual per-account addon upgrades should be required for handoff capture.
4. Watch each account's native game status and native Twitch IRC status. After handoff, the Firefox processes for that profile should exit and both native connection counters should become online.
5. Stop and relaunch the controller. If the Windows DPAPI cache was successfully saved, both native connections should start directly, without **any Firefox startup**.
6. If Twitch or PokéIdle genuinely requires a new login, authenticate through the opened Firefox profile. The controller should then resume its normal native runtime.

## Limitations / validation

This experimental runtime has its own game socket, command dispatcher, native Auto Catch and configurable stock replenishment, but **does not reproduce the complete Auto Catch+, Moth Watch, Snatch or Hunt Atlas features**. Firefox userscripts themselves stop running when the browser closes. Verify the native behavior and remaining gaps before treating it as a production replacement.

The implementation has been committed and branch-specific build/test checks exist, but there has been **no authenticated end-to-end live run or local Rust compiler validation from this environment**. Rust TLS support also requires regenerating and committing `Cargo.lock` before using `cargo --locked`. Inspect CI build results before installing a preview executable.

To diagnose handoff stalls, share **sanitized logs** showing only the status/error messages. Never share browser cookies, OAuth data, or native credential vault contents.

## Crash diagnostics in 0.1.7-native.6

A reported Windows exit occurred directly after logging `stopping Charizard Firefox PID ...`, without a panic or native WebSocket confirmation. This log **does not establish an exact crash cause**. The previous release profile set `panic = "abort"`, which could terminate the whole GUI when *any* Rust thread panicked, without usable diagnostics. This preview switches to unwind panics, catches native game/Twitch worker panics, and records **sanitized thread and source-file location** in `moth-controller.log`.

The handoff now closes/reaps Firefox on a **separate worker per account** rather than blocking egui's UI thread. The log records distinct milestones:

1. `capturing native game + Twitch IRC session via Firefox BiDi`
2. `validated native game + IRC handshakes; closing Firefox asynchronously`
3. `Firefox exited after authenticated handoff`
4. `starting native game + Twitch IRC workers`
5. `Native PokéIdle WSS received authenticated welcome` and `Native Twitch IRC received authenticated welcome`

If a worker panics, its status becomes **Failed** while the controller should remain open. If a future crash still abruptly ends the log without `Rust panic`, investigate OS-level process termination, native-library faults, or an external restart instead of assuming the WebSocket protocol was rejected. No cookies, OAuth tokens or handshake payloads are written to logs.

Test one account and confirm actual battle XP, captured Pokémon, stock changes and IRC XP bonus before running four. Authenticated WebSockets alone are not proof of equivalent farming; native Auto Catch and replenishment still need a real live comparison to the established addons.

## Manager-style native runtime migration (native.7)

This revision uses the public [DiuleJR/pokeidle-manager](https://github.com/DiuleJR/pokeidle-manager) implementation as an **architectural reference**. The code in this branch was authored for Moth; it is not a verbatim copy of Manager's GPL-3.0-only modules. Any future direct reuse of Manager source requires a distribution/license review.

**Single game socket owner.** Firefox/BiDi closes only the game's browsing context, while keeping the authenticated Twitch page and Firefox process alive. On another thread Rust opens the game WebSocket and IRC, and requires a real PokéIdle `welcome` and IRC `001`. If either cannot authenticate, Moth queues reopening `https://pokeidle.io/app` in the original browser. Only once both native servers have confirmed login does it shut down the complete Firefox profile. Closing the browser occurs outside the UI thread. This prevents the prior close-before-confirm race.

**Native game command transport.** The Rust game WebSocket worker is now the sole sender of client protocol frames in native mode. It has a bounded per-account queue, validates known command shapes, drops unconfirmed pending commands on disconnect and reconnects with fresh welcome/state. The first native controls include center navigation, return to the last confirmed hunt, and native Auto Catch on/off.

**Native Auto Catch.** During initial Firefox authentication, Moth imports `moth-pokeidle-autocatch-v4` localStorage settings and caches them with the protected native credentials. Disabled capture stays disabled. When capture is enabled, confirmed wild deaths create per-slot corpse entries; the native processor selects available balls from the server-provided catalog and inventory, sends `ball.throw`, respects the cooldown and a bounded corpse lifetime, and reconciles server `bola` results. Unknown ball IDs and Master Balls are not silently spent. This is a new protocol implementation and must be live-verified against the established userscript before being considered equivalent.

**Native restock.** When Auto Catch's saved restock configuration is enabled, the native game worker observes authoritative shop catalogs, gold and inventory. It applies the configured ball, potion and revive thresholds and target quantities, honors the gold reserve and maximum purchase quantity, sends one `shop.buy` at a time, and waits for purchase and inventory confirmation before starting another order. Missing server prices prevent buying; an acknowledged purchase cannot be repeated while its inventory update is outstanding.

**Still outside scope of this migration:** fully porting the on-page interfaces and marketplace actions of Moth Watch, Snatch and Hunt Atlas; in-browser Pokémon inspection UIs; every advanced Auto Catch targeting detail. Do not assume those scripts keep running after Firefox closes. The underlying server-side battle automation continues when connected, but battle/XP must be confirmed in a live session.

No Rust compiler or authenticated PokéIdle profile was available in this editing environment. Although protocol/unit tests were committed, **neither Windows compilation nor live four-account native gameplay has been independently verified here**. The test workflow builds the experimental branch and is not a reason to delay branch commits. Do not merge into production until the native implementation demonstrably replaces Firefox for your required actions.

## TLS provider and recovered Firefox profiles (native.8)

Windows logs from native.7 showed panics at `rustls-0.23.46/src/crypto/mod.rs:249` simultaneously in the native game, Twitch and cached-session workers. This is rustls' ambiguous process-default cryptography provider, **not** a rejected game login. The controller now links rustls 0.23 with `ring` and explicitly calls `rustls::crypto::ring::default_provider().install_default()` at the very beginning of `main()`, before launching any TLS workers. Rustls documents this as the correct remedy when transitive dependencies enable more than one provider. The initialization is once per process.

Previous crash recovery could also leave multiple Firefox processes advertising the same dedicated Moth profile. A fresh Launch no longer rejects those with an unconditional `Use Stop before launching` message. It checks for an existing browser **before opening a cached game socket**; if one owns the account, the controller attaches to its existing BiDi endpoint and attempts the normal, server-confirmed native handoff. After game and Twitch are both verified, Firefox cleanup walks all processes with that *exact Moth profile argument*, selecting PIDs deterministically. Explicit Stop also cleans remaining exact-profile processes. Other Firefox profiles are never killed by process-name wildcard.

If a stranded Firefox was launched without a BiDi debug endpoint (for example, a separate Google sign-in session), automatic attachment is not possible. Finish sign-in and close it using that profile's Stop action; do not kill all Firefox processes system-wide.

This revision is committed but was **not compiled or authenticated-session-tested in the editing environment**. Build/test the branch locally, use one account to verify that both native welcome stages succeed, then try all four.


## Stuck verified handoff / TLS panic (native.9)

The user reported a single-account handoff that closed the PokéIdle tab, then panicked at `rustls-0.23.46/src/crypto/mod.rs:249:14` in an unnamed thread. Firefox remained open but no native game connection took over. There were **two separate faults**: the TLS provider ambiguity, and the fact that an uncaught verification-worker panic prevented the controller from receiving any completion event. The UI then stayed indefinitely at `Authenticating in Firefox`.

This build centralizes `native_tls::ensure()`, runs it at startup and **immediately before every PokéIdle/Twitch TLS connection**, and exercises it in a unit test. It also catches panics in both the verification and protected-cache workers, always sends their result to the UI, and shows a per-account native error instead of an indefinite generic login label.

After any attempted game-tab close with failed verification, Firefox's **existing BiDi session** explicitly checks whether a PokéIdle game context exists; if not, it recreates the tab and waits for the BiDi acknowledgement. Close/recovery requests wake the monitor immediately, and timeouts cancel pending mailbox entries. A failed native connection must not silently leave the game absent from Firefox.

The log now emits `Moth native controller v0.1.7-native.9 · verified TLS provider initialized: true` at startup. **Check this exact entry** when testing; older experimental controller binaries can still reproduce the panic. The controller must not log `native game welcome + Twitch IRC 001 confirmed` until both authenticated Rust sockets are real. Check `Native PokéIdle WSS received authenticated welcome`, `Native Twitch IRC received authenticated welcome`, and `Firefox exited after authenticated handoff` for completed migration.

Rebuild the executable with `cargo test && cargo build --release` and launch **`controller/target/release/moth-controller.exe`**. The editing environment still has no Rust compiler or authenticated account, so the fix is committed but not locally compiled or live verified.

## Native restock reliability and userscript retirement (native.11)

This preview's native Auto Catch restocking has been aligned with the Auto Catch+ client protocol. It issues `shop.buy` for configured ball/potion/revive IDs using the game's **authoritative welcome shop catalog**, `estado.gold`, `estado.balls` and `estado.items`, with the saved `goldReserve`, thresholds and target quantities. It never assumes unknown prices are safe. A purchase is acknowledged only by a matching `compra` event (ID/kind/name or exact quantity+cost) or an increase in that specific inventory item's count. The native worker has **one order in flight per account**. It waits for the authoritative stock update after acknowledgement, and uses bounded backoff on missing acknowledgements, lost inventory updates, connection failures or uncertain sends. It does not blindly repeat a potentially accepted order.

Existing Auto Catch preferences are imported once from `moth-pokeidle-autocatch-v4` during Firefox authentication and cached in the OS-protected native session. This revision also handles the old addon defaults when stored preferences omit individual thresholds. **The userscript does not need to execute during native restocking.** Restock on/off can now be changed in the controller under **Account details → Automation**; that change is saved without Firefox. The UI shows pending/confirmed purchase counts, stock/sync status, missing shop catalogs, gold protection and last error. The log records `Native restock: requesting ...` followed by `Native restock: confirmed and reconciled ...` **without logging tokens or cookies**.

Important: completion of a Rust `cargo test` suite is not a replacement for a real shop purchase; this workspace has no Rust compiler or authenticated game session. Verify purchase logs and the corresponding in-game gold/stock deltas with one account. If it remains at **Waiting for shop**, the selected ID does not have an authoritative shop price in the current `welcome` catalog; don't try to force purchases using guessed prices.

**Next userscript-independence milestones, deliberately incremental:** move the complete Auto Catch ball-choice, reserves and restock threshold editor into native controller settings; remove the remaining reliance on localStorage preference migration; replace page-bound Hunt Atlas diagnostics/controls with protocol-derived Rust state; then migrate marketplace and monitoring addons one at a time. Keep browser scripts as temporary authentication/migration helpers only until their behavior is covered natively. Never remove a working production userscript merely because an unverified native equivalent exists.
