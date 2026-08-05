---
status: implemented
packet: 27_phase-h-final-validation
task_ids:
  - TASK-120
---

# 27_phase-h-final-validation

## Goal

Run the Phase H final validation gate: rebuild any checked-in WASM artifacts whose live tests depend on changed bindings or manifests, run the focused test matrix for all four review-finding packets, and confirm `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` pass before declaring the review findings resolved and closing TASK-120.

## Problem Statement

After Packets 24, 25, and 26 land, a final validation gate is needed to: (1) rebuild any checked-in WASM artifacts whose live tests depend on changed bindings or manifests, (2) run the focused test matrix covering all six affected test files, (3) confirm workspace build and clippy pass, and (4) update `docs/07_implementation_status.md` to close TASK-120 and verify TASK-120b's live-evidence citation (added by packet 26) still references the real live support-module tests and the Benchy-with-tree-support acceptance check.

Note: the test file originally named `live_support_generation_tdd.rs` was deleted in commit `b6fb366` (2026-04-30) when the prepass support stage was renamed to `SupportGeometry`. Its live-dispatch tests now live in `crates/slicer-host/tests/live_layer_support_tdd.rs`.

## Architecture Constraints

- Packet 27 depends on Packets 24, 25, and 26 completing before it runs.
- The focused test matrix (6 test files) is the acceptance gate for all four review-finding packets.
- The full workspace test suite is NOT run because pre-existing slicer-cli failures are known and unrelated.
- The WASM rebuild step is necessary because changes in Packets 24/25 may affect guest bindings.

## Data and Contract Notes

- TASK-120b's evidence was rewritten by packet 26 (closed 2026-04-24) to cite live-dispatch tests; this packet only verifies the citation is intact.
- The originally-proposed TASK-120b1 child task was never created. The Benchy-with-tree-support acceptance tracking was folded directly into TASK-120b. AC-11 verifies the resulting test names appear in Workstream 3 rather than asserting on a separate task ID.
- Pre-existing slicer-cli failures should be checked against the previously known failures before being treated as regressions.
- `build-core-modules.sh` is bash; on Windows it runs from Git Bash (the project's standard Windows shell setup). On Linux/macOS it runs from any POSIX shell.

## Risks and Tradeoffs

- **Risk**: WASM rebuild takes significant time.
  - Accepted: It is necessary for reproducible evidence; do it once at the start of the validation gate.
- **Risk**: Stale WASM binaries cause test failures even after rebuild.
  - Mitigation: If rebuild itself fails, that's a build infrastructure issue — stop and fix the build script first.
