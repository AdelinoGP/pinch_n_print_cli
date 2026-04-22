# Design: 23_prepass-seam-planning-orca-parity

## Controlling Code Paths

- Prepass stage ordering is defined in `crates/slicer-host/src/execution_plan.rs::STAGE_ORDER`
- Prepass required-slot validation and blackboard commit routing live in `crates/slicer-host/src/prepass.rs`
- Prepass storage lives in `crates/slicer-host/src/blackboard.rs`
- Manifest stage validation lives in `crates/slicer-host/src/manifest.rs`
- Prepass dispatch/export routing lives in `crates/slicer-host/src/dispatch.rs`
- Current prepass WIT surface lives in `wit/world-prepass.wit`
- Current prepass SDK surface lives in `crates/slicer-sdk/src/traits.rs`, `prepass_builders.rs`, and `prepass_types.rs`
- Macro-generated prepass glue lives in `crates/slicer-macros/src/lib.rs::build_prepass_world_glue`
- The current apply-stage seam logic remains in `modules/core-modules/seam-placer/src/lib.rs`

## Architecture Constraints

- The new stage must slot into the fixed prepass order without breaking existing `required_slots(...)` rules
- `SeamPlanIR` must be write-once on the blackboard like the other prepass artifacts
- The packet must preserve the existing `seam-placer` claim semantics from `docs/01_system_architecture.md`; the new planner module is therefore claim-free in this design
- The layer-stage WIT world stays stable for packet `23`; planned seams are injected into the existing `PerimeterRegionView.resolved_seam` surface rather than adding a second layer-world handle

## Selected Implementation Approach

Add a dedicated prepass artifact and a new prepass module, but keep the apply stage stable.

1. Add `SeamPlanIR { schema_version, entries: Vec<SeamPlanEntry> }`
2. Add `PrePass::SeamPlanning` immediately after `PrePass::LayerPlanning`
3. Extend `world-prepass.wit` with `seam-planning-output` and `run-seam-planning`
4. Add `seam-planner-default` as a new prepass core module that scores seam candidates using current prepass host-services and emits `SeamPlanEntry` records
5. In layer dispatch, look up `SeamPlanIR.entries[*]` by `RegionKey` and populate the matching `PerimeterRegionView.resolved_seam` before calling `seam-placer`

This keeps packet `22` valid, avoids a second layer-world seam handle, and confines the new architecture to the prepass boundary where it belongs.

## Rejected Alternatives

- Extend `LayerPlanIR` with inline seam-planning fields.
  Rejected because `LayerPlanIR` already owns Z-plane participation and config merge; adding seam scoring there would conflate planner responsibilities and widen every existing consumer.
- Move `com.core.seam-placer` itself to `PrePass::SeamPlanning`.
  Rejected because the live apply stage still needs a module boundary, and changing claim ownership mid-slice would create avoidable claim-policy drift.
- Add a brand-new WIT world just for seam planning.
  Rejected because the repo already has a dedicated prepass world, prepass SDK builders, and prepass macro glue; extending them is the narrower change.

## Explicit Code Change Surface

- `crates/slicer-ir/src/slice_ir.rs`
  - add `SeamPlanEntry`
  - add `SeamPlanIR`
- `crates/slicer-ir/src/lib.rs`
  - re-export the new IR types
- `crates/slicer-host/src/blackboard.rs`
  - add `seam_plan: Option<Arc<SeamPlanIR>>`
  - add `BlackboardPrepassSlot::SeamPlan`
  - add `commit_seam_plan(...)` and `seam_plan()`
- `crates/slicer-host/src/prepass.rs`
  - add `PrepassStageOutput::SeamPlan`
  - add `required_slots("PrePass::SeamPlanning")`
  - add `commit_stage_output` arm for `SeamPlanIR`
- `crates/slicer-host/src/execution_plan.rs`
  - insert `"PrePass::SeamPlanning"` into `STAGE_ORDER`
- `crates/slicer-host/src/manifest.rs`
  - allow the new stage id during manifest ingestion
- `crates/slicer-host/src/dispatch.rs`
  - route `"PrePass::SeamPlanning"` to `run-seam-planning`
  - convert collected seam-plan output to `SeamPlanIR`
  - inject planned seams into the matching `PerimeterRegionView` before `Layer::PerimetersPostProcess`
- `wit/world-prepass.wit`
  - add `seam-planning-output`
  - add `run-seam-planning`
- `crates/slicer-sdk/src/prepass_types.rs`
  - add SDK-side seam-plan records
- `crates/slicer-sdk/src/prepass_builders.rs`
  - add `SeamPlanningOutput`
- `crates/slicer-sdk/src/traits.rs`
  - add `run_seam_planning(...)`
- `crates/slicer-sdk/src/prelude.rs` and `crates/slicer-sdk/src/lib.rs`
  - export the new prepass types/builders
- `crates/slicer-macros/src/lib.rs`
  - extend `build_prepass_world_glue` for the new export and builder drain
- `modules/core-modules/seam-planner-default/`
  - new manifest, source, and tests
- `modules/core-modules/seam-placer/src/lib.rs`
  - simplify to apply-only behavior after packet `22`
- `crates/slicer-host/tests/dispatch_tdd.rs`
  - prepass commit, duplicate-key rejection, and missing-slot tests
- `crates/slicer-host/tests/execution_plan_tdd.rs`
  - stage-order and manifest-validation tests
- `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`
  - manifest contract tests for `seam-planner-default`
- `crates/slicer-host/tests/live_seam_path_tdd.rs`
  - seam-plan injection test
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
  - seam-plan-to-live-wall evidence test

## Data and Contract Notes

- `SeamPlanIR.entries[*].region_key.{global_layer_index, object_id, region_id}` is the stable join key between prepass planning and the layer apply stage
- `SeamPlanIR.entries[*].chosen_candidate` uses the existing `SeamPosition` shape so the layer stage can reuse current seam application code
- `SeamPlanIR.entries[*].scored_candidates` keeps the scored candidate list for debugging, evidence, and deterministic regression checks
- Layer-stage injection happens before the guest sees `PerimeterRegionView`, so the apply module still reads `resolved_seam()` through the existing WIT surface

## Risks and Tradeoffs

- Adding a new prepass stage widens scheduler, manifest, blackboard, and SDK surfaces in one packet; this is why the packet remains draft until explicitly scheduled
- The planner module is claim-free by design, which preserves current `seam-placer` claim stability but means claim policy for prepass seam planning is intentionally deferred
- Using the existing prepass host-services may be slower than a future host-precomputed seam-visibility map, but it keeps the packet inside current host-service capabilities

## Open Questions

- No packet-local backlog ambiguity remains now that `docs/07_implementation_status.md` tracks this slice under `TASK-159`; the packet stays `draft` only because packet `22` is a prerequisite and packet `15` is still active

## Locked Assumptions and Invariants

- packet `22` lands first and becomes the apply-stage baseline
- `SeamPlanIR` is write-once on the blackboard and keyed by deterministic `RegionKey`
- the layer-world WIT surface remains stable; prepass planning is injected into the existing `resolved_seam` view
