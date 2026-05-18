# Requirements: 01_rev2_manifest-ir-access-and-config-schema

## Packet Metadata

- Grouped task IDs:
  - `TASK-122` — Populate `[config.schema]` for all 17 core-module manifests so the `config-schema` CLI returns real per-module schemas. (Already done for parser/CLI infrastructure; remaining work is full-format conversion.)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The MED-1 review finding from the `01_manifest-ir-access-and-config-schema` spec review identified that AC-2 states the CLI should return `type`, `min`, `max`, `default`, `display`, and `group` fields, but the implementation only returned `type` and `key`. The parser infrastructure and CLI serializer have been fixed to handle all fields, but the 16 remaining core modules are still in shorthand format (`wall_count = "int"`) which only populates `type`. All 17 modules need to be converted to the full table format to demonstrate the complete feature.

## In Scope

- Convert 16 remaining core-module `.toml` manifests from shorthand to full table format:
  - `classic-perimeters`, `fuzzy-skin`, `gyroid-infill`, `layer-planner-default`, `lightning-infill`, `path-optimization-default`, `rectilinear-infill`, `seam-placer`, `skirt-brim`, `support-surface-ironing`, `traditional-support`, `tree-support`, `wipe-tower`
  - `mesh-segmentation` and `paint-segmentation` keep wildcard keys as shorthand strings (type-only)
  - `paint-region-annotator` stays empty table (no config fields)
  - `arachne-perimeters` is already done (verify)
- Verify CLI output has all 6 AC-2 fields per entry
- Update AC-2 language in `packet.spec.md` to reflect null-serialization for absent optional fields

## Out of Scope

- Any community modules (none exist yet)
- Changes to `[ir-access]` declarations
- Runtime access audit plumbing (TASK-123 series)
- WIT consolidation (TASK-144 series)

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — Config Field Types Reference (§ Config Field Types Reference, lines 834-842), Module Manifest Schema example (lines 562-672)
- `crates/slicer-host/src/manifest.rs` — `ConfigSchema`, `ConfigFieldEntry` structs; `read_config_schema`, `parse_config_field_entry` functions
- `crates/slicer-host/src/config_schema.rs` — `build_config_schema_json` function

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- All 17 core-module manifests have `[config.schema]` sections.
- 16 manifests use full table format with type, default, min, max, display, group per field.
- 2 manifests (`mesh-segmentation`, `paint-segmentation`) preserve wildcard string keys as type-only shorthand.
- 1 manifest (`paint-region-annotator`) has empty `[config.schema]`.
- `config-schema` CLI returns all 6 AC-2 fields for every non-empty entry; absent fields are `null`.
- `core_module_ir_access_contract_tdd.rs` and `config_schema_tdd.rs` both pass completely.

## Verification Commands

- `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d['schema']))"` → 16
- `grep -c '^\[config.schema\.' modules/core-modules/*/*.toml` → 16+
- `cargo test --package slicer-host --test core_module_ir_access_contract_tdd` → 3/3 pass
- `cargo test --package slicer-host --test config_schema_tdd` → 42/42 pass
