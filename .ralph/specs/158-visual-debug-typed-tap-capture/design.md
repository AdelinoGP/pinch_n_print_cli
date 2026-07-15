# Design: 158-visual-debug-typed-tap-capture

## Controlling Code Paths

- Primary code path: `crates/pnp-cli/src/visual_debug.rs::run_visual_debug` (the packet-157 handler dispatched from `crates/pnp-cli/src/main.rs:437-444`, `Cmd::VisualDebug`) for the `Model` source variant, calling into a new `slicer-runtime` capture API and assembling the resulting typed captures into packet-157's own `Manifest`/`ImageEntry` structs.
- Grounded fact (packet 157 is now implemented, commit `3e33ca01`): `run_visual_debug`'s `Model` branch currently performs **no real execution** — it builds one placeholder `ImageEntry` per `tap x visualization` from `req.taps`/`req.layers.first()` with no scheduler run, no tap validation, and no layers beyond the first. This packet is the first to actually invoke the runtime pipeline for a visual-debug request.
- Neighboring tests/fixtures: `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` (new) alongside the existing `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` (packet 157's own contract tests and fixture conventions).

## Architecture Constraints

- Taps are runtime-owned, request-gated, and read typed post-stage/post-host-hook state; they do not create scheduler edges, module invocations, module-visible access, WIT APIs, or manifest IR access.
- Blackboard IR is immutable during per-layer work; per-layer IR is borrowed at the executor boundary and copied into renderer-owned capture data before `LayerArena` release.
- The fixed scheduler order and four-phase execution remain authoritative. The closure may stop at the furthest selected tap, while correctness-required extra execution is recorded as expansion.
- Capture failure is fatal to the visual-debug product; no partial capture is reported as successful. Existing progress-event ordering and required failure visibility remain intact.
- This packet does not add rendering, G-code parsing, coordinate logic, WASM, OrcaSlicer parity, or agent-skill behavior.

## Code Change Surface

- Selected approach: add a request-gated typed tap registry with one adapter per documented tap in `slicer-runtime` (new pub API, e.g. a `visual_debug_capture` module), invoked only at committed executor boundaries; `crates/pnp-cli`'s `run_visual_debug` calls this new API for the `Model` source and assembles the renderer-owned capture results into its own `Manifest`/`ImageEntry` values. `slicer-runtime` cannot import `pnp-cli`'s types (dependency direction is `pnp-cli -> slicer-runtime`), so the new runtime API must be expressed in runtime-owned/`slicer-ir`-owned types, translated by `pnp-cli`.
- Exact functions, traits, manifests, tests, and fixtures: `crates/pnp-cli/src/visual_debug.rs` (`VisualDebugRequest`, `VisualDebugSource::Model`, `TapSelector`, `LayerSelector`, `Manifest`, `ImageEntry`, `run_visual_debug` at lines 257-370); the new `slicer-runtime` capture-execution entry point and typed adapter registry; `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` (new, follows the `visual_debug_request_bundle_tdd.rs` convention); `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` as a read-only fixture/pattern reference. Exact new runtime-side symbol names must still be confirmed by a bounded executor-boundary dispatch (Step 3) before implementation; the packet-157 seam itself is now resolved (see below).
- Rejected alternatives and reasons: per-module snapshots are rejected by ADR-0037 because they violate host ownership and bounded deterministic capture; a debug WASM module is rejected because it adds an unnecessary access/ownership contract; post-hoc G-code-only observation cannot localize the introducing stage; unconditional tap allocation would violate the zero ordinary-slice overhead requirement; mirroring packet 157's request/manifest types inside `slicer-runtime` is rejected because it would duplicate the wire contract instead of consuming it.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/` - role: new pub capture-execution API, typed adapter registry, and post-stage commit-boundary hooks; expected change: add request-gated closure execution and post-stage typed capture, returning renderer-owned capture values in runtime/`slicer-ir`-owned types (no pnp-cli type dependency).
- `crates/pnp-cli/src/visual_debug.rs` - role: packet-157-owned request/manifest/bundle model and the `run_visual_debug` handler; expected change: replace the current no-op `Model`-source placeholder-image-entry loop with a call into the new `slicer-runtime` capture API, tap validation against the documented tap inventory, iteration over all selected layers (not just `req.layers.first()`), and assembly of real `ImageEntry` values plus execution-expansion/failure reporting. This is the minimal command-to-runtime dispatch seam; parsing, validation, bundle lifecycle, overwrite behavior, and base manifest semantics stay as packet 157 left them.
- `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` - role: focused contract coverage (new file, mirrors `visual_debug_request_bundle_tdd.rs`); expected change: add positive, lifecycle, determinism, and negative tests driven through `run_visual_debug`/the CLI request path.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 99-110, 143-163, 180-213 only - purpose: closure, lifetime, adapter, and documented source-field contract.
- `docs/01_system_architecture.md` - lines 65-109, 246-500, 567-665 only - purpose: fixed stages, tier ownership, postpass ownership, I/O, and arena lifetime.
- `docs/09_progress_events.md` - lines 56-109, 116-143 only - purpose: required event fields, ordering, backpressure, and failure semantics.
- `crates/pnp-cli/src/visual_debug.rs` (complete, 381 lines) - purpose: exact packet-157 request/manifest/bundle types and the current no-op `Model`-source placeholder logic this packet replaces.
- `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` - purpose: existing fixture/test conventions for the visual-debug CLI contract.

## Out-of-Bounds Files

- `modules/`, `crates/slicer-schema/wit/`, module manifests, and IR schema definitions - no contract changes are permitted.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and guest WASM artifacts - never load or edit.
- PNG/raster/rendering code and final G-code parser/renderer surfaces - owned by packets 159 and 160.
- `crates/pnp-cli/src/main.rs` command parsing, `crates/pnp-cli/src/visual_debug.rs` request validation (`validate_request`), bundle lifecycle, overwrite behavior, and base manifest field set - owned by packet 157; packet 158 may only add to `run_visual_debug`'s `Model`-source body and populate additive `ImageEntry`/`Manifest` fields, never touch `validate_request`, `VisualDebugRequest`, or the bundle create/overwrite/atomic-write logic.
- `.claude/skills/`, `docs/17_agent_debugging.md`, and any agent-skill surface - owned by packet 161 or existing tooling.
- OrcaSlicer source or documented references - no parity scope applies.

## Expected Sub-Agent Dispatches

- Question: Which executor functions are the post-stage/post-host-hook commit boundaries for each documented tap family, and what typed IR borrow is available there? Scope: `crates/slicer-runtime/src/**` plus bounded symbol search in `crates/slicer-ir/src/**`. Return: `LOCATIONS` with at most 20 entries. Purpose: place adapters without browsing unrelated code.
- Question: What is the narrowest new `slicer-runtime` pub entry point (function/module) that `crates/pnp-cli/src/visual_debug.rs::run_visual_debug` can call to run the fixed-stage dependency closure and receive renderer-owned typed capture values, given `slicer-runtime` cannot depend on `pnp-cli`? Scope: `crates/slicer-runtime/src/lib.rs` pub surface (`execution_plan`, `layer_executor`, `run`, `blackboard`) plus `crates/pnp-cli/src/visual_debug.rs`. Return: `LOCATIONS` at most 20 entries. Purpose: fix the exact seam shape before writing adapters.

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

- `run_visual_debug`'s `Model` branch has no tap validation today (any `TapSelector` name silently "succeeds"); the new tap registry must reject unknown taps itself (AC-N2) rather than assume packet 157 already gates this.
- `layer_info` currently reads only `req.layers.first()`; the implementation must iterate all selected layers instead of preserving this single-layer shortcut, or AC-3's multi-layer retention cannot be satisfied.
- A tap's correctness dependencies may require extra layers or whole-print work; recording expansion in the manifest preserves explainability but must not retain those unselected captures.
- The adapter inventory is broad; each adapter should remain a thin typed projection so IR schema drift causes compile/test failures rather than silent field loss.
- `slicer-runtime` cannot depend on `pnp-cli`'s `Manifest`/`ImageEntry`/`VisualDebugRequest` types (dependency direction is `pnp-cli -> slicer-runtime`); the new runtime capture API must be expressed in runtime/`slicer-ir`-owned types and translated by `pnp-cli`, not by mirroring or importing packet 157's types into `slicer-runtime`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: narrowest `slicer-runtime` capture entry point for `run_visual_debug` to call; `LOCATIONS` at most 20 entries.

## Open Questions

- [FWD] Which existing executor helper is the narrowest stable seam for invoking all post-stage adapters without duplicating stage dispatch, and what exact runtime-owned type should carry a capture result back to `crates/pnp-cli/src/visual_debug.rs`? Resolve by bounded runtime symbol lookup during implementation (Step 1).
