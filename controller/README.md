# Moth Controller

The desktop controller manages authenticated PokéIdle and Twitch connections for up to four accounts. Browser windows are needed for sign-in and optional interactive gameplay, not for ordinary background execution.

## Run and build

- Download a Windows or Linux binary from [Releases](https://github.com/MOTHblank/pokeidle-addons/releases/latest).
- To build locally, run `cargo build --release` from the `controller/` directory.
- Firefox or Firefox Developer Edition is required if sign-in has not been saved or has expired. Set `MOTH_FIREFOX` to its executable path if necessary.
- In **Accounts**, four cards show account names, PokéIdle login state, Twitch login state, and Start/Stop controls. A successful setup confirms both connections before saving the setup marker.
- The **Open in Firefox** action is for manual browser play, while **Start account** prefers saved credentials and opens a visible Firefox window if authentication is needed.

### Build while the Windows app is running

Windows prevents overwriting an executable in use. From PowerShell in `controller/`, run:

```powershell
.\build-windows.ps1
```

The script detects a locked `target/release/moth-controller.exe` and builds to a separate output folder instead, without stopping active sessions. It prints the exact executable path.

## Game icons and cache

In **Options**, **Download game icons** queues known public Poké Ball and item images on the dedicated cache worker. Downloads are stored per user:

- Windows: `%LOCALAPPDATA%\Moth\PokeIdle\VisualCache`
- Linux: `~/.cache/moth-controller/visual` (or `$XDG_CACHE_HOME/moth-controller/visual`)

The committed [asset manifest](assets/visual/manifest.json) currently contains no bundled sprites. Pokémon image URLs have not yet been mapped, so the button only downloads the known images. See [visual asset notes](assets/visual/README.md) for mapping, packaging, and permission details.

## Automation and Hunt Atlas

The app handles Auto Catch, restocking (including potions and revives), Moth Watch market rules and Snatch, and Hunt Atlas estimates. Historical battle details are available through Dashboard details. Mission settings remain read-only until reliable update commands are identified.

The ordinary desktop release version is **0.1.7**. The latest available published release may be older until a new release is built and uploaded.

## Development

```sh
cd controller
cargo check
cargo test
cargo build --release
```

The desktop app requires live authenticated PokéIdle and Twitch sessions to verify sign-in and game data. Keep credentials out of logs and commits. The validation workflow runs manually.
