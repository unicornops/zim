#!/usr/bin/env bash
set -euo pipefail

"$1" &
app_pid=$!
trap 'kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true' EXIT

if ! timeout 45s xdotool search --sync --onlyvisible --pid "$app_pid" --name '^Zim$' >/dev/null; then
  echo 'Zim did not open a visible desktop window within 45 seconds.' >&2
  exit 1
fi
