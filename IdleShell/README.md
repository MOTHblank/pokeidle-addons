# PokéIdle Idle Shell

A minimal Windows WebView2 host for running persistent PokéIdle accounts with
browser-native userscript execution and optional persistent Twitch/Kick stream panes.

## Architecture

- .NET 10 WinForms host.
- Two persistent WebView2 game profiles: AccountA and AccountB.
- One shared stream environment with separate named stream profiles.
- The UI is split into two independent Game 1 / Game 2 workspaces.
- Each workspace has a collapsible stream dock. It is collapsed by default so the
  game keeps almost the full vertical workspace for normal navigation.
- The stream dock has a service selector and ten stable slots per service: T1–T10
  for Twitch and K1–K10 for Kick. Slot labels identify the stream position, not
  the login profile, so multiple tabs no longer alternate between account names.
- Each slot is mapped to an enabled login profile behind the scenes. A single login
  profile may carry the full ten-slot service capacity.
- Existing PokéIdle userscripts remain ordinary `*.user.js` files in `/addons`.
- The shell embeds the official upstream **Violentmonkey 2.49.0 MV3** extension
  and installs it into each WebView2 profile with
  `CoreWebView2Profile.AddBrowserExtensionAsync`.
- `prepare-violentmonkey.ps1` downloads the official release archive once and
  verifies its SHA-256 before placing the unpacked extension under
  `IdleShell/vendor/violentmonkey`. The downloaded runtime is ignored by git.
- The shell sends the repository's `*.user.js` files to Violentmonkey through
  VM's own `ParseScript` command on the extension's options page. Chromium and
  Violentmonkey then own metadata parsing, matching, `run_at`, frame targeting,
  sandboxing, GM APIs, storage, resources, dependencies and execution.

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

The shell uses the official Violentmonkey 2.49.0 codebase, with only the
WebView2-specific manifest packaging adjustment described above. The only custom
integration is the host-side installation plus synchronization of the repository's
userscripts into VM's own script database. Generated extension state stays
under LocalAppData rather than being committed to the repository.


## Twitch low-resource addon

`addons/twitch-low-resource.user.js` runs on Twitch stream pages. It repeatedly
forces the lowest quality exposed by Twitch's own quality menu (normally 160p),
re-applies the setting after player/navigation changes, and disables nonessential
chat/sidebar rendering with conservative CSS. It does not spoof the player
protocol or rewrite Twitch media requests, so it remains compatible with normal
Twitch playback while materially reducing bandwidth and video decode/render cost.
