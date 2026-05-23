Closes #9576

## Summary
- Add a transient top-centered workspace HUD showing the current UI zoom percentage after zoom in, zoom out, and reset actions.
- Share zoom stepping logic with tests so min/max clamps still report the current clamped value and invalid stored zoom values do not show the HUD.
- Apply zoom factor changes from workspace window-settings updates so keybinding-driven zoom changes take effect immediately.

## Validation
- `cargo test --manifest-path /workspace/warp/Cargo.toml -p warp adjusted_zoom_level`
- `cargo fmt --manifest-path /workspace/warp/Cargo.toml -- app/src/workspace/view.rs app/src/workspace/view/zoom_level_hud.rs` could not run because `cargo-fmt`/`rustfmt` is not installed in this environment.
