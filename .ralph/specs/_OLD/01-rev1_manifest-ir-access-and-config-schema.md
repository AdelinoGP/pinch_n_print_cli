---
status: implemented
packet: 01-rev1_manifest-ir-access-and-config-schema
task_ids:
  - TASK-122
---

# 01-rev1_manifest-ir-access-and-config-schema

## Goal

Fix the incomplete TASK-122 implementation (13/17 modules have empty `[config.schema]`), fix the invalid `"boolean"` type in `path-optimization-default`, and wire the `config-schema` CLI subcommand so the verification command produces real output.

## Problem Statement

The spec review of `01_manifest-ir-access-and-config-schema` revealed:
1. TASK-121 was correctly completed (all 17 modules have non-empty `[ir-access]`).
2. TASK-122 was marked `[x]` in docs/07 but only 4/17 modules have non-empty `[config.schema]` — 13 have empty sections.
3. `path-optimization-default.toml` uses `"boolean"` which is not a valid type per `docs/03_wit_and_manifest.md` Config Field Types Reference.
4. The `config-schema` CLI subcommand exists (defined in `cli.rs`) but the `main.rs` arm is a stub that just emits `{}`.

Both gaps block DEV-008 (config-schema CLI returning real per-module schemas).

## Architecture Constraints

- Config schema `type` values must be one of: `"bool"`, `"int"`, `"float"`, `"string"`, `"enum"`, `"float-list"`, `"string-list"` (per docs/03 lines 834-842).
- Config schema response format must match docs/01 lines 465-480: `{ "schema": [ { "module": "...", "fields": [ ... ] } ] }`.
- The `load_live_modules_for_plan` function (or a variant) must be used to discover and load module manifests from `module_dir` so their schemas can be passed to `build_config_schema_json`.

## Data and Contract Notes

- Config schema types must match the valid set exactly — no `"boolean"`, no custom types.
- Modules with no config fields should still have an empty `[config.schema]` section (already present in all 17 files).
- The `build_config_schema_json` function already filters to modules with non-empty `config_schema.entries`, so empty-schema modules will be absent from the JSON output — this is correct behavior.
- The response format from docs/01 is `{ "schema": [ { "module": "<id>", "fields": [ { "key": "...", "type": "...", "default": ..., "display": "...", "group": "..." } ] } ] }`.

## Risks and Tradeoffs

- Some modules may genuinely have no config fields (the TOML has empty `[config.schema]` but no fields are actually declared — this is fine, the JSON output simply won't list those modules).
- Config field values (defaults, ranges) must be accurate — incorrect defaults would cause wrong behavior at runtime. Source-code audit is required rather than guessing.
- The CLI stub replacement must handle module loading errors gracefully (print error to stderr and exit non-zero).
