#!/usr/bin/env python3
"""Extract a Chrome CRX2/CRX3 package without third-party dependencies."""

from __future__ import annotations

import argparse
import io
import json
import shutil
import struct
import sys
import zipfile
from pathlib import Path, PurePosixPath

CRX_MAGIC = b"Cr24"
ZIP_MAGICS = (b"PK\x03\x04", b"PK\x05\x06", b"PK\x07\x08")


def zip_offset(data: bytes) -> tuple[int, str]:
    if data.startswith(ZIP_MAGICS):
        return 0, "zip"

    if len(data) < 12 or data[:4] != CRX_MAGIC:
        raise ValueError("not a CRX/ZIP file (missing Cr24/PK signature)")

    version = struct.unpack_from("<I", data, 4)[0]

    if version == 2:
        if len(data) < 16:
            raise ValueError("truncated CRX2 header")
        public_key_len, signature_len = struct.unpack_from("<II", data, 8)
        offset = 16 + public_key_len + signature_len
    elif version == 3:
        header_len = struct.unpack_from("<I", data, 8)[0]
        offset = 12 + header_len
    else:
        raise ValueError(f"unsupported CRX version {version}")

    if offset >= len(data) or data[offset : offset + 4] not in ZIP_MAGICS:
        raise ValueError("CRX header is invalid or ZIP payload is missing")

    return offset, f"crx{version}"


def safe_members(zf: zipfile.ZipFile) -> list[zipfile.ZipInfo]:
    members: list[zipfile.ZipInfo] = []
    for info in zf.infolist():
        path = PurePosixPath(info.filename)
        if path.is_absolute() or ".." in path.parts:
            raise ValueError(f"unsafe archive path: {info.filename}")
        members.append(info)
    return members


def manifest_from_zip(zf: zipfile.ZipFile) -> dict:
    try:
        raw = zf.read("manifest.json")
    except KeyError as exc:
        raise ValueError("manifest.json is missing") from exc
    return json.loads(raw.decode("utf-8-sig"))


def main() -> int:
    here = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(
        description="Extract /hunt for PokeIdle's committed CRX package."
    )
    parser.add_argument(
        "package",
        nargs="?",
        type=Path,
        default=here / "hunt-for-pokeidle-1.0.17.crx",
        help="CRX/ZIP path (default: %(default)s)",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=here / "source",
        help="output directory (default: %(default)s)",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="print archive file names without extracting",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="replace an existing output directory",
    )
    parser.add_argument(
        "--expected-version",
        default="1.0.17",
        help="fail if manifest.json has a different version",
    )
    args = parser.parse_args()

    data = args.package.read_bytes()
    offset, container = zip_offset(data)

    with zipfile.ZipFile(io.BytesIO(data[offset:])) as zf:
        members = safe_members(zf)
        manifest = manifest_from_zip(zf)
        actual_version = str(manifest.get("version", ""))

        if args.expected_version and actual_version != args.expected_version:
            raise ValueError(
                f"manifest version {actual_version!r} != expected {args.expected_version!r}"
            )

        print(
            f"{args.package.name}: {container}, "
            f"extension={manifest.get('name')!r}, version={actual_version}, "
            f"files={len(members)}"
        )

        if args.list:
            for info in members:
                print(info.filename)
            return 0

        output = args.output.resolve()
        if output.exists():
            if not args.force:
                raise FileExistsError(
                    f"{output} already exists; pass --force to replace it"
                )
            shutil.rmtree(output)

        output.mkdir(parents=True)
        for info in members:
            target = output.joinpath(*PurePosixPath(info.filename).parts)
            if info.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with zf.open(info) as src, target.open("wb") as dst:
                shutil.copyfileobj(src, dst)

    print(f"extracted to {output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, zipfile.BadZipFile, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
