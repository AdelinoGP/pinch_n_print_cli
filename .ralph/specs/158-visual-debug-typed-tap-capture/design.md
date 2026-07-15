# Design: 158-visual-debug-typed-tap-capture

## Controlling Code Paths

- Primary code path: the model-backed `pnp_cli visual-debug` execution path from packet 157 into the runtime executor's fixed stage dispatch and post-stage commit boundaries.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/visual_debug_typed_tap_capture_tdd.rs` and the smallest existing model-backed visual-debug fixture established by packet 157.

## Architecture Constraints

- Taps are runtime-owned, request-gated, and read typed post-stage/post-host-hook state; they do not create scheduler edges, module invocations, module-visible access, WIT APIs, or manifest IR access.
- Blackboard IR is immutable during per-layer work; per-layer IR is borrowed at the executor boundary and copied into renderer-owned capture data before `LayerArena` release.
- The fixed scheduler order and four-phase execution remain authoritative. The closure may stop at the furthest selected tap, while correctness-required extra execution is recorded as expansion.
- Capture failure is fatal to the visual-debug product; no partial capture is reported as successful. Existing progress-event ordering and required failure visibility remain intact.
- This packet does not add rendering, G-code parsing, coordinate logic, WASM, OrcaSlicer parity, or agent-skill behavior.

## Code Change Surface

- Selected approach: add a request-gated typed tap registry with one adapter per documented tap, invoke it only at committed executor boundaries, and pass renderer-owned capture records through packet 157's exported manifest/bundle model.
- Exact functions, traits, manifests, tests, and fixtures: packet 157's exported visual-debug command/request/manifest seam; the minimal `crates/pnp-cli` command-to-runtime dispatch seam; runtime executor stage-boundary hooks; typed adapter registry and capture record types; `visual_debug_typed_tap_capture_tdd.rs`; packet 157's model-backed fixture/request helpers. Exact symbol names must be confirmed by the packet-157 export dispatch before implementation.
- Rejected alternatives and reasons: per-module snapshots are rejected by ADR-0037 because they violate host ownership and bounded deterministic capture; a debug WASM module is rejected because it adds an unnecessary access/ownership contract; post-hoc G-code-only observation cannot localize the introducing stage; unconditional tap allocation would violate the zero ordinary-slice overhead requirement.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/` - role: runtime command/executor and capture integration; expected change: add request-gated closure execution and post-stage typed capture.
- `crates/pnp-cli/` - role: visual-debug command-to-runtime dispatch; expected change: wire packet 157's parsed/validated model-backed request into packet 158's runtime capture execution, without moving parsing or validation into this packet.
- `crates/slicer-runtime/tests/visual_debug_typed_tap_capture_tdd.rs` - role: focused contract coverage; expected change: add positive, lifecycle, determinism, and negative tests.
- The packet-157-owned request/manifest source file identified by the export dispatch - role: integration seam; expected change: only additive capture fields required to carry renderer-owned typed captures and execution expansion.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 99-110, 143-163, 180-213 only - purpose: closure, lifetime, adapter, and documented source-field contract.
- `docs/01_system_architecture.md` - lines 65-109, 246-500, 567-665 only - purpose: fixed stages, tier ownership, postpass ownership, I/O, and arena lifetime.
- `docs/09_progress_events.md` - lines 56-109, 116-143 only - purpose: required event fields, ordering, backpressure, and failure semantics.
- Packet 157's packet artifacts - delegated bounded symbol lookup only - purpose: exact exported request/manifest types and integration seam.

## Out-of-Bounds Files

- `modules/`, `crates/slicer-schema/wit/`, module manifests, and IR schema definitions - no contract changes are permitted.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and guest WASM artifacts - never load or edit.
- PNG/raster/rendering code and final G-code parser/renderer surfaces - owned by packets 159 and 160.
- `crates/pnp-cli/` command parsing, request validation, bundle lifecycle, overwrite behavior, and base manifest semantics - owned by packet 157; packet 158 may edit only the minimal command-to-runtime dispatch seam.
- `.claude/skills/`, `docs/17_agent_debugging.md`, and any agent-skill surface - owned by packet 161 or existing tooling.
- OrcaSlicer source or documented references - no parity scope applies.

## Expected Sub-Agent Dispatches

- Question: What exact public request, tap, manifest, and bundle integration symbols does packet 157 export, and where are they defined? Scope: `.ralph/specs/157-visual-debug-request-bundle-contract/**`. Return: `LOCATIONS` with at most 20 `file:line` entries and one context line each. Purpose: establish the dependency seam without inventing names.
- Question: Which executor functions are the post-stage/post-host-hook commit boundaries for each documented tap family, and what typed IR borrow is available there? Scope: `crates/slicer-runtime/src/**` plus bounded symbol search in `crates/slicer-ir/src/**`. Return: `LOCATIONS` with at most 20 entries. Purpose: place adapters without browsing unrelated code.
- Question: Does the focused typed-tap test target already exist, and what smallest existing fixture/helper can drive a model-backed request? Scope: `crates/slicer-runtime/tests/**`. Return: `FACT` in 5 lines or fewer. Purpose: choose test edits without fabricating fixture paths.

## Data and Contract Notes

- IR/manifest contracts: adapters consume the exact documented source fields from `docs/specs/visual-pipeline-debug.md`; packet 157 owns the request and manifest model; capture records are renderer-owned and do not alter IR schemas.
- WIT boundary: unchanged. No capture data crosses a module boundary and no module receives a new read capability.
- Determinism/scheduler constraints: preserve fixed stage order, deterministic selected tap/layer ordering, bounded selected-layer retention, and explicit manifest expansion reasons.

## Locked Assumptions and Invariants

- Packet 157 is the sole owner of request parsing, source mode, bundle lifecycle, overwrite behavior, and base manifest semantics.
- Capture occurs only after the selected stage's host hook and commit boundary.
- A successful visual-debug run never contains dangling arena borrows, unrequested retained snapshots, or silently omitted selected taps.
- Ordinary `pnp_cli slice` does not allocate, serialize, or invoke visual-debug capture machinery.

## Risks and Tradeoffs

- The packet-157 export seam is not yet present in this workspace; an incorrect assumed symbol would create contract duplication, so implementation must stop at the bounded export lookup if the seam is not available.
- A tap's correctness dependencies may require extra layers or whole-print work; recording expansion in the manifest preserves explainability but must not retain those unselected captures.
- The adapter inventory is broad; each adapter should remain a thin typed projection so IR schema drift causes compile/test failures rather than silent field loss.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: packet-157 export lookup; `LOCATIONS` at most 20 entries.

## Open Questions

- [BLOCK] What exact exported type names and source file does packet 157 provide for the request, selected tap, bundle manifest, and typed capture entry? Resolve by the required packet-157 export dispatch before activation.
- [FWD] Which existing executor helper is the narrowest stable seam for invoking all post-stage adapters without duplicating stage dispatch? Resolve by bounded runtime symbol lookup during implementation.
