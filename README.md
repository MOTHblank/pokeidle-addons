# pokeidle-addons

PokéIdle userscripts plus a lightweight native controller for running dedicated browser profiles.

## Repository layout

- `addons/` — installable Tampermonkey/Violentmonkey-compatible userscripts.
- `controller/` — native Rust launcher/controller for dedicated PokéIdle browser profiles.
- `upstream/` — captured upstream PokéIdle client data.

## Runtime architecture

The controller does **not** embed a browser and does **not** implement a userscript engine.

PokéIdle runs in a normal installed browser with an isolated profile per game account. The existing userscripts remain ordinary `*.user.js` files and are executed by an existing userscript manager such as Violentmonkey or Tampermonkey.

The controller is responsible for browser/profile lifecycle and desktop orchestration only.

## Controller

The Rust controller currently provides the foundation for:

- finding a local Firefox installation;
- creating isolated Moth/PokéIdle profile directories;
- launching Firefox with a selected profile;
- opening PokéIdle in that profile.

The implementation intentionally contains no WebView2, Chromium embedding, C#, or custom userscript runtime.

See `controller/README.md` for the current development scope.

## Upstream snapshot

The tracked upstream snapshot is retained separately under `upstream/` and is not part of the Rust controller runtime.
