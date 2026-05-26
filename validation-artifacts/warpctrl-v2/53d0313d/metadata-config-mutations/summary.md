# Warp Control CLI validation: metadata-config-mutations
Exact SHA: `53d0313df1f712cf98b1c53e9272c588141da350`
Artifact branch: `zach/warpctrl-validation-artifacts/53d0313d/metadata-config-mutations`

## Result counts
- Pass: 22
- Fail: 1
- Skip: 0

## Notable findings
- Built the requested exact commit with `warp_control_cli` and verified local-control discovery against a running `WarpOss` instance.
- The no-credentials onboarding path reached a logged-out terminal workspace through computer use.
- All assigned theme, appearance, setting, and keybinding commands were executed; `keybinding get copy` returned `missing_target` and is the only failed required command.
- Changed theme/setting state was restored where possible using additional captured `warpctrl` invocations.

## Commands not executed
- None of the requested commands were skipped.

## Blockers
- No test credentials were available or required. The validation used logged-out-safe outside-Warp control permissions only.
