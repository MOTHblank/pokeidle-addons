# pokeidle-addons

PokéIdle userscripts plus a lightweight Windows WebView2 shell for running two
persistent accounts without incognito browser windows.

## Layout

- addons/ — installable Tampermonkey-compatible userscripts.
- IdleShell/ — native .NET 10/WebView2 two-account host.
- upstream/ — captured upstream PokéIdle client data.

The Idle Shell does not replace Tampermonkey yet. It provides the browser/session
container and a small userscript injection layer so the existing addons can be
used without a full browser.

See IdleShell/README.md for build and runtime details.
