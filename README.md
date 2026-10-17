# Moth Controller — PokéIdle

A lightweight desktop companion for managing up to **four PokéIdle accounts**. Moth Controller keeps game and Twitch connections running natively in the background; Firefox is used for sign-in when needed and for optional interactive play.

**[Download the latest release](https://github.com/MOTHblank/pokeidle-addons/releases/latest)** · [Report an issue](https://github.com/MOTHblank/pokeidle-addons/issues)

## Features

- **Dashboard:** account overview, active hunts, Pokémon and opponent information, items, and session details.
- **Hunt Atlas:** hunt comparisons, experience estimates, capture planning, and Outland tiers.
- **Automation:** configure Auto Catch, item restocking, and market buying rules.
- **Accounts:** manage four independent profiles and their PokéIdle and Twitch connection states.
- **Game artwork:** item icons and Pokémon sprites load as needed and are cached locally. No separate asset download is required.

## Getting started

Download and open Moth Controller, then set up your account profiles in **Accounts**. Sign in to PokéIdle and Twitch when prompted. Once connected, use **Dashboard** to monitor sessions, **Automation** to adjust behavior, and **Hunt Atlas** to explore hunts.

The native controller does **not** need Violentmonkey, userscripts, or a browser window left open during normal background operation. **Open in Firefox** remains available for interactive gameplay.

## Building from source

From the `controller/` directory:

```sh
cargo build --release
```

See the [controller development notes](controller/README.md) and [visual asset documentation](controller/assets/visual/README.md) for implementation details.
