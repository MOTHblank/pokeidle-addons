# PokéIdle Idle Shell

A minimal Windows WebView2 host for running persistent PokéIdle accounts with
native userscript injection and optional persistent Twitch/Kick stream panes.

## Architecture

- .NET 10 WinForms host.
- Two persistent WebView2 game profiles: AccountA and AccountB.
- One shared stream environment with separate named stream profiles.
- The UI is split into two independent Game 1 / Game 2 workspaces.
- Each workspace has its own stream tab strip and can use up to 10 stream panes
  at once, including Twitch and Kick accounts.
- Existing PokéIdle userscripts remain ordinary `*.user.js` files in `/addons`.
- The shell registers them directly with WebView2's
  `AddScriptToExecuteOnDocumentCreatedAsync`; no browser extension or CRX is
  required.
- The runtime follows the common Greasemonkey/Tampermonkey/Violentmonkey API
  conventions instead of bundling a partial browser-extension clone.

## Userscripts

The native runner supports:

- `@match` and `@include`
- `@run-at document-start`, `document-end`, and `document-idle`
- `unsafeWindow`
- `GM_getValue`, `GM_setValue`, `GM_deleteValue`, `GM_listValues`
- `GM_addStyle`, `GM_setClipboard`, `GM_openInTab`
- `GM_xmlhttpRequest` using browser CORS for Hunt Atlas
- safe no-op implementations for menu, notification, and resource APIs

A repository-wide scan found no `@require` or `@resource` dependencies.
Hunt Atlas is the only addon using `GM_xmlhttpRequest`, and it only performs
public GET requests to PokéAPI. The runner namespaces GM storage by userscript
name.

## Build

Install the .NET 10 SDK and the Evergreen WebView2 Runtime.

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

## Background game and stream mode

Each game workspace can be switched independently between **Foreground** and
**Background**. Background games remain alive and keep running their userscripts;
the shell probes JavaScript responsiveness and 1-second timer drift every 30
seconds and shows a per-game health indicator.

Inactive stream panes default to **Background** mode:
`CoreWebView2Controller.IsVisible = false`, while Chromium's background
timer throttling flags remain disabled. **Parked** mode keeps a pane rendered
off-screen as a fallback.

The toolbar probe samples visibility state, focus, and `setTimeout(1000)` drift
every 30 seconds and writes `probe.csv` under the LocalAppData root.

## Stream link routing

Twitch/Kick links inside PokéIdle are intercepted by
`addons/idleshell-link-router.user.js` and handed to the host. The same
streamer URL is shared between the two game accounts, so a click in either game
fans the URL out to **both Game 1 and Game 2**, one stream pane per game using
the matching enabled Twitch/Kick login profile. Manual `G1 + Stream` and
`G2 + Stream` remain available for opening a stream explicitly.

Stream profiles are configured through Accounts and persisted in
`accounts.json`. The legacy `stream-accounts.txt` file is imported on first
run and then mirrored by AccountManager.

## Scope

The shell intentionally keeps userscripts simple and transparent: no proprietary
extension bootstrap, no CRX extraction, and no checked-in generated build state.
