# Task Map: 110_arachne-voronoi-skt-foundations

Maps packet task IDs (T-200..T-205, T-P96-E) to their source rows in the roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 10 (T-200..T-205) and "Inherited from P96" (T-P96-E).

## Phase 10 — Foundations (Voronoi + SkeletalTrapezoidation)

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-200 | ADR `0023-arachne-port-strategy.md`: document Voronoi crate selection (D-7 CLOSED), pure-Rust constraints, degeneracy handling expectations | Phase 10 | Step 1 | pending |
| T-201 | Vendor / depend on chosen Voronoi crate (boostvoronoi v0.12); wrap in `slicer-core::voronoi` with Orca-shaped API surface (`voronoi_from_segments`) | Phase 10 | Step 2 | pending |
| T-202 | Port `SkeletalTrapezoidationGraph` (half-edge graph storing `r_min`, `r_max`, `central` per edge) | Phase 10 | Step 3 | pending |
| T-203 | Discretize parabolic VD edges to line segments (`discretize_parabolic_edge`) | Phase 10 | Step 4 | pending |
| T-204 | Port the 9-stage pre-processing pipeline from `WallToolPaths.cpp:590-604` into `crates/slicer-core/src/arachne/preprocess.rs` | Phase 10 | Step 5 | pending |
| T-205 | CREATE NEW `modules/core-modules/arachne-perimeters/` skeleton (P108/T-090 already deleted the old fake; path confirmed absent): manifest with `id = "com.core.arachne-perimeters"`, `incompatible-with = ["com.core.classic-perimeters"]` only; empty `LayerModule` impl + `warn!`; add workspace member | Phase 10 | Step 6 | pending |

## Inherited from P96 — Arachne MMU per-color boundary-level dedup

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-P96-E | Per-color boundary-level MMU dedup at Arachne preprocessing: `preprocess_per_color_inputs(painted_cells, tie_break)` in `arachne/preprocess.rs`; bisector edges contracted/removed per tie-break rule from ADR-0013 / T-P96-A0 investigation (P105) | Phase 10/12 (P96-inherited) | Step 5 | pending |

## Cross-Packet Contracts

- **P103 prerequisite**: `offset2_ex`, `opening_ex`, `polygon_tree` — used by `preprocess_input_outline` in Step 5.
- **P105 (`implemented`)**: `docs/specs/orca-mmu-perimeter-investigation.md` (authored by T-P96-A0 in P105) is PRESENT in tree. Step 5 reads it via a FACT dispatch; substitute a `MultiMaterialSegmentation.cpp` LOCATIONS dispatch only if a citation is missing.
- **T-P96-E tie-break rule** is grounded in ADR-0013 (`docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`) + the T-P96-A0 one-pager (P105 deliverable).

## Docs Impact

- `docs/adr/0023-arachne-port-strategy.md` — NET-NEW (Step 1); closes D-7 formally.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — D-7 row updated to reference ADR-0023; T-200..T-205 + T-P96-E flipped to DONE (Step 7).
- `docs/01_system_architecture.md` — sub-section entries for `voronoi`, `skeletal_trapezoidation`, `arachne::preprocess` (Step 7).
