# Task Map: 108_perimeter-special-modes-and-seam

Maps packet task IDs (T-070..T-083, T-074b/c/d, T-077, T-P98-SEAM) to their source rows in the roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 7 (T-070..T-077), Phase 8 (T-080..T-083), and "Inherited from P98" (T-P98-SEAM).

## Phase 7 — Classic special modes

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-070 | Register `extra_perimeters` config key; ensure it flows through `RegionMapIR` → `ConfigView` per D-5 | Phase 7 | Step 1 | done |
| T-071 | Honour `extra_perimeters` config bonus: `loop_number = wall_count + extra_perimeters - 1` (Orca line 1569) | Phase 7 | Step 1 | done |
| T-072 | Register `smaller_perimeter_line_width`, `smaller_perimeter_threshold_mm`, `narrow_loop_length_threshold_mm` config keys | Phase 7 | Step 2 | done |
| T-073 | Implement narrow-island handling: islands < threshold use `smaller_ext_perimeter_flow` (Orca lines 1611-1628) | Phase 7 | Step 2 | done |
| T-074b | Detect non-planar regions via `region.nonplanar_surface.is_some()`; emit `LoopType::NonPlanarShell` walls instead of `Outer`/`Inner` | Phase 7 | Step 2 | done |
| T-074c | Read `SurfaceGroup.shell_count` from Blackboard; override `wall_count` for non-planar regions | Phase 7 | Step 2 | done |
| T-074d | Skip thin-wall, gap-fill, and `infill_areas` emission for non-planar regions | Phase 7 | Step 2 | done |
| T-077 | Register `extra_perimeters_on_overhangs`; add extra perimeters in regions covered by `SliceRegionView::overhang_areas()` | Phase 7 | Step 5 | done |

## Phase 8 — Seam-candidate quality

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-080 | Replace every-vertex-candidate heuristic with sharp-corner threshold (config key `seam_candidate_angle_threshold_deg`, default ≈30°) | Phase 8 | Step 3 | done |
| T-081 | Register `seam_candidate_angle_threshold_deg` config key in `docs/15_config_keys_reference.md` and both `.toml` manifests | Phase 8 | Step 3 | done |
| T-082 | Audit `seam-placer/src/lib.rs` for dependency on dense candidate lists; document in roadmap if downstream contract requires changes | Phase 8 | Step 4 | done |
| T-083 | Confirm/document interaction with `seam-planner-default`: does its PrePass output feed perimeter-side candidate generation? | Phase 8 | Step 4 | done |

## Inherited from P98 — paint_seam stroke consumption obligation

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-P98-SEAM | Consume `seam_enforcer`/`seam_blocker` painted semantics in seam-candidate generation + seam-placer selection; supersede `D-98-SEAM-NO-CONSUMER` | Phase 8 (P98-inherited) | Step 4 | done |

## Deletion — fake arachne-perimeters

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-090 | Delete `modules/core-modules/arachne-perimeters/` (the 512-line fake iterative-inset module — NOT real Arachne). P110 creates a fresh real-Arachne skeleton later. | M1 cleanup | Step 0 | done |
| T-091 | Remove `arachne-perimeters` workspace member entry from root `Cargo.toml`; confirm `cargo build --workspace` green. | M1 cleanup | Step 0 | done |
| T-092 | Remove all doc/spec references to the fake `com.core.arachne-perimeters` / `arachne-perimeters` M1 module; leave historical/decision context intact. | M1 cleanup | Step 0 | done |

## Deferred / Deviation Registrations

| Deviation ID | Reason | Registered in Step |
| --- | --- | --- |
| `D-108-SEAM-CONSUMED` | Supersedes the `D-98-SEAM-NO-CONSUMER` note in `docs/07_implementation_status.md`; registered in `docs/DEVIATION_LOG.md` at packet close | Step 4 |

## Forward Dependencies

| Symbol | Producing Packet | Status | Impact |
| --- | --- | --- | --- |
| `SliceRegionView::surface_group()` | P104 (T-023) | draft — FORWARD-DEP | T-074b/c/d blocked until P104 ships |
| `SliceRegionView::overhang_areas()` | P104 (T-023) | draft — FORWARD-DEP | T-077 blocked until P104 + P107 ship |
| `OverhangRegion.xy_footprint` population | P106 (O-T010) | draft — FORWARD-DEP | T-077 blocked until P106 ships |
| Wall-sequence / spacing model | P105 (T-051..T-054c) | draft — FORWARD-DEP | T-074d (ThinWall/GapFill skip) needs P105's variants |
