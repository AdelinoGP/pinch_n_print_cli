# Task Map: 26_live-support-module-evidence → docs/07

This packet maps to Workstream 3 tasks TASK-120b and TASK-120 (Benchy support evidence and Phase H acceptance).

## Task ID Mapping

| Packet Step | docs/07 Task | Notes |
|---|---|---|
| Step 2 | TASK-120b | Split `live_support_generation_tdd.rs` into commit-path and live-dispatch tiers |
| Step 3 | TASK-120b | Real `tree-support.wasm` live-dispatch test |
| Step 4 | TASK-120b | Real `traditional-support.wasm` live-dispatch test |
| Step 5 | TASK-120b | Determinism assertion across repeated runs |
| Step 7 | TASK-120 | Extend `run_slicer_host` with `--config` support |
| Step 8 | TASK-120 | JSON config fixture for tree-support |
| Step 9 | TASK-120 | Filtered module-dir builder (tree-support active holder) |
| Step 10 | TASK-120 | Support-enabled Benchy acceptance tests |
| Step 11 | TASK-120b | Update `docs/07_implementation_status.md` TASK-120b status |

## Superseding Relationship

- Packet 26 does NOT supersede any prior packet. It upgrades the evidence for TASK-120b from synthetic commit-helper tests to real live-dispatch tests, and adds the true Benchy-with-tree-support acceptance harness.

## docs/07 Reconciliation Note

**TASK-120b appears as `[x]` (closed 2026-04-21) in docs/07**, citing `live_support_generation_tdd.rs` and "6 integration tests proving host-path commit of SupportMaterial paths". However, those existing tests use the synthetic `HostExecutionContext` commit-helper harness — they do not load real `.wasm` binaries or exercise the production `WasmRuntimeDispatcher`/`LayerStageRunner::run_stage` path.

This packet (26) clarifies the TASK-120b scope: the commit-helper evidence was a necessary intermediate step but is insufficient for Phase H acceptance. Step 11 of this packet updates the TASK-120b entry in docs/07 to cite the real live-dispatch evidence and reflect that the task was reopened to add that stronger evidence. The docs/07 TASK-120b entry should not be treated as finally closed until Step 11 of this packet lands.

## Parallelism Note

- Packet 26 runs in parallel with Packets 24 and 25. It does not block on either of those tracks.
