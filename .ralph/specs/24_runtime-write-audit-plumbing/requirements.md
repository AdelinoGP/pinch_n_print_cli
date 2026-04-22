# Requirements: 24_runtime-write-audit-plumbing

## Problem Statement

The layer runtime-write audit plumbing uses `ir_path_for_layer_stage` as the authoritative source for `ModuleAccessAudit.runtime_writes`, which returns coarse IR roots like `"PerimeterIR"` for `Layer::PerimetersPostProcess`. When `seam-placer` declares a narrow manifest write `PerimeterIR.resolved-seam` and the guest calls `push_resolved_seam`, the audit still records `"PerimeterIR"` — causing the undeclared-access validator to either (a) reject the coarse path against the narrow manifest declaration, or (b) miss that the narrow path was actually exercised. The fix requires instrumenting the output-builder methods directly to record canonical subfield paths, and updating the dispatch/executor chain to carry `runtime_writes` alongside `runtime_reads`.

## Grouped Task IDs

- TASK-123b (Record per-layer execution audits and plumb into `DagValidationRequest.access_audits`)
- TASK-124 (Enforce undeclared runtime read/write faults at WIT boundary)

## In-Scope

- `runtime_writes` field on `HostExecutionContext` (mirrors `runtime_reads`)
- Canonical write-path helper: `record_write(ctx, "PerimeterIR.regions.walls")` etc.
- Instrumentation of `HostPerimeterOutputBuilder::{push_wall_loop, push_reordered_wall_loop, push_resolved_seam}`
- Decision on coarse writes for `HostInfillOutputBuilder` and `HostSupportOutputBuilder` (in-scope for symmetry, out-of-scope for prepass/finalization/postpass)
- `LayerStageRunner::run_stage` updated to return `(LayerStageOutput, Vec<String>, Vec<String>)` (reads, writes)
- `WasmRuntimeDispatcher` layer dispatch updated to extract and return `runtime_writes`
- `execute_single_layer` audit construction updated to use collected `runtime_writes` not `ir_path_for_layer_stage`
- Regression tests: `pipeline_tdd.rs` (3 new AC tests), `core_module_ir_access_contract_tdd.rs` (live seam audit regression)
- Negative test: missing instrumentation fails against manifest

## Out-of-Scope

- Prepass runtime-write plumbing
- Postpass/finalization runtime-write plumbing
- `ir_path_for_layer_stage` fallback removal
- Read-path normalization

## Authoritative Docs

- `docs/04_host_scheduler.md` — §Manifest ↔ Runtime Naming Map, §IR Access Path Format
- `docs/02_ir_schemas.md` — `PerimeterIR.regions.walls`, `PerimeterIR.resolved-seam`
- `crates/slicer-host/src/validation.rs` — `validate_undeclared_access`
- `crates/slicer-host/src/layer_executor.rs` — `execute_single_layer`, `ir_path_for_layer_stage`
- `crates/slicer-host/src/wit_host.rs` — `HostPerimeterOutputBuilder`, `HostExecutionContext`
- `crates/slicer-host/src/dispatch.rs` — `WasmRuntimeDispatcher::dispatch_layer_call`

## Acceptance Summary

After this packet lands:
1. `HostExecutionContext` has a `runtime_writes: Vec<String>` field initialized in `new`.
2. Each perimeter output builder push method records its canonical subfield path.
3. `LayerStageRunner::run_stage` signature returns writes alongside reads.
4. `execute_single_layer` constructs `ModuleAccessAudit.runtime_writes` from the collected writes, not from `ir_path_for_layer_stage`.
5. The `seam-placer` live audit regression proves narrow write `PerimeterIR.resolved-seam` validates correctly.
6. The coarse-fallback test proves non-instrumented stages still audit correctly.

## Verification

```
cargo test -p slicer-host --test pipeline_tdd -- --nocapture
cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture
cargo build --workspace
cargo clippy --workspace -- -D warnings
```
