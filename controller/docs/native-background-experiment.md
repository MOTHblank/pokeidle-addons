# Experimental browserless PokéIdle transport

Branch: `experiment/native-background-sessions` (not merged into `master`).

This experiment independently implements a native, TLS-protected game WebSocket connection in Rust. It **does not implement the full PokéIdle Manager feature set**. The goal of the first stage is to verify a browserless authenticated transport and real server state updates on one disposable/test account.

## Preparation

1. Check out this experimental branch locally. Build from the source with `cd controller && cargo run`. A first build without `--locked` refreshes `Cargo.lock` to include the TLS crates introduced by the experiment. Rust is not available in the current review environment, so this branch has **not been compiler- or live-session-validated**.
2. In the test account's isolated Firefox profile, install the **experimental** Controller Bridge 1.3.14 manually using Violentmonkey:
   [Controller Bridge experimental installer](https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/experiment/native-background-sessions/addons/controller-bridge.user.js).
   Don't replace other users' production Bridge installations, or expect the public `master` update URL to deliver this experimental version.
3. Start that account's Firefox through Moth Controller, authenticate normally, then **reload the game tab** so the bridge observes the opening WebSocket `hello`.
4. Once the account is connected, click **Testar WS nativo / Try native WS** on that account's dashboard card.

The experimental command captures the game-confirmed WebSocket URL/hello and Firefox cookie values through the **existing BiDi session**, without creating a second BiDi session. It validates a `wss://` PokéIdle hostname, closes the controlled Firefox, then connects Rust directly to the game. A real `welcome` is required before the native UI becomes Online.

If the first native handshake fails, the controller automatically attempts to reopen that profile's Firefox using its previous visible/headless preference. During subsequent network interruptions the Rust worker retries the native socket; after repeated failures the controller tries Firefox fallback. To return to Firefox voluntarily, **Stop** the native session and press **Launch**.

## What works in this prototype

- A separate native WebSocket per opted-in account, independent of a rendered Firefox process after successful handoff
- Authenticated `hello` / `welcome` and ongoing JSON `estado` observation
- Native RFC WebSocket ping/pong, freshness tracking and basic reconnect attempts
- Lightweight trainer level, total XP, gold, gems and hunt-slug projection where supplied by server patches
- No credentials written to disk by the experimental handoff; no raw WebSocket messages printed in application logs
- The original Firefox launch/stop behavior remains the default for all other accounts

## Important restrictions

**No browser userscripts run in native mode.** In particular, this first stage **does not provide** Auto Catch+, restocking, Moth Watch, Snatch, Hunt Atlas UI, Twitch chat joining, or browser-side action commands. Gameplay and XP progress **must be checked empirically**; an open native socket does not prove that all gameplay continues correctly without the client. Do not test with high-value market purchases, limited items or a critical farming session.

Firefox WebDriver BiDi must support `storage.getCookies` for this approach; cookie and handshake behavior has not yet been validated against a live game instance. If authentication relies on different headers or token rotation, native login can fail or later expire.

This branch has not been built or end-to-end tested in the current environment. Treat it as a transport prototype, not a replacement for Firefox or a production release.

## Next validation milestones

1. Compile and run Rust unit tests on Windows and Linux, and regenerate/commit Cargo.lock
2. Capture whether the server accepts Rust `hello` after Firefox closes and whether it keeps delivering meaningful state
3. Compare XP, battle, capture and inventory changes against a browser session
4. Implement confirmed native automation and browserless account lifecycle only after validating live protocol behavior
5. Verify long-duration reconnects and logins before considering a merge

Source references: `controller/src/native_background.rs`, `controller/src/monitor.rs` and `addons/controller-bridge.user.js`.
