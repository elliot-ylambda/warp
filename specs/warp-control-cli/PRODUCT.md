# Summary
Warp ships `warpctrl` as the standalone local control CLI for operating already-running local Warp app instances. `warpctrl` is an allowlisted, typed control plane for agent and developer workflows that need to inspect or change Warp product surfaces such as windows, tabs, panes, terminal sessions, blocks, settings, appearance, command surfaces, files opened in Warp, projects, and Warp Drive objects.
The public contract is catalog-first: command names, selectors, permissions, result types, and errors are defined up front so implementation shards can add handlers without changing the security stance.
# Product stance
`warpctrl` is the decided binary name.
The initial target is the full allowlisted public catalog, with implementation status advertised per action by the running app.
Authenticated actions require a verified Warp-managed terminal invocation and a logged-in Warp user in the selected app.
External invocations are limited to logged-out-safe local-control actions.
There is no standalone secret-based authenticated external scripting path.
The transport is local same-machine control of running Warp app instances only. Network or hosted control transports are out of scope.
# Goals
- Provide a stable scriptable CLI for operating running Warp app instances without launching the GUI executable in CLI mode.
- Give agents a typed, permissioned way to preserve and organize visible Warp UI context instead of relying on brittle screen automation or arbitrary internal dispatch.
- Support deterministic targeting across instances, windows, tabs, panes, sessions, terminal blocks, visible file/path intents, projects, surfaces, and Warp Drive objects.
- Keep every action allowlisted, named by public product nouns, classified by state/data category, permission category, authenticated-user requirement, allowed invocation context, and implementation status.
- Preserve native-tools-first boundaries: agents should still use native file, shell, web, MCP, and conversation tools when those tools are the better surface.
# Non-goals and excluded surfaces
- Replacing the Oz CLI or mixing cloud-agent management into `warpctrl`.
- Exposing arbitrary internal action dispatch, raw view dispatch, debug helpers, crash/panic helpers, heap dumps, token-copying helpers, or broad developer-only commands.
- Mutating local filesystem data through `warpctrl`; file/path support is limited to visible Warp app intents such as opening a path and listing files already open in Warp.
- Submitting accepted commands, submitting agent prompts, or causing agent execution through this catalog.
- Arbitrary ACL editing, public sharing, guest sharing, or broad sharing policy mutation. The catalog may include the typed personal-to-team Drive sharing action only because it maps to a constrained Warp Drive product flow.
- Standalone secret-based authenticated external scripting.
- Network control endpoints or hosted control URLs.
# User stories
## Agent workspace orchestration
An agent can inspect visible Warp structure, choose or create an appropriate workspace layout, focus or name targets, open relevant surfaces, and leave Warp in a readable task-shaped state for the user.
## Existing-session debugging and repair
An agent can inspect which instance, window, tab, pane, session, block, and surface are active or targetable before applying focus, layout, panel, or settings changes.
## Warp Drive navigation and typed operations
An agent can list, inspect, open, create, update, insert, delete, share to the current team, or run approved Warp Drive objects only through typed catalog actions with authenticated Warp-terminal grants where required.
## Demos and walkthroughs
A script or agent can put Warp into a known presentation state: theme, zoom, window/tab/pane layout, focused targets, panels, command surfaces, and Warp Drive views.
## Personalization and onboarding
An agent can inspect user-approved preferences, propose Warp equivalents, and apply allowlisted settings, appearance, keybinding, layout, and surface changes with explicit permission categories.
# Targeting model
Selectors are deterministic and hierarchical where the UI hierarchy is hierarchical:
- Instance: active, opaque instance ID, or PID convenience filter.
- Window: active, opaque ID, scoped index, or exact title.
- Tab: active, opaque ID, scoped index, or exact title.
- Pane: active, opaque ID, or scoped index.
- Session: active, opaque ID, or scoped index.
- Block: active/current where unambiguous, opaque ID, or scoped index.
- File/path: path plus optional line and column for visible app intents.
- Project/workspace: path, opaque project/workspace ID, or exact name where exposed.
- Drive object: opaque ID, with type-scoped exact lookup for interactive use.
Active defaults are allowed only when unambiguous. Explicit IDs must resolve exactly or fail with `stale_target`. Missing active targets fail with `missing_target`. Ambiguous selectors fail with `ambiguous_target`. Commands must not silently retarget to a nearby instance, tab, pane, session, file, or Drive object.
# State/data categories and permissions
Every action belongs to exactly one category:
- Metadata reads: structure and non-content metadata such as instances, windows, tabs, panes, sessions, capabilities, actions, settings keys, themes, keybindings, project identity, and Drive object IDs/names/types.
- Underlying data reads: terminal block output, input buffer contents, history, Drive object contents, AI conversation content, or other user data.
- App-state mutations: visible UI state such as focus, layout, panels, surface opens, file/project opens, and input-buffer staging without execution.
- Metadata/configuration mutations: persistent metadata or configuration such as titles, tab colors, themes, zoom, font size, keybindings, and allowlisted settings.
- Underlying data mutations: terminal execution through explicit `input.run`, typed Warp Drive CRUD/insert/share-to-team/workflow-run operations, and other actions that can change user data or cause external side effects.
A command that touches multiple categories requires the strongest applicable permission.
# Public action catalog
## Instance, app, capability, and action metadata
- `instance.list`
- `instance.inspect`
- `app.ping`
- `app.version`
- `app.active`
- `app.focus`
- `capability.list`
- `capability.inspect`
- `action.list`
- `action.inspect`
## Auth status and app login routing
- `auth.status`
- `auth.login`
These actions report local/authenticated grant availability or open the selected app's normal sign-in UI. They do not create a standalone external secret identity.
## Windows, tabs, panes, and sessions
- `window.list`
- `window.inspect`
- `window.create`
- `window.focus`
- `window.close`
- `tab.list`
- `tab.inspect`
- `tab.create`
- `tab.activate`
- `tab.move`
- `tab.close`
- `tab.rename`
- `tab.reset_name`
- `tab.color.set`
- `tab.color.clear`
- `pane.list`
- `pane.inspect`
- `pane.split`
- `pane.focus`
- `pane.navigate`
- `pane.resize`
- `pane.maximize`
- `pane.unmaximize`
- `pane.close`
- `pane.rename`
- `pane.reset_name`
- `session.list`
- `session.inspect`
- `session.activate`
- `session.previous`
- `session.next`
- `session.reopen_closed`
## Blocks, input, and history
- `block.list`
- `block.inspect`
- `block.output`
- `input.get`
- `input.insert`
- `input.replace`
- `input.clear`
- `input.mode.set`
- `input.run`
- `history.list`
Input insert/replace/clear/mode commands stage visible input only. `input.run` is the only terminal execution action and requires authenticated Warp-terminal authority plus underlying-data-mutation permission.
## Appearance, settings, and keybindings
- `theme.list`
- `theme.get`
- `theme.set`
- `theme.system.set`
- `theme.light.set`
- `theme.dark.set`
- `appearance.get`
- `appearance.font_size.increase`
- `appearance.font_size.decrease`
- `appearance.font_size.reset`
- `appearance.zoom.increase`
- `appearance.zoom.decrease`
- `appearance.zoom.reset`
- `setting.list`
- `setting.get`
- `setting.set`
- `setting.toggle`
- `keybinding.list`
- `keybinding.get`
Settings mutations are protocol actions handled by the running app. `warpctrl` must not bypass the app by editing settings files directly.
## Surfaces
- `surface.settings.open`
- `surface.command_palette.open`
- `surface.command_search.open`
- `surface.warp_drive.open`
- `surface.warp_drive.toggle`
- `surface.resource_center.toggle`
- `surface.ai_assistant.toggle`
- `surface.code_review.toggle`
- `surface.left_panel.toggle`
- `surface.right_panel.toggle`
- `surface.vertical_tabs.toggle`
## Files and projects
- `file.list`
- `file.open`
- `project.active`
- `project.list`
- `project.open`
These actions address Warp-visible app/editor/project state only.
## Warp Drive
- `drive.list`
- `drive.inspect`
- `drive.open`
- `drive.notebook.open`
- `drive.env_var_collection.open`
- `drive.object.share.open`
- `drive.object.create`
- `drive.object.update`
- `drive.object.delete`
- `drive.object.insert`
- `drive.object.share_to_team`
- `drive.workflow.run`
Drive metadata listing requires authenticated user state. Drive content reads and mutations require authenticated Warp-terminal authority and the corresponding data-read or data-mutation permission.
# CLI shape
The CLI command hierarchy is noun-oriented and mirrors the action names:
- `warpctrl instance list`
- `warpctrl instance inspect --instance <id>`
- `warpctrl capability list`
- `warpctrl capability inspect tab.create`
- `warpctrl action inspect drive.workflow.run`
- `warpctrl tab create --window active`
- `warpctrl pane split --direction right`
- `warpctrl block output --block-id <id> --plain`
- `warpctrl surface.settings.open --page scripting`
- `warpctrl drive inspect <id>`
JSON output and structured errors are supported for discovery, reads, mutations, and failures.
# Implementation status
The running app advertises implementation status per action. Unsupported catalog entries return `unsupported_action`; names intentionally outside the public catalog return `not_allowlisted` or fail enum parsing before dispatch.
