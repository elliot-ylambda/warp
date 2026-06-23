# Agent session resume on pane restore

Make Warp re-launch your Claude Code / Codex sessions when it restores tabs after a
quit/relaunch. Warp restores each pane's layout + cwd as usual; this adds: capture the
agent session that was running in each pane, and on restore re-run its exact resume
command (`claude --resume <id>` / `codex resume <id>`) once the shell finishes booting.

macOS only. Personal build.

## How it works

```
capture (shell, user-space)                 replay (Rust, in Warp)
  claude wrapper  ─┐                          snapshot(): read registry[pane_uuid]
  codex hooks     ─┼─► ~/.warp/agent-resume/    → freeze command into the pane snapshot
                   │     <pane_uuid>.json        → persisted in SQLite (terminal_panes)
                   │     { "command", "cwd" }            │
                   └────────────────────────►   restore: after the shell's first
                                                 Bootstrapped event, run the command
```

- **Key = the pane UUID** (`WARP_TERMINAL_SESSION_UUID`), which is stable across
  quit/restore and unique per tab — so multiple agents in the *same directory* are
  disambiguated (a directory-based scheme can't do that).
- **Warp stays agent-agnostic**: Rust only stores/replays an opaque command string.
  Adding another agent later is just another capture script — no Rust change.

## Install (capture layer)

```bash
./tools/agent-resume/install.sh
# then restart your shell (or: source ~/.zshrc)
```

Installs the `claude` wrapper + Codex `SessionStart`/`SessionEnd` hooks + the registry
CLI into `~/.warp/agent-resume-bin/`, and wires `~/.zshrc` and `~/.codex/config.toml`.
Requires `jq` and `uuidgen` (uuidgen ships with macOS; `brew install jq` if needed).

The wrapper/hooks only record when launched inside a Warp pane.

## Build the app (replay layer)

```bash
./tools/agent-resume/build-app.sh
```

Builds the **OSS-channel** Warp with this feature compiled in, names it "Warp (Elliot)",
and installs it to `/Applications`. It co-installs alongside your downloaded Warp:
different bundle id (`dev.warp.WarpOss` vs `dev.warp.Warp-Stable`) and a separate data
dir (`~/.warp-oss`), so the two never clobber each other's session state.

## What survives what

| Scenario | Resumes? |
|---|---|
| `Cmd-Q` / update-and-restart | ✅ yes |
| Crash | ⚠️ best-effort — Warp has **no periodic autosave**; pane state is saved on UI mutations and flushed at quit (`on_will_terminate`). A crash recovers only what was last saved. No worse than today. |
| Machine reboot | resumes the *conversation* (`--resume`), never the live process — that's physics, not a bug. |

## Known limitations

- **`claude --continue` / the interactive picker is not captured** (we can't know the
  chosen id up front). Fresh starts and explicit `claude --resume <id>` are covered.
  The wrapper forces a known id on fresh starts via `claude --session-id <uuid>`.
- **Graceful-exit behavior:** the registry entry is *not* removed when an agent exits
  cleanly (only overwritten on the next launch in that pane). This is the safe default
  — it guarantees the entry is present when Warp snapshots at quit. The cost is that a
  session you closed may reopen on the next restore. (Removing on exit risks the
  opposite failure: the entry vanishing before Warp snapshots. Revisit only if Warp
  gains a guaranteed quit-time snapshot ordering.)

## Files

| File | Role |
|---|---|
| `warp-agent-resume` | registry CLI: `write <uuid> <cmd> <cwd>` / `remove <uuid>` |
| `claude.zsh` | `claude()` wrapper — capture on fresh start / explicit resume |
| `codex-session-start.sh` / `codex-session-end.sh` | Codex hooks |
| `config.toml.snippet` | Codex hook registration (installer applies it) |
| `install.sh` | install capture layer into the shell |
| `build-app.sh` | build + brand + install the co-installable app |
| `tests/` | self-contained bash tests for the scripts |

Rust side: `app/src/agent_resume.rs` (registry reader), the `on_restore_command` field
on `TerminalPaneSnapshot`, the `terminal_panes.on_restore_command` column, and the
replay in `app/src/pane_group/mod.rs` + `pty_controller.rs`.
