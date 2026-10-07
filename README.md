# pokeidle-addons

PokéIdle userscripts plus a lightweight native controller for running dedicated browser profiles.

## Repository layout

- `addons/` — installable Tampermonkey/Violentmonkey-compatible userscripts.
- `controller/` — native Rust launcher/controller for dedicated PokéIdle browser profiles.
- `upstream/` — captured upstream PokéIdle client data.

## Runtime architecture

The controller does **not** embed a browser and does **not** implement a userscript engine.

PokéIdle runs in normal installed Firefox with one isolated profile per game account:

- **Game 1 profile** — PokéIdle + exactly one Twitch login + exactly one KICK login.
- **Game 2 profile** — PokéIdle + exactly one Twitch login + exactly one KICK login.

The game and both streaming services deliberately share the same browser profile. That means a stream opened for Game 1 uses Game 1's Twitch/KICK sessions, while Game 2 uses Game 2's sessions.

The existing userscripts remain ordinary `*.user.js` files and are executed by the **official Violentmonkey Firefox extension**. The controller opens their official raw `.user.js` URLs for installation/update; Violentmonkey still owns the actual installation and execution. Initial installation therefore uses Violentmonkey's normal confirmation UI, while the scripts carry `@updateURL`/`@downloadURL` metadata for subsequent updates.

## Resource policy

The controller keeps the native side tiny and lets Firefox do the actual web work.

Stream handling is intentionally simple:

- live channels are opened as **chat-only** Twitch/KICK pop-outs in the **same game profile**;
- each game has 10 Twitch + 10 KICK chat slots;
- the stream manager remembers those slots in the game's localStorage and can open/close them individually or all at once;
- there are no hidden browser instances or background WebViews;
- actual video is only loaded when the user deliberately opens a normal stream page;
- Twitch chat pop-outs are left alone by the low-resource video addon;
- the live-stream scanner uses a low-frequency existence check instead of a whole-document mutation observer.

This keeps the process count at roughly one Firefox instance per game account. The normal stream path adds chat connections only, not video decoders for every followed channel.

## Controller

The Rust controller provides:

- finding a local Firefox installation;
- creating isolated Game 1/Game 2 profile directories;
- seeding conservative, one-time Firefox low-overhead defaults;
- launching Firefox with the selected game profile;
- opening Twitch/KICK login pages in the matching profile;
- opening all repository addon installer/update URLs in the matching profile;
- opening the stream manager or triggering the live-chat scanner in the matching profile.

The controller intentionally contains no WebView2, Chromium embedding, C#, or custom userscript runtime.

See `controller/README.md` for the current development scope.

## Upstream snapshot

The tracked upstream snapshot is retained separately under `upstream/` and is not part of the Rust controller runtime.
