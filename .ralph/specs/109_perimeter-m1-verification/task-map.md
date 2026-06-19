# Task Map: 109_perimeter-m1-verification

Maps packet task IDs (T-100..T-105, T-P96-A, T-P96-C3, T-P96-D, T-P96-F) to their source rows in the roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 9 (T-100..T-105) and "Inherited from P96" (T-P96-A, T-P96-C3, T-P96-D, T-P96-F).

## Phase 9 — Verification

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-100 | Build reference-fixture parity harness under `crates/slicer-runtime/tests/integration/perimeter_parity.rs` | Phase 9 | Step 1 | pending |
| T-101 | Record OrcaSlicer reference outputs for 6 M1 fixtures: solid square, holed square, multi-tool triangle, overhang ramp, bridge fixture, spiral-vase cone | Phase 9 | Step 2 | pending |
| T-102 | TDD sweep for 7 edge cases: 3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override | Phase 9 | Step 3 | pending |
| T-103 | Walk every M1 deviation entry from T-003; close each with implementing task ID, or document residual deviation | Phase 9 | Step 6 | pending |
| T-104 | Update `docs/07_implementation_status.md` to mark Classic parity complete | Phase 9 | Step 6 | pending |
| T-105 | Run `cargo test --workspace` once at M1 close (closure ceremony) | Phase 9 | Step 7 | pending |

## Inherited from P96 — AC-22b reshape + cleanup

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-P96-A | Reshape AC-22b assertion from union-baseline to per-color fragmentation; rename test to `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` | Phase 9 (P96-inherited) | Step 4 | pending |
| T-P96-C3 | Parity verification: golden-file check of `cube_4color` G-code against recorded OrcaSlicer reference | Phase 9 (P96-inherited) | Step 4 | pending |
| T-P96-D | Delete unused `external_contour` IR field after T-P96-A through T-P96-C3 land GREEN; bump `CURRENT_SLICE_IR_SCHEMA_VERSION` | Phase 1 (P96-inherited, deferred to P109) | Step 5 | pending |
| T-P96-F | Re-baseline cube_4color SHA; add `D-109-AC22-PARITY-RESHAPE` deviation entry superseding `D-96-AC22-EXTERNAL-CONTOUR` | Phase 9 (P96-inherited) | Step 6 | pending |

## Forward Dependencies

| Symbol | Producing Packet | Status | Impact |
| --- | --- | --- | --- |
| Wall-emission stack (outer/inner widths, wall_sequence, ThinWall, GapFill) | P105 | draft — FORWARD-DEP | Parity harness (T-101) baselines need this; T-P96-D BLOCKED on P105 shipping `bisector_edge_skip_mask` |
| `OverhangRegion.xy_footprint` + overhang quartile polygons | P106 | draft — FORWARD-DEP | Overhang-ramp fixture (T-101) meaningful only after P106 ships |
| `overhang_quartile_polygons()` view accessor | P107 | draft — FORWARD-DEP | Closes D-104-OVERHANG-QUARTILE-NONE (referenced in AC-6) |
| Special modes + seam quality | P108 | draft — FORWARD-DEP | Spiral-vase-cone + bridge fixtures produce parity-correct output only after P108 ships |

## Deferred / Deviation Registrations

| Deviation ID | Reason | Registered in Step |
| --- | --- | --- |
| `D-109-AC22-PARITY-RESHAPE` | Registers cube_4color SHA + supersedes `D-96-AC22-EXTERNAL-CONTOUR` per ADR-0013 | Step 6 (T-P96-F) |
