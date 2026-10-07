# pokeidle-addons

PokéIdle userscripts plus a lightweight Windows WebView2 shell for running two
persistent game accounts with lightweight Twitch/KICK chat presence.

## Stream architecture

Idle Shell does not create a WebView2 instance for every stream. It keeps the two
PokéIdle game WebViews, while Twitch and KICK chat presence is handled separately.
Each configured login owns at most one hidden chat-only WebView2 host, and that host
can keep multiple channel chats connected. Video is not loaded by the resident
stream subsystem; an explicit external-browser action is required to watch video.


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
