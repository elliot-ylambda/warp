#!/usr/bin/env bash
set -euo pipefail
[[ -n "${WARP_TERMINAL_SESSION_UUID:-}" ]] || exit 0
warp-agent-resume remove "$WARP_TERMINAL_SESSION_UUID"
