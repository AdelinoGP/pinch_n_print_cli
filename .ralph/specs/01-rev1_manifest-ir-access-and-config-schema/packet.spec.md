---
status: implemented
packet: 01-rev1_manifest-ir-access-and-config-schema
task_ids:
  - TASK-122
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 01-rev1_manifest-ir-access-and-config-schema

## Goal

Fix the incomplete TASK-122 implementation (13/17 modules have empty `[config.schema]`), fix the invalid `"boolean"` type in `path-optimization-default`, and wire the `config-schema` CLI subcommand so the verification command produces real output.

## Scope Boundaries

- In scope:
  - Fix `path-optimization-default.toml`: `"boolean"` → `"bool"`
  - Populate `[config.schema]` for the 13 modules that have empty sections by auditing their source
  - Wire `HostCommands::ConfigSchema` in `crates/slicer-host/src/main.rs` to call `build_config_schema_json`
  - Pass `config_schema_json_includes_modules_with_config_fields` test in `runtime_wiring_tdd.rs`
  - Verify with `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules`

- Out of scope:
  - Any community modules
  - TASK-121 (ir-access; already complete)
  - Runtime access audit plumbing (TASK-123 series)
  - WIT consolidation (TASK-144 series)
  - Modifying `01_manifest-ir-access-and-config-schema/packet.spec.md`

## Acceptance Criteria

- **Given** `path-optimization-default.toml`, **when** the config schema is parsed, **then** `path_optimization_emit_layer_markers` has type `"bool"` (not `"boolean"`).
- **Given** the 13 modules with previously empty `[config.schema]`, **when** the CLI `config-schema` command is called, **then** those modules appear in the JSON output with their declared fields.
- **Given** `modules/core-modules/` populated with all 17 non-empty config schemas, **when** `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules` is called, **then** the output is a JSON object with module IDs as keys and field arrays as values, per docs/01 lines 465-480.
- **Given** `runtime_wiring_tdd.rs`, **when** `config_schema_json_includes_modules_with_config_fields` runs, **then** it passes.

## Verification

- `grep 'boolean' modules/core-modules/path-optimization-default/path-optimization-default.toml` → 0 matches
- `grep 'bool' modules/core-modules/path-optimization-default/path-optimization-default.toml` → at least 1 match
- `cargo run --package slicer-host -- config-schema --module-dir modules/core-modules` → non-`{}` JSON output
- `cargo test --package slicer-host --test runtime_wiring_tdd -- config_schema_json_includes_modules_with_config_fields -- --nocapture` → passes
- `grep -c '\[config.schema\]' modules/core-modules/**/*.toml` → 17 (confirms sections exist; spot-check for non-empty content)

## Authoritative Docs

- `docs/01_system_architecture.md` — JSON query protocol (lines 465-480), Stage I/O Contract (lines 335-357)
- `docs/03_wit_and_manifest.md` — Config Field Types Reference (lines 834-842), Module Manifest Schema example (lines 562-672)
- `crates/slicer-host/src/cli.rs` — `HostCommands::ConfigSchema` definition
- `crates/slicer-host/src/main.rs` — current stub at `HostCommands::ConfigSchema` arm (lines 252-255)
- `crates/slicer-host/src/config_schema.rs` — `build_config_schema_json` function
- `crates/slicer-host/tests/runtime_wiring_tdd.rs` — existing test for schema JSON building

## OrcaSlicer Reference Obligations

None. This is manifest-contract and CLI wiring work.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
