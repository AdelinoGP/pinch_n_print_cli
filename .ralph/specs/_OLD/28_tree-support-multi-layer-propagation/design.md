# Design: 28_tree-support-multi-layer-propagation

## Controlling Code Paths

1. **`wit/world-prepass.wit`** — canonical WIT for the prepass world. A new stage is added alongside `run-mesh-segmentation`, `run-mesh-analysis`, `run-layer-planning`, `run-paint-segmentation`, `run-seam-planning`. Shape mirrors `run-seam-planning` (mesh-object-view input + push-based output resource + per-(layer, object, region) record).
2. **`crates/slicer-ir/src/slice_ir.rs`** — `SupportPlanIR` + `SupportPlanEntry` definitions. `SemanticRegion`, `SeamPlanIR`, and `SeamPlanEntry` are the immediate structural precedent.
3. **`crates/slicer-ir/src/lib.rs`** — re-export `SupportPlanIR` and `SupportPlanEntry` from the crate root (alphabetical insertion in the existing `pub use` block).
4. **`crates/slicer-host/src/prepass.rs`** — extend `PrepassStageOutput` enum with a `SupportPlan(Arc<SupportPlanIR>)` variant; extend `ir_path_for_prepass_output`; extend `commit_stage_output` to call a new `commit_support_plan`; extend `ensure_stage_prerequisites` to gate `"PrePass::SupportGeometry"` on `SurfaceClassification` + `LayerPlan`.
5. **`crates/slicer-host/src/blackboard.rs`** — add `support_plan: OnceCell<Arc<SupportPlanIR>>` slot, `commit_support_plan`, `support_plan` accessor; add `BlackboardPrepassSlot::SupportPlan` enum variant and include it in any exhaustive match sites (there are a small number — search `BlackboardPrepassSlot::`).
6. **`crates/slicer-host/src/dispatch.rs`** — runtime must route `Layer::Support` read requests for `SupportPlanIR` to the committed blackboard slot. The existing dispatch bridge for `SeamPlanIR` is the precedent (`push_perimeter_regions` reads `seam_plan_ir` similarly).
7. **`crates/slicer-host/src/wit_host.rs` / prepass runtime dispatcher** — extend `WasmRuntimeDispatcher` (or wherever prepass dispatch lives; see `execute_prepass`'s `runner.run_stage`) to implement the `run-support-geometry` export: feed `list<MeshObjectView>` + the output resource, collect pushed entries into `SupportPlanIR`, return as `PrepassStageOutput::SupportPlan`.
8. **`modules/core-modules/support-planner/`** — new crate. Layout mirrors `seam-planner-default/`:
   - `Cargo.toml` (package `support-planner`)
   - `support-planner.toml` (manifest)
   - `src/lib.rs` (`#[slicer_module] impl PrepassModule for SupportPlanner { fn run_support_geometry(...) }`)
   - `wit-guest/` (standalone cdylib workspace; re-exports `SupportPlanner` via `#[cfg(target_arch = "wasm32")]`)
9. **`modules/core-modules/tree-support/tree-support.toml`** — add `"SupportPlanIR"` to `[ir-access].reads`.
10. **`modules/core-modules/tree-support/src/lib.rs`** — extend `run_support` to consult the committed `SupportPlanIR` (via `LayerView` or equivalent SDK accessor) and emit branches from it when an entry exists for `(layer_index, object_id, region_id)`. Fall through to `fill_expolygon_tree` (grid-MST) otherwise.
11. **`modules/core-modules/traditional-support/src/lib.rs`** — module-level doc comment only. Manifest `reads` unchanged.
12. **`modules/core-modules/build-core-modules.sh`** — append `"support-planner:support_planner_guest"` to `MODULES` array (alphabetical order preserved — insert between `support-surface-ironing` and `traditional-support`).
13. **Tests:**
    - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` (new file) — hosts all prepass-stage level tests including the negative cases.
    - `crates/slicer-host/tests/live_support_generation_tdd.rs` (existing; extend Section C) — adds the planner-consuming tier of Layer::Support dispatch tests.

## Architecture Constraints

- **PrePass is sequential, per-stage; per-layer tier is rayon-parallel.** `PrePass::SupportGeometry` runs once, globally, before any per-layer work. `Layer::Support` runs per-layer in rayon. Propagation across layers must happen in the PrePass stage — a per-layer reader can never propagate because it has no cross-layer view guaranteed-consistent under parallelism.
- **Prerequisite chain in `ensure_stage_prerequisites` is a single source of truth.** The new stage must declare its deps there and nowhere else; do not add bespoke checks inside the support-planner module.
- **Blackboard slots are write-once.** The `support_plan` slot uses `OnceCell` like the other prepass slots; duplicate commits must error via the existing `LayerArenaError`-style pattern (`commit_seam_plan` is the precedent).
- **Claim uniqueness.** `support-planner` is a new claim, orthogonal to `support-generator`. Two modules holding `support-planner` in the same stage get first-winner alphabetical dedup with an Info diagnostic (as per `dedup_same_claim_modules` in `execution_plan.rs`).
- **WIT contract changes trigger a rebuild cascade.** Any change to `world-prepass.wit` means rebuilding every prepass wasm. The CLAUDE.md §"WIT/Type Changes Checklist" enumerates the search surface.
- **The SDK trait for prepass modules must support the new stage.** Inspect `PrepassModule` in `crates/slicer-sdk/src/traits.rs`; add `fn run_support_geometry(...)` with a default-unimplemented body so existing prepass modules continue to compile.
- **Coordinate system invariant.** All IR coordinates use `1 unit = 100 nm` (docs/08). `SupportPlanEntry.branch_segments` uses `ExtrusionPath3D` which carries mm-valued `Point3WithWidth` — match the existing support path convention so `tree-support`'s emitter can pass segments through without re-scaling.

## Implementation Approach (selected)

**Single-layer-at-a-time propagation, per-object, sequential, no avoidance cache.**

The planner walks the model top-to-bottom over the object's mesh bounds at the WIT-exposed `MeshObjectView` granularity. v1 is **layer-height-agnostic** (uniform 0.2 mm assumed); `LayerPlanIR` is a host-side scheduling prerequisite via `ensure_stage_prerequisites` but is not read at runtime, and is therefore not declared in the planner's manifest `[ir-access].reads`. A follow-up packet that surfaces `LayerPlanIR.layers` and `SurfaceClassificationIR.object_annotations` through the prepass WIT will let the planner consume them and add them back to the manifest reads. For each object:

1. **Contact-point extraction** (mirrors `detect_overhangs`):
   - For every layer `l` from top to bottom:
     - For every facet classified as `FacetClass::Overhang` or `FacetClass::Bridge` whose footprint intersects layer `l`, emit a contact point at the facet centroid projected to layer `l`'s Z.
     - For every `SupportEnforcer` region polygon in `PaintRegionIR.per_layer[l]`, emit a contact point at the polygon centroid.
     - Skip layers whose regions all have `needs_support = false` AND no enforcer paint (and drop points that fall inside `SupportBlocker` regions).
   - Store per-layer contact points as `Vec<PlannedSupportNode>` with `position: Point2`, `parent: Option<usize>`, `origin_layer: u32`.

2. **Top-down propagation** (mirrors `drop_nodes`'s merge-and-move shape without avoidance):
   - For layer `l` from top to bottom:
     - Take the set of active nodes at `l` (newly-added contacts plus nodes propagated from `l+1`).
     - Group by "part" using a single `ExPolygon` hit-test against `SliceIR.regions[*].polygons` on that layer — OrcaSlicer's per-part grouping.
     - For each group, run Prim MST over node positions (`MinimumSpanningTree::prim` structural analog).
     - Pass 1 — merge: for each node with an MST neighbor closer than `merge_distance_mm` (config-driven; default `0.8`), merge into the neighbor (record parent, drop duplicate).
     - Pass 2 — move: for each surviving node, move by `tan(angle) * layer_height` mm toward its MST neighbor or toward the nearest contour edge (no avoidance in v1; a node that would step outside its region is clamped to the region boundary).
     - Record the resulting per-layer branch segments (one segment per edge in the moved MST) into `SupportPlanEntry { global_layer_index: l, object_id, region_id, branch_segments }`.
   - Stop propagating a node when it reaches the build plate (`l == 0`) or its move would leave the active region's bounding slab.

3. **Config keys for v1:**
   - `support_enabled: bool` (default `true`)
   - `support_branch_angle_deg: float` (default `45.0`, bounds `0.0..=75.0`)
   - `support_branch_merge_distance_mm: float` (default `0.8`, bounds `0.1..=5.0`)
   - `support_max_branches_per_layer: int` (default `1024`, bounds `1..=10000`) — hard cap for defense-in-depth, analogous to `MAX_SAMPLES_PER_EXPOLY` in the grid filler.
   - `line_width: float` (default `0.4`) — propagated into `ExtrusionPath3D.points[*].width`.

4. **Determinism:** node order is stable because grouping is keyed on `(object_id, region_id, sorted(node_position))`; MST input is a deterministic Vec; Prim's `min_element` tie-breaks by insertion order (use `(distance, index)` tuple keying as in `SeamPlanIR`'s scorer for symmetry).

### Rejected alternatives

- **Full OrcaSlicer port.** ~3.7k C++ lines plus the avoidance/collision cache infrastructure. Not justifiable in one packet.
- **Push propagation into `Layer::Support` with a shared `Mutex<BranchState>`.** Violates the "per-layer tier is rayon-parallel, stage-internal state is forbidden" invariant in `docs/04`.
- **Merge `support-planner` functionality into the existing `tree-support` crate with dual stages.** Per user instruction: the package is its own crate, `support-planner`. Keeps the `Layer::Support` alphabetical dedup behavior predictable (only `tree-support` and `traditional-support` hold `support-generator` on `Layer::Support`; the new claim `support-planner` lives on `PrePass::SupportGeometry` alone).

## Explicit Code Change Surface

### Files created
- `modules/core-modules/support-planner/Cargo.toml`
- `modules/core-modules/support-planner/support-planner.toml`
- `modules/core-modules/support-planner/src/lib.rs`
- `modules/core-modules/support-planner/wit-guest/Cargo.toml`
- `modules/core-modules/support-planner/wit-guest/src/lib.rs`
- `crates/slicer-host/tests/prepass_support_generation_tdd.rs`

### Files modified
- `wit/world-prepass.wit` — add `run-support-geometry` export + `support-geometry-output` resource + `support-plan-entry` record.
- `crates/slicer-ir/src/slice_ir.rs` — add `SupportPlanIR`, `SupportPlanEntry`.
- `crates/slicer-ir/src/lib.rs` — re-export `SupportPlanIR`, `SupportPlanEntry`.
- `crates/slicer-host/src/prepass.rs` — `PrepassStageOutput::SupportPlan`, `ir_path_for_prepass_output`, `commit_stage_output`, `ensure_stage_prerequisites`.
- `crates/slicer-host/src/blackboard.rs` — `support_plan` slot, `commit_support_plan`, `support_plan`, `BlackboardPrepassSlot::SupportPlan`.
- `crates/slicer-host/src/dispatch.rs` (and/or `wit_host.rs` / `layer_executor.rs`) — route `Layer::Support` reader of `SupportPlanIR` to the committed slot.
- `crates/slicer-host/src/wit_host.rs` — prepass dispatcher implements the new `run-support-geometry` call path (mirrors `run-seam-planning`).
- `crates/slicer-sdk/src/traits.rs` — add `fn run_support_geometry(...)` to `PrepassModule` (default `Err(ModuleError::unimplemented(...))`).
- `crates/slicer-sdk/src/builders.rs` — add `SupportPlanOutputBuilder` if that's the pattern `seam-planner-default` uses; otherwise reuse existing builder plumbing.
- `modules/core-modules/tree-support/tree-support.toml` — `reads` += `"SupportPlanIR"`.
- `modules/core-modules/tree-support/src/lib.rs` — `run_support` consults `SupportPlanIR` via a new SDK accessor on `PaintRegionLayerView` / `LayerSupportView` (or the equivalent layer-view trait); fall back to `fill_expolygon_tree`.
- `modules/core-modules/traditional-support/src/lib.rs` — module-level doc comment only.
- `modules/core-modules/build-core-modules.sh` — `MODULES` array entry.
- `crates/slicer-host/tests/live_support_generation_tdd.rs` — add Section C + three new tests.

### Artifacts rebuilt
- `modules/core-modules/support-planner/support-planner.wasm` (new)
- `modules/core-modules/tree-support/tree-support.wasm` (rebuild after manifest change)

## Data and Contract Notes

- `SupportPlanIR.entries` keying: `(global_layer_index, object_id, region_id)`. Multiple entries may exist for the same `(layer, object)` across different `region_id`s — match `SeamPlanIR.entries`'s multiplicity contract.
- **v1 single-region limitation:** `MeshObjectView` does not currently surface per-region segmentation, so the v1 planner emits every entry under the canonical `region_id = 0` bucket. Single-region objects (the Benchy fixture and the live-dispatch test geometries) match correctly because `tree-support`'s `support_plan_segments_for(object_id, region_id)` is invoked with `region_id = 0` for those regions. Multi-region objects will collapse all branches into the first region until a follow-up packet plumbs region info through the prepass WIT.
- `SupportPlanEntry.branch_segments: Vec<ExtrusionPath3D>` — each segment is a polyline (typically 2-point but may be multi-point for long branches). Points use `ExtrusionRole::SupportMaterial` and `speed_factor` computed from the planner's `support_speed` config key (identical to existing tree-support formula: `support_speed / BASE_SPEED`).
- `tree-support`'s `run_support` must preserve deterministic ordering: iterate `SupportPlanIR.entries` in input order, not via HashMap iteration, and emit `ExtrusionPath3D` in that order.
- `traditional-support` does not read `SupportPlanIR`. If this invariant is violated (e.g. a future refactor adds the read), the contract audit in `core_module_ir_access_contract_tdd.rs` must fail. Add an assertion there if one does not already cover support modules.

## Risks and Tradeoffs

- **Risk:** Simplified propagation without avoidance produces branches that pass through model walls on complex geometries (overhangs that hang over solid body below). *Mitigation:* clamp each propagated node to the active region's bounding contour on each move; accept the resulting "branches that hug wall surfaces" as a v1 limitation documented in `requirements.md` out-of-scope. A follow-up packet adds `TreeSupportData` avoidance cache.
- **Risk:** `PrePass::SupportGeometry` adds a new hard dependency on `SurfaceClassificationIR`, which is host-built-in today. If a test harness ever disables the built-in mesh analysis, the planner silently becomes a no-op. *Mitigation:* the negative AC on missing `LayerPlanIR` covers the equivalent failure for layer planning; add a parallel test on missing surface classification if the built-in execution ever becomes optional.
- **Risk:** Two support modules sharing `support-planner` claim drop silently via alphabetical dedup. *Mitigation:* covered by AC-dedup negative test; the Info diagnostic is surfaced on stderr by `main.rs` so operators see it.
- **Tradeoff:** Per-layer MST is still O(V²) Prim. OrcaSlicer carries the same complexity. For very dense contact sets this still spikes, but V is bounded by `support_max_branches_per_layer` (default 1024), so worst-case MST work is ≤ ~10⁶ ops per layer per part — well under a pathological grid-MST case.
- **Tradeoff:** Introducing a new core module grows the module set and therefore the cold-start module-load time. Acceptable; the prepass is gated on `support-planner` being installed and does nothing when it's absent.

## Open Questions

All packet-scope open questions are resolved:

- **Q1: Module name.** *Resolved —* `support-planner` (per user instruction).
- **Q2: `SupportPlanIR` shape.** *Resolved —* mirror `SeamPlanIR` (per-(layer, object, region) entries; list of `ExtrusionPath3D` segments).
- **Q3: Claim layout.** *Resolved —* new claim `support-planner` on `PrePass::SupportGeometry`. `support-generator` on `Layer::Support` is unchanged.
- **Q4: IR version bumps.** *Resolved —* none. `SupportIR` unchanged; new `SupportPlanIR` starts at `1.0.0`.
- **Q5: `task_ids` mapping.** *Resolved —* new row `TASK-161` to be added to `docs/07_implementation_status.md` before activation (draft line provided in `requirements.md`).

Remaining implementation-time decisions (tracked as step exit conditions in `implementation-plan.md`):

- Exact SDK surface used by `tree-support` to reach the committed `SupportPlanIR` from `run_support` (probably a new accessor on the existing layer view — to be picked during Step 6 discovery; no packet-scope ambiguity because the contract is "read the blackboard slot," only the SDK name is open).
- Exact `ModuleError` variant for "planner ran but had no `LayerPlanIR`" — defaults to `PrepassExecutionError::StagePrerequisite`; module-side error is not invoked because the prerequisite check short-circuits before dispatch.

## Locked Assumptions

1. `PrePass::SeamPlanning` (TASK-159, packet `23-rev1`) has already landed — its WIT record, `SeamPlanIR`, and the `run-seam-planning` dispatch path are the structural reference for this packet. Confirmed closed in `docs/07` line 97.
2. `SurfaceClassificationIR` is host-built-in via `execute_mesh_analysis` and always committed before any user PrePass stage begins (`execute_prepass_with_builtins`).
3. `LayerPlanIR` is committed by `PrePass::LayerPlanning` before any stage that declares it as a prerequisite.
4. `ExtrusionPath3D` and `Point3WithWidth` already carry `width`, `flow_factor`, `speed_factor`, and `role` fields — no extension needed.
5. The `#[slicer_module]` macro already handles `PrepassModule` with stage routing to the correct WIT export; adding `run_support_geometry` to the trait and macro keyed-stage map is mechanical.
6. Packet 26's percent-unit fix and `MAX_SAMPLES_PER_EXPOLY` cap in `tree-support/src/lib.rs` and `traditional-support/src/lib.rs` remain in place — this packet extends tree-support's code but does not remove that fix.
7. No other packet is `status: active` when this packet activates.
