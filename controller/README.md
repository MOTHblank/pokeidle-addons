# Moth Controller

Native Rust controller for PokéIdle using dedicated Firefox profiles on **Windows and Linux**.

The graphical app manages up to four isolated game accounts. Each account uses one
Firefox profile for PokéIdle, Twitch login, and its userscripts. There is no
embedded WebView or separate browser engine.

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
   active Pokémon, trainer XP, **EXP/hour**, estimated time to the next trainer
   level, Auto Catch, confirmed Twitch XP bonuses, and warnings. Use **Show details**
   on an account to open its full-width inventory, capture statistics, tabs,
   streams and addon diagnostics below the overview. Only one detailed account
   is expanded at a time; **Hide details** returns to the compact view.
   Launch, stop and browser-mode controls remain available in the overview.

The Controller estimates EXP/hour from recent cumulative trainer XP gains
over elapsed time, using a rolling history to smooth out short combat bursts.
The time to the next level uses the game's next-level cumulative XP threshold.
These figures are estimates, not promises: the EXP rate displays 0 when no
recent XP was earned, while the time to level up stays unavailable until the
character is earning XP. Missing data and counter resets never produce guessed
figures. Both remain lightweight and require no additional game polling.
Detailed monitoring diagnostics are collapsed below the dashboard by default.

Twitch chat sharing and stream scanning are handled by the userscripts.
The initial stream scan runs about 30 seconds after the site loads; subsequent
stream scans run every five minutes. The controller's game/Firefox telemetry
uses separate layers:
- **Heartbeat:** a minimal read of the known game tab every **45 seconds**.
  It confirms whether the game page is reachable and whether login is present,
  without traversing the tab tree or reading game inventory.
- **Game statistics:** full game and Controller Bridge data every **3 minutes**
  when signed in, or **45 seconds** while loading or awaiting login.
- **Firefox tabs:** a full per-tab inventory roughly every **10–12 minutes**,
  or earlier when the tab context list changes during a game scan, login
  changes, or a Controller-requested tab opens. Game scans still use
  `browsingContext.getTree` to find the game, but do not run JavaScript in
  unchanged tabs.
- **Recovery:** reconnects try again every **5 seconds**, and an invalidated
  heartbeat context triggers a full game discovery without blindly reopening
  tabs. Initial expensive scans start at 12-second offsets across Game1–Game4;
  recovered sessions do not wait for the stagger.
- **Addon version checks:** unchanged at **4 hours**; **stream scanning** stays
  separate, every **5 minutes**.

The Rust UI repaints cached data every **3 seconds**, without extra BiDi
requests. Overview cards distinguish a recent heartbeat from older game data,
and the expanded tabs panel shows when its inventory was last inspected.
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
