# Moth Controller

Native Rust launcher/controller for dedicated PokéIdle browser profiles.

## Current scope

The first slice is intentionally small:

- finds a locally installed Firefox;
- creates an isolated Moth/PokéIdle profile directory;
- launches Firefox with `-no-remote` and that profile;
- opens `https://pokeidle.io/app` by default.

There is **no embedded browser**, WebView runtime, custom userscript engine, or forked userscript-manager code here.

The next layer will provision the existing userscript manager in these dedicated browser profiles and then add browser/process lifecycle management.

## Build

From this directory:

    cargo build

Run Account A:

    cargo run -- --profile AccountA

Run Account B:

    cargo run -- --profile AccountB

A custom Firefox executable can be supplied with:

    cargo run -- --firefox "C:\Program Files\Mozilla Firefox\firefox.exe"
