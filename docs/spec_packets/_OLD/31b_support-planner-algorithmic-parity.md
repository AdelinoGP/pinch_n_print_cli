---
status: implemented
packet: 31b_support-planner-algorithmic-parity
task_ids:
  - TASK-163
---

# 31b_support-planner-algorithmic-parity

## Goal

Close the five algorithmic v1 limitations of `support-planner` (gaps 3–7 from packet 28) using the architectural foundation established by packet `31a_support-geometry-prepass-and-layer-height`: (3) avoidance/collision cache built from `SupportGeometryView` outlines at support resolution; (4) per-node radius tapering along `tan(tree_support_branch_diameter_angle) * dist_to_top`; (5) raft prefix layers per `support_raft_layers` and interface-layer densification per `support_interface_top_layers` / `support_interface_bottom_layers`; (6) wall-count-aware move scaling per `tree_support_wall_count`; (7) four OrcaSlicer config keys (`tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_diameter_angle`, `tree_support_branch_distance`) wired into the manifest. After this packet, `support-planner` implements the algorithmic shape of OrcaSlicer's `TreeSupport::drop_nodes` (avoidance/collision, radius taper, raft/interface, wall-count move scaling) and is anchored against drift by a deterministic self-capture regression check on the synthetic overhang fixture. External OrcaSlicer numerical parity is not in scope of this packet.

## Problem Statement

Packet `31a_support-geometry-prepass-and-layer-height` established the architectural foundation: `SupportGeometryIR` (coarse per-layer polygon outlines at support layer resolution), `PrePass::SupportGeometry` (a lightweight host-built-in prepass computing them), and `support_layer_height_mm` / `support_top_z_distance_mm` config keys.

This packet closes the five algorithmic v1 limitations of `support-planner` that remain after packet `30_support-planner-prepass-wit-plumbing` (packet 28's v1 limitations, gaps 3–7): (3) avoidance/collision cache using `SupportGeometryView` outlines; (4) per-node radius tapering; (5) raft prefix layers and interface-layer densification; (6) wall-count-aware move scaling; (7) four OrcaSlicer config keys wired into the manifest.

After this packet, `support-planner` implements OrcaSlicer's `TreeSupport::drop_nodes` algorithmic structure end-to-end, using variable-height support resolution from packet 31a. A deterministic self-capture regression anchor on the synthetic overhang fixture guards against drift; cross-slicer numerical parity against an external OrcaSlicer slice is explicitly not in scope.

## Architecture Constraints

- **`SupportGeometryView` is at support resolution, not model resolution.** The avoidance/collision polygons are built from coarse support layer outlines. The planner's propagation loop operates at support layer granularity. Near model contact zones, `SupportGeometryView` carries intermediate model-resolution layers (from 31a Q2 resolution), so collision is accurate at the critical interface.
- **No new WIT change in this packet.** Packet 31a already added `SupportGeometryView` as a WIT parameter on `run-support-geometry`. This packet only consumes it.
- **No new IR type.** `SupportPlanIR` is unchanged; the algorithmic changes affect only how the planner computes and emits entries.
- **Determinism.** The `SupportGeometryView` projection from 31a is already deterministic (sorted by `(global_support_layer_index, object_id, region_id)`). The avoidance polygon union uses deterministic Clipper-style operations.
- **Schema bump:** `SupportPlanEntry.global_layer_index` widened `u32` → `i32` (Q2 resolved path a).

## Data and Contract Notes

- **`SupportGeometryView` key:** `(global_support_layer_index, object_id, region_id) → Vec<ExPolygon>`. Intermediate model-resolution layers near column tops use `global_support_layer_index = u32::MAX` (per 31a Q2 resolution).
- **Avoidance formula:** `avoidance_polys = collision_polys.inflate(branch_radius + tree_support_branch_distance / 2)`. Config-driven (matches OrcaSlicer's `TreeModelVolumes.cpp`).
- **Radius taper formula:** `radius_mm = clamp(branch_diameter / 2 + tan(diameter_angle_rad) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`. `MAX_BRANCH_RADIUS = 6.0 mm` matches OrcaSlicer's hard upper clamp.
- **Wall-count move formula:** `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`.
- **`SupportPlanIR.global_layer_index`** resolves to `i32` if Q2 (raft Z convention from 31a) resolves to path (a); otherwise the host adds a `raft_layers` field and index is `u32`.
- **Diagnostic shape:** `Diagnostic { level: Warn, code: "support-planner.node-clamped-out", message: format!("node ({:.3},{:.3}) clamped-out at support layer {} after avoidance/collision check", x, y, layer), source: ModuleId("com.core.support-planner") }`.
- **Diagnostic delivery (v1).** No typed `Diagnostic` channel exists between guest WASM modules and the host yet, so the AC-N3 warning is emitted via `slicer_sdk::host::log(LogLevel::Warn, ...)` with the canonical `support-planner.node-clamped-out:` prefix carrying layer/object/position fields. Tests assert on the captured log via `slicer_sdk::host::test_support::install_log_capture`. Promoting this to a structured `Diagnostic` over the prepass output WIT is tracked as `TASK-163b` in `docs/07_implementation_status.md`.

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

- **Risk: regression-anchor goldens drift with intentional algorithmic improvements.** Mitigation: when an intentional improvement changes branch shape, re-capture goldens and explicitly note the re-capture in the packet that introduces the change.
- **Risk: avoidance projection produces oscillatory results.** Mitigation: clamp projected target to line segment between current node and original target; if outside avoidance_polys, drop the node and emit diagnostic.
- **Tradeoff: interface densification doubles entry count for top/bottom layers of every column.** Acceptable; tree-support emitter handles the count.
- **Tradeoff: coarse support resolution (from 31a) means fewer collision samples than model resolution.** Acceptable for v2. The `support_top_z_distance` refinement provides high-resolution collision near the model where it matters most.
