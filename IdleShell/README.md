# PokéIdle Idle Shell

A minimal Windows WebView2 host for running two persistent PokéIdle accounts without
browser tabs or private windows.

## Architecture

- .NET 10 WinForms host.
- One shared WebView2 environment.
- Two persistent WebView2 profiles: AccountA and AccountB.
- Existing PokéIdle userscripts live in /addons.
- Userscripts are loaded from *.user.js files and injected before page scripts.
- The host currently provides a small compatibility layer for unsafeWindow and
  GM_xmlhttpRequest.
- Chromium background throttling flags are enabled experimentally.

WebView2 multi-profile support allows profiles to share system resources while
keeping profile data isolated.

## Build

Install the .NET 10 SDK and the WebView2 Runtime.

    dotnet build IdleShell/IdleShell.csproj -c Release
    dotnet run --project IdleShell/IdleShell.csproj

## Data

Persistent account data is stored under:

    %LOCALAPPDATA%\Moth\IdleShell\PokeIdle\

The two profile names are AccountA and AccountB.

## Limitations

This is an initial shell, not a complete Tampermonkey replacement.

- Only @match and @run-at metadata are interpreted.
- GM_xmlhttpRequest uses the WebView fetch implementation.
- Other GM_* APIs are not implemented.
- The shell does not spoof document.hidden or document.hasFocus().
- Browser backgrounding flags are experimental.
