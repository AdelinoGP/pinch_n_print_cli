# Implementation Plan: 01-rev1_manifest-ir-access-and-config-schema

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Fix `"boolean"` type in path-optimization-default

- Task IDs:
  - `TASK-122`
- Objective: Replace the invalid `"boolean"` type with the correct `"bool"` type in `path-optimization-default.toml`.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference (lines 834-842)
- OrcaSlicer refs: None
- Verification: `grep 'boolean' modules/core-modules/path-optimization-default/path-optimization-default.toml` returns 0 matches; `grep 'bool' modules/core-modules/path-optimization-default/path-optimization-default.toml` returns ≥1 match.

### Step 2: Audit source for 13 empty-schema modules

- Task IDs:
  - `TASK-122`
- Objective: Read source for the 13 modules with empty `[config.schema]` to identify their actual config keys. Modules: `arachne-perimeters`, `classic-perimeters`, `fuzzy-skin`, `gyroid-infill`, `lightning-infill`, `paint-region-annotator`, `rectilinear-infill`, `seam-placer`, `skirt-brim`, `support-surface-ironing`, `traditional-support`, `tree-support`, `wipe-tower`.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema (lines 558-672)
  - `crates/slicer-host/src/config_schema.rs` — Config field schema shape
- OrcaSlicer refs: None
- Verification: Compile a list of config keys per module with their inferred types.

### Step 3: Populate config.schema for infill modules

- Task IDs:
  - `TASK-122`
- Objective: Add `[config.schema]` entries for `gyroid-infill`, `lightning-infill`, `rectilinear-infill`.
- Files expected to change:
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml`
  - `modules/core-modules/lightning-infill/lightning-infill.toml`
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference
- OrcaSlicer refs: None
- Verification: Each file has at least one non-comment line under `[config.schema]`.

### Step 4: Populate config.schema for perimeter and post-process modules

- Task IDs:
  - `TASK-122`
- Objective: Add `[config.schema]` entries for `arachne-perimeters`, `classic-perimeters`, `fuzzy-skin`, `seam-placer`, `paint-region-annotator`.
- Files expected to change:
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`
  - `modules/core-modules/seam-placer/seam-placer.toml`
  - `modules/core-modules/paint-region-annotator/paint-region-annotator.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference
- OrcaSlicer refs: None
- Verification: Each file has at least one non-comment line under `[config.schema]`.

### Step 5: Populate config.schema for support modules

- Task IDs:
  - `TASK-122`
- Objective: Add `[config.schema]` entries for `traditional-support`, `tree-support`, `support-surface-ironing`.
- Files expected to change:
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/tree-support/tree-support.toml`
  - `modules/core-modules/support-surface-ironing/support-surface-ironing.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference
- OrcaSlicer refs: None
- Verification: Each file has at least one non-comment line under `[config.schema]`.

### Step 6: Populate config.schema for finalization modules

- Task IDs:
  - `TASK-122`
- Objective: Add `[config.schema]` entries for `skirt-brim` and `wipe-tower`.
- Files expected to change:
  - `modules/core-modules/skirt-brim/skirt-brim.toml`
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference
- OrcaSlicer refs: None
- Verification: Each file has at least one non-comment line under `[config.schema]`.

### Step 7: Wire config-schema CLI subcommand

- Task IDs:
  - `TASK-122` (schema population part)
  - CLI wiring (new implicit task)
- Objective: Replace the stub `HostCommands::ConfigSchema` arm in `main.rs` with real implementation that loads modules and calls `build_config_schema_json`.
- Files expected to change:
  - `crates/slicer-host/src/main.rs`
- Authoritative docs:
  - `crates/slicer-host/src/config_schema.rs` — `build_config_schema_json` function
  - `crates/slicer-host/src/manifest.rs` — `load_live_modules_for_plan` or `load_module_from_paths`
  - `docs/01_system_architecture.md` — JSON response format (lines 465-480)
- OrcaSlicer refs: None
- Verification: `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules` produces non-`{}` JSON matching the docs/01 format.

### Step 8: Verify runtime_wiring_tdd test passes

- Task IDs:
  - `TASK-122`
- Objective: Run the existing test that validates schema JSON building.
- Files expected to change: None
- Authoritative docs:
  - `crates/slicer-host/tests/runtime_wiring_tdd.rs`
- OrcaSlicer refs: None
- Verification: `cargo test --package slicer-host --test runtime_wiring_tdd -- config_schema_json_includes_modules_with_config_fields -- --nocapture` → PASS.

## Packet Completion Gate

- All 13 empty-schema modules have at least one config field declared under `[config.schema]`.
- `path-optimization-default.toml` uses `"bool"` type (not `"boolean"`).
- `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules` returns non-empty JSON with correct structure.
- `runtime_wiring_tdd::config_schema_json_includes_modules_with_config_fields` passes.
- `docs/07_implementation_status.md` TASK-122 marker remains `[x]` (already marked complete — no change needed after fixes land).
- `packet.spec.md` for this revision is ready to move to `status: implemented`.
