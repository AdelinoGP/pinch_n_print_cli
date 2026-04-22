# Requirements: 26_live-support-module-evidence

## Problem Statement

The current TASK-120b evidence in `live_support_generation_tdd.rs` consists of synthetic `HostExecutionContext` commit-helper tests that prove the commit path works but do not prove that real `tree-support.wasm` or `traditional-support.wasm` modules actually run on the production host dispatch path, produce non-empty `SupportIR` output, and remain deterministic across repeated runs. Additionally, the Benchy acceptance harness in `benchy_end_to_end_tdd.rs` does not run with support enabled and does not assert support-specific output markers.

## Grouped Task IDs

- TASK-120b (Restore support generation on the live Benchy path)
- TASK-120 (Produce a fully sliced Benchy `.gcode` with tree supports enabled as Phase H acceptance)

## In-Scope

- Split `live_support_generation_tdd.rs` into commit-path tests (keep existing) and new real live-dispatch tests
- New live-dispatch tests loading real `tree-support.wasm` and `traditional-support.wasm` via `WasmInstancePool` + `WasmRuntimeDispatcher`
- Determinism assertion across repeated runs
- Optional `SupportEnforcer`/`SupportBlocker` paint precedence case using existing `PaintRegionIR` helpers
- Extension of `run_slicer_host` helper with optional `--config JSON` file passing
- JSON config fixture under `resources/test_config/` with `support_enabled: true` and valid tree-support config keys
- Filtered module-dir fixture builder that excludes `traditional-support.wasm` so `tree-support` is the active holder
- Support-specific G-code marker assertion (`;TYPE:Support`)
- Byte-determinism assertion across two identical support-enabled Benchy runs
- `docs/07_implementation_status.md` TASK-120b status update

## Out-of-Scope

- TASK-135 matrix (seams, top/bottom fills, travel) — separate slice
- Postpass WIT repair surfaced during discovery
- Path-optimization-default changes beyond support acceptance

## Authoritative Docs

- `docs/04_host_scheduler.md` — claim system, support-generator holder resolution
- `docs/01_system_architecture.md` — Stage I/O Contract for `Layer::Support`
- `modules/core-modules/tree-support/tree-support.toml`
- `modules/core-modules/traditional-support/traditional-support.toml`
- `crates/slicer-host/tests/dispatch_tdd.rs` — production-dispatch support/paint fixtures
- `crates/slicer-host/tests/live_seam_path_tdd.rs` — pattern for loading real core-module `.wasm` on live host path
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — current Benchy harness

## Acceptance Summary

After this packet lands:
1. `live_support_generation_tdd.rs` has two clear tiers: commit-path tests and real live-dispatch tests — both green.
2. Real `tree-support.wasm` and `traditional-support.wasm` modules produce non-empty `SupportIR` on the production path.
3. Repeated identical dispatches produce byte-identical `SupportIR`.
4. Benchy acceptance runs with support enabled and produces `;TYPE:Support` markers.
5. Two identical support-enabled Benchy runs produce byte-identical output.
6. `docs/07_implementation_status.md` TASK-120b cites the real evidence.

## Verification

```
cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture
cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture
cargo build --workspace
cargo clippy --workspace -- -D warnings
```
