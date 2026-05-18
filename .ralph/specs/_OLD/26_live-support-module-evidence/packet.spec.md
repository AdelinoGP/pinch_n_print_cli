---
status: implemented
packet: 26_live-support-module-evidence
task_ids:
  - TASK-120b
  - TASK-120
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 26_live-support-module-evidence

## Goal

Replace the synthetic `HostExecutionContext` commit-helper tests with real live-dispatch tests that load checked-in `tree-support.wasm` and `traditional-support.wasm` modules, run `Layer::Support` through the production `WasmRuntimeDispatcher`/`LayerStageRunner::run_stage` path, and prove deterministic non-empty `SupportIR` output. Then add a true support-enabled Benchy acceptance harness that uses a filtered module directory so `tree-support` is the active support holder, and asserts support-specific output markers in the emitted `.gcode`.

## Scope Boundaries

- **In scope:** Split of `live_support_generation_tdd.rs` into commit-path tests and new real live-dispatch tests; `tree-support.wasm` and `traditional-support.wasm` loading on the production host path; `WasmRuntimeDispatcher`/`LayerStageRunner::run_stage` integration; deterministic repeated-run assertion; optional `SupportEnforcer`/`SupportBlocker` paint precedence case; `benchy_end_to_end_tdd.rs` extension with `--config` JSON wiring, filtered tree-support module-dir builder fixture, support-specific G-code assertion; JSON config fixture under `resources/test_config/`; `docs/07_implementation_status.md` TASK-120b status update.
- **Out of scope:** TASK-135 matrix (seams, top/bottom fills, travel) in the same slice; broader `path-optimization-default` changes; postpass WIT repair surfaced during discovery.

## Prerequisites and Blockers

- **Depends on:** None (this track runs in parallel with Tracks 1 and 2).
- **Unblocks:** `docs/07_implementation_status.md` TASK-120b closure note update.
- **Activation blockers:** None.

## Acceptance Criteria

- **Given** `crates/slicer-host/tests/live_support_generation_tdd.rs`, **when** the test suite runs, **then** it distinguishes commit-path tests (existing `commit_layer_outputs_for_test` harness) from real live-dispatch tests (new `WasmRuntimeDispatcher` + real `.wasm` loading harness) and both tiers pass. | `cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture 2>&1 | tail -30`
- **Given** a real `tree-support.wasm` loaded via `WasmInstancePool::get` and dispatched via `WasmRuntimeDispatcher::dispatch_layer_call` for `Layer::Support`, **when** the dispatch completes, **then** the resulting `SupportIR.support_paths` is non-empty and each path has `ExtrusionRole::SupportMaterial`. | `cargo test -p slicer-host --test live_support_generation_tdd tree_support_live_dispatch_produces_non_empty_support_ir -- --nocapture 2>&1 | tail -20`
- **Given** a real `traditional-support.wasm` loaded via `WasmInstancePool::get` and dispatched via `WasmRuntimeDispatcher::dispatch_layer_call` for `Layer::Support`, **when** the dispatch completes, **then** the resulting `SupportIR.support_paths` is non-empty and each path has `ExtrusionRole::SupportMaterial`. | `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_live_dispatch_produces_non_empty_support_ir -- --nocapture 2>&1 | tail -20`
- **Given** two identical `Layer::Support` dispatches for the same layer index, **when** they run sequentially, **then** both produce byte-identical `SupportIR.support_paths` (determinism). | `cargo test -p slicer-host --test live_support_generation_tdd support_deterministic_across_repeated_runs -- --nocapture 2>&1 | tail -20`
- **Given** the `run_slicer_host` helper extended with optional `--config JSON` support, **when** it is called with a JSON config that enables `support_enabled` and points to `tree-support`, **then** the real binary exits successfully and produces a `.gcode` file. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --nocapture 2>&1 | tail -20`
- **Given** the filtered module-dir builder fixture that excludes `traditional-support.wasm`, **when** it is used to stage modules for a Benchy run, **then** `tree-support` is the active support-generator holder (claim wins). | `cargo test -p slicer-host --test benchy_end_to_end_tdd tree_support_active_holder -- --nocapture 2>&1 | tail -20`
- **Given** a support-enabled Benchy acceptance run, **when** the run completes, **then** the emitted `.gcode` contains at least one support-specific marker (`;TYPE:Support` or `;TYPE:Support interface`) and two identical runs produce byte-identical output. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_support_marker_present benchy_support_deterministic -- --nocapture 2>&1 | tail -20`
- **Given** `resources/test_config/benchy-tree-support.json` (or similar), **when** it is loaded by the acceptance test, **then** it contains `support_enabled: true` and keys matching the `tree-support.toml` `config.schema` (`support_density`, `support_angle`, `support_speed`, `line_width`). | `grep -E 'support_enabled|support_density|support_angle|support_speed|line_width' resources/test_config/benchy-tree-support.json`

## Negative Test Cases

- **Given** a Benchy acceptance run without support enabled, **when** the output is checked for support markers, **then** no `;TYPE:Support` or `;TYPE:Support interface` markers appear. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_no_support -- --nocapture 2>&1 | tail -20`

Note: The stale-binary failure mode is covered by the positive ACs (30–32): if the `.wasm` binaries are stale or empty, `SupportIR.support_paths` will be empty and those tests will fail. No separate negative AC is needed.

## Verification

- `cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/04_host_scheduler.md` — claim system, support-generator holder resolution
- `docs/01_system_architecture.md` — Stage I/O Contract for `Layer::Support`
- `modules/core-modules/tree-support/tree-support.toml`
- `modules/core-modules/traditional-support/traditional-support.toml`

## OrcaSlicer Reference Obligations

- None.
