# Moth Controller (Rust)

The default application is a **native-first** PokéIdle and Twitch client with separate credentials/sessions for up to four accounts. Uses `egui/eframe`, native PokéIdle WebSocket and Twitch IRC, and Firefox WebDriver BiDi only for browser login, authentication handoff and interactive Firefox sessions.

## Run / Build

- Download a Windows or Linux binary from [Releases](https://github.com/MOTHblank/pokeidle-addons/releases/latest), or build with `cargo build --release`.
- Run `cargo test` and `cargo check` from `controller/`.
- Firefox or Firefox Developer Edition is needed only for initial authentication, expired sessions and **Open in Firefox**. Set `MOTH_FIREFOX` to its path if not detected. Linux interactive browser sessions need X11 or Wayland, and GUI/system dependencies installed.
- Use **Accounts** to register accounts, **Start native** to connect and **Open in Firefox** for an interactive session. Credentials are cached in the existing protected OS store.

## Native functionality

The Rust backend handles authenticated game sessions and Twitch IRC directly, Auto Catch ball selection and restock (including potions and revives), item catalogs, server automation settings, hunt metadata and Hunt Atlas projections, account dashboards, and persistent history. The options page includes independent focus/tray refresh throttling. See [native transport and automation notes](docs/native-background-experiment.md), [Hunt Atlas notes](docs/native-hunts-atlas.md) and [native visual cache](assets/visual/README.md) for implementation details and known limitations.

## Optional Firefox browser extensions

**Violentmonkey and all userscripts are optional.** Nothing must be installed to complete native onboarding. In **Accounts**, enable a slot and choose **Start native**; if necessary, authenticate in its temporary Firefox profile. The setup guide completes automatically after an authenticated session; there is no script-installation step. Connection state and login tools remain visible, while special Google/Twitch sign-in options are under **Login and profile tools**.

For occasional Firefox scripts only, expand **Optional browser scripts (advanced)**. Legacy installers point to [pokeidle-addons-premium](https://github.com/MOTHblank/pokeidle-addons-premium). Their versions are checked only when that optional panel is opened or its **Check now** button is pressed; missing scripts do not trigger dashboard warnings or get selected for installation automatically. Scripts must be chosen individually; there is no install-all action. If running optional browser Auto Catch, avoid simultaneously running native Auto Catch on the same account.

The old `pokeidle-addons/master/addons/` installer URLs are retired; reinstall any previous user scripts from Premium to change their update feed. Moth Watch's native rule buyer now includes an account-local Snatch queue, automatic held-listing observation, exact listing verification and high-priority release bursts; the browser-only full market browser and historical valuations remain optional features.

For configuration and protocol details of the **native Moth Watch rule buyer**, see [market rule buying](docs/native-market-rule-buying.md). It is off by default and can import existing Moth Watch rule JSON. The native Automation page also surfaces any observed mission fields read-only until their write/acknowledgement protocol has been verified.

## Native Auto Catch fidelity

For its runtime mechanics, capture receipt semantics, reserve accounting, and regression coverage, see [native Auto Catch notes](docs/native-autocatch.md).


Moth Controller uses a separate native `ball.throw` engine; the premium Auto Catch+ userscript is optional, and running both at once on one account is not recommended. Native automation imports the user's normal `intervalMs`, two-ball priority and per-ball reserves from the protected account capture settings.

As of `0.1.7-native.29`, the capture editor supports **100–3000 ms configurable throw spacing** (100 ms default like Auto Catch+). The oldest eligible corpse receives priority, using its server-observed death time and the game's 30-second lifetime as the available deadline signal. An unanswered throw can be retried after **1 second**; a server-confirmed failure may retry at the configured interval. A successful `batalha.ev` `bola` receipt closes that slot, and duplicate/late failures cannot reopen a captured corpse. A slot is discarded at expiration, on a hunt/center transition or disconnect, without guessing that it remains available.

Every outbound native throw reserves one selected ball *immediately*, even when server `estado.balls` snapshots lag, so the configured reserve is not accidentally spent during rapid throws. New authenticated stock observations reconcile previously reserved uses when stock reductions are reflected. Socket send failures roll back local debits; reconnect clears all stale corpse/inventory state before starting new throws. Master Balls remain excluded and numeric ball IDs are resolved from the authenticated game ball catalog, never guessed from stale website assets.

The Automation page shows distinct **native sends, matched captures, explicit failures, one-second timeout retries and per-ball capture usage**. Unrelated manual/server-auto catches remain part of the dashboard totals, not the native Auto Catch success count.

These are source-level fixes with new Rust regression tests for deadline priority, fast cadence, unanswered/failed retries, stock reservations, duplicate receipts and disconnect/expiration. This environment lacks `cargo`/`rustc` and a live authenticated game session; compilation and in-game behavior **are not yet verified**.

## Verification

```sh
cd controller
cargo test
cargo check
```

Native connection and recovery need live PokéIdle/Twitch session verification, not only compilation. Do not log or commit authenticated game/Twitch tokens.
