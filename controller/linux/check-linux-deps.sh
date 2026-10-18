#!/usr/bin/env bash
# Diagnostic only: does not install packages, access secrets, or start Firefox.
set -uo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
failed=0

required_command() {
  local name="$1" package="$2"
  if command -v "$name" >/dev/null 2>&1; then
    printf '[OK] %s\n' "$name"
  else
    printf '[MISSING] %s (Ubuntu/Debian: %s)\n' "$name" "$package"
    failed=1
  fi
}

printf 'Moth Controller — Linux dependency check\n'
if [[ -n "${MOTH_FIREFOX:-}" ]]; then
  if [[ -f "$MOTH_FIREFOX" ]] || command -v "$MOTH_FIREFOX" >/dev/null 2>&1; then
    printf '[OK] Firefox override: %s\n' "$MOTH_FIREFOX"
  else
    printf '[MISSING] MOTH_FIREFOX does not resolve to an executable: %s\n' "$MOTH_FIREFOX"
    failed=1
  fi
else
  firefox_found=0
  for name in firefox-developer-edition firefox-developer firefox-dev firefox firefox-bin; do
    if command -v "$name" >/dev/null 2>&1; then
      printf '[OK] Firefox: %s\n' "$name"
      firefox_found=1
      break
    fi
  done
  if (( !firefox_found )); then
    printf '[MISSING] Firefox (or set MOTH_FIREFOX=/path/to/firefox)\n'
    failed=1
  fi
fi

required_command curl curl
required_command xdg-open xdg-utils
required_command secret-tool libsecret-tools

if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
  printf '[INFO] No X11/Wayland display detected. GUI mode requires a desktop session.\n'
fi

if [[ -f "$here/moth-controller" ]]; then
  if command -v ldd >/dev/null 2>&1; then
    if ! libraries="$(ldd "$here/moth-controller" 2>&1)"; then
      printf '[MISSING] Could not inspect binary shared libraries:\n%s\n' "$libraries"
      failed=1
    else
      missing="$(printf '%s\n' "$libraries" | grep 'not found' || true)"
      if [[ -n "$missing" ]]; then
        printf '[MISSING] Shared libraries:\n%s\n' "$missing"
        printf 'On Ubuntu/Debian, check libsqlite3-0, libgtk-3-0, libgl1 and X11/Wayland libraries.\n'
        failed=1
      else
        printf '[OK] Binary shared-library dependencies\n'
      fi
    fi
  else
    printf '[INFO] ldd unavailable; shared libraries not checked\n'
  fi
else
  printf '[INFO] No moth-controller binary beside this script; skipped library check.\n'
fi

printf '\nSecret Service requires an unlocked desktop keyring (GNOME Keyring, KWallet, etc.).\n'
printf 'This check does not read stored credentials or verify keyring unlock state.\n'
if (( failed )); then
  printf '\nSome requirements are missing. Install them, then run this check again.\n'
  exit 1
fi
printf '\nRequired commands are present. Runtime login still needs real-world verification.\n'
