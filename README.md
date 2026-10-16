# Moth Controller — PokéIdle

Native-first Rust controller for up to four PokéIdle accounts on Windows and Linux. The primary runtime uses native authenticated PokéIdle WebSocket sessions and Twitch IRC; **Firefox is used only when login/authentication needs it or when deliberately opened for interactive play**.

**Não é necessário instalar Violentmonkey nem userscripts para usar as sessões nativas, Auto Catch ou restock.** Cada conta pode fazer login no Firefox uma vez e continuar no controlador sem manter abas abertas.

**Downloads:** [latest release](https://github.com/MOTHblank/pokeidle-addons/releases/latest)

## Getting started / Como começar

1. Download Moth Controller from [Releases](https://github.com/MOTHblank/pokeidle-addons/releases/latest), on Windows or Linux.
2. Add an account and select **Start native**. If authentication is missing or expired, the controller opens its isolated Firefox profile and captures a fresh session through Firefox BiDi.
3. Sign in to PokéIdle (and Twitch if desired) in that profile. The controller hands off to native WebSocket/IRC sessions and closes temporary authentication Firefox when verification succeeds.
4. Use **Open in Firefox** for an optional interactive browser session. Native Auto Catch, restocking, Hunt Atlas projections and history operate independently of userscripts.

For build and configuration details see [controller/README.md](controller/README.md). For native architecture, limitations and verification notes see [native session documentation](controller/docs/native-background-experiment.md). For optional cached item sprites see [native visual assets](controller/assets/visual/README.md).

## Occasional browser userscripts

All former public browser scripts, including the experimental native-branch Bridge and Twitch improvements, are maintained in **[pokeidle-addons-premium](https://github.com/MOTHblank/pokeidle-addons-premium)**. The optional catalog includes Auto Catch+, Controller Bridge, Hunt Atlas, Performance+, Live Stream Scanner and Twitch Low Resource, alongside Moth Watch and the upstream scraper.

These scripts are **not required** to run Moth Controller. If you still want browser scripts, install them from the Premium repository, one per intended feature, and avoid overlapping native and browser Auto Catch. Existing Violentmonkey installations pointing at the retired `pokeidle-addons/master/addons/*.user.js` URLs need to be reinstalled from Premium once to receive future updates.

## Development

```sh
cd controller
cargo test
cargo check
cargo build --release
```

The native UI is implemented in Rust using egui/eframe. No WebView, Tauri or userscript engine is needed. Automated builds for the controller are in `.github/workflows/`.

Issues and feature requests: [GitHub Issues](https://github.com/MOTHblank/pokeidle-addons/issues).
