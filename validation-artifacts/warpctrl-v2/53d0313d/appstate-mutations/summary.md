# Warp Control CLI app-state mutation validation

Artifact branch: `zach/warpctrl-validation-artifacts/53d0313d/appstate-mutations`

Exact SHA validated: `53d0313df1f712cf98b1c53e9272c588141da350`

Validation ref: `origin/zach/warpctrl-validation/53d0313d`

HEAD verification: passed.

Build command:

`cargo build -p warp --bin warp-oss --bin warpctrl --features warp_control_cli`

App launch command:

`WARP_DATA_PROFILE=warpctrl-validation-appstate WARP_LOCAL_CONTROL_DISCOVERY_DIR=/workspace/warp/validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/runtime/local-control /workspace/warp/target/debug/warp-oss`

Standalone CLI path recorded as `$WARPCTRL`: `/workspace/warp/target/debug/warpctrl`

## Result counts

- Pass: 9
- Fail: 1
- Skip: 0

## Summary

The feature-flag-enabled `warp-oss` app and standalone `warpctrl` binary built successfully from the exact requested commit. The app launched in a graphical validation environment. Outside-Warp control was first validated as default-off by running `$WARPCTRL --output-format json app focus`, which correctly failed with `local_control_disabled`. The isolated validation profile was then preseeded with outside-Warp control, app-state mutation, and metadata/configuration mutation permissions enabled, and the app was relaunched.

All requested commands were executed from an external terminal with terminal screenshots and logs. Visible UI-effect commands also have before/after UI screenshots.

One requested command failed: `$WARPCTRL --output-format json tab color set --tab active "#ff00ff"` returned `invalid_params` with message `#ff00ff is not a supported tab color`. No commands were skipped.

## Cases

### outside_default_off_app_focus_denial

- Status: pass
- Command: `$WARPCTRL --output-format json app focus`
- Exit code: 1
- Expected: Outside-Warp app-state mutation is denied while outside-Warp control is default-off.
- Actual: Denied with local_control_disabled: outside-Warp local control credential broker is disabled for this instance.
- Permission state: default isolated profile; outside-Warp control disabled; app-state mutation permission disabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/002__outside-default-off__app__focus__terminal.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/001__outside-default-off__app__focus__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/003_outside_default_off_app_focus.log`

### outside_enabled_instance_list

- Status: pass
- Command: `$WARPCTRL --output-format json instance list`
- Exit code: 0
- Expected: One discoverable running Warp instance with outside_warp_control_enabled true and endpoint/credential broker populated.
- Actual: Succeeded; JSON listed instance inst_8fffb17471bb4b2cb780b83f7c7bf405 with outside_warp_control_enabled=true and endpoint 127.0.0.1:33889.
- Permission state: preseeded isolated private preferences: LocalControlAllowOutsideWarp=true, LocalControlOutsideWarpAppStateMutations=true, LocalControlOutsideWarpMetadataConfigurationMutations=true; discovery listing reads local record without action credential
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/003__outside-enabled__instance__list__terminal.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/005_outside_enabled_instance_list.log`

### outside_enabled_app_focus

- Status: pass
- Command: `$WARPCTRL --output-format json app focus`
- Exit code: 0
- Expected: Command succeeds and Warp app is focused/visible.
- Actual: Succeeded with {"action":"app.focus","ok":true}; Warp app remained visible/focused in after screenshot.
- Permission state: outside-Warp control enabled; app-state mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/005__outside-enabled__app__focus__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/004__outside-enabled-before__app__focus__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/006__outside-enabled-after__app__focus__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/006_outside_enabled_app_focus.log`

### outside_enabled_tab_create

- Status: pass
- Command: `$WARPCTRL --output-format json tab create`
- Exit code: 0
- Expected: Command succeeds and a new terminal tab appears.
- Actual: Succeeded with created=true, previous_count=1, count=2, active_index=1; after screenshot shows new tab state.
- Permission state: outside-Warp control enabled; app-state mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/008__outside-enabled__tab__create__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/007__outside-enabled-before__tab__create__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/009__outside-enabled-after__tab__create__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/007_outside_enabled_tab_create.log`

### outside_enabled_tab_rename

- Status: pass
- Command: `$WARPCTRL --output-format json tab rename --tab active "WarpCtrl Validation Tab"`
- Exit code: 0
- Expected: Command succeeds and active tab title becomes WarpCtrl Validation Tab.
- Actual: Succeeded with {"action":"tab.rename","ok":true}; after screenshot shows the renamed active tab.
- Permission state: outside-Warp control enabled; metadata/configuration mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/011__outside-enabled__tab__rename__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/010__outside-enabled-before__tab__rename__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/012__outside-enabled-after__tab__rename__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/008_outside_enabled_tab_rename.log`

### outside_enabled_tab_reset_name

- Status: pass
- Command: `$WARPCTRL --output-format json tab reset-name --tab active`
- Exit code: 0
- Expected: Command succeeds and active tab title resets.
- Actual: Succeeded with {"action":"tab.reset_name","ok":true}; after screenshot shows the reset tab title.
- Permission state: outside-Warp control enabled; metadata/configuration mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/014__outside-enabled__tab__reset-name__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/013__outside-enabled-before__tab__reset-name__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/015__outside-enabled-after__tab__reset-name__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/009_outside_enabled_tab_reset_name.log`

### outside_enabled_tab_color_set_hex_ff00ff

- Status: fail
- Command: `$WARPCTRL --output-format json tab color set --tab active "#ff00ff"`
- Exit code: 1
- Expected: Requested validation expected command success and visible magenta/pink active tab color.
- Actual: Failed with invalid_params: #ff00ff is not a supported tab color.
- Permission state: outside-Warp control enabled; metadata/configuration mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/017__outside-enabled__tab-color__set__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/016__outside-enabled-before__tab-color__set__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/018__outside-enabled-after__tab-color__set__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/010_outside_enabled_tab_color_set.log`

### outside_enabled_tab_color_clear

- Status: pass
- Command: `$WARPCTRL --output-format json tab color clear --tab active`
- Exit code: 0
- Expected: Command succeeds and active tab color clears.
- Actual: Succeeded with {"action":"tab.color.clear","ok":true}; after screenshot captured cleared state.
- Permission state: outside-Warp control enabled; metadata/configuration mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/020__outside-enabled__tab-color__clear__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/019__outside-enabled-before__tab-color__clear__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/021__outside-enabled-after__tab-color__clear__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/011_outside_enabled_tab_color_clear.log`

### outside_enabled_pane_rename

- Status: pass
- Command: `$WARPCTRL --output-format json pane rename --pane active "WarpCtrl Validation Pane"`
- Exit code: 0
- Expected: Command succeeds and active pane title/name becomes WarpCtrl Validation Pane where pane names are visible.
- Actual: Succeeded with {"action":"pane.rename","ok":true}; after screenshot captured resulting UI state.
- Permission state: outside-Warp control enabled; metadata/configuration mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/023__outside-enabled__pane__rename__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/022__outside-enabled-before__pane__rename__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/024__outside-enabled-after__pane__rename__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/012_outside_enabled_pane_rename.log`

### outside_enabled_pane_reset_name

- Status: pass
- Command: `$WARPCTRL --output-format json pane reset-name --pane active`
- Exit code: 0
- Expected: Command succeeds and active pane title/name resets where pane names are visible.
- Actual: Succeeded with {"action":"pane.reset_name","ok":true}; after screenshot captured resulting UI state.
- Permission state: outside-Warp control enabled; metadata/configuration mutations enabled
- Terminal screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/026__outside-enabled__pane__reset-name__terminal.png`
- Before UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/025__outside-enabled-before__pane__reset-name__ui.png`
- UI screenshot: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/027__outside-enabled-after__pane__reset-name__ui.png`
- Log: `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/013_outside_enabled_pane_reset_name.log`

## Screenshots

- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/001__outside-default-off__app__focus__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/002__outside-default-off__app__focus__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/003__outside-enabled__instance__list__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/004__outside-enabled-before__app__focus__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/005__outside-enabled__app__focus__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/006__outside-enabled-after__app__focus__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/007__outside-enabled-before__tab__create__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/008__outside-enabled__tab__create__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/009__outside-enabled-after__tab__create__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/010__outside-enabled-before__tab__rename__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/011__outside-enabled__tab__rename__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/012__outside-enabled-after__tab__rename__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/013__outside-enabled-before__tab__reset-name__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/014__outside-enabled__tab__reset-name__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/015__outside-enabled-after__tab__reset-name__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/016__outside-enabled-before__tab-color__set__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/017__outside-enabled__tab-color__set__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/018__outside-enabled-after__tab-color__set__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/019__outside-enabled-before__tab-color__clear__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/020__outside-enabled__tab-color__clear__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/021__outside-enabled-after__tab-color__clear__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/022__outside-enabled-before__pane__rename__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/023__outside-enabled__pane__rename__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/024__outside-enabled-after__pane__rename__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/025__outside-enabled-before__pane__reset-name__ui.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/026__outside-enabled__pane__reset-name__terminal.png`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/screenshots/027__outside-enabled-after__pane__reset-name__ui.png`

## Logs

- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/001_build.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/002_app_default_off.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/003_outside_default_off_app_focus.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/004_app_enabled.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/005_outside_enabled_instance_list.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/006_outside_enabled_app_focus.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/007_outside_enabled_tab_create.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/008_outside_enabled_tab_rename.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/009_outside_enabled_tab_reset_name.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/010_outside_enabled_tab_color_set.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/011_outside_enabled_tab_color_clear.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/012_outside_enabled_pane_rename.log`
- `validation-artifacts/warpctrl-v2/53d0313d/appstate-mutations/logs/013_outside_enabled_pane_reset_name.log`

## Blockers and skipped commands

No commands were skipped. The only blocker/validation failure is the unsupported requested hex tab color noted above.
