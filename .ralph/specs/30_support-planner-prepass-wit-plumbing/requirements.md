# Requirements: 30_support-planner-prepass-wit-plumbing

## Packet Metadata

- Grouped task IDs:
  - `TASK-162`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Packet `28_tree-support-multi-layer-propagation` introduced `PrePass::SupportGeneration`, the `SupportPlanIR` blackboard contract, and the `support-planner` core module. To stay inside one bounded slice, packet 28 deliberately accepted two correctness gaps as v1 limitations and documented them in `modules/core-modules/support-planner/src/lib.rs`:

1. **Layer-height-agnostic.** The planner walks `(bmax[2] - bmin[2]) / DEFAULT_LAYER_HEIGHT_MM` instead of consulting the committed `LayerPlanIR.layers`. This works for the test fixtures that use a uniform 0.2 mm layer height but silently produces wrong layer counts and Z-values for any object configured with variable layer heights. `LayerPlanIR` was dropped from the planner's manifest `[ir-access].reads` because the prepass WIT does not surface it to the guest.
2. **Single-region per object.** Every emitted `SupportPlanEntry` uses the canonical `region_id = "0"` bucket because `MeshObjectView` does not carry per-region segmentation. Single-region fixtures match correctly because tree-support invokes `support_plan_segments_for(object_id, 0)`; multi-region objects collapse all branches into the first region.

Both gaps are correctness bugs (not just incomplete OrcaSlicer parity) and both block the algorithmic packets that follow: the avoidance/collision cache needs the real layer plan, and any geometry-aware multi-region branch placement needs real region IDs on each entry.

This packet closes both gaps by extending `run-support-generation` to receive a `layer-plan-view` and a `region-segmentation-view` projected from the committed `LayerPlanIR.layers` and `RegionMapIR.entries`. The planner walks the real layer plan and emits one `SupportPlanEntry` per `(layer, object, region)` triple that exists in `RegionMapIR`. Branch geometry remains object-wide for now — every region under one object on a layer receives the same branch segments — because true geometry-aware multi-region placement requires per-layer slice polygons that are deferred to packet `31b`.

This packet does **not** supersede packet 28; it extends the v1 contract additively and removes the documented v1 carve-outs.

## In Scope

- WIT extension: `wit/world-prepass.wit` adds `layer-plan-view-entry` + `layer-plan-view` records, `region-segmentation-view-entry` + `region-segmentation-view` records, and threads both as new positional parameters of `export run-support-generation` between `objects` and `output`.
- SDK types: `crates/slicer-sdk/src/prepass_types.rs` defines matching host-side `LayerPlanView`, `LayerPlanViewEntry`, `RegionSegmentationView`, `RegionSegmentationViewEntry` structs, all re-exported from `crates/slicer-sdk/src/prelude.rs`.
- SDK trait: `PrepassModule::run_support_generation` in `crates/slicer-sdk/src/traits.rs` takes the two new args; default body still returns `Err(ModuleError::unimplemented(...))` so other prepass modules continue to compile.
- Macro: `#[slicer_module]` in `crates/slicer-macros/src/lib.rs` threads the two new args from the WIT export to the trait method when the manifest's `stage.id == "PrePass::SupportGeneration"`.
- Host glue: `crates/slicer-host/src/wit_host.rs` projects `LayerPlanIR.layers` and `RegionMapIR.entries` into the WIT views and passes them when invoking the support-generation export.
- Host scheduler: `crates/slicer-host/src/prepass.rs::required_slots("PrePass::SupportGeneration")` returns `&[SurfaceClassification, LayerPlan, RegionMap]`.
- Manifest: `modules/core-modules/support-planner/support-planner.toml` `[ir-access].reads` becomes `["MeshIR", "SurfaceClassificationIR", "LayerPlanIR", "RegionMapIR", "PaintRegionIR"]` and the v1 layer-height-agnostic comment block is removed.
- Planner code: `modules/core-modules/support-planner/src/lib.rs` removes `DEFAULT_LAYER_HEIGHT_MM`, walks `layer_plan_view.layers` (using `effective_layer_height` per-layer for the `tan(angle) * h` move step and `z` for entry coordinates), and emits one entry per `(layer, object, region)` triple from `region_segmentation_view`. Module-level v1 doc comments for limits (1) and (2) are removed.
- Tests: new file `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` covering variable-layer-height walk, multi-region entry emission, missing-RegionMap prerequisite, empty-region-map skip, empty-layer-plan-view fatal. Existing tests in `crates/slicer-host/tests/prepass_support_generation_tdd.rs` and `crates/slicer-host/tests/live_support_generation_tdd.rs` extended where needed; new live test `planner_consuming_tier::tree_support_live_dispatch_finds_branches_for_real_region_id` covers the multi-region tree-support path.
- Build: rebuild every prepass `.wasm` (additive WIT change forces cascade); the `MODULES` array order is unchanged.
- Backlog: `docs/07_implementation_status.md` adds `TASK-162` row under Workstream 3.

## Out of Scope

- Per-layer geometry-aware branch placement when an object has multiple regions: branches remain object-wide and are duplicated across each region the object owns on a layer. Packets `31a` and `31b` add `SupportGeometryView`-based avoidance/collision and per-region geometry separation.
- `TreeSupportData` avoidance and collision caches.
- Per-node radius tapering along `tan(angle) * dist_to_top`.
- Raft prefix layers and interface-layer densification.
- Wall-count-aware `max_move_distance` scaling.
- The three OrcaSlicer config keys `tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_distance` (added in packet `31b`).
- Replacing `MinimumSpanningTree::prim` with a heap-based variant.
- Catchup / variable-per-region effective layer heights interacting with branch propagation — this packet honors `LayerPlanIR.layers[*].effective_layer_height` per-layer but does not recompute per-region effective heights mid-walk.
- Changes to `Layer::Support` scheduling order or claim layout.

## Authoritative Docs

- `docs/01_system_architecture.md` — Pipeline tiers, Stage I/O Contract for `PrePass::SupportGeneration`.
- `docs/02_ir_schemas.md` — `LayerPlanIR.layers` shape (Z + effective layer height per layer), `RegionMapIR.entries` keyed by `RegionKey`, `SupportPlanIR.entries` keying.
- `docs/03_wit_and_manifest.md` — Prepass world, host-boundary enforcement, manifest schema, **additive WIT change rebuild rule** (every prepass `.wasm` must be rebuilt).
- `docs/04_host_scheduler.md` — `ensure_stage_prerequisites`, sequential PrePass execution.
- `docs/05_module_sdk.md` — PrePass module authoring; signature evolution policy.

## OrcaSlicer Reference Obligations

- None new. The WIT plumbing is repo-internal; the algorithmic OrcaSlicer parity continues in packets `31a` and `31b`.

## Acceptance Summary

- **Positive cases:**
  - WIT records and `run-support-generation` export shape exact (AC-1).
  - SDK types defined and re-exported (AC-2).
  - SDK trait signature updated (AC-3).
  - Host-side prerequisite slice extended with `RegionMap` (AC-4).
  - Manifest reads list updated and v1 comment removed (AC-5).
  - `DEFAULT_LAYER_HEIGHT_MM` constant deleted from planner; layer count derived from `LayerPlanView` (AC-6).
  - Planner produces correct entries on a 4-layer variable-height fixture (AC-7).
  - Planner emits one entry per region per layer in a multi-region fixture (AC-8).
  - Tree-support live-dispatch finds branches for a non-zero `region_id` (AC-9).
  - Build cascade succeeds for every prepass module (AC-10).
  - `TASK-162` row added to `docs/07` (AC-11).
- **Negative cases:**
  - Missing `RegionMapIR` prerequisite returns `MissingRequiredPrepass`.
  - Object with empty region-map entries is skipped (no entries emitted, no error).
  - Empty `LayerPlanView.layers` returns a fatal `ModuleError` with the substring `"empty layer-plan-view"`.
- **Measurable outcomes:**
  - `prepass_support_generation_layer_plan_tdd` adds at least 5 tests, all passing.
  - `live_support_generation_tdd::planner_consuming_tier` adds at least 1 new test, passing.
  - All 7 tests in `prepass_support_generation_tdd.rs` (packet 28) continue passing.
  - All 13 tests in `live_support_generation_tdd.rs` (packets 26 + 28) continue passing.
- **Cross-packet impact:** Unblocks packet `31a_support-geometry-prepass-and-layer-height`. Adds `RegionMap` to the `PrePass::SupportGeneration` prerequisite slice — any other packet that schedules `SupportGeneration` must now also schedule `RegionMapping`.

Draft line to paste into `docs/07_implementation_status.md` under Workstream 3:

```
- [ ] TASK-162 Surface `LayerPlanIR.layers` and `RegionMapIR.entries` to the prepass guest via new WIT views (`layer-plan-view`, `region-segmentation-view`) so `support-planner` walks the real layer plan and emits one entry per `(layer, object, region)`. Closes the v1 layer-height-agnostic and single-region carve-outs from packet `28_tree-support-multi-layer-propagation`. Wired by packet `30_support-planner-prepass-wit-plumbing`.
```

## Cross-Packet Dependencies and Unblockers

- **Depends on:** `28_tree-support-multi-layer-propagation` (must be `implemented`, not `active`, when this packet activates).
- **Does not supersede:** anything. Additive correction of v1 carve-outs.
- **Unblocks:** `31a_support-geometry-prepass-and-layer-height` (which in turn unblocks `31b_support-planner-algorithmic-parity`).

## Verification Commands

```
cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture
cargo test -p support-planner --lib
bash modules/core-modules/build-core-modules.sh
bash modules/core-modules/build-core-modules.sh --check
cargo build --workspace
cargo clippy --workspace -- -D warnings
```

## Step Completion Expectations

See `implementation-plan.md` — every step carries explicit precondition, postcondition, and falsifying check.
