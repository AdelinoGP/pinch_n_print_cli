---
status: implemented
packet: 27_phase-h-final-validation
task_ids:
  - TASK-120
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 27_phase-h-final-validation

## Goal

Run the Phase H final validation gate: rebuild any checked-in WASM artifacts whose live tests depend on changed bindings or manifests, run the focused test matrix for all four review-finding packets, and confirm `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` pass before declaring the review findings resolved and closing TASK-120.

## Scope Boundaries

- **In scope:** WASM artifact rebuild via `modules/core-modules/build-core-modules.sh` (run via Git Bash on Windows or any POSIX shell on Linux/macOS); focused test matrix run (6 test files); workspace build and clippy verification; `docs/07_implementation_status.md` TASK-120 closure plus verification that TASK-120b's existing live-evidence citation (added by packet 26) still references the live support-module tests.
- **Out of scope:** Full workspace test suite (known slicer-cli-only failures are pre-existing); new feature development; broader doc changes beyond TASK-120 closure and TASK-120b verification.

## Prerequisites and Blockers

- **Depends on:** Packets 24, 25, and 26 (all review-finding tracks must be complete before Packet D runs).
- **Unblocks:** Phase H acceptance gate review readiness.
- **Activation blockers:** Packets 24, 25, 26 all complete.

## Acceptance Criteria

- **Given** `modules/core-modules/build-core-modules.sh`, **when** it runs, **then** it exits 0 and produces updated `.wasm` artifacts for `seam-placer.wasm` and any support modules whose tests were affected by changed bindings or manifests. | `./modules/core-modules/build-core-modules.sh 2>&1 | tail -20; echo "EXIT: $?"`
- **Given** all four packets are implemented, **when** `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` runs, **then** all tests pass. | `cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture 2>&1 | tail -10`
- **Given** all four packets are implemented, **when** `cargo test -p slicer-host --test pipeline_tdd` runs, **then** all tests pass. | `cargo test -p slicer-host --test pipeline_tdd -- --nocapture 2>&1 | tail -10`
- **Given** all four packets are implemented, **when** `cargo test -p slicer-host --test wit_drift_detection_tdd` runs, **then** all tests pass. | `cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture 2>&1 | tail -10`
- **Given** all four packets are implemented, **when** `cargo test -p slicer-host --test live_layer_support_tdd` runs, **then** all tests pass including the real live-dispatch tier (`tree_support_live_dispatch_produces_non_empty_support_ir`, `traditional_support_live_dispatch_produces_non_empty_support_ir`, `support_deterministic_across_repeated_runs`, `support_enforcer_blocker_paint_precedence`). Note: this file replaced the deleted `live_support_generation_tdd.rs` after the prepass-stage rename to `SupportGeometry` (commit `b6fb366`, 2026-04-30). | `cargo test -p slicer-host --test live_layer_support_tdd -- --nocapture 2>&1 | tail -10`
- **Given** all four packets are implemented, **when** `cargo test -p slicer-host --test live_seam_path_tdd` runs, **then** all tests pass. | `cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture 2>&1 | tail -10`
- **Given** all four packets are implemented, **when** `cargo test -p slicer-host --test benchy_end_to_end_tdd` runs, **then** all tests pass including the support-enabled Benchy acceptance tests. | `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture 2>&1 | tail -10`
- **Given** all four packets are implemented, **when** `cargo build --workspace` runs, **then** it exits 0. | `cargo build --workspace 2>&1 | tail -5`
- **Given** all four packets are implemented, **when** `cargo clippy --workspace -- -D warnings` runs, **then** it exits 0 with no warnings. | `cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
- **Given** `docs/07_implementation_status.md`, **when** the TASK-120b entry is read, **then** it cites the real live `tree-support.wasm` and `traditional-support.wasm` dispatch tests by name (`tree_support_live_dispatch_produces_non_empty_support_ir` and `traditional_support_live_dispatch_produces_non_empty_support_ir`), not just `HostExecutionContext` commit-helper tests. This was added by packet 26 (closed 2026-04-24); this AC verifies it is still intact. | `grep -A5 'TASK-120b ' docs/07_implementation_status.md | grep -E 'tree_support_live_dispatch_produces_non_empty_support_ir|traditional_support_live_dispatch_produces_non_empty_support_ir'`
- **Given** `docs/07_implementation_status.md`, **when** Workstream 3 is reviewed, **then** the TASK-120 family explicitly tracks the Benchy-with-tree-support acceptance check by naming `benchy_with_support_enabled`, `benchy_support_marker_present`, and `benchy_support_deterministic` (the three tests in `benchy_end_to_end_tdd.rs` that exercise the support-enabled path against `resources/test_config/benchy-tree-support.json`). | `grep -A20 'TASK-120b ' docs/07_implementation_status.md | grep -E 'benchy_with_support_enabled|benchy_support_marker_present|benchy_support_deterministic'`
- **Given** all nine verification commands above pass, **when** `docs/07_implementation_status.md` is read, **then** TASK-120 is marked complete (`[x]`) and Phase H is no longer blocked on the live Benchy run. | `grep -E '^\- \[x\] TASK-120 ' docs/07_implementation_status.md`

## Negative Test Cases

- **Given** a WASM artifact that was not rebuilt after manifest or bindings changes, **when** the live dispatch tests run, **then** they fail with a stale-binding error before the rebuild step is reached. | Run without rebuild: `cargo test -p slicer-host --test live_layer_support_tdd -- --nocapture 2>&1 | grep -i 'stale\|bind\|version' | head -5`
- **Given** a focused test that fails, **when** `cargo build --workspace` is run, **then** it may still pass (structural build integrity is independent of test correctness). | Verify separately: `cargo build --workspace 2>&1 | tail -3`

## Verification

- `./modules/core-modules/build-core-modules.sh`
- `cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture`
- `cargo test -p slicer-host --test pipeline_tdd -- --nocapture`
- `cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture`
- `cargo test -p slicer-host --test live_layer_support_tdd -- --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/07_implementation_status.md`
- `docs/11_operational_governance_and_acceptance_gate.md`
- `docs/12_architecture_gate_metrics.md`

## OrcaSlicer Reference Obligations

- None.
