# pokeidle-addons

PokéIdle userscripts plus a lightweight Windows WebView2 shell for running two
persistent game accounts and optional Twitch/Kick stream panes.

## Layout

- `addons/` — installable Tampermonkey-compatible userscripts.
- `IdleShell/` — native .NET 10/WebView2 host.
- `upstream/` — captured upstream PokéIdle client data.

Idle Shell uses a browser-native WebView2 extension userscript engine, so the shipped addons work
without installing Tampermonkey or any browser extension. The same `*.user.js`
files remain usable in a normal userscript manager.

See `IdleShell/README.md` for build and runtime details.
