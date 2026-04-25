---
status: implemented
packet: 28_tree-support-multi-layer-propagation
task_ids:
  - TASK-120
  - TASK-120b
  - TASK-161
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 28_tree-support-multi-layer-propagation

## Goal

Introduce a new `PrePass::SupportGeneration` stage plus a `SupportPlanIR` blackboard contract that carries per-layer organic branch geometry produced by OrcaSlicer-style multi-layer propagation. Ship a first core-module planner (`support-planner`) that holds a new `support-planner` claim and implements a simplified port of OrcaSlicer's `TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes` (top-down `SupportNode` propagation + per-layer Prim MST for branch merging), without the full avoidance/collision/radius-tapering machinery. Update the `tree-support` per-layer `Layer::Support` module so its `run_support` implementation emits extrusion paths directly from `SupportPlanIR` when it is committed on the blackboard, with the existing grid-MST filler preserved as a fallback for when no planner module is loaded. Update `traditional-support`'s manifest and source comments to make explicit that it remains a per-layer scan-line filler and does not consume `SupportPlanIR`, so the two support modules coexist cleanly under the new contract.

## Scope Boundaries

- **In scope:** New WIT `export run-support-generation` + `support-generation-output` resource + `support-plan-entry` record in `wit/world-prepass.wit`; new `pub struct SupportPlanIR` and `pub struct SupportPlanEntry` in `crates/slicer-ir/src/slice_ir.rs`; host scheduler wiring in `crates/slicer-host/src/prepass.rs` (`PrepassStageOutput::SupportPlan`, `BlackboardPrepassSlot::SupportPlan`, `ensure_stage_prerequisites` entry for `PrePass::SupportGeneration`, `ir_path_for_prepass_output` mapping to `"SupportPlanIR"`, commit path); new `modules/core-modules/support-planner/` crate (manifest + `wit-guest/` + `src/lib.rs`) holding `support-planner` claim on `PrePass::SupportGeneration`, reading `MeshIR`+`SurfaceClassificationIR`+`LayerPlanIR`+`PaintRegionIR` and writing `SupportPlanIR`; simplified `detect_overhangs` + `drop_nodes` + per-layer `MinimumSpanningTree` merging implemented in that crate; `modules/core-modules/tree-support/` manifest + module source updates so `Layer::Support` prefers `SupportPlanIR`-driven emission when present and falls back to the existing grid-MST filler otherwise; `modules/core-modules/traditional-support/` manifest note + doc comment making its per-layer nature explicit (no functional change to its algorithm); integration tests under `crates/slicer-host/tests/prepass_support_generation_tdd.rs` and extensions to `crates/slicer-host/tests/live_support_generation_tdd.rs`; addition of `support-planner` to `modules/core-modules/build-core-modules.sh` `MODULES` list; rebuild of `tree-support.wasm` + `support-planner.wasm`.
- **Out of scope:** Full OrcaSlicer parity (avoidance grids / collision cache, branch radius tapering along `tan(angle) * height`, raft interaction, interface-layer logic, wall-count-aware `max_move_distance`, `tree_support_branch_angle/diameter/distance` tuning); changes to `Layer::Support` scheduling order (it still runs in the per-layer rayon tier); GUI or global config changes outside the new `support-planner` manifest keys; any change to packet 26's Benchy acceptance tests (they continue to exercise the grid-MST fallback path against a module set that has no `support-planner`); replacing `MinimumSpanningTree` with a heap-based variant (pre-existing O(V²) hazard mirrored from canonical OrcaSlicer — deferred).

## Prerequisites and Blockers

- **Depends on:** Packet `26_live-support-module-evidence` (real live support-dispatch evidence is the baseline this packet extends). Percent-unit fix + sample cap in `tree-support/src/lib.rs` and `traditional-support/src/lib.rs` landed by packet 26 must remain in place.
- **Unblocks:** Moving tree-support output from a radial grid-MST filler toward OrcaSlicer-style organic branches; future packets for full OrcaSlicer parity (avoidance grids, radius tapering, raft/interface handling).
- **Activation blockers:** (a) `TASK-161` row added to `docs/07_implementation_status.md` so the packet maps cleanly onto the backlog; (b) confirmation that no other packet is `status: active`.

## Acceptance Criteria

- **Given** `wit/world-prepass.wit`, **when** read, **then** it declares `export run-support-generation` with input `list<mesh-object-view>`, a `support-generation-output` resource with `push-support-plan: func(entry: support-plan-entry) -> result<_, string>`, and a `support-plan-entry` record carrying `global-layer-index: layer-idx`, `object-id: object-id`, `region-id: region-id`, and `branch-segments: list<list<point3-with-width>>`. | `grep -E 'run-support-generation|support-generation-output|support-plan-entry|push-support-plan|branch-segments' wit/world-prepass.wit`
- **Given** `crates/slicer-ir/src/slice_ir.rs`, **when** read, **then** it defines `pub struct SupportPlanIR { pub schema_version: SemVer, pub entries: Vec<SupportPlanEntry> }` and `pub struct SupportPlanEntry { pub global_layer_index: u32, pub object_id: ObjectId, pub region_id: RegionId, pub branch_segments: Vec<ExtrusionPath3D> }` and both are re-exported from `slicer_ir::` via `crates/slicer-ir/src/lib.rs`. | `grep -nE 'pub struct SupportPlanIR|pub struct SupportPlanEntry' crates/slicer-ir/src/slice_ir.rs && grep -nE 'SupportPlanIR|SupportPlanEntry' crates/slicer-ir/src/lib.rs`
- **Given** `crates/slicer-host/src/prepass.rs`, **when** read, **then** `PrepassStageOutput` has a `SupportPlan(Arc<SupportPlanIR>)` variant, `ir_path_for_prepass_output` returns `Some(String::from("SupportPlanIR"))` for it, and the commit path writes it to the blackboard via a new `commit_support_plan` slot. | `grep -nE 'SupportPlan\(Arc<SupportPlanIR>\)|"SupportPlanIR"|commit_support_plan' crates/slicer-host/src/prepass.rs crates/slicer-host/src/blackboard.rs`
- **Given** `crates/slicer-host/src/prepass.rs` `ensure_stage_prerequisites`, **when** queried for `"PrePass::SupportGeneration"`, **then** the returned required-slots slice contains exactly `BlackboardPrepassSlot::SurfaceClassification` and `BlackboardPrepassSlot::LayerPlan` in that order. | `grep -nA2 '"PrePass::SupportGeneration"' crates/slicer-host/src/prepass.rs | head -5`
- **Given** `modules/core-modules/support-planner/support-planner.toml`, **when** read, **then** `[stage].id = "PrePass::SupportGeneration"`, `[claims].holds = ["support-planner"]`, `[ir-access].reads` contains `"MeshIR"`, `"SurfaceClassificationIR"`, `"PaintRegionIR"` (note: `LayerPlanIR` is a host-side scheduling prerequisite via `ensure_stage_prerequisites`, not a runtime read of the v1 layer-height-agnostic planner), `[ir-access].writes = ["SupportPlanIR"]`, and `[module].wit-world = "slicer:world-prepass@1.0.0"`. | `grep -E 'id = "PrePass::SupportGeneration"|holds    = \["support-planner"\]|writes = \["SupportPlanIR"\]|wit-world    = "slicer:world-prepass@1.0.0"' modules/core-modules/support-planner/support-planner.toml`
- **Given** the `support-planner` core-module is loaded into an execution plan and `PrePass::SupportGeneration` runs against a fixture with `SurfaceClassificationIR.needs_support = true` on ≥3 layers, **when** `execute_prepass_with_builtins` returns, **then** the blackboard's committed `SupportPlanIR.entries` is non-empty and every entry carries `branch_segments` with ≥1 two-point segment, each segment endpoint typed as `Point3WithWidth`. | `cargo test -p slicer-host --test prepass_support_generation_tdd support_planner_produces_branches_for_overhang_fixture -- --nocapture 2>&1 | tail -20`
- **Given** a Layer::Support dispatch of the updated `tree-support` module with a committed `SupportPlanIR` carrying a known branch set for the target layer, **when** the dispatch completes, **then** the emitted `SupportIR.support_paths` point coordinates match the `SupportPlanIR` branch segment endpoints byte-for-byte (x,y,z,width tolerance < 1e-4 mm) and carry `ExtrusionRole::SupportMaterial`. | `cargo test -p slicer-host --test live_support_generation_tdd planner_consuming_tier::tree_support_live_dispatch_consumes_support_plan_ir -- --nocapture 2>&1 | tail -20`
- **Given** the same `PrePass::SupportGeneration` dispatch run twice in sequence on the same fixture, **when** both resulting `SupportPlanIR` values are compared, **then** `entries.len()`, each entry's `branch_segments.len()`, and every endpoint coordinate are byte-identical. | `cargo test -p slicer-host --test prepass_support_generation_tdd support_planner_is_deterministic_across_runs -- --nocapture 2>&1 | tail -20`
- **Given** a Layer::Support dispatch of `traditional-support` first with no `SupportPlanIR` committed and then with a non-empty `SupportPlanIR` committed for the same layer, **when** both emitted `SupportIR.support_paths` are compared, **then** they are byte-identical (point count, coordinates, role, order). | `cargo test -p slicer-host --test live_support_generation_tdd planner_consuming_tier::traditional_support_live_dispatch_ignores_support_plan_ir -- --nocapture 2>&1 | tail -20`
- **Given** a Layer::Support dispatch of the updated `tree-support` module with no `SupportPlanIR` committed on the blackboard, **when** the dispatch completes, **then** it falls through to the existing grid-MST filler and emits a non-empty `SupportIR.support_paths` whose point count equals the grid-MST filler's output count for the same region and config. | `cargo test -p slicer-host --test live_support_generation_tdd planner_consuming_tier::tree_support_live_dispatch_falls_back_to_grid_when_plan_absent -- --nocapture 2>&1 | tail -20`
- **Given** `modules/core-modules/build-core-modules.sh` `MODULES` array, **when** read, **then** it contains an entry `"support-planner:support_planner_guest"` and running `build-core-modules.sh --check` reports `support-planner.wasm` as up to date after the packet builds it. | `grep -E '"support-planner:support_planner_guest"' modules/core-modules/build-core-modules.sh && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'support-planner' | head -3`

## Negative Test Cases

- **Given** a `SurfaceClassificationIR` with `needs_support = false` for every facet of every object and no `SupportEnforcer` paint regions, **when** `support-planner` runs through `execute_prepass`, **then** the committed `SupportPlanIR.entries` is empty and the module returns `Ok(())` (no `ModuleError`). | `cargo test -p slicer-host --test prepass_support_generation_tdd support_planner_emits_empty_plan_when_no_overhangs -- --nocapture 2>&1 | tail -20`
- **Given** an `ExecutionPlan` whose `prepass_stages` lists `PrePass::SupportGeneration` before `PrePass::LayerPlanning` has committed a `LayerPlanIR`, **when** `execute_prepass` runs, **then** it returns `PrepassExecutionError::StagePrerequisite` naming `"LayerPlanIR"` as the missing slot. | `cargo test -p slicer-host --test prepass_support_generation_tdd prepass_support_generation_fails_without_layer_plan -- --nocapture 2>&1 | tail -20`
- **Given** two core modules that both declare `holds = ["support-planner"]` on `PrePass::SupportGeneration`, **when** `load_live_modules_for_plan` runs, **then** the alphabetical first-winner dedup keeps one module, drops the other, and emits a `DiagnosticLevel::Info` diagnostic whose message contains `dropped: claim 'support-planner'`. | `cargo test -p slicer-host --test prepass_support_generation_tdd support_planner_claim_dedup -- --nocapture 2>&1 | tail -20`

## Verification

- `cargo test -p slicer-host --test prepass_support_generation_tdd -- --nocapture`
- `cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture`
- `bash modules/core-modules/build-core-modules.sh`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — §Pipeline tiers (PrePass → Per-Layer → PostPass), §Stage I/O Contract for `Layer::Support`, §Tier 1 PrePass stage list.
- `docs/02_ir_schemas.md` — existing `SeamPlanIR` section (precedent for new `SupportPlanIR` keyed by `(layer, object, region)`), §IR Versioning Contract.
- `docs/03_wit_and_manifest.md` — §prepass world, §module manifest schema, §host-boundary enforcement.
- `docs/04_host_scheduler.md` — §PrePass Execution (sequential), §ensure_stage_prerequisites, §Global claim conflicts, §Full Lifecycle.
- `docs/05_module_sdk.md` — PrePass module authoring pattern.
- `.ralph/specs/23-rev1_prepass-seam-planning-orca-parity/` — precedent packet for adding a PrePass stage + accompanying IR; structural reference only (no file modifications in that packet).

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::detect_overhangs` (overhang contact point detection; fixture inspiration), `TreeSupport::drop_nodes` (line 2625; top-down propagation, per-layer MST grouping, two-pass merge-then-move).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — `SupportNode` struct (contact-point node shape we mirror in a simplified form).
- `OrcaSlicerDocumented/src/libslic3r/MinimumSpanningTree.cpp` — `MinimumSpanningTree::prim` (O(V²) Prim; our `PrePass::SupportGeneration` MST matches this complexity class intentionally — the scaling win is that V drops from grid samples to propagated branch nodes, not a different algorithm).
