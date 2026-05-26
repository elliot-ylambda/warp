# Context
PRODUCT.md defines the public `warpctrl` product contract. SECURITY.md is the normative security policy. This document describes implementation mechanics for the shared protocol, catalog, app bridge, CLI, and validation flow.
# Current implementation baseline
The repository already has these local-control building blocks:
- `crates/local_control/src/catalog.rs` owns public action metadata.
- `crates/local_control/src/protocol.rs` owns wire envelopes, typed parameter/result payloads, and structured errors.
- `crates/local_control/src/selectors.rs` owns current window/tab/pane selector shapes.
- `crates/local_control/src/auth.rs` owns scoped credential request and grant types.
- `crates/local_control/src/discovery.rs` owns per-instance discovery records.
- `app/src/local_control/mod.rs` owns the app-side bridge/server skeleton.
- `crates/warp_cli/src/bin/warpctrl.rs` and `app/src/bin/warpctrl.rs` own standalone CLI entry points.
# Contract rules
- `ActionKind` serialized names are the canonical public protocol names.
- `ActionKind::ALL` contains only approved public actions.
- Excluded local filesystem mutation names and standalone secret-auth names may be listed as excluded constants, but they must not deserialize into `ActionKind`.
- `ActionMetadata` must include implementation status, state/data category, permission category, authenticated-user requirement, allowed invocation contexts, target scope, parameter spec, and result spec.
- Implemented foundation actions can advertise `OutsideWarp` only when logged-out-safe.
- Authenticated actions advertise `InsideWarp` only and require a verified Warp-managed terminal grant before execution.
# Canonical action name changes
Use these public names when porting handlers and tests:
- `app.inspect` -> `instance.inspect`
- `app.settings.open` -> `surface.settings.open`
- `app.command_palette.open` -> `surface.command_palette.open`
- `app.command_search.open` -> `surface.command_search.open`
- `app.warp_drive.open` -> `surface.warp_drive.open`
- `app.warp_drive.toggle` -> `surface.warp_drive.toggle`
- `app.resource_center.toggle` -> `surface.resource_center.toggle`
- `app.ai_assistant.toggle` -> `surface.ai_assistant.toggle`
- `app.code_review.toggle` -> `surface.code_review.toggle`
- `app.vertical_tabs.toggle` -> `surface.vertical_tabs.toggle`
- `pane.session.previous` -> `session.previous`
- `pane.session.next` -> `session.next`
- `appearance.font_size` -> `appearance.font_size.increase`, `appearance.font_size.decrease`, or `appearance.font_size.reset`
- `appearance.zoom` -> `appearance.zoom.increase`, `appearance.zoom.decrease`, or `appearance.zoom.reset`
- `appearance.set` -> `theme.set`, `theme.system.set`, `theme.light.set`, or `theme.dark.set`
Add these metadata/read names instead of using app-specific aliases:
- `capability.list`
- `capability.inspect`
- `action.list`
- `action.inspect`
- `block.inspect`
- `block.output`
- `drive.inspect`
- `drive.object.create`
- `drive.object.update`
- `drive.object.delete`
- `drive.object.insert`
- `drive.object.share_to_team`
- `drive.workflow.run`
# Shared protocol mechanics
The request envelope contains protocol version, request ID, target selector, action kind, and action parameters. The response envelope contains protocol version, request ID, and either success data or `ControlError`.
Selectors remain extensible. Current compiled selectors cover window, tab, and pane. Protocol payloads add Drive object IDs/types so Drive shards can share canonical parameter and result contracts without changing action names.
# App bridge mechanics
The local-control HTTP or socket handler runs off the UI thread. It must authenticate and deserialize requests, then schedule app-state work onto the main app context using the existing model-spawning bridge. The bridge must revalidate credentials, action metadata, invocation context, authenticated-user requirement, and target scope before resolving selectors or dispatching handlers.
Handler implementation order:
1. Decode request envelope.
2. Verify protocol version.
3. Authenticate credential.
4. Load `ActionMetadata` from the catalog.
5. Verify invocation context, permission category, authenticated-user requirement, and target/resource restrictions.
6. Validate action parameters.
7. Resolve selectors deterministically.
8. Dispatch only typed allowlisted handlers.
9. Return structured result or error.
# CLI mechanics
`warpctrl` should follow existing CLI conventions used by the repository's CLI tooling:
- clap-style noun subcommands;
- JSON and human-readable output modes;
- stable structured errors;
- generated or checked completions and reference docs from the catalog;
- no GUI initialization for ordinary CLI invocation.
CLI parser work must be derived from the catalog so names, help, completions, and docs do not drift from `ActionKind::ALL`.
# Security implementation notes
- Outside-Warp control defaults off.
- Inside-Warp credential requests are rejected until app-issued terminal proof verification is implemented.
- External clients cannot receive authenticated-user grants.
- Public settings read/write actions must not expose or mutate private local-control enablement settings.
- The bridge, not the CLI, is the enforcement point for action metadata and grants.
# Validation plan
Run the narrowest useful checks first:
- `git diff --check -- specs/warp-control-cli/PRODUCT.md specs/warp-control-cli/TECH.md specs/warp-control-cli/SECURITY.md crates/local_control/src/catalog.rs crates/local_control/src/protocol.rs crates/local_control/src/protocol_tests.rs`
- stale-language grep across `specs/warp-control-cli/*.md` for banned framing and auth-scope terms;
- `cargo check -p local_control` when the Rust toolchain is available;
- `cargo nextest run --no-fail-fast --workspace local_control::protocol_tests` when tests are available in the environment.
If a command is unavailable in a cloud shard, report it as skipped with the exact toolchain or environment blocker.
# Fan-out handoff
This shard establishes the dependency gate for other implementation shards. Other shards should port handlers and tests to the canonical names above, use `ActionMetadata` for permission enforcement, and avoid adding handlers for excluded surfaces.
