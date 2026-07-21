# Task Map: support-planner-geometric-correctness

Canonical backlog rows added 2026-07-19; both closed today. The prior "no current `docs/07` task IDs are mapped" framing is replaced.

| docs/07 task ID | Source-plan work item | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `TASK-281` | B5 | Steps 1-3, 6 | `docs/specs/support-modules-orca-port.md` §B5 | `support-planner/src/lib.rs::tapered_radius`, source unit tests, existing `radius_tapers_with_distance_to_top` oracle | `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::calc_branch_radius` second overload (line 1801) | S | Verbatim port of the two-piece tip-cone formula; interface-aware widening intentionally excluded per packet design. |
| `TASK-282` | B6 | Steps 1, 4-6 | `docs/specs/support-modules-orca-port.md` §B6 | `support-planner/src/lib.rs::run_support_geometry`, `LayerCollisionCache`, `slicer_sdk::host::offset_polygons` seam, source geometry tests, coordinate fixture | none additional | M | `SupportGeometryViewEntry.outlines` and the SDK API return `ExPolygon` values; no direct `slicer-core` dependency added to the guest graph. |

Aggregate context cost across rows: `M`; no row exceeds `M` and no row is L. TASK-281 and TASK-282 closed 2026-07-19 by this packet.
