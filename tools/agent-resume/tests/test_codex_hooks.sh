#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
export WARP_AGENT_RESUME_DIR="$TMP/reg"
export PATH="$HERE:$PATH"
export WARP_TERMINAL_SESSION_UUID="bb22"

echo '{"session_id":"sess-77","cwd":"/tmp/repo","source":"startup"}' | bash "$HERE/codex-session-start.sh"
f="$WARP_AGENT_RESUME_DIR/bb22.json"
grep -q '"command": "codex resume sess-77"' "$f" || { echo "FAIL: start"; exit 1; }
grep -q '"cwd": "/tmp/repo"' "$f" || { echo "FAIL: cwd"; exit 1; }

echo '{"session_id":"sess-77","cwd":"/tmp/repo"}' | bash "$HERE/codex-session-end.sh"
[[ ! -f "$f" ]] || { echo "FAIL: end did not remove"; exit 1; }

# No-op outside a Warp pane.
unset WARP_TERMINAL_SESSION_UUID
echo '{"session_id":"x","cwd":"/tmp"}' | bash "$HERE/codex-session-start.sh"
[[ -z "$(ls -A "$WARP_AGENT_RESUME_DIR" 2>/dev/null)" ]] || { echo "FAIL: wrote outside pane"; exit 1; }
echo "PASS"
