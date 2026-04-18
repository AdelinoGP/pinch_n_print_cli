---
status: active
packet: 03-rev2_wit-canonical-source-and-validation
task_ids:
  - TASK-144
  - TASK-145
  - TASK-146
backlog_source: docs/07_implementation_status.md
supersedes: 03-rev1_wit-canonical-source-and-validation
---

# Packet Contract: 03-rev2_wit-canonical-source-and-validation

## Goal

Fix all clippy errors in `slicer-host` exposed by Rust/Cargo 1.94.0 (which promotes previously-warned lints to errors under `-D warnings`) and fix a pre-existing bug in `wit_drift_detection_tdd` where the test asserts the wrong WIT `with:` block key format.

## Scope Boundaries

- In scope:
  - Fix `wit_drift_detection_tdd::host_bindgen_with_keys_use_canonical_world_names` — the test checks for version-unsuffixed `with:` keys (`slicer:world-layer/config-types/config-view`) but wasmtime `bindgen!` emits version-suffixed keys (`slicer:world-layer/config-types@1.0.0.config-view`). Update all 4 canonical world keys in the test.
  - Fix unused `ConfigFieldEntry` import in `execution_plan.rs:12` — move inside `#[cfg(test)]` block or remove
  - Fix large `Result` variants in `dag.rs:27` and `execution_plan.rs:298` — box `SchedulerError` and `LiveModuleLoadError` with `Box<...>`
  - Fix `layer_executor.rs:263` — `sort_by` → `sort_by_key`
  - Fix `manifest.rs:359` — `map_or` simplification
  - Fix `mesh_analysis.rs:119` — `!mesh.indices.len().is_multiple_of(3)`
  - Fix `prepass.rs:219` — unnecessary `.unwrap()` after `is_some` check
  - Fix `region_mapping.rs:119` — remove `.clone()` on `u64` (which is `Copy`)
  - Fix `slice_postprocess.rs:296,310` — remove `.clone()` on `PaintValue` (which is `Copy`)
  - Fix `dispatch.rs:253` — reduce function arguments from 11 to ≤7
  - Fix `wit_host.rs` missing docs on `pub mod` blocks and struct fields

- Out of scope:
  - `slicer-macros` and `slicer-core` (both are already clippy-clean)
  - New WIT functionality or schema changes
  - Custom payload widening (TASK-149/150)

## Prerequisites and Blockers

- Depends on: None (self-contained remediation)
- Unblocks: `04_custom-payload-widening` (clippy must be green to proceed)
- Activation blockers: None — all issues are confirmed and fixable

## Acceptance Criteria

- **Given** `wit_drift_detection_tdd` runs with the fixed test assertions, **when** `host_bindgen_with_keys_use_canonical_world_names` executes, **then** it checks for the version-suffixed `with:` keys (`slicer:world-layer/config-types@1.0.0.config-view`, etc.) and the test passes. | `cargo test --package slicer-host --test wit_drift_detection_tdd -- host_bindgen_with_keys_use_canonical_world_names --nocapture 2>&1 | tail -3`

- **Given** `cargo clippy --package slicer-host -- -D warnings` runs, **when** the command completes, **then** it exits with code 0 and emits no errors. | `cargo clippy --package slicer-host -- -D warnings 2>&1 | grep "^error:" | wc -l` (expect 0)

- **Given** all source files are updated, **when** `cargo build --package slicer-host` runs, **when** the build completes, **then** it succeeds with zero errors. | `cargo build --package slicer-host 2>&1 | grep "^error" | head -5`

- **Given** `manifest_ingestion_tdd` runs with the wit_world filter, **when** tests execute, **then** both tests (`wit_world_mismatch_rejects_invalid_package_name`, `wit_world_major_version_mismatch_rejects_future_major`) pass. | `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture 2>&1 | tail -3`

- **Given** `live_module_loading_tdd` runs, **when** tests execute, **then** all 13 tests pass. | `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture 2>&1 | tail -3`

## Negative Test Cases

- **Given** the test uses the old version-unsuffixed key format (`slicer:world-layer/config-types/config-view`), **when** `wit_drift_detection_tdd` runs, **then** `host_bindgen_with_keys_use_canonical_world_names` panics with assertion failure. | Verifiable by reverting the fix

- **Given** `cargo clippy --package slicer-host -- -D warnings` is run before the fixes, **when** the command completes, **then** it produces errors for: unused import, large Result variants, sort_by, map_or, is_multiple_of, unnecessary unwrap, clone on Copy, too many arguments, missing docs. | Confirmed by pre-clean audit

## Verification

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture`
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture`
- `cargo clippy --package slicer-host -- -D warnings`

## Authoritative Docs

- `crates/slicer-host/src/wit_host.rs` — actual wasmtime `bindgen!` `with:` keys
- `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — failing test
- Rust Clippy documentation for each lint:
  - `clippy::result-large-err` (dag.rs, execution_plan.rs)
  - `clippy::unnecessary-sort-by` (layer_executor.rs)
  - `clippy::unnecessary-map-or` (manifest.rs)
  - `clippy::manual-is-multiple-of` (mesh_analysis.rs)
  - `clippy::unnecessary-unwrap` (prepass.rs)
  - `clippy::clone-on-copy` (region_mapping.rs, slice_postprocess.rs)
  - `clippy::too-many-arguments` (dispatch.rs)
  - `clippy::missing-docs` (wit_host.rs)

## OrcaSlicer Reference Obligations

None. This is an internal Rust/WASM tooling remediation task.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
