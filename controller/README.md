# Moth Controller

Native Rust Windows controller for dedicated PokéIdle Firefox profiles.

## Run

The controller is intended to be a normal Windows application. A release build produces:

    moth-controller.exe

Double-clicking it opens the controller window and automatically launches the two dedicated game profiles.

## Current UI

- **Game 1** — launch/relaunch AccountA.
- **Game 2** — launch/relaunch AccountB.
- **Launch Both** — launch/relaunch both profiles.
- **Open Profiles** — open the profile data directory in Explorer.
- **Close** — close the controller without closing Firefox.

Firefox profile data lives under:

    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\

## Architecture

The controller does not embed a browser, use WebView2, or implement a userscript engine.

PokéIdle runs in normal Firefox processes. An existing userscript manager such as Violentmonkey or Tampermonkey will own the repository's ordinary `*.user.js` files.

The controller is responsible for desktop/process/profile orchestration only.

## Development

From this directory:

    cargo build

For an optimized build:

    cargo build --release

The release executable is:

    target\release\moth-controller.exe
