# Requirements: orca-gcode-emission-contract

## Packet Metadata

- Grouped task IDs:
  - `TASK-119` — restore Orca-identical live GCode comment and emission contract
  - `TASK-119a` — enumerate the exact Orca-native comment and token contract in one canonical spec surface
  - `TASK-119b` — emit the canonical sequence with matching spelling, ordering, and placement
  - `TASK-119c` — add compatibility regressions proving the emitted `.gcode` preserves the contract end-to-end
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The live host emit path currently converts `LayerCollectionIR` into mostly raw `G1` moves plus pass-through comments. That is not enough for OrcaSlicer-compatible preview and visualization semantics. The missing gap is larger than comment headers alone: the host must own one exact emit contract for layer-change comments, `;TYPE:` role boundaries, seam-started wall-loop preservation, and travel/retraction serialization when fill, support, seam, or travel decisions reach the final postpass path.

This packet owns the emitted-text contract only. It does not restore the upstream feature producers. Those producer packets remain separate so the repo can validate emit behavior against synthetic fixtures now, then layer the real fill/support/seam/travel generation work on top of the same contract later.

## In Scope

- one host-owned constant or helper surface for Orca label spelling, role grouping, and layer-header ordering
- `;LAYER_CHANGE`, `;Z:`, and `;HEIGHT:` emission rules
- `;TYPE:` emission rules for walls, fill, support, brim/skirt, and wipe/prime geometry
- seam-started wall-loop preservation on the emit path
- retract/unretract/travel/Z-hop serialization rules
- whole-postpass compatibility regressions from `LayerCollectionIR` to final text

## Out of Scope

- generating top/bottom fill geometry
- generating support geometry
- producing or choosing resolved seams
- deciding retract/no-retract policy ownership
- generating SkirtBrim or WipeTower entities
- Benchy feature-evidence assertions beyond the emit contract itself

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — canonical layer/export ordering and travel/retract sequencing
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeWriter.cpp` — low-level text emission shape
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.hpp` — viewer-facing comment/token expectations
- `OrcaSlicerDocumented/src/libslic3r/GCode/PostProcessor.hpp` — postpass preservation requirements
- `OrcaSlicerDocumented/tests/fff_print/test_gcode.cpp` — upstream regression idioms for GCode text

## Acceptance Summary

### Positive Cases

- The host emits `;LAYER_CHANGE`, `;Z:<value>`, and `;HEIGHT:<value>` in one deterministic order before the first move of each layer block.
- The host maps `ExtrusionRole` values to exact Orca `;TYPE:` labels and inserts them once per contiguous role block.
- Seam-started wall loops remain seam-started in the final text instead of being silently re-anchored by the emitter.
- Retract, unretract, travel, and Z-hop commands serialize in a canonical order that matches the final text contract.
- Whole-postpass fixture runs are byte-deterministic across repeated executions.

### Negative Cases

- If a role family is absent from a layer, the emitter does not fabricate its `;TYPE:` label.
- If no retract/unretract is queued for a layer, the output contains no retract/unretract lines.

### Measurable Outcomes

- `gcode_emit_tdd` asserts exact line fragments and ordering, not just comment presence.
- `postpass_gcode_emit_contract_tdd` proves a full `execute_postpass()` round-trip preserves the contract byte-for-byte.
- The canonical label map lives in one source surface and every acceptance test reads that same emitted text path.

### Cross-Packet Impact

- Packets `12` through `20` can restore feature producers independently once this output contract exists.
- Packet `21` uses this packet's emitted labels and headers as its end-to-end Benchy evidence surface.

## Verification Commands

- `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_layer_headers_before_first_extrusion -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_type_comments_at_role_boundaries -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd preserves_seam_started_wall_loop_order_in_output -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd serializes_retract_travel_and_z_hop_in_canonical_order -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd omits_absent_role_labels_and_retraction_lines -- --exact --nocapture`
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd full_postpass_pipeline_preserves_orca_emission_contract -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the target emit or postpass surface is identified and failing tests are in place
- Postcondition: one part of the contract is emitted from the real host path, not from ad hoc test-only formatting helpers
- Falsifying check: the narrowest text assertion fails if ordering, spelling, or omission behavior regresses

## Cross-Packet Notes

- This packet intentionally owns fill, support, seam, and travel emission semantics even before those producers are fully restored.
- Producer packets must not add Orca-specific formatting logic outside the canonical emit surface established here.