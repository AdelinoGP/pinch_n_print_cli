---
status: draft
packet: 24_runtime-write-audit-plumbing
task_ids:
  - TASK-123b
  - TASK-124
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 24_runtime-write-audit-plumbing

## Goal

Repair the layer runtime-write audit plumbing so modules that write narrow manifest paths (e.g. `seam-placer` writing `PerimeterIR.resolved-seam`) do not trip the undeclared-access validator, which currently treats any write not in `ir_path_for_layer_stage` as missing and any write not covered by the manifest as a failure.

## Scope Boundaries

- **In scope:** `runtime_writes` field on `HostExecutionContext`; instrumentation of `HostPerimeterOutputBuilder::{push_wall_loop, push_reordered_wall_loop, push_resolved_seam}`; update to `LayerStageRunner::run_stage` signature; update to `WasmRuntimeDispatcher` dispatch path; update to `execute_single_layer` audit construction; regression tests in `pipeline_tdd.rs` and `core_module_ir_access_contract_tdd.rs`.
- **Out of scope:** Prepass runtime-write plumbing; postpass/finalization runtime-write plumbing; Infill/Support coarse root writes; read-path normalization; `ir_path_for_layer_stage` fallback removal.

## Prerequisites and Blockers

- **Depends on:** None.
- **Unblocks:** Packet 25 (WIT drift lock — requires seam narrow-write audit to be correct before doc updates).
- **Activation blockers:** None.

## Acceptance Criteria

- **Given** a `HostExecutionContext` initialized for a perimeter-output builder call, **when** `push_wall_loop` is invoked, **then** `ctx.runtime_writes` contains `"PerimeterIR.regions.walls"`. | `cd crates/slicer-host && cargo test -p slicer-host --test pipeline_tdd push_wall_loop_records_runtime_write -- --nocapture 2>&1 | tail -20`
- **Given** a `HostExecutionContext` initialized for a perimeter-output builder call, **when** `push_reordered_wall_loop` is invoked, **then** `ctx.runtime_writes` contains `"PerimeterIR.regions.walls"`. | `cd crates/slicer-host && cargo test -p slicer-host --test pipeline_tdd push_reordered_wall_loop_records_runtime_write -- --nocapture 2>&1 | tail -20`
- **Given** a `HostExecutionContext` initialized for a perimeter-output builder call, **when** `push_resolved_seam` is invoked, **then** `ctx.runtime_writes` contains `"PerimeterIR.resolved-seam"`. | `cd crates/slicer-host && cargo test -p slicer-host --test pipeline_tdd push_resolved_seam_records_runtime_write -- --nocapture 2>&1 | tail -20`
- **Given** a `WasmRuntimeDispatcher` dispatch for `Layer::PerimetersPostProcess` where the guest calls `push_wall_loop`, **when** the dispatch completes, **then** the returned `runtime_writes` includes `"PerimeterIR.regions.walls"` (not `"PerimeterIR"`). | `cd crates/slicer-host && cargo test -p slicer-host --test core_module_ir_access_contract_tdd perimeter_narrow_write_audit -- --nocapture 2>&1 | tail -20`
- **Given** a `seam-placer` module with manifest write `PerimeterIR.resolved-seam`, **when** it runs on the live path and calls `push_resolved_seam`, **then** the constructed `ModuleAccessAudit.runtime_writes` contains only `"PerimeterIR.resolved-seam"` (not the coarse `"PerimeterIR"` root). | `cd crates/slicer-host && cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_placer_narrow_manifest_write_validates -- --nocapture 2>&1 | tail -20`
- **Given** a layer stage that writes an IR type not yet instrumented for narrow writes (e.g. `Layer::Infill`), **when** the stage completes and `ir_path_for_layer_stage` returns the coarse fallback, **then** the audit still records the fallback write path without panicking. | `cd crates/slicer-host && cargo test -p slicer-host --test pipeline_tdd infill_coarse_fallback_audit -- --nocapture 2>&1 | tail -20`

## Negative Test Cases

- **Given** a guest module that calls `push_wall_loop` but the host `runtime_writes` instrumentation is not wired, **when** the dispatch completes, **then** the returned `runtime_writes` does not contain `"PerimeterIR.regions.walls"` and the corresponding `ModuleAccessAudit.runtime_writes` assertion fails. | `cd crates/slicer-host && cargo test -p slicer-host --test pipeline_tdd missing_runtime_writes_fails -- --nocapture 2>&1 | tail -20`
- **Given** a module that declares a narrow write path in its manifest but the runtime audit collects a coarser path (e.g. `"PerimeterIR"` instead of `"PerimeterIR.resolved-seam"`), **when** `validate_undeclared_access` is called, **then** it returns an error indicating the declared path was not exercised. | `cd crates/slicer-host && cargo test -p slicer-host --test core_module_ir_access_contract_tdd coarse_write_rejected_against_narrow_manifest -- --nocapture 2>&1 | tail -20`

## Verification

- `cargo test -p slicer-host --test pipeline_tdd -- --nocapture`
- `cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/04_host_scheduler.md` — §Manifest ↔ Runtime Naming Map; §IR Access Path Format
- `docs/02_ir_schemas.md` — `PerimeterIR` field layout (`regions.walls`, `resolved-seam`)
- `docs/03_wit_and_manifest.md` — perimeter output builder interface

## OrcaSlicer Reference Obligations

- None.
