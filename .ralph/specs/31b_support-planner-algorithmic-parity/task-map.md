# Task Map: 31b_support-planner-algorithmic-parity → docs/07

This packet closes the five algorithmic v1 limitations of `support-planner` (gaps 3–7 from packet 28) using the architectural foundation established by packet `31a_support-geometry-prepass-and-layer-height`. It is additive — packets 26, 28, 30, and 31a stay `implemented` and are not superseded.

## Task ID Mapping

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-163` (algo) | Step 1 | `docs/01`–`docs/05`, `docs/08`, `docs/09` | none | `TreeSupport.cpp` 720–800, 1460–1700, 1913, 2625–2860; `TreeSupport.hpp`; `TreeModelVolumes.cpp` | Read-only discovery of OrcaSlicer references + confirmation of `SupportGeometryView` shape from 31a. |
| `TASK-163` (algo) | Step 2 | `docs/02` | `.ralph/specs/31b_support-planner-algorithmic-parity/design.md` | none | Resolve Q2 (raft Z convention, inherited from 31a). |
| `TASK-163` (algo) | Step 3 | `docs/03` | `modules/core-modules/support-planner/support-planner.toml` | none | Config schema rewrite (4 dropped, 9 added). |
| `TASK-163` (algo) | Step 4 | `docs/02`, `docs/04` | `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` (new) | `TreeSupport.cpp` 720–800, 2625–2860 | Failing TDD tests covering the new ACs. |
| `TASK-163` (algo) | Step 5 | `docs/05`, `docs/08` | `modules/core-modules/support-planner/src/lib.rs`, `crates/slicer-helpers/src/geometry.rs` | `TreeSupport.cpp` 720–800, 2625–2860; `TreeModelVolumes.cpp` | Avoidance, collision, wall-count move, radius tapering. Consumes `SupportGeometryView` from 31a. |
| `TASK-163` (algo) | Step 6 | `docs/02` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-host/src/dispatch.rs`, `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/support-planner/src/lib.rs` | `TreeSupport.cpp` 1460–1700, 1913 | Raft + interface densification. |
| `TASK-163` (algo) | Step 7 | `docs/03` | none (config schema bounds only) | none | Config-validation negatives (bounds in toml). |
| `TASK-163` (algo) | Step 8 | `docs/02`, `docs/12` | `resources/golden/benchy_tree_support_orca_branch_count.txt` (new), `resources/golden/benchy_tree_support_orca_endpoints.txt` (new), `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` | `TreeSupport.cpp` overall | Resolve Q3; Benchy OrcaSlicer parity check. |
| `TASK-163` (algo) | Step 9 | `docs/05` | `modules/core-modules/support-planner/src/lib.rs` | none | Remove v1 module-level doc bullets for limits 3–7. |
| `TASK-163` (algo) | Step 10 | `docs/03` | `modules/core-modules/support-planner/wit-guest/target/`, `support-planner.wasm` | none | Rebuild support-planner.wasm only (no WIT change — 31a already extended the WIT). |
| `TASK-163` (algo) | Step 11 | `docs/07` | `docs/07_implementation_status.md` | none | Backlog row. |
| `TASK-163` (algo) | Step 12 | `docs/11`, `docs/12` | none | none | Packet completion gate. |

## Superseding Relationship

Packet 31b does **not** supersede any prior packet.

- Packet `28_tree-support-multi-layer-propagation` (status: `implemented`). Documented all seven v1 limitations. Packets 30, 31a, and 31b close them additively.
- Packet `30_support-planner-prepass-wit-plumbing` (status: `implemented`). Closed limits 1 + 2 (layer-height-agnostic and single-region). Unchanged by 31b.
- Packet `31a_support-geometry-prepass-and-layer-height` (status: `implemented` before 31b activates). Established the architectural foundation: `SupportGeometryIR`, `PrePass::SupportGeometry`, `support_layer_height_mm`, `support_top_z_distance_mm`. 31b consumes `SupportGeometryView` and the new config keys.
- Packet `26_live-support-module-evidence` (status: `implemented`). Unchanged — the grid-MST fallback path is independent of all v2 algorithmic work.

## What Changed vs Prior Packet 31 Draft

The prior packet 31 design attempted to read `SliceIR` (Tier 2 data) during `PrePass::SupportGeneration` (Tier 1). This violated the project architecture — `SliceIR` is produced by `Layer::Slice` which runs after all prepass stages.

The revised 31a + 31b structure:

1. **Packet 31a** (architectural foundation): `PrePass::SupportGeometry` is a host-built-in prepass that computes coarse support outlines at support layer resolution using `LayerPlanIR` before any slicing. `SupportGeometryIR` is committed to the blackboard. `support_layer_height_mm` enables variable-height supports (a genuine ModularSlicer differentiator vs OrcaSlicer which ties support resolution to model resolution).
2. **Packet 31b** (algorithmic): The support-planner reads `SupportGeometryView` (projected from `SupportGeometryIR`) for avoidance/collision at support resolution. Near model contact zones, `SupportGeometryView` carries intermediate model-resolution layers (from 31a's `support_top_z_distance` refinement), so collision is accurate where it matters most.

This makes ModularSlicer strictly better than OrcaSlicer for high-resolution prints: supports use coarse resolution (3× fewer layers for a 0.3mm support layer height vs 0.1mm model), dramatically reducing compute while maintaining support quality.

## docs/07 Reconciliation Note

`TASK-163` is partially fulfilled by 31a (architecture) and 31b (algorithms). Draft lines for `docs/07_implementation_status.md` Workstream 3:

**31a line:**
```
- [ ] TASK-163 (partial) Establish `SupportGeometryIR`, `PrePass::SupportGeometry`, `support_layer_height_mm`, and `support_top_z_distance_mm` as the architectural foundation for variable-height support planning. Support planner emits at coarse support resolution; emitter interpolates to model resolution near column tops. Continues TASK-120 acceptance evidence. Wired by packet `31a_support-geometry-prepass-and-layer-height`.
```

**31b line (addendum):**
```
- [ ] TASK-163 (algorithmic) Close the five algorithmic v1 limitations (avoidance/collision cache from SupportGeometryView, radius tapering, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys) on the foundation established by packet `31a_support-geometry-prepass-and-layer-height`. Continues TASK-120 acceptance evidence. Wired by packet `31b_support-planner-algorithmic-parity`.
```

`TASK-120` (Phase H acceptance with tree supports) receives substantial new evidence after 31b lands.

## Open Questions to Resolve Before Activation

All open questions resolved:
- **Q2 (resolved):** Raft Z convention — signed `global_layer_index` (`i32`). Raft entries use `global_layer_index = -1, -2, ..., -raft_layers`.
- **Q3 (resolved):** Numerical tolerance — both branch count within ±10% **and** endpoint Hausdorff distance ≤ 0.5mm must hold. Either failing fails the test.
- Q1 (resolved by 31a): Support layer boundary — accumulator approach. Q2 (intermediate model-resolution layers for `support_top_z_distance`). Q3 (sentinel = 0.0 for model layer height). Q4 (SupportGeometryIR is Tier-1-only).

## Parallelism Note

Packet 31b is serial by construction. It must run after packet 31a closes. No parallel tracks. All open questions resolved before activation — no step blocks on an unresolved question.