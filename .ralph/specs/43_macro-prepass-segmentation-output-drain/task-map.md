# Task Map: macro-prepass-segmentation-output-drain

This file bridges the packet's implementation steps back to `docs/07_implementation_status.md`. The packet covers three task IDs (`TASK-130`, `TASK-130a`, `TASK-130b`) and closes DEV-025 mismatch 3 — a cross-cutting closure that warrants the explicit map.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-130` (umbrella) | Step 0 | `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` (FACT-confirm `status: implemented`) | none | none | S | Activation gate. If Packet 42 not implemented, halt. Sufficient evidence: Step-0 Notes addendum with binary answers to six FACT questions. |
| `TASK-130b` | Step 1 | `docs/02_ir_schemas.md` (PaintRegionIR + MeshSegmentationIR sections) | `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (NEW), `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (NEW) | none | M | RED-state authoring of all named AC tests. Sufficient evidence: tests compile or fail with predictable errors recorded. |
| `TASK-130a` | Step 2 | `docs/05_module_sdk.md` (delegate SUMMARY for prepass arm lifecycle) | `crates/slicer-macros/src/lib.rs` (PaintSegmentation arm body + legacy comment removal) | none | S/M | The drain insertion. Sufficient evidence: AC-1 (`macro_arm_drains_regions_to_wit`) and AC-6 (`legacy_comment_block_removed`) GREEN; build green. |
| `TASK-130b` | Step 3 | `docs/05_module_sdk.md` (delegate SUMMARY for `#[slicer_module]` emit pattern) | `test-guests/sdk-prepass-guest/src/lib.rs` (or equivalent — Step 0 confirms path), `test-guests/sdk-prepass-guest.component.wasm` (rebuilt artifact) | none | M | Guest fixture extension. Sufficient evidence: guest builds; .wasm size delta confirmed. |
| `TASK-130b` | Step 4 | `docs/02_ir_schemas.md` (PaintRegionIR section, narrow) | `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (refinements) | `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:211-225` (delegate; ExPolygon parity for hole-bearing fixture comments) | S/M | PaintSegmentation round-trip GREEN. Sufficient evidence: AC-2, AC-3, AC-5, AC-6 + `empty_polygons_rejected_at_host_validator`, `no_early_return_bypasses_drain` GREEN. AC-2 is the substantive proof. |
| `TASK-130b` | Step 5 | `docs/02_ir_schemas.md` (MeshSegmentationIR section, narrow) | `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (refinements) | none | S | MeshSegmentation round-trip GREEN. Sufficient evidence: AC-4 GREEN. |
| `TASK-130`, `TASK-130a`, `TASK-130b` | Step 6 | none | none | none | S | Regression sweep over the four named macro_*_tdd files + any Step-0-enumerated tests that load `sdk-prepass-guest.component.wasm`. Sufficient evidence: every regression target FACT-pass. |
| `TASK-130`, `TASK-130a`, `TASK-130b` | Step 7 | `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | doc edits only | none | S | Backlog + DEV-025 closure. Sufficient evidence: AC-7, AC-8, AC-9 GREEN. |
| `TASK-130`, `TASK-130a`, `TASK-130b` | Step 8 | `packet.spec.md` (re-read for AC list) | `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` (status flip) | none | S | Acceptance ceremony — full AC sweep + clippy + workspace test. |

## Aggregate

- Aggregate context cost: **M** (sum of step costs; no single step at L).
- TASK-130 (umbrella) closes when TASK-130a and TASK-130b close — Steps 2, 4, 5 produce the substantive evidence.
- TASK-130c is **not** addressed by this packet — it is Packet 42's territory.
- DEV-025 closure is **final** at the end of this packet (mismatches 1+2 from Packet 06, mismatches 4+5 from Packet 42, mismatch 3 from this packet → all five closed).

## Why this packet exists as a separate slice

The original DEV-025 audit registered three mismatches (1, 2, 3). Packet 06 closed 1 + 2 and laid the macro-arm scaffolding, then explicitly DEFERRED mismatch 3 because closing it required SDK and WIT shape changes that were out of scope at the time. While planning the deferred close, an architectural review surfaced two additional mismatches (4: paint value channel string-coerced; 5: SDK paint-region polygons hole-blind) that the original audit did not catch. Closing mismatch 3 against the un-corrected transport would have entrenched silent geometric and value corruption.

Therefore the deferred closure was split into two packets:
- **Packet 42** (`paint-region-transport-widening`) — closes mismatches 4 + 5 by widening the SDK and WIT shapes so paint regions carry hole-bearing polygons and typed values end-to-end.
- **Packet 43** (this packet) — closes mismatch 3 by draining the macro arm against the corrected transport, plus adds the end-to-end macro-path round-trip tests TASK-130b demands.

Plan B (single combined packet) was rejected at the design stage on aggregate-cost grounds (would have hit L) and on review-clarity grounds (mixing transport refactor with macro-arm drain would have made the AC commentary harder to audit). The split keeps each packet at aggregate `M` and isolates failure modes.
