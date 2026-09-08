# Task Map: internal-bridge-over-infill

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. Skip it for a single-task packet unless another explicit mapping need requires it.

This packet is single-task (`ISSUE-82`) but carries an explicit mapping need: the backlog source is an issue file, not a `TASK-###` row, so the `docs/07_implementation_status.md` TASK-ID crosswalk is **N-A** (the backlog uses issue files under `docs/specs/orca-feature-gap/issues/`, not `TASK-###` IDs). The mapping below is the authoritative one.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (backlog uses issue files, not `TASK-###`) | Steps 1–6 | `docs/specs/bridge-parity-plan.md` (queue row #1, work item W-C) | `crates/slicer-core/src/algos/bridge_over_infill.rs` (new), `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/`, `crates/slicer-gcode/src/emit.rs`, `modules/core-modules/rectilinear-infill/` | `PrintObject::bridge_over_infill` (`PrintObject.cpp`) | M | `ISSUE-82` (`docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md`) → plan queue row #1 (W-C) → this packet. Single task; no `TASK-###` crosswalk row exists. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
