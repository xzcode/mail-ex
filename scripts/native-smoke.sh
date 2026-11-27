#!/usr/bin/env bash
# Debug custom-protocol binary; isolated HOME and X display, no real mailbox credentials.
# In Docker, run as a non-root user with --network none.
set -euo pipefail
binary="$(realpath "$1")"
test -x "$binary"
test "$(id -u)" != 0 || { echo "Run smoke test as a non-root user" >&2; exit 1; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export HOME="$tmp/home" XDG_DATA_HOME="$tmp/data" XDG_CONFIG_HOME="$tmp/config" XDG_CACHE_HOME="$tmp/cache" XDG_RUNTIME_DIR="$tmp/runtime"
mkdir -p "$HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export TMPDIR="$tmp" MAIL_EX_SMOKE=1
set +e
timeout 25s dbus-run-session -- xvfb-run -a -s '-screen 0 1440x1000x24' "$binary" --smoke-fixtures >"$tmp/smoke.log" 2>&1
code=$?
set -e
cat "$tmp/smoke.log"
grep -qx 'MAIL_EX_FRONTEND_READY main' "$tmp/smoke.log"
grep -q '^MAIL_EX_FRONTEND_READY compose-' "$tmp/smoke.log"
grep -q '^MAIL_EX_FRONTEND_READY reader-' "$tmp/smoke.log"
if grep -q 'MAIL_EX_SMOKE_.*_ERROR' "$tmp/smoke.log"; then exit 1; fi
test "$code" = 124
echo "Native main, composer and reader startup with real IPC passed"
