# Task Map: external-surface-classification-at-slice

This packet narrows packet `12_live-top-bottom-surface-fill` (status `implemented`) by closing the host-side wiring gap. It does NOT supersede packet 12 — packet 12's narrow Acceptance Criteria (module-level role generation + commit-side preservation) remain valid and continue to pass. This packet adds the missing producer side (host populates `SlicedRegion` flags) so the consumer side can finally light up on the live Benchy path.

## Backlog Mapping

| Backlog Task ID | Status before packet | Status after packet | Notes |
| --- | --- | --- | --- |
| TASK-120a | implemented (closed by packet 12) | unchanged | Module / commit / role-preservation still valid; packet 12 was correctly scoped to its own ACs. |
| TASK-164 (new) | n/a | open → implemented at packet completion | Wiring of `PrePass::MeshAnalysis` output into `SlicedRegion` at slice time. |

## Step → Task Mapping

All steps in `implementation-plan.md` map to TASK-164.

| Step | Task IDs | Output |
| --- | --- | --- |
| Step 0 | TASK-164 | FACT — `bridge_regions` populated state |
| Step 1 | TASK-164 | Schema bump + mechanical fix-ups |
| Step 2 | TASK-164 | TDD file authored (failing) |
| Step 3 | TASK-164 | `classify_region_surfaces` helper implemented |
| Step 4 | TASK-164 | `execute_layer_slice` extended; production caller wired; WIT-data conversion populated |
| Step 5 | TASK-164 | `layer_slice_tdd.rs` test callers updated |
| Step 6 | TASK-164 | Acceptance + doc updates |

## Cross-Packet Relationships

- **Predecessor**: packet `12_live-top-bottom-surface-fill` (implemented). This packet adds a header pointer in that packet's `packet.spec.md` to make the dependency explicit; status of packet 12 is NOT flipped.
- **Successors unblocked by this packet**:
  - `35_multi-layer-top-bottom-thickness` — extends the Z-window in `classify_region_surfaces`.
  - `36_bridge-detector-orca-parity` — replaces the boolean `is_bridge` with validated bridge polygons in `SlicedRegion.bridge_areas`.
  - `38_top-surface-ironing` — needs precise topmost-layer detection from this packet's classifier (and from packet 35 once `top_solid_layers` is honored).

## Authoritative Docs Mapping

| Step | Docs to Consult |
| --- | --- |
| Step 0 | `docs/02_ir_schemas.md` § BridgeRegion |
| Step 1 | `docs/02_ir_schemas.md` § SliceIR additive-minor rule |
| Step 2 | `docs/02_ir_schemas.md` § SurfaceClassificationIR; `docs/08_coordinate_system.md` |
| Step 3 | `docs/08_coordinate_system.md` |
| Step 4 | `docs/04_host_scheduler.md` § Per-Layer Execution + Blackboard Structure |
| Step 5 | none |
| Step 6 | `docs/02_ir_schemas.md`; `docs/DEVIATION_LOG.md`; `docs/07_implementation_status.md` |

## OrcaSlicer Reference Mapping

| Step | OrcaSlicer References |
| --- | --- |
| Step 2 | `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` (FACT delegation only — confirm role taxonomy) |
| Step 3 | `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` (FACT delegation only — confirm divergence from inter-layer subtraction; record in `docs/DEVIATION_LOG.md`) |

All OrcaSlicer reads are delegated; never load this tree into the implementer's own context.
