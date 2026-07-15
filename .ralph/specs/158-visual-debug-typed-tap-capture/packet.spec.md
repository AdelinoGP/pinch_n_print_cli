---
status: draft
packet: 158-visual-debug-typed-tap-capture
task_ids:
  - TASK-268
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Packet 157 must export the request and manifest model before this packet is implemented.
---

# Packet Contract: 158-visual-debug-typed-tap-capture

## Goal

Add request-gated, typed post-stage capture at the executor boundary so model-backed visual-debug requests execute only the scheduler dependency closure required by their selected taps and export bounded renderer-owned captures through packet 157's request/manifest model.

## Scope Boundaries

This packet adds the runtime tap registry/adapter layer, post-stage capture timing, selected-layer filtering, dependency-closure execution, and the minimal `crates/pnp-cli` command-to-runtime dispatch seam for model-backed visual-debug requests. Captures are typed, immutable-source reads copied into renderer-owned data and represented through packet 157's exported request and bundle-manifest model. Packet 157 remains the sole owner of request parsing/validation, source mode, bundle lifecycle, overwrite behavior, and base manifest semantics. It does not render images, parse G-code, change module/WIT/IR contracts, add a module, or modify ordinary `pnp_cli slice` behavior.

## Prerequisites and Blockers

- Depends on: packet `157-visual-debug-request-bundle-contract` and its exported request/manifest model; ADR-0037.
- Unblocks: packet `159-visual-debug-intermediate-renderer`.
- Activation blockers: packet 157's exported Rust symbols and capture/manifest extension seam must be confirmed before implementation.

## Acceptance Criteria

- **AC-1. Given** a model-backed packet-157 request selecting one documented typed tap and one selected layer, **when** `pnp_cli visual-debug --request request.json --output bundle-dir` executes, **then** the packet-157 manifest contains exactly one typed capture entry for that requested tap and layer, with the requested tap identity preserved and a non-empty typed capture payload available to the downstream renderer, without a PNG being required or produced by this packet. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- typed_tap_capture_records_selected_layer --exact`
- **AC-2. Given** a model-backed request selecting taps at two scheduler stages, **when** the visual-debug execution runs, **then** every prerequisite stage in fixed scheduler order runs through the furthest selected tap, each selected capture is taken after that stage's host hook/commit boundary, and no stage after the furthest selected tap is executed. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- dependency_closure_stops_at_furthest_tap --exact`
- **AC-3. Given** a model-backed request selecting a subset of layers for a typed tap, **when** the selected closure executes, **then** the manifest records only the requested layer captures while any additional layers required for correctness are recorded as executed-but-unrendered expansion with a non-empty reason, and no unselected layer capture is retained. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- selected_layers_bound_capture_retention --exact`
- **AC-4. Given** the same model-backed request and deterministic inputs executed twice, **when** both runs complete, **then** the typed capture entries have identical tap identities, layer indices, source schema versions, and serialized payload ordering. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- typed_capture_is_deterministic --exact`
- **AC-N1. Given** an ordinary `pnp_cli slice` invocation with no visual-debug request, **when** the slice executes, **then** no visual-debug tap is registered, no capture allocation or serialization occurs, and no visual-debug manifest entry is emitted. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- ordinary_slice_has_no_tap_capture --exact`
- **AC-N2. Given** a visual-debug request containing an unknown or unsupported typed tap, **when** request execution begins, **then** it fails with a typed validation error naming the unsupported tap and produces no successful partial capture bundle or manifest success result. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- unknown_tap_is_rejected_without_success --exact`
- **AC-N3. Given** a selected tap whose source IR is unavailable at its documented post-stage boundary, **when** capture is attempted, **then** execution fails rather than retaining a dangling borrow, fabricating geometry, or reporting a successful partial capture. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- unavailable_tap_source_fails_without_partial_success --exact`

## Negative Test Cases

- **AC-N4. Given** a request that selects a typed tap but does not select any layer applicable to that tap, **when** validation runs, **then** it rejects the request with an actionable validation error and does not execute the pipeline closure. | `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd -- tap_without_applicable_layer_is_rejected --exact`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --all-targets --test visual_debug_typed_tap_capture_tdd`

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - direct read of the complete 235-line contract, especially lines 99-110 and 143-163.
- `docs/19_visual_debug.md` - direct read of the complete 58-line usage and bundle behavior guide.
- `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - direct read of the complete 44-line decision record.
- `docs/01_system_architecture.md` - direct reads of lines 65-109, 246-500, and 567-665 for stage order, ownership, and lifetimes.
- `docs/09_progress_events.md` - direct read of the complete 196-line event contract; capture must not weaken event ordering or failure visibility.
- `docs/11_operational_governance_and_acceptance_gate.md` - direct read of the complete 179-line governance and acceptance contract.
- `docs/07_implementation_status.md` - delegated task-location fact: TASK-268 is the typed tap capture row at lines 239-240.
- `docs/specs/visual-pipeline-debug-plan.md` - direct read of the complete 15-line dependency queue; packet 157 precedes packet 158.

## Doc Impact Statement

- **`docs/specs/visual-pipeline-debug.md`** - update the canonical visual-debug contract to describe the typed post-stage capture records, renderer-owned capture payloads, selected-layer retention, executed-but-unrendered dependency expansion with reasons, and the typed capture/manifest fields produced by this packet. Preserve packet 157's ownership of request parsing/validation, source mode, bundle lifecycle, overwrite behavior, and base manifest semantics.
  Verification: `rg -n -m 1 '^## Bundle Contract$' docs/specs/visual-pipeline-debug.md && rg -n -m 1 '^### Dependency Closure$' docs/specs/visual-pipeline-debug.md && rg -n -m 1 '^### Intermediate IR Path$' docs/specs/visual-pipeline-debug.md && rg -n -m 1 'typed post-stage|renderer-owned|selected-layer|executed but not rendered|capture.*manifest' docs/specs/visual-pipeline-debug.md`
- **`docs/19_visual_debug.md`** - update the bundle-reading and model-mode guidance to explain that model-backed requests may contain typed captures before rendering, how typed tap/layer entries and execution expansion appear in `manifest.json`, and that this packet does not require or produce PNGs. Preserve packet 157's request and bundle lifecycle guidance.
  Verification: `rg -n -m 1 '^## Request Shape$' docs/19_visual_debug.md && rg -n -m 1 '^## Reading A Bundle$' docs/19_visual_debug.md && rg -n -m 1 'model-backed|typed capture|tap.*layer|execution expansion|manifest\.json|does not require or produce PNG' docs/19_visual_debug.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
