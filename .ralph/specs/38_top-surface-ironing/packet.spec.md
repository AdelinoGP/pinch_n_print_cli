---
status: draft
packet: top-surface-ironing
task_ids:
  - TASK-168
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: top-surface-ironing

## Goal

Ship a new `Layer::InfillPostProcess` core module `top-surface-ironing` that emits a low-flow zigzag pass over committed `TopSolidInfill` polygons at the topmost surface layer, tagged with `ExtrusionRole::Ironing` and producing `;TYPE:Ironing` G-code blocks. Mirrors Orca's `Layer::make_ironing` semantics. Configurable via `ironing: bool`, `ironing_speed`, `ironing_flow`, `ironing_spacing`, `ironing_pattern`.

## Scope Boundaries

- In scope:
  - new core module `modules/core-modules/top-surface-ironing/` with `manifest.toml`, `Cargo.toml`, and `src/lib.rs`
  - module declares `[ir-access].reads = ["InfillIR.regions"]` and `[ir-access].writes = ["InfillIR.regions"]` (transform chain — read-then-write establishes ordering after the fill module)
  - module logic: filter `InfillIR.regions[*].solid_infill` paths to those with `role == TopSolidInfill`; identify the bounding ExPolygon; generate a zigzag at `ironing_spacing` mm with `ironing_flow` × extrusion; tag paths `ExtrusionRole::Ironing`
  - "topmost-layer" filter: only emit ironing for regions that are **top-of-stack** (layer is the highest active layer for the object, OR the next layer above this region is missing). For multi-layer top thickness (`top_solid_layers > 1`), only the topmost layer of the top-solid stack gets ironed
  - confirm `ExtrusionRole::Ironing` already maps to `;TYPE:Ironing` in `crates/slicer-host/src/gcode_emit.rs` (FACT in Step 0); if not, add the mapping
  - new TDD coverage at module level (`top_surface_ironing_emission_tdd`) and at host E2E (Benchy with `ironing=true`)
  - WASM build for the new module via `./modules/core-modules/build-core-modules.sh`
- Out of scope:
  - support-surface ironing (already shipped in `support-surface-ironing` module)
  - non-rectilinear ironing patterns (`ironing_pattern: "rectilinear"` only for v1)
  - cooling/temperature overrides for ironing pass
  - ironing scope outside top surfaces (e.g., bottom or solid-infill ironing)
  - changing the role-to-G-code marker map beyond the `Ironing` entry (verified in Step 0)

## Prerequisites and Blockers

- Depends on:
  - packet `12-rev1_external-surface-classification-at-slice` — provides the `is_top_surface` flag
  - packet `35_multi-layer-top-bottom-thickness` — provides precise topmost-layer detection (without packet 35, every layer in a multi-layer top-solid stack would be ironed; with 35, only the topmost layer is)
- Unblocks:
  - none directly
- Activation blockers:
  - packets 12-rev1 and 35 must be `implemented`
  - Step 0 FACT confirms `ExtrusionRole::Ironing` → `;TYPE:Ironing` mapping exists; if not, scope expands by one line in `gcode_emit.rs`

## Acceptance Criteria

- **Given** a `Layer::Infill` commit producing one `TopSolidInfill` rectangle at world Z = topmost layer of an object (`is_topmost_top_surface == true`), **when** the new `top-surface-ironing` module runs as `Layer::InfillPostProcess`, **then** the resulting `InfillIR.regions[0].solid_infill` contains at least one path whose `role == ExtrusionRole::Ironing` AND whose `flow_factor < 0.5` (low-flow ironing pass) AND whose `points.len() >= 4` (at least one full zigzag stroke). | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd top_layer_emits_ironing_path_with_reduced_flow -- --exact --nocapture`
- **Given** a `Layer::Infill` commit at a NON-topmost layer (the layer below has region geometry directly above), **when** the module runs, **then** `solid_infill` contains zero `ExtrusionRole::Ironing` paths. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd non_topmost_layer_emits_no_ironing -- --exact --nocapture`
- **Given** a region with `top_solid_layers = 3` and the current layer is `topmost - 2` (i.e. top-solid layer but not the topmost layer), **when** the module runs, **then** `solid_infill` contains zero `ExtrusionRole::Ironing` paths. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd interior_top_solid_layer_emits_no_ironing -- --exact --nocapture`
- **Given** module config `ironing: false`, **when** the module runs at any layer, **then** `solid_infill` contains zero `ExtrusionRole::Ironing` paths AND existing `TopSolidInfill` paths are preserved. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd disabled_config_emits_no_ironing_preserves_input -- --exact --nocapture`
- **Given** module config `ironing_spacing = 0.2` (mm), **when** the module runs over a 10 mm × 10 mm top surface, **then** the resulting ironing path has at least 50 stroke points (10 mm / 0.2 mm spacing = 50 strokes minimum). | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd ironing_spacing_controls_stroke_count -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end with `ironing: true`, **when** the slicer produces G-code, **then** the output contains at least one `;TYPE:Ironing` block AND at least one `;TYPE:Top surface` block (top-surface fill is preserved before the ironing pass). | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --exact --nocapture`

## Negative Test Cases

- **Given** `InfillIR.regions[*].solid_infill` containing only `BottomSolidInfill` paths (no top-surface fill at this layer), **when** the module runs, **then** zero ironing paths are emitted. | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd bottom_only_layer_emits_no_ironing -- --exact --nocapture`
- **Given** module config with `ironing_flow = 0.0`, **when** the module runs, **then** validation rejects the config with a clear diagnostic naming the offending key (zero flow would extrude nothing). | `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd zero_ironing_flow_is_config_error -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/05_module_sdk.md` — `#[slicer_module]` macro, builder lifecycles, `Layer::InfillPostProcess` patterns. Read directly.
- `docs/02_ir_schemas.md` — `InfillIR.regions[*].solid_infill` (transform-chain semantics). Read directly; one section.
- `docs/04_host_scheduler.md` — § "Composable Multi-Writer Patterns" (read-then-write transform chain). Delegate SUMMARY ≤ 200 words.
- `docs/03_wit_and_manifest.md` — `[ir-access]` declaration rules. Read directly; one section.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_ironing` (line ~`1530`). Delegate SUMMARY ≤ 200 words for the algorithm + default values.
- `OrcaSlicerDocumented/src/libslic3r/Layer.hpp` — `LayerRegion::make_ironing` declaration. Delegate FACT.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::ironing()` parallel invoker. FACT for invocation order vs fill.

All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble. The implementer must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list
- delegate every cargo run and OrcaSlicer reference
- stop reading at 60% context and hand off at 85%

This is the lowest-risk packet in the chain (new module, no schema changes, no scheduler changes), but the topmost-layer detection logic depends on packet 35's `top_solid_layers` plumbing being correct.
