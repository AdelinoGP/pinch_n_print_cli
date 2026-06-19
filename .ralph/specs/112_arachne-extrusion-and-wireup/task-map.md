# Task Map: 112_arachne-extrusion-and-wireup

Maps packet task IDs (T-220..T-234) to their source rows in the roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 12 (T-220..T-227) and Phase 13 (T-230..T-234).

## Phase 12 — Extrusion generation

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-220 | Port centrality filtering (`filterCentral`, `filterNoncentralRegions`) into `skeletal_trapezoidation/centrality.rs` | Phase 12 | Step 1 | pending |
| T-221 | Bead-count assignment on central edges (`optimal_bead_count(R)` per edge) — `assign_bead_counts` in `bead_count.rs` | Phase 12 | Step 2 | pending |
| T-222 | Port bead-count upward + downward propagation (`propagateBeadingsUpward`, `propagateBeadingsDownward`); marks `TransitionMiddle` / `TransitionEnd` | Phase 12 | Step 3 | pending |
| T-223 | Port `generateToolpaths()` — emits `Vec<VariableWidthLines>` sorted by `inset_idx` — in `arachne/generate_toolpaths.rs` | Phase 12 | Step 4 | pending |
| T-224 | Define `ExtrusionLine { junctions, inset_idx, is_odd, is_closed }` + `ExtrusionJunction { p, perimeter_index }` IR types; bump `CURRENT_SLICE_IR_SCHEMA_VERSION` minor by 1 | Phase 12 | Step 8 | pending |
| T-225 | Port `stitch_extrusions` (join open polylines within `bead_width_x - 1 nm`) into `arachne/stitch.rs` | Phase 12 | Step 5 | pending |
| T-226 | Port `simplifyToolPaths` (DP simplification per `ExtrusionLine`) into `arachne/simplify.rs` | Phase 12 | Step 6 | pending |
| T-227 | Port `removeSmallLines` (drop odd non-closed lines shorter than `min_length_factor * min_width`) into `arachne/remove_small.rs` | Phase 12 | Step 7 | pending |

## Phase 13 — Wire-up + verification

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-230 | Wire all of `slicer-core::arachne` + `slicer-core::beading` + `slicer-core::skeletal_trapezoidation` into `arachne-perimeters::run_perimeters` (IMPLEMENTS real pipeline in the P110-created empty skeleton; old fake was DELETED by P108) | Phase 13 | Step 9 | pending |
| T-231 | Extend parity harness (P109/T-100) with 4 Arachne fixtures: tapered wedge, narrow strip with widening, max-bead-count cap, complex multi-feature polygon; also extends cube_4color for Arachne per-color fragmentation | Phase 13 | Step 10 | pending |
| T-232 | Walk every M2 deviation entry; close or justify D-7 (ADR-0023/P110), D-9 (T-215b/P111), D-15 (orca-mmu investigation/P105) in the roadmap | Phase 13 | Step 11 | pending |
| T-233 | Update `docs/01_system_architecture.md` Tier-2 box: remove "iterative-inset approximation" caveat; add "real Arachne (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack)" citing P112 | Phase 13 | Step 11 | pending |
| T-234 | Final `cargo test --workspace` (M2 closure ceremony per CLAUDE.md §"Test Discipline" workspace-test exception) | Phase 13 | Step 12 | pending |

## Cross-Packet Contracts

- **FORWARD-DEP P110**: `crates/slicer-core/src/skeletal_trapezoidation/` and `crates/slicer-core/src/arachne/preprocess.rs` do NOT exist until P110 ships.
- **FORWARD-DEP P111**: `crates/slicer-core/src/beading/` does NOT exist until P111 ships.
- **FORWARD-DEP P109**: `crates/slicer-runtime/tests/integration/perimeter_parity.rs` does NOT exist until P109 ships; T-231 extends it.
- **T-232 deviation walk**: D-7, D-9, D-15 live in `docs/specs/perimeter-modules-orca-parity-roadmap.md` — NOT in `docs/DEVIATION_LOG.md`. AC-11's closure greps must target the roadmap file for these three IDs. New M2 deviations added to `docs/DEVIATION_LOG.md` use `D-112-<SLUG>` format.
- **T-224 schema bump**: computed at activation from live `CURRENT_SLICE_IR_SCHEMA_VERSION` (4.3.0 at authoring); implementer increments minor by 1 from whatever the activation-time value is — do NOT hardcode.

## Deferred / Deviation Registrations

| Deviation ID | Reason | Registered in Step |
| --- | --- | --- |
| `D-112-<SLUG>` (if needed) | Any net-new M2 deviations discovered during implementation | Step 11 (T-232) |
