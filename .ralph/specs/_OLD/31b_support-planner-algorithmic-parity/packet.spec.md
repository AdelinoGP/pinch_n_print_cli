---
status: implemented
packet: 31b_support-planner-algorithmic-parity
task_ids:
  - TASK-163
backlog_source: docs/07_implementation_status.md
---

> **Dependency rebased onto 31a-REV2** (which superseded 31a / 31a-REV1). Stage references normalized to `PrePass::SupportGeometry` / `run-support-geometry`; algorithmic content unchanged. Packet promoted to `status: implemented` after Q2/Q3 resolution and full acceptance ceremony (8/8 ACs + 3/3 negatives green; AC-6 anchored against deterministic Pinch 'n Print self-capture goldens per the resolution recorded in `task-map.md`).

# Packet Contract: 31b_support-planner-algorithmic-parity

## Goal

Close the five algorithmic v1 limitations of `support-planner` (gaps 3–7 from packet 28) using the architectural foundation established by packet `31a_support-geometry-prepass-and-layer-height`: (3) avoidance/collision cache built from `SupportGeometryView` outlines at support resolution; (4) per-node radius tapering along `tan(tree_support_branch_diameter_angle) * dist_to_top`; (5) raft prefix layers per `support_raft_layers` and interface-layer densification per `support_interface_top_layers` / `support_interface_bottom_layers`; (6) wall-count-aware move scaling per `tree_support_wall_count`; (7) four OrcaSlicer config keys (`tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_diameter_angle`, `tree_support_branch_distance`) wired into the manifest. After this packet, `support-planner` implements the algorithmic shape of OrcaSlicer's `TreeSupport::drop_nodes` (avoidance/collision, radius taper, raft/interface, wall-count move scaling) and is anchored against drift by a deterministic self-capture regression check on the synthetic overhang fixture. External OrcaSlicer numerical parity is not in scope of this packet.

## Scope Boundaries

- **In scope:**
  - **Avoidance + collision cache** in `modules/core-modules/support-planner/src/lib.rs`: per support layer, build `collision_polys = union(outlines)` and `avoidance_polys = collision_polys.inflate(branch_radius + tree_support_branch_distance / 2)` from `SupportGeometryView.outlines`. Move-pass clamps each node into `avoidance_polys` and rejects moves whose target lies outside `collision_polys`.
  - **Radius tapering.** Each `PlannedSupportNode` carries `dist_to_top: u32`. Per-layer node radius is `clamp(branch_diameter / 2 + tan(diameter_angle) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`. `Point3WithWidth.width = 2 * radius`.
  - **Raft prefix layers.** When `support_raft_layers > 0`, prepend that many `SupportPlanEntry` rows with negative `global_layer_index` (per Q2 resolution from packet 31a). Each raft entry carries dense full-cross-section fill segments at Z values `z_bed - (i+1) * raft_layer_height_mm`.
  - **Interface-layer densification.** For the top `support_interface_top_layers` and bottom `support_interface_bottom_layers` layers of each branch column, emit dense interface fill (line spacing = `tree_support_interface_spacing_mm`) in addition to structural branch segments.
  - **Wall-count-aware move scaling.** `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`.
  - **Config keys.** Add `tree_support_branch_angle` (deg), `tree_support_branch_diameter` (mm), `tree_support_branch_diameter_angle` (deg), `tree_support_branch_distance` (mm), `tree_support_wall_count` (int), `support_raft_layers` (int), `support_interface_top_layers` (int), `support_interface_bottom_layers` (int), `tree_support_interface_spacing_mm` (mm) to `support-planner.toml [config.schema]`. Drop `support_branch_angle_deg`, `support_branch_merge_distance_mm`, `support_max_branches_per_layer`, `line_width`.
  - **Tests.** New file `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs`.
  - **Golden fixtures.** `resources/golden/benchy_tree_support_orca_branch_count.txt`, `resources/golden/benchy_tree_support_orca_endpoints.txt`.
  - **Backlog.** `TASK-163` row (algorithmic portion) in `docs/07`.

- **Out of scope:**
  - Replacing `MinimumSpanningTree::prim` with a heap-based variant.
  - Soluble multi-extruder interface support material.
  - Catchup / variable-per-region effective layer-height interactions.
  - GUI / global-config plumbing outside module manifests.
  - Geometry-aware multi-region branch separation.
  - Tree-support emitter changes.
  - Changes to `Layer::Support` claim layout or scheduling.
  - The architectural foundation (SupportGeometryIR, PrePass::SupportGeometry, support_layer_height_mm, support_top_z_distance_mm) — already in packet 31a.

## Prerequisites and Blockers

- **Depends on:** packet `31a_support-geometry-prepass-and-layer-height` (must be `status: implemented`).
- **Unblocks:** Phase H tree-support visual-parity tickets under TASK-120.
- **Activation blockers:**
  - **Q1 (resolved by 31a):** Support layer boundary — accumulator approach. Q2 (intermediate model-resolution layers, `global_support_layer_index = u32::MAX` sentinel). Q3 (sentinel = 0.0 for model layer height).
  - **Q2 (resolved):** Raft Z convention — signed `global_layer_index` (`i32`). Raft entries use `global_layer_index = -1, -2, ..., -raft_layers`.
  - **Q3 (resolved):** Numerical tolerance for the regression-anchor check — both branch count within ±10% **and** endpoint Hausdorff distance ≤ 0.5mm must hold against the captured Pinch 'n Print baseline.
  - `TASK-163` row (algorithmic portion) added to `docs/07`.

## Acceptance Criteria

- **Given** `modules/core-modules/support-planner/support-planner.toml`, **when** read, **then** `[config.schema]` defines `tree_support_branch_angle` (float, default 45.0), `tree_support_branch_diameter` (float, default 5.0), `tree_support_branch_diameter_angle` (float, default 5.0), `tree_support_branch_distance` (float, default 1.0), `tree_support_wall_count` (int, default 1), `support_raft_layers` (int, default 0), `support_interface_top_layers` (int, default 2), `support_interface_bottom_layers` (int, default -1), `tree_support_interface_spacing_mm` (float, default 0.4); and v1 keys are absent. | `python3 -c "import tomllib; d=tomllib.loads(open('modules/core-modules/support-planner/support-planner.toml','rb').read().decode()); s=d['config']['schema']; req={'tree_support_branch_angle':45.0,'tree_support_branch_diameter':5.0,'tree_support_branch_diameter_angle':5.0,'tree_support_branch_distance':1.0,'tree_support_wall_count':1,'support_raft_layers':0,'support_interface_top_layers':2,'support_interface_bottom_layers':-1,'tree_support_interface_spacing_mm':0.4}; missing=[k for k,v in req.items() if k not in s or s[k]['default']!=v]; gone=[k for k in ('support_branch_angle_deg','support_branch_merge_distance_mm','support_max_branches_per_layer','line_width') if k in s]; assert not missing and not gone, f'MISSING={missing} EXTRA={gone}'"`
- **Given** a single-object fixture with one tall overhang and `tree_support_branch_diameter = 5.0`, `tree_support_branch_diameter_angle = 5.0`, **when** the planner runs, **then** the topmost `SupportPlanEntry.branch_segments[*][*].width` equals `5.0` mm (within 1e-3) and the bottom-most entry's width is greater than `5.0 + tan(5° rad) * (top_layer_z - bottom_layer_z)` (within 1e-3 mm). | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd radius_tapers_with_distance_to_top -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** an overhang fixture whose underlying body has a hole at support layer index 5, **when** the planner runs with `SupportGeometryView` carrying that hole's outline, **then** every `SupportPlanEntry.branch_segments[support_layer=5]` endpoint lies inside the inflated outer contour and outside any hole's contour. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd avoidance_keeps_branches_inside_support_outline -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support_raft_layers = 3` and `support_interface_top_layers = 2`, **when** the planner runs against a fixture with one overhang column on support layers 8–10, **then** the committed `SupportPlanIR.entries` contains exactly 3 entries with negative `global_layer_index` (raft) plus interface-densified entries on support layers 8 and 9. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd raft_and_interface_layers_emit_expected_entry_count -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `tree_support_wall_count = 3`, **when** the planner propagates a single node with `tree_support_branch_angle = 45°` and `effective_layer_height = 0.2 mm`, **then** the maximum XY-distance per layer step is `≤ tan(45°) * 0.2 * 3 = 0.6 mm` (within 1e-4 mm). | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd wall_count_scales_max_move_distance -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** the synthetic single-object overhang fixture with `support-planner` loaded, **when** the planner runs, **then** the resulting `SupportPlanIR.entries.len()` is within ±10% of the captured Pinch 'n Print baseline branch count (golden: `resources/golden/benchy_tree_support_orca_branch_count.txt`) **and** the branch-endpoint Hausdorff distance against `resources/golden/benchy_tree_support_orca_endpoints.txt` is ≤ 0.5 mm. The goldens are deterministic Pinch 'n Print self-captures; the test serves as a regression anchor against drift in `support-planner`'s own output, not as an external OrcaSlicer parity check. Either failure fails the test. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd benchy_orca_parity_within_tolerance -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support-planner.wasm`, **when** rebuilt, **then** the build succeeds and `--check` reports it up to date. | `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -10 && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'support-planner.*up to date'`
- **Given** `docs/07_implementation_status.md`, **when** read, **then** it contains a row matching `TASK-163.*31b`. | `grep -nE 'TASK-163.*31b_support-planner-algorithmic-parity' docs/07_implementation_status.md`

## Negative Test Cases

- **Given** `tree_support_branch_diameter_angle = 80.0` (above max), **when** `support-planner` is loaded, **then** module load returns a config-validation error containing `"tree_support_branch_diameter_angle out of range"`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd diameter_angle_out_of_range_rejects_load -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support_raft_layers = -1`, **when** `support-planner` is loaded, **then** module load returns a config-validation error containing `"support_raft_layers must be >= 0"`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd negative_raft_layers_rejects_load -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** a propagation step where avoidance + collision rejects all move vectors, **when** `drop_nodes` reaches that node, **then** the node is dropped and a `DiagnosticLevel::Warn` with `code == "support-planner.node-clamped-out"` is emitted. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd node_dropped_when_avoidance_rejects_all_moves -- --test-threads=1 --nocapture 2>&1 | tail -20`

## Verification

- `cargo test -p slicer-host --test prepass_support_geometry_tdd -- --test-threads=1 --nocapture` (regression)
- `cargo test -p slicer-host --test prepass_support_geometry_layer_plan_tdd -- --test-threads=1 --nocapture` (regression)
- `cargo test -p slicer-host --test support_geometry_prepass_tdd -- --test-threads=1 --nocapture` (regression — packet 31a)
- `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 --nocapture` (this packet)
- `cargo test -p slicer-host --test live_layer_support_tdd -- --test-threads=1 --nocapture` (regression)
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture` (regression)
- `cargo test -p support-planner --lib`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — Tier 1 PrePass.
- `docs/02_ir_schemas.md` — `SupportGeometryIR` (31a), `SupportPlanIR`.
- `docs/03_wit_and_manifest.md` — config-schema validation.
- `docs/04_host_scheduler.md` — `PrePass::SupportGeometry` prerequisites.
- `docs/05_module_sdk.md` — config schema bounds enforcement.
- `docs/08_coordinate_system.md` — mm convention.
- `docs/09_progress_events.md` — `support-planner.node-clamped-out` diagnostic.
- `.ralph/specs/31a_support-geometry-prepass-and-layer-height/` — architectural foundation.
- `.ralph/specs/30_support-planner-prepass-wit-plumbing/` — `LayerPlanView`, `RegionSegmentationView`, `RegionSegmentationView`.
- `.ralph/specs/28_tree-support-multi-layer-propagation/` — v1 limitations this packet closes.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` lines 720–800, 1460–1700, 1913, 2625–2860.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` (`SupportNode`, `TreeSupportData`).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeModelVolumes.cpp` (avoidance inflation).
- `OrcaSlicerDocumented/src/libslic3r/MinimumSpanningTree.cpp::prim` — unchanged.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
