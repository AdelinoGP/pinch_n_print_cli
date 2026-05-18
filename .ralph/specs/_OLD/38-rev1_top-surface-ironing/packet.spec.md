---
status: implemented
packet: top-surface-ironing-rev1
task_ids:
  - TASK-169
supersedes: 38_top-surface-ironing
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: top-surface-ironing-rev1

## Goal

Ship a new core module `top-surface-ironing` running at `PostPass::LayerFinalization` (object-scope, sequential, full `Vec<LayerCollectionIR>` visibility) that emits a low-flow zigzag pass over `TopSolidInfill` polygons on the topmost top-solid layer per region, tagged with `ExtrusionRole::Ironing` and producing `;TYPE:Ironing` G-code blocks. Mirrors Orca's `posIroning` phase (`PrintObject::ironing()` at OrcaSlicer `PrintObject.cpp:838`) which runs strictly after all per-layer infill is committed. Configurable via `ironing: bool`, `ironing_speed`, `ironing_flow`, `ironing_spacing`, `ironing_pattern`. Defaults align with OrcaSlicer (`ironing_spacing = 0.1 mm`, `ironing_flow = 0.10`, `ironing_speed = 20 mm/s`).

## Why supersede 38_top-surface-ironing

Predecessor packet `38_top-surface-ironing` placed the module at `Layer::InfillPostProcess` — a rayon-parallel per-layer stage with no cross-layer look-ahead. The packet's stated detection mechanism (the `is_top_surface` flag from packet 12-rev1) is set on `SliceRegionView` at slice time but is **not** propagated to `PerimeterRegionView` (the parameter type for `Layer::InfillPostProcess`). Consequently the predecessor's implementation fell back to using `infill_areas.is_empty()` as a proxy, which cannot distinguish topmost-of-stack from interior top-solid layers and produced a vacuous AC-TSI-3 test plus a Benchy E2E that never emitted `;TYPE:Ironing`. This rev1 packet redesigns at the correct stage (`PostPass::LayerFinalization`, sequential, full Vec<LayerCollectionIR>) where topmost-layer detection is a direct lookup, not a proxy.

## Scope Boundaries

- In scope:
  - rewrite `modules/core-modules/top-surface-ironing/{Cargo.toml, top-surface-ironing.toml, src/lib.rs, tests/top_surface_ironing_emission_tdd.rs, wit-guest/Cargo.toml, wit-guest/src/lib.rs}` to mirror the `skirt-brim` template (object-scope `FinalizationModule` trait, `run_finalization` callback, `FinalizationOutputBuilder` output channel)
  - manifest declares `stage.id = "PostPass::LayerFinalization"`, `[ir-access].reads = ["LayerCollectionIR"]`, `[ir-access].writes = ["LayerCollectionIR.ironing"]` (kebab-case sub-field; final string confirmed by Step 0 dispatch); `[hints].layer-parallel-safe = false`; `[claims].holds = []`; `[claims].requires = []`
  - module logic: scan all layers; for each `(object_id, region_key)` find the highest layer index whose region carries `TopSolidInfill` paths; compute the bounding/union ExPolygon of those paths; generate a rectilinear zigzag at `ironing_spacing` mm; emit each stroke as an `ExtrusionPath3D` with `role == ExtrusionRole::Ironing` and `flow_factor == ironing_flow`; push via `output.push_entity_to_layer(layer_index, path, region_key)`
  - rewrite `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` with object-scope fixtures (multi-layer `Vec<LayerCollectionIR>`, including the realistic interior-of-top-solid-stack case mandated by AC-TSI-3)
  - keep / update Benchy E2E test `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence`
  - bump hardcoded `core_modules_directory_is_discoverable_and_all_load` count in `crates/slicer-host/tests/manifest_ingestion_tdd.rs` from 19 to 20 (or whatever current count + 1 is); reconcile `core_modules_all_have_placeholder_wasm_flag_set` per Step 0 discovery
  - investigate and reconcile `claim_transition_matrix_tdd::stable_holder_across_layers_is_valid_for_non_transitionable_claim` regression observed in the predecessor pass
  - WASM rebuild via `./modules/core-modules/build-core-modules.sh`
  - insert `TASK-169` row into `docs/07_implementation_status.md`
  - mark predecessor `.ralph/specs/38_top-surface-ironing/packet.spec.md` `status: superseded` (planner action; no source edits owned by predecessor packet beyond its own frontmatter)
- Out of scope:
  - support-surface ironing (already shipped in `support-surface-ironing` module)
  - non-rectilinear ironing patterns (`ironing_pattern: "rectilinear"` only for v1; reject other values at config validation)
  - cooling/temperature overrides for ironing pass
  - bottom-surface or generic solid-infill ironing
  - changes to `crates/slicer-host/src/gcode_emit.rs` — predecessor Step 0 confirmed `ExtrusionRole::Ironing => ";TYPE:Ironing"` already exists at line 91
  - extending the `slicer-sdk` to expose `is_top_surface()` on `PerimeterRegionView` (the rationale for moving stages was specifically to avoid this expansion)
  - any host-stage addition (we use the existing `PostPass::LayerFinalization` stage; no new stage variants)
  - retroactive edit of the existing TASK-168 row in `docs/07_implementation_status.md` (it belongs to closed packet 36-rev1; we add a fresh TASK-169 row)

## Prerequisites and Blockers

- Depends on:
  - packet `12-rev1_external-surface-classification-at-slice` — `implemented`
  - packet `35_multi-layer-top-bottom-thickness` — `implemented` (provides `top_shell_layers` config plumbing)
  - `skirt-brim` module exists and compiles as a working `PostPass::LayerFinalization` reference template
- Unblocks: none
- Activation blockers:
  - Step 0 dispatch must confirm exact ir-access path string for the writes target (`"LayerCollectionIR.ironing"` is best-guess; the canonical kebab-case field name is set by the IR schema and the `skirt-brim` pattern uses `"LayerCollectionIR.skirt-brim"`)
  - Step 0 dispatch must confirm whether `FinalizationOutputBuilder` supports an APPEND-after-region insertion mode, or whether the host's `splice(0..0, ...)` prepend behavior in `dispatch.rs:2877` is the only available primitive (which would force ironing entities to appear before fill entities — a semantic mismatch). If only prepend is available, the packet adds a Step 0a to extend the SDK with an APPEND mode before Step 3 implementation; this remains within the rev1 packet's scope.

## Acceptance Criteria

- **Given** a `Vec<LayerCollectionIR>` with 5 layers (z = 0.0 / 0.2 / 0.4 / 0.6 / 0.8) where only layer index 4 carries one region with `TopSolidInfill` paths covering a 10 mm × 10 mm square and `top_shell_layers = 1`, **when** the module runs at `PostPass::LayerFinalization` with `ironing: true`, **then** `output.entity_pushes()` contains at least one `(layer_index, path, region_key)` tuple where `layer_index == 4`, `path.role == ExtrusionRole::Ironing`, `path.flow_factor < 0.5`, and `path.points.len() >= 4`; AND zero pushes target `layer_index ∈ {0,1,2,3}`. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd topmost_layer_emits_ironing_with_reduced_flow -- --exact --nocapture`
- **Given** a single-layer fixture whose region has only `BottomSolidInfill` paths and no `TopSolidInfill`, **when** the module runs with `ironing: true`, **then** `output.entity_pushes()` contains zero tuples whose `path.role == ExtrusionRole::Ironing`. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd non_top_solid_layer_emits_no_ironing -- --exact --nocapture`
- **Given** a 6-layer fixture with `top_shell_layers = 3` where layer indices 3, 4, and 5 ALL carry `TopSolidInfill` paths over the same XY region and only layer 5 is the topmost (no further layer above), **when** the module runs with `ironing: true`, **then** `output.entity_pushes()` contains at least one tuple with `path.role == ExtrusionRole::Ironing` and `layer_index == 5`, AND zero such tuples for `layer_index ∈ {3, 4}`. This is the substantive interior-of-stack case the predecessor packet failed to test. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd interior_top_solid_layer_emits_no_ironing -- --exact --nocapture`
- **Given** module config `ironing: false` and a fixture with realistic topmost top-solid geometry, **when** the module runs, **then** `output.entity_pushes()` is empty AND the input `Vec<LayerCollectionIR>` is bytewise unchanged across the call (verified by cloning input pre-call and comparing post-call). | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd disabled_config_emits_no_ironing_preserves_input -- --exact --nocapture`
- **Given** a topmost layer with one region whose `TopSolidInfill` paths cover a 10 mm × 10 mm square and module config `ironing_spacing = 0.1` (Orca default), **when** the module runs, **then** the union of `points.len()` across all `Ironing` pushes for that region is `>= 100` (one stroke per 0.1 mm across 10 mm). | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd ironing_spacing_controls_stroke_count -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end with `ironing: true`, **when** the slicer produces G-code, **then** the output contains at least one `;TYPE:Ironing` block AND at least one `;TYPE:Top surface` block (top-fill is preserved before the ironing pass). | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --exact --nocapture`

## Negative Test Cases

- **Given** a single-layer fixture whose region has only `BottomSolidInfill` paths (no top-surface fill at any layer), **when** the module runs, **then** `output.entity_pushes()` contains zero `Ironing`-role tuples. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd bottom_only_layer_emits_no_ironing -- --exact --nocapture`
- **Given** module config with `ironing_flow = 0.0`, **when** `on_print_start` runs, **then** it returns `Err(ModuleError::fatal(...))` whose diagnostic message contains the literal substring `ironing_flow` (zero flow would extrude nothing). | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd zero_ironing_flow_is_config_error -- --exact --nocapture`
- **Given** module config with `ironing_pattern = "concentric"`, **when** `on_print_start` runs, **then** it returns `Err(ModuleError::fatal(...))` whose diagnostic message contains the literal substring `ironing_pattern` (only `rectilinear` is supported in v1). | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd unsupported_ironing_pattern_is_config_error -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `cargo build -p top-surface-ironing`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
- `cargo test -p slicer-host --test manifest_ingestion_tdd -- --nocapture`
- `cargo test -p slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --nocapture`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only — run once at acceptance ceremony, never during implementation iterations)

## Authoritative Docs

- `docs/04_host_scheduler.md` — § PostPass / `PostPass::LayerFinalization` (lines 680–717), § Composable Multi-Writer Patterns (lines 309–317), § ir-access path syntax (lines 57–63). Read directly.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro and `FinalizationModule` trait shape. Read directly.
- `docs/02_ir_schemas.md` — `LayerCollectionIR`, `InfillRegion.solid_infill`, `InfillRegion.ironing`, `ExtrusionPath3D`, `ExtrusionRole::Ironing`. Read directly.
- `docs/03_wit_and_manifest.md` — `[ir-access]` declaration rules; manifest schema. Read directly.
- `docs/08_coordinate_system.md` — recall: 1 unit = 100 nm, NOT 1 nm. Use `Point2::from_mm` / `mm_to_units()` for all dimensional config.
- `docs/09_progress_events.md` — emit no progress events from this module unless host conventions require it (Step 0 FACT to confirm).

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_ironing` (~line 1530). Predecessor packet's Step 0 already returned a SUMMARY: zigzag at `ironing_spacing` (Orca default 0.1 mm), flow at `ironing_flow * layer_height` (Orca default 10%), speed `ironing_speed` (Orca default 20 mm/s), role `erIroning`. Re-delegate SUMMARY ≤ 200 words at Step 0 for parity confirmation if needed.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::ironing()` at line 838. FACT: confirm Orca runs ironing as a distinct phase strictly after `posInfill` and before `posSlice`-derived G-code emission (matches our `PostPass::LayerFinalization` placement).
- `OrcaSlicerDocumented/src/libslic3r/Layer.hpp` — `LayerRegion::make_ironing` declaration. FACT.

All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the spec-packet-generator's context_discipline preamble. The implementer must:

- Treat `design.md`'s code change surface as authoritative; touch nothing outside it.
- Honor `design.md`'s out-of-bounds list (no host stage additions; no SDK extensions to `PerimeterRegionView`; no OrcaSlicer reads).
- Delegate every cargo run and every OrcaSlicer reference.
- Stop reading at 60% context; hand off at 85%.

This is a stage-relocation packet, NOT a new-module packet. The module crate, test file, manifest filename, and wit-guest directory all already exist (from the superseded predecessor); the implementation pass rewrites their contents in place. The biggest implementation risk is the `FinalizationOutputBuilder` insertion-order question (prepend vs append vs targeted-after-region) — Step 0 FACT must resolve this before Step 3 implementation, and Step 0a is the contingent SDK-extension step if append support is missing.
