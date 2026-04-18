# Task Map: 03_rev1_wit-canonical-source-and-validation

This packet reopens TASK-144, TASK-145, and TASK-146 from `docs/07_implementation_status.md` (marked `[x]` but incomplete per audit). It does not supersede any prior packet — it completes the work the original `03_wit-canonical-source-and-validation` claimed to have done.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-145` | Step 1 (push-z-hop in disk) | `docs/03_wit_and_manifest.md` | `wit/world-postpass.wit` | None | Add missing method to canonical disk |
| `TASK-144` | Step 2 (postpass dep includes) | `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/wit_host.rs` | None | Add dep includes to postpass bindgen block |
| `TASK-144` | Step 3 (all worlds dep includes) | `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/wit_host.rs` | None | Add dep includes to remaining 3 world blocks |
| `TASK-146` | Step 4 (clippy gate) | None | `crates/slicer-core/src/triangle_mesh_slicer.rs`, `crates/slicer-core/src/paint_region.rs` | None | Fix 3 clippy errors |
| `TASK-144,145,146` | Step 5 (re-verification) | `docs/03_wit_and_manifest.md` | Workspace-wide | None | Re-run all acceptance criteria |