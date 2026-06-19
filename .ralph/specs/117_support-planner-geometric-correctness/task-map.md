# Task Map: support-planner-geometric-correctness

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-254` | Step 1, Step 2, Step 3, Step 6 | `docs/specs/support-modules-orca-port.md` §B5 | `modules/core-modules/support-planner/src/lib.rs::tapered_radius` (current line 888) + new `tests/tapered_radius_tip_cone.rs` + migrated `radius_tapers_with_distance_to_top` in existing `tests/orca_parity_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::calc_branch_radius` second overload (delegate; never load) | S | Step 2 RED; Step 3 GREEN; Step 6 gates AC-1, AC-2, AC-3, AC-4, AC-N1. |
| `TASK-255` | Step 1, Step 4, Step 5, Step 6 | `docs/specs/support-modules-orca-port.md` §B6 | `modules/core-modules/support-planner/src/lib.rs` — delete `inflate_polygon` (current line 901); substitute `slicer_core::polygon_ops::offset` at the call site around line 226; new `tests/avoidance_offset_concave.rs` | none (Orca's Clipper2 use is described in §B6 directly) | S | Step 4 RED; Step 5 GREEN; Step 6 gates AC-5, AC-6, AC-7. Verify `slicer-core` is already a path dependency in `support-planner/Cargo.toml`. |

Aggregate context cost across rows: `S`. No row exceeds `S`; no row L.
