#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
export WARP_AGENT_RESUME_DIR="$(mktemp -d)/agent-resume"
CLI="$HERE/warp-agent-resume"

"$CLI" write deadbeef "claude --resume abc-123" "/tmp/proj"
f="$WARP_AGENT_RESUME_DIR/deadbeef.json"
[[ -f "$f" ]] || { echo "FAIL: file not created"; exit 1; }
grep -q '"command": "claude --resume abc-123"' "$f" || { echo "FAIL: command missing"; exit 1; }
grep -q '"cwd": "/tmp/proj"' "$f" || { echo "FAIL: cwd missing"; exit 1; }
perms="$(stat -f '%Lp' "$f")"; [[ "$perms" == "600" ]] || { echo "FAIL: file perms $perms"; exit 1; }
dperms="$(stat -f '%Lp' "$WARP_AGENT_RESUME_DIR")"; [[ "$dperms" == "700" ]] || { echo "FAIL: dir perms $dperms"; exit 1; }

"$CLI" remove deadbeef
[[ ! -f "$f" ]] || { echo "FAIL: file not removed"; exit 1; }
"$CLI" remove deadbeef   # must be idempotent / no error
echo "PASS"
