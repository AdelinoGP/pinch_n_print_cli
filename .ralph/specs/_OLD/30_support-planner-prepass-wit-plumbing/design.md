# Design: 30_support-planner-prepass-wit-plumbing

## Controlling Code Paths

- **Primary code paths:**
  - `wit/world-prepass.wit` — add records and extend `run-support-geometry` parameters.
  - `crates/slicer-sdk/src/prepass_types.rs` — add host-side `LayerPlanView` / `RegionSegmentationView` types.
  - `crates/slicer-sdk/src/prelude.rs` — re-export new types.
  - `crates/slicer-sdk/src/traits.rs` — extend `PrepassModule::run_support_geometry` signature.
  - `crates/slicer-macros/src/lib.rs` — thread the two new args from the generated WIT shim into the trait method when stage id is `PrePass::SupportGeometry`.
  - `crates/slicer-host/src/wit_host.rs` — project `LayerPlanIR.layers` and `RegionMapIR.entries` into the WIT views before invoking the export.
  - `crates/slicer-host/src/prepass.rs` — extend `required_slots` with `BlackboardPrepassSlot::RegionMap`.
  - `modules/core-modules/support-planner/support-planner.toml` — manifest reads list.
  - `modules/core-modules/support-planner/src/lib.rs` — replace bounding-box layer derivation with `layer_plan_view.layers` walk; emit one entry per region from `region_segmentation_view`.
  - `modules/core-modules/support-planner/wit-guest/src/lib.rs` — regenerate after WIT change.
  - All other prepass `wit-guest/` shims — recompile only (their export signatures are unaffected).
- **Neighboring tests or fixtures:**
  - `crates/slicer-host/tests/prepass_support_generation_tdd.rs` (packet 28) — must remain green.
  - `crates/slicer-host/tests/live_support_generation_tdd.rs` (packets 26 + 28) — must remain green; `planner_consuming_tier` module gets one new test.
  - new file: `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs`.
- **OrcaSlicer comparison surface:** None new. WIT plumbing is repo-internal.

## Architecture Constraints

- **Additive WIT change rebuild cascade.** Every prepass `wit-guest/` and resulting `.wasm` must be rebuilt because the WIT package version is `slicer:world-prepass@1.0.0` and changing any export shape inside that package invalidates all bindings in the package. Existing prepass modules' export signatures (`run-mesh-segmentation`, `run-mesh-analysis`, `run-layer-planning`, `run-paint-segmentation`, `run-seam-planning`) are not changed by this packet, but the package as a whole must compile cleanly under the extended `run-support-geometry` shape. We do not bump the WIT package version (still `1.0.0`); per `docs/03_wit_and_manifest.md` §additive change rule, adding parameters to one export is treated as an additive minor revision so long as it ships before any external consumer locks the v1.0.0 shape. The package's authoritative version line (`package slicer:world-prepass@1.0.0;`) is unchanged; the new records and parameters carry no version annotation.
- **`ensure_stage_prerequisites` single source of truth.** Adding `RegionMap` to the prerequisite slice is the only place the new dependency is declared. The planner module does not duplicate the check — it trusts that the slot is committed when its export is invoked.
- **Blackboard slots remain write-once.** No new slot is introduced; this packet only adds a read of the existing `RegionMap` slot via the host-side projector.
- **Manifest reads must include every IR the planner reads at runtime.** `LayerPlanIR` and `RegionMapIR` move into `[ir-access].reads` because the host now projects their data through the WIT into the guest. The host-side audit (`runtime_reads ⊆ manifest_reads`) must continue to pass.
- **Coordinate system invariant.** `LayerPlanIR.layers[*].z` is mm-valued (per `docs/02_ir_schemas.md`). The WIT projector passes Z values through unchanged. The planner emits `Point3WithWidth.z` in mm to match the existing contract.
- **Determinism.** `RegionMapIR.entries` is a `HashMap` — projection into the WIT view must sort by `(global_layer_index ASC, object_id ASC, region_id ASC)` so two identical runs produce byte-identical `LayerPlanView` and `RegionSegmentationView` inputs and therefore byte-identical `SupportPlanIR.entries`. Packet 28's `support_planner_is_deterministic_across_runs` test must continue to pass.

## Code Change Surface

### Selected approach

**Project committed prepass IRs into purpose-built WIT views; thread them into one new `run-support-geometry` parameter list.**

The host computes `LayerPlanView` from `Blackboard::layer_plan().layers` and `RegionSegmentationView` from `Blackboard::region_map().entries` in `wit_host.rs::project_for_support_generation` (new helper) just before calling the WIT export. The guest receives both views as arguments to `run_support_geometry`. The planner uses `LayerPlanView.layers.len()` as `num_layers` and `LayerPlanView.layers[i].z` as the entry Z. For per-region emission, the planner iterates `RegionSegmentationView.entries` filtered to `(global_layer_index == current_layer)` and creates one `SupportPlanEntry` per `region_id`, copying the same `branch_segments` set under each region.

The planner stops walking layers above the maximum `LayerPlanView.layers[*].global_layer_index` and below 0, instead of deriving a count from bounding-box height.

### Exact functions, traits, manifests, tests, or fixtures expected to change

**Created:**
- `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` — 5 tests:
  - `planner_walks_real_layer_plan_with_variable_layer_heights` (positive)
  - `planner_emits_one_entry_per_region_in_region_map` (positive)
  - `prepass_support_generation_fails_without_region_map` (negative)
  - `planner_skips_object_with_empty_region_map` (negative)
  - `host_projector_orders_region_segmentation_deterministically` (determinism)

**Modified — WIT and SDK:**
- `wit/world-prepass.wit` — add 4 records, extend `run-support-geometry` signature.
- `crates/slicer-sdk/src/prepass_types.rs` — add 4 structs.
- `crates/slicer-sdk/src/prelude.rs` — re-export 4 new types.
- `crates/slicer-sdk/src/traits.rs` — extend `PrepassModule::run_support_geometry` signature.
- `crates/slicer-macros/src/lib.rs` — extend the prepass-stage routing arm for `PrePass::SupportGeometry` to include the two new args.

**Modified — host:**
- `crates/slicer-host/src/wit_host.rs` — add `project_layer_plan_view`, `project_region_segmentation_view`, and call them in the `run-support-geometry` dispatcher path. Order projection deterministically.
- `crates/slicer-host/src/prepass.rs` — extend `required_slots("PrePass::SupportGeometry")` with `BlackboardPrepassSlot::RegionMap`.
- `crates/slicer-host/src/dispatch.rs` — no behaviour change expected; verify no exhaustive-match site needs updating after the WIT change.

**Modified — modules:**
- `modules/core-modules/support-planner/support-planner.toml` — `[ir-access].reads` reordered to `["MeshIR", "SurfaceClassificationIR", "LayerPlanIR", "RegionMapIR", "PaintRegionIR"]`; remove the v1 layer-height-agnostic comment block.
- `modules/core-modules/support-planner/src/lib.rs`:
  - Module-level docs: drop the v1 layer-height-agnostic and single-region bullets; rewrite to reflect the new contract.
  - Remove constant `DEFAULT_LAYER_HEIGHT_MM`.
  - Change `run_support_geometry` signature to `(&self, objects, layer_plan, region_segmentation, output, config)`.
  - In `plan_for_object`, replace the bounds-derived `num_layers` / `layer_height` block with `layer_plan.layers` indexing.
  - Replace the hard-coded `region_id: "0".to_string()` site with a loop over `region_ids` for the current `(layer, object)`.
  - Add an early `return Err(ModuleError::fatal(_, "empty layer-plan-view"))` if `layer_plan.layers.is_empty()`.
- `modules/core-modules/support-planner/wit-guest/src/lib.rs` — regenerate stub re-export.
- All other `modules/core-modules/*/wit-guest/` shims — recompile only (no source change).
- `modules/core-modules/build-core-modules.sh` — no source change; verify cascade rebuilds every prepass `.wasm`.

**Modified — backlog:**
- `docs/07_implementation_status.md` — append `TASK-162` row under Workstream 3.

### Rejected alternatives

- **Add LayerPlanIR/RegionMapIR as resources with accessor methods (mirroring `paint-segmentation-output`'s push pattern).** Rejected because the views are read-only (the planner does not write back) and a passive record carries less binding surface than a resource. Records are also cheaper to project deterministically.
- **Bump WIT package version to `slicer:world-prepass@1.1.0`.** Rejected because no external consumer locks the v1.0.0 shape; an internal additive change does not justify the rebuild cost of every dependent crate's package manifest. Revisit if/when the WIT package is ever published outside this workspace.
- **Compute per-layer slice polygons inside the planner via plane-triangle intersection on `MeshObjectView`.** Rejected — duplicates work, costs determinism, and is a poor fit for limit (2) which is solved by reading `RegionMapIR` keys, not by re-slicing.

## Data and Contract Notes

- **WIT records:**
  - `record layer-plan-view-entry { global-layer-index: layer-idx, z: f32, effective-layer-height: f32 }`
  - `record layer-plan-view { layers: list<layer-plan-view-entry> }`
  - `record region-segmentation-view-entry { object-id: object-id, layer-index: layer-idx, region-ids: list<region-id> }`
  - `record region-segmentation-view { entries: list<region-segmentation-view-entry> }`
- **Export signature:** `export run-support-geometry: func(objects: list<mesh-object-view>, layer-plan: layer-plan-view, region-segmentation: region-segmentation-view, output: support-geometry-output, config: config-view) -> result<_, module-error>;`
- **Determinism contract:** the host projector iterates `RegionMapIR.entries` after sorting keys by `(layer_index, object_id, region_id)` (all ascending). Two consecutive runs must produce byte-identical `LayerPlanView` and `RegionSegmentationView` inputs. The packet 28 determinism test (`support_planner_is_deterministic_across_runs`) must continue to pass.
- **Branch geometry vs region keying:** the planner emits the same `branch_segments` `Vec<ExtrusionPath3D>` under each region the object owns on a layer. Tree-support's `support_plan_segments_for(object, region)` lookup matches by `region_id` so any region the object owns will find branches.
- **Host-side prerequisite chain:** `required_slots("PrePass::SupportGeometry")` becomes `&[SurfaceClassification, LayerPlan, RegionMap]`. Order matters for the negative-AC error message (the host reports the first missing slot). Existing AC-4 from packet 28 (`prepass_support_generation_fails_without_layer_plan`) must still pass — it uses a fixture missing `LayerPlanIR`, and `LayerPlan` precedes `RegionMap` in the slice.

## Locked Assumptions and Invariants

1. `LayerPlanIR.layers` is committed with non-empty `layers` whenever `LayerPlanning` runs successfully. The host audit asserts this; the planner can therefore treat an empty `LayerPlanView.layers` as a host-internal error and return `ModuleError::fatal`.
2. `RegionMapIR.entries` keys are unique `(layer_index, object_id, region_id)` triples (per `docs/02_ir_schemas.md`).
3. `region-id` is a string at the WIT layer (`type region-id = string;` in `wit/deps/ir-types.wit`) and a `u64` in the host IR; the existing `parse_canonical_region_id` helper in `crates/slicer-host/src/dispatch.rs` round-trips both. The host projector emits canonical decimal strings for region IDs in `RegionSegmentationView` and the planner passes them straight through to the output.
4. Packet 28's claim layout (`support-planner` claim on `PrePass::SupportGeometry`) is unchanged. No other packet holds that claim.
5. The packet 26 and packet 28 acceptance tests (Benchy + tree-support fallback) still exercise the grid-MST fallback path when no `support-planner` is loaded — this packet does not change the fallback path.
6. No IR schema versions bump. Adding fields to a WIT view that only `support-planner` consumes does not affect any persisted IR.

## Risks and Tradeoffs

- **Risk: rebuild cascade slows local builds.** Every prepass module's `wit-guest/` rebuilds when the WIT package's binding output changes. Mitigation: order Step 6 (rebuild cascade) at the end of the implementation plan so the host code path is verified first; one rebuild covers all modules.
- **Risk: host projector ordering bug breaks determinism.** `HashMap` iteration is non-deterministic. Mitigation: explicit sort by `(layer, object, region)` in the projector with a determinism test asserting byte-identical projections across two runs.
- **Risk: a prepass module other than `support-planner` accidentally accepts the new args via the macro.** Mitigation: the macro routes args based on `stage.id`; the `PrepassModule::run_support_geometry` default body returns `unimplemented`, so any module declaring a different stage id never sees the new args. Verify with a build of every existing prepass core module.
- **Tradeoff: branch geometry is still object-wide.** This is correct for single-region objects (the common case) and produces over-emission (duplicate segments per region) for multi-region objects. Geometry-aware multi-region placement is deferred to packet `31b` because it requires per-layer slice polygons that this packet does not surface.
- **Tradeoff: fatal `ModuleError` on empty `LayerPlanView`.** A bug elsewhere in the host that produces an empty layer plan now turns into a hard module failure rather than a silent no-op. This is intentional — silent no-ops are the v1 behavior we are correcting.

## Open Questions

All open questions are resolved.

- **Q1: WIT package version bump?** Resolved — stay on `slicer:world-prepass@1.0.0`. No external consumer locks the v1.0.0 shape; additive change documented in `docs/03_wit_and_manifest.md` rebuild rule.
- **Q2: Where does region-segmentation projection live?** Resolved — `crates/slicer-host/src/wit_host.rs::project_region_segmentation_view`, alongside the existing prepass projectors. Sorted deterministically by `(layer, object, region)`.
- **Q3: How does the planner handle objects with no regions in `RegionMapIR`?** Resolved — skip the object entirely (no entries emitted, no error). Negative AC covers this case.
- **Q4: Does this packet change packet 28's existing tests?** Resolved — packet 28 tests use uniform 0.2 mm layer heights and single-region fixtures; they continue to pass with no test changes. The new variable-height + multi-region cases live in a new test file.
- **Q5: Does `region-id` string canonical form match the host's `RegionId` round-trip?** Resolved — yes, `parse_canonical_region_id` in `dispatch.rs` already handles canonical decimal strings and is called from `harvest_support_plan_ir`. The projector emits the same canonical decimal form.
