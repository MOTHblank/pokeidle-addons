# Moth Controller

Native Rust Windows controller for dedicated PokéIdle Firefox profiles.

## Run

The controller is intended to be a normal Windows application. A release build produces:

    moth-controller.exe

Double-clicking it opens the controller window and automatically manages the enabled dedicated game profiles (up to four).

## Profiles

Each game account owns exactly one Firefox profile:

    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game1
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game2
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game3
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game4

That same profile is used for:

- PokéIdle;
- the game account's one Twitch login;
- the game account's one KICK login;
- any Twitch/KICK stream tabs opened from PokéIdle.

Do not create separate stream-login profiles. Keeping the sessions together is deliberate and avoids extra Firefox processes and duplicated browser state.

## Current UI

The **Accounts** window manages up to four independent Firefox profiles. Each row operates on exactly one game Firefox profile:

- **Open Game** — launch that profile.
- **Twitch** — open Twitch login in that profile.
- **KICK** — open KICK in that profile.
- **Streams** — open the userscript stream manager in that profile.
- **Scan Live** — open PokéIdle and trigger the userscript live-chat scanner.
- **Addons** — open the repository’s core `.user.js` installer/update URLs in that profile.
- **Profile** — open the profile directory in Explorer.

The controller never stores Twitch/KICK passwords or tokens. Their sessions remain in Firefox profile storage.

## Stream/resource model

The controller does not create hidden stream windows or background browser engines.

The stream userscript provides 10 Twitch + 10 KICK chat slots for each game. Those slots open the official chat-only pop-out URLs in the already-running game profile, so the profile's existing Twitch/KICK login is reused. The manager can open or close slots individually or all at once.

The actual video pages are separate and optional. The normal stream path is chat-only, so 20 configured channels do not mean 20 video decoders.

The goal is approximately one Firefox instance per game account, with stream resource usage coming from chat connections only until video is explicitly requested.

## Architecture

The controller does not embed a browser, use WebView2, or implement a userscript engine.

PokéIdle runs in normal Firefox processes. The **official Violentmonkey Firefox extension** owns the repository's ordinary `*.user.js` files.

The controller opens the raw `.user.js` URLs for installation/update. The first install still goes through Violentmonkey's normal confirmation UI; the scripts carry `@updateURL` and `@downloadURL` so later updates are handled by Violentmonkey.

The controller is responsible for desktop/process/profile orchestration and launching stream/addon commands in the correct profile.

## Development

From this directory:

    cargo build

For an optimized build:

    cargo build --release

The release executable is:

    target\release\moth-controller.exe
