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
- The postpass dispatch currently passes `&[]` (empty slice) for the command list (dispatch.rs line 707) instead of the actual `gcode_ir.commands`
- Layer-world modules write via builders (`perimeter-output-builder`, `infill-output-builder`, etc.); the deep-copy read path is not exercised on live WASM execution paths
- Finalization-world modules read `Vec<LayerCollectionIR>`; the deep-copy path is similarly not exercised on live WASM execution paths

## Code Change Surface

- Selected approach:
  - TASK-129a: Fix `dispatch_postpass_gcode_call` to pass `gcode_ir.commands.as_slice()` instead of `&[]`; add `postpass_gcode_boundary_tdd.rs` and `postpass_gcode_command_preservation_tdd.rs` regression tests
  - TASK-129b: Add `layer_world_deep_copy_tdd.rs` proving `LayerCollectionIR` fields survive through the layer-world WIT boundary from arena view through WASM call and back
  - TASK-129c: Add `finalization_world_deep_copy_tdd.rs` proving `Vec<LayerCollectionIR>` fields survive through the finalization-world WIT boundary
- Exact functions, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/dispatch.rs` — `dispatch_postpass_gcode_call` (line 707): change `&[]` to `gcode_ir.commands.as_slice()`
  - `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs` — new file, all 8 GCodeCommand variant round-trip tests
  - `crates/slicer-host/tests/postpass_gcode_command_preservation_tdd.rs` — new file, order/content preservation tests
  - `crates/slicer-host/tests/layer_world_deep_copy_tdd.rs` — new file, LayerCollectionIR deep-copy tests
  - `crates/slicer-host/tests/finalization_world_deep_copy_tdd.rs` — new file, Vec<LayerCollectionIR> deep-copy tests
- Rejected alternatives that were not chosen:
  - Adding boundary coverage via integration tests only (rejected — TDD requires isolated regression tests)
  - Using mock runner instead of real WASM path (rejected — the acceptance criteria require live-path coverage)

## Data and Contract Notes

- IR or manifest contracts touched:
  - `GCodeIR.commands: Vec<GCodeCommand>` — all 8 variants must round-trip
  - `LayerCollectionIR` — all fields must survive deep-copy through layer-world WIT boundary
  - `Vec<LayerCollectionIR>` — all layers, z values, entities, tool_changes, z_hops must survive finalization-world WIT boundary
- WIT boundary considerations:
  - Postpass boundary: `gcode-output-builder` resource is pushed into the store before the call; the builder collects pushes from the guest and commits them back to the host GCodeIR after the call
  - Layer-world boundary: `LayerArena` provides the layer collection view; deep-copy must preserve all entity fields
  - Finalization-world boundary: `world-finalization.wit` exposes `layer-collection-view`; deep-copy must preserve all layers and entities
- Determinism or scheduler constraints: The boundary operations must be deterministic; order/content preservation is required

## Locked Assumptions and Invariants

- The `gcode-output-builder` is the only channel through which a postpass module may emit GCode commands; direct mutation of `GCodeIR.commands` by the guest is not permitted
- Empty command lists are valid — no contract violation should be raised for an empty list
- All entity fields in `LayerCollectionIR` are preserved bit-for-bit through the layer-world boundary — no normalization, rounding, or truncation
- All layer indices and z values in `Vec<LayerCollectionIR>` are preserved bit-for-bit through the finalization-world boundary

## Risks and Tradeoffs

- Risk: Changing `dispatch_postpass_gcode_call` to pass a non-empty slice may expose latent bugs in guests that do not handle the full command list correctly
- Mitigation: The TDD tests use a minimal guest (macro-authored or raw WIT) that echoes back the command list for verification
- Risk: Deep-copy tests may be slow if they construct full `Vec<LayerCollectionIR>` with many layers
- Mitigation: Use minimal fixture data (2-3 layers, 2-3 entities per layer) sufficient to prove field preservation

## Open Questions

- None at this time. The scope is well-defined by DEV-006 and the acceptance criteria are exact. This packet should remain `draft` only until the directory is created and files are written.
