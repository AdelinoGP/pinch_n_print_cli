# Task Map: support-planner-geometric-correctness

The batch anchor names B5/B6 as `TASK-254`/`TASK-255`, but current `docs/07_implementation_status.md` assigns those IDs to unrelated infill work. The closed broad `TASK-163 (algorithmic)` row mentions radius tapering but does not own this still-present tip-floor or offset replacement. No replacement IDs are invented here.

| docs/07 task ID | Source-plan work item | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `— (unmapped B5)` | B5 | Steps 1-3, 6 | `docs/specs/support-modules-orca-port.md` §B5 | `support-planner/src/lib.rs::tapered_radius`, source unit tests, existing `radius_tapers_with_distance_to_top` oracle | `TreeSupport.cpp::calc_branch_radius` second overload; delegated | S | Do not reuse closed `TASK-163 (algorithmic)` or colliding `TASK-254`. |
| `— (unmapped B6)` | B6 | Steps 1, 4-6 | `docs/specs/support-modules-orca-port.md` §B6 | `support-planner/src/lib.rs::run_support_geometry`, `LayerCollisionCache`, existing `slicer_sdk::host::offset_polygons` seam, source geometry tests, coordinate fixture | none additional | M | `SupportGeometryViewEntry.outlines` and the SDK API return `ExPolygon` values; no direct `slicer-core` dependency may be added to the guest graph. |

Aggregate context cost across rows: `M`; no row exceeds `M` and no row is L. Activation is blocked until the backlog maintainer supplies canonical IDs.
