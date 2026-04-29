---
status: draft
packet: 31a_support-geometry-prepass-and-layer-height
task_ids:
  - TASK-163
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 31a_support-geometry-prepass-and-layer-height

## Goal

Establish the architectural foundation for variable-height support planning in ModularSlicer. Unlike OrcaSlicer, which ties support layer height to model layer height (wasteful for high-resolution prints), ModularSlicer already has `LayerPlanIR` before any slicing begins — enabling support planning at a different (coarser) resolution than the model. This packet introduces:

1. **`SupportGeometryIR`** — a new Tier-1 IR type holding per-layer 2D polygon outlines at support layer resolution (not model resolution).
2. **`PrePass::SupportGeometry`** — a lightweight host-built-in prepass stage that computes coarse support outlines via plane-triangle intersection at support layer boundaries only.
3. **`support_layer_height_mm` config key** — new config on both `support-planner` and `tree-support` modules, defaulting to the model layer height (no behavior change by default).
4. **`support_top_z_distance_mm` config key** — special-case refinement near the model: when a support column's top is within this distance of the model, the top few support layers use model resolution (not support resolution) so `support_top_z_distance` is honored precisely. This requires interpolation between support resolution and model resolution at the top of each column.
5. **`tree-support` module updates** — traditional supports also use `support_layer_height_mm`; the `tree-support` emitter handles the height interpolation when emitting support paths at model resolution.

After this packet, support generation can plan at coarse resolution (fast, sparse outlines) while the emitter interpolates down to model resolution for actual path planning. The foundation enables OrcaSlicer-competitive support quality at significantly reduced compute (especially for high-layer-count prints).

## Scope Boundaries

- **In scope:**
  - **`SupportGeometryIR`** in `crates/slicer-ir/src/slice_ir.rs`: keyed `(global_support_layer_index, object_id, region_id) → Vec<ExPolygon>`. Schema version `1.0.0`. Re-exported from `crates/slicer-ir/src/lib.rs`.
  - **`PrePass::SupportGeometry`** (host-built-in, not a guest module) in `crates/slicer-host/src/prepass.rs`: computes coarse polygon outlines via plane-triangle intersection at support layer boundaries. Uses `LayerPlanIR` to determine support layer boundaries (every K-th model layer where K = floor(support_layer_height / model_layer_height)). Special-case: near model contact zones, adds additional intermediate layers to honor `support_top_z_distance_mm`.
  - **`BlackboardPrepassSlot::SupportGeometry`** + `commit_support_geometry` + `support_geometry()` accessor in `crates/slicer-host/src/blackboard.rs`.
  - **`required_slots("PrePass::SupportGeneration")`** extended to include `SupportGeometry` (after RegionMap). `[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]`.
  - **WIT extension** — new `support-geometry-view-entry` + `support-geometry-view` records in `wit/world-prepass.wit` and a `support-geometry: support-geometry-view` parameter threaded into `export run-support-generation` (between `region-segmentation` and `output`).
  - **SDK types** `SupportGeometryView` + `SupportGeometryViewEntry` in `crates/slicer-sdk/src/prepass_types.rs`, re-exported from `prelude.rs`.
  - **SDK trait** `PrepassModule::run_support_generation` extended to accept `&SupportGeometryView`.
  - **Macro** `#[slicer_module]` threads the new arg.
  - **Host projector** `project_support_geometry_view` in `crates/slicer-host/src/wit_host.rs` — deterministic ordering by `(global_support_layer_index, object_id, region_id)`.
  - **`support_layer_height_mm`** added to `support-planner.toml [config.schema]` (float, default = 0.0 meaning "use model layer height", min 0.05, max 1.0) and to `modules/core-modules/tree-support/tree-support.toml`.
  - **`support_top_z_distance_mm`** added to both manifests (float, default 0.0, min 0.0, max 5.0). When > 0, the top of each support column is refined to use model resolution layers within this distance of the model contact point.
  - **`support-planner.toml [ir-access].reads`** adds `"SupportGeometryIR"`.
  - **Support interpolation** in `support-planner/src/lib.rs`: when emitting support entries, the planner interpolates from support resolution outlines down to per-model-layer entries near the top of each column (within `support_top_z_distance`). Each interpolated entry carries the model-layer Z and effective height.
  - **`tree-support` updates** in `modules/core-modules/tree-support/src/lib.rs`: `tree-support` reads `SupportGeometryIR` for collision when available; when `support_layer_height_mm > model_layer_height`, the emitter interpolates support paths from coarse support geometry to model resolution. If `SupportGeometryIR` is not committed (e.g., no support-planner loaded), `tree-support` falls back to its existing grid-MST path using model-resolution slices (existing behavior).
  - **Tests** in `crates/slicer-host/tests/support_geometry_prepass_tdd.rs` (new): verify SupportGeometryIR is produced at support resolution, verify `support_top_z_distance` refinement adds intermediate layers near model contact, verify interpolation produces correct model-layer entries from coarse outlines.
  - **Backlog** `TASK-163` row in `docs/07_implementation_status.md`.

- **Out of scope:**
  - Algorithmic features that consume `SupportGeometryIR` (avoidance/collision, radius tapering, wall-count scaling, raft, interface densification) — these are in packet 31b.
  - GUI/config wiring for `support_layer_height_mm` and `support_top_z_distance_mm` outside the module manifests.
  - Changes to the scheduler DAG order beyond inserting `PrePass::SupportGeometry` before `PrePass::SupportGeneration`.
  - Per-region support layer height (all regions use the same support layer height in this packet).
  - Soluble support material behavior.

## Prerequisites and Blockers

- **Depends on:** Packet `30_support-planner-prepass-wit-plumbing` (must be `status: implemented`).
- **Unblocks:** Packet `31b_support-planner-algorithmic-parity` (which consumes `SupportGeometryView` for avoidance/collision and the new config keys for radius taper, wall-count, raft, interface).
- **Activation blockers (must be resolved before flipping to `active`):**
  - **Q1 (resolved):** Support layer boundary computed via accumulator approach: walk `LayerPlanIR.layers` accumulating `effective_layer_height`; emit a support layer boundary when accumulated >= `support_layer_height_mm`. Catch-up layers count their full `effective_layer_height`.
  - **Q2 (resolved):** `support_top_z_distance` refinement via intermediate model-resolution layers: for each support column, add `SupportGeometryIR` entries at every model layer within `support_top_z_distance_mm` of the contact Z. Entries use `global_support_layer_index = u32::MAX` sentinel (model layer, not a support layer).
  - **Q3 (resolved):** `support_layer_height_mm = 0.0` sentinel for "use model layer height". Config schema `min > 0` ensures 0.0 is never a valid layer height.
  - **Q4 (resolved):** `SupportGeometryIR` is Tier-1-only and does not survive into Tier 2. Tree-support module falls back to grid-MST path when no `support-planner` is loaded.

## Acceptance Criteria

- **Given** `crates/slicer-ir/src/slice_ir.rs`, **when** read, **then** it defines `pub struct SupportGeometryIR { pub schema_version: SemVer, pub entries: HashMap<SupportGeometryKey, Vec<ExPolygon>> }` and `crates/slicer-ir/src/lib.rs` re-exports it. | `grep -nE 'pub struct SupportGeometryIR' crates/slicer-ir/src/slice_ir.rs && grep -nE 'SupportGeometryIR' crates/slicer-ir/src/lib.rs`
- **Given** `crates/slicer-host/src/blackboard.rs`, **when** read, **then** it contains `BlackboardPrepassSlot::SupportGeometry`, `commit_support_geometry(&self, ir: Arc<SupportGeometryIR>)`, and `fn support_geometry(&self) -> Option<Arc<SupportGeometryIR>>`. | `grep -nE 'SupportGeometry' crates/slicer-host/src/blackboard.rs`
- **Given** `crates/slicer-host/src/prepass.rs::required_slots`, **when** queried with `"PrePass::SupportGeneration"`, **then** the returned slice equals `&[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]` in that order. | `grep -nA5 '"PrePass::SupportGeneration"' crates/slicer-host/src/prepass.rs | head -8`
- **Given** `wit/world-prepass.wit`, **when** read, **then** it declares `record support-geometry-view-entry { global-support-layer-index: layer-idx, object-id: object-id, region-id: region-id, outlines: list<ex-polygon> }`, `record support-geometry-view { entries: list<support-geometry-view-entry> }`, and `export run-support-generation` carries `support-geometry: support-geometry-view` between `region-segmentation` and `output`. | `grep -nE 'record support-geometry-view-entry|record support-geometry-view\b|support-geometry: support-geometry-view' wit/world-prepass.wit`
- **Given** `modules/core-modules/support-planner/support-planner.toml`, **when** read, **then** `[config.schema]` defines `support_layer_height_mm` (float, default 0.0, min 0.05, max 1.0, display "Support Layer Height") and `support_top_z_distance_mm` (float, default 0.0, min 0.0, max 5.0, display "Support Top Z Distance"). | `grep -nE 'support_layer_height_mm|support_top_z_distance_mm' modules/core-modules/support-planner/support-planner.toml`
- **Given** `modules/core-modules/tree-support/tree-support.toml`, **when** read, **then** `[config.schema]` defines `support_layer_height_mm` and `support_top_z_distance_mm` with the same defaults and ranges. | `grep -nE 'support_layer_height_mm|support_top_z_distance_mm' modules/core-modules/tree-support/tree-support.toml`
- **Given** `modules/core-modules/support-planner/support-planner.toml`, **when** read, **then** `[ir-access].reads` includes `"SupportGeometryIR"` alongside the entries from packet 30. | `grep -nE '"SupportGeometryIR"' modules/core-modules/support-planner/support-planner.toml`
- **Given** a fixture with model layer height 0.1mm and `support_layer_height_mm = 0.3`, **when** `PrePass::SupportGeometry` runs, **then** the committed `SupportGeometryIR.entries` contains exactly floor(70 / 0.3) = 233 support layers (for a 70mm tall object) with `global_support_layer_index` ranging from 0 to 232, each carrying the union of all model slice outlines at that support layer's Z. | `cargo test -p slicer-host --test support_geometry_prepass_tdd support_geometry_produces_coarse_outlines_at_support_resolution -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** a support column whose top contact is at model layer 50 (Z = 5.0mm) and `support_top_z_distance_mm = 0.5`, **when** `PrePass::SupportGeometry` runs, **then** `SupportGeometryIR` includes additional intermediate outline entries at model layers 47, 48, 49, 50 (within 0.5mm of Z=5.0), in addition to the coarse support layer entries. | `cargo test -p slicer-host --test support_geometry_prepass_tdd support_top_z_distance_adds_refinement_layers -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support_layer_height_mm = 0.0` (sentinel for "use model layer height"), **when** `PrePass::SupportGeometry` runs, **then** the committed `SupportGeometryIR` has one entry per model layer (support resolution == model resolution), exactly as if no special support height were configured. | `cargo test -p slicer-host --test support_geometry_prepass_tdd support_layer_height_zero_uses_model_resolution -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support-planner.wasm` built after the WIT change, **when** rebuilt, **then** the build succeeds and `--check` reports it up to date. | `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -10 && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'support-planner.*up to date'`
- **Given** `docs/07_implementation_status.md`, **when** read, **then** it contains exactly one row matching `^- \[.\] TASK-163 ` whose body references `31a_support-geometry-prepass-and-layer-height`. | `grep -nE '^- \[.\] TASK-163 .*31a_support-geometry-prepass-and-layer-height' docs/07_implementation_status.md`

## Negative Test Cases

- **Given** `support_layer_height_mm = 0.03` (below the minimum of 0.05), **when** `support-planner` is loaded, **then** module load returns a config-validation error whose message contains the literal substring `"support_layer_height_mm must be >= 0.05"`. | `cargo test -p slicer-host --test support_geometry_prepass_tdd support_layer_height_below_minimum_rejects_load -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** an `ExecutionPlan` whose `prepass_stages` schedules `PrePass::SupportGeneration` before `PrePass::SupportGeometry` has committed a `SupportGeometryIR`, **when** `execute_prepass` runs, **then** it returns `PrepassExecutionError::MissingRequiredPrepass { stage_id: "PrePass::SupportGeneration", slot: BlackboardPrepassSlot::SupportGeometry }`. | `cargo test -p slicer-host --test support_geometry_prepass_tdd prepass_support_generation_fails_without_support_geometry -- --test-threads=1 --nocapture 2>&1 | tail -20`
- **Given** `support_top_z_distance_mm = -0.5` (negative), **when** `support-planner` is loaded, **then** module load returns a config-validation error whose message contains the literal substring `"support_top_z_distance_mm must be >= 0"`. | `cargo test -p slicer-host --test support_geometry_prepass_tdd negative_support_top_z_distance_rejects_load -- --test-threads=1 --nocapture 2>&1 | tail -20`

## Verification

- `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture` (regression — packet 28's tests still green)
- `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture` (regression — packet 30's tests still green)
- `cargo test -p slicer-host --test support_geometry_prepass_tdd -- --test-threads=1 --nocapture` (this packet)
- `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture` (regression)
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture` (regression)
- `cargo test -p support-planner --lib`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — Tier 1 PrePass (sequential), Tier 2 ECS layer (parallel), `LayerPlanIR` role as pre-slicing layer sequence.
- `docs/02_ir_schemas.md` — `SupportGeometryIR` shape; `LayerPlanIR.layers` for support layer boundary computation; IR Versioning Contract.
- `docs/03_wit_and_manifest.md` — prepass world, additive WIT change rebuild rule, config-schema validation.
- `docs/04_host_scheduler.md` — `PrePass::SupportGeometry` ordering before `PrePass::SupportGeneration`, `ensure_stage_prerequisites`.
- `docs/05_module_sdk.md` — config schema bounds enforcement.
- `docs/08_coordinate_system.md` — mm convention for layer heights and Z distances.
- `.ralph/specs/30_support-planner-prepass-wit-plumbing/` — WIT view pattern; `LayerPlanView` and `RegionSegmentationView` from packet 30 are used unchanged.

## OrcaSlicer Reference Obligations

- OrcaSlicer does NOT have support layer height independent from model layer height — this is a ModularSlicer differentiator. No OrcaSlicer reference required for this architectural feature.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — `SupportNode` struct (for understanding `dist_mm_to_top` and how it relates to column height).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` lines 720–800 — `generate_contact_points` and how OrcaSlicer handles contact Z (for understanding what `support_top_z_distance` refinement should approximate).

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`