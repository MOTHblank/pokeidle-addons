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

## Current scope

The former `UserscriptLoader.cs` compatibility injector remains in the
repository only as a fallback/reference while Tampermonkey is being validated.

The next shell layer should build tabs and explicit foreground/background
lifecycle on top of the profile/extension foundation.
