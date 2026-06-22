#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
export WARP_AGENT_RESUME_DIR="$TMP/reg"
export PATH="$TMP/bin:$HERE:$PATH"
mkdir -p "$TMP/bin"
# Fake `claude` binary that records the args it was invoked with, then exits 0.
cat > "$TMP/bin/claude" <<'EOF'
#!/usr/bin/env bash
echo "$@" > "$WARP_AGENT_RESUME_DIR/../last_args"
exit 0
EOF
chmod +x "$TMP/bin/claude"

# Load wrapper; CLAUDE_REAL_BIN points at the fake to avoid `command -v` picking the function.
export CLAUDE_REAL_BIN="$TMP/bin/claude"
source "$HERE/claude.zsh" 2>/dev/null || . "$HERE/claude.zsh"

# Case A: fresh start in a Warp pane → registry written, --session-id injected
export WARP_TERMINAL_SESSION_UUID="aa11"
( cd "$TMP" && claude )
[[ -f "$WARP_AGENT_RESUME_DIR/aa11.json" ]] || { echo "FAIL A: no registry file"; exit 1; }
grep -q '"command": "claude --resume ' "$WARP_AGENT_RESUME_DIR/aa11.json" || { echo "FAIL A: bad command"; exit 1; }
grep -q -- '--session-id' "$TMP/last_args" || { echo "FAIL A: no --session-id passed"; exit 1; }

# Case B: explicit --resume <id> → registry records that id, no --session-id added
rm -f "$WARP_AGENT_RESUME_DIR/aa11.json"
claude --resume zzz-999
grep -q '"command": "claude --resume zzz-999"' "$WARP_AGENT_RESUME_DIR/aa11.json" || { echo "FAIL B"; exit 1; }

# Case C: --continue → no registry file
rm -f "$WARP_AGENT_RESUME_DIR/aa11.json"
claude --continue
[[ ! -f "$WARP_AGENT_RESUME_DIR/aa11.json" ]] || { echo "FAIL C: continue should not record"; exit 1; }

# Case D: outside a Warp pane → no registry file
unset WARP_TERMINAL_SESSION_UUID
claude
[[ -z "$(ls -A "$WARP_AGENT_RESUME_DIR" 2>/dev/null)" ]] || { echo "FAIL D: recorded outside pane"; exit 1; }
echo "PASS"
