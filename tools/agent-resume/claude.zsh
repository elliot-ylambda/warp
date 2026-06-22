# Wrapper that records a resumable Claude session keyed by the Warp pane UUID.
# The registry file persists until the pane's shell exits (or the next invocation
# overwrites it). No intra-function cleanup is done so the entry survives a crash.
claude() {
  local real="${CLAUDE_REAL_BIN:-}"
  if [[ -z "$real" ]]; then
    # Resolve the real binary, skipping this function.
    real="$(whence -p claude 2>/dev/null || command -v -p claude 2>/dev/null || true)"
  fi
  [[ -z "$real" ]] && { echo "claude: real binary not found" >&2; return 127; }

  # Only act inside a Warp pane.
  if [[ -z "${WARP_TERMINAL_SESSION_UUID:-}" ]]; then
    "$real" "$@"; return $?
  fi

  local uuid="$WARP_TERMINAL_SESSION_UUID"
  local args=("$@") resume_id="" mode="fresh"
  local i
  for ((i=1; i<=${#args[@]}; i++)); do
    case "${args[i]}" in
      --resume|-r) resume_id="${args[i+1]:-}"; mode="resume" ;;
      --resume=*)  resume_id="${args[i]#*=}"; mode="resume" ;;
      --continue|-c|--print|-p) mode="passthrough" ;;
      --session-id|--session-id=*) mode="passthrough" ;;
    esac
  done

  case "$mode" in
    passthrough)
      "$real" "$@"; return $?
      ;;
    resume)
      [[ -n "$resume_id" ]] && warp-agent-resume write "$uuid" "claude --resume $resume_id" "$PWD"
      "$real" "$@"; return $?
      ;;
    fresh)
      local sid
      sid="$(uuidgen | tr 'A-Z' 'a-z')"
      warp-agent-resume write "$uuid" "claude --resume $sid" "$PWD"
      "$real" --session-id "$sid" "$@"; return $?
      ;;
  esac
}
