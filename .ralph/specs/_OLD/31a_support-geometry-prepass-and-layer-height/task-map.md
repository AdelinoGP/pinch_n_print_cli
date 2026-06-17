# Task Map: 31a_support-geometry-prepass-and-layer-height → docs/07

This packet establishes the architectural foundation for variable-height support planning in Pinch 'n Print. It does NOT close the algorithmic v1 limitations (avoidance/collision, radius taper, raft, wall-count, config keys) — those ship in packet `31b_support-planner-algorithmic-parity`.

## Task ID Mapping

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-163` (arch) | Step 1 | `docs/02`, `docs/01` | none | none | Read LayerPlanIR shape; confirm support boundary computation approach. |
| `TASK-163` (arch) | Step 2 | `docs/02` | `.ralph/specs/31a_support-geometry-prepass-and-layer-height/design.md` | none | Resolve Q1 (support layer boundary formula). |
| `TASK-163` (arch) | Step 3 | `docs/02` | `.ralph/specs/31a_support-geometry-prepass-and-layer-height/design.md` | none | Resolve Q2 (support_top_z_distance refinement). |
| `TASK-163` (arch) | Step 4 | `docs/02` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/lib.rs` | none | SupportGeometryIR type. |
| `TASK-163` (arch) | Step 5 | `docs/04` | `crates/slicer-host/src/blackboard.rs` | none | Blackboard slot + commit/accessor. |
| `TASK-163` (arch) | Step 6 | `docs/01`, `docs/04` | `crates/slicer-host/src/prepass.rs` | none | PrePass::SupportGeometry built-in computation. |
| `TASK-163` (arch) | Step 7 | `docs/03` | `wit/world-prepass.wit` | none | New WIT records + extended export signature. |
| `TASK-163` (arch) | Step 8 | `docs/03`, `docs/05` | `crates/slicer-sdk/src/preprep_types.rs`, `crates/slicer-sdk/src/prelude.rs`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-host/src/wit_host.rs` | none | SDK + macro + projector. |
| `TASK-163` (arch) | Step 9 | `docs/04` | `crates/slicer-host/src/prepass.rs` | none | required_slots extension. |
| `TASK-163` (arch) | Step 10 | `docs/04` | `crates/slicer-host/tests/support_geometry_prepass_tdd.rs` (new) | none | TDD tests for SupportGeometryIR production. |
| `TASK-163` (arch) | Step 11 | `docs/03` | `modules/core-modules/support-planner/support-planner.toml` | none | Manifest reads + config schema. |
| `TASK-163` (arch) | Step 12 | `docs/03` | `modules/core-modules/tree-support/tree-support.toml` | none | Tree-support config schema additions. |
| `TASK-163` (arch) | Step 13 | `docs/02`, `docs/05` | `modules/core-modules/support-planner/src/lib.rs` | none | Support interpolation from coarse to model resolution. |
| `TASK-163` (arch) | Step 14 | `docs/03` | none (config schema bounds) | none | Config validation negatives. |
| `TASK-163` (arch) | Step 15 | `docs/03` | every `modules/core-modules/*/wit-guest/`, every `*.wasm` | none | Cascade rebuild. |
| `TASK-163` (arch) | Step 16 | `docs/07` | `docs/07_implementation_status.md` | none | Backlog row. |
| `TASK-163` (arch) | Step 17 | `docs/11`, `docs/12` | none | none | Packet completion gate. |

## Superseding Relationship

Packet 31a does **not** supersede any prior packet.

- Packet `28_tree-support-multi-layer-propagation` (status: `implemented`). Unchanged.
- Packet `30_support-planner-prepass-wit-plumbing` (status: `implemented` before 31a activates). Unchanged — 31a extends it additively.
- Packet `31b_support-planner-algorithmic-parity` (future). Consumes `SupportGeometryView` and the new config keys to implement avoidance/collision, radius tapering, wall-count scaling, raft prefix, and interface densification.

## What Changed vs Original Packet 31 Draft

The original packet 31 (now discarded) introduced `PrePass::SlicePreview` as a new sequential prepass stage and attempted to read `SliceIR` (Tier 2 data) during prepass. This violated the architecture and was rejected. The revised packet 31a establishes a clean architectural foundation:

1. `PrePass::SupportGeometry` is a lightweight host-built-in prepass that computes coarse support outlines at support layer resolution (not model resolution) using `LayerPlanIR` to determine boundaries before any slicing.
2. `SupportGeometryIR` is a new dedicated IR type (not `SliceIR` reuse) holding coarse support geometry at support layer resolution.
3. `support_layer_height_mm` enables a genuine Pinch 'n Print differentiator: support planning at a different (coarser) resolution than the model.
4. `support_top_z_distance_mm` refines the interface near model contact zones.

This makes Pinch 'n Print strictly better than OrcaSlicer for high-resolution prints: supports can use 3× fewer layers than the model, dramatically reducing compute while maintaining support quality.

## docs/07 Reconciliation Note

`TASK-163` is partially fulfilled by this packet (architecture foundation). The algorithmic features (31b) complete the task.

Draft line for `docs/07_implementation_status.md` Workstream 3:

```
- [ ] TASK-163 (partial) Establish `SupportGeometryIR`, `PrePass::SupportGeometry`, `support_layer_height_mm`, and `support_top_z_distance_mm` as the architectural foundation for variable-height support planning. Support planner emits at coarse support resolution; emitter interpolates to model resolution near column tops. Continues TASK-120 acceptance evidence. Wired by packet `31a_support-geometry-prepass-and-layer-height`. Algorithms (avoidance, radius taper, raft, wall-count) ship in packet `31b_support-planner-algorithmic-parity`.
```

## Open Questions to Resolve Before Activation

All open questions are resolved:
- **Q1:** Accumulator approach — walk `LayerPlanIR.layers` accumulating `effective_layer_height`; emit support layer boundary when accumulated >= `support_layer_height_mm`.
- **Q2:** Intermediate model-resolution layers for `support_top_z_distance` refinement — add `SupportGeometryIR` entries at every model layer within `support_top_z_distance_mm` of contact Z; `global_support_layer_index = u32::MAX` sentinel.
- **Q3:** Sentinel = 0.0 for "use model layer height" — config schema `min > 0` ensures 0.0 is never a valid layer height.
- **Q4:** `SupportGeometryIR` is Tier-1-only and does not survive into Tier 2. Tree-support falls back to grid-MST when no `support-planner` is loaded.

## Parallelism Note

Packet 31a is serial by construction. It must run after packet 30 closes. Steps 2, 3, and 6 each block on an open question and must be resolved before proceeding.
