#!/usr/bin/env bash
# Use an isolated test HOME and an X display. No real mailbox credentials are used.
# On a container, run as a non-root user with --network none.
set -euo pipefail
binary="$1"
binary="$(realpath "$binary")"
test -x "$binary"
test "$(id -u)" != 0 || { echo "Run smoke test as a non-root user" >&2; exit 1; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export HOME="$tmp/home" XDG_DATA_HOME="$tmp/data" XDG_CONFIG_HOME="$tmp/config" XDG_CACHE_HOME="$tmp/cache" XDG_RUNTIME_DIR="$tmp/runtime"
mkdir -p "$HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export MAIL_EX_SMOKE=1
set +e
timeout 35s dbus-run-session -- xvfb-run -a -s '-screen 0 1440x1000x24' "$binary" >"$tmp/smoke.log" 2>&1
code=$?
set -e
cat "$tmp/smoke.log"
grep -q '^MAIL_EX_FRONTEND_READY main$' "$tmp/smoke.log"
test "$code" = 124
echo "Native window startup and main frontend IPC readiness passed"
