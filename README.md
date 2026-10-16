# Moth Controller — PokéIdle

Moth Controller manages up to **four PokéIdle accounts** with separate sign-ins. The desktop application handles PokéIdle and Twitch connections in the background. Firefox opens when you need to sign in or choose **Open in Firefox** for interactive play.

**Downloads:** [Latest release](https://github.com/MOTHblank/pokeidle-addons/releases/latest)

## Getting started

1. Download Moth Controller for Windows or Linux.
2. Open **Accounts**. All four account slots appear as cards; enable and name the ones you use.
3. Click **Start account**. If a saved login is not available, a visible Firefox window opens.
4. Sign in to **PokéIdle** and **Twitch** in that Firefox profile. Each card shows both connections separately. Once confirmed, Moth Controller can continue without keeping Firefox open.
5. Visit **Automation** for Auto Catch, restocking, and market rules, or **Hunt Atlas** for hunt details and comparisons.

## Game icons

Under **Options**, choose **Download game icons** to fetch known Poké Ball and item images into your local cache. Additional artwork, especially Pokémon species sprites, requires a maintained mapping of game image sources. The cache manifest is currently empty; see [asset documentation](controller/assets/visual/README.md).

## Build

From `controller/`, run `cargo build --release`. Windows users whose existing executable is still running can run `.\build-windows.ps1` in PowerShell to build without overwriting that executable.

See [controller instructions](controller/README.md) and [GitHub Issues](https://github.com/MOTHblank/pokeidle-addons/issues).
