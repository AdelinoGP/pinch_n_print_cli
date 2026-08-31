# Task Map: 239d-support-coarse-floating-planes

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. Skip it for a single-task packet unless another explicit mapping need requires it.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-523` | `Step 1` | `docs/07_implementation_status.md` (record), `docs/specs/support-independent-layer-z-split-plan.md` | `crates/slicer-core/src/algos/support_geometry.rs` (read-only), both planner `lib.rs` (read-only) | — | `S` | Measure-first: decimation reconciliation (tree planner has no `support_step`; host schedule never reaches the meshed-object planner path — tree's only `_support_geometry` read is the mesh-less legacy fallback, traditional param never read), coarse baseline (0.3/0.2 → 0 off-grid rows), disabled baseline for AC-N1 |
| `TASK-524` | `Step 2` | `docs/spec_packets/239c-support-layer-height-producer/design.md` | `modules/core-modules/tree-support-planner/src/lib.rs`, `tests/tree_family_tdd.rs` | `SupportMaterial.cpp` `raft_and_intermediate_support_layers`; `SupportCommon.cpp` `generate_support_layers` | `M` | Tree coarse derivation: bracket demanded interface/contact planes, stack at `dist/n`, grouping/midpoint; AC-2 + AC-N3 tests |
| `TASK-525` | `Step 3` | `docs/spec_packets/239c-support-layer-height-producer/design.md` | `modules/core-modules/traditional-support-planner/src/lib.rs`, `tests/traditional_family_tdd.rs` | `SupportMaterial.cpp` `raft_and_intermediate_support_layers` | `M` | Traditional twin + `support_step` neutralization when pitch >= gap; AC-3 test |
| `TASK-526` | `Step 4` | `docs/spec_packets/239c-support-layer-height-producer/packet.spec.md` (test-naming convention) | `crates/slicer-runtime/tests/integration/support_family_closure.rs`, `tests/integration/main.rs` | — | `M` | Real-slice AC-1 (off-grid rows + E>0 on every off-grid support row) and AC-N1 (disabled baseline) with the wrapper convention |
| `TASK-527` | `Step 5` | `docs/07_implementation_status.md` (TASK-519 pattern) | `crates/slicer-gcode/src/emit.rs` (read-only) | `GCode.cpp` `_extrude` (comparison target) | `S` | Measure-first coarse `height_delta` verdict (applied height vs plane delta at E for a 0.3-pitch row) |
| `TASK-528` | `Step 6` | `docs/07_implementation_status.md` (TASK-527 record) | `crates/slicer-gcode/tests/gcode_emit_tdd.rs` | — | `S` | Verdict test asserting the recorded branch (AC-4) |
| `TASK-529` | `Step 7` | `docs/19_visual_debug.md` (delegated) | `tmp/` artifacts, `tmp/239d-human-validation.md` | — | `M` | Human-gate artifacts: 0.3-pitch slices, visual-debug bundle, gate document |
| `TASK-530` | `Step 8` | `docs/07_implementation_status.md` (delegated), `docs/specs/support-independent-layer-z-split-plan.md`, `docs/specs/support-parity-gap-register.md` | docs only | — | `M` | Registration (TASK-523..530, queue row 4, gap row) + closure gates + full suite |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
