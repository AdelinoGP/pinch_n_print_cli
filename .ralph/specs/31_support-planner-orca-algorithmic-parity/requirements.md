# Requirements: 31_support-planner-orca-algorithmic-parity

## Packet Metadata

- Grouped task IDs:
  - `TASK-163`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Packet `28_tree-support-multi-layer-propagation` shipped a deliberately-simplified port of OrcaSlicer's `TreeSupport::detect_overhangs` + `drop_nodes`. The module-level docs in `modules/core-modules/support-planner/src/lib.rs` enumerated seven v1 limitations. Packet `30_support-planner-prepass-wit-plumbing` closed the two correctness gaps (layer-height-agnostic; single-region per object). The remaining five gaps are algorithmic: branches do not avoid the underlying body geometry, every branch is the same width, no raft or interface layers are emitted, the per-layer move step ignores `tree_support_wall_count`, and the four OrcaSlicer config keys (`tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_diameter_angle`, `tree_support_branch_distance`) are absent. These five gaps together prevent the module from producing OrcaSlicer-grade tree supports on real models — branches pass through walls, plate-adjacent branches are unrealistically thin, raft layers do not exist for low-bed-adhesion materials, interface densification is missing so support detaches from the model, and movement scaling is off by a factor of `wall_count`.

This packet closes all five gaps in one bounded slice. The single key blocker is per-layer 2D outline geometry (`SlicePreviewIR`): every algorithmic feature here needs it (avoidance for collisions, radius tapering for clamp-to-region, raft for cross-section, interface for fill-area, wall-count move for boundary handling). Rather than recompute it inside the module, we add a host-built-in `PrePass::SlicePreview` stage that runs after `PrePass::RegionMapping` and before `PrePass::SupportGeneration` (subject to Q3 below) and commits a `SlicePreviewIR` to the blackboard. The planner reads it through a new `slice-preview-view` WIT record produced by the existing host projector pattern from packet 30.

This packet does **not** supersede packets 28 or 30. Packet 28 still owns the simplified-port decision and the `SupportPlanIR` blackboard contract. Packet 30 still owns the WIT plumbing for `LayerPlanIR` and `RegionMapIR`. This packet extends both additively and removes the v1-limitation bullets from the module-level docs.

## In Scope

- New host-built-in PrePass stage `PrePass::SlicePreview` (resolution dependent on Q3) that produces `SlicePreviewIR` from `MeshIR` + `LayerPlanIR` + `RegionMapIR` via plane-triangle intersection at every layer's Z.
- New IR type `SlicePreviewIR` in `crates/slicer-ir/src/slice_ir.rs` keyed `(global_layer_index, object_id, region_id) → Vec<ExPolygon>`.
- New WIT records `slice-preview-view-entry` + `slice-preview-view` and an additional `slice-preview` parameter on `export run-support-generation`.
- New SDK types `SlicePreviewView` + `SlicePreviewViewEntry` in `crates/slicer-sdk/src/prepass_types.rs`, re-exported from the prelude.
- Host wiring: `BlackboardPrepassSlot::SlicePreview` enum variant + `commit_slice_preview` + accessor; extension of `required_slots` to `[SurfaceClassification, LayerPlan, RegionMap, SlicePreview]`; `wit_host.rs::project_slice_preview_view` deterministic projector.
- `support-planner` algorithmic changes:
  - **Avoidance + collision cache** built once per layer from `slice_preview_view`. Move-pass clamps each node into the inflated outline and rejects moves whose target lies outside the un-inflated outline. Nodes whose every move direction is rejected are dropped with a `support-planner.node-clamped-out` diagnostic.
  - **Per-node radius tapering.** `PlannedSupportNode` gains `dist_to_top: u32`; per-emit radius is `clamp(branch_diameter / 2 + tan(diameter_angle_rad) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`. `Point3WithWidth.width = 2 * radius`.
  - **Raft prefix layers.** When `support_raft_layers > 0`, prepend that many entries with `global_layer_index ∈ [-raft_layers, -1]` (signed widening of `global_layer_index` — see Q2). Each raft entry carries dense full-cross-section fill segments.
  - **Interface-layer densification.** For the top `support_interface_top_layers` and bottom `support_interface_bottom_layers` layers of each branch column, emit additional dense fill at line spacing `tree_support_interface_spacing_mm`.
  - **Wall-count-aware move.** `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`.
- `support-planner.toml` config schema: nine new keys (`tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_diameter_angle`, `tree_support_branch_distance`, `tree_support_wall_count`, `support_raft_layers`, `support_interface_top_layers`, `support_interface_bottom_layers`, `tree_support_interface_spacing_mm`) and four removed keys (`support_branch_angle_deg`, `support_branch_merge_distance_mm`, `support_max_branches_per_layer`, `line_width`).
- `support-planner.toml [ir-access].reads` adds `"SlicePreviewIR"`.
- New test file `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` with one positive AC per limit + numerical Benchy parity check.
- Golden fixtures: `resources/golden/benchy_tree_support_orca_branch_count.txt`, `resources/golden/benchy_tree_support_orca_endpoints.txt` (extracted once from a clean OrcaSlicer slice of `resources/test_models/benchy.stl` with `resources/test_config/benchy-tree-support.json`).
- Module-level v1 limitation doc bullets removed from `support-planner/src/lib.rs`.
- Backlog: `TASK-163` row appended to `docs/07_implementation_status.md`.

## Out of Scope

- Replacing `MinimumSpanningTree::prim` with a heap-based variant (carries forward).
- Soluble multi-extruder interface support material.
- Catchup / variable-per-region effective layer-height interactions inside one object on one layer (planner uses `LayerPlanView.layers[i].effective_layer_height` per global layer; per-region overrides are deferred).
- GUI / global-config plumbing for the new keys (manifest-only).
- Geometry-aware multi-region branch separation when an object owns several regions on one layer that do not overlap geometrically — every region still receives the same branch set, then per-region clamping uses that region's outlines.
- Tree-support emitter changes beyond honoring `Point3WithWidth.width`.
- Changes to `Layer::Support` claim layout or scheduling.
- The `TreeSupportData` lazy cache (we ship a single-pass simplification — outlines are computed eagerly at PrePass time, not lazily inside the planner loop).

## Authoritative Docs

- `docs/01_system_architecture.md` — pipeline tiers and the new `PrePass::SlicePreview` placement.
- `docs/02_ir_schemas.md` — `SlicePreviewIR` shape; `SupportPlanIR` schema bump if signed `global_layer_index` is chosen for raft (Q2).
- `docs/03_wit_and_manifest.md` — prepass world, additive WIT change rebuild rule, config-schema validation.
- `docs/04_host_scheduler.md` — built-in vs user PrePass stage decision (Q3), `ensure_stage_prerequisites`.
- `docs/05_module_sdk.md` — config schema bounds enforcement.
- `docs/08_coordinate_system.md` — radius/width mm convention; raft Z values relative to bed.
- `docs/09_progress_events.md` — diagnostic emission contract.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` lines 720–800, 1460–1700, 1913, 2625–2860 — propagation, interface, raft and avoidance reference.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — `SupportNode` and `TreeSupportData` reference.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeModelVolumes.cpp` — avoidance polygon inflation logic.
- `OrcaSlicerDocumented/src/libslic3r/MinimumSpanningTree.cpp::prim` — unchanged O(V²) Prim.

## Acceptance Summary

- **Positive cases:**
  - WIT and IR additions for `SlicePreviewIR` (AC-1, AC-2).
  - Prerequisite slice extension to `[SurfaceClassification, LayerPlan, RegionMap, SlicePreview]` (AC-3).
  - Manifest config schema (9 keys present, 4 v1 keys absent) (AC-4).
  - Radius tapering observable in emitted widths (AC-5).
  - Avoidance keeps branches inside layer outlines (AC-6).
  - Raft + interface entry counts match expectations (AC-7).
  - Wall-count scaling on move distance (AC-8).
  - Benchy OrcaSlicer parity within tolerance (AC-9).
  - Build cascade succeeds (AC-10).
  - `TASK-163` row in `docs/07` (AC-11).
- **Negative cases:**
  - Missing `SlicePreviewIR` prerequisite returns `MissingRequiredPrepass`.
  - Out-of-range `tree_support_branch_diameter_angle` rejects module load.
  - Negative `support_raft_layers` rejects module load.
  - Node fully boxed-in by avoidance + collision is dropped with a `node-clamped-out` diagnostic.
- **Measurable outcomes:**
  - Module-level v1 doc bullets fully removed.
  - All packet 28 + 30 tests continue passing.
  - Packet 21/26 Benchy support tests continue passing.
  - `benchy_orca_parity_within_tolerance` passes against the golden fixtures.
- **Cross-packet impact:** Adds a built-in `PrePass::SlicePreview` stage; any future packet that schedules `SupportGeneration` must continue scheduling the upstream chain.

Draft line for `docs/07_implementation_status.md` (Workstream 3):

```
- [ ] TASK-163 Close the five algorithmic v1 limitations of `support-planner` (avoidance/collision cache, radius tapering, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys) by introducing `PrePass::SlicePreview` + `SlicePreviewIR` and consuming the per-layer outlines through a new `slice-preview-view` on `run-support-generation`. Continues TASK-120 acceptance evidence. Wired by packet `31_support-planner-orca-algorithmic-parity`.
```

## Cross-Packet Dependencies and Unblockers

- **Depends on:** packet `30_support-planner-prepass-wit-plumbing` (must be `implemented`).
- **Does not supersede:** anything. Additive correction of v1 algorithmic carve-outs.
- **Unblocks:** Phase H tree-support visual-parity tickets under TASK-120.

## Verification Commands

```
cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture
cargo test -p support-planner --lib
bash modules/core-modules/build-core-modules.sh
bash modules/core-modules/build-core-modules.sh --check
cargo build --workspace
cargo clippy --workspace -- -D warnings
```

## Step Completion Expectations

See `implementation-plan.md` — every step carries explicit precondition, postcondition, and falsifying check. Steps 4, 8, and 11 cannot start until the corresponding open question is resolved.
