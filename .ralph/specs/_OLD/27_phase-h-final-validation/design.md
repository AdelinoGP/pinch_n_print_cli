# Design: 27_phase-h-final-validation

## Controlling Code Paths

1. **`modules/core-modules/build-core-modules.sh`** — rebuilds all checked-in WASM artifacts.
2. **`crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`** — IR access contract tests.
3. **`crates/slicer-host/tests/pipeline_tdd.rs`** — pipeline tests.
4. **`crates/slicer-host/tests/wit_drift_detection_tdd.rs`** — WIT drift detection tests.
5. **`crates/slicer-host/tests/live_layer_support_tdd.rs`** — live support evidence tests (this file replaced the deleted `live_support_generation_tdd.rs` after the prepass-stage rename to `SupportGeometry`, commit `b6fb366` 2026-04-30).
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
live_layer_support_tdd
live_seam_path_tdd
benchy_end_to_end_tdd
```

Each must exit 0.

### Step 3: Workspace build and clippy

Run `cargo build --workspace` and `cargo clippy --workspace -- -D warnings`. Both must exit 0.

### Step 4: Update `docs/07_implementation_status.md`

The TASK-120b live-evidence citation was already added by packet 26 (closed 2026-04-24); this packet's Step 4 work is:

1. **Verify** TASK-120b's existing entry still names `tree_support_live_dispatch_produces_non_empty_support_ir` and `traditional_support_live_dispatch_produces_non_empty_support_ir` (the two live-dispatch tests that supersede the older HostExecutionContext commit-helper-only evidence).
2. **Verify** the TASK-120 family (which folded the original "TASK-120b1" tracking into TASK-120b) still names the three Benchy-with-tree-support acceptance tests by name: `benchy_with_support_enabled`, `benchy_support_marker_present`, `benchy_support_deterministic`.
3. **Close TASK-120 itself**: change its checkbox from `[~]` to `[x]` and add a closure note referencing this packet, since this packet *is* the Phase H final validation gate that TASK-120 was waiting on. Update the "Phase H remains open …" sentence near the top of the file accordingly.

Note: the originally-proposed `TASK-120b1` child task was never created; the equivalent tracking was folded directly into TASK-120b. This packet does not introduce a new task ID.

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

## Open Questions

- None.

## Locked Assumptions

1. Packet 27 runs after Packets 24, 25, 26 complete.
2. Full workspace test suite failures are pre-existing and unrelated to this work.
3. `build-core-modules.sh` exits 0 on a clean tree.
