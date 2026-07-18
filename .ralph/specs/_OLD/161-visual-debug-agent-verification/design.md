# Design: 161-visual-debug-agent-verification

## Controlling Code Paths

- **Capture (runtime):** `crates/slicer-runtime/src/layer_executor.rs` already holds
  `SUPPORTED_TAP_STAGE_IDS` (:578), `CapturedIr` (:593), `StageCapture` (:619),
  `CaptureRequest`/`CaptureOutput` (:653/:662), `execute_captured_stages` (:779),
  and `capture_ir_for_stage` (:734) for the seven arena taps. This packet extends
  `CapturedIr` and adds two new capture entry points alongside the arena path: a
  **Blackboard-read** path reading committed slots off the `PrepassContext` from
  `crates/slicer-runtime/src/run.rs:636` (`prepare_prepass_context`) via the
  `Blackboard` accessors (`crates/slicer-runtime/src/blackboard.rs:141-271`), and a
  **PostPass whole-print** path capturing the finalized `Vec<LayerCollectionIR>`
  and `GCodeIR` around `crates/slicer-runtime/src/postpass.rs:87/:111/:135`.
- **Render (runtime):** `crates/slicer-runtime/src/visual_debug_render.rs` —
  `render_stage_capture` (:941), `compute_viewport_bounds` (:231),
  `GeometryView`/`RenderView` (:89/:101), `Canvas`/`draw_overlay` (:852),
  `swept_fill_shape` (:369) — gains geometry/overlay handling for the new
  `CapturedIr` variants, the RegionMapping `SliceIR` join, the LayerPlanning
  overlay annotation, and the mm-vs-100 nm projection split. No synthetic-diagram
  render mode is added.
- **CLI (pnp-cli):** `crates/pnp-cli/src/visual_debug.rs` — `validate_request`
  (:199), `LayerSelector` (:55), `ValidationError` (:89), `render_view_for_visualization`
  (:340), `resolve_requested_layer_indices` (:407), `run_model_source` (:446),
  `run_visual_debug` (:642), gcode branch (:689-833) — gains two-phase fail-closed
  validation, the `{start,end}` range variant, and PrePass/PostPass tap wiring.
  `crates/pnp-cli/src/visual_debug_gcode.rs` header/allow cleanup (:32-37).
- **Agent surface:** `.claude/skills/visual-debug/SKILL.md` + two examples.
- **Verification:** the five new/extended test targets named under Files in Scope.

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

## Code Change Surface

- Extend `CapturedIr` with variants for `SurfaceClassificationIR`, `SeamPlanIR`,
  a SupportGeometry composite (`SupportGeometryIR` + `SupportPlanIR`), `SliceIR`,
  a RegionMapping composite (`RegionMapIR` + joined `SliceIR`), the finalized
  `Vec<LayerCollectionIR>` (LayerFinalization), and `GCodeIR` (GCodeEmit). Each
  reports its own `schema_version` for the manifest.
- Add `SUPPORTED_TAP_STAGE_IDS` entries and the two capture entry points
  (Blackboard-read via `PrepassContext`; PostPass via the finalized/emitted IR).
  `Layer::PaintRegionAnnotation`/`SlicePostProcess`, `Layer::Slice`,
  PaintSegmentation read the `SliceIR` slot; OverhangAnnotation and MeshAnalysis
  read `SurfaceClassificationIR`.
- Renderer: geometry/overlay for the new variants; region-plan tint via
  `RegionPlan` (`config` resolved through `config_for()`); seam-point and
  branch-segment overlays in mm; LayerPlanning overlay annotation from
  `LayerPlanIR.global_layers`. No synthetic mode.
- `validate_request` two-phase: phase 1 rejects unknown kinds, `diagnostic_overlay`
  on gcode, and `Name`; phase 2 resolves `Index`/range/z-only against the schedule
  and fails closed. New `LayerSelector::Range { start, end }` with
  `#[serde(deny_unknown_fields)]`; new `ValidationError` variants.
- Cleanup: remove stale header + blanket `#![allow(dead_code)]` in
  `visual_debug_gcode.rs`; correct the `TravelMove` doc comment.
- Docs: add `SupportGeometryIR` to `docs/02_ir_schemas.md`; correct the
  `docs/specs/visual-pipeline-debug.md` tap inventory (SeamPlanIR/RegionPlan
  fields, mm-unit flags, LayerPlanning->overlay, RegionMapping->join).
- Rejected alternatives: forcing PrePass/PostPass taps through
  `execute_captured_stages` (wrong boundary); a synthetic-diagram render mode
  (the ADR-0037 amendment removes it); warn-and-continue validation (ADR-0041 rejects it);
  named-layer resolution (no name concept exists).

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/layer_executor.rs` - `CapturedIr` variants, tap ids, Blackboard-read + PostPass capture entry points.
- `crates/slicer-runtime/src/visual_debug_render.rs` - new-variant geometry/overlay, RegionMapping join, LayerPlanning overlay, mm/100 nm projection.
- `crates/slicer-runtime/src/postpass.rs` - read-only capture hook for finalized `Vec<LayerCollectionIR>` and `GCodeIR`; no emission-behavior change.
- `crates/pnp-cli/src/visual_debug.rs` - two-phase fail-closed validation, `Range` selector, PrePass/PostPass wiring, render dispatch for new taps.
- `crates/pnp-cli/src/visual_debug_gcode.rs` - remove stale header and blanket `#![allow(dead_code)]`.
- `crates/slicer-ir/src/slice_ir.rs` - `TravelMove` doc comment only (mm, not 100 nm); no type/field change.
- `docs/02_ir_schemas.md` - add normative `SupportGeometryIR` (+ `SupportGeometryKey`) definition.
- `docs/specs/visual-pipeline-debug.md` - correct drifted tap-inventory field names, LayerPlanning row, RegionMapping description.
- `docs/19_visual_debug.md` - agent guide updates for selectors, fail-closed behavior, and the completed tap set.
- `.claude/skills/visual-debug/SKILL.md`, `.claude/skills/visual-debug/examples/model-backed.md`, `.claude/skills/visual-debug/examples/standalone-gcode.md` - agent surface.
- `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`, `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs`, `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs`, `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs` - runtime tests.
- `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`, `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs` - CLI tests.

May reuse a minimal existing test fixture constructor if a confirmed accessor requires it; do not add a new geometry fixture generator.

**Test-target wiring (S7):** every new `*_tdd.rs` above goes at the **top level** of `crates/<crate>/tests/` so `cargo test -p <crate> --test <file_stem>` addresses it as its own standalone binary (matching the `arachne_*` precedent in `slicer-runtime/tests/` and all existing `visual_debug_*_tdd.rs` in `pnp-cli/tests/`). Do **not** place them inside an aggregated bucket subdir (`slicer-runtime/tests/{contract,unit,executor,integration,e2e}/`): a file dropped there without a `mod` line in that bucket's `main.rs` silently never compiles and the AC's `--test` filter reports a false "0 tests run".

## Out-of-Bounds Files

- `crates/slicer-schema/wit/**`, module manifests, `modules/**`, guest shim sources, and any new module-visible API - capture reads committed slots/arena only (ADR-0037).
- Scheduler DAG/edge definitions and ordinary `pnp_cli slice` production flow - no behavior change; no visual-debug capture in the ordinary path.
- G-code emission logic in `postpass.rs` beyond a read-only capture hook - do not alter what is emitted.
- Coordinate-system helpers/constants in `slicer-core`/`slicer-helpers` - use existing helpers; invent no new conversion math.
- `OrcaSlicerDocumented/` - no parity scope; do not load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, broad test output - never load or edit.

## Expected Sub-Agent Dispatches

- Question: exact field accessors + `schema_version` for each new `CapturedIr` source type (`SurfaceClassificationIR`, `SeamPlanIR`, `SupportGeometryIR`/`SupportPlanIR`, `SliceIR`, `RegionMapIR`, `GCodeIR`, `LayerCollectionIR`). Scope: `crates/slicer-ir/src/slice_ir.rs` only. Return: `SNIPPETS` at most 3, 30 lines each.
- Question: the exact `PrepassContext`/`Blackboard` accessor names and the `execute_postpass` capture point that expose committed slots and finalized/emitted IR read-only. Scope: `run.rs`, `blackboard.rs`, `postpass.rs`. Return: `LOCATIONS` at most 20 entries.
- Question: smallest existing deterministic visual-debug fixture reusable for a whole-print PostPass determinism run. Scope: the named test directories only. Return: `LOCATIONS` at most 20 entries.
- Question: do the focused contract/validation/determinism/overhead tests, `build-guests --check`, all-target check, and clippy pass. Scope: repository commands only. Return: `FACT` in 5 lines or fewer.

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

## Context Cost Estimate

- Aggregate: `L`
- Largest step: capture extension (Step 3, `L` — split by tap family in the plan).
- Highest-risk dispatch and return format: exact new-`CapturedIr` field/schema
  inventory from `slice_ir.rs`; `SNIPPETS` at most 3, 30 lines each.

## Open Questions

- No `[FWD]` blockers remain — all consumed seams are grounded landed symbols.
- [FWD] Preflight-scoped: if `spec-review --preflight` rules the aggregate `L`
  surface non-atomic, split at the Blackboard-vs-PostPass seam noted under Risks
  before activation. This is a packaging question for preflight, not a missing API.
