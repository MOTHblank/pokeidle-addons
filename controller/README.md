# Moth Controller

Moth Controller is the native desktop application for [PokéIdle Addons](../README.md). It manages up to four independently authenticated game profiles and native Twitch connections.

**[Latest release](https://github.com/MOTHblank/pokeidle-addons/releases/latest)**

## Using the app

Set up profiles in **Accounts**, signing in to PokéIdle and Twitch when the app requests it. The account cards show connection status and session controls. Firefox or Firefox Developer Edition is used for authentication when needed; use **Open in Firefox** whenever you want to play interactively.

Once authenticated, the game and Twitch connections operate in the background without requiring an open browser window. No userscript manager or browser add-on is required.

The **Dashboard** displays account activity and details; **Hunt Atlas** compares hunts and capture outcomes; **Automation** contains Auto Catch, restocking, and market rules. Account-specific options are managed inside the app.

## Artwork

Item icons and Pokémon sprites are loaded lazily from supported public game assets and cached on disk. There is **no manual “Download game icons” step or button**. Missing images do not prevent the controller from functioning.

See [native visual asset notes](assets/visual/README.md) for cache behavior and developer tooling.

## Building

From this directory:

```sh
cargo build --release
```

On Windows, if an older executable is still running, the optional `build-windows.ps1` script builds into a separate output directory instead of overwriting the active file.

For development, use `cargo check` and `cargo test` from `controller/`. Do not commit credentials or authentication data.
