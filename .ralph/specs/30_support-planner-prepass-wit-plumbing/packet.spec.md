---
status: draft
packet: 30_support-planner-prepass-wit-plumbing
task_ids:
  - TASK-162
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 30_support-planner-prepass-wit-plumbing

## Goal

Close the two correctness gaps that packet 28 carved out as v1 limitations of `support-planner`: (a) the planner is layer-height-agnostic because `LayerPlanIR.layers` is not surfaced through the prepass WIT, and (b) every emitted entry uses the canonical `region_id = "0"` bucket because per-region segmentation is not surfaced either. This packet extends `run-support-generation` to receive `layer-plan-view` and `region-segmentation-view` inputs derived from the committed `LayerPlanIR` and `RegionMapIR`, updates `support-planner` to walk the real layer plan and emit one entry per `(layer, object, region)` triple that exists in `RegionMapIR`, restores `LayerPlanIR` to the planner's manifest reads, adds `RegionMapIR` to those reads, and adds `RegionMap` to the host-side prerequisite chain for `PrePass::SupportGeneration`. Branch geometry remains object-wide (the same segment set is emitted under each region the object owns on a layer); geometry-aware multi-region branch placement is deferred to packet `31`.

## Scope Boundaries

- **In scope:** Extend `wit/world-prepass.wit` with `layer-plan-view-entry` + `layer-plan-view` records, `region-segmentation-view-entry` + `region-segmentation-view` records, and add both as parameters of `export run-support-generation` (between `objects` and `output`); extend `crates/slicer-sdk/src/prepass_types.rs` with matching host-side types; extend the `PrepassModule::run_support_generation` trait signature in `crates/slicer-sdk/src/traits.rs` to take `&LayerPlanView` and `&RegionSegmentationView`; extend `#[slicer_module]` macro in `crates/slicer-macros/src/lib.rs` to thread the new args; extend the host prepass dispatcher in `crates/slicer-host/src/wit_host.rs` to project `LayerPlanIR.layers` and `RegionMapIR.entries` into the new WIT views before calling `run-support-generation`; add `BlackboardPrepassSlot::RegionMap` to the prerequisite slice for `PrePass::SupportGeneration` in `crates/slicer-host/src/prepass.rs::required_slots`; update `modules/core-modules/support-planner/support-planner.toml` `[ir-access].reads` to include `"LayerPlanIR"` and `"RegionMapIR"` (alongside the existing `MeshIR`, `SurfaceClassificationIR`, `PaintRegionIR`); update `modules/core-modules/support-planner/src/lib.rs` to walk `layer_plan_view.layers` directly (replacing the `DEFAULT_LAYER_HEIGHT_MM` derivation) and emit one `SupportPlanEntry` per `(layer, object, region)` triple from `region_segmentation_view`; remove the v1 doc-comment limitations covering layer-height-agnostic and single-region behavior; rebuild every prepass `.wasm` (additive WIT change forces cascade); add `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` covering variable-layer-height + multi-region cases; add `TASK-162` row to `docs/07_implementation_status.md`.
- **Out of scope:** Per-layer geometry-aware branch placement across multi-region objects (every region under one object still gets the same branch segments — closing this gap requires per-layer slice polygons and is deferred to packet `31`); the `TreeSupportData` avoidance/collision cache; per-node radius tapering along `tan(angle) * dist_to_top`; raft prefix layer emission; interface-layer densification; wall-count-aware `max_move_distance` scaling; the three OrcaSlicer config keys `tree_support_branch_angle`/`_diameter`/`_distance` (added in packet `31`); replacing `MinimumSpanningTree::prim` with a heap-based variant.

## Prerequisites and Blockers

- **Depends on:** Packet `28_tree-support-multi-layer-propagation` (introduces the `support-planner` crate, `PrePass::SupportGeneration` stage, and `SupportPlanIR` blackboard slot this packet extends).
- **Unblocks:** Packet `31_support-planner-orca-algorithmic-parity` — the avoidance/collision cache and radius tapering both require `LayerPlanView` data, and the geometry-aware multi-region branch placement requires the per-region entry shape this packet ships.
- **Activation blockers:**
  - `TASK-162` row added to `docs/07_implementation_status.md`.
  - Packet `28_tree-support-multi-layer-propagation` moved to `status: implemented` (only one packet may be active at a time).

## Acceptance Criteria

- **Given** `wit/world-prepass.wit`, **when** read, **then** it declares `record layer-plan-view-entry { global-layer-index: layer-idx, z: f32, effective-layer-height: f32 }`, `record layer-plan-view { layers: list<layer-plan-view-entry> }`, `record region-segmentation-view-entry { object-id: object-id, layer-index: layer-idx, region-ids: list<region-id> }`, `record region-segmentation-view { entries: list<region-segmentation-view-entry> }`, and `export run-support-generation` carries `layer-plan: layer-plan-view, region-segmentation: region-segmentation-view` between the `objects` and `output` parameters. | `grep -nE 'record layer-plan-view-entry|record layer-plan-view\b|record region-segmentation-view-entry|record region-segmentation-view\b|layer-plan: layer-plan-view|region-segmentation: region-segmentation-view' wit/world-prepass.wit`
- **Given** `crates/slicer-sdk/src/prepass_types.rs`, **when** read, **then** it defines `pub struct LayerPlanViewEntry { pub global_layer_index: u32, pub z: f32, pub effective_layer_height: f32 }`, `pub struct LayerPlanView { pub layers: Vec<LayerPlanViewEntry> }`, `pub struct RegionSegmentationViewEntry { pub object_id: String, pub layer_index: u32, pub region_ids: Vec<RegionId> }`, and `pub struct RegionSegmentationView { pub entries: Vec<RegionSegmentationViewEntry> }`, all re-exported from the SDK prelude. | `grep -nE 'pub struct LayerPlanViewEntry|pub struct LayerPlanView\b|pub struct RegionSegmentationViewEntry|pub struct RegionSegmentationView\b' crates/slicer-sdk/src/prepass_types.rs && grep -nE 'LayerPlanView|RegionSegmentationView' crates/slicer-sdk/src/prelude.rs`
- **Given** `crates/slicer-sdk/src/traits.rs`, **when** read, **then** the `PrepassModule::run_support_generation` signature is `fn run_support_generation(&self, objects: &[MeshObjectView], layer_plan: &LayerPlanView, region_segmentation: &RegionSegmentationView, output: &mut SupportGenerationOutput, config: &ConfigView) -> Result<(), ModuleError>` and its default body returns `Err(ModuleError::unimplemented("run_support_generation"))`. | `grep -nA8 'fn run_support_generation' crates/slicer-sdk/src/traits.rs | head -12`
- **Given** `crates/slicer-host/src/prepass.rs::required_slots`, **when** queried with `"PrePass::SupportGeneration"`, **then** the returned slice equals `&[BlackboardPrepassSlot::SurfaceClassification, BlackboardPrepassSlot::LayerPlan, BlackboardPrepassSlot::RegionMap]` in that order. | `grep -nA4 '"PrePass::SupportGeneration"' crates/slicer-host/src/prepass.rs | head -8`
- **Given** `modules/core-modules/support-planner/support-planner.toml`, **when** read, **then** `[ir-access].reads` is exactly `["MeshIR", "SurfaceClassificationIR", "LayerPlanIR", "RegionMapIR", "PaintRegionIR"]` (order preserved) and the v1 layer-height-agnostic comment block above the array is removed. | `grep -nE 'reads  = \["MeshIR", "SurfaceClassificationIR", "LayerPlanIR", "RegionMapIR", "PaintRegionIR"\]' modules/core-modules/support-planner/support-planner.toml && ! grep -n 'layer-height-agnostic' modules/core-modules/support-planner/support-planner.toml`
- **Given** `modules/core-modules/support-planner/src/lib.rs`, **when** read, **then** the constant `DEFAULT_LAYER_HEIGHT_MM` is removed and the planner derives layer count + per-layer Z + per-layer effective height from the `&LayerPlanView` argument (no `bmax[2] / 0.2` arithmetic remains). | `! grep -n 'DEFAULT_LAYER_HEIGHT_MM' modules/core-modules/support-planner/src/lib.rs && ! grep -nE 'object_height / .*layer_height\.ceil' modules/core-modules/support-planner/src/lib.rs`
- **Given** a fixture object with bounding-box height 2.0 mm whose `LayerPlanIR.layers` declares 4 layers with z-values `[0.4, 0.8, 1.2, 2.0]` and effective heights `[0.4, 0.4, 0.4, 0.8]`, **when** `support-planner` runs through `execute_prepass_with_builtins`, **then** the committed `SupportPlanIR.entries` only carry `global_layer_index ∈ {0, 1, 2, 3}` (no entry indexes 4 or above) and the highest entry's `branch_segments[*][0].z` is within 1e-4 mm of `2.0`. | `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd planner_walks_real_layer_plan_with_variable_layer_heights -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** a fixture object whose `RegionMapIR.entries` declare two regions with `RegionId` 7 and 42 both present on global layer 5 for `object_id = "obj-multi"`, **when** `support-planner` runs against an overhang on layer 5, **then** the committed `SupportPlanIR.entries` contains two entries with `(global_layer_index = 5, object_id = "obj-multi")` whose `region_id` values are `7` and `42` (in deterministic ascending order) and whose `branch_segments` are byte-identical between the two entries. | `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd planner_emits_one_entry_per_region_in_region_map -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** the planner has emitted entries for `region_id ∈ {7, 42}` on layer 5, **when** `tree-support` runs `Layer::Support` against a `LayerView` whose `region_id() == 42`, **then** `paint.support_plan_segments_for("obj-multi", 42)` returns a non-empty slice and the emitted `SupportIR.support_paths[*]` carry `ExtrusionRole::SupportMaterial` with point coordinates byte-identical to the planner's `region_id = 42` entry. | `cargo test -p slicer-host --test live_support_generation_tdd planner_consuming_tier::tree_support_live_dispatch_finds_branches_for_real_region_id -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** every existing prepass core module (`mesh-segmentation`, `paint-segmentation`, `layer-planner-default`, `paint-region-annotator`, `seam-planner-default`), **when** `bash modules/core-modules/build-core-modules.sh` runs after the WIT change, **then** every artifact is rebuilt without compile error and `--check` reports each `.wasm` as up to date. | `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -20 && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'STALE' ; test $? -eq 1`
- **Given** `docs/07_implementation_status.md`, **when** read, **then** it contains exactly one row matching `^- \[.\] TASK-162 ` whose body mentions `LayerPlanIR` and `RegionMapIR` and references this packet by slug. | `grep -nE '^- \[.\] TASK-162 .*LayerPlanIR.*RegionMapIR.*30_support-planner-prepass-wit-plumbing' docs/07_implementation_status.md`

## Negative Test Cases

- **Given** an `ExecutionPlan` whose `prepass_stages` schedules `PrePass::SupportGeneration` before `PrePass::RegionMapping` has committed a `RegionMapIR`, **when** `execute_prepass` runs, **then** it returns `PrepassExecutionError::MissingRequiredPrepass { stage_id: "PrePass::SupportGeneration", slot: BlackboardPrepassSlot::RegionMap }`. | `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd prepass_support_generation_fails_without_region_map -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** a fixture whose `RegionMapIR.entries` is empty for `object_id = "obj-noregions"` on every layer, **when** `support-planner` runs with overhangs detected on that object, **then** the committed `SupportPlanIR.entries` contains zero entries for `object_id = "obj-noregions"` (the planner skips an object that owns no regions in `RegionMapIR`) and the module returns `Ok(())`. | `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd planner_skips_object_with_empty_region_map -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** a fixture whose `LayerPlanIR.layers` is non-empty but a host bug populates the `LayerPlanView` with `layers: vec![]`, **when** `support-planner.run_support_generation` is invoked directly, **then** it returns `Err(ModuleError::fatal(_, msg))` whose `msg` contains the literal substring `"empty layer-plan-view"`. | `cargo test -p support-planner --lib empty_layer_plan_view_returns_fatal_module_error -- --nocapture 2>&1 | tail -20`

## Verification

- `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture` (regression — packet 28's existing tests stay green)
- `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture`
- `cargo test -p support-planner --lib`
- `bash modules/core-modules/build-core-modules.sh`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — §Pipeline tiers (PrePass sequential), §Stage I/O Contract for `PrePass::SupportGeneration`.
- `docs/02_ir_schemas.md` — `LayerPlanIR`, `RegionMapIR`, `SupportPlanIR` (existing); §IR Versioning Contract (no version bumps — additive WIT change does not bump IR schemas).
- `docs/03_wit_and_manifest.md` — §prepass world, §host-boundary enforcement (declared reads), §additive WIT change rebuild rule.
- `docs/04_host_scheduler.md` — §PrePass Execution (sequential), §`ensure_stage_prerequisites`, §Full Lifecycle.
- `docs/05_module_sdk.md` — PrePass module authoring, signature evolution rules.
- `.ralph/specs/28_tree-support-multi-layer-propagation/` — precedent packet (carved out the v1 limits this packet closes).

## OrcaSlicer Reference Obligations

- None new for this packet — the WIT plumbing is repo-internal. Behavioural OrcaSlicer parity continues in packet `31`.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
