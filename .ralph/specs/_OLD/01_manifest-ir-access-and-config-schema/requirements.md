# Requirements: manifest-ir-access-and-config-schema

## Packet Metadata

- Grouped task IDs:
  - `TASK-121` — Populate `[ir-access]` for all 17 core-module manifests per docs/01 Stage I/O Contract. Covers DEV-002.
  - `TASK-122` — Populate `[config.schema]` for all 17 core-module manifests so the `config-schema` CLI returns real per-module schemas. Covers DEV-008.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

All 17 core-module TOML manifests currently have empty `[ir-access]` and `[config.schema]` sections. This means:
1. The host cannot enforce the Stage I/O Contract at runtime (DEV-002) because declarations are missing.
2. The `config-schema` CLI returns no usable output (DEV-008) because schemas are absent.

Both gaps block the architecture acceptance gate.

## In Scope

- Enumerate all 17 core modules and determine their stage from `[stage].id`.
- Map each stage to its authoritative reads/writes from the Stage I/O Contract table in `docs/01_system_architecture.md`.
- Populate `[ir-access].reads` and `[ir-access].writes` for every module.
- Populate `[config.schema]` with real fields for every module that accepts config, using the types in `docs/03_wit_and_manifest.md`.
- Make `core_module_ir_access_contract_tdd.rs` green as the acceptance gate.

## Out of Scope

- Runtime audit plumbing (TASK-123 series).
- WIT consolidation (TASK-144 series).
- Community modules.
- `config-schema` CLI implementation (that is a separate host-side task).

## Authoritative Docs

- `docs/01_system_architecture.md` — Stage I/O Contract table (rows 335–357), Module Access Contract, Data Dependency Matrix
- `docs/03_wit_and_manifest.md` — Module Manifest Schema (§ Module Manifest Schema (TOML)), Config Field Types Reference, Valid Reads/Writes
- `docs/02_ir_schemas.md` — IR field path names for reads/writes

## OrcaSlicer Reference Obligations

None. This is a manifest-contract task.

## Acceptance Summary

- All 17 core-module manifests have non-empty `[ir-access].reads` and `[ir-access].writes` covering exactly the IR paths their stage requires.
- All 17 core-module manifests have a `[config.schema]` section (even if empty for modules that accept no config).
- `core_module_ir_access_contract_tdd.rs` passes with no skipped tests.
- Manifests do not declare reads/writes that contradict the Stage I/O Contract.

## Verification Commands

- `cargo test --package slicer-host --test core_module_ir_access_contract_tdd -- --nocapture`
- Manual: `grep -r 'reads\s*=\s*\[\]' modules/core-modules/**/*.toml` should return zero matches for populated manifests.