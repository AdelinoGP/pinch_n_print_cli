---
status: implemented
packet: 03-rev2_wit-canonical-source-and-validation
task_ids:
  - TASK-144
  - TASK-145
  - TASK-146
supersedes: 03-rev1_wit-canonical-source-and-validation
---

# 03-rev2_wit-canonical-source-and-validation

## Goal

Fix all clippy errors in `slicer-host` exposed by Rust/Cargo 1.94.0 (which promotes previously-warned lints to errors under `-D warnings`) and fix a pre-existing bug in `wit_drift_detection_tdd` where the test asserts the wrong WIT `with:` block key format.

## Problem Statement

The `03_wit-canonical-source-and-validation` packet was marked `implemented` and `03-rev1` corrected remaining gaps, but a Rust/Cargo 1.94.0 update (January 2026) introduced stricter clippy lints that are now errors under `-D warnings`. Additionally, a pre-existing bug in the `wit_drift_detection_tdd.rs` test causes it to panic at runtime: the test checks for WIT `with:` block keys using the format `slicer:world-layer/config-types/config-view` but the actual wasmtime `bindgen!` macro emits `slicer:world-layer/config-types@1.0.0.config-view` (with `@1.0.0` version suffix).

The packet must fix all of these to pass the completion gate.

If this packet reopens or narrows a prior packet: this is the second revision of `03-wit-canonical-source-and-validation`, which was the first revision. `03-rev1` addressed `push-z-hop` and remaining inline WIT blocks. This rev-2 addresses clippy regressions and the test assertion bug.

## Architecture Constraints

- The boxed error approach (`Box<SchedulerError>`, `Box<LiveModuleLoadError>`) preserves the existing error semantics while reducing the Result size. This is a standard Rust pattern for large error types.
- The `#[allow(missing_docs)]` approach for `wit_host.rs` modules is the pragmatic choice — the bindgen-generated modules have auto-generated docs that would duplicate std library documentation.

## Locked Assumptions and Invariants

- The wasmtime `bindgen!` `with:` key format is `world/package@version.interface-name`. This is stable and not something this project controls.
- `u64` and `PaintValue` are `Copy` — removing `.clone()` does not change semantics.
- The boxed error types preserve the same error values — no error information is lost.

## Risks and Tradeoffs

- **dispatch.rs argument bundling**: Grouping parameters into a struct is a small API change. Any callers of the function must be updated. However, clippy only fires when there are ≥8 arguments (this function has 11), so the change surface is contained.
- **Box<SchedulerError> change**: Any code that matches on `SchedulerError` directly (without `&`) would break. Verify no such match exists before boxing.
