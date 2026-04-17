# Implementation Plan: 01_rev2_manifest-ir-access-and-config-schema

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Verify arachne-perimeters.toml is complete and use as format reference

- Task IDs:
  - `TASK-122`
- Objective: Confirm the existing `arachne-perimeters.toml` full-format conversion is complete and correct. This file serves as the canonical example for the remaining conversions.
- Files expected to change: None (audit only)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference (lines 834-842)
- OrcaSlicer refs: None
- Verification: `grep -A 10 '^\[config.schema\]' modules/core-modules/arachne-perimeters/arachne-perimeters.toml` shows all entries in full table format with type, default, min, max, display, group

### Step 2: Convert perimeter modules (classic-perimeters, arachne-perimeters)

- Task IDs:
  - `TASK-122`
- Objective: Convert `classic-perimeters.toml` to full table format. `arachne-perimeters.toml` is already done (verify only).
- Files expected to change:
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema example (lines 562-672)
- OrcaSlicer refs: None
- Verification: `grep '^\[config.schema\.' modules/core-modules/classic-perimeters/classic-perimeters.toml | wc -l` → 4 entries

### Step 3: Convert infill modules (rectilinear-infill, gyroid-infill, lightning-infill)

- Task IDs:
  - `TASK-122`
- Objective: Convert all three infill modules to full table format.
- Files expected to change:
  - `modules/core-modules/rectilinear-infill/rectilinear-infill.toml`
  - `modules/core-modules/gyroid-infill/gyroid-infill.toml`
  - `modules/core-modules/lightning-infill/lightning-infill.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema example
- OrcaSlicer refs: None
- Verification: Each module has 4 entries in full table format

### Step 4: Convert support modules (traditional-support, tree-support, support-surface-ironing)

- Task IDs:
  - `TASK-122`
- Objective: Convert all three support modules to full table format.
- Files expected to change:
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/tree-support/tree-support.toml`
  - `modules/core-modules/support-surface-ironing/support-surface-ironing.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema example
- OrcaSlicer refs: None
- Verification: Each module has 5 entries in full table format (support modules share similar schema)

### Step 5: Convert layer planning and post-process modules (layer-planner-default, seam-placer, fuzzy-skin, paint-region-annotator)

- Task IDs:
  - `TASK-122`
- Objective: Convert these four modules. Note: `layer-planner-default` has wildcard keys that stay as shorthand; `paint-region-annotator` has empty schema.
- Files expected to change:
  - `modules/core-modules/layer-planner-default/layer-planner-default.toml`
  - `modules/core-modules/seam-placer/seam-placer.toml`
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`
  - `modules/core-modules/paint-region-annotator/paint-region-annotator.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema example
- OrcaSlicer refs: None
- Verification: `layer-planner-default` has 2 full + 2 shorthand wildcard entries; `seam-placer` has 1 full entry; `fuzzy-skin` has 3 full entries; `paint-region-annotator` has empty `[config.schema]`

### Step 6: Convert finalization modules (skirt-brim, wipe-tower, path-optimization-default)

- Task IDs:
  - `TASK-122`
- Objective: Convert all three finalization/post-process modules to full table format.
- Files expected to change:
  - `modules/core-modules/skirt-brim/skirt-brim.toml`
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema example
- OrcaSlicer refs: None
- Verification: Each module has entries in full table format

### Step 7: Preserve wildcard string keys in mesh-segmentation and paint-segmentation

- Task IDs:
  - `TASK-122`
- Objective: Verify `mesh-segmentation.toml` and `paint-segmentation.toml` keep their wildcard string keys as-is (they cannot be converted to full table format). Verify `paint-segmentation` config comments are preserved.
- Files expected to change: None (verify only)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Module Manifest Schema example
- OrcaSlicer refs: None
- Verification: Both manifests still have `"*" = "string"` entries in shorthand format

### Step 8: Full verification — run CLI and all tests

- Task IDs:
  - `TASK-122`
- Objective: Run the full verification suite to confirm all 16 modules converted and CLI outputs correct format.
- Files expected to change: None
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference
- OrcaSlicer refs: None
- Verification:
  - `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d['schema']) == 16, f'Expected 16, got {len(d['schema'])}'; print('PASS')"`
  - `cargo test --package slicer-host --test core_module_ir_access_contract_tdd` → 3/3 pass
  - `cargo test --package slicer-host --test config_schema_tdd` → 42/42 pass
  - `grep -c '^\[config.schema\.' modules/core-modules/*/*.toml` → at least 16

### Step 9: Update packet AC-2 language

- Task IDs:
  - `TASK-122`
- Objective: Update `01_manifest-ir-access-and-config-schema/packet.spec.md` AC-2 to reflect null-serialization for absent optional fields (this was clarified during MED-1 investigation).
- Files expected to change:
  - `01_manifest-ir-access-and-config-schema/packet.spec.md`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — Config Field Types Reference
- OrcaSlicer refs: None
- Verification: AC-2 language is updated to say all 6 fields always returned; absent optional fields are `null`

## Packet Completion Gate

- All 16 modules converted to full table format (or verified as correct shorthand/wildcard/empty).
- `config-schema` CLI returns 16 module schemas with all 6 AC-2 fields present.
- `core_module_ir_access_contract_tdd.rs` passes (3/3).
- `config_schema_tdd.rs` passes (42/42).
- `01_manifest-ir-access-and-config-schema/packet.spec.md` AC-2 updated.
- `packet.spec.md` for this packet (01_rev2) moved to `status: implemented`.
