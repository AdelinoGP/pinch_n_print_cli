---
status: implemented
packet: 01_rev2_manifest-ir-access-and-config-schema
task_ids:
  - TASK-122
---

# 01-rev2_manifest-ir-access-and-config-schema

## Goal

Convert all 17 core-module manifests to the full `[config.schema]` table format, ensure the `config-schema` CLI returns all six acceptance fields (`type`, `min`, `max`, `default`, `display`, `group`) for every field declared, and update AC-2 to reflect the null-serialization behavior for absent optional fields.

## Problem Statement

The MED-1 review finding from the `01_manifest-ir-access-and-config-schema` spec review identified that AC-2 states the CLI should return `type`, `min`, `max`, `default`, `display`, and `group` fields, but the implementation only returned `type` and `key`. The parser infrastructure and CLI serializer have been fixed to handle all fields, but the 16 remaining core modules are still in shorthand format (`wall_count = "int"`) which only populates `type`. All 17 modules need to be converted to the full table format to demonstrate the complete feature.

## Architecture Constraints

- The manifest parser (`read_config_schema` in `manifest.rs`) already handles both shorthand and full table formats — no parser changes needed.
- The CLI serializer (`build_config_schema_json` in `config_schema.rs`) already outputs all 6 AC-2 fields with `null` for absent values — no serializer changes needed.
- `ConfigFieldEntry` struct already has all required fields — no struct changes needed.
- The `config-schema` CLI already uses the correct `--module-dir` flag — no CLI changes needed.

## Data and Contract Notes

- IR paths: N/A (no IR changes)
- WIT boundary: N/A (no WIT changes)
- Config field types must be from the valid set: `"bool"`, `"int"`, `"float"`, `"string"`, `"enum"`, `"float-list"`, `"string-list"`
- `ConfigFieldEntry.default` is stored as `Option<String>` (TOML value serialized as string)

## Risks and Tradeoffs

- **Risk**: Manual conversion of 16 manifests is error-prone. **Mitigation**: Use a script to generate the full table entries from shorthand, then manually verify and add display/group values.
- **Risk**: Reasonable default values chosen here may differ from OrcaSlicer. **Mitigation**: Acceptable — the schema is for documentation/UI purposes; runtime defaults come from the module's actual compiled defaults.
- **Tradeoff**: Full table format is more verbose. **Justification**: Enables UI generation, validation tooling, and complete CLI output as required by AC-2.
