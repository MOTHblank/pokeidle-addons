# Moth Controller

Native Rust Windows controller for dedicated PokéIdle Firefox profiles.

## Run

The controller is intended to be a normal Windows application. A release build produces:

    moth-controller.exe

Double-clicking it opens the controller window and automatically launches the two dedicated game profiles.

## Profiles

Each game account owns exactly one Firefox profile:

    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game1
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game2

That same profile is used for:

- PokéIdle;
- the game account's one Twitch login;
- the game account's one KICK login;
- any Twitch/KICK stream tabs opened from PokéIdle.

Do not create separate stream-login profiles. Keeping the sessions together is deliberate and avoids extra Firefox processes and duplicated browser state.

## Current UI

- **Game 1** — launch/relaunch Game 1's Firefox profile.
- **Game 2** — launch/relaunch Game 2's Firefox profile.
- **Launch Both** — launch/relaunch both profiles.
- **Open Profiles** — open the profile data directory in Explorer.
- **Close** — close the controller without closing Firefox.

## Stream/resource model

The controller does not create hidden stream windows or background browser engines.

When the live-stream userscript finds Twitch/KICK channels, it opens normal browser pages in the already-running game profile. Twitch pages are then reduced by the repository's low-resource userscript. Twitch chat pop-outs continue to use the same profile and login but are not suppressed by the low-resource video rules.

The goal is approximately one Firefox instance per game account, with stream resource usage added only while stream tabs are actually open.

## Architecture

The controller does not embed a browser, use WebView2, or implement a userscript engine.

PokéIdle runs in normal Firefox processes. An existing userscript manager such as Violentmonkey or Tampermonkey owns the repository's ordinary `*.user.js` files.

The controller is responsible for desktop/process/profile orchestration only.

## Development

From this directory:

    cargo build

For an optimized build:

    cargo build --release

The release executable is:

    target\release\moth-controller.exe
