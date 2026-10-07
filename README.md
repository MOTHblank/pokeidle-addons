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
The live-stream addon adds a manual scan button under PokéIdle's **Abrir Inventário** button.
It scans the currently rendered Twitch/KICK channel links only when clicked, then routes
those live/online channels through Idle Shell's chat-presence system.

See `IdleShell/README.md` for build and runtime details.
