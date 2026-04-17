---
status: draft
packet: z-envelope-and-wit-boundary-gaps
task_ids:
  - TASK-127
  - TASK-129
  - TASK-129a
  - TASK-129b
  - TASK-129c
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: z-envelope-and-wit-boundary-gaps

## Goal

Enforce the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` at output-commit boundaries (DEV-005), and close the remaining non-segmentation WIT-boundary gaps on live execution paths including real postpass GCode command lists crossing the boundary, layer-world deep-copy coverage, and finalization-world deep-copy coverage (DEV-006).

## Scope Boundaries

- In scope:
  - TASK-127: Enforce non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` at output-commit boundaries. Covers DEV-005.
  - TASK-129a: Pass real postpass GCode command lists into `dispatch_postpass_gcode_call` and add coverage for per-command content crossing the WIT boundary. Covers DEV-006.
  - TASK-129b: Add live-path boundary coverage for layer-world deep-copy behavior outside native fallback code. Continues DEV-006.
  - TASK-129c: Add live-path boundary coverage for finalization-world deep-copy behavior outside native fallback code. Continues DEV-006.

- Out of scope:
  - Prepass segmentation input-shape gaps (TASK-128 series — separate packet)
  - `#[slicer_module]` prepass segmentation bridge (TASK-130 series — separate packet)
  - Manifest population (Workstream 1 tasks)
  - WIT consolidation (Workstream 1 tasks)

## Acceptance Criteria

- **Given** a module that writes path Z at output-commit time, **when** the path Z is outside `[layer.z, layer.z + effective_layer_height]`, **then** the host returns a fatal contract error with required diagnostics naming the module, stage, violating field, and the out-of-envelope value.
- **Given** a postpass module that calls `dispatch_postpass_gcode_call`, **when** it receives a non-empty GCode command list with real command content, **then** the content crosses the WIT boundary correctly and is not truncated or defaulted.
- **Given** a live per-layer execution run, **when** the layer-world deep-copy path is exercised, **then** all IR fields are preserved correctly through the copy and no data is lost relative to native fallback behavior.
- **Given** a live finalization execution run, **when** the finalization-world deep-copy path is exercised, **then** all IR fields are preserved correctly through the copy and no data is lost relative to native fallback behavior.
- **Given** the Z-envelope enforcement and WIT-boundary gap closures, **when** tests run, **then** all related tests pass without regression.

## Verification

- `cargo test --package slicer-host -- --nocapture` (full host test suite)
- Specific tests for Z-envelope enforcement, postpass GCode content, and deep-copy boundary coverage (to be added)

## Authoritative Docs

- `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules, Per-Layer Error Handling
- `docs/02_ir_schemas.md` — IR field paths and Z semantics
- `docs/03_wit_and_manifest.md` — WIT boundary enforcement
- `docs/04_host_scheduler.md` — Output commit validation, Proactive Validation Points

## OrcaSlicer Reference Obligations

None. This is an infra/WIT boundary task, not geometry parity.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`