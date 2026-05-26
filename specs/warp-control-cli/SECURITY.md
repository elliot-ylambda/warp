# warpctrl security architecture
`warpctrl` is a local same-machine control CLI for already-running Warp app instances. This document is the normative security policy for the feature. PRODUCT.md defines user-facing scope; TECH.md defines implementation mechanics. If either conflicts with this document, update that document before implementation.
# Current stance
- `warpctrl` is the decided binary name.
- The full allowlisted catalog is the public contract, with per-action implementation status advertised by the running app.
- Authenticated actions require verified Warp-managed terminal invocation and a true logged-in Warp user in the selected app.
- External invocations are limited to logged-out-safe local-control actions.
- There is no standalone secret-based authenticated external scripting path.
- Accepted command submission, agent prompt submission, local filesystem data mutation, arbitrary internal dispatch, arbitrary ACL/public/external sharing, and network control transports are out of scope.
# Security goals
- Prevent unauthenticated localhost clients from invoking control actions.
- Prevent browser-origin JavaScript from becoming an ambient localhost control client.
- Support multiple running Warp app instances without a shared global mutating port or global credential.
- Separate discovery metadata from control authority.
- Require explicit in-app enablement before outside-Warp local control can issue credentials or accept requests.
- Distinguish verified Warp-managed terminal invocation from external same-user invocation using app-issued proof, not caller-declared labels.
- Classify every action by state/data category, permission category, authenticated-user requirement, target scope, and allowed invocation context.
- Enforce those classifications in the selected Warp app before selector resolution or handler dispatch.
- Preserve deterministic targeting and fail on ambiguous, missing, stale, malformed, unsupported, or unallowlisted requests.
- Keep raw credential material, app auth tokens, terminal output, input text, Drive contents, and other sensitive data out of logs, errors, discovery records, completions, and generated docs.
# Trust boundaries
## OS-user boundary
Discovery records, sockets or endpoints, and credential references must be readable only by the owning OS user. This protects against other local users and network peers. It is not a complete defense against compromised same-user software.
## Invocation boundary
Same-user invocation does not imply the same authority. External clients receive only logged-out-safe grants. Verified Warp-managed terminal clients can request broader grants only after presenting app-issued proof and satisfying Scripting settings.
## Authenticated-user boundary
Actions that touch Warp-user-backed state require authenticated authority tied to the selected app's current logged-in user. The CLI never receives raw Firebase, OAuth, server, or cloud service tokens. If the app logs out or switches users, authenticated grants fail.
## Action boundary
Every action maps to one permission category. The bridge compares requested action metadata against the presented grant before resolving selectors.
## Target boundary
Credentials may be scoped to an instance, action family, target family, or resource. A grant for one instance or target must not authorize another.
# Invocation contexts
## Verified Warp-managed terminal
A `warpctrl` process started inside a Warp-managed terminal may present an app-issued execution proof. The proof must be bound to a live terminal/session, selected app instance, expiry, and revocation state. Environment variables may carry handles or hints, but caller-set variables are not authority by themselves.
Verified terminal context can raise the maximum eligible grant set. It does not bypass Scripting settings, Agent Profile policy, authenticated-user requirements, action categories, or target restrictions.
## External client
A `warpctrl` process started outside Warp, such as another terminal app, IDE, launch agent, or background script, is external. External control defaults off. When enabled, external clients can receive only logged-out-safe local-control grants and cannot receive authenticated-user grants.
# Enablement and settings
Warp owns a Settings > Scripting surface for local scripting controls.
Required settings behavior:
- Inside-Warp control is enabled only for verified Warp-managed terminal invocations.
- Outside-Warp control defaults off and requires an explicit user gesture.
- Enablement states are local-only and must not sync through Settings Sync, Warp Drive, or server-backed preferences.
- Public `warpctrl` commands, direct protocol requests, scripts, ordinary settings files, registry/plist/defaults edits, and cloud preferences must not be able to enable local control.
- Disabling a context invalidates existing credentials for that context.
- Granular permissions are independent for metadata reads, underlying data reads, app-state mutations, metadata/configuration mutations, and underlying data mutations.
The foundation implementation may use private local-only settings as an interim storage mechanism only when those settings are excluded from user-visible settings files, generated schemas, Settings Sync, Warp Drive, and local-control settings read/write actions.
# Credential model
The credential model is scoped and action-aware. A credential or grant records:
- issuing instance;
- protocol version;
- action or action family;
- permission category;
- invocation context;
- authenticated-user subject when present;
- optional target/resource restrictions;
- issue time, expiry, and revocation identity;
- integrity protection against widening.
Credential issuance is app-owned. The CLI can request, load, and present credentials, but it cannot mint authority. The bridge validates credentials again on every request.
# Discovery
Each participating Warp process publishes per-user discovery metadata for compatible local instances. Discovery records contain instance identity, PID, build/channel metadata, protocol version, and only the endpoint/credential-reference data allowed by the selected invocation context.
Discovery requirements:
- owner-only file or socket permissions;
- no raw broad credential in plaintext discovery records;
- stale record pruning by PID and health checks;
- no terminal contents, environment values, auth tokens, Drive contents, or sensitive target state;
- no actionable outside-Warp endpoint or credential reference when outside-Warp control is disabled.
# Transport protections
- Bind local control listeners to loopback or an owner-only local socket.
- Do not set permissive CORS headers.
- Require valid scoped credentials for control requests.
- Use non-GET request methods for mutations.
- Keep unauthenticated health metadata minimal.
- Preserve structured error envelopes for security failures.
# Permission categories
## Metadata reads
Return local structure or non-content metadata, such as instances, app version, active chain, windows, tabs, panes, sessions, themes, setting keys, keybindings, action metadata, capability metadata, project identity, and Drive object IDs/names/types.
## Underlying data reads
Return user data without changing it, such as block output, terminal history, input buffer contents, Drive object contents, AI conversation content, and similar content-bearing state.
## App-state mutations
Change visible local Warp UI state without changing underlying user data, such as creating tabs, splitting panes, focusing targets, opening panels, opening files/projects/views, and staging input text without execution.
## Metadata/configuration mutations
Change persistent metadata or configuration, such as names, colors, themes, font size, zoom, keybindings, or allowlisted settings.
## Underlying data mutations
Can change user data, execute code, run approved workflows, or cause external side effects. This category includes `input.run`, Drive object create/update/delete/insert/share-to-team, and `drive.workflow.run`. It requires authenticated Warp-terminal authority plus explicit underlying-data-mutation permission.
# Authenticated-user requirements
New catalog actions default to authenticated-user required unless deliberately reviewed as logged-out-safe. Logged-out-safe actions are limited to local app structure, local appearance metadata, and UI/app-state operations that do not expose or mutate Warp-user-backed state.
Actions require authenticated user state when they read or mutate Warp Drive, AI conversation traces, synced settings, team/account data, cloud-backed user state, or user-authored runnable content.
# Deterministic target resolution
- Instance selection occurs before request dispatch.
- Multiple compatible instances require explicit selection unless there is one unambiguous active instance.
- Active selectors are allowed only when unambiguous.
- Explicit IDs resolve exactly or return `stale_target`.
- Missing targets return `missing_target`.
- Ambiguous selectors return `ambiguous_target`.
- Session-scoped actions against non-terminal panes return `target_state_conflict`.
- The bridge must not fall back to neighboring targets.
# Structured errors
Security- and safety-relevant errors include:
- `local_control_disabled`
- `unauthorized_local_client`
- `insufficient_permissions`
- `authenticated_user_required`
- `authenticated_user_unavailable`
- `execution_context_not_allowed`
- `ambiguous_instance`
- `ambiguous_target`
- `stale_target`
- `invalid_selector`
- `unsupported_action`
- `not_allowlisted`
- `invalid_params`
- `target_state_conflict`
- `missing_target`
- `no_instance`
The CLI must preserve these errors in human-readable and JSON output.
# Required controls for action families
Before an action family is advertised as implemented:
- The action exists in the typed catalog.
- Metadata declares state/data category, permission category, authenticated-user requirement, allowed invocation contexts, target scope, parameter spec, and result spec.
- The bridge enforces permission category and authenticated-user policy before selector resolution.
- Invalid, expired, revoked, insufficient, disabled, unsupported, and unallowlisted requests fail closed.
- Tests cover allowed and denied credential paths, authenticated-user denial, selector failure, and success behavior.
- Logs and errors avoid credentials and sensitive user data.
- Operator docs distinguish implemented actions from catalog stubs.
