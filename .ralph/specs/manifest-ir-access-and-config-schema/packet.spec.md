---
status: draft
packet: manifest-ir-access-and-config-schema
task_ids:
  - TASK-121
  - TASK-122
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: manifest-ir-access-and-config-schema

## Goal

Populate `[ir-access]` declarations and `[config.schema]` for all 17 core-module manifests so the Stage I/O Contract enforcement (DEV-002) and `config-schema` CLI (DEV-008) both go live.

## Scope Boundaries

- In scope:
  - All 17 core-module `.toml` manifests under `modules/core-modules/`
  - `[ir-access].reads` and `[ir-access].writes` populated per the Stage I/O Contract table in `docs/01_system_architecture.md`
  - `[config.schema]` populated for every module that accepts config, following the schema types in `docs/03_wit_and_manifest.md`
  - `core_module_ir_access_contract_tdd.rs` turning green as the acceptance gate

- Out of scope:
  - Any non-core (community) modules
  - Runtime access audit plumbing (TASK-123)
  - WIT consolidation (TASK-144)

## Acceptance Criteria

- **Given** the 17 core-module manifests, **when** each manifest's `[ir-access]` is populated from the Stage I/O Contract table, **then** `core_module_ir_access_contract_tdd.rs` passes for every module.
- **Given** a module manifest with config fields, **when** `config-schema` CLI is called, **then** it returns a real per-module JSON schema with type, min, max, default, display, and group fields.
- **Given** `docs/01_system_architecture.md` Stage I/O Contract rows, **when** manifests are checked against it, **then** no module declares reads/writes that contradict the Stage I/O Contract for its stage.

## Verification

- `cargo test --package slicer-host --test core_module_ir_access_contract_tdd -- --nocapture`
- `cargo run --package slicer-host -- config-schema --module-path modules/core-modules` (once implemented)

## Authoritative Docs

- `docs/01_system_architecture.md` — Stage I/O Contract table, Module Access Contract
- `docs/03_wit_and_manifest.md` — Module Manifest Schema, Config Field Types Reference, Valid Reads/Writes section
- `docs/02_ir_schemas.md` — IR field paths referenced in `[ir-access]`

## OrcaSlicer Reference Obligations

None. This is a manifest-contract and infra task, not a geometry-parity task.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`