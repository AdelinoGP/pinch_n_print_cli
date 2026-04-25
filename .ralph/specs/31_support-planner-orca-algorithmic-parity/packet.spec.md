---
status: draft
packet: 31_support-planner-orca-algorithmic-parity
task_ids:
  - TASK-163
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 31_support-planner-orca-algorithmic-parity

## Goal

Close the five algorithmic v1 limitations of `support-planner` left after packet `30_support-planner-prepass-wit-plumbing` plumbs `LayerPlanIR.layers` and `RegionMapIR.entries` through the prepass WIT: (3) introduce a simplified `TreeSupportData`-style avoidance/collision cache built from per-layer slice polygons; (4) implement per-node radius tapering along `tan(tree_support_branch_diameter_angle) * dist_to_top` so wider branches form near the build plate; (5) emit raft prefix layers below `z=0` per `support_raft_layers` and interface-layer densification per `support_interface_top_layers` / `support_interface_bottom_layers`; (6) scale the per-layer move step by `tree_support_wall_count` (the OrcaSlicer wall-count-aware `max_move_distance`); (7) wire the four OrcaSlicer config keys (`tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_distance`, `tree_support_branch_diameter_angle`) into the planner's `[config.schema]` and renaming the existing v1 keys to align. After this packet, `support-planner` produces output matching OrcaSlicer's `TreeSupport::drop_nodes` for the Benchy and synthetic single-object overhang fixtures within the documented numerical tolerance.

## Scope Boundaries

- **In scope:**
  - **Per-layer slice geometry plumbing.** Extend the prepass WIT (`wit/world-prepass.wit`) and SDK (`crates/slicer-sdk/src/prepass_types.rs`) with a `slice-preview-view` projecting one `ExPolygon` list per `(global_layer_index, object_id, region_id)` so the planner can run avoidance against actual layer outlines. Host projection lives in `crates/slicer-host/src/wit_host.rs::project_slice_preview_view`. Add `BlackboardPrepassSlot::SlicePreview` and a new `commit_slice_preview` slot. A new built-in PrePass stage `PrePass::SlicePreview` runs before `PrePass::SupportGeneration` and computes per-layer 2D outlines via plane-triangle intersection of every active region's mesh footprint at each layer's Z.
  - **Avoidance + collision cache** in `modules/core-modules/support-planner/src/lib.rs`: per layer, compute `avoidance_polys = union(slice_preview[l].outlines).inflate(branch_radius + safety)` and `collision_polys = union(slice_preview[l].outlines)`. The propagation move-pass clamps each node into `avoidance_polys` and rejects move vectors whose target lies outside `collision_polys`.
  - **Radius tapering.** Each `PlannedSupportNode` carries a `dist_to_top: u32` (layer count from the top contact). Per-layer node radius is `clamp(branch_diameter / 2 + tan(diameter_angle) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`. Radius is propagated into emitted `Point3WithWidth.width = 2 * radius`.
  - **Raft prefix layers.** When `support_raft_layers > 0`, prepend that many `SupportPlanEntry` rows with `global_layer_index ∈ [-raft_layers, -1]` (signed widening of `global_layer_index` to `i32` in the IR — see Open Question Q4 if this becomes a blocker) carrying full-cross-section dense-fill branch segments. Z values are `z_bed - (i+1) * raft_layer_height_mm`.
  - **Interface-layer densification.** For the top `support_interface_top_layers` and bottom `support_interface_bottom_layers` layers of each branch column, emit dense interface fill (line spacing = `tree_support_interface_spacing_mm`, default `0.4`) in addition to the structural branch segments.
  - **Wall-count-aware move scaling.** Use `tree_support_wall_count.max(1)` as the multiplier on `tan(branch_angle) * effective_layer_height` to compute `max_move_distance` per layer.
  - **Config keys.** Add `tree_support_branch_angle` (deg, replaces `support_branch_angle_deg`), `tree_support_branch_diameter` (mm), `tree_support_branch_diameter_angle` (deg), `tree_support_branch_distance` (mm, replaces `support_branch_merge_distance_mm`), `tree_support_wall_count` (int), `support_raft_layers` (int), `support_interface_top_layers` (int), `support_interface_bottom_layers` (int, `-1` falls back to `top_layers`), and `tree_support_interface_spacing_mm` (mm) to `support-planner.toml [config.schema]`. Drop the old `support_branch_angle_deg`, `support_branch_merge_distance_mm`, `support_max_branches_per_layer`, and `line_width` keys (replaced by branch-radius-derived widths).
  - **Manifest reads.** Add `"SlicePreviewIR"` to `support-planner.toml [ir-access].reads`.
  - **Tests.** New file `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` with one positive AC per limit + a numerical parity check against a Benchy-derived golden fixture.
  - **Backlog.** Add `TASK-163` to `docs/07_implementation_status.md`.

- **Out of scope:**
  - Replacing `MinimumSpanningTree::prim` with a heap-based variant — pre-existing complexity ceiling carries forward.
  - Soluble multi-extruder interface support material.
  - Catchup / variable-per-region effective layer-height interactions inside one object on one layer.
  - GUI / global-config plumbing for the new keys outside the module's `[config.schema]`.
  - Geometry-aware multi-region branch separation when an object owns several regions on one layer that do not overlap geometrically — this packet still emits the same branch set for each region, then per-region clamps use that region's slice outlines for collision. True per-region branch independence is deferred to a follow-up packet.
  - Tree-support emitter changes beyond honoring `Point3WithWidth.width` (already supported).
  - Changes to `Layer::Support` claim layout or scheduling.

## Prerequisites and Blockers

- **Depends on:**
  - Packet `30_support-planner-prepass-wit-plumbing` must be `status: implemented`. Its `LayerPlanView` and `RegionSegmentationView` are the structural template for `SlicePreviewView`.
  - Packet `28_tree-support-multi-layer-propagation` (already `status: implemented`).
- **Unblocks:** Phase H tree-support visual-parity tickets (TASK-120 acceptance evidence with non-grid branches against Benchy; tracked under TASK-120 in `docs/07_implementation_status.md`).
- **Activation blockers (must be resolved before flipping to `active`):**
  - **Q1 (resolved):** `tree_support_branch_distance` semantic — confirmed equal to OrcaSlicer's branch-spacing parameter, used as the merge distance.
  - **Q2 (open):** Raft Z convention — does `SupportPlanIR.global_layer_index` widen to `i32` to allow negative raft indices, or does the host introduce a separate `raft_layers` field on `SupportPlanIR`? Resolution required before Step 11 starts.
  - **Q3 (open):** Should `PrePass::SlicePreview` be a built-in host stage (computed without a guest module, like `MeshAnalysis`) or a user-supplied prepass module? A built-in is faster but binds layer-slicing logic into `slicer-host`; a user module preserves modularity. Resolution required before Step 4 starts.
  - **Q4 (open):** Avoidance polygon inflation amount — fixed `branch_radius + 0.4 mm` safety, or config-driven (`tree_support_branch_distance / 2`)? Resolution required before Step 8 starts.
  - **Q5 (open):** Numerical tolerance for the OrcaSlicer parity check — branch-count exact match within ±1 per layer, or coordinate Hausdorff distance ≤ 0.5 mm? Resolution required before Step 14 starts.
  - `TASK-163` row added to `docs/07_implementation_status.md`.

## Acceptance Criteria

- **Given** `wit/world-prepass.wit`, **when** read, **then** it declares `record slice-preview-view-entry { global-layer-index: layer-idx, object-id: object-id, region-id: region-id, outlines: list<ex-polygon> }` and `record slice-preview-view { entries: list<slice-preview-view-entry> }`, and `export run-support-generation` carries `slice-preview: slice-preview-view` between `region-segmentation` and `output`. | `grep -nE 'record slice-preview-view-entry|record slice-preview-view\b|slice-preview: slice-preview-view' wit/world-prepass.wit`
- **Given** `crates/slicer-ir/src/slice_ir.rs`, **when** read, **then** it defines `pub struct SlicePreviewIR { pub schema_version: SemVer, pub entries: HashMap<RegionKey, Vec<ExPolygon>> }` and `crates/slicer-ir/src/lib.rs` re-exports it. | `grep -nE 'pub struct SlicePreviewIR' crates/slicer-ir/src/slice_ir.rs && grep -nE 'SlicePreviewIR' crates/slicer-ir/src/lib.rs`
- **Given** `crates/slicer-host/src/prepass.rs::required_slots`, **when** queried with `"PrePass::SupportGeneration"`, **then** the returned slice equals `&[SurfaceClassification, LayerPlan, RegionMap, SlicePreview]` in that order. | `grep -nA5 '"PrePass::SupportGeneration"' crates/slicer-host/src/prepass.rs | head -8`
- **Given** `modules/core-modules/support-planner/support-planner.toml`, **when** read, **then** `[config.schema]` defines `tree_support_branch_angle` (float, default 45.0), `tree_support_branch_diameter` (float, default 5.0), `tree_support_branch_diameter_angle` (float, default 5.0), `tree_support_branch_distance` (float, default 1.0), `tree_support_wall_count` (int, default 1), `support_raft_layers` (int, default 0), `support_interface_top_layers` (int, default 2), `support_interface_bottom_layers` (int, default -1), `tree_support_interface_spacing_mm` (float, default 0.4); and the v1 keys `support_branch_angle_deg`, `support_branch_merge_distance_mm`, `support_max_branches_per_layer`, `line_width` are absent. | `python3 -c "import tomllib; d=tomllib.loads(open('modules/core-modules/support-planner/support-planner.toml','rb').read().decode()); s=d['config']['schema']; req={'tree_support_branch_angle':45.0,'tree_support_branch_diameter':5.0,'tree_support_branch_diameter_angle':5.0,'tree_support_branch_distance':1.0,'tree_support_wall_count':1,'support_raft_layers':0,'support_interface_top_layers':2,'support_interface_bottom_layers':-1,'tree_support_interface_spacing_mm':0.4}; missing=[k for k,v in req.items() if k not in s or s[k]['default']!=v]; gone=[k for k in ('support_branch_angle_deg','support_branch_merge_distance_mm','support_max_branches_per_layer','line_width') if k in s]; assert not missing and not gone, f'MISSING={missing} EXTRA={gone}'"`
- **Given** a single-object fixture with one tall overhang and `tree_support_branch_diameter = 5.0`, `tree_support_branch_diameter_angle = 5.0`, **when** the planner runs through `execute_prepass_with_builtins`, **then** the topmost `SupportPlanEntry.branch_segments[*][*].width` value equals `5.0` mm (within 1e-3) and the bottom-most entry's width is greater than `5.0 + tan(5° rad) * (top_layer_z - bottom_layer_z)` (within 1e-3 mm), proving radius tapers linearly with `dist_to_top * effective_layer_height`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd radius_tapers_with_distance_to_top -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** an overhang fixture whose underlying body has a hole at layer 5, **when** the planner runs with `SlicePreviewIR` carrying that hole's outline, **then** every `SupportPlanEntry.branch_segments[layer=5]` endpoint lies inside the inflated outer contour and outside any hole's contour (point-in-polygon check using `slicer_helpers::geometry::point_in_polygon`). | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd avoidance_keeps_branches_inside_layer_outline -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support_raft_layers = 3` and `support_interface_top_layers = 2`, **when** the planner runs against a fixture with one overhang column on layers 8–10, **then** the committed `SupportPlanIR.entries` contains exactly 3 entries with negative `global_layer_index` (raft) plus interface-densified entries on layers 8 and 9 whose `branch_segments` count equals the structural-branch count plus the count of dense-fill segments produced for `tree_support_interface_spacing_mm = 0.4`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd raft_and_interface_layers_emit_expected_entry_count -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `tree_support_wall_count = 3`, **when** the planner propagates a single node from layer 10 to layer 0 with `tree_support_branch_angle = 45°` and `effective_layer_height = 0.2 mm`, **then** the maximum XY-distance traversed by the node between any two adjacent layers is `≤ tan(45°) * 0.2 * 3 = 0.6 mm` (within 1e-4 mm). | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd wall_count_scales_max_move_distance -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** the Benchy parity fixture (`resources/test_models/benchy.stl`) sliced under `resources/test_config/benchy-tree-support.json` with `support-planner` loaded, **when** the planner runs, **then** the resulting `SupportPlanIR.entries.len()` is within ±10% of the OrcaSlicer-produced reference branch count for the same model and config (golden file: `resources/golden/benchy_tree_support_orca_branch_count.txt`), and the branch-endpoint Hausdorff distance (computed by `slicer_helpers::geometry::hausdorff_distance` against `resources/golden/benchy_tree_support_orca_endpoints.txt`) is ≤ 0.5 mm. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd benchy_orca_parity_within_tolerance -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** every prepass `.wasm`, **when** rebuilt, **then** the `support-planner.wasm` build cascade succeeds and `--check` reports the binary up to date. | `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -10 && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'support-planner.*up to date'`
- **Given** `docs/07_implementation_status.md`, **when** read, **then** it contains exactly one row matching `^- \[.\] TASK-163 ` whose body references this packet's slug. | `grep -nE '^- \[.\] TASK-163 .*31_support-planner-orca-algorithmic-parity' docs/07_implementation_status.md`

## Negative Test Cases

- **Given** an `ExecutionPlan` whose `prepass_stages` schedules `PrePass::SupportGeneration` before `PrePass::SlicePreview` has committed a `SlicePreviewIR`, **when** `execute_prepass` runs, **then** it returns `PrepassExecutionError::MissingRequiredPrepass { stage_id: "PrePass::SupportGeneration", slot: BlackboardPrepassSlot::SlicePreview }`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd prepass_support_generation_fails_without_slice_preview -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `tree_support_branch_diameter_angle = 80.0` (above the OrcaSlicer-documented bound of `0..=90 - epsilon`), **when** `support-planner` is loaded, **then** module load returns a config-validation error whose message contains the literal substring `"tree_support_branch_diameter_angle out of range"`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd diameter_angle_out_of_range_rejects_load -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support_raft_layers = -1`, **when** `support-planner` is loaded, **then** module load returns a config-validation error whose message contains the literal substring `"support_raft_layers must be >= 0"`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd negative_raft_layers_rejects_load -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** a propagation step where the avoidance + collision cache rejects the desired move vector for every direction, **when** `drop_nodes` reaches that node, **then** the node is dropped (not propagated to the next layer) and a `DiagnosticLevel::Warn` diagnostic is emitted whose `code == "support-planner.node-clamped-out"`. | `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd node_dropped_when_avoidance_rejects_all_moves -- --test-threads=1 --nocapture 2>&1 | tail -20`

## Verification

- `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture` (regression — packet 28's tests still green)
- `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture` (regression — packet 30's tests still green)
- `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 --nocapture` (this packet)
- `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture` (regression — tree-support live path still green)
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture` (regression — packet 21/26 Benchy supports still green)
- `cargo test -p support-planner --lib`
- `bash modules/core-modules/build-core-modules.sh`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — §Pipeline tiers, §Stage I/O Contract for `PrePass::SupportGeneration` and the new `PrePass::SlicePreview`.
- `docs/02_ir_schemas.md` — `SlicePreviewIR` (new), `SupportPlanIR.entries` (signed `global_layer_index` if Q2 resolves that way), `IR Versioning Contract` (this packet bumps `SupportPlanIR` schema if raft uses signed indices).
- `docs/03_wit_and_manifest.md` — §prepass world (new `slice-preview-view`), §host-boundary enforcement, §additive WIT change rebuild rule, §config schema validation.
- `docs/04_host_scheduler.md` — §PrePass Execution, `ensure_stage_prerequisites`, built-in vs user prepass stages.
- `docs/05_module_sdk.md` — config schema bounds enforcement, prepass module authoring.
- `docs/08_coordinate_system.md` — mm-vs-units convention for radius, raft Z values, interface line spacing.
- `docs/09_progress_events.md` — `support-planner.node-clamped-out` diagnostic emission.
- `.ralph/specs/30_support-planner-prepass-wit-plumbing/` — structural precedent for the new WIT view; this packet copies its projector pattern.
- `.ralph/specs/28_tree-support-multi-layer-propagation/` — original simplified port whose v1 limitations this packet closes.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`:
  - Lines 720–800: `TreeSupport::generate_contact_points` and `tree_support_branch_diameter_angle` handling — radius tapering (`diameter_angle_scale_factor`).
  - Lines 1460–1700: avoidance/collision computations during interface generation.
  - Line 2625 onward (`drop_nodes`): authoritative shape for the propagation loop, including `max_move_distance = tan(angle) * layer_height * wall_count` (line 2634).
  - Line 1913: `support_interface_top_layers` interface-layer densification entry point.
  - `m_raft_layers` references throughout: raft prefix Z convention.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp`:
  - `SupportNode` struct: fields we mirror in `PlannedSupportNode` (`dist_mm_to_top`, `radius`, `parent`, etc.).
  - `TreeSupportData` class declaration: shape of the avoidance/collision cache (we ship a single-pass simplification, not the full lazy cache).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeModelVolumes.cpp`: avoidance polygon inflation logic.
- `OrcaSlicerDocumented/src/libslic3r/MinimumSpanningTree.cpp::prim`: same O(V²) Prim variant we already use; unchanged.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
