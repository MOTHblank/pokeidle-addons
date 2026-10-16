# Native-first Moth Controller — four-account game + Twitch IRC

**Branch:** [experiment/native-background-sessions](https://github.com/MOTHblank/pokeidle-addons/tree/experiment/native-background-sessions)  
**Preview controller:** `0.1.7-native.4`

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

This is a native protocol experiment, **not a full port of Auto Catch+, Moth Watch, Snatch or the Hunt Atlas UI**. DOM userscripts stop running once Firefox closes; essential gameplay automation still needs native equivalents.

The implementation has been committed and branch-specific build/test checks exist, but there has been **no authenticated end-to-end live run or local Rust compiler validation from this environment**. Rust TLS support also requires regenerating and committing `Cargo.lock` before using `cargo --locked`. Inspect CI build results before installing a preview executable.

To diagnose handoff stalls, share **sanitized logs** showing only the status/error messages. Never share browser cookies, OAuth data, or native credential vault contents.
