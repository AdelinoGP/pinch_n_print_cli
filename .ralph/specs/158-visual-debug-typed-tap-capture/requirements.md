# Requirements: 158-visual-debug-typed-tap-capture

## Packet Metadata

- Grouped task IDs: `TASK-268`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

Packet 157 establishes the opt-in visual-debug command and exports the versioned request and bundle-manifest model, but it does not observe typed intermediate IR. TASK-268 supplies the missing runtime seam: selected post-stage, post-host-hook values must be captured without adding module-visible access or retaining unbounded layer snapshots. The implementation must consume packet 157's exported request/manifest model rather than define a parallel request, source-mode, tap, or manifest contract.

## In Scope

- Consume packet 157's exported request model, including its model-backed source mode, selected layers, and typed tap identities.
- Consume and extend packet 157's exported bundle-manifest model only as needed to record typed capture identity, layer, source schema version, execution expansion, and capture failure state.
- Register typed adapters for every tap listed in the visual-pipeline-debug stage inventory, using the documented source fields and real host IR types.
- Capture only after the corresponding scheduler stage and host hook have committed their state.
- Copy selected source data into renderer-owned capture values before `LayerArena` storage is released.
- Execute only the fixed scheduler dependency closure needed to reach the furthest selected tap, including correctness-required expansion recorded with a reason.
- Own the minimal `crates/pnp-cli` command-to-runtime dispatch seam that passes packet 157's parsed/validated model-backed request to capture execution.
- Filter retained captures by the request's selected layers and release unselected per-layer data promptly.
- Preserve deterministic tap, layer, and payload ordering and existing progress-event failure semantics.
- Add focused contract, lifecycle, closure, determinism, and negative tests.

## Out of Scope

- PNG encoding, rasterization, shared viewport, palette, legend, or any rendering implementation.
- Final G-code parsing, G-code rendering, unsupported-command handling, or G-code renderer tests.
- Changes to WIT contracts, module manifests, IR schemas, module-visible Blackboard access, or scheduler stage order.
- New WASM modules, guest artifacts, WASM build work, or guest-facing capture APIs.
- Coordinate conversion, geometry projection, or millimeter/unit policy.
- OrcaSlicer comparison or source translation.
- Agent skills, guide documentation, HTML galleries, or ordinary-slice instrumentation.

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - direct complete read; stage inventory and typed capture rules are the controlling requirements.
- `docs/19_visual_debug.md` - direct complete read; model-mode closure and failure behavior are user-facing constraints.
- `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - direct complete read; runtime ownership and lifetime decision.
- `docs/01_system_architecture.md` - direct ranges 65-109, 246-500, 567-665; stage order, fixed execution, IR ownership, and arena lifetime.
- `docs/09_progress_events.md` - direct complete read; event ordering and fatal/non-fatal visibility remain unchanged.
- `docs/11_operational_governance_and_acceptance_gate.md` - direct complete read; determinism, recoverability, coupling, and compatibility gates.
- `docs/07_implementation_status.md` - delegated summary of TASK-268 at lines 239-240.
- `docs/specs/visual-pipeline-debug-plan.md` - direct complete read; packet dependency queue.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4`; typed capture is request-gated, post-stage, closure-bounded, layer-bounded, and deterministic.
- Negative: `AC-N1` through `AC-N4`; ordinary slices, unsupported taps, unavailable source state, and empty applicable layer selections cannot silently succeed.
- Cross-packet impact: packet 157 owns request parsing, source-mode validation, bundle lifecycle, overwrite policy, and manifest base model. Packet 158 consumes that exported model and adds typed capture data; packet 159 consumes the resulting renderer-owned captures. Packet 158 must not duplicate or reinterpret packet 157's wire contract.
- CLI ownership: packet 157 owns command parsing and request validation; packet 158 owns only the minimal `crates/pnp-cli` dispatch wiring from that validated request to runtime capture execution.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --all-targets --test visual_debug_typed_tap_capture_tdd` | Run the focused typed tap capture contract and negative tests. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | Compile runtime, CLI, and all test targets after the capture seam is wired. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Enforce workspace lint and architecture-quality gate. | FACT pass/fail |

## Step Completion Expectations

- Capture must be post-stage and post-host-hook, never module-visible and never a pre-commit snapshot.
- Any correctness-driven execution expansion is allowed only when represented in the packet-157 manifest model with its reason; expansion must not become retained capture data unless requested.
- No successful result may expose a borrow into `LayerArena` or report a partial typed capture set.
- Ordinary slice execution remains capture-free unless an explicit visual-debug request is active.

## Context Discipline Notes

- `docs/01_system_architecture.md` is large; only the ranges specified above may be read directly.
- Packet 157 is implemented (commit `3e33ca01`); its request/manifest model is grounded at `crates/pnp-cli/src/visual_debug.rs:14-370` and its test convention at `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`. `slicer-runtime` cannot import these types (dependency direction is `pnp-cli -> slicer-runtime`), so implementers must delegate a bounded symbol lookup for the new `slicer-runtime` capture entry point, not for packet 157's own types.
- Runtime symbol tracing and cargo verification must be delegated with bounded FACT, LOCATIONS, or SNIPPETS returns.
