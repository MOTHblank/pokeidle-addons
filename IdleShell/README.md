# PokéIdle Idle Shell

A minimal Windows WebView2 host for two persistent PokéIdle accounts with
browser-native userscript execution and lightweight Twitch/KICK chat presence.

## Architecture

- .NET 10 WinForms host.
- Two persistent WebView2 game profiles: AccountA and AccountB.
- WebView2 is used for PokéIdle itself, not for every stream/channel.
- Stream chat presence is a separate subsystem. A stream slot is account + channel + connection state; it does not own a browser surface.
- Twitch uses one hidden chat-only WebView2 per Twitch login profile. That WebView loads the official Twitch popout chat surface for one channel and keeps additional channel chats in hidden iframes inside the same profile/renderer.
- KICK uses the same one-hidden-WebView-per-login model with its official chat popout surface. No KICK video/player page is loaded by Idle Shell.
- A login can therefore cover the full ten-channel service capacity without creating ten browser instances.
- The stream dock is a lightweight control strip. It never lays out stream video. Collapsing the dock does not disconnect chat presence.
- Open channel in browser is an explicit external action for users who want video; video playback is not part of Idle Shell's resident stream system.
- Existing PokéIdle userscripts remain ordinary `*.user.js` files in `/addons`.
- The shell embeds the official upstream Violentmonkey 2.49.0 MV3 extension and installs it into each game WebView2 profile.
- `prepare-violentmonkey.ps1` downloads and verifies that extension before placing it under `IdleShell/vendor/violentmonkey`.
## Userscripts

Violentmonkey provides:

- `@match` and `@include`
- `@run-at document-start`, `document-end`, and `document-idle`
- `unsafeWindow`
- `GM_getValue`, `GM_setValue`, `GM_deleteValue`, `GM_listValues`
- `GM_addStyle`, `GM_setClipboard`, `GM_openInTab`
- `GM_xmlhttpRequest` using browser CORS for Hunt Atlas


The engine is intentionally not a fork of Violentmonkey. It uses the real
WebView2 browser-extension mechanism while keeping the repository's `*.user.js`
files compatible with normal userscript managers.

A repository-wide scan found no `@require` or `@resource` dependencies.
Hunt Atlas is the only addon using `GM_xmlhttpRequest`, and it only performs
public GET requests to PokéAPI. The runner namespaces GM storage by userscript
name.

## Build

Install the .NET 10 SDK, the Evergreen WebView2 Runtime, and PowerShell.
The first `.\\run.ps1` or `.\\build.ps1` run downloads and verifies the
official Violentmonkey 2.49.0 MV3 package.

    .\\run.ps1 -NoRun
    dotnet build IdleShell/IdleShell.csproj -c Release
    dotnet run --project IdleShell/IdleShell.csproj

Or:

    .\run.ps1
    .\run.ps1 -NoRun
    .\run.ps1 -Clean
    .\run.ps1 -Publish
    .\run.ps1 -DebugBuild

Normal build output is `IdleShell/bin/<Configuration>/net10.0-windows`.
Published output is `IdleShell/publish`.

The repository deliberately does not commit `bin/`, `obj/`, `out/`,
`publish/`, PDBs, or CRX files.

## Startup and diagnostics

Idle Shell checks the installed WebView2 Runtime before creating its
environments. Missing runtime errors are surfaced directly.

Fatal UI/process exceptions are written to:

    %LOCALAPPDATA%\Moth\IdleShell\idleshell.log

Normal addon, pane, routing, and restore information uses the same log.

## Persistent data

Persistent account data is stored under:

    %LOCALAPPDATA%\Moth\IdleShell\

Game profile data:

    %LOCALAPPDATA%\Moth\IdleShell\PokeIdle\

Stream profile data:

    %LOCALAPPDATA%\Moth\IdleShell\Streams\

## Background game and chat presence

Each game workspace can be switched independently between **Foreground** and
**Background**. Background games remain alive and keep running their userscripts;
the shell probes JavaScript responsiveness and 1-second timer drift every 30
seconds and shows a per-game health indicator.

Chat presence is deliberately independent of the game foreground/background state.
When the stream dock is collapsed, joined Twitch/KICK chats remain connected so the
account can continue to appear in chat.

The chat WebView environment does not disable Chromium background throttling. Its
only resident browser surfaces are the per-login chat hosts, and those hosts use
WebView2's low-memory target where available.
## Stream link routing

Twitch/Kick links inside PokéIdle are intercepted natively by the WebView2 host.
Game anchor clicks, `window.open`, popup requests, navigation backstops, and the
live-stream userscript all feed the same routing path. There is no competing legacy
router: the addon calls the shell's `__idleshell_openL## Stream link routing

Twitch/KICK links inside PokéIdle are intercepted natively by the WebView2 host.
Game anchor clicks, `window.open`, popup requests, navigation backstops, and the
live-chat userscript all feed the same routing path.

`addons/stream-auto-open.user.js` adds **Scan Live Streams** directly under
PokéIdle's **Abrir Inventário** button. Clicking it scans the currently rendered
Twitch/KICK channel links for live/online markers and sends every detected channel
through Idle Shell's chat-presence router. There is no background stream scan.

The stream dock exposes ten stable slots per service: T1-T10 for Twitch and
K1-K10 for KICK. The slots are management identities, not browser instances.
The same channel URL can be joined from both Game 1 and Game 2; the presence manager
reference-counts that shared channel so leaving one workspace does not disconnect
the other.

Twitch chat presence uses Twitch's official chat popout surface. KICK chat presence
uses KICK's documented per-channel chat popout surface. Neither service loads a
video/player page inside Idle Shell.

Right-click a slot for **Join/Rejoin chat**, **Log in**, **Open channel in browser**,
**Leave chat**, or closing the slot. The external browser action is the only path
that starts resident video playback.
## Scope

The shell uses the official Violentmonkey 2.49.0 codebase, with only the
WebView2-specific manifest packaging adjustment described above. The only custom
integration is the host-side installation plus synchronization of the repository's
userscripts into VM's own script database. Generated extension state stays
under LocalAppData rather than being committed to the repository.


## Twitch low-resource addon

`addons/twitch-low-resource.user.js` remains a standalone userscript for normal
Twitch browser usage. Idle Shell does not load it into its chat-presence host,
because the host does not play video.
