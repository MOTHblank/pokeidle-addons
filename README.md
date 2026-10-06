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

The upstream snapshot currently tracked in this repository is PokéIdle v1.240.1.
The live-stream addon watches the upstream SPA's rendered Twitch/Kick channel links
instead of binding to a single private selector, so it continues working when
the stream list is rebuilt during navigation or live/offline updates.

See `IdleShell/README.md` for build and runtime details.
