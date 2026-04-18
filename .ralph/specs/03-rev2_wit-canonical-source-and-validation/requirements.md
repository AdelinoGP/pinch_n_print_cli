# Requirements: 03-rev2_wit-canonical-source-and-validation

## Packet Metadata

- Grouped task IDs:
  - `TASK-144` — Consolidate WIT source (clippy remediation)
  - `TASK-145` — Normalize WIT identifiers (clippy remediation)
  - `TASK-146` — Add host-side allowlist validation (clippy remediation)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `03-rev1_wit-canonical-source-and-validation`

## Problem Statement

The `03_wit-canonical-source-and-validation` packet was marked `implemented` and `03-rev1` corrected remaining gaps, but a Rust/Cargo 1.94.0 update (January 2026) introduced stricter clippy lints that are now errors under `-D warnings`. Additionally, a pre-existing bug in the `wit_drift_detection_tdd.rs` test causes it to panic at runtime: the test checks for WIT `with:` block keys using the format `slicer:world-layer/config-types/config-view` but the actual wasmtime `bindgen!` macro emits `slicer:world-layer/config-types@1.0.0.config-view` (with `@1.0.0` version suffix).

The packet must fix all of these to pass the completion gate.

If this packet reopens or narrows a prior packet: this is the second revision of `03-wit-canonical-source-and-validation`, which was the first revision. `03-rev1` addressed `push-z-hop` and remaining inline WIT blocks. This rev-2 addresses clippy regressions and the test assertion bug.

## In Scope

- TASK-144/145/146: Fix the `wit_drift_detection_tdd` test assertion for the correct `with:` block key format (version-suffixed)
- TASK-144/145/146: Fix all slicer-host clippy errors exposed by Rust/Cargo 1.94.0:
  - `execution_plan.rs:12` — unused `ConfigFieldEntry` import
  - `dag.rs:27` — `Result<Vec<ModuleNode>, SchedulerError>` Err variant > 128 bytes
  - `execution_plan.rs:298` — `Result<LiveModuleLoadOutput, LiveModuleLoadError>` Err variant > 128 bytes
  - `layer_executor.rs:263` — `sort_by` → `sort_by_key`
  - `manifest.rs:359` — `map_or` simplification
  - `mesh_analysis.rs:119` — manual `is_multiple_of` → `.is_multiple_of(3)`
  - `prepass.rs:219` — unnecessary unwrap after `is_some` check on `ir_path`
  - `region_mapping.rs:119` — `.clone()` on `u64` (which is `Copy`)
  - `slice_postprocess.rs:296,310` — `.clone()` on `PaintValue` (which is `Copy`)
  - `dispatch.rs:253` — function has 11 arguments (max 7)
  - `wit_host.rs:177,381,578,690` — missing doc comments on `pub mod` blocks
  - `wit_host.rs:862,866,868,870,874,876` — missing doc comments on struct fields

## Out of Scope

- New WIT functionality or schema changes
- Changes to `slicer-macros` or `slicer-core` (both are clippy-clean)
- Custom payload widening (TASK-149/150) — separate packet `04_custom-payload-widening`
- Changes to IR schema versions

## Authoritative Docs

- `crates/slicer-host/src/wit_host.rs` — `with:` block key format (wasmtime bindgen output)
- `crates/slicer-host/tests/wit_drift_detection_tdd.rs` — failing test
- `crates/slicer-host/src/execution_plan.rs` — large Result fix
- `crates/slicer-host/src/dag.rs` — large Result fix
- `crates/slicer-host/src/layer_executor.rs` — sort_by fix
- `crates/slicer-host/src/manifest.rs` — map_or fix
- `crates/slicer-host/src/mesh_analysis.rs` — is_multiple_of fix
- `crates/slicer-host/src/prepass.rs` — unnecessary unwrap fix
- `crates/slicer-host/src/region_mapping.rs` — clone on Copy fix
- `crates/slicer-host/src/slice_postprocess.rs` — clone on Copy fix
- `crates/slicer-host/src/dispatch.rs` — too many arguments fix
- `crates/slicer-host/src/wit_host.rs` — missing docs fix

## OrcaSlicer Reference Obligations

None. This is an internal Rust/WASM tooling remediation task.

## Acceptance Summary

- Positive cases:
  - `wit_drift_detection_tdd` passes all 9 tests (including `host_bindgen_with_keys_use_canonical_world_names`)
  - `manifest_ingestion_tdd -- wit_world` passes (2 tests)
  - `live_module_loading_tdd` passes (13 tests)
  - `cargo clippy --package slicer-host -- -D warnings` exits 0 with zero errors
- Negative cases:
  - The test assertion for `with:` keys must use the version-suffixed format
- Measurable outcomes:
  - Exactly 0 clippy errors in slicer-host
  - Exactly 9/9 wit_drift_detection_tdd tests pass
  - Exactly 2/2 manifest_ingestion_tdd wit_world tests pass
  - Exactly 13/13 live_module_loading_tdd tests pass
- Cross-packet impact:
  - `04_custom-payload-widening` remains blocked until this packet is complete (clippy must be green)

## Verification Commands

- `cargo build --package slicer-host`
- `cargo test --package slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture`
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture`
- `cargo clippy --package slicer-host -- -D warnings`

## Step Completion Expectations

Each step in `implementation-plan.md` must produce:
- Step 1 (test fix): `wit_drift_detection_tdd` test updated to version-suffixed key format; all 9 tests pass
- Step 2 (clippy fixes): All 10 clippy error categories resolved; clippy exits 0 with `-D warnings`
