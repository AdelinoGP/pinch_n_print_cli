# Requirements: 28_tree-support-multi-layer-propagation

## Problem Statement

After packet `26_live-support-module-evidence` landed, the `tree-support.wasm` core module does emit `SupportIR.support_paths` on the live Benchy path, but only by running a naive **single-layer grid-MST filler**: for each `ExPolygon` on a given layer it samples a dense interior grid, builds a Prim minimum-spanning tree across those samples, and emits tree edges as extrusion paths. The output is a radial line pattern confined to one layer, not an organic branching structure, and has no relation to OrcaSlicer's actual tree-support algorithm.

OrcaSlicer's tree support in `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` is fundamentally a **multi-layer, top-down propagation** algorithm (`TreeSupport::drop_nodes`): it detects overhang contact points across all layers, places `SupportNode` instances at those contacts, then walks layers top → bottom moving each node by `tan(angle) * layer_height` per layer, merging close nodes via a per-layer minimum spanning tree, and only emits extrusion paths after propagation finishes. The per-layer MST is used only as a merge-decision helper; the decisive work is the cross-layer propagation.

The current slicer-host architecture cannot host OrcaSlicer's algorithm inside `Layer::Support` because that stage runs in the rayon-parallel per-layer tier (layers are computed in arbitrary order). Multi-layer propagation must execute sequentially during `PrePass` — before per-layer work begins — and write its branch geometry to the blackboard so per-layer emitters can consume it.

This packet introduces that architectural change. It adds a new `PrePass::SupportGeneration` stage, a new `SupportPlanIR` blackboard contract, and a first core-module implementation (`support-planner`) that performs a simplified version of `detect_overhangs` + `drop_nodes` (without avoidance/collision grids, radius tapering, raft handling, or interface-layer logic — those are deferred to future packets). The per-layer `tree-support` module is updated so its `Layer::Support` stage consumes `SupportPlanIR` when present and emits branches from pre-planned geometry; the existing grid-MST filler remains as a fallback path so the module continues to work when no planner is installed. The `traditional-support` module stays on the per-layer `Layer::Support` tier — its scan-line fill is algorithmically per-layer and correctly declines to consume `SupportPlanIR`; we only add a module-level doc comment making this explicit, plus (if needed) an assertion that its `[ir-access].reads` does not claim `SupportPlanIR`.

This packet does **not** supersede packet 26. Packet 26's commit-path and real live-dispatch tiers stay in place; this packet extends them with a new "planner-consuming" tier.

## Grouped Task IDs

- **TASK-120** — umbrella Phase H acceptance with tree supports (still `[~]`; this packet closes the organic-branch gap but not the full Phase H matrix).
- **TASK-120b** — live support generation evidence (re-closed by packet 26 on the grid-MST path; this packet upgrades the tree-support algorithm without reopening the task).
- **TASK-161** — *(to be added to `docs/07_implementation_status.md` before the packet activates)* Introduce `PrePass::SupportGeneration` plus `SupportPlanIR` and wire `tree-support` to consume it so Benchy-path tree supports are organic branches instead of radial grid-MST output. Deepens OrcaSlicer parity. Supports TASK-120.

Draft line to paste under Workstream 3 in `docs/07_implementation_status.md`:

```
- [ ] TASK-161 Introduce `PrePass::SupportGeneration` plus a canonical `SupportPlanIR` blackboard contract so tree-support branches can be planned across layers (simplified port of OrcaSlicer `TreeSupport::drop_nodes`) and emitted by `Layer::Support` from pre-planned geometry. Continues DEV-009, deepens Orca parity, and supports TASK-120.
```

## In-Scope

- New WIT prepass stage: `export run-support-generation` with a `support-generation-output` resource carrying `push-support-plan`, plus a `support-plan-entry` record.
- New IR: `SupportPlanIR` + `SupportPlanEntry` in `slicer-ir`, with `schema_version: SemVer { major: 1, minor: 0, patch: 0 }`. (No version bump on existing IRs; `SupportIR` is unchanged.)
- Host wiring: `PrepassStageOutput::SupportPlan`, a new `BlackboardPrepassSlot::SupportPlan`, a new `commit_support_plan` on `Blackboard`, and an `ensure_stage_prerequisites` entry for `PrePass::SupportGeneration`.
- New core-module crate `modules/core-modules/support-planner/` holding claim `support-planner` on `PrePass::SupportGeneration`, implementing simplified OrcaSlicer-style propagation (no avoidance/collision/radius-tapering). Reads `MeshIR`, `SurfaceClassificationIR`, `LayerPlanIR`, `PaintRegionIR`; writes `SupportPlanIR`.
- `modules/core-modules/tree-support/`: manifest `reads` updated to include `SupportPlanIR`; `src/lib.rs` updated so `run_support` consults the committed plan (per-layer slice of `SupportPlanIR`) and emits branches from it when present, falling through to the grid-MST filler when absent. Existing percent-unit fix + `MAX_SAMPLES_PER_EXPOLY` sample cap from packet 26 preserved.
- `modules/core-modules/traditional-support/`: no algorithmic change; module-level doc comment added stating per-layer scan-line nature and explicitly noting `SupportPlanIR` is ignored. Manifest `reads` unchanged (no `SupportPlanIR`).
- `modules/core-modules/build-core-modules.sh`: add `"support-planner:support_planner_guest"` to the `MODULES` list.
- Rebuild of `tree-support.wasm` and `support-planner.wasm` via the build script.
- Integration tests:
  - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` — overhang-fixture positive, empty-overhangs negative, missing-`LayerPlanIR` prerequisite negative, claim dedup negative, determinism across repeated runs.
  - Extensions to `crates/slicer-host/tests/live_support_generation_tdd.rs` — new Section C "planner-consuming tier": `tree_support_consumes_support_plan_ir`, `tree_support_falls_back_to_grid_when_plan_absent`, `traditional_support_ignores_support_plan_ir`.

## Out-of-Scope

- Full OrcaSlicer parity:
  - `TreeSupportData` avoidance/collision cache (`get_avoidance`, `get_collision`, `get_collision_polys`).
  - Per-node radius tapering via `tan(angle) * dist_mm_to_top` and `tree_support_branch_diameter_angle`.
  - Raft layer interaction (`m_raft_layers`).
  - Interface layer stacking (`support_interface_top_layers` / `_bottom_layers`).
  - Wall-count-aware `max_move_distance` scaling.
  - `tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_distance` config tuning.
- Replacing `MinimumSpanningTree` Prim with a heap-based O(V log E) variant — OrcaSlicer itself carries the same O(V²) TODO; defer to a follow-up packet.
- Any change to `Layer::Support` scheduling order or its rayon-parallel tier.
- GUI / global config changes outside the new `support-planner` `[config.schema]` keys.
- Packet 26's Benchy acceptance tests (`benchy_with_support_enabled`, `benchy_support_marker_present`, `benchy_support_deterministic`, `benchy_no_support_marker_when_disabled`, `tree_support_active_holder`). They continue to exercise the grid-MST fallback against module sets that do not install `support-planner`.
- Any change to the `support-generator` claim contract held by `tree-support` and `traditional-support` on `Layer::Support`.

## Authoritative Docs

- `docs/01_system_architecture.md` — Pipeline tiers; `Layer::Support` Stage I/O Contract; Tier 1 PrePass stage list.
- `docs/02_ir_schemas.md` — Existing `SeamPlanIR` shape (structural precedent); `IR Versioning Contract`.
- `docs/03_wit_and_manifest.md` — prepass world; module manifest schema; host-boundary enforcement (declared reads / writes).
- `docs/04_host_scheduler.md` — PrePass Execution (sequential); `ensure_stage_prerequisites`; Global claim conflicts (first-winner alphabetical dedup); Full Lifecycle.
- `docs/05_module_sdk.md` — PrePass module authoring pattern.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`:
  - `TreeSupport::detect_overhangs` — reference for how contact points are enumerated from overhang geometry + support-enforcer paint regions.
  - `TreeSupport::drop_nodes` (line 2625) — reference for the top-down propagation pattern: group by part, build per-group MST, merge-then-move in two passes. The simplified port keeps the shape but removes avoidance/collision/radius-taper.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — `SupportNode` struct as structural reference for our simplified `PlannedSupportNode`.
- `OrcaSlicerDocumented/src/libslic3r/MinimumSpanningTree.cpp` — `MinimumSpanningTree::prim` (the O(V²) Prim implementation we match in complexity class; the win over packet 26's grid-MST is in input sizing, not algorithm choice).

## Acceptance Summary

After this packet lands:

1. `wit/world-prepass.wit` declares `PrePass::SupportGeneration` via `run-support-generation` + `support-generation-output` + `support-plan-entry`.
2. `slicer-ir::{SupportPlanIR, SupportPlanEntry}` are the canonical blackboard types for per-layer branch geometry produced during PrePass.
3. `slicer-host` schedules `PrePass::SupportGeneration` with prerequisite slots `SurfaceClassification` and `LayerPlan`, commits its output to a new `BlackboardPrepassSlot::SupportPlan`, and audits the read/write masks in the same shape as `PrePass::SeamPlanning`.
4. `support-planner` core-module is present, discovered by `load_live_modules_for_plan`, holds claim `support-planner`, reads `MeshIR`+`SurfaceClassificationIR`+`LayerPlanIR`+`PaintRegionIR`, writes `SupportPlanIR`, and produces non-empty branch plans for a Benchy-shaped overhang fixture.
5. `tree-support`'s `Layer::Support` dispatch reads `SupportPlanIR` when present and emits the committed branch segments with `ExtrusionRole::SupportMaterial`; when no plan is present, it falls back to the packet-26 grid-MST filler (byte-identical to the fallback path it ran before this packet).
6. `traditional-support` continues to emit its scan-line fill whether or not `SupportPlanIR` is committed.
7. Determinism holds: two identical prepass runs produce byte-identical `SupportPlanIR`; two identical `Layer::Support` dispatches with the same plan produce byte-identical `SupportIR`.
8. Negative cases: empty overhangs → empty plan (no error); missing `LayerPlanIR` → `PrepassExecutionError::StagePrerequisite`; two `support-planner` holders → first-winner dedup plus diagnostic.

## Cross-Packet Dependencies and Unblockers

- **Depends on:** packet 26 (live support-module evidence; percent-unit + sample-cap fixes in both support modules).
- **Does not supersede:** any prior packet. This is an additive architectural extension, not a correction.
- **Unblocks:** follow-up packets for full OrcaSlicer parity (avoidance/collision, radius tapering, raft/interface, branch tuning).

## Verification

```
cargo test -p slicer-host --test prepass_support_generation_tdd -- --nocapture
cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture
bash modules/core-modules/build-core-modules.sh
cargo build --workspace
cargo clippy --workspace -- -D warnings
```
