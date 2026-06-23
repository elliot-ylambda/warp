#!/usr/bin/env bash
# Claude Code SessionStart hook: record the *actual* live session for this Warp pane so it
# can be resumed on restore. Fires on a fresh start, `claude --resume <id>`, the interactive
# session picker, and `claude --continue` -- in every case the stdin payload carries the
# real session_id. (The old claude() shell wrapper could only know an id it was given on the
# command line, so it silently missed the picker and --continue and left a stale entry.)
#
# Keyed by the pane UUID, so multiple agents in the same directory stay disambiguated.
# No removal on exit: the entry is overwritten by the next session in this pane, which keeps
# it present when Warp snapshots at quit (see README "Graceful-exit behavior").
set -uo pipefail
[[ -n "${WARP_TERMINAL_SESSION_UUID:-}" ]] || exit 0   # only act inside a Warp pane
payload="$(cat)"
sid="$(printf '%s' "$payload" | jq -r '.session_id // empty')"
cwd="$(printf '%s' "$payload" | jq -r '.cwd // empty')"
[[ -n "$sid" ]] || exit 0
# Call the registry CLI by absolute path (sibling of this script) so the hook does not
# depend on the agent inheriting the shell PATH.
BIN="$(cd "$(dirname "$0")" && pwd)"
"$BIN/warp-agent-resume" write "$WARP_TERMINAL_SESSION_UUID" "warp_agent_resume_launch claude $sid" "$cwd" >/dev/null 2>&1 || true
exit 0
