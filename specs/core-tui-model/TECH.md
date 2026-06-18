# Core TUI Agent Model and Verifiable Send/Follow-Up Flow

## Problem statement
The branch has a `tui` WarpUI backend and a skeletal `warp-tui` binary, but the TUI app **cannot yet send prompts to Warp's agent backend or maintain a conversation**.

This plan adds the shared state and initialization needed for a TUI-native agent session that can:
1. Send an initial prompt.
2. Receive streamed output into conversation state.
3. Send a follow-up in the same conversation.

This is a minimal working slice, but it is built to extend. We prefer good seams now over throwaway stubs.

## Ultimate verifiable goal
A **view-tree-free** test drives the TUI agent core. It:
- Initializes the TUI singleton graph in a test app context.
- Constructs a synthetic `AgentSessionOwnerId` from a fresh `EntityId` — no `TerminalView`, no `RootTuiView`, no runtime/driver. This proves conversation ownership is fully decoupled from views.
- Submits prompt `"first"` to the real agent backend and receives the streamed response.
- Submits follow-up prompt `"second"` in the same conversation and receives another streamed response.
- Asserts **one conversation with two ordered exchanges**, and that the second request carries the first request's conversation/task context.

The prototype builds no auth flow: the test authenticates with the existing warp-dev credential and hits the real server, so it is network-dependent (not a hermetic unit test). That is acceptable for proving the end-to-end send/follow-up slice.

**Target command (name may change):**
```sh
cargo test -p warp --features tui core_tui_model_sends_initial_prompt_and_follow_up
```
The test must verify both *request construction* and *history mutation* for the initial and follow-up prompts.

## Current state
### TUI entry path
- `app/src/bin/tui.rs:9` sets channel state and calls `warp::run_tui()`.
- `LaunchMode::Tui` exists at `app/src/lib.rs:400`.
- `run_internal` short-circuits **before** `initialize_app`, calling `crate::tui::init(ctx)` directly at `app/src/lib.rs:1078`. Enough for the logo-only TUI, not for agent requests.
- The TUI root at `app/src/tui.rs:45` is a view-only placeholder (logo/version) that starts `spawn_tui_driver` (`app/src/tui.rs:107`). It registers no agent state.

### GUI Agent Mode ownership
- `TerminalView` stores the per-pane Agent Mode cluster (`BlocklistAIController`, `BlocklistAIContextModel`, `BlocklistAIActionModel`, `BlocklistAIInputModel`) at `app/src/terminal/view.rs (2700-2720)` and constructs it at `app/src/terminal/view.rs (3451-3496)`.
- This is the stateful analog for the TUI work, but the TUI equivalent should be a **singleton model, not a view**.

### Reusable agent infrastructure
- `BlocklistAIHistoryModel` (`app/src/ai/blocklist/history_model.rs:208`) is the durable conversation store. Its terminology is terminal-specific: it keys live/cleared/active conversations by `terminal_view_id` (`app/src/ai/blocklist/history_model.rs (208-245)`). Conceptually these IDs are **local agent-session owners**, not necessarily terminal views.
- `ResponseStream` (`app/src/ai/blocklist/controller/response_stream.rs:76`) already wraps `generate_multi_agent_output`, cancellation, retry, stream-truncation detection, and stream events. **Good primitive to reuse.**
- Request construction flows through `RequestInput` and `api::RequestParams::new` (`app/src/ai/blocklist/controller.rs (2088-2507)`, `app/src/ai/agent/api.rs (92-345)`). `RequestParams::new` depends on shared singletons: settings, permissions, LLM preferences, execution profiles, server API, workspace state, MCP managers, API key managers, and network status.
- **Streamed agent text arrives via `ClientActions`**, not a separate channel. `apply_client_actions` (`app/src/ai/blocklist/history_model.rs:1716`) dispatches to `conversation.apply_client_action` (`app/src/ai/agent/conversation.rs:2367`), where `AddMessagesToTask` / `AppendToMessageContent` carry `AgentOutput` messages. Tool *execution* is queued separately from the finished output; with no tools advertised, none arrive.

## Proposed design

### 1. Introduce `AgentSessionOwnerId` (derived from the owning view)
A newtype over the owning **view's** `EntityId`. It is the owner key for agent conversation state — view-derived but view-agnostic, so the GUI's terminal view and the TUI's root view both produce one, and future sub-views can each own a session with no extra plumbing.
```rust
/// Identifies the local owner of agent conversation state.
///
/// Backed by the `EntityId` of the owning view (the TUI root view today; the
/// terminal view on the GUI side). The shared `AgentConversationEngine` and
/// `CoreTuiModel` use it as the owner key, converting to the `EntityId` the
/// existing `BlocklistAIHistoryModel` APIs already take via `entity_id()`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct AgentSessionOwnerId(EntityId);

impl AgentSessionOwnerId {
    pub fn new(view_id: EntityId) -> Self {
        Self(view_id)
    }

    pub fn entity_id(self) -> EntityId {
        self.0
    }
}
```
GUI `TerminalView` derives one from its view id; each TUI view derives one from its view id. The semantic owner becomes "the local agent session," decoupled from any specific view type.

### 2. Extract a shared agent engine (`AgentConversationEngine`)
**Why not hold `BlocklistAIController` directly?** The controller is scoped to a single terminal pane: its `new()` (`controller.rs:432`) requires a live `TerminalModel`, a PTY-backed `ActiveSession`, an `AgentViewController`, and the input/context/action models, and it holds `terminal_model: Arc<FairMutex<TerminalModel>>` throughout. Reusing it would force the TUI to stand up that entire terminal Agent Mode cluster — the opposite of the view-tree-free design — for logic that is, underneath, terminal-agnostic. It also drags in the controller's terminal/GUI-specific extras (shared sessions, action execution, passive suggestions, slash commands) that the prototype does not want. Extracting a shared engine gives the TUI the controller's canonical send/stream logic *without* that coupling, while the controller keeps those extras layered on top of the shared path.

The create-stream + subscribe + fold-into-history logic lives inline in `BlocklistAIController::send_request_input` (`controller.rs:2320`) and `handle_response_stream_event` (`controller.rs:2696`). That fold logic is already almost entirely delegation to `BlocklistAIHistoryModel`, keyed by an owner `EntityId` + `conversation_id` (`Init` → `initialize_output_for_response_stream`, `ClientActions` → `apply_client_actions`, `Finished`/error → `mark_response_stream_completed_*`). The only terminal-specific bits are feature-flagged shared-session forwarding (`controller.rs:2725-2753`) and `skill_path_origin` (which is `Local` for a non-terminal session).

Extract this into a backend-neutral `AgentConversationEngine` (new module under `app/src/ai/blocklist/`, e.g. `agent_conversation_engine`, no terminal/GUI deps) that owns:
- creating the `ResponseStream` from a `RequestParams` + `AIIdentifiers`;
- appending the user exchange and marking the conversation `InProgress` via `BlocklistAIHistoryModel::update_conversation_for_new_request_input`;
- subscribing and folding `ResponseStreamEvent`s into history.

Terminal-only behavior is injected, not baked in:
- a `SkillPathOrigin` (controller: from its session; TUI: `Local`);
- an optional shared-session forwarder (controller: its terminal model; TUI: none);
- a `ClientActions` action sink (controller: `BlocklistAIActionModel`; TUI: none in phase one — §6).

**`BlocklistAIController` is refactored to call `AgentConversationEngine`**, so there is a *single* send/stream code path used by both surfaces — not a parallel copy. That is the point of the seam: the TUI gets the controller's canonical streaming/dispatch logic without inheriting the terminal model, the action/input/context models, or the agent-view controller. `CoreTuiModel` calls the same `AgentConversationEngine` with local hooks.

`AgentConversationEngine` keys history by the owner id (`AgentSessionOwnerId::entity_id()`), passing through to the existing `BlocklistAIHistoryModel` APIs — **no wholesale rename of the history model is required** for the prototype.

**This is the seed of the shared agent engine, designed to grow.** We want the TUI to eventually support the full Agent Mode feature set (orchestration, skill discovery, tools, queued prompts, …). Today `AgentConversationEngine` owns only the send/stream slice, but it is the shared home those capabilities migrate into (or behind) over time, so neither surface reimplements them. We carve **bottom-up** — grow `AgentConversationEngine` and shrink `BlocklistAIController` toward a thin GUI adapter — rather than mutating the controller in place or branching it on a `mode` flag, because that keeps the high-churn GUI path working and independently testable at each step. How far this goes (one shared controller vs. a shared engine with thin per-surface adapters) is an explicit later decision (see Open decisions).

### 3. Run the full singleton bootstrap for the TUI, skipping heavyweight terminal pieces
For the prototype we do **not** carve `initialize_app` into shared/GUI/TUI phases. The singleton models the main app registers are cheap, so `LaunchMode::Tui` runs the same `initialize_app` (`app/src/lib.rs:1142`) bootstrap and simply skips the few heavyweight, terminal-specific pieces. This is much smaller and lower-risk than a phase split, and it still leaves the agent singleton graph fully populated.

Concretely:
- **Delete** the `LaunchMode::Tui` short-circuit at `app/src/lib.rs:1079-1084` so TUI flows through `initialize_app` like every other mode.
- Inside `initialize_app`, gate the heavyweight, terminal-specific work behind a non-TUI check — anything that spawns a process or PTY or builds the GUI workspace: the terminal-server process, the default terminal / `ActiveSession` (the PTY-backed session), and the GUI `launch(...)` workspace build. Everything else (settings, auth/server, persistence, workspaces/API managers, cloud/sync, MCP, and the agent models the send path needs) runs unchanged.
- For TUI, run `tui::init` in place of the GUI `launch(...)`, registering `CoreTuiModel` (§4) and creating the root TUI window/view (whose `EntityId` seeds the root `AgentSessionOwnerId`). Remove the `LaunchMode::Tui => unreachable!()` arm in `launch` (`app/src/lib.rs:2645`).

The send path needs the singleton graph, not a live terminal session: the TUI supplies its own local `SessionContext` (§7) instead of an `ActiveSession`.

#### Risks / constraints
- **Preserve ordering.** `initialize_app`'s registration order is load-bearing (AI models subscribe to `UpdateManager`; `BlocklistAIHistoryModel::new` consumes restored sqlite bundles; `RequestParams::new` reads `UserWorkspaces`/`ApiKeyManager`/MCP). Gating the heavyweight pieces must not reorder the rest.
- **Settle the skip list by compiling the TUI path.** Start by skipping only the terminal-server process, the default terminal/`ActiveSession`, and the GUI workspace `launch(...)`. If a skipped manager turns out to be required by `ResponseStream` + `RequestParams::new` + `BlocklistAIHistoryModel`, pull just that one back in rather than un-skipping the heavyweight work.
- **Preserve cfg gates.** Keep existing `#[cfg(feature = "local_tty"/"local_fs")]` / platform gates intact.

### 4. Add the `CoreTuiModel` singleton
The TUI app's stateful agent-session owner — the closest analog to the Agent Mode state `TerminalView` holds today, but as one app-level singleton. For the prototype there is a **single session** (orchestration is out of scope), owned by the root TUI view. Conversation/transcript content stays in the shared `BlocklistAIHistoryModel`; `CoreTuiModel` owns only the per-session pointers into history and the in-flight stream, and delegates all send/stream work to the shared `AgentConversationEngine` (§2).
```rust
/// App-level singleton owning the TUI app's single agent session.
///
/// Single-session for the prototype (orchestration is out of scope); the
/// session is owned by the root TUI view. Conversation content lives in
/// `BlocklistAIHistoryModel`; send/stream work is delegated to the shared
/// `AgentConversationEngine` (§2).
pub struct CoreTuiModel {
    /// Owner of the single session (the root view); `None` until registered.
    /// History is keyed on `owner.entity_id()`.
    owner: Option<AgentSessionOwnerId>,
    /// Conversation the next prompt follows up in / the last one streamed to.
    /// `None` until the first prompt starts a conversation.
    active_conversation_id: Option<AIConversationId>,
    /// The in-flight stream while a response is streaming, created and folded
    /// into history by `AgentConversationEngine`; `None` when idle.
    in_flight: Option<ModelHandle<ResponseStream>>,
}

impl Entity for CoreTuiModel {
    type Event = CoreTuiModelEvent;
}

impl SingletonEntity for CoreTuiModel {}
```
Public API (single session, so no per-call `owner`):
```rust
impl CoreTuiModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self;

    /// Registers the single session's owner (the root view). Idempotent.
    pub fn register_session(
        &mut self,
        owner: AgentSessionOwnerId,
        ctx: &mut ModelContext<Self>,
    );

    /// Sends `prompt`: starts a new conversation if there is none, otherwise
    /// follows up in the active conversation. Errors if a request is already in
    /// flight. Delegates the send/stream to the shared `AgentConversationEngine` (§2).
    pub fn send_prompt(
        &mut self,
        prompt: String,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<(AIConversationId, ResponseStreamId)>;

    /// Cancels the in-flight request, if any.
    pub fn cancel_active_request(&mut self, ctx: &mut ModelContext<Self>);

    pub fn active_conversation_id(&self) -> Option<AIConversationId>;

    pub fn has_in_flight_request(&self) -> bool;
}
```
Events (so future transcript/input views observe instead of polling):
```rust
pub enum CoreTuiModelEvent {
    /// A prompt was accepted and a request was sent.
    PromptSubmitted { conversation_id: AIConversationId },
    /// Streamed output mutated the conversation (drives transcript repaints).
    ConversationUpdated { conversation_id: AIConversationId },
    /// The active request reached a terminal state (success or error).
    RequestFinished { conversation_id: AIConversationId },
}
```
Shared singletons it reads (all registered by `initialize_app` per §3): `BlocklistAIHistoryModel` (conversation state); `LLMPreferences` + `AIExecutionProfilesModel` (model ids / context window); `BlocklistAIPermissions` + `AISettings` (request settings); `ServerApiProvider` (transport, via `ResponseStream`); `NetworkStatus` (retry behavior). Requests go through the shared `AgentConversationEngine` (§2), which creates the `ResponseStream`.

### 5. The minimal request path
`CoreTuiModel::send_prompt` reuses the request-construction helpers and the shared `AgentConversationEngine` (§2), so it adds no bespoke send/stream logic:
1. Resolve `active_conversation_id`, or start a new conversation via `BlocklistAIHistoryModel::start_new_conversation(owner.entity_id(), …)`.
2. Build context via the TUI context builder (§7) and an `AIAgentInput::UserQuery`.
3. Build `RequestInput`, then `api::RequestParams::new(...)` with `supported_tools_override: Some(vec![])` (§6).
4. Hand the request to `AgentConversationEngine` with local hooks (`SkillPathOrigin::Local`, no shared-session forwarder, no action sink). `AgentConversationEngine` creates the `ResponseStream`, appends the user exchange, marks the conversation `InProgress`, subscribes, and folds events into history — including streamed agent text via `ClientActions` (`AddMessagesToTask` / `AppendToMessageContent` carrying `AgentOutput`), which arrives even with tools disabled.
5. Record the returned `ResponseStream` as `in_flight`; clear it and emit `RequestFinished` when the engine reports the stream finished.

The follow-up reuses `active_conversation_id`, so step 3 sends the prior `ConversationData` (tasks + server token), giving the second request the first request's context. Because the fold is `AgentConversationEngine`'s — identical to the GUI controller's — the TUI and GUI cannot drift.

### 6. No client tools in phase one (and why we don't hold an action model)
Pass `supported_tools_override: Some(vec![])` so the server advertises no client tools. This scopes the first milestone to **text exchange and follow-up continuity**. Streamed agent *text* still arrives via `ClientActions` (`AddMessagesToTask` / `AppendToMessageContent`) and is folded into history by `AgentConversationEngine` (§5); that path needs no action model.

The review asked whether `CoreTuiModel` should hold a `BlocklistAIActionModel` alongside the controller. For phase one the answer is **no, and holding one would be premature**:
- With no tools advertised, no executable client actions arrive, so an action model would have nothing to do.
- `BlocklistAIActionModel` is itself terminal-coupled: its `new()` (`action_model.rs:247`) takes a `TerminalModel` + `ActiveSession` purely to eagerly build the full executor cluster, including the `ShellCommandExecutor` (`shell_command.rs:38`) that runs commands as blocks in the terminal grid. Holding it would re-introduce exactly the coupling §2 removes.

Instead, `AgentConversationEngine` exposes a `ClientActions` **action sink** (§2): today the GUI controller plugs in its `BlocklistAIActionModel`; the TUI plugs in nothing. Nothing in the shared send/stream path hard-codes "no actions" assumptions.

**When we add tools.** This is a proof of concept, so we are **not** building the full action layer. The milestone ships zero tools; the first increment, if any, is a single neutral tool (e.g. `read_files`) — not the full set. The agreed seam, for when tools land:
- **Inject the executor set** into `BlocklistAIActionModel` (drop its `terminal_model`/`active_session` params) via `BlocklistAIActionExecutor::for_gui(...)` / `::for_tui(...)` builders, so the action model's queue/phase/permission logic stays shared and surface-agnostic.
- **Executors depend on an `AgentSession` enum** — `Terminal(ModelHandle<ActiveSession>)` (GUI) / `Tui(ModelHandle<TuiSession>)` (TUI) — exposing the small surface the neutral executors actually use (current working directory, path resolution, shell type). `TuiSession` is a new, **local-only** model (cwd + shell; no remote/host). The enum keeps executors on one concrete `ModelHandle<AgentSession>` with exhaustive matching (no `dyn`), which fits this entity system.
- **Shell behind a `ShellRunner` trait** — GUI runs commands as terminal-grid blocks; the TUI provides its own `for_tui` command runner (execute + capture output) implementing the same `ActionExecution` contract. This is the one genuinely surface-specific executor.
- **Move the action-result→follow-up loop into `AgentConversationEngine`** (it currently lives in the controller, `controller.rs:442-558`) so both surfaces share it. Shape the action sink as the controller's existing contract — *client actions in → finished action results out → drives the follow-up* — so adding tools is "implement the sink," not re-threading the stream path.

Most executors are already backend-neutral (`read_files`, `file_glob`, `grep`, `request_file_edits`, `search_codebase`, MCP, documents, `ask_user_question`, `run_agents`) and need the filesystem/MCP/codebase singletons we already init (§3), not a terminal. The only true blocker today is that `BlocklistAIActionExecutor::new` (`execute.rs:280`) builds the whole cluster eagerly against a terminal `ActiveSession`; the `AgentSession` enum + executor-set injection remove that, leaving shell as the lone new executor.

### 7. A small TUI context builder
Rather than depending on `BlocklistAIContextModel`, add a focused builder that returns the `Arc<[AIAgentContext]>` and `SessionContext` a TUI query needs:
```rust
/// Builds the request context for a TUI agent query. Phase one includes only
/// session-independent context; richer context (attachments, selections,
/// project rules) can be layered in later without changing call sites.
pub struct TuiAgentContextBuilder;

impl TuiAgentContextBuilder {
    /// Directory + current time + execution environment context.
    pub fn context(app: &AppContext) -> Arc<[AIAgentContext]>;

    /// A local, non-terminal `SessionContext` (no terminal session/shell).
    pub fn session_context(app: &AppContext) -> SessionContext;
}
```
Phase-one context: current working directory + home, current time, and execution-environment (OS/shell) where available.

Explicitly excluded for now: terminal block selection, selected terminal text, pending attachments, long-running command snapshots.

`SessionContext`'s fields are private and it only has terminal/test constructors today, so this needs a small additive constructor (e.g. `SessionContext::local(cwd)`). That is preferable to widening the existing terminal constructor.

## Testing strategy

### Primary end-to-end model test (no view tree)
Drives `CoreTuiModel` directly using a synthetic `AgentSessionOwnerId` from a fresh `EntityId` (no `TerminalView`, no `RootTuiView`, no runtime/driver), proving conversation ownership is fully decoupled from views.
- Hits the **real** agent backend, authenticated with the warp-dev credential (no auth flow is built in the prototype). The shared `AgentConversationEngine` (§2) drives the live `ResponseStream`; no fake stream is emitted.
- Asserts that `CoreTuiModel`:
    - Registers as a singleton and tracks state under the `AgentSessionOwnerId`.
    - Starts a new conversation on the first prompt and appends the user query + the real streamed agent text to `BlocklistAIHistoryModel`.
    - Marks the conversation successful after the stream finishes.
    - Sends the second prompt as a **follow-up in the same conversation**, with the second `RequestParams` carrying the first request's task/server context.

### Engine-parity regression test
After `BlocklistAIController` is refactored onto the shared `AgentConversationEngine`, the existing controller / Agent Mode tests must still pass unchanged — proving the GUI send/stream path is behavior-preserving. Add direct coverage that `AgentConversationEngine` folds `Init` / `ClientActions` / `Finished` / error events into `BlocklistAIHistoryModel` correctly given an owner id + conversation id.

### Initializer coverage
Verify that `LaunchMode::Tui` runs the full `initialize_app` singleton bootstrap (minus the heavyweight terminal pieces) **before** registering `CoreTuiModel`. No need to launch the terminal UI — just prove the models required by request construction exist in a test app context.

## Non-goals
- **No final UI yet:** no transcript rendering, scrollable transcript, text input editor, shell-command interleaving, or model selector. This plan prepares the state/request pipeline those PRs read and write.
- **No `BlocklistAIActionModel` / tool execution.** Pass empty supported tools. Leave a seam (the `AgentConversationEngine` action sink); don't build the action layer. See §6 for why holding an action model now would be premature and the planned path for tools.
- **No wholesale rename** of `BlocklistAIHistoryModel`'s `terminal_view_id`-named APIs/events. Not needed for the prototype: the shared engine passes the owner id through to the existing APIs. Such a rename can be a later cleanup.

## Sequencing
1. **Shared agent engine** — introduce `AgentSessionOwnerId` and extract `AgentConversationEngine` (§2) from `BlocklistAIController`, refactoring the controller onto it (with its terminal hooks). Behavior-preserving for the GUI. (Engine-parity regression test.)
2. **Initializer** — route `LaunchMode::Tui` through `initialize_app`, gating the heavyweight terminal pieces; run `tui::init` in place of the GUI `launch(...)`. (Initializer coverage.)
3. **`CoreTuiModel` + context builder + request path** — text-only, no tools. (Primary end-to-end model test.)

Later PRs (out of scope): transcript view, input view, action/shell execution.

## Open decisions before implementation
- **`AgentSessionOwnerId`:** resolved — a newtype over the owning view's `EntityId`, defined near the agent conversation/history types (per-view, not per-model, to support future sub-views). It is the owner key the shared `AgentConversationEngine` and `CoreTuiModel` use; it converts to the `EntityId` the existing history APIs take, so no history rename is needed.
- **Agent logic reuse (shared engine vs. controller):** resolved — extract a shared `AgentConversationEngine` (Option B) and refactor `BlocklistAIController` onto it; the TUI reuses that single path with local hooks rather than holding the controller / action model or hand-rolling its own send/stream logic.
- **Action-layer seam (for tools later):** resolved — when tools are added, the action model's executor set is injected via `for_gui`/`for_tui` builders, and executors depend on an `AgentSession` enum (`Terminal(ActiveSession)` / `Tui(TuiSession)`, TUI **local-only**), with shell behind a `ShellRunner` trait. This is a proof of concept: the milestone ships zero tools and a first tool (if any) would be a single neutral one — not the full action layer. See §6.
- **How far sharing goes (controller vs. engine):** open — both end states give the TUI full parity: **E1** one shared `BlocklistAIController` with every collaborator abstracted (per-surface impls; the TUI holds a controller, no separate `CoreTuiModel`), or **E2** a shared `AgentConversationEngine` with thin per-surface adapters (GUI controller + `CoreTuiModel`). Step one is identical for both (extract `AgentConversationEngine`, abstract collaborators incrementally), so this is deferred. Principle: carve **bottom-up** (grow `AgentConversationEngine`, shrink the controller toward a GUI adapter), never a `mode` flag branched through the controller.
- **Initialization factoring boundary:** resolved — run the full `initialize_app` for `LaunchMode::Tui`, gating only the heavyweight terminal pieces (terminal-server process, default terminal/`ActiveSession`, GUI workspace `launch(...)`). Settle the exact skip list when compiling the TUI path.
- **`SessionContext` constructor:** needs a small additive local (non-terminal) constructor; confirm shape during implementation.
- **Auth + server for the test:** resolved — the prototype builds no auth flow; the end-to-end test reuses the warp-dev credential and exercises the real server through the shared `AgentConversationEngine` (§2). The test is network-dependent by design.
