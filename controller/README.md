# Moth Controller

> **Experimental architecture branch:** [Native game + Twitch IRC guide](docs/native-background-experiment.md) · [Native Hunts / Hunt Atlas](docs/native-hunts-atlas.md). Rust owns game, Twitch IRC, capture and hunt intelligence; Firefox is for authentication or recovery only. Some advanced marketplace/Atlas features still need native parity.

Native Rust game and Twitch IRC controller for PokéIdle on **Windows and Linux**.

The graphical app manages up to four isolated native game/Twitch sessions.
Firefox profiles are used for first login or expired authentication only, not for normal gameplay or chat.

## Running the app

Use a release executable built for your operating system:
- Windows: `moth-controller.exe`
- Linux: `moth-controller`

On Linux, install Firefox (Developer Edition is preferred), the Violentmonkey
extension, and your distribution's `xdg-utils` and `curl` packages
(for desktop opening and retrieving the live addon list).
A desktop session with X11 or Wayland is required for the controller GUI and
visible Firefox. Headless mode remains an optional per-account browser mode.

If Firefox is not found automatically, set `MOTH_FIREFOX` to the complete
path to the Firefox executable before launching the controller.

## Profile and log locations

- Windows: `%LOCALAPPDATA%\Moth\PokeIdle\`
- Linux: `$XDG_DATA_HOME/Moth/PokeIdle/` (default: `~/.local/share/Moth/PokeIdle/`)

Within that directory, profiles are under `Profiles/Game1` through
`Profiles/Game4`, account preferences are in `accounts.json`, and
`moth-controller.log` records runtime errors.

The Linux and Windows data directories are separate; the controller does not
automatically copy profiles between operating systems.

## Accounts and userscripts

1. Open **Account Manager**, and use **Open Game** to launch a visible Firefox
   for each account's login and setup.
2. Use **Google sign-in** when browser automation prevents authentication; it
   launches visible Firefox without the remote-debugging endpoint.
3. Use **Install Violentmonkey** and **Manage Addons → Install all (first setup)**
   in each enabled Firefox profile. Confirm installation in Violentmonkey. The
   installer queries the live GitHub `addons/` directory, not a baked-in list.
   Use **Repair selected** to open only the selected installers later. An
   opened installer tab is not proof of installation.
   Enable automatic updates in Violentmonkey once per profile. Its updates
   do not require another installer run or a new controller build. **Check now**
   queries the published script versions without installing anything, and
   **Manage Addons** displays detected, outdated, or unverified scripts.
   Dashboard warnings link straight to the affected profile's management UI.
   All version and installation checks are non-blocking, and the game is never
   reloaded automatically. Reload the tab manually after an update.
   Initial setup instructions disappear after the complete live setup is
   verified, or when the user explicitly finishes setup with a subset of
   the available addons. **Show setup guide** reopens those instructions.
4. Return to the main dashboard. The default **compact overview** shows all
   enabled accounts in a two-column layout when space allows: current hunt,
   active Pokémon, **Gold**, **Gems**, **EXP/hour**, Auto Catch, confirmed Twitch
   XP bonuses, and warnings. Gold, Gems and EXP/hour share one compact
   three-column row. Use **Show details** on an account to open its full-width
   trainer/Pokémon XP, inventory, capture statistics, tabs, streams and addon
   diagnostics below the overview. Only one detailed account is expanded at
   a time; **Hide details** returns to the compact view. Launch, stop and
   browser-mode controls remain available in the overview.

The Controller estimates EXP/hour from recent cumulative trainer XP gains
over elapsed time, using a rolling history to smooth out short combat bursts.
This is an estimate, not a promise: the EXP rate displays 0 when no recent XP
was earned. Missing data and counter resets never produce guessed rates.
Gold and Gems are displayed only with a fresh verified bridge snapshot, rather
than showing an unverified zero. This uses existing game telemetry without
additional polling.
Detailed monitoring diagnostics are collapsed below the dashboard by default.

Twitch chat sharing and stream scanning are handled by the userscripts.
The initial stream scan runs about 30 seconds after the site loads; subsequent
stream scans run every five minutes. The controller's game/Firefox telemetry
uses separate layers:
- **Heartbeat:** a minimal read of the known game tab every **45 seconds**
  when the controller is focused, or **90 seconds** when unfocused or hidden
  in the Windows system tray. It confirms page reachability without game inventory.
- **Compact statistics:** each account is sampled every **60 seconds** while
  the controller has focus, including Controller Bridge balances and XP data.
- **Expanded statistics:** **only the one account with details open while
  the controller is focused** is sampled every **5 seconds**. Opening details
  requests an immediate sample; closing them returns to the 60-second cadence.
  The fast path reuses the discovered game tab and refreshes the tab tree at most
  once per minute, unless navigation invalidates that context. Hunt Atlas and
  capture statistics are included without spawning a new BiDi session.
- **Unfocused or hidden to tray:** switching to another application (even
  without minimizing Moth) or hiding Moth in the Windows system tray puts
  **all accounts on the 3-minute background cadence**. UI repaint requests
  drop from every **3 seconds** to every **30 seconds**. Refocusing or
  restoring the window resumes the 60-second compact or 5-second expanded
  cadence automatically; the selected detail panel remains open. Unknown
  OS focus state does not force background mode. Gameplay and userscripts
  keep running unchanged.
- **Firefox tabs:** a full per-tab inventory remains roughly every
  **10–12 minutes**, or sooner when a full context scan detects changed tabs
  or a Controller-requested tab opens. Fast detail samples don't inspect
  unrelated tabs.
- **Recovery:** a failed game probe or timed-out heartbeat first retries
  over the **existing BiDi session** rather than discarding Firefox's single
  active session. Closed connections retry every **5 seconds**; the Firefox
  error `Maximum number of active sessions` backs off to **60 seconds**,
  since opening more sockets cannot clear an orphaned session. Initial
  expensive scans start at 12-second offsets across Game1–Game4; recovered
  sessions do not wait for the stagger. If Firefox retains an unusable BiDi
  session, the affected account shows **BIDI SESSION BLOCKED** and
  **Repair monitor (restart Firefox)**. This explicit action restarts only
  that Firefox profile in the same visible/headless mode. It briefly
  interrupts gameplay; monitoring failures never auto-restart an account.
- **Addon version checks:** unchanged at **4 hours**; **stream scanning** stays
  separate, every **5 minutes**.

The Rust UI repaints cached data every **3 seconds** while focused, or every
**30 seconds** while unfocused or hidden in the tray, independently of BiDi
sampling. OS focus events wake the UI when returning to Moth. Overview cards
distinguish a recent heartbeat from older game data, and the expanded tabs
panel shows when its inventory was last inspected.
KICK links can be opened in a normal browser.
Linux does not currently have the Windows system tray integration.

## Browser lifecycle and monitoring

The controller first tries WebDriver BiDi `browser.close`, then a native
close request (Windows) or SIGTERM (Linux), and force-terminates only as a
last resort. It prevents launching into profiles already owned by a Firefox
process. Recovered processes are never automatically terminated on controller
exit; use the account's **Stop** action to close them explicitly.

Commands sent through the BiDi tab monitor have a bounded queue, expiry, and
completion/error status. A failed or uncertain tab creation is not retried
blindly, to avoid accidentally opening duplicates.

The controller does not store browser account passwords or tokens.

## Publishing userscript changes

A modified userscript must increase its `@version` and its matching
DOM version marker. Violentmonkey compares versions: changing code on
`master` without incrementing `@version` will not automatically
reach existing installations.

For local Git commits, run `git config core.hooksPath .githooks` once
to enable the included version guard. It rejects staged userscript edits
that do not increase the version or have a mismatching marker. No GitHub
Actions, new controller release, or second userscript updater is needed.

### Native account interfaces (experimental)

- **Panel → Hunts · Atlas:** Current unlocked hunts, spawn composition, active Pokémon, live trainer/Pokémon XP/h, current hunt session totals, battle drops, shinies, and native hunt switching.
- **Panel → Inventory:** Server-reported balls/items by account (or combined), names/categories from a lazily fetched public item catalog, and a separate tally of items acquired through battle drops rather than purchases.
- **Account details → Automation → Configure native Auto Catch:** Ball priority and reserves, restock IDs/thresholds and protected gold reserve; settings are saved to the OS-encrypted account cache and applied to the running native worker.

These features require no browser userscripts after authentication. Hunt projections remain experimental, and the session drop ledger starts with the current native controller runtime.
