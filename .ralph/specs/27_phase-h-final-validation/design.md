# Design: 27_phase-h-final-validation

## Controlling Code Paths

1. **`modules/core-modules/build-core-modules.sh`** — rebuilds all checked-in WASM artifacts.
2. **`crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`** — IR access contract tests.
3. **`crates/slicer-host/tests/pipeline_tdd.rs`** — pipeline tests.
4. **`crates/slicer-host/tests/wit_drift_detection_tdd.rs`** — WIT drift detection tests.
5. **`crates/slicer-host/tests/live_support_generation_tdd.rs`** — live support evidence tests.
6. **`crates/slicer-host/tests/live_seam_path_tdd.rs`** — live seam path tests.
7. **`crates/slicer-host/tests/benchy_end_to_end_tdd.rs`** — Benchy acceptance tests.
8. **`docs/07_implementation_status.md`** — status tracking document.

## Architecture Constraints

- Packet 27 depends on Packets 24, 25, and 26 completing before it runs.
- The focused test matrix (6 test files) is the acceptance gate for all four review-finding packets.
- The full workspace test suite is NOT run because pre-existing slicer-cli failures are known and unrelated.
- The WASM rebuild step is necessary because changes in Packets 24/25 may affect guest bindings.

## Implementation Approach

### Step 1: WASM rebuild

Run `modules/core-modules/build-core-modules.sh` to rebuild all checked-in WASM artifacts. This ensures that:
- `seam-placer.wasm` is rebuilt with any changed guest glue from Packet 24/25
- Support modules are rebuilt if the new tests expose stale binaries

### Step 2: Run focused test matrix

Run all six test files in sequence:
```
core_module_ir_access_contract_tdd
pipeline_tdd
wit_drift_detection_tdd
live_support_generation_tdd
live_seam_path_tdd
benchy_end_to_end_tdd
```

Each must exit 0.

### Step 3: Workspace build and clippy

Run `cargo build --workspace` and `cargo clippy --workspace -- -D warnings`. Both must exit 0.

### Step 4: Update `docs/07_implementation_status.md`

Update TASK-120b entry to cite the real live support-module evidence:
- Old: cites HostExecutionContext commit-helper tests
- New: cites tree-support.wasm + traditional-support.wasm live dispatch tests + support-enabled Benchy acceptance

Also update Workstream 3 to track the new true Benchy-with-tree-support acceptance check. If TASK-120b1 is used as a child task for the acceptance harness, create that entry.

## Data and Contract Notes

- TASK-120b currently mixes generator restoration with acceptance evidence — the new evidence splits these concerns.
- TASK-120b1 (if created) = the real Benchy-with-tree-support acceptance harness.
- Pre-existing slicer-cli failures should be checked against the previously known failures before being treated as regressions.

## Risks and Tradeoffs

- **Risk**: WASM rebuild takes significant time.
  - Accepted: It is necessary for reproducible evidence; do it once at the start of the validation gate.
- **Risk**: Stale WASM binaries cause test failures even after rebuild.
  - Mitigation: If rebuild itself fails, that's a build infrastructure issue — stop and fix the build script first.

## Open Questions

- None.

## Locked Assumptions

1. Packet 27 runs after Packets 24, 25, 26 complete.
2. Full workspace test suite failures are pre-existing and unrelated to this work.
3. `build-core-modules.sh` exits 0 on a clean tree.
