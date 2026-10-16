# Native visual assets

## Wired upstream icon catalog

Moth Controller includes a checked-in copy of the public `assets/items-icons.json` from
`MOTHblank/pokeidle-upstream` (snapshot 2026-10-16, revision
`264aac7169ed21c27faf7ca2ffc8eee4368e9bc3`):

- `controller/assets/visual/upstream-item-icons.json` maps **419 real item IDs** to public game icon paths (395 PNG, 24 GIF).
- `controller/src/native_visuals.rs` validates and resolves those paths under `https://pokeidle.io/assets/`, alongside the existing five Poké Ball icons and Golden Potion.
- Visible item icons are requested on demand, asynchronously cached on the user's disk, and never include account credentials.
- The Dashboard inventory, Poké Ball usage, and Auto Catch/Restock pickers display available images with existing text fallbacks. GIF icons display a still first frame.
- The build now enables image PNG and GIF decoding with a synchronized Cargo lockfile. Missing URLs or unsupported images never prevent gameplay.

## Pokémon sprites on Overview cards

The player-versus-opponent artwork is now resolved from the same real game
metadata as `DiuleJR/pokeidle-manager` (reference: GPL-3.0-only
`src-tauri/src/assets.rs` and `src/inventory/asset-components.tsx`):

- Owned Pokémon use their actual `looktype` (or `lookShiny` when shiny).
  They **do not** use the species ID as a sprite looktype.
- Wild opponents expose `speciesId`. On first use, the native cache resolves
  those IDs using `creatures.json`, `creatures-novos.json`,
  `creatures-outland-novos.json`, and `creatures-sprites-lab.json`.
- Both sprites prefer the real outfit frames from the game's
  `outfits-index.json` and per-looktype manifests. Up to twelve verified
  animation rectangles are cropped from the cached sheet on a background
  worker; the native cards advance them at 250 ms only while the window has
  focus. If the outfit has no usable frames, the game's verified
  `marker-atlas.json` slot provides a static icon.
- All metadata and sheets are downloaded **lazily** on background workers,
  and cached on the player's disk. Decoded frame textures are reused by egui;
  the app doesn't bundle game sprite sheets. Previously cached static images
  remain usable if the remote asset service fails.
- Failed requests can be retried after a two-minute cooldown instead of
  showing permanent missing-art placeholders until an application restart.
  If no sprite is available, cards retain Pokémon names, levels, combat
  state and their small placeholder symbols. No guessed URLs are used.

This ports sprite *resolution* and cropping behavior, not third-party game
artwork. The game is the only runtime art origin and redistribution of art
is not implied by public access. The standalone embedded sprite manifest
remains intentionally empty.

The existing `manifest.json` and `native_visual_assets.generated.rs` still describe
**embedded** art, which remains empty. That is intentional: the new mapped icons
download on request, rather than embedding private-upstream binary blobs in a
public release. No downloaded binary images were added to this repository.

## Optional embedded cache generation

Moth Controller has a reusable egui sprite renderer, currently used by the native **Auto Catch / Restock** picker. It supports ball and item IDs, plus distinct Pokémon species and looktype keys, for reuse across native views. The current simplified native navigation no longer exposes the old Inventory and Pokémon pages; those integrations can reuse this module when the pages return. It never requires a WebView.

## Wiring the current pokeidle-upstream image snapshot

The controller tracks the upstream game's **419 item ID → icon-path mappings** in `upstream-item-icons.json`. The new `upstream-available-images.json` records **87 image files confirmed in the upstream repository snapshot**, not guessed CDN paths. Moth Controller first tries the game-hosted image URL and then falls back to the exact corresponding public `pokeidle-upstream` image when it exists. Images load on demand into a per-user disk cache without blocking the interface.

To bundle verified artwork once, clone or update `MOTHblank/pokeidle-upstream` locally next to this repository and run from the `pokeidle-addons` repository root:

```sh
python scripts/sync-native-visuals.py --upstream-dir ../pokeidle-upstream --local-only
```

This command uses the checked-in mappings and copies only PNG/GIF files that exist at verified snapshot paths. It produces `controller/assets/visual/manifest.json` and `controller/src/native_visual_assets.generated.rs`; after reviewing distribution rights, commit those together with `controller/assets/visual/*.{png,gif}`. To additionally ask the game CDN for icons absent from the repository snapshot, omit `--local-only`.

The upstream `image-index.json` has no standalone sprite URLs, but Pokémon rendering no longer depends on that index: the game's looktypes, marker atlas and outfit manifests are now used by the native Overview.

## Developer: create the committed art cache once

From the repository root, with Python 3.10+ and internet access:

```sh
python scripts/sync-native-visuals.py
# Optional: include other catalog inventory artwork.
python scripts/sync-native-visuals.py --all-items
# Optional: include Pokémon sprites from an explicit ID-to-URL JSON index.
python scripts/sync-native-visuals.py --pokemon-index pokemon-sprites.json
```

The synchronizer obtains PokéIdle's sprites and public item metadata (for the optional full catalog), validates PNG signatures and size, and writes:

- `controller/assets/visual/*.png`
- `controller/assets/visual/manifest.json` (source URLs, IDs, SHA-256)
- `controller/src/native_visual_assets.generated.rs` (compile-time `include_bytes!` map)

Review asset licenses/permissions before redistribution. **Game assets are not automatically licensed for inclusion in this repository merely because they are publicly viewable.** The sync intentionally does not commit artwork. After reviewing assets, commit both the PNGs, manifest and generated Rust file together. Do not commit Pokémon artwork without confirming redistribution rights.

To inspect available sources without fetching, run:

```sh
python scripts/sync-native-visuals.py --dry-run
```

The Pokémon JSON index format is:

```json
{"sprites": [{"id": 25, "icon": "https://pokeidle.io/path/to/pikachu.png"}]}
```

These IDs are the **species IDs** reported by the game. We don't guess sprite URLs or reuse combat looktypes as species IDs.

`--refresh` re-downloads, while repeated runs preserve valid cached files. `--allow-external` permits the game's known third-party item host, which should be reviewed separately.

## Runtime

**Bundled first**: the generated Rust file embeds checked-in PNGs into the controller executable; no copied assets directory is required with releases. Each image is decoded and uploaded as an egui texture only when first displayed. Subsequent frames reuse GPU textures with no file I/O.

**Fallback for missing assets**: the native controller may asynchronously fetch trusted game-origin PNGs when needed, through one bounded worker, and keep them in a per-user disk cache:

- Windows: `%LOCALAPPDATA%/Moth/PokeIdle/VisualCache`
- Linux: `$XDG_CACHE_HOME/moth-controller/visual`, or `~/.cache/moth-controller/visual`

This process has no access to game credentials and uses the public static image origin only. HTTPS is required; redirects, traversal and untrusted absolute URLs are rejected. Downloads and disk writes never block the egui render loop. Unknown sprites fall back to their item/species name and ID. An unavailable art server cannot block native gameplay or restocking.

The asset bundler accepts PNG and GIF for existing item/ball artwork (GIFs display their initial frame); Pokémon index entries must still be verified before import. The decoder supports PNG and GIF, to keep the native binary and dependency graph lightweight. No sprite images are required to compile; when `manifest.json` is empty, images can still populate the runtime cache as they appear.

## Maintenance

On new game releases, re-run the synchronizer with `--refresh`, review the SHA-256 changes, and commit updated images and generated code. No downloading takes place during a normal CI build.

Run `cargo test` in `controller/` to verify native image URL validation and sprite ID fallbacks.
