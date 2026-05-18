# Design: 01-rev1_manifest-ir-access-and-config-schema

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/*/*.toml` — `[config.schema]` sections to populate (13 files)
  - `crates/slicer-host/src/main.rs` — `HostCommands::ConfigSchema` arm to wire
  - `crates/slicer-host/src/config_schema.rs` — `build_config_schema_json` already exists
  - `crates/slicer-host/src/cli.rs` — `ConfigSchema` subcommand already defined
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/runtime_wiring_tdd.rs` — `config_schema_json_includes_modules_with_config_fields`
  - `crates/slicer-host/tests/cli_tdd.rs` — CLI parsing tests for `ConfigSchema` subcommand
- OrcaSlicer comparison surface: None

## Architecture Constraints

- Config schema `type` values must be one of: `"bool"`, `"int"`, `"float"`, `"string"`, `"enum"`, `"float-list"`, `"string-list"` (per docs/03 lines 834-842).
- Config schema response format must match docs/01 lines 465-480: `{ "schema": [ { "module": "...", "fields": [ ... ] } ] }`.
- The `load_live_modules_for_plan` function (or a variant) must be used to discover and load module manifests from `module_dir` so their schemas can be passed to `build_config_schema_json`.

## Proposed Changes

1. **Fix `path-optimization-default.toml`**: Change line 32 from `path_optimization_emit_layer_markers = "boolean"` to `path_optimization_emit_layer_markers = "bool"`.

2. **Audit source for 13 empty-schema modules**: For each of the 13 modules, read the module's source files (`src/` or `wit-guest/src/`) and/or WIT world definition to find what config keys it actually uses. Cross-reference against the existing config entries already known for `layer-planner-default`, `mesh-segmentation`, `paint-segmentation`, and `path-optimization-default`.

3. **Populate `[config.schema]` for 13 modules**: Add TOML entries following the schema in `docs/03_wit_and_manifest.md` Module Manifest Schema example. Each field needs: `type` (from valid set), `default`, and optionally `min`, `max`, `display`, `group`, `advanced`.

4. **Wire `ConfigSchema` CLI arm**: Replace the stub in `main.rs` with:
   - Call `load_live_modules_for_plan` with the provided `module_dir` to get loaded modules.
   - Pass the loaded modules to `build_config_schema_json`.
   - Print the resulting JSON to stdout.

## Data and Contract Notes

- Config schema types must match the valid set exactly — no `"boolean"`, no custom types.
- Modules with no config fields should still have an empty `[config.schema]` section (already present in all 17 files).
- The `build_config_schema_json` function already filters to modules with non-empty `config_schema.entries`, so empty-schema modules will be absent from the JSON output — this is correct behavior.
- The response format from docs/01 is `{ "schema": [ { "module": "<id>", "fields": [ { "key": "...", "type": "...", "default": ..., "display": "...", "group": "..." } ] } ] }`.

## Risks and Tradeoffs

- Some modules may genuinely have no config fields (the TOML has empty `[config.schema]` but no fields are actually declared — this is fine, the JSON output simply won't list those modules).
- Config field values (defaults, ranges) must be accurate — incorrect defaults would cause wrong behavior at runtime. Source-code audit is required rather than guessing.
- The CLI stub replacement must handle module loading errors gracefully (print error to stderr and exit non-zero).

## Open Questions

- None. The path is clear: fix type, audit sources, populate schemas, wire CLI.
