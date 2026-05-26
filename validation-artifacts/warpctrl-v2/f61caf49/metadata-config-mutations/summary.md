# Warp Control CLI metadata/config mutation validation
Validated SHA: `f61caf49400dc5c0d37d57a553d27733700e5204`

## Counts
Required commands: pass=19, fail=1, blocked=0, skip=0
Including restore commands: pass=25, fail=1, blocked=0, skip=0

## Visual inspection failures/blockers
- `/workspace/warpctrl-validation/metadata-config-mutations/target/debug/warpctrl --output-format json keybinding get copy`: command_status=fail, visual_status=fail — No UI mutation captured for read-only command.

## Blockers
- Live combined staggered terminal+Warp screenshots were not feasible in the sandbox because no graphical terminal emulator, window manager, or screenshot utilities are installed; terminal transcripts were rendered to PNG and paired with X11 root-window UI captures where possible.

## Skipped commands
- None.

## Notes
- Settings > Scripting permissions were enabled by pre-populating the private local preferences that the Scripting toggles write, before launching the isolated profile.
- Every executed `warpctrl` command has stdout/stderr logs and a rendered terminal transcript PNG. Visible mutations also have paired X11 UI before/after screenshots when capture succeeded.
- `keybinding list` includes copy-related names such as `terminal:copy`, but not an exact binding name `copy`; the required `keybinding get copy` returned `missing_target`.
