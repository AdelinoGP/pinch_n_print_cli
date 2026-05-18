# Requirements: 31b_support-planner-algorithmic-parity

## Packet Metadata

- Grouped task IDs:
  - `TASK-163` (algorithmic portion — architectural foundation in 31a)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: implemented

## Problem Statement

Packet `31a_support-geometry-prepass-and-layer-height` established the architectural foundation: `SupportGeometryIR` (coarse per-layer polygon outlines at support layer resolution), `PrePass::SupportGeometry` (a lightweight host-built-in prepass computing them), and `support_layer_height_mm` / `support_top_z_distance_mm` config keys.

This packet closes the five algorithmic v1 limitations of `support-planner` that remain after packet `30_support-planner-prepass-wit-plumbing` (packet 28's v1 limitations, gaps 3–7): (3) avoidance/collision cache using `SupportGeometryView` outlines; (4) per-node radius tapering; (5) raft prefix layers and interface-layer densification; (6) wall-count-aware move scaling; (7) four OrcaSlicer config keys wired into the manifest.

After this packet, `support-planner` implements OrcaSlicer's `TreeSupport::drop_nodes` algorithmic structure end-to-end, using variable-height support resolution from packet 31a. A deterministic self-capture regression anchor on the synthetic overhang fixture guards against drift; cross-slicer numerical parity against an external OrcaSlicer slice is explicitly not in scope.

## In Scope

- `SupportGeometryView` consumption in `support-planner`: per-layer avoidance + collision polygon sets built from `SupportGeometryView.outlines` at support resolution. Move-pass clamps nodes into avoidance polygons; drops + diagnoses nodes whose target lies inside collision polygons.
- **Per-node radius tapering.** `PlannedSupportNode` gains `dist_to_top: u32`; per-emit radius is `clamp(branch_diameter / 2 + tan(diameter_angle_rad) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`. `Point3WithWidth.width = 2 * radius`.
- **Raft prefix layers.** When `support_raft_layers > 0`, prepend that many entries with negative `global_layer_index` (per Q2 resolution from 31a). Each raft entry carries dense full-cross-section fill segments.
- **Interface-layer densification.** For the top `support_interface_top_layers` and bottom `support_interface_bottom_layers` layers of each branch column, emit additional dense fill at line spacing `tree_support_interface_spacing_mm`.
- **Wall-count-aware move scaling.** `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`.
- Config keys: `tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_diameter_angle`, `tree_support_branch_distance`, `tree_support_wall_count`, `support_raft_layers`, `support_interface_top_layers`, `support_interface_bottom_layers`, `tree_support_interface_spacing_mm` on `support-planner.toml`. Drop v1 keys `support_branch_angle_deg`, `support_branch_merge_distance_mm`, `support_max_branches_per_layer`, `line_width`.
- New test file `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs`.
- Golden fixtures: `resources/golden/benchy_tree_support_orca_branch_count.txt`, `resources/golden/benchy_tree_support_orca_endpoints.txt`.
- Module-level v1 limitation doc bullets removed from `support-planner/src/lib.rs`.

## Out of Scope

- Replacing `MinimumSpanningTree::prim` with a heap-based variant.
- Soluble multi-extruder interface support material.
- Catchup / variable-per-region effective layer-height interactions.
- GUI / global-config plumbing outside module manifests.
- Geometry-aware multi-region branch separation.
- Tree-support emitter changes.
- Changes to `Layer::Support` claim layout or scheduling.
- The architectural foundation (SupportGeometryIR, PrePass::SupportGeometry, support_layer_height_mm, support_top_z_distance_mm) — already in 31a.

## Authoritative Docs

- `docs/01_system_architecture.md` — Tier 1 PrePass (sequential).
- `docs/02_ir_schemas.md` — `SupportGeometryIR` (from 31a), `SupportPlanIR`.
- `docs/03_wit_and_manifest.md` — manifest `[ir-access].reads`, config-schema validation.
- `docs/04_host_scheduler.md` — `PrePass::SupportGeometry` prerequisite chain.
- `docs/05_module_sdk.md` — config schema bounds enforcement.
- `docs/08_coordinate_system.md` — mm convention for radius, raft Z values.
- `docs/09_progress_events.md` — diagnostic emission contract.
- `.ralph/specs/31a_support-geometry-prepass-and-layer-height/` — architectural foundation this packet builds on.
- `.ralph/specs/30_support-planner-prepass-wit-plumbing/` — `LayerPlanView` and `RegionSegmentationView` from packet 30.
- `.ralph/specs/28_tree-support-multi-layer-propagation/` — original simplified port whose v1 limitations this packet closes.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` lines 720–800, 1460–1700, 1913, 2625–2860.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` (`SupportNode`, `TreeSupportData`).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeModelVolumes.cpp` (avoidance inflation).
- `OrcaSlicerDocumented/src/libslic3r/MinimumSpanningTree.cpp::prim` — unchanged.

## Acceptance Summary

- **Positive cases:**
  - Config schema has 9 new keys, lacks 4 v1 keys (AC-1).
  - Radius tapering observable in emitted widths (AC-2).
  - Avoidance keeps branches inside coarse support outlines (AC-3).
  - Raft + interface entry counts match expectations (AC-4).
  - Wall-count scaling on move distance (AC-5).
  - Self-capture regression anchor on synthetic overhang fixture within tolerance (AC-6).
  - Build succeeds (AC-7).
  - `TASK-163` row in `docs/07` (AC-8).
- **Negative cases:**
  - Out-of-range `tree_support_branch_diameter_angle` rejects load.
  - Negative `support_raft_layers` rejects load.
  - Node fully boxed-in by avoidance + collision is dropped with `node-clamped-out` diagnostic.

Draft line for `docs/07_implementation_status.md` (Workstream 3 — addendum to 31a):

```
- [ ] TASK-163 (algorithmic) Close the five algorithmic v1 limitations (avoidance/collision cache from SupportGeometryView, radius tapering, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys) on the foundation established by packet `31a_support-geometry-prepass-and-layer-height`. Continues TASK-120 acceptance evidence. Wired by packet `31b_support-planner-algorithmic-parity`.
```

## Cross-Packet Dependencies and Unblockers

- **Depends on:** packet `31a_support-geometry-prepass-and-layer-height` (must be `status: implemented`).
- **Does not supersede:** anything. Purely additive to 31a.
- **Unblocks:** Phase H tree-support visual-parity tickets under TASK-120.

## Verification Commands

```
cargo test -p slicer-host --test prepass_support_geometry_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test prepass_support_geometry_layer_plan_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test support_geometry_prepass_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test live_layer_support_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture
cargo test -p support-planner --lib
cargo build --workspace
cargo clippy --workspace -- -D warnings
```