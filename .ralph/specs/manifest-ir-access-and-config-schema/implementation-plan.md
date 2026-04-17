# Implementation Plan: manifest-ir-access-and-config-schema

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Audit current manifest state

- Task IDs:
  - `TASK-121`
  - `TASK-122`
- Objective: Enumerate all 17 core-module TOML files, record their current `[ir-access]` and `[config.schema]` state, confirm which modules are covered by `core_module_ir_access_contract_tdd.rs`.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/01_system_architecture.md` — Stage I/O Contract table
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema
- OrcaSlicer refs: None
- Verification: `grep -r 'reads\s*=\s*\[\]' modules/core-modules/**/*.toml | wc -l` (count before) and `grep -r 'config.schema' -l modules/core-modules/**/*.toml | wc -l`

### Step 2: Populate ir-access for PrePass modules

- Task IDs:
  - `TASK-121`
- Objective: Populate `[ir-access].reads` and `[ir-access].writes` for `mesh-segmentation`, `paint-segmentation`, `layer-planner-default`.
- Files expected to change:
  - `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`
  - `modules/core-modules/paint-segmentation/paint-segmentation.toml`
  - `modules/core-modules/layer-planner-default/layer-planner-default.toml`
- Authoritative docs:
  - `docs/01_system_architecture.md` — Stage I/O Contract table rows for PrePass stages
  - `docs/02_ir_schemas.md` — IR field path names
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test core_module_ir_access_contract_tdd -- --nocapture 2>&1 | grep -E 'mesh-segmentation|paint-segmentation|layer-planner'`

### Step 3: Populate ir-access for per-layer modules

- Task IDs:
  - `TASK-121`
- Objective: Populate `[ir-access]` for all Layer-stage modules (Perimeters, Infill, Support, PathOptimization, SlicePostProcess, PerimetersPostProcess, SupportPostProcess).
- Files expected to change:
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
  - `modules/core-modules/seam-placer/seam-placer.toml`
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml`
  - `modules/core-modules/lightning-infill/lightning-infill.toml`
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`
  - `modules/core-modules/paint-region-annotator/paint-region-annotator.toml`
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/tree-support/tree-support.toml`
  - `modules/core-modules/support-surface-ironing/support-surface-ironing.toml`
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml`
- Authoritative docs:
  - `docs/01_system_architecture.md` — Stage I/O Contract table rows for Layer stages
  - `docs/02_ir_schemas.md` — IR field path names
- OrcaSlicer refs: None
- Verification: Same `core_module_ir_access_contract_tdd` test, filter for per-layer module names.

### Step 4: Populate ir-access for PostPass finalization modules

- Task IDs:
  - `TASK-121`
- Objective: Populate `[ir-access]` for `skirt-brim` and `wipe-tower` (`PostPass::LayerFinalization`).
- Files expected to change:
  - `modules/core-modules/skirt-brim/skirt-brim.toml`
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
- Authoritative docs:
  - `docs/01_system_architecture.md` — Stage I/O Contract table row for PostPass::LayerFinalization
  - `docs/02_ir_schemas.md` — IR field path names
- OrcaSlicer refs: None
- Verification: Same test, filter for `skirt-brim` and `wipe-tower`.

### Step 5: Populate config.schema for all modules

- Task IDs:
  - `TASK-122`
- Objective: Add `[config.schema]` sections to all 17 TOML files. For each module, inspect its source code for config keys used and populate the schema. Use field types from `docs/03_wit_and_manifest.md` Config Field Types Reference.
- Files expected to change: All 17 TOML files
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference, Module Manifest Schema
- OrcaSlicer refs: None
- Verification: `grep -c '\[config.schema\]' modules/core-modules/**/*.toml` should return 17.

### Step 6: Verify full test pass

- Task IDs:
  - `TASK-121`
  - `TASK-122`
- Objective: Run `core_module_ir_access_contract_tdd.rs` to full green. If any module fails, return to Step 2–5 and fix.
- Files expected to change: Potentially any TOML corrected based on test feedback
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test core_module_ir_access_contract_tdd -- --nocapture` — all tests pass.

## Packet Completion Gate

- All 17 core-module TOML files have populated `[ir-access].reads` and `[ir-access].writes`.
- All 17 core-module TOML files have a `[config.schema]` section.
- `core_module_ir_access_contract_tdd.rs` passes completely.
- `docs/07_implementation_status.md` TASK-121 and TASK-122 marked complete.
- `packet.spec.md` ready to move to `status: implemented`.