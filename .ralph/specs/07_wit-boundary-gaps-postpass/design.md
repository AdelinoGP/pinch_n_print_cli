# Design: wit-boundary-gaps-postpass

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-host/src/dispatch.rs` — `WasmRuntimeDispatcher::dispatch_postpass_gcode_call` (line 626)
  - `crates/slicer-host/src/dispatch.rs` — `WasmRuntimeDispatcher::dispatch_layer_call` (layer-world deep-copy via `HostExecutionContext`)
  - `crates/slicer-host/src/dispatch.rs` — `WasmRuntimeDispatcher::dispatch_finalization_call` (finalization-world deep-copy)
  - `crates/slicer-host/src/postpass.rs` — `execute_postpass` (line 163)
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/postpass_executor_tdd.rs` — existing postpass executor tests
  - `crates/slicer-host/tests/macro_postpass_text_roundtrip_tdd.rs` — existing postpass text roundtrip
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` — existing world roundtrip tests
- OrcaSlicer comparison surface: None — this packet addresses WIT boundary enforcement internal to the scheduler, not OrcaSlicer behavior

## Architecture Constraints

- WIT boundary rules from `docs/03_wit_and_manifest.md`: modules never receive more data than declared; access control enforced per-call
- GCodeIR is owned by the host and passed into postpass modules as a mutable reference; the `gcode-output-builder` is the only write surface modules may use
- The postpass dispatch currently passes `&[]` (empty slice) for the command list instead of the actual `gcode_ir.commands`, and the postpass WIT surface omits both full command payload input and explicit `Unretract` output support
- Layer-world modules write via builders (`perimeter-output-builder`, `infill-output-builder`, etc.); there is no `LayerCollectionIR` read resource in `world-layer.wit`, so TASK-129b must validate the existing builder-to-arena-to-commit path rather than inventing a new read surface
- Finalization-world modules read `Vec<LayerCollectionIR>`, but the current `layer-collection-view` only exposes metadata (`layer-index`, `z`, `entity-count`, `tool-changes`); `ordered_entities` and `z_hops` require WIT widening before live-path deep-copy can be fully verified
- DEV-014 remains in force: canonical WIT files, host inline WIT, macro inline WIT, and hand-written guest WIT must be updated together and guarded by drift tests in the same implementation slice

## Code Change Surface

- Selected approach:
  - TASK-129a: Widen the postpass WIT surface so guest input carries full `GCodeCommand` payloads and `gcode-output-builder` carries explicit `Unretract` writes. The selected representation is a payload-bearing WIT `variant gcode-command` mirrored into host bindings, macro glue, test guests, and the SDK trait surface. `dispatch_postpass_gcode_call` then accepts a `&[GCodeCommand]` parameter, converts those commands into the widened WIT input, and passes them through the boundary instead of `&[]`.
  - TASK-129b: Add `layer_world_deep_copy_tdd.rs` proving the existing live layer-world builder-to-arena-to-commit path preserves `ordered_entities`, `tool_changes`, and `z_hops`. Do not add a new layer-world read resource.
  - TASK-129c: Widen the finalization WIT surface so `layer-collection-view` exposes `ordered_entities()` and `z_hops()`, mirror that shape in host bindings and macro glue, and add `finalization_world_deep_copy_tdd.rs` proving full completed-layer input survives through the finalization-world boundary.
- Exact functions, manifests, tests, or fixtures expected to change:
  - `wit/deps/ir-types.wit` — add `push-unretract` to `gcode-output-builder`
  - `wit/world-postpass.wit` — replace thin `gcode-command-view` input with payload-bearing postpass command input for all eight variants
  - `wit/world-finalization.wit` — add `print-entity-view`, `z-hop-view`, and `layer-collection-view` methods for `ordered-entities()` and `z-hops()`
  - `crates/slicer-host/src/dispatch.rs` — `dispatch_postpass_gcode_call`: add `commands: &[GCodeCommand]` parameter, convert commands into widened WIT input, and pass them to `bindings.call_run_gcode_postprocess`
  - `crates/slicer-host/src/dispatch.rs` — `WasmRuntimeDispatcher::run_gcode_postprocess`: pass `&gcode_ir.commands` to `dispatch_postpass_gcode_call`
  - `crates/slicer-host/src/wit_host.rs` — mirror widened postpass/finalization WIT, add `Unretract` host collection support, widen `LayerCollectionViewData`, and implement finalization read methods for `ordered_entities` and `z_hops`
  - `crates/slicer-sdk/src/traits.rs` — change the postpass trait input surface to full `GCodeCommand` values and expose `ordered_entities()` / `z_hops()` on finalization `LayerCollectionView`
  - `crates/slicer-sdk/src/postpass_builders.rs` — add `push_unretract`
  - `crates/slicer-macros/src/lib.rs` — mirror widened postpass/finalization WIT and stop constructing empty or placeholder deep-copy inputs
  - `test-guests/postpass-guest/src/lib.rs` — mirror widened postpass WIT for raw guest coverage
  - `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs` — new file, all 8 GCodeCommand variant round-trip tests
  - `crates/slicer-host/tests/postpass_gcode_command_preservation_tdd.rs` — new file, order/content preservation tests
  - `crates/slicer-host/tests/postpass_gcode_empty_list_tdd.rs` — new file, negative case proving empty list is valid
  - `crates/slicer-host/tests/layer_world_deep_copy_tdd.rs` — new file, LayerCollectionIR deep-copy tests
  - `crates/slicer-host/tests/finalization_world_deep_copy_tdd.rs` — new file, Vec<LayerCollectionIR> deep-copy tests
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — add drift assertions for `push-unretract`, widened postpass command input, and widened finalization read methods
- Rejected alternatives that were not chosen:
  - Adding boundary coverage via integration tests only (rejected — TDD requires isolated regression tests)
  - Using mock runner instead of real WASM path (rejected — the acceptance criteria require live-path coverage)
  - Adding a new layer-world read resource exposing `LayerCollectionIR` (rejected — architecturally incoherent with the write-oriented layer world and unnecessary for TASK-129b)

## Data and Contract Notes

- IR or manifest contracts touched:
  - `GCodeIR.commands: Vec<GCodeCommand>` — all 8 variants must cross the widened postpass boundary with exact payloads
  - `gcode-output-builder` — must expose `push-unretract` so guest output can express the full IR command set
  - `LayerCollectionIR` — layer-world coverage targets the committed host IR after builder output drains through the production commit path
  - `Vec<LayerCollectionIR>` — finalization WIT widening must expose `ordered_entities` and `z_hops` in addition to metadata-level fields
- WIT boundary considerations:
  - Postpass boundary: `gcode-output-builder` resource is pushed into the store before the call; the builder collects pushes from the guest and commits them back to the host GCodeIR after the call. The widened command input must be mirrored in canonical WIT, host bindgen, macro glue, and any hand-written guest WIT copies.
  - Layer-world boundary: validate the existing builder outputs and host commit path rather than inventing a new read surface.
  - Finalization-world boundary: `world-finalization.wit` exposes `layer-collection-view`; widening that resource must preserve all completed-layer data and remain mirrored across canonical and inline WIT definitions.
- Determinism or scheduler constraints: The boundary operations must be deterministic; order/content preservation is required

## Locked Assumptions and Invariants

- The `gcode-output-builder` is the only channel through which a postpass module may emit GCode commands; direct mutation of `GCodeIR.commands` by the guest is not permitted
- Empty command lists are valid — no contract violation should be raised for an empty list
- The postpass implementation slice must update canonical WIT, host inline WIT, macro inline WIT, and hand-written guest WIT together; partial updates are not acceptable
- TASK-129b must not add a new layer-world read resource; the live commit path is the authoritative surface under test
- All entity fields in committed `LayerCollectionIR` are preserved bit-for-bit through the layer-world boundary — no normalization, rounding, or truncation
- All completed-layer fields exposed through widened finalization WIT are preserved bit-for-bit through the finalization-world boundary

## Risks and Tradeoffs

- Risk: Widening duplicated WIT surfaces can reintroduce DEV-014 drift if canonical, host, macro, and guest copies do not change together
- Mitigation: Extend `wit_drift_detection_tdd` in the same slice and run it before boundary-runtime tests
- Risk: Changing `dispatch_postpass_gcode_call` to pass a non-empty payload-bearing command list may expose latent guest bugs or bindgen mismatches
- Mitigation: Start with focused postpass boundary and SDK tests before running broader host tests
- Risk: Widened finalization inputs increase bindgen and macro deep-copy complexity
- Mitigation: Reuse the existing witness-based finalization pattern with minimal 2-3 layer fixtures that isolate field-preservation assertions

## Open Questions

- None. Scope-changing decisions are resolved: postpass and finalization widen their WIT surfaces; layer-world remains on the existing live builder-to-commit path; packet status stays `draft` during in-flight implementation because this run began from a draft packet.
