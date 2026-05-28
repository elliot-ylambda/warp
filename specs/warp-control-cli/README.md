# warpctrl operator README
`warpctrl` is the provisional standalone CLI for controlling an already-running local Warp app instance. It is intended for scripts, demos, agent workflows, and developer automation that need to perform allowlisted Warp UI actions without launching the GUI executable in CLI mode.
The first implementation slice is intentionally narrow:
- discover compatible running Warp instances;
- select one instance implicitly when unambiguous or explicitly with `--instance`;
- send authenticated local-control requests through the per-instance discovery record;
- create a new terminal tab with `warpctrl tab create`.
The local-control protocol and catalog are broader than this slice, but commands outside the implemented capability set should fail with structured unsupported-action errors until their handlers land.
## Packaging model
`warpctrl` should be packaged as a separate CLI artifact from the Warp GUI app while reusing shared repository code:
- `crates/local_control` owns discovery records, local authentication material, client transport, protocol envelopes, action names, and error types.
- `crates/warp_cli` owns command parsing conventions for local-control subcommands.
- the app-side bridge owns the per-process loopback listener and dispatches supported actions onto the live Warp UI context.
The binary should initialize only CLI parsing, instance discovery, local authentication loading, request serialization, HTTP transport, and output formatting. It should not initialize GUI state, terminal models, rendering, workspaces, or main-app startup paths.
During the provisional naming period, release artifacts and helper names may be channelized, but operator docs and examples should use `warpctrl` unless an integration branch explicitly documents a channel-specific alias.
This branch wires the standalone binary target and app-bundle hooks that include `warpctrl` when the `warp_control_cli` feature is present:
- `cargo build -p warp --bin warpctrl --features warp_control_cli`
- `script/macos/bundle --artifact app --features warp_control_cli ...`
- `script/linux/bundle --artifact app ...` with `warp_control_cli` included in the feature set
A dedicated `--artifact warpctrl` packaging mode remains follow-up work, as does Windows installer/helper exposure.
## Install and invocation guidance
### macOS
Build locally with `cargo build -p warp --bin warpctrl --features warp_control_cli`, then run `target/debug/warpctrl` or copy/symlink that binary onto `PATH`.
For distributable app-bundle checks, use `script/macos/bundle --artifact app --features warp_control_cli` with the desired channel/signing flags. The bundle script includes `warpctrl` under the app bundle's `Resources/bin` directory when the feature is present.
### Linux
Build locally with `cargo build -p warp --bin warpctrl --features warp_control_cli`, then run `target/debug/warpctrl` or copy/symlink that binary onto `PATH`.
For distributable app-package checks, include `warp_control_cli` in the app feature set and use `script/linux/bundle --artifact app` with the desired channel/package selection. The Linux package install step places `warpctrl` alongside the app executable when the feature is present.
Run `warpctrl --version` after installation to confirm the shell is resolving the expected build.
### Windows
Build locally with `cargo build -p warp --bin warpctrl --features warp_control_cli`, then run `target\debug\warpctrl.exe` or copy that binary onto `PATH`.
The Windows-native binary target exists in this slice. Installer helper creation and release-artifact wiring still need a later packaging change before docs can promise an installer-provided `warpctrl` command.
## End-to-end local test flow
Use matching app and CLI bits from the same branch or release artifact so the protocol version and action catalog agree.
1. Start Warp and leave at least one window open.
2. Confirm that the local-control server registered the running process:
   ```bash
   warpctrl instance list
   ```
3. If exactly one compatible instance is listed, create a new terminal tab:
   ```bash
   warpctrl tab create
   ```
4. If multiple compatible instances are listed, copy the desired `instance_id` and target it explicitly:
   ```bash
   warpctrl tab create --instance <instance_id>
   ```
5. Verify the running app receives focus for the selected instance and a new terminal tab appears according to Warp's normal new-tab placement behavior.
6. In a future slice that implements `tab list`, inspect state before and after the mutation:
   ```bash
   warpctrl tab list --instance <instance_id>
   ```
Expected failures:
- no running compatible app: exits non-zero with a no-instance error;
- multiple ambiguous instances: exits non-zero and asks for `--instance`;
- unsupported app build or stale discovery record: exits non-zero with a protocol, stale-target, or transport error;
- `tab.create` not yet implemented by the running app bridge: exits non-zero with an unsupported-action error.
## Security model
The local-control protocol is designed for same-user scripting, not cross-user or network access. The trust boundary is the local user account.
- **Loopback-only listener.** Each Warp process binds its control server to `127.0.0.1` on an ephemeral port. The listener is not reachable from the network.
- **Broker-mediated scoped credentials.** Discovery records contain endpoint metadata, not scoped control credentials. A separate owner-only broker-token file is used as foundation-slice bootstrap proof when requesting an action-scoped credential from the running app. Every control request must then present that app-issued scoped credential in the `Authorization` header.
- **File-permission-gated discovery.** Discovery records and broker-token files are stored in a per-user local-control directory. On POSIX platforms, files are created with `0600` permissions (owner read/write only) and the directory is `0700`. On Windows, records live under the current user's app data directory; stronger ACL validation remains follow-up work. Any same-user process that can read the broker-token file can request scoped credentials, so this is a foundation guardrail, not strong same-user malicious-app isolation.
- **Stale-record pruning.** On each `instance list` or implicit discovery call, records whose PID is no longer alive are deleted automatically, preventing stale tokens from lingering on disk.
- **No CORS.** The control endpoints do not set permissive CORS headers, so browser-origin JavaScript cannot read responses even if it guesses the port. The bearer token requirement provides a second layer since browsers cannot read the discovery file.
```mermaid
sequenceDiagram
    participant CLI as warpctrl
    participant FS as ~/.warp/local-control/
    participant HTTP as Warp loopback server<br/>(127.0.0.1:ephemeral)
    participant Bridge as App bridge

    CLI->>FS: Read discovery records (user-only permissions / ACL)
    FS-->>CLI: instance_id, endpoint, broker-token path
    CLI->>CLI: Prune stale PIDs, select instance
    CLI->>HTTP: POST /v1/control/credentials<br/>Authorization: Bearer <broker-token>
    HTTP-->>CLI: Scoped credential or structured denial
    CLI->>HTTP: POST /v1/control<br/>Authorization: Bearer <scoped credential>
    HTTP->>HTTP: Verify credential was issued by this instance
    alt Invalid or missing credential
        HTTP-->>CLI: Structured unauthorized error
    else Valid credential
        HTTP->>Bridge: Dispatch action to app context
        Bridge-->>HTTP: Structured result or error
        HTTP-->>CLI: JSON response envelope
    end
```
**Known limitations and future hardening:**
- The foundation broker token is stored in a plaintext owner-only file outside the discovery JSON. Any compromised process running as the same user that can read this file can request scoped credentials.
- Broker tokens do not rotate during a Warp session. Scoped credentials are short-lived and validated against the app's in-memory issued-credential store.
- Windows local-control authentication is not complete until discovery-record ACL creation and validation are implemented.
- Once higher-risk handlers land (e.g. `input.insert`, command execution), the same-user boundary becomes a code-execution trust boundary. Consider separating the token from the discovery metadata, adding per-request nonces, or switching to a Unix domain socket with `SO_PEERCRED` for kernel-verified caller identity.
## Documentation review notes
- Treat `warpctrl` as provisional executable naming until packaging signs off on final artifact aliases.
- Keep examples scoped to discovery and `tab create` until additional app-side handlers are implemented.
- Do not document catalog commands as usable just because they exist in protocol enums or parser scaffolding; operator docs should distinguish implemented commands from planned allowlist entries.
- Windows packaging may initially follow the existing helper-wrapper pattern rather than shipping a native standalone executable. Update this README when that decision is final.
