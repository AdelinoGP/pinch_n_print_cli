# Requirements: 01-rev1_manifest-ir-access-and-config-schema

## Packet Metadata

- Grouped task IDs:
  - `TASK-122` — Populate `[config.schema]` for all 17 core-module manifests (incomplete; this packet finishes it)
  - CLI wiring task — Wire `config-schema` CLI subcommand to call `build_config_schema_json`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The spec review of `01_manifest-ir-access-and-config-schema` revealed:
1. TASK-121 was correctly completed (all 17 modules have non-empty `[ir-access]`).
2. TASK-122 was marked `[x]` in docs/07 but only 4/17 modules have non-empty `[config.schema]` — 13 have empty sections.
3. `path-optimization-default.toml` uses `"boolean"` which is not a valid type per `docs/03_wit_and_manifest.md` Config Field Types Reference.
4. The `config-schema` CLI subcommand exists (defined in `cli.rs`) but the `main.rs` arm is a stub that just emits `{}`.

Both gaps block DEV-008 (config-schema CLI returning real per-module schemas).

## In Scope

- Fix `path-optimization-default.toml`: change `"boolean"` → `"bool"` for `path_optimization_emit_layer_markers`.
- Audit the 13 modules with empty `[config.schema]` by reading their source under `modules/core-modules/*/src/` and `modules/core-modules/*/wit-guest/src/`.
- Populate `[config.schema]` for all 13 missing modules per `docs/03_wit_and_manifest.md` Config Field Types Reference (valid types: `"bool"`, `"int"`, `"float"`, `"string"`, `"enum"`, `"float-list"`, `"string-list"`).
- Wire `HostCommands::ConfigSchema` in `crates/slicer-host/src/main.rs` to load modules from `module_dir` and call `build_config_schema_json`, replacing the current stub.
- Make `config_schema_json_includes_modules_with_config_fields` test pass.

## Out of Scope

- Any community modules.
- TASK-121 work (ir-access; already complete and verified).
- Runtime access audit plumbing (TASK-123 series).
- WIT consolidation (TASK-144 series).
- Modifying `01_manifest-ir-access-and-config-schema/packet.spec.md`.

## Authoritative Docs

- `docs/01_system_architecture.md` — JSON query protocol format (lines 465-480)
- `docs/03_wit_and_manifest.md` — Config Field Types Reference (lines 834-842), Module Manifest Schema (lines 558-672)
- `crates/slicer-host/src/config_schema.rs` — `build_config_schema_json` function signature and behavior
- `crates/slicer-host/src/cli.rs` — `ConfigSchema` subcommand definition
- `crates/slicer-host/tests/runtime_wiring_tdd.rs` — existing test harness for schema JSON

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- `path-optimization-default.toml` uses `"bool"` type (not `"boolean"`).
- All 17 core modules have non-empty `[config.schema]` sections with valid types.
- `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules` returns non-empty JSON matching the docs/01 format.
- `runtime_wiring_tdd::config_schema_json_includes_modules_with_config_fields` passes.

## Verification Commands

- `grep 'boolean' modules/core-modules/path-optimization-default/path-optimization-default.toml` → 0 matches
- `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules` → non-`{}` JSON
- `cargo test --package slicer-host --test runtime_wiring_tdd -- config_schema_json_includes_modules_with_config_fields -- --nocapture` → PASS
