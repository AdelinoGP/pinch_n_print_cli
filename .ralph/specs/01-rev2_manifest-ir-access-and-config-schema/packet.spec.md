---
status: active
packet: 01_rev2_manifest-ir-access-and-config-schema
task_ids:
  - TASK-122
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 01_rev2_manifest-ir-access-and-config-schema

## Goal

Convert all 17 core-module manifests to the full `[config.schema]` table format, ensure the `config-schema` CLI returns all six acceptance fields (`type`, `min`, `max`, `default`, `display`, `group`) for every field declared, and update AC-2 to reflect the null-serialization behavior for absent optional fields.

## Scope Boundaries

- In scope:
  - Convert 16 remaining core-module `.toml` manifests from shorthand format (`wall_count = "int"`) to full table format (`[config.schema.wall_count]` with type, default, min, max, display, group fields)
  - `arachne-perimeters.toml` already converted (partial)
  - `paint-region-annotator.toml` stays empty (no config fields; table format with no entries)
  - `mesh-segmentation.toml` and `paint-segmentation.toml` use `*` wildcard keys — preserve these as shorthand strings
  - Verify `config-schema` CLI returns all 6 AC-2 fields for every module
  - Update AC-2 language to reflect null-serialization for absent optional fields

- Out of scope:
  - Any community modules (none exist yet)
  - Changes to `[ir-access]` declarations (TASK-121 already complete)
  - Runtime access audit plumbing (TASK-123 series)
  - WIT consolidation (TASK-144 series)

## Acceptance Criteria

- **Given** a core-module manifest with a shorthand config field (e.g., `wall_count = "int"`), **when** `config-schema` CLI is called, **then** the field appears in the JSON output with `"type": "int"` and all five remaining AC-2 fields (`min`, `max`, `default`, `display`, `group`) serialized as `null`.
- **Given** a core-module manifest with a full-format config field (e.g., `wall_count = { type = "int", default = 3, min = 1, max = 10, display = "Wall Count", group = "Walls" }`), **when** `config-schema` CLI is called, **then** the field appears in the JSON output with all six AC-2 fields populated from the manifest.
- **Given** `paint-region-annotator.toml` (no config fields), **when** `config-schema` CLI is called, **then** it does not appear in the schema output (empty entries filtered out, per existing behavior).
- **Given** `mesh-segmentation.toml` and `paint-segmentation.toml` (wildcard string keys), **when** `config-schema` CLI is called, **then** those entries appear with `"type": "string"` and other fields as `null`.
- **Given** the updated `packet.spec.md` AC-2, **when** the acceptance criterion is reviewed, **then** it accurately reflects that absent optional fields serialize as `null`.

## Verification

- `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); print('modules:', len(d['schema']))"` → 16 (paint-region-annotator excluded)
- `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); f=d['schema'][0]['fields'][0]; print('has type:', 'type' in f, 'has min:', 'min' in f, 'has max:', 'max' in f, 'has default:', 'default' in f, 'has display:', 'display' in f, 'has group:', 'group' in f)"` → all True
- `grep -c '^\[config.schema\.' modules/core-modules/*/*.toml` → 16 (confirms full table format)
- `cargo test --package slicer-host --test core_module_ir_access_contract_tdd` → passes
- `cargo test --package slicer-host --test config_schema_tdd` → 42/42 pass

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — Config Field Types Reference (lines 834-842), Module Manifest Schema example with full table format (lines 562-672)
- `crates/slicer-host/src/manifest.rs` — `ConfigSchema`, `ConfigFieldEntry` structs; `read_config_schema` and `parse_config_field_entry` functions
- `crates/slicer-host/src/config_schema.rs` — `build_config_schema_json` function
- `crates/slicer-host/src/cli.rs` — `HostCommands::ConfigSchema`

## OrcaSlicer Reference Obligations

None. This is a manifest-contract and CLI wiring task.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` (not applicable — no new task IDs created)
