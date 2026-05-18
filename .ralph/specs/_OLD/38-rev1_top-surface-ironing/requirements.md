# Requirements: top-surface-ironing-rev1

## Packet Metadata

- Grouped task IDs:
  - `TASK-169` (NEW; supersedes the unsuccessful TASK-168 attempt under packet 38_top-surface-ironing)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Supersedes: `38_top-surface-ironing`

## Problem Statement

The live path lacks an ironing pass over top surfaces. Orca emits a low-flow zigzag pass over the topmost top-solid layer's `TopSolidInfill` polygons (`PrintObject::ironing()` at `PrintObject.cpp:838` — runs as a distinct phase strictly after all per-layer infill is committed; analog of our `PostPass::LayerFinalization`). Without ironing, printed top surfaces show extrusion lines and inter-line gaps.

Predecessor packet `38_top-surface-ironing` attempted this at `Layer::InfillPostProcess` — wrong stage. `Layer::InfillPostProcess` is rayon-parallel per layer with no cross-layer look-ahead, and the `is_top_surface` flag set on `SliceRegionView` at slice time does not propagate to `PerimeterRegionView`. The predecessor implementation fell back to a structurally incorrect `infill_areas.is_empty()` proxy that could not distinguish topmost-of-stack from interior top-solid layers; the AC-TSI-3 test was vacuous (an empty-region fixture); and the Benchy E2E never emitted `;TYPE:Ironing`. The architectural fix — chosen by the user after spec-review — is to relocate to `PostPass::LayerFinalization` (object-scope, sequential, full `Vec<LayerCollectionIR>` visibility), where topmost-layer detection is a direct scan rather than a proxy.

This packet redesigns at the correct stage, mirroring the `skirt-brim` module (the existing object-scope reference) for skeleton, callback shape, and output mechanism. Defaults align with OrcaSlicer (`0.1 mm` / `0.10` / `20 mm/s`), eliminating the divergence flagged by the predecessor's spec-review.

## In Scope

- Rewrite (not new-create) of the existing `modules/core-modules/top-surface-ironing/` directory to use:
  - manifest stage `id = "PostPass::LayerFinalization"`
  - `[ir-access].reads = ["LayerCollectionIR"]`
  - `[ir-access].writes = ["LayerCollectionIR.ironing"]` (exact kebab-case path confirmed by Step 0; `skirt-brim` uses `"LayerCollectionIR.skirt-brim"` as the precedent)
  - `[claims].holds = []`, `[claims].requires = []`
  - `[hints].layer-parallel-safe = false`
  - `[config.schema]` with five keys and Orca-aligned defaults: `ironing: bool` (default `false`), `ironing_speed: f64` (default `20.0`), `ironing_flow: f64` (default `0.10`), `ironing_spacing: f64` (default `0.1`), `ironing_pattern: String` (default `"rectilinear"`)
- Rewrite `src/lib.rs` to:
  - implement `FinalizationModule` (the trait `skirt-brim` uses; **not** `LayerModule`)
  - `fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError>` — read all five keys; reject `ironing_flow <= 0.0` and `ironing_pattern != "rectilinear"` via `ModuleError::fatal(code, message)` whose message names the offending key
  - `fn run_finalization(&self, layers: &[LayerCollectionView], output: &mut FinalizationOutputBuilder, _config: &ConfigView) -> Result<(), ModuleError>` (exact signature from `skirt-brim/src/lib.rs:300-305`) — scan all layers; for each `(object_id, region_key)` pair find the highest layer index whose region carries `TopSolidInfill` paths; on that layer compute the union/bounding ExPolygon of those paths; generate a rectilinear zigzag at `ironing_spacing` mm in scaled units (recall: 1 unit = 100 nm, see `docs/08_coordinate_system.md`); push each stroke as one `ExtrusionPath3D` with `role == ExtrusionRole::Ironing`, `flow_factor == ironing_flow`, `speed_factor` derived from `ironing_speed`; emit via `output.push_entity_to_layer(layer_index, path, region_key)` (the `skirt-brim` precedent at `src/lib.rs:347-349`)
- Rewrite `tests/top_surface_ironing_emission_tdd.rs` with object-scope fixtures: a builder that constructs realistic `Vec<LayerCollectionIR>` stacks, including a 6-layer top_shell_layers=3 fixture where the AC-TSI-3 test substantively exercises interior-vs-topmost discrimination on real geometry
- Update `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence` if the existing implementation needs adjustment for the new stage's output channel (the test asserts on G-code text — this should not need substantive change since `;TYPE:Ironing` is produced by `gcode_emit.rs:91` regardless of stage)
- Bump the hardcoded module count in `crates/slicer-host/tests/manifest_ingestion_tdd.rs::core_modules_directory_is_discoverable_and_all_load` and reconcile `core_modules_all_have_placeholder_wasm_flag_set` per Step 0 finding
- Reconcile `crates/slicer-host/tests/claim_transition_matrix_tdd.rs::stable_holder_across_layers_is_valid_for_non_transitionable_claim`: predecessor pass observed a `MissingDependency` regression after adding the 20th module to the registry — Step 0 must determine whether this is a count-driven invariant in the test fixture (mechanical fix) or a real claim-graph bug (substantive fix)
- WASM rebuild via `./modules/core-modules/build-core-modules.sh`
- Insert `TASK-169` row into `docs/07_implementation_status.md` after acceptance ceremony
- Edit `.ralph/specs/38_top-surface-ironing/packet.spec.md` frontmatter only (`status: superseded`, `superseded_by: 38-rev1_top-surface-ironing`) — done by the planner during packet authoring, not by the implementer

## Out of Scope

- Support-surface ironing (already shipped in `modules/core-modules/support-surface-ironing/`)
- Non-rectilinear ironing patterns (concentric, etc.)
- Bottom-surface or generic solid-infill ironing
- Per-region ironing config overrides
- Variable ironing spacing across the top surface
- Adding `is_top_surface()` to `PerimeterRegionView` (the rationale for moving stages was to avoid this expansion)
- Adding a new host stage variant (we use the existing `PostPass::LayerFinalization`)
- Editing `crates/slicer-host/src/gcode_emit.rs` (predecessor's Step 0 confirmed `ExtrusionRole::Ironing => ";TYPE:Ironing"` already at line 91)
- Editing `crates/slicer-host/src/dispatch.rs` for stage routing (the dispatch site for `PostPass::LayerFinalization` already exists per `dispatch.rs:2815, 1124-1130`)
- Retroactive edits to the existing TASK-168 row at `docs/07_implementation_status.md:81` (it belongs to closed packet 36-rev1)
- Reverting any of the predecessor packet's existing files in the `top-surface-ironing/` directory before rewriting them — this packet's implementer overwrites in place

## Authoritative Docs

- `docs/04_host_scheduler.md` — `PostPass::LayerFinalization` semantics (lines 680–717), Composable Multi-Writer Patterns (lines 309–317), ir-access path syntax (lines 57–63). Read directly.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro, `FinalizationModule` trait, `FinalizationOutputBuilder` API. Read directly; delegate SUMMARY for sections > 100 lines.
- `docs/02_ir_schemas.md` — `LayerCollectionIR`, `InfillRegion.solid_infill`, `InfillRegion.ironing`, `ExtrusionPath3D`, `ExtrusionRole::Ironing`. Read directly; one section.
- `docs/03_wit_and_manifest.md` — `[ir-access]` declaration rules. Read directly.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm. Use `Point2::from_mm` / `mm_to_units()`. Read directly; small file.
- `docs/09_progress_events.md` — likely no events emitted by this module; FACT confirm.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing` (~line 1530). Delegate SUMMARY ≤ 200 words for the algorithm. (Predecessor packet has already produced this SUMMARY; reuse if available.)
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp::ironing()` (~line 838). FACT: confirm phase order (`posInfill` → `posIroning`).
- `OrcaSlicerDocumented/src/libslic3r/Layer.hpp::LayerRegion::make_ironing`. FACT.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases: see `packet.spec.md`. Cover (1) topmost layer emits Ironing path with reduced flow + ≥ 4 points; (2) non-top-solid layer emits zero; (3) **interior-of-top-solid-stack layer emits zero on real geometry** (the substantive AC-TSI-3 fix); (4) `ironing: false` emits zero AND preserves input; (5) `ironing_spacing: 0.1` over 10 mm × 10 mm produces ≥ 100 stroke points; (6) Benchy E2E produces `;TYPE:Ironing` and `;TYPE:Top surface` blocks.
- Negative cases: bottom-only layer produces zero Ironing pushes; `ironing_flow = 0.0` is a config error naming the key; `ironing_pattern = "concentric"` is a config error naming the key.
- Measurable outcomes:
  - `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd` PASS (8 tests).
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence` PASS.
  - `cargo test -p slicer-host --test manifest_ingestion_tdd` PASS.
  - `cargo test -p slicer-host --test claim_transition_matrix_tdd` PASS.
  - `cargo build --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
  - `cargo test --workspace` PASS at acceptance ceremony.
- Cross-packet impact:
  - Predecessor `38_top-surface-ironing` moves to `superseded` with `superseded_by: 38-rev1_top-surface-ironing` (planner-owned frontmatter edit during packet authoring).

## Verification Commands

- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --nocapture`
- `cargo test -p slicer-host --test manifest_ingestion_tdd -- --nocapture`
- `cargo test -p slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only)

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L).

## Context Discipline Notes

- Large files in the read-only path (delegate; do NOT load full):
  - `crates/slicer-host/src/dispatch.rs` — only the `PostPass::LayerFinalization` dispatch site (already located at line 2815, 1124-1130, 2877). FACT-narrowed reads only.
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — delegate SUMMARY only.
- OrcaSlicer trees the implementer must NOT load directly: all of `OrcaSlicerDocumented/`.
- Likely temptation reads (avoid):
  - `crates/slicer-sdk/src/views.rs` beyond the `LayerCollectionView`, `FinalizationOutputBuilder`, and `ConfigView` types
  - All other core modules beyond `skirt-brim` (the canonical PostPass template) and the existing `top-surface-ironing/` (which the implementer is rewriting)
- Sub-agent return formats:
  - cargo runs → FACT pass/fail with failing-assertion ≤ 20 lines on FAIL
  - OrcaSlicer SUMMARY → ≤ 200 words
  - Reference-template module summary → SUMMARY ≤ 300 words for `skirt-brim` skeleton
  - SDK API discovery → FACT or LOCATIONS (file:line)
