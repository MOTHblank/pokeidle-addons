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

**Violentmonkey and all userscripts are optional.** Nothing must be installed to complete native onboarding, and script detection is not part of native game authentication. The Accounts page can optionally open installers for legacy scripts that now live in [pokeidle-addons-premium](https://github.com/MOTHblank/pokeidle-addons-premium). The controller checks their published versions only as an auxiliary diagnostic when that panel is used. If running optional browser Auto Catch, avoid simultaneously running native Auto Catch on the same account.

The old `pokeidle-addons/master/addons/` installer URLs are retired; reinstall any previous user scripts from Premium to change their update feed. Moth Watch, Hunt Atlas browser mode, and advanced browser-only market workflows remain separate optional features.

For configuration and protocol details of the **native Moth Watch rule buyer**, see [market rule buying](docs/native-market-rule-buying.md). It is off by default and can import existing Moth Watch rule JSON. The native Automation page also surfaces any observed mission fields read-only until their write/acknowledgement protocol has been verified.

## Verification

```sh
cd controller
cargo test
cargo check
```

Native connection and recovery need live PokéIdle/Twitch session verification, not only compilation. Do not log or commit authenticated game/Twitch tokens.
