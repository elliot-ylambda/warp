#!/usr/bin/env bash
# Installs the agent-resume capture layer (claude wrapper + codex hooks + registry CLI)
# into your shell so that running `claude`/`codex` inside Warp records a resumable
# session per pane. The Rust side of Warp reads ~/.warp/agent-resume/<pane_uuid>.json
# on restore and re-runs the captured command.
#
# Safe to re-run (idempotent). macOS, zsh.
set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
BIN="$HOME/.warp/agent-resume-bin"
REG="$HOME/.warp/agent-resume"

mkdir -p "$BIN" "$REG"
chmod 700 "$REG"

install -m 0755 "$SRC/warp-agent-resume" "$SRC/codex-session-start.sh" "$SRC/codex-session-end.sh" "$BIN/"
install -m 0644 "$SRC/claude.zsh" "$BIN/claude.zsh"

# Wire ~/.zshrc (PATH for the CLI + source the claude wrapper) once.
marker="# >>> warp agent-resume >>>"
if ! grep -qF "$marker" "$HOME/.zshrc" 2>/dev/null; then
  {
    echo ""
    echo "$marker"
    echo "export PATH=\"\$HOME/.warp/agent-resume-bin:\$PATH\""
    echo "source \"\$HOME/.warp/agent-resume-bin/claude.zsh\""
    echo "# <<< warp agent-resume <<<"
  } >> "$HOME/.zshrc"
  echo "Added agent-resume block to ~/.zshrc"
else
  echo "~/.zshrc already wired (skipping)"
fi

# Wire ~/.codex/config.toml hooks once (paths point at the installed bin).
CODEX_CFG="$HOME/.codex/config.toml"
if [[ -f "$CODEX_CFG" ]] && grep -qF "agent-resume-bin/codex-session-start.sh" "$CODEX_CFG"; then
  echo "~/.codex/config.toml already wired (skipping)"
else
  mkdir -p "$HOME/.codex"
  cat >> "$CODEX_CFG" <<EOF

# >>> warp agent-resume >>>
[[hooks.SessionStart]]
matcher = "startup|resume"
[[hooks.SessionStart.hooks]]
type = "command"
command = "$BIN/codex-session-start.sh"

[[hooks.SessionEnd]]
[[hooks.SessionEnd.hooks]]
type = "command"
command = "$BIN/codex-session-end.sh"
# <<< warp agent-resume <<<
EOF
  echo "Added agent-resume hooks to ~/.codex/config.toml"
fi

echo ""
echo "Done. Requirements: jq, uuidgen (uuidgen is preinstalled on macOS; 'brew install jq' if missing)."
echo "Restart your shell (or 'source ~/.zshrc') so the claude wrapper takes effect."
echo "Note: the wrapper only records when launched inside a Warp pane (WARP_TERMINAL_SESSION_UUID set)."
