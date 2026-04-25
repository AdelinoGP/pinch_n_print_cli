# Design: 31_support-planner-orca-algorithmic-parity

## Controlling Code Paths

- **Primary code paths:**
  - `wit/world-prepass.wit` — add `slice-preview-view-entry` + `slice-preview-view` records and extend `run-support-generation` parameters.
  - `crates/slicer-ir/src/slice_ir.rs` + `lib.rs` — add `SlicePreviewIR` keyed `(global_layer_index, object_id, region_id) → Vec<ExPolygon>` and re-export.
  - `crates/slicer-host/src/blackboard.rs` — `BlackboardPrepassSlot::SlicePreview`, `commit_slice_preview`, `slice_preview()` accessor.
  - `crates/slicer-host/src/prepass.rs` — add `PrepassStageOutput::SlicePreview(Arc<SlicePreviewIR>)`, `commit_stage_output` arm, extend `required_slots` for `PrePass::SupportGeneration`, and (depending on Q3) add a built-in computation in `execute_prepass_with_builtins`.
  - `crates/slicer-host/src/wit_host.rs` — `project_slice_preview_view` projector (deterministic ordering); extend prepass dispatcher to pass it to `run-support-generation`.
  - `crates/slicer-sdk/src/prepass_types.rs` + `prelude.rs` — `SlicePreviewView`, `SlicePreviewViewEntry`.
  - `crates/slicer-sdk/src/traits.rs` — extend `PrepassModule::run_support_generation` signature.
  - `crates/slicer-macros/src/lib.rs` — thread the new arg in the `PrePass::SupportGeneration` route.
  - `modules/core-modules/support-planner/support-planner.toml` — `[config.schema]` rewrite (4 dropped, 9 added) + `[ir-access].reads += "SlicePreviewIR"`.
  - `modules/core-modules/support-planner/src/lib.rs` — replace v1 propagation block with: avoidance/collision-aware move pass; `dist_to_top` tracking on each `PlannedSupportNode`; per-emit radius computation; raft prefix entries; interface densification; `tree_support_wall_count`-scaled `max_move_distance`. Drop the v1 limitations doc bullets.
  - `crates/slicer-helpers/src/geometry.rs` (or equivalent) — add `polygon_inflate(polygon, dist_mm) -> Vec<Polygon>` and `point_in_polygons(point, polygons) -> bool` if not already present.
- **Neighboring tests or fixtures:**
  - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` (packet 28) — must remain green.
  - `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` (packet 30) — must remain green.
  - `crates/slicer-host/tests/live_support_generation_tdd.rs` (packets 26 + 28 + 30) — must remain green.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (packets 21, 26, 27) — must remain green.
  - new file: `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs`.
  - new fixtures: `resources/golden/benchy_tree_support_orca_branch_count.txt`, `resources/golden/benchy_tree_support_orca_endpoints.txt`.
- **OrcaSlicer comparison surface:**
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` (lines 720–800, 1460–1700, 1913, 2625–2860).
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` (`SupportNode`, `TreeSupportData`).
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeModelVolumes.cpp` (avoidance inflation).

## Architecture Constraints

- **PrePass remains sequential, single-stage-at-a-time.** `PrePass::SlicePreview` is a new sequential stage. Stage ordering enforced by `ensure_stage_prerequisites`.
- **`SlicePreviewIR` is committed once.** Existing `OnceCell`-based blackboard pattern applies; duplicate commits return `LayerArenaError::AlreadyCommitted`.
- **Coordinate system invariant.** All polygon points in `SlicePreviewIR` are mm-space (`Point2`) at the host IR layer; the WIT projection passes them through unchanged. Radius and Z values follow the same mm convention.
- **Determinism.** `SlicePreviewIR.entries` is a `HashMap`; the projector sorts iteration by `(global_layer_index ASC, object_id ASC, region_id ASC)`. Inflation and union polygon construction use deterministic algorithms (Clipper polygon offset / Polygon::union). The packet 28 determinism test must continue passing.
- **Schema bumps.** Adding signed `global_layer_index` to `SupportPlanIR.entries` (if Q2 resolves toward signed indices) bumps `SupportPlanIR.schema_version` to `1.1.0` per `docs/02 §IR Versioning Contract`. The host's manifest validator must accept `min-ir-schema = "1.0.0"` consumers reading `1.1.0` data via the additive-field rule.
- **Diagnostic emission.** `support-planner.node-clamped-out` follows the structured diagnostic contract in `docs/09`. Code is namespaced with the module's id.

## Code Change Surface

### Selected approach

**Single-pass eager outline cache + node-by-node clamp + per-layer reset.**

`PrePass::SlicePreview` runs once per slicing job, computes 2D outlines for every region on every active layer via plane-triangle intersection, and commits `SlicePreviewIR`. The planner reads it through `slice-preview-view` at module-execution time. Inside `support-planner`:

1. **Avoidance + collision cache** is built lazily per layer. For layer `l`, take the union of `slice_preview[l].outlines` across all regions of the active object. Let `collision_polys = union(outlines)` (the "no-go inside the body" set). Let `avoidance_polys = collision_polys.inflate(branch_radius + safety)` (the "stay-out-but-include-margin" set). The inflation distance is `branch_radius + tree_support_branch_distance / 2` (Q4 resolution pending).
2. **Move pass** for each surviving node:
   - Compute the desired XY move vector (current `dx, dy` toward MST neighbor or contour edge).
   - Cap its magnitude at `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`.
   - If the resulting target lies outside `avoidance_polys`, project it back inward to the nearest avoidance contour point.
   - If the projected target lies inside `collision_polys` (i.e. inside the model body), drop the node and emit `support-planner.node-clamped-out`.
3. **Radius tapering.** Each node tracks `dist_to_top: u32` (incremented on every layer step). At emit time, `radius_mm = (branch_diameter / 2.0) + tan(diameter_angle_rad) * dist_to_top * effective_layer_height`, clamped to `[branch_diameter / 2, MAX_BRANCH_RADIUS = 6.0 mm]`. Every `Point3WithWidth.width = 2 * radius_mm`.
4. **Raft.** After the bottom layer (l=0) walk completes, if `support_raft_layers > 0`, prepend that many entries with `global_layer_index = -i` (signed widening) at Z values `z_bed - (i+1) * raft_layer_height_mm` (raft_layer_height = `effective_layer_height` of layer 0). Each raft entry's `branch_segments` carries dense full-cross-section fill computed by rectilinear scan-line at line spacing `tree_support_interface_spacing_mm`.
5. **Interface densification.** Track each branch column's first-touch and last-touch layer indices. For the top `support_interface_top_layers` and bottom `support_interface_bottom_layers` layers of each column, emit additional dense fill segments alongside the structural branch segments. Interface segments use the same `Point3WithWidth.width` as the structural branch but with closer line spacing.
6. **Wall-count move.** Embedded in (2). Replaces the v1 single-multiple `tan(angle) * h` step.

### Exact functions, traits, manifests, tests, or fixtures expected to change

**Created:**
- `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` — at least 9 tests (one per AC + the 4 negatives).
- `resources/golden/benchy_tree_support_orca_branch_count.txt` — single integer (the OrcaSlicer reference branch count for the Benchy + tree-support config).
- `resources/golden/benchy_tree_support_orca_endpoints.txt` — newline-delimited `x,y,z` coordinates of the OrcaSlicer reference branch endpoints.
- (If Q3 resolves toward a user module) `modules/core-modules/slice-preview/` — but default plan is built-in (Q3 resolution required).

**Modified — WIT, IR, SDK:**
- `wit/world-prepass.wit` — 2 new records, 1 new export parameter.
- `crates/slicer-ir/src/slice_ir.rs` + `lib.rs` — `SlicePreviewIR` (and possibly `SupportPlanIR` schema bump for signed indices).
- `crates/slicer-host/src/blackboard.rs` — `BlackboardPrepassSlot::SlicePreview`, slot, accessor, commit fn.
- `crates/slicer-host/src/prepass.rs` — `PrepassStageOutput::SlicePreview`, prerequisite extension, optional built-in computation.
- `crates/slicer-host/src/wit_host.rs` — projector + dispatcher wiring.
- `crates/slicer-sdk/src/prepass_types.rs` + `prelude.rs` — view types.
- `crates/slicer-sdk/src/traits.rs` — trait signature.
- `crates/slicer-macros/src/lib.rs` — macro arg routing.

**Modified — module:**
- `modules/core-modules/support-planner/support-planner.toml` — config schema rewrite + reads list.
- `modules/core-modules/support-planner/src/lib.rs` — comprehensive rewrite of `plan_for_object`'s propagation block; module-level v1 doc bullets removed; `MAX_BRANCH_RADIUS` constant added.
- `modules/core-modules/support-planner/wit-guest/src/lib.rs` — regenerate.

**Modified — backlog:**
- `docs/07_implementation_status.md` — `TASK-163` row.

### Rejected alternatives

- **Lazy `TreeSupportData`-style avoidance cache.** OrcaSlicer's class lazily computes avoidance per requested `(layer, radius)` pair and memoizes results across the propagation. Rejected for v2 because eager pre-compute fits the prepass-first-then-emit pipeline cleanly and trades CPU+memory for code simplicity. Revisit if profiling shows pre-compute is a bottleneck.
- **Recompute `SlicePreviewIR` inside `support-planner`.** Avoids the new prepass stage but duplicates plane-triangle intersection in every guest module that wants outlines. Rejected — adds compute and removes a clean reuse point for future packets.
- **Skip raft layers in v2.** The user request says "all v1 limitations" and raft is one of the seven; documenting it as out-of-scope would re-open a v3 packet later. Carry it.
- **Per-region branch independence.** Genuinely splitting branch sets across multiple regions on one layer requires propagation-time region attribution that we do not yet have. Defer to a follow-up packet.

## Data and Contract Notes

- **`SlicePreviewIR`:** `pub struct SlicePreviewIR { schema_version: SemVer { major: 1, minor: 0, patch: 0 }, entries: HashMap<RegionKey, Vec<ExPolygon>> }`. `RegionKey` already exists in `slicer-ir`. Multiple `ExPolygon`s per key represent disconnected outline pieces for the region on that layer.
- **WIT records:**
  - `record slice-preview-view-entry { global-layer-index: layer-idx, object-id: object-id, region-id: region-id, outlines: list<ex-polygon> }`
  - `record slice-preview-view { entries: list<slice-preview-view-entry> }`
- **Export signature:** `export run-support-generation: func(objects: list<mesh-object-view>, layer-plan: layer-plan-view, region-segmentation: region-segmentation-view, slice-preview: slice-preview-view, output: support-generation-output, config: config-view) -> result<_, module-error>;`
- **Prerequisite slice for `PrePass::SupportGeneration`:** `[SurfaceClassification, LayerPlan, RegionMap, SlicePreview]`.
- **`SupportPlanIR.global_layer_index`** widens to `i32` if Q2 resolves that way; otherwise the host adds a `raft_layers: u32` field to `SupportPlanIR` and the IR tree maps raft entries by `(raft_layer_index ∈ [0, raft_layers))`. Schema bump in either case.
- **Determinism:** the projector sorts. The avoidance build uses Clipper-style deterministic union (`Polygons::union_polygons`). MST and merge passes from packet 28 are unchanged.
- **Diagnostic shape:** `Diagnostic { level: Warn, code: "support-planner.node-clamped-out", message: format!("node ({:.3},{:.3}) clamped-out at layer {} after avoidance/collision check", x, y, layer), source: ModuleId("com.core.support-planner") }`.

## Locked Assumptions and Invariants

1. Packet 30's `LayerPlanView` and `RegionSegmentationView` shapes are stable. This packet adds a new view alongside; it does not change the existing two.
2. `LayerPlanIR.layers[i].effective_layer_height` is the authoritative per-layer `dz` for radius taper math and move math.
3. `tree_support_wall_count = 0` falls through to `max(1, n)` per OrcaSlicer line 2632.
4. `tree_support_branch_diameter` is the diameter (not radius) per OrcaSlicer; `branch_radius = tree_support_branch_diameter / 2`.
5. Raft layer height equals `effective_layer_height` of layer 0 (no separate raft layer height config in v2).
6. Dense-fill segments are produced via rectilinear scan-line, deterministic across runs.
7. `MAX_BRANCH_RADIUS = 6.0 mm` matches OrcaSlicer's hard upper clamp.
8. The host-built-in `PrePass::SlicePreview` (per Q3 resolution) does not require a guest module; `execute_prepass_with_builtins` handles it before calling user prepass modules.
9. Packet 26's grid-MST fallback in `tree-support` remains the path when `support-planner` is not loaded.
10. `support-planner.toml` config keys are validated by `slicer-host` at module-load time per `docs/03 §config schema validation`; out-of-range values fail load with a config error.

## Risks and Tradeoffs

- **Risk: per-layer outline cache balloons memory for large prints.** A 200-layer Benchy at 4 regions/layer with 50 polygon points per region holds ≈ 40k points. Acceptable. A pathological 1000-layer multi-object slice could push this to ~200k. Mitigation: monitor in packet 27 perf gates; if it's a problem, switch to lazy compute (the rejected alternative becomes the v3 path).
- **Risk: raft signed `global_layer_index` cascades through downstream consumers.** Every consumer of `SupportPlanIR.entries[*].global_layer_index` must accept the widened type. Mitigation: Q2's resolution explicitly chooses one path; if signed-widening is chosen, this packet's grep for downstream consumers is part of Step 11.
- **Risk: avoidance projection produces a target arbitrarily far from the desired direction.** The algorithm could oscillate. Mitigation: clamp the projected target to lie on the line segment between current node and original target; if that intersection is outside `avoidance_polys`, drop the node (and emit the diagnostic). Convergence within one layer is guaranteed because the move distance is bounded.
- **Risk: Benchy parity tolerance is too tight or too loose.** Q5's resolution decides; the packet stays draft until decided.
- **Risk: dropping `MAX_SAMPLES_PER_EXPOLY` (the v1 cap) and `support_max_branches_per_layer` removes a defense against pathological geometry.** Mitigation: `tree_support_wall_count`-scaled move + avoidance + collision already bound branch density per layer. If perf shows it's still needed, restore as a hidden hard cap.
- **Tradeoff: eager outline computation costs CPU during PrePass.** Trade against simpler code path and reuse for future packets. Acceptable.
- **Tradeoff: interface densification doubles the entry count for top/bottom layers of every column.** Acceptable; tree-support's emitter handles the count without issue.

## Open Questions

The following must be resolved before the packet activates:

- **Q1 (resolved):** `tree_support_branch_distance` — equal to OrcaSlicer's `tree_support_branch_distance` config key; used as the merge-decision distance threshold (replaces v1 `support_branch_merge_distance_mm`). Default `1.0 mm`.
- **Q2 (open):** Raft Z convention. Two options:
  - (a) Widen `SupportPlanIR.entries[*].global_layer_index` to `i32`; raft layers carry negative indices; `SupportPlanIR.schema_version` bumps to `1.1.0`.
  - (b) Add `pub raft_layers: Vec<RaftLayer>` field to `SupportPlanIR` keyed by `raft_layer_index ∈ [0, support_raft_layers)`; `global_layer_index` stays `u32`; `SupportPlanIR.schema_version` bumps to `1.1.0`.
  - **Decision required before Step 11.**
- **Q3 (open):** Built-in vs user module for `PrePass::SlicePreview`.
  - (a) Built-in: `execute_prepass_with_builtins` computes `SlicePreviewIR` from `MeshIR` + `LayerPlanIR` + `RegionMapIR` directly. No guest module required. Cleaner but binds slicing logic to the host.
  - (b) User module: ship a `slice-preview` core module under `modules/core-modules/`. More modular but requires a new guest crate.
  - **Decision required before Step 4.**
- **Q4 (open):** Avoidance polygon inflation amount.
  - (a) `branch_radius + 0.4 mm` fixed safety.
  - (b) `branch_radius + tree_support_branch_distance / 2` (config-driven).
  - **Decision required before Step 8.**
- **Q5 (open):** Numerical tolerance for the Benchy parity check.
  - (a) Branch-count match within ±10% (current AC default).
  - (b) Endpoint Hausdorff distance ≤ 0.5 mm (current AC default).
  - (c) Both must hold.
  - **Decision required before Step 14.**
