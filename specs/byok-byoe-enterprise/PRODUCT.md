# Enterprise BYOK / BYOE

## Summary
Let enterprise teams use their own LLM providers in Warp. A team admin configures shared API keys and OpenAI-compatible custom endpoints (e.g. OpenRouter, LiteLLM, Samba) once in the Warp Admin Panel, and those providers appear automatically in every member's model picker. Admins can also choose whether members may add their own local keys and endpoints. Team-managed provider keys are stored by Warp, so they work for both interactive agent requests and Oz cloud agents; member-managed providers stay on the member's device and work only for interactive requests.

## Problem
Today BYOK and BYO-endpoint are self-serve only: each user pastes keys locally, nothing is stored, and those providers can't be shared across a team or used by cloud agents. Enterprises want to (a) provision approved providers for the whole team centrally, (b) optionally let members bring their own, and (c) be confident inference actually goes to their providers. A recurring admin complaint is configuring BYO but still being able to pick Warp-managed models, or silently falling back to them.

## Goals
- A team admin configures shared keys and custom endpoints once; they appear for all members.
- Admins control whether members may add their own keys/endpoints.
- Team-managed providers work for both interactive agent requests and Oz cloud agents. User-managed providers work for interactive agent requests.
- Make it obvious how to keep inference on the team's providers (turn off Direct API and Warp credit fallback).
- Leave the existing self-serve BYOK/BYOE experience unchanged for non-enterprise users.

## Non-goals
- Member-managed (local, device-only) providers powering cloud agents.
- Changing self-serve (non-enterprise) BYOK/BYOE behavior.

## Design
A [Loom walkthrough](https://www.loom.com/share/6eb6e0f8f0764d2a9e8ec4f886957e63) with basic flows. Treat the Loom as the visual reference for the admin section and the member settings layout; this Behavior section is the source of truth for behavior.

## Behavior

### Roles and terms
- "Team admin" is a member with admin/owner permissions; only admins see and edit the Admin Panel. 
- "Team member" is any member; members see the in-app settings surface. 
- A "team-managed" provider is a key or endpoint an admin configured in the Admin Panel - stored by Warp and shared with the whole team. 
- A "user-managed" provider is a key or endpoint a member added in their own Warp settings - stored only on that member's device.

### Admin Panel — Models page (Enterprise Team Admin)
- The Models page gains a "Bring Your Own Keys & Endpoints" section alongside the existing Direct API and AWS Bedrock sections. The section has a master enable toggle. When off, no team-managed providers are active for the team and the section's configuration controls are hidden/disabled. When on, the admin can configure team keys and endpoints and they become available to members. When the admin enables team-managed BYO (adds keys/endpoints) while Direct API is still enabled, the section surfaces a prominent prompt explaining that members can still select Warp-managed models, with a one-click affordance to turn off Direct API.
- Team API keys: the admin can paste a key for each first-party provider Warp supports for BYOK (e.g. OpenAI, Anthropic, Google). Each provider row shows whether a key is currently set without revealing the stored value. Saving a key persists it for the team; clearing a row removes that team key.
- Team custom endpoints: the admin can add one or more OpenAI-compatible Chat Completions endpoints. Each endpoint has a name, URL, API key, and one or more models, where each model has a model name (sent to the endpoint) and an optional alias (shown to members). The admin can add, edit, and remove endpoints.
  - Endpoint validation: an endpoint cannot be saved without a name, a valid URL, an API key, and at least one model with a non-empty model name. Invalid fields are indicated inline and block saving (same behaviour as current client). 
- Secrets entered by the admin (provider keys and endpoint API keys) are never displayed back in plaintext after saving — to the admin or to members. They render as a masked/"set" state.
- "Allow users to bring their own models" toggle: when on, members may add their own local keys and custom endpoints in their Warp settings; when off, the member-facing self-serve BYO UI is disabled.
- Cloud-agent note: the section states that team-managed keys and endpoints are stored by Warp and are therefore available to Oz cloud agents, unlike member-managed providers which never leave the member's device.
- Saving any team-managed configuration propagates to members: the next time a member's client loads team settings, the team-managed providers (and the "allow users" permission) reflect the admin's latest saved state.
- Validation errors from saving (e.g. malformed endpoint) are shown to the admin and the prior saved state is preserved until a valid save succeeds.

### Team member — Warp settings surface
- The member's AI/Custom Inference settings present two clearly distinct groups: "Provided by your team" (team-managed, read-only) shown first, and "User added keys" (the member's own, editable) shown below. Both groups use the same visual layout (provider key rows and endpoint cards). _The API key section of the "Provided by your team" will not show the redacted API key, instead just whether the API key is configured (checkmark) or not, and whether it's active or overridden by the "user added keys"_. 
- The "Provided by your team" group lists the enabled and disabled team-managed providers and team-managed endpoints (name + model chips). It is read-only: members cannot edit, add, or remove these entries, and the stored secret values are never shown.
- The team group includes a short explanation that these were configured by the team admin, are shared with everyone, and also power cloud agents.
- The "User added keys" group is the existing self-serve BYOK/BYOE experience: paste provider keys, and add/edit/remove custom endpoints (name, URL, API key, model name + alias). It carries a one-line description clarifying that these stay on the member's device and are not available to cloud agents.
- When the admin's "Allow users to bring their own models" is off, the "User added keys" group is visibly disabled (controls non-interactive and dimmed) with an explanation that the team admin has turned it off; the member can still use the team-provided providers above. Any keys/endpoints the member previously saved locally are not editable while disabled, but will persist in the event the enterprise admin switches this setting back on. 
- When "Allow users to bring their own models" is on (and the member's plan permits BYO), the "User added keys" group is fully interactive as it is for self-serve users today (excluding the SuperGrok and Warp credit fallback toggles). 

### Model picker and routing behaviour/logic
- Team-managed models (each team endpoint's models, shown by alias when set, and the providers the team supplies) appear in the member's model picker.
- User-managed models also appear in the picker when "Allow users to bring their own models" is on, exactly as in the self-serve experience.
- With Direct API off, a member's request to a configured team or user provider does not fall back to Warp-managed inference; if the provider is unreachable or misconfigured the request surfaces an error rather than silently using Warp credits.
- Currently, custom-endpoint models appear in the picker as `<model alias> (Custom • <endpoint name>)`. Since an enterprise admin may set endpoint name `a`, and the user may also set an endpoint with name `a`, we always disambiguiate by showing Team-provided endpoints as `<model alias> (Team • endpoint name)`, and user-provided endpoints as `<model alias> (Custom • endpoint name)`. i.e. we are doing Team provided custom endpoints `UNION` User provided custom endpoints. 
- First-party provider keys (OpenAI/Anthropic/Google) never create duplicate picker entries: a member's own key for a provider takes precedence over the team key for that provider, so the standard model appears once — routing through the member's key when set, otherwise the team key. The user will be able to toggle a "Use my api key when team-provided key and user-provided key" are both present (on by default). 

### Cloud agents (Oz)
- Team-managed keys and endpoints are usable by Oz cloud agents for that team: a cloud agent run can perform inference through the team's configured providers without any per-member device state.
- Member-managed (local) keys and endpoints are never used by cloud agents; they exist only on the member's device for interactive requests.
- The product communicates this distinction in both surfaces (admin "available to cloud agents" note, member "stays on your device" note) so the difference in cloud-agent availability is never surprising.

### States, edge cases, and invariants
- Non-enterprise/self-serve users see no change: the existing single BYOK/BYOE settings experience is preserved, with no "Provided by your team" group.
- Disabling the section master toggle (3) removes team-managed providers from members' pickers and hides the member "Provided by your team" group on their next settings/picker load; it does not delete the member's own user-managed providers.
- Turning "Allow users to bring their own models" off does not delete a member's locally stored keys/endpoints; it disables the UI and prevents their selection/use until re-enabled.

## Open Questions

- Should admins be able to prevent team members from adding their own API key and Custom Endpoints separately? (i.e. it's okay for users to add their own API key, but it's not okay for them to add their own Custom Endpoint). Do we need separate toggles, or should we just treat these the same?
- Should we default to "Allow users to bring their own models" for a newly enabled team?
