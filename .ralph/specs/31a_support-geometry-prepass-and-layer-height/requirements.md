# Requirements: 31a_support-geometry-prepass-and-layer-height

## Packet Metadata

- Grouped task IDs:
  - `TASK-163` (partial — architecture foundation; algorithmic features in 31b)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

OrcaSlicer ties support layer height to model layer height — so a 0.1mm miniature print gets 0.1mm support layers. This is wasteful: support structures don't need that resolution. OrcaSlicer computes `lslices` at model resolution for the whole model, then runs tree support — it can't do otherwise because slice data comes from the layer loop.

ModularSlicer has a structural advantage: `LayerPlanIR` is committed in Tier 1 (before any slicing) and describes the complete layer sequence. This means we can plan support geometry at a different (coarser) resolution than the model — even determining support layer boundaries before a single triangle is intersected.

This packet establishes the architectural foundation for this differentiator:
1. `SupportGeometryIR` — coarse per-layer polygon outlines at support layer resolution
2. `PrePass::SupportGeometry` — lightweight host-built-in prepass that computes them
3. `support_layer_height_mm` config — enables coarse support resolution
4. `support_top_z_distance_mm` config — refinement near model contact zones

After this packet, support planning runs at coarse resolution (fast, sparse outlines) while the support-planner interpolates down to model resolution near the top of each column. This is a genuine competitive advantage: OrcaSlicer-competitive support quality at significantly reduced compute.

## In Scope

- `SupportGeometryIR` in `crates/slicer-ir/src/slice_ir.rs`: keyed `(global_support_layer_index, object_id, region_id) → Vec<ExPolygon>`. Schema version `1.0.0`. Re-exported from `crates/slicer-ir/src/lib.rs`.
- `PrePass::SupportGeometry` (host-built-in) in `crates/slicer-host/src/prepass.rs`: computes coarse polygon outlines via plane-triangle intersection at support layer boundaries. Uses `LayerPlanIR.layers` to determine support layer boundaries. Adds intermediate model-resolution outline layers within `support_top_z_distance_mm` of model contact zones.
- `BlackboardPrepassSlot::SupportGeometry`, `commit_support_geometry(&Arc<SupportGeometryIR>)`, `support_geometry()` accessor in `crates/slicer-host/src/blackboard.rs`.
- `required_slots("PrePass::SupportGeneration")` extended to `[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]`.
- WIT extension: `support-geometry-view-entry` + `support-geometry-view` records in `wit/world-prepass.wit`; `support-geometry: support-geometry-view` parameter on `export run-support-generation`.
- SDK types: `SupportGeometryView`, `SupportGeometryViewEntry` in `crates/slicer-sdk/src/prepass_types.rs`, re-exported from `prelude.rs`.
- SDK trait: `PrepassModule::run_support_generation` extended to accept `&SupportGeometryView`.
- `#[slicer_module]` macro threads the new arg.
- Host projector `project_support_geometry_view` in `crates/slicer-host/src/wit_host.rs` — deterministic ordering by `(global_support_layer_index, object_id, region_id)`.
- `support_layer_height_mm` config (float, default 0.0 meaning "use model layer height", min 0.05, max 1.0) on `support-planner.toml` and `tree-support.toml`.
- `support_top_z_distance_mm` config (float, default 0.0, min 0.0, max 5.0) on both manifests.
- `support-planner.toml [ir-access].reads` adds `"SupportGeometryIR"`.
- Support interpolation: `support-planner/src/lib.rs` interpolates from coarse support resolution to per-model-layer entries near column tops (within `support_top_z_distance`). Each interpolated entry carries the model-layer Z and effective height.
- `tree-support` module: when `support_layer_height_mm > model_layer_height`, the emitter interpolates support paths from coarse support geometry to model resolution. Falls back to existing grid-MST path when `SupportGeometryIR` is not available.
- New test file `crates/slicer-host/tests/support_geometry_prepass_tdd.rs`.
- Backlog: `TASK-163` row in `docs/07_implementation_status.md`.

## Out of Scope

- Algorithmic features consuming `SupportGeometryView` (avoidance/collision, radius tapering, wall-count scaling, raft, interface densification) — in packet 31b.
- GUI/config wiring outside the module manifests.
- Per-region support layer height.
- Soluble support material.
- Changes to the scheduler DAG order beyond `PrePass::SupportGeometry` before `PrePass::SupportGeneration`.

## Authoritative Docs

- `docs/01_system_architecture.md` — Tier 1 PrePass (sequential), Tier 2 ECS layer (parallel), `LayerPlanIR` pre-slicing role.
- `docs/02_ir_schemas.md` — `SupportGeometryIR` shape, `LayerPlanIR.layers` for support boundary computation.
- `docs/03_wit_and_manifest.md` — prepass world, config-schema validation, additive WIT change rebuild rule.
- `docs/04_host_scheduler.md` — `PrePass::SupportGeometry` ordering, `ensure_stage_prerequisites`.
- `docs/05_module_sdk.md` — config schema bounds enforcement.
- `docs/08_coordinate_system.md` — mm convention for layer heights.

## Acceptance Summary

- **Positive cases:**
  - `SupportGeometryIR` defined and re-exported (AC-1).
  - Blackboard slot and commit/accessor present (AC-2).
  - `required_slots` extended to 4 entries (AC-3).
  - WIT records and `run-support-generation` shape correct (AC-4).
  - Config keys present on `support-planner.toml` (AC-5).
  - Config keys present on `tree-support.toml` (AC-6).
  - Manifest reads includes `SupportGeometryIR` (AC-7).
  - Coarse outlines produced at support resolution for 0.1mm model / 0.3mm support (AC-8).
  - `support_top_z_distance` refinement adds intermediate layers near model contact (AC-9).
  - `support_layer_height_mm = 0.0` defaults to model resolution (AC-10).
  - Build succeeds (AC-11).
  - `TASK-163` row in `docs/07` (AC-12).
- **Negative cases:**
  - `support_layer_height_mm` below minimum rejects load.
  - Missing `SupportGeometryIR` prerequisite returns error.
  - Negative `support_top_z_distance_mm` rejects load.

Draft line for `docs/07_implementation_status.md` (Workstream 3):

```
- [ ] TASK-163 (partial) Establish `SupportGeometryIR`, `PrePass::SupportGeometry`, `support_layer_height_mm`, and `support_top_z_distance_mm` as the architectural foundation for variable-height support planning. Support planner emits at coarse support resolution; emitter interpolates to model resolution near column tops. Continues TASK-120 acceptance evidence. Wired by packet `31a_support-geometry-prepass-and-layer-height`. Algorithms (avoidance, radius taper, raft, wall-count) ship in packet `31b_support-planner-algorithmic-parity`.
```

## Cross-Packet Dependencies and Unblockers

- **Depends on:** packet `30_support-planner-prepass-wit-plumbing` (must be `implemented`).
- **Does not supersede:** anything. Purely additive.
- **Unblocks:** packet `31b_support-planner-algorithmic-parity` (which consumes `SupportGeometryView` for avoidance/collision and the new config keys for radius taper, wall-count, raft, interface).

## Verification Commands

```
cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test support_geometry_prepass_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture
cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture
cargo test -p support-planner --lib
cargo build --workspace
cargo clippy --workspace -- -D warnings
```