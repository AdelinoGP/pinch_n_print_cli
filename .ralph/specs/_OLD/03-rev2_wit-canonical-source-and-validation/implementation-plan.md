# Implementation Plan: 03-rev2_wit-canonical-source-and-validation

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (TASK-144, TASK-145, TASK-146).
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Fix `wit_drift_detection_tdd` wrong `with:` key format

- Task IDs:
  - `TASK-144`
- Objective: Fix the `host_bindgen_with_keys_use_canonical_world_names` test to check for the correct wasmtime `bindgen!` `with:` key format. The test checks for `slicer:world-layer/config-types/config-view` but wasmtime emits `slicer:world-layer/config-types@1.0.0.config-view` (with `@1.0.0` version suffix).
- Precondition: The test `wit_drift_detection_tdd::host_bindgen_with_keys_use_canonical_world_names` panics at runtime
- Postcondition: All 4 canonical world keys in the test use version-suffixed format; all 4 prepass/postpass/finalization/postpass world keys are updated
- Files expected to change:
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs`
- Authoritative docs:
  - `crates/slicer-host/src/wit_host.rs` lines 328, 382, 579, 691 — actual `with:` keys used by wasmtime bindgen
- OrcaSlicer refs: None
- Verification:
  - `cargo test --package slicer-host --test wit_drift_detection_tdd -- host_bindgen_with_keys_use_canonical_world_names --nocapture`
- Exit condition: The test passes (does not panic) and asserts the correct version-suffixed keys

---

### Step 2: Fix all slicer-host clippy errors

- Task IDs:
  - `TASK-144`
  - `TASK-145`
  - `TASK-146`
- Objective: Fix all 10 categories of clippy errors in slicer-host exposed by Rust/Cargo 1.94.0
- Precondition: `cargo clippy --package slicer-host -- -D warnings` produces errors
- Postcondition: `cargo clippy --package slicer-host -- -D warnings` exits 0 with zero errors
- Files expected to change:
  - `crates/slicer-host/src/execution_plan.rs` — remove or move unused `ConfigFieldEntry` import; box `LiveModuleLoadError`
  - `crates/slicer-host/src/dag.rs` — box `SchedulerError`
  - `crates/slicer-host/src/layer_executor.rs` — `sort_by` → `sort_by_key`
  - `crates/slicer-host/src/manifest.rs` — `map_or` simplification
  - `crates/slicer-host/src/mesh_analysis.rs` — `is_multiple_of`
  - `crates/slicer-host/src/prepass.rs` — remove unnecessary `unwrap`
  - `crates/slicer-host/src/region_mapping.rs` — remove `.clone()` on `u64`
  - `crates/slicer-host/src/slice_postprocess.rs` — remove `.clone()` on `PaintValue` (2 sites)
  - `crates/slicer-host/src/dispatch.rs` — reduce argument count (11 → ≤7)
  - `crates/slicer-host/src/wit_host.rs` — add `#[allow(missing_docs)]` on each `pub mod` block
  - `crates/slicer-host/src/main.rs` — add `#[allow(dead_code)]` to each `Noop*Runner` stub struct
- Authoritative docs: None needed (these are Rust idiom fixes)
- OrcaSlicer refs: None
- Verification:
  - `cargo clippy --package slicer-host -- -D warnings 2>&1 | grep "^error:" | wc -l` → 0
- Exit condition: Zero clippy errors; `cargo build --package slicer-host` succeeds

---

## Packet Completion Gate

- Step 1 complete: `wit_drift_detection_tdd` passes all 9 tests
- Step 2 complete: `cargo clippy --package slicer-host -- -D warnings` exits 0
- `cargo build --package slicer-host` succeeds
- `cargo test --package slicer-host --test manifest_ingestion_tdd -- wit_world --nocapture` passes (2 tests)
- `cargo test --package slicer-host --test live_module_loading_tdd -- --nocapture` passes (13 tests)
- `docs/07_implementation_status.md` updated: TASK-144, TASK-145, TASK-146 remain marked complete (they are still complete — this is a remediation pass)
- `packet.spec.md` status updated to `implemented`
- `03-rev1_wit-canonical-source-and-validation/packet.spec.md` status updated to `superseded`

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`
- Confirm full workspace build and clippy are green for slicer-host
- Confirm drift detection test reports zero drift for all four worlds and three dependency interfaces
- Confirm all 9 wit_drift_detection_tdd tests pass
- Record any remaining packet-local risk explicitly before moving to `status: implemented`
