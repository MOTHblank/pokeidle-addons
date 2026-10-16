#!/usr/bin/env python3
"""One-time, deterministic visual cache synchronizer for Moth Controller.

Run from the repository root:
  python scripts/sync-native-visuals.py
  python scripts/sync-native-visuals.py --all-items
  python scripts/sync-native-visuals.py --pokemon-index /path/to/sprites.json

The generated cache and native_visual_assets.generated.rs can be committed to
Git. The native app embeds the resulting images at compile time; no runtime
HTTP request is needed for those sprites. Respect game/artwork rights before
redistributing generated assets.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
ASSET_DIR = ROOT / "controller" / "assets" / "visual"
UPSTREAM_PATHS = ASSET_DIR / "upstream-available-images.json"
UPSTREAM_ITEMS = ASSET_DIR / "upstream-item-icons.json"
MANIFEST = ASSET_DIR / "manifest.json"
GENERATED = ROOT / "controller" / "src" / "native_visual_assets.generated.rs"
CATALOG_URL = "https://pokeidle.io/assets/items.json"
MAX_BYTES = 1024 * 1024
BALLS = {
    1: "/assets/site/assets/ui/ball-poke.png",
    2: "/assets/site/assets/ui/ball-great.png",
    3: "/img/ball-super.png",
    4: "/assets/site/assets/ui/ball-ultra.png",
    5: "/img/ball-beast.png",
}
HEALING = {
    200: "/assets/markitems/small_potion.png",
    201: "/assets/markitems/great_potion.png",
    202: "/assets/markitems/ultra_potion.png",
    203: "/assets/markitems/hyper_potion.png",
    204: "/assets/markitems/ultimate_potion.png",
    205: "/assets/markitems/revive.png",
    206: "/assets/markitems/max_revive.png",
    70070: "/img/itens/golden-potion.png",
}


def candidates(path: str, allow_external: bool = False) -> list[str]:
    """Allowed HTTPS origins only, never arbitrary paths or credentials."""
    if not isinstance(path, str) or not path or ".." in path or "\\" in path:
        return []
    parsed = urllib.parse.urlsplit(path)
    if parsed.query or parsed.fragment or parsed.username or parsed.password:
        return []
    if parsed.scheme or parsed.netloc:
        if parsed.scheme != "https":
            return []
        if parsed.hostname not in ({"pokeidle.io", "www.pokeidle.io"} |
                                   ({"pokexguides.com", "www.pokexguides.com"}
                                    if allow_external else set())):
            return []
        return [path]
    if not path.startswith("/") or path.startswith("//"):
        return []
    # Both paths appear in upstream and Auto Catch+; fall back if the first
    # mirror is missing. A response must be an actual image, not HTML.
    paths = [path]
    if path.startswith("/assets/markitems/"):
        paths.append("/assets/site" + path)
        paths.append("/assets/site/assets/markitems/" + path.rsplit("/", 1)[-1])
    if path.startswith("/assets/site/assets/"):
        paths.append(path.replace("/assets/site", "", 1))
    return list(dict.fromkeys("https://pokeidle.io" + p for p in paths))


def image_extension(raw: bytes) -> str | None:
    # egui's image crate decodes static PNGs and the first frame of GIFs.
    if raw.startswith(bytes.fromhex("89504e470d0a1a0a")):
        return "png"
    if raw.startswith((b"GIF87a", b"GIF89a")):
        return "gif"
    return None


def upstream_relative(origin: str) -> str | None:
    """Convert a trusted game image path into a repository snapshot file."""
    url = urllib.parse.urlsplit(origin)
    if url.scheme or url.netloc:
        if url.scheme != "https" or url.hostname not in (
            "pokeidle.io", "www.pokeidle.io"
        ):
            return None
        raw = url.path
    else:
        raw = origin
    if not raw.startswith("/") or ".." in raw or "\\" in raw or "?" in raw:
        return None
    path = raw.lstrip("/")
    if not (path.startswith("assets/site/assets/") or path.startswith("img/")):
        return None
    if not path.endswith((".png", ".gif")):
        return None
    return path


def read_upstream_asset(directory: Path, origin: str,
                        available: set[str]) -> bytes | None:
    path = upstream_relative(origin)
    if path is None or path not in available:
        return None
    target = (directory / path).resolve()
    root = directory.resolve()
    if not target.is_relative_to(root) or not target.is_file():
        return None
    raw = target.read_bytes()
    if len(raw) > MAX_BYTES or image_extension(raw) is None:
        return None
    return raw


def get(url: str) -> bytes:
    req = urllib.request.Request(
        url, headers={"User-Agent": "MothControllerVisualSync/1.0",
                      "Accept": "image/png,image/gif,image/webp,application/json"})
    # urllib follows redirects; constrain the final origin too.
    with urllib.request.urlopen(req, timeout=12) as response:
        destination = urllib.parse.urlsplit(response.url)
        source = urllib.parse.urlsplit(url)
        if destination.scheme != "https" or destination.hostname != source.hostname:
            raise ValueError("redirected to a different image host")
        if int(response.headers.get("Content-Length") or 0) > MAX_BYTES:
            raise ValueError("asset exceeded size limit")
        raw = response.read(MAX_BYTES + 1)
    if len(raw) > MAX_BYTES:
        raise ValueError("asset exceeded size limit")
    return raw


def load_catalog(source: str | None) -> dict:
    if source:
        return json.loads(Path(source).read_text(encoding="utf-8"))
    return json.loads(get(CATALOG_URL))


def collect(args: argparse.Namespace) -> list[tuple[str, int, str]]:
    sources: dict[tuple[str, int], str] = {
        ("ball", ident): path for ident, path in BALLS.items()
    }
    sources.update({("item", ident): path for ident, path in HEALING.items()})
    if args.upstream_dir:
        # Exact item ID mapping from the same upstream snapshot as the images.
        # No network calls and no guessed paths or IDs.
        catalog = json.loads(UPSTREAM_ITEMS.read_text(encoding="utf-8"))
        for key, icon in catalog.items():
            try:
                ident = int(key)
            except (ValueError, TypeError):
                continue
            if ident > 0 and isinstance(icon, str):
                sources[("item", ident)] = "/assets/" + icon if icon.startswith(
                    "site/assets/"
                ) else "/" + icon
    if args.all_items or args.catalog:
        catalog = load_catalog(args.catalog)
        for item in catalog.get("items", []):
            try:
                ident = int(item["id"])
                icon = item["icon"]
            except (ValueError, TypeError, KeyError):
                continue
            if ident > 0 and isinstance(icon, str):
                sources.setdefault(("item", ident), icon)
    if args.pokemon_index:
        pokemon_index = json.loads(Path(args.pokemon_index).read_text(encoding="utf-8"))
        entries = pokemon_index.get("sprites", []) if isinstance(pokemon_index, dict) else pokemon_index
        for entry in entries:
            try:
                ident = int(entry["id"])
                icon = entry["icon"]
            except (ValueError, TypeError, KeyError):
                continue
            if ident > 0 and isinstance(icon, str):
                sources[("pokemon", ident)] = icon
    return [(kind, ident, url) for (kind, ident), url in sorted(sources.items())]


def generate(entries: list[dict]) -> None:
    lines = [
        "// Generated by scripts/sync-native-visuals.py; commit with assets.",
        "// Each include_bytes! uses an actual checked-in image.",
        "pub fn bytes(kind: &str, id: u64) -> Option<&'static [u8]> {",
        "    match (kind, id) {",
    ]
    for e in sorted(entries, key=lambda item: (item["kind"], item["id"])):
        path = "/assets/visual/" + e["file"]
        lines.append(f'        ("{e["kind"]}", {e["id"]}) => Some(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "{path}"))),')
    lines.extend(["        _ => None,", "    }", "}", ""])
    GENERATED.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all-items", action="store_true",
                        help="also retrieve catalog icons for all game items")
    parser.add_argument("--catalog", help="read a local items.json instead of fetching it")
    parser.add_argument("--pokemon-index", help="JSON file with sprites: [{id, icon}]")
    parser.add_argument("--allow-external", action="store_true",
                        help="allow known public third-party item-art hosts")
    parser.add_argument("--refresh", action="store_true",
                        help="download again instead of reusing the local cache")
    parser.add_argument("--dry-run", action="store_true",
                        help="report the asset list without network access or writing")
    parser.add_argument("--upstream-dir", type=Path,
                        help="read verified artwork from a local pokeidle-upstream checkout")
    parser.add_argument("--local-only", action="store_true",
                        help="do not request missing artwork from the game CDN")
    args = parser.parse_args()
    if args.local_only and not args.upstream_dir:
        parser.error("--local-only requires --upstream-dir")
    if args.upstream_dir and not args.upstream_dir.is_dir():
        parser.error("--upstream-dir must point to an existing checkout")
    assets = collect(args)
    if args.dry_run:
        for kind, ident, path in assets:
            print(f"{kind:8} {ident:7} {path}")
        return 0

    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    prior = {}
    if MANIFEST.exists():
        try:
            prior = {(e["kind"], int(e["id"])): e
                     for e in json.loads(MANIFEST.read_text(encoding="utf-8")).get("assets", [])}
        except (ValueError, KeyError, TypeError):
            pass

    available = set()
    if args.upstream_dir:
        data = json.loads(UPSTREAM_PATHS.read_text(encoding="utf-8"))
        available = set(data.get("files", []))
    successful: dict[tuple[str, int], dict] = {}
    for kind, ident, origin in assets:
        key = (kind, ident)
        old = prior.get(key)
        if old and not args.refresh:
            existing = ASSET_DIR / old["file"]
            if existing.is_file() and hashlib.sha256(existing.read_bytes()).hexdigest() == old["sha256"]:
                successful[key] = old
                continue
        urls = candidates(origin, args.allow_external)
        local_raw = (read_upstream_asset(args.upstream_dir, origin, available)
                     if args.upstream_dir else None)
        for url in urls:
            try:
                raw = local_raw
                if raw is None:
                    if args.local_only:
                        break
                    raw = get(url)
                ext = image_extension(raw)
                if ext is None:
                    raise ValueError("unsupported image format / not image content")
                filename = f"{kind}-{ident}.{ext}"
                (ASSET_DIR / filename).write_bytes(raw)
                successful[key] = {"kind": kind, "id": ident, "file": filename,
                                   "source": ("pokeidle-upstream:" + upstream_relative(origin)
                                              if local_raw is not None else url),
                                   "sha256": hashlib.sha256(raw).hexdigest()}
                print(f"OK  {kind} #{ident}: {filename} ({len(raw)} bytes)")
                break
            except (ValueError, OSError, urllib.error.URLError) as error:
                print(f"ERR {kind} #{ident} {url}: {error}", file=sys.stderr)
        if key not in successful:
            print(f"SKIP {kind} #{ident}: no valid image", file=sys.stderr)

    # Do not discard previously cached entries when running a curated sync.
    for key, entry in prior.items():
        if key in successful or not (ASSET_DIR / entry["file"]).is_file():
            continue
        successful[key] = entry
    contents = {"schema": 1, "assets": sorted(successful.values(),
                  key=lambda e: (e["kind"], e["id"]))}
    MANIFEST.write_text(json.dumps(contents, indent=2, ensure_ascii=False) + "\n",
                        encoding="utf-8")
    generate(contents["assets"])
    print(f"Bundled {len(successful)} sprites; commit controller/assets/visual "
          "and controller/src/native_visual_assets.generated.rs")
    return 0 if successful else 1


if __name__ == "__main__":
    sys.exit(main())
