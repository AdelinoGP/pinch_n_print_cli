# Requirements: z-envelope-and-wit-boundary-gaps

## Packet Metadata

- Grouped task IDs:
  - `TASK-127` — Enforce non-planar Z envelope at output-commit boundaries. Covers DEV-005.
  - `TASK-129` / `TASK-129a` / `TASK-129b` / `TASK-129c` — Close remaining non-segmentation WIT-boundary gaps on live execution paths. Covers DEV-006.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

DEV-005: Non-planar Z envelope enforcement is missing at output-commit boundaries. Modules that write path Z could emit Z values outside their declared layer envelope, corrupting the print.

DEV-006: Postpass GCode command content and executable WIT-boundary coverage still have live gaps. Specifically, `dispatch_postpass_gcode_call` may not receive real command lists, and layer-world/finalization-world deep-copy behavior outside native fallback code lacks live-path boundary coverage.

## In Scope

- TASK-127: Implement proactive Z-envelope validation at output-commit for any module that writes path Z. Validate against `[layer.z, layer.z + effective_layer_height]`. Emit fatal contract error with precise diagnostics on violation.
- TASK-129a: Pass real postpass GCode command lists into `dispatch_postpass_gcode_call`. Add coverage for per-command content crossing the WIT boundary.
- TASK-129b: Add live-path boundary coverage for layer-world deep-copy behavior outside native fallback code.
- TASK-129c: Add live-path boundary coverage for finalization-world deep-copy behavior outside native fallback code.

## Out of Scope

- Prepass segmentation SDK→WIT alignment (TASK-128 series — separate packet)
- `#[slicer_module]` prepass bridge (TASK-130 series — separate packet)
- Manifest population (Workstream 1)
- WIT consolidation (Workstream 1)

## Authoritative Docs

- `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules (rows 260–268), Per-Layer Error Handling (rows 269–274)
- `docs/02_ir_schemas.md` — `ExtrusionPath3D`, Z field semantics
- `docs/03_wit_and_manifest.md` — WIT boundary enforcement
- `docs/04_host_scheduler.md` — Proactive Validation Points (rows 568–574), Phase 4 Execution

## OrcaSlicer Reference Obligations

None.

## Acceptance Summary

- Z-envelope enforcement active at output-commit; violations produce fatal contract errors with required diagnostics.
- Real postpass GCode command lists cross the WIT boundary correctly via `dispatch_postpass_gcode_call`.
- Layer-world deep-copy has live-path boundary coverage with no data loss vs. native fallback.
- Finalization-world deep-copy has live-path boundary coverage with no data loss vs. native fallback.
- All related tests pass.

## Verification Commands

- `cargo test --package slicer-host -- --nocapture` (full host test suite)
- Negative test for Z-envelope violation (test that a module writing out-of-envelope Z gets fatal error)