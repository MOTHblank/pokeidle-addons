# Native-first Moth Controller: PokéIdle + Twitch IRC

**Branch:** [`experiment/native-background-sessions`](https://github.com/MOTHblank/pokeidle-addons/tree/experiment/native-background-sessions)  
**Controller:** `0.1.7-native.2`  
**Experimental browser helpers:** Controller Bridge `1.3.14`, Twitch Low Resource `3.2.1`, Stream Scanner `7.2.1`.

This is an **architecture-replacement branch**, not a separate optional plugin or an opt-in feature. The normal target runtime is **native Rust game WebSocket + native Rust Twitch IRC**. Firefox is a temporary authentication/bootstrap and recovery mechanism only. The old production `master` remains unchanged.

## Default lifecycle

1. Launch an enabled account. If credentials are not already available in memory, Moth opens its isolated Firefox profile (headless by default) for PokéIdle login and its existing authenticated Twitch session. Twitch sign-in still happens in Firefox if required.
2. The existing stream scanner reads `twitch.lives` and `twitch.assistindoEm` from the game; it starts one managed Twitch chat popout. Twitch Low Resource retains the current browser-authenticated IRC socket and its `PASS oauth:…`, `NICK`, and `CAP REQ` messages in memory only.
3. When the game bridge and stream scanner are ready, the controller **automatically** requests both game + Twitch handshakes over its existing Firefox BiDi session. It validates both. Missing Twitch authorization means Firefox stays open and handoff is retried; the controller never fabricates an OAuth token.
4. With both credentials captured, Moth closes the controlled Firefox instance, starts both Rust TLS WebSockets, and waits for server-authenticated `welcome` (PokéIdle) and IRC `001` (Twitch). The game and Twitch sockets have separate Rust workers and reconnect loops.
5. Rust owns Twitch `JOIN` / `PART` reconciliation directly. The desired channels come from the native game stream (`twitch.lives` union rewarded `twitch.assistindoEm`). Identical channels remain joined without socket or tab reloads. Joins are paced at **600 ms per channel**, with a **100-room limit**.
6. On persistent connection or authentication failure, the controller drops native workers and reopens that account's Firefox profile for recovery. The normal objective remains a **zero-browser** runtime.

**No manual `Try native WS` button exists.** The dashboard reports the native game and IRC phases, IRC joined/desired room count, game frames, battle events and captures.

## Installation and development

The experimental controller installs and checks its userscripts from this **experimental branch**, not from `master`. This includes branch-specific `@updateURL` / `@downloadURL` for the three modified scripts.

1. Check out the branch.
2. Build and start using `cd controller && cargo run`. Because this experiment enables Rust TLS support on an existing `tungstenite` dependency, **`Cargo.lock` needs regeneration and committing** on a machine with Cargo and crate registry access before using `--locked`. The isolated branch CI runs `cargo test` and `cargo check` (without `--locked`).
3. Ensure Controller Bridge `1.3.14`, Twitch Low Resource `3.2.1` and Stream Scanner `7.2.1` are installed in the target profile. Use **Manage Addons → Install all** for the branch versions and reload game/Twitch pages.
4. Log into PokéIdle and Twitch using the same isolated Firefox profile as before.
5. Watch the account dashboard: Firefox is the **authentication stage**, not the intended runtime. Once its game WebSocket and authenticated IRC socket have both been observed, Moth performs native handoff automatically.

## Scope and constraints

This branch implements the **transport and chat participation**, not all features previously implemented in page userscripts. Auto Catch+, Moth Watch, Snatch, Hunt Atlas UI and any other DOM-dependent userscript **do not run with Firefox closed**. A native game socket can receive battles and XP but is **not equivalent to every browser-side automation**. This is a real limitation, not a reason to run a hidden browser indefinitely.

The user's established method of chat-only IRC bonus participation is preserved, not replaced by video playback. **Twitch videos are never opened or played by the native IRC worker.** Server-reported bonus data is exposed on the native dashboard.

The current Twitch handoff requires the browser's real `PASS oauth:…` handshake from a managed authenticated chat popout. If Twitch changes its browser auth format, Moth will not silently forge credentials or downgrade to anonymous IRC. Credentials are retained in process memory for reconnects; no new credential files, token logs or storage are introduced.

This environment had no Rust toolchain, package downloads or authenticated live game session. Accordingly, **the new branch is not compile-tested, session-tested, or production-ready** here. The native game protocol's progress and reward correctness require live comparison before leaving the experiment branch.

## Test checklist

- [ ] Windows and Linux: `cargo test` and `cargo check` compile and pass; regenerate `Cargo.lock`.
- [ ] Both Firefox handshakes are observed after login, without a second BiDi session.
- [ ] Automatic handoff closes Firefox and both Rust connections receive authenticated welcome frames.
- [ ] Game battle/XP and native Twitch IRC joins continue after Firefox closes.
- [ ] JOIN/PART updates preserve unchanged channels and reward-active channels.
- [ ] Twitch chat-only XP bonus stays game-confirmed, as with the existing browser chat method.
- [ ] Credential expiration and TCP disconnections reopen Firefox only when recovery is necessary.
- [ ] All four account profiles remain isolated.
- [ ] Native replacements for any remaining essential browser automations work before this branch becomes a release.
