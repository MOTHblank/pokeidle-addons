# /hunt for PokeIdle — third-party reference

This directory contains the actual Chrome Web Store package for **/hunt for PokeIdle 1.0.17** and a deterministic extractor for research and Hunt Atlas comparison work.

- Extension: `/hunt for PokeIdle`
- Chrome Web Store ID: `mhnplloljgchpfpcpbhmcokmnefodjnh`
- Package: `hunt-for-pokeidle-1.0.17.crx`
- Version: **1.0.17**
- Package originally committed to this repository by MOTHblank on 2026-11-08
- Original package blob: `d498efc9021426390f90caf4473eaa07f1dfdc4e`

The previous 1.0.12 mirror has been superseded by this package.

## Extract the source

From this directory:

```powershell
python .\extract_crx.py
```

That creates `source/` containing the extension exactly as stored inside the signed CRX archive, including `manifest.json`, JavaScript, CSS, icons and other assets.

To inspect the package without extracting it:

```powershell
python .\extract_crx.py --list
```

To replace an existing extraction:

```powershell
python .\extract_crx.py --force
```

The extractor understands CRX2, CRX3 and plain ZIP input, validates archive paths against zip-slip, requires `manifest.json`, and by default verifies that the manifest version is `1.0.17`.

## Scope

This is **reference-only** third-party code. Moth Controller does not import or execute it. It is kept here so pacing, geometry, combat, XP/h, bicycle handling and other hunt-estimation behavior can be compared against Hunt Atlas using an exact package snapshot.

## Attribution and licensing

The extension is third-party work by Liang. Treat its licensing independently from `pokeidle-addons`. The package is preserved here for reference/audit purposes; do not assume its contents inherit this repository's license.
