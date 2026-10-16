# Native visual assets

## Wired upstream icon catalog

Moth Controller includes a checked-in copy of the public `assets/items-icons.json` from
`MOTHblank/pokeidle-upstream` (snapshot 2026-10-16, revision
`264aac7169ed21c27faf7ca2ffc8eee4368e9bc3`):

- `controller/assets/visual/upstream-item-icons.json` maps **419 real item IDs** to public game icon paths (395 PNG, 24 GIF).
- `controller/src/native_visuals.rs` validates and resolves those paths under `https://pokeidle.io/assets/`, alongside the existing five Poké Ball icons and Golden Potion.
- **Options → Download game icons** queues those assets for a single asynchronous worker. Images are decoded only as needed by the UI and stored in the per-user disk cache; downloads never include account credentials.
- The Dashboard inventory, Poké Ball usage, and Auto Catch/Restock pickers display available images with existing text fallbacks. GIF icons display a still first frame.
- The build now enables image PNG and GIF decoding with a synchronized Cargo lockfile. Missing URLs or unsupported images never prevent gameplay.

**No Pokémon sprites were falsely mapped.** The updated upstream `image-index.json`
contains empty `sprites`, `items`, and `balls` arrays, and its scrape manifest
reports `imageFileCount: 0`. The published `assets/creatures-sprites-lab.json`
contains looktypes and creature IDs, but not standalone, verified species sprite
image URLs. The native Pokémon art path therefore remains unmapped until actual
images and their correct species IDs are provided.

The existing `manifest.json` and `native_visual_assets.generated.rs` still describe
**embedded** art, which remains empty. That is intentional: the new mapped icons
download on request, rather than embedding private-upstream binary blobs in a
public release. No downloaded binary images were added to this repository.

## Optional embedded cache generation

Moth Controller has a reusable egui sprite renderer, currently used by the native **Auto Catch / Restock** picker. It supports ball, item, and Pokémon species IDs for reusing the same cache in other screens. The current simplified native navigation no longer exposes the old Inventory and Pokémon pages; those integrations can reuse this module when the pages return. It never requires a WebView.

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

The current decoder intentionally supports PNG only, to keep the native binary and dependency graph lightweight. No sprite images are required to compile; when `manifest.json` is empty, images can still populate the runtime cache as they appear.

## Maintenance

On new game releases, re-run the synchronizer with `--refresh`, review the SHA-256 changes, and commit updated images and generated code. No downloading takes place during a normal CI build.

Run `cargo test` in `controller/` to verify native image URL validation and sprite ID fallbacks.
