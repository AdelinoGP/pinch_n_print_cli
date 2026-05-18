# Design: manifest-ir-access-and-config-schema

## Controlling Code Paths

- Primary code path: `modules/core-modules/**/manifest.toml` (17 files)
- Neighboring tests or fixtures: `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`
- OrcaSlicer comparison surface: None

## Architecture Constraints

- Each module's `[ir-access]` is determined solely by its declared stage (`[stage].id`).
- The 17 core modules and their stages:
  - `mesh-segmentation` → `PrePass::MeshSegmentation`
  - `paint-segmentation` → `PrePass::PaintSegmentation`
  - `layer-planner-default` → `PrePass::LayerPlanning`
  - `classic-perimeters` / `arachne-perimeters` → `Layer::Perimeters`
  - `seam-placer` → `Layer::PerimetersPostProcess`
  - `rectilinear-infill` / `gyroid-infill` / `lightning-infill` → `Layer::Infill`
  - `fuzzy-skin` → `Layer::PerimetersPostProcess`
  - `paint-region-annotator` → `Layer::SlicePostProcess`
  - `traditional-support` / `tree-support` → `Layer::Support`
  - `support-surface-ironing` → `Layer::SupportPostProcess`
  - `skirt-brim` / `wipe-tower` → `PostPass::LayerFinalization`
  - `path-optimization-default` → `Layer::PathOptimization`

## Proposed Changes

1. **Inventory all 17 core-module TOML files** and record current `reads = []` / `writes = []` / `config.schema` state.
2. **For each module, apply the Stage I/O Contract table** from `docs/01_system_architecture.md` to populate reads and writes.
3. **Add config.schema fields** from the module's actual config keys used in its source (`src/` or `wit-guest/src/`).
4. **Mark tests as the gate**: `core_module_ir_access_contract_tdd.rs` enumerates the expected contract per module; green = done.

## Data and Contract Notes

- IR paths must exactly match field names in `crates/slicer-ir/src/` (e.g., `SliceIR.regions.infill_areas`, not `SliceIR.regions.infill-areas`).
- Paint region reads must include semantic-specific paths or `PaintRegionIR.Custom.<module-id>` for custom semantics.
- `layer-parallel-safe = false` must be set on finalization modules; the TOML template already has it but it must be verified.
- Config schema `type` must be one of: `"bool"`, `"int"`, `"float"`, `"string"`, `"enum"`, `"float-list"`, `"string-list"`.

## Risks and Tradeoffs

- Some modules may have config fields in source that are not yet documented in the schema reference. Use the fields that are clearly present; leave unknown ones as `[config.schema]` (empty) until they are confirmed.
- Stage I/O Contract table may not cover every nuance of a module's actual IR usage — use the table as the authoritative baseline, not a substitute for reading the source.

## Open Questions

- Does `core_module_ir_access_contract_tdd.rs` fully enumerate all 17 modules, or does it only cover a subset? All modules.
- Are there any core modules that are host-built-in (not WASM) and therefore do not need manifest declarations? No.
- Should `path-optimization-default` declare read access to `PerimeterIR`, `InfillIR`, and `SupportIR` per the Stage I/O Contract? Yes
