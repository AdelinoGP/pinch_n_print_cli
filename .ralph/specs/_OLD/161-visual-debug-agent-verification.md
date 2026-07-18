---
status: implemented
packet: 161-visual-debug-agent-verification
task_ids:
  - TASK-271
---

# 161-visual-debug-agent-verification

## Goal

Close the visual-pipeline-debug queue against its original design: implement the
remaining stage taps across all three capture mechanisms, make request selection
fail closed, add the agent skill, correct the drifted spec/IR docs, and prove the
whole surface is contract-complete, deterministic, and absent from ordinary slicing.

## Problem Statement

The visual-pipeline-debug queue (commit `a453158`) promised post-stage taps for
every scheduler stage, a fail-closed bundle contract, and an agent surface.
Packets 157-160 shipped only the seven `Layer::*` arena taps plus the standalone
`final_gcode` path, left three request-selection paths silently dropping requested
output, and never authored the agent skill; the design and IR docs also drifted
(stale `SeamPlanIR`/`RegionPlan` field names, missing `SupportGeometryIR` in
`docs/02`). TASK-271 closes the queue by finishing the tap inventory across the
three capture mechanisms, making selection fail closed, adding the skill, fixing
the docs and the packet-160 cleanup, and proving the full surface is
contract-complete, deterministic, and absent from ordinary slicing.

## Architecture Constraints

- Capture reads committed IR only and adds no module, WIT, or Blackboard API
  (ADR-0037). Blackboard slots are write-once and read-only during Tier 2; the
  Blackboard-read path clones the slot payload after `prepare_prepass_context`,
  never a live borrow. The PostPass path reads the finalized layer IRs and emitted
  `GCodeIR` without changing emission behavior.
- Three tap classes with distinct closures (ADR-0040): Blackboard-read runs prepass
  only; arena taps truncate the per-layer sequence over selected layers; PostPass
  taps run the whole-print prefix (all layers -> finalization -> postpass) and are
  the only documented deviation from the minimal-closure criterion. The manifest
  `executed_stage_ids`/`executed_layer_indices` must represent whole-print
  execution for a PostPass tap.
- Request selection fails closed (ADR-0041): no requested visualization or layer is
  ever silently omitted from a successful bundle. Layers are anonymous — `Name` is
  rejected; selection is `Index` / `{start,end}` range / z-only `Detail`.
- RegionMapping renders real `SliceIR` geometry via region-key join; LayerPlanning
  is an overlay only; no synthetic-diagram render mode (ADR-0037 amendment,
  deviation `D-161-ADR-0037-AMENDED`).
- Ordinary `pnp_cli slice` must not capture, allocate, serialize, render, spawn a
  visual-debug process, or write visual-debug artifacts.
- The skill is independent of `.claude/skills/debug-pipeline/SKILL.md`: geometry
  localization may start with visual-debug; timing/DAG/manifest diagnosis stays
  with `debug-pipeline`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Mixed units within the new taps (verified against `crates/slicer-ir/src/slice_ir.rs`):
  `ExPolygon` geometry (`SurfaceClassificationIR` footprints/quartiles,
  `SupportGeometryIR.entries`, `SliceIR` polygons, RegionMapping join) is Point2
  100 nm; `SeamPlanIR` seam point (`chosen_candidate.point`, `Point3WithWidth`),
  `SupportPlanIR.branch_segments` (`ExtrusionPath3D`), and `GCodeIR::Move` are f32
  mm. The renderer must project each in its own unit basis into the shared viewport.

## Data and Contract Notes

- **Blackboard-read taps** (source in `crates/slicer-ir/src/slice_ir.rs`):
  MeshAnalysis -> `SurfaceClassificationIR.per_object[].{bridge_regions[].xy_footprint,
  overhang_regions[].xy_footprint}` + `overhang_quartile_polygons` (Point2 100 nm);
  SeamPlanning -> `SeamPlanIR.entries[].{region_key, chosen_candidate.point (mm),
  scored_candidates[].reason}` (NOT `seam_xy`); SupportGeometry ->
  `SupportGeometryIR.{support_layer_height_mm, support_top_z_distance_mm,
  entries (Point2 100 nm)}` + `SupportPlanIR.entries[].branch_segments (mm)` with
  `SupportPlanEntry.global_layer_index: i32` (raft negatives); PaintSegmentation /
  `Layer::Slice` / `Layer::PaintRegionAnnotation`/`SlicePostProcess` ->
  `SliceIR.regions[].{polygons, infill_areas, segment_annotations}`; RegionMapping ->
  `RegionMapIR.entries` (`RegionKey`, `RegionPlan` with `config: ConfigId` via
  `config_for()`) joined to `SliceIR` on `(global_layer_index, object_id, region_id,
  variant_chain)`.
- **PostPass taps:** LayerFinalization -> finalized `Vec<LayerCollectionIR>`
  (`ordered_entities`, `travel_moves`, `tool_changes`, `z_hops`, `annotations`);
  GCodeEmit -> `GCodeIR.commands` (`GCodeCommand::Move.{x,y,z,e,f,role}`, f32 mm).
- **Manifest:** the shipped `Manifest`/`ImageEntry` (`visual_debug.rs:247/:296`)
  fields are reused; `executed_stage_ids`/`executed_layer_indices` extended to
  express whole-print PostPass closure. `warnings` (`:257/:308`) remain for
  rendered-with-caveats only; fail-closed validation means no dropped-selection warnings.
- **Determinism:** compare complete manifest and PNG bytes plus image/layer/tap/warning
  ordering; visual-debug taps create no scheduler edge or module-visible access.

## Locked Assumptions and Invariants

- No new module/WIT/Blackboard API; capture reads committed slots/arena/finalized IR.
- Layers are anonymous; `Name` is rejected, never resolved; ranges via `{start,end}`.
- The synthetic-diagram render mode does not exist; every implemented tap uses the
  geometry/overlay renderer.
- PostPass taps are the only minimal-closure deviation and record their whole-print
  closure in the manifest.
- Determinism includes manifest bytes, PNG bytes, and all ordering for both modes.
- Ordinary slice emits no visual-debug artifact or runtime signal.
- Editing `slicer-ir`/`slicer-runtime` requires `cargo xtask build-guests --check`
  before attributing any guest/host test failure to this packet.

## Risks and Tradeoffs

- **Packet is XL.** Three capture subsystems + validation + docs + skill + tests in
  one packet; preflight may require a split. If split, the natural seam is
  Blackboard-read taps + validation + docs + skill (one packet) and PostPass
  whole-print taps (a dependent packet); this design keeps them together per the
  monolithic decision but marks the seam.
- PostPass determinism runs a full slice — slower tests; keep to the smallest
  deterministic fixture and one PostPass tap in the determinism run.
- The mm-vs-100 nm split is the same hazard that produced the `TravelMove` drift;
  contract tests must pin unit handling per source, not just image existence.
