# PokéIdle Idle Shell

A minimal Windows WebView2 host for running persistent PokéIdle accounts with
real Chromium browser-extension support.

## Architecture

- .NET 10 WinForms host.
- One shared WebView2 environment.
- Two persistent WebView2 profiles: AccountA and AccountB.
- Browser extensions are enabled in the WebView2 environment.
- Tampermonkey is installed once per profile and then persists with that profile.
- Existing PokéIdle userscripts remain ordinary `*.user.js` files in `/addons`.
- The shell does not execute those scripts itself.
- Tampermonkey owns userscript metadata, grants, storage, cross-origin APIs,
  execution timing, and page access.

WebView2 profiles separate cookies and other profile data while allowing
multiple profiles to share browser resources. Browser extensions installed into
a profile are persisted for future sessions.

## First-time Tampermonkey setup

Current Tampermonkey releases are distributed under a proprietary license.
The repository therefore does not vendor the extension package.

Run once from the repository:

    powershell -ExecutionPolicy Bypass -File IdleShell/setup-tampermonkey.ps1

The script downloads the official stable Tampermonkey package and extracts it
to:

    %LOCALAPPDATA%\Moth\IdleShell\Extensions\Tampermonkey

Idle Shell installs that unpacked package into AccountA and AccountB
automatically.

## Addon provisioning

On startup the shell:

1. Reads all `addons/*.user.js` files.
2. Generates a Tampermonkey provisioning document at:

       %LOCALAPPDATA%\Moth\IdleShell\Tampermonkey\tm.json

3. Serves that document from a loopback HTTP endpoint.
4. Registers Tampermonkey's `jsonImport` configuration with the generated
   SHA-256 integrity hash.
5. Installs or starts Tampermonkey in both WebView2 profiles.

Tampermonkey 5.5+ supports `jsonImport` provisioning. The current 5.6
release accepts exported provisioning data and reuses script UUIDs during
reprovisioning.

The policy is written under the WebView2-specific Windows policy root instead
of the normal Microsoft Edge browser policy root. This is deliberate because
Microsoft documents that most Edge browser-only policies do not affect
WebView2. The policy path is therefore an integration point that must be
validated against the actual WebView2 Runtime before it is considered a final
deployment mechanism.

## Build

Install the .NET 10 SDK and the WebView2 Runtime.

    dotnet build IdleShell/IdleShell.csproj -c Release
    dotnet run --project IdleShell/IdleShell.csproj

## Data

Persistent account data is stored under:

    %LOCALAPPDATA%\Moth\IdleShell\PokeIdle\

The two profile names are AccountA and AccountB.

Tampermonkey's installed-extension state is stored in those WebView2 profiles.

## Stream background mode

Inactive stream panes default to **Background** mode: `CoreWebView2Controller.IsVisible
= false`, which stops compositing (the expensive part) while the Chromium flags in
`AppConfig` (`--disable-background-timer-throttling`,
`--disable-renderer-backgrounding`, `--disable-backgrounding-occluded-windows`) keep
page timers unthrottled. `TrySuspend` is deliberately not used — it pauses script
timers and animations, which would stop the addons.

Note: `CoreWebView2Settings.PreferredBackgroundTimerWakeInterval` (the prerelease
"unthrottled hidden timers" API) is **not** exposed by the pinned WebView2 package
(1.0.4258.31); the browser flags cover this instead.

The toolbar button "Hidden panes:" toggles Background ↔ Parked (off-screen but still
rendering — use it only if a platform pauses when hidden). The toggle persists to
session.json.

## Probe

While the app runs, every 30 s each pane evaluates a small script that reports
`document.visibilityState`, `document.hidden`, `document.hasFocus()` and the drift of
a `setTimeout(1000)`; rows are appended to:

    %LOCALAPPDATA%\Moth\IdleShell\probe.csv

Columns: `timestamp,title,kind,state,visibility,hidden,focus,driftMs`. Run a stream
in Background mode for ~10 minutes: if `state=Background` rows keep
`visibility=visible` and `driftMs` near 0 (and video/chat/points keep advancing),
hidden panes are safe; if they show `hidden`/large drift, switch those panes to
Parked or revisit the host choice. Uncheck "Probe" in the toolbar to stop logging.

## Stream link routing (drop farming on every account)

Twitch/Kick links clicked inside the PokéIdle game are intercepted and opened
as **background stream panes — one per configured Twitch/Kick login**, so each
link plays simultaneously on both accounts:

1. `addons/idleshell-link-router.user.js` (installed via Tampermonkey into the
   game profiles) hooks `window.open` and anchor clicks on pokeidle.io and
   posts matching twitch.tv/kick.com URLs to the host over
   `window.chrome.webview.postMessage`.
2. Each pane also gets a shell bootstrap injected into **every isolated world**
   (`AddScriptToExecuteOnDocumentCreatedAsync`) defining
   `window.__idleshell_openLink(url)` for direct use by any script.
3. The host fans the URL out to one hidden Background-mode pane per stream
   profile, creating them on first use and persisting them to session.json.
4. Backstops if the userscript is bypassed: game-pane popups
   (`NewWindowRequested`, captured instead of spawning OS windows) and game-pane
   navigations onto a stream host (`NavigationStarting`, cancelled + rerouted).

Stream accounts live in WebView2 profiles `Stream1`, `Stream2` under
`AppConfig.StreamProfiles`, overridable with a text file:

    %LOCALAPPDATA%\Moth\IdleShell\stream-accounts.txt   (one alphanumeric name per line)

All stream profiles share one user data folder so Chromium pools their
processes (lower RAM per extra account); cookies/logins stay separated per
profile. To log a background account in: click it in the stream picker
(Foreground), use "Log in with Twitch" — its OAuth popup now opens as a visible
pane in the same profile — sign in, then press **Background**.

Toolbar: **Foreground** shows the selected stream, **Background** hides all
streams at once (games take full width; hidden panes stop compositing but keep
running timers/addons), **Mute/Unmute** toggles audio on the visible stream.
Routing events are appended to `%LOCALAPPDATA%\Moth\IdleShell\idleshell.log`.

## Current scope

The former `UserscriptLoader.cs` compatibility injector remains in the
repository only as a fallback/reference while Tampermonkey is being validated.

Known follow-ups: per-pane (not global) Foreground/Background choice, and
verifying on Windows that the pinned WebView2 runtime honors the unthrottle
flags with `IsVisible=false` (the probe CSV answers this).
