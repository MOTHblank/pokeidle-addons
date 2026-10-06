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
- The shell no longer injects those userscripts itself.
- Tampermonkey owns userscript metadata, grants, storage, cross-origin APIs,
  execution timing, and page access.

WebView2 profiles separate cookies and other profile data while allowing
multiple profiles to share browser resources. Browser extensions installed into
a profile are persisted for future WebView2 sessions.

## First-time Tampermonkey setup

Current Tampermonkey releases are distributed under a proprietary license.
The repository therefore does not vendor the extension package.

Run once from the repository:

    powershell -ExecutionPolicy Bypass -File IdleShell/setup-tampermonkey.ps1

The script downloads the official Tampermonkey package and extracts it to:

    %LOCALAPPDATA%\Moth\IdleShell\Extensions\Tampermonkey

Idle Shell installs that unpacked package into AccountA and AccountB
automatically.

The setup script currently pins Tampermonkey 5.6.6242.

## Build

Install the .NET 10 SDK and the WebView2 Runtime.

    dotnet build IdleShell/IdleShell.csproj -c Release
    dotnet run --project IdleShell/IdleShell.csproj

## Data

Persistent account data is stored under:

    %LOCALAPPDATA%\Moth\IdleShell\PokeIdle\

The two profile names are AccountA and AccountB.

Tampermonkey's installed-extension state is stored in those WebView2
profiles.

## Current scope

The shell now delegates userscript execution to Tampermonkey. The former
`UserscriptLoader.cs` compatibility injector remains in the repository only
as a fallback/reference while the integration is being validated.

The next shell layer should build tabs and explicit foreground/background
lifecycle on top of this profile/extension foundation.
