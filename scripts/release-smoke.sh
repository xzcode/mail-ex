#!/usr/bin/env bash
# Smoke an optimized binary using a disposable home; no real accounts.
set -euo pipefail
binary="$(realpath "$1")"
test -x "$binary"
test "$(id -u)" != 0 || { echo "Run smoke as non-root"; exit 1; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export HOME="$tmp/home" XDG_DATA_HOME="$tmp/data" XDG_CONFIG_HOME="$tmp/config" XDG_CACHE_HOME="$tmp/cache" XDG_RUNTIME_DIR="$tmp/runtime"
mkdir -p "$HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export TMPDIR="$tmp" MAIL_EX_RELEASE_SMOKE=1
set +e
timeout 20s dbus-run-session -- xvfb-run -a -s '-screen 0 1440x1000x24' "$binary" >"$tmp/startup.log" 2>&1
code=$?
set -e
cat "$tmp/startup.log"
test "$code" = 124
grep -qx 'MAIL_EX_RELEASE_FRONTEND_READY main' "$tmp/startup.log"
echo "Optimized native frontend IPC startup passed"
