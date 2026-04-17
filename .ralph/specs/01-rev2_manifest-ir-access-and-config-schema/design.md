# Design: 01-rev2_manifest-ir-access-and-config-schema

## Controlling Code Paths

- Primary code path: `modules/core-modules/*/*.toml` (16 files to convert + 1 to verify)
- Neighboring tests or fixtures: `crates/slicer-host/tests/config_schema_tdd.rs`, `crates/slicer-host/tests/config_view_binding_tdd.rs`, `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`
- OrcaSlicer comparison surface: None

## Architecture Constraints

- The manifest parser (`read_config_schema` in `manifest.rs`) already handles both shorthand and full table formats — no parser changes needed.
- The CLI serializer (`build_config_schema_json` in `config_schema.rs`) already outputs all 6 AC-2 fields with `null` for absent values — no serializer changes needed.
- `ConfigFieldEntry` struct already has all required fields — no struct changes needed.
- The `config-schema` CLI already uses the correct `--module-dir` flag — no CLI changes needed.

## Proposed Changes

### Per-module manifest conversion

Convert each shorthand manifest entry from:
```toml
[config.schema]
wall_count = "int"
line_width = "float"
```

To full table format:
```toml
[config.schema]
[config.schema.wall_count]
type    = "int"
default = 3
min     = 1
max     = 10
display = "Wall Count"
group   = "Walls"

[config.schema.line_width]
type    = "float"
default = 0.4
min     = 0.1
max     = 2.0
display = "Line Width"
group   = "Walls"
```

### Field value decisions

For each module, derive sensible defaults by inspecting:
1. Field name (e.g., `wall_count` → default=3, min=1, max=10)
2. Field type (e.g., `float` → reasonable mm range)
3. Field name semantics (e.g., `*_speed` → mm/s units, `*_width`/`*_height` → mm)

Use OrcaSlicer defaults as reference where available. Use sensible engineering defaults otherwise.

### Wildcard key handling

`mesh-segmentation.toml` and `paint-segmentation.toml` use wildcard keys (`"mesh_seg_mark:*" = "string"`). These cannot be converted to full table format (the `*` is a runtime pattern). Keep as shorthand strings — the parser correctly handles this case.

### Empty schema handling

`paint-region-annotator.toml` has no config fields. Keep `[config.schema]` with no entries. The CLI already filters out empty schemas.

## Data and Contract Notes

- IR paths: N/A (no IR changes)
- WIT boundary: N/A (no WIT changes)
- Config field types must be from the valid set: `"bool"`, `"int"`, `"float"`, `"string"`, `"enum"`, `"float-list"`, `"string-list"`
- `ConfigFieldEntry.default` is stored as `Option<String>` (TOML value serialized as string)

## Risks and Tradeoffs

- **Risk**: Manual conversion of 16 manifests is error-prone. **Mitigation**: Use a script to generate the full table entries from shorthand, then manually verify and add display/group values.
- **Risk**: Reasonable default values chosen here may differ from OrcaSlicer. **Mitigation**: Acceptable — the schema is for documentation/UI purposes; runtime defaults come from the module's actual compiled defaults.
- **Tradeoff**: Full table format is more verbose. **Justification**: Enables UI generation, validation tooling, and complete CLI output as required by AC-2.

## Open Questions

1. Should `arachne-perimeters.toml` be used as the canonical example format, or is the docs/03 example preferred? **Resolution**: Use docs/03 as authoritative format reference.
2. Should enum fields (e.g., `seam_mode = "string"`) be given explicit `values` constraints? **Resolution**: No — string is used as free-text here; values constraint only if module source actually restricts the set.
3. Should `layer-planner-default`'s `"layer_height:*"` and `"object_height:*"` wildcard keys remain as shorthand or be converted? **Resolution**: Keep as shorthand — these are runtime wildcard patterns that cannot be expressed as full table entries.
