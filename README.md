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

The existing userscripts remain ordinary `*.user.js` files and are executed by an existing userscript manager such as Violentmonkey or Tampermonkey.

## Resource policy

The controller keeps the native side tiny and lets Firefox do the actual web work.

Stream handling is intentionally simple:

- live channels are opened as normal Firefox tabs/windows in the **same game profile**;
- there are no hidden browser instances or background WebViews;
- Twitch stream pages use the low-resource addon to target the lowest advertised video quality (preferring 160p) and hide nonessential UI;
- Twitch chat pop-outs are left alone so they remain usable without restoring the old hidden-chat architecture;
- the live-stream scanner uses a low-frequency existence check instead of a whole-document mutation observer.

This keeps the process count at roughly one Firefox instance per game account, with extra cost only when live stream tabs are actually open.

## Controller

The Rust controller currently provides:

- finding a local Firefox installation;
- creating isolated Game 1/Game 2 profile directories;
- seeding conservative, one-time Firefox low-overhead defaults;
- launching Firefox with the selected game profile;
- opening PokéIdle in that profile.

The controller intentionally contains no WebView2, Chromium embedding, C#, or custom userscript runtime.

See `controller/README.md` for the current development scope.

## Upstream snapshot

The tracked upstream snapshot is retained separately under `upstream/` and is not part of the Rust controller runtime.
