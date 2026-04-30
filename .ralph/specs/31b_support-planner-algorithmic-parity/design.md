# Design: 31b_support-planner-algorithmic-parity

## Controlling Code Paths

- **Primary code paths:**
  - `modules/core-modules/support-planner/support-planner.toml` — config schema rewrite (4 dropped, 9 added).
  - `modules/core-modules/support-planner/src/lib.rs` — avoidance/collision build from `SupportGeometryView`, radius tapering, raft prefix, interface densification, wall-count move scaling, `dist_to_top` tracking, `MAX_BRANCH_RADIUS` constant, v1 doc bullet removal.
  - `crates/slicer-helpers/src/geometry.rs` — `polygon_inflate`, `point_in_polygons`, `hausdorff_distance` helpers.
- **Neighboring tests or fixtures:**
  - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` (packet 28) — must remain green.
  - `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` (packet 30) — must remain green.
  - `crates/slicer-host/tests/support_geometry_prepass_tdd.rs` (packet 31a) — must remain green.
  - `crates/slicer-host/tests/live_support_generation_tdd.rs` — must remain green.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — must remain green.
  - new file: `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs`.
  - new fixtures: `resources/golden/benchy_tree_support_orca_branch_count.txt`, `resources/golden/benchy_tree_support_orca_endpoints.txt`.

## Architecture Constraints

- **`SupportGeometryView` is at support resolution, not model resolution.** The avoidance/collision polygons are built from coarse support layer outlines. The planner's propagation loop operates at support layer granularity. Near model contact zones, `SupportGeometryView` carries intermediate model-resolution layers (from 31a Q2 resolution), so collision is accurate at the critical interface.
- **No new WIT change in this packet.** Packet 31a already added `SupportGeometryView` as a WIT parameter on `run-support-geometry`. This packet only consumes it.
- **No new IR type.** `SupportPlanIR` is unchanged; the algorithmic changes affect only how the planner computes and emits entries.
- **Determinism.** The `SupportGeometryView` projection from 31a is already deterministic (sorted by `(global_support_layer_index, object_id, region_id)`). The avoidance polygon union uses deterministic Clipper-style operations.
- **Schema bump.** If Q2 (raft Z convention) resolves to signed `global_layer_index`.

## Code Change Surface

### Selected approach

**Consume `SupportGeometryView` for collision; apply radius taper + wall-count + raft + interface on top.**

The planner reads `SupportGeometryView.outlines` at support resolution. Per support layer `L`, it builds `collision_polys = union(SG[L][object_id][region_id].outlines)` and `avoidance_polys = collision_polys.inflate(branch_radius + tree_support_branch_distance / 2)`. The propagation move-pass uses `max_move_distance = tan(branch_angle) * effective_layer_height * wall_count.max(1)` and clamps into `avoidance_polys`. Each `PlannedSupportNode` tracks `dist_to_top`; at emit time radius is `clamp(branch_diameter / 2 + tan(diameter_angle) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`. Raft entries are prepended with negative `global_layer_index`. Interface densification adds dense fill segments for the top/bottom N layers of each branch column.

### Exact functions, traits, manifests, tests, or fixtures expected to change

**Created:**
- `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` — 8 tests (5 positive + 3 negative).
- `resources/golden/benchy_tree_support_orca_branch_count.txt` — single integer.
- `resources/golden/benchy_tree_support_orca_endpoints.txt` — newline-delimited `x,y,z` coordinates.

**Modified — module:**
- `modules/core-modules/support-planner/support-planner.toml` — config schema rewrite (4 dropped, 9 added).
- `modules/core-modules/support-planner/src/lib.rs` — comprehensive rewrite of propagation block; v1 doc bullets removed; `MAX_BRANCH_RADIUS = 6.0` constant added; `dist_to_top` field on `PlannedSupportNode`.

**Modified — IR (raft only if Q2 resolves to signed global_layer_index):**
- `crates/slicer-ir/src/slice_ir.rs` — `SupportPlanEntry.global_layer_index` widened to `i32`.

**Modified — backlog:**
- `docs/07_implementation_status.md` — `TASK-163` row (algorithmic portion).

### Rejected alternatives

- **Per-model-layer collision instead of per-support-layer.** Rejected — the architecture from 31a intentionally plans at coarse support resolution. Per-model-layer collision would require running `PrePass::SupportGeometry` at model resolution, defeating the performance advantage.
- **Lazy TreeSupportData-style avoidance cache.** Rejected — `SupportGeometryView` is pre-computed by 31a and already available. Eager per-support-layer build is simpler and fits the propagation model.
- **Skip raft layers.** Raft is one of the five algorithmic gaps; skipping it would re-open a future packet.

## Data and Contract Notes

- **`SupportGeometryView` key:** `(global_support_layer_index, object_id, region_id) → Vec<ExPolygon>`. Intermediate model-resolution layers near column tops use `global_support_layer_index = u32::MAX` (per 31a Q2 resolution).
- **Avoidance formula:** `avoidance_polys = collision_polys.inflate(branch_radius + tree_support_branch_distance / 2)`. Config-driven (matches OrcaSlicer's `TreeModelVolumes.cpp`).
- **Radius taper formula:** `radius_mm = clamp(branch_diameter / 2 + tan(diameter_angle_rad) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`. `MAX_BRANCH_RADIUS = 6.0 mm` matches OrcaSlicer's hard upper clamp.
- **Wall-count move formula:** `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`.
- **`SupportPlanIR.global_layer_index`** resolves to `i32` if Q2 (raft Z convention from 31a) resolves to path (a); otherwise the host adds a `raft_layers` field and index is `u32`.
- **Diagnostic shape:** `Diagnostic { level: Warn, code: "support-planner.node-clamped-out", message: format!("node ({:.3},{:.3}) clamped-out at support layer {} after avoidance/collision check", x, y, layer), source: ModuleId("com.core.support-planner") }`.

## Locked Assumptions and Invariants

1. Packet 31a's `SupportGeometryView` shape is stable — this packet consumes it without modifying the WIT.
2. `LayerPlanView` and `RegionSegmentationView` from packet 30 are unchanged.
3. `tree_support_wall_count = 0` falls through to `max(1, n)` per OrcaSlicer line 2632.
4. `tree_support_branch_diameter` is the diameter (not radius) per OrcaSlicer; `branch_radius = tree_support_branch_diameter / 2`.
5. Raft layer height = `effective_layer_height` of layer 0 (no separate config).
6. Dense-fill segments produced via rectilinear scan-line, deterministic.
7. `MAX_BRANCH_RADIUS = 6.0 mm` matches OrcaSlicer's hard upper clamp.
8. `support_top_z_distance_mm` refinement (from 31a) ensures intermediate model-resolution layers exist near column tops — collision is accurate at the critical interface even though the propagation loop operates at support resolution.
9. Packet 26's grid-MST fallback in `tree-support` remains the path when `support-planner` is not loaded.

## Risks and Tradeoffs

- **Risk: Benchy parity tolerance too tight or loose.** Q3 resolution decides; the packet stays draft until decided.
- **Risk: avoidance projection produces oscillatory results.** Mitigation: clamp projected target to line segment between current node and original target; if outside avoidance_polys, drop the node and emit diagnostic.
- **Tradeoff: interface densification doubles entry count for top/bottom layers of every column.** Acceptable; tree-support emitter handles the count.
- **Tradeoff: coarse support resolution (from 31a) means fewer collision samples than model resolution.** Acceptable for v2. The `support_top_z_distance` refinement provides high-resolution collision near the model where it matters most.

## Open Questions

All open questions are resolved.

- **Q1 (resolved by 31a):** Support layer boundary — accumulator approach. Q2 (intermediate model-resolution layers for `support_top_z_distance`). Q3 (sentinel = 0.0 for model layer height).
- **Q2 (resolved):** Raft Z convention — **(a) Signed `global_layer_index` (`i32`)**. Simpler than a separate `raft_layers` field on `SupportPlanIR`. Raft entries use `global_layer_index = -1, -2, ..., -raft_layers` with Z values `z_bed - (i+1) * raft_layer_height_mm`. Host `harvest_support_plan_ir` and tree-support's `support_plan_segments_for` handle negative indices.
- **Q3 (resolved):** Numerical tolerance for Benchy parity check — **(c) Both must hold**: branch count within ±10% **AND** endpoint Hausdorff distance ≤ 0.5mm. Either failing means the test fails.