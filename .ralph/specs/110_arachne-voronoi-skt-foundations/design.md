# Design: 110_arachne-voronoi-skt-foundations

## Controlling Code Paths

- **Voronoi wrapper:** `crates/slicer-core/src/voronoi.rs` (NEW). The wrapper hides `boostvoronoi`'s C++-flavored API (`VoronoiBuilder`, `VoronoiDiagram`) behind an Orca-shaped surface: `voronoi_from_segments(&[Segment]) -> Result<HalfEdgeGraph, VoronoiError>`. `Segment` is a thin newtype around `(Point2, Point2)` keyed in slicer-coordinate units (1 unit = 100 nm). `Point2` is `slicer_ir::Point2` (i64 struct at `crates/slicer-ir/src/slice_ir.rs:81`) — NOT `slicer_core::geometry::Point2` (which does not define its own Point2; `geometry.rs` imports from `slicer_ir`). `boostvoronoi` is already an optional dep in `crates/slicer-core/Cargo.toml` at v0.12 under `host-algos` feature. `HalfEdgeGraph` mirrors boostvoronoi's output but stores indices into pre-allocated `Vec`s rather than raw pointers — this is the structure the SKT graph builds on top of.
- **Skeletal trapezoidation graph:** `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (NEW) re-exports `graph::SkeletalTrapezoidationGraph` + `discretize::discretize_parabolic_edge`. `graph.rs` carries the half-edge struct with `r_min: f64`, `r_max: f64`, `central: bool` fields per edge; construction takes a `HalfEdgeGraph` + the input polygons and assigns `r_min`/`r_max` by computing min/max distance from each edge's pair of endpoints to the nearest input polygon edge.
- **Discretization:** `discretize.rs` (NEW) implements `discretize_parabolic_edge(focus, line_a, line_b, max_segment_len)` — the parabola lives between a point `focus` and the line through `line_a..line_b`; the algorithm subdivides until each chord is ≤ `max_segment_len`. Reference: OrcaSlicer `SkeletalTrapezoidation::discretize_parabolic_edge`.
- **9-stage preprocess + per-color MMU dedup:** `crates/slicer-core/src/arachne/preprocess.rs` (NEW). `preprocess_input_outline(polys, params)` implements the verbatim sequence from `WallToolPaths.cpp:590-604`. `preprocess_per_color_inputs(painted_cells, tie_break)` implements T-P96-E: for each `(ToolIndex, Polygons)` pair, walk each polygon's edges; for edges shared with a neighboring cell of a different `ToolIndex`, apply the tie-break rule (lower `ToolIndex.0` wins by default per ADR-0013) and contract/remove the edge on the losing side.
- **`arachne-perimeters` skeleton (NEW):** `modules/core-modules/arachne-perimeters/` (NEW directory, created in the path P108 already emptied by deleting the old fake). Manifest: `id = "com.core.arachne-perimeters"`, `holds = ["perimeter-generator"]`, `incompatible-with = ["com.core.classic-perimeters"]` (only — no other entries). `src/lib.rs`: empty `LayerModule` impl, returns `Ok(())`, emits `warn!("arachne-perimeters skeleton loaded — no walls produced; real impl ships in P112")`. PRECONDITION for T-205: `! test -d modules/core-modules/arachne-perimeters` must pass. Add as workspace member in root `Cargo.toml`.

## Neighboring Tests & Fixtures

- `crates/slicer-core/tests/` already carries unit suites for `polygon_ops`, `medial_axis`, `geometry`, `flow` from M1 packets P103/P105. The new test files (`voronoi_stress.rs`, `skt_graph_golden.rs`, `parabolic_discretize.rs`, `preprocess_golden.rs`) match that pattern and place their fixtures under `tests/fixtures/voronoi/`, `tests/fixtures/skt/`, `tests/fixtures/arachne_preprocess/`.
- `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs` already exists (NOT `tests/contract/dag_validation.rs` — that file is absent); AC-N2 adds one test function there. The file is registered as `mod dag_validation_tdd;` in `tests/unit/main.rs:15`. Use `--test unit` for AC-N2's cargo test command. The dispatch fixture from P100 (`tests/common/dispatch_fixture.rs`) is reused — no new fixture infrastructure.
- Golden fixtures use the same JSON-serialized IR pattern P109's parity harness establishes — small files, deterministic, committed.

## Architecture Constraints

<!-- snippet: coord-system -->
- **Coordinate system hazard.** All Voronoi/SKT geometry passes through `slicer_core::voronoi::Segment` keyed in slicer units (1 unit = 100 nm). OrcaSlicer's `epsilon_offset` is `SCALED_EPSILON * 0.5` ≈ 11.5 µm in real space; in slicer units that's `115` (not `11500` — divide OrcaSlicer constants by 100 per `docs/08_coordinate_system.md`). The implementer MUST translate every OrcaSlicer scale constant through `mm_to_units` or the explicit `/100` rule before pasting into Rust. Any constant > 100000 in geometry code is a red flag.

<!-- snippet: wasm-staleness -->
- **Guest WASM staleness.** P110 triggers a guest rebuild on TWO fronts: (1) Step 6 CREATES a new core module `modules/core-modules/arachne-perimeters/` (a fresh guest with its own manifest + `src/lib.rs`, in the path P108 emptied); and (2) Steps 2/5 add new modules to `crates/slicer-core/**`, a **universal guest dependency baked into every guest WASM** (per CLAUDE.md §"Guest WASM Staleness"). So after the Step 2/5 `slicer-core` edits AND after the Step 6 module creation the implementer MUST run `cargo xtask build-guests --check` and rebuild if `STALE:`; otherwise AC-N2's DAG-validation test can fail with a typed-instantiation error masquerading as a packet bug. **OPEN RISK (implementer must verify in Step 2):** the new Voronoi code wraps `boostvoronoi`, a C++-FFI crate that does NOT target `wasm32`. It MUST stay behind the host-only `host-algos` feature so it is never compiled into a guest; if `voronoi.rs`/`skeletal_trapezoidation`/`arachne::preprocess` leak into the default (guest) feature set, every guest build breaks. Gate them host-only and confirm `cargo xtask build-guests` still compiles.

- **boostvoronoi licensing & versioning.** boostvoronoi is BSL-1.0; the workspace is MIT/Apache-2.0. The ADR (T-200) MUST record the license decision; the dependency is added to `slicer-core/Cargo.toml` only. No other crate depends on it.
- **Determinism.** `voronoi_from_segments` MUST be deterministic for the same input segment order. boostvoronoi's underlying Boost.Polygon algorithm is deterministic; the wrapper does not introduce any HashMap/HashSet over float keys (use sorted Vec or BTreeMap if needed).
- **No panics in geometry code.** Every wrapper function returns `Result<_, VoronoiError>` or `Result<_, PreprocessError>`. Internal `unwrap()` is forbidden in `voronoi.rs`, `graph.rs`, `discretize.rs`, `preprocess.rs` — invariants get explicit error variants.

## Selected Approach

**boostvoronoi wrapping + Orca-shaped re-export.** `voronoi.rs` is a thin Rust wrapper around `boostvoronoi::VoronoiBuilder`. Input segments are inserted via `boostvoronoi::Builder::with_segments`; output diagram is converted to a `HalfEdgeGraph` struct that mirrors what OrcaSlicer's `SkeletalTrapezoidation` ingests. This trades a deeper Rust abstraction for parity with the OrcaSlicer code path, which makes T-202's graph port a direct field-by-field translation.

Rejected alternatives:
- **voronator** — single-author Rust Voronoi crate (https://crates.io/crates/voronator). Smaller surface, missing T-junction handling that boostvoronoi inherits from Boost.Polygon. Rejected per D-7 default.
- **Direct Boost.Polygon FFI** — would require building a C++ dependency. Rejected per "pure-Rust constraint" in ADR-0023.
- **Implementing VD from scratch** — Fortune's algorithm in 1–2 KLOC. Tempting but high risk on degeneracy correctness; T-203's parabolic-discretization math is non-trivial even with a working VD.

For T-P96-E: per-color preprocessing AT THE BOUNDARY (NOT per-edge mask, NOT post-VD edge skipping). The boundary contraction happens before `voronoi_from_segments` is called per color. This matches OrcaSlicer's MMU Arachne path per the T-P96-A0 investigation: Orca preprocesses per-color cells, then runs Arachne normally on each. Rejected: per-edge skip-mask reuse from Classic (T-P96-C0..C2). Reason: SkeletalTrapezoidation traces walls through the half-edge graph at runtime — there's no "edge to skip" at output time because the wall isn't a polygon edge anymore.

## Code Change Surface

Primary files (≤ 3 per step; aggregate listed for the packet):

| File | Status | Step | Notes |
| --- | --- | --- | --- |
| `docs/adr/0023-arachne-port-strategy.md` | NEW | Step 1 | ADR documenting boostvoronoi selection; cross-references D-7 CLOSED in roadmap (slot 0023: unused free gap; tree max ADR is 0031) |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | EDIT | Step 1 | Add ADR-0023 reference to D-7 row (D-7 is already CLOSED there, NOT in deviation_audit_history.md) |
| `crates/slicer-core/Cargo.toml` | EDIT | Step 2 | Extend `boostvoronoi` v0.12 from optional (`host-algos` feature) to always-on or broader feature gate; NOT adding new dep |
| `crates/slicer-core/src/lib.rs` | EDIT | Steps 2/3/5 | `pub mod` registrations |
| `crates/slicer-core/src/voronoi.rs` | NEW | Step 2 | `voronoi_from_segments` + types |
| `crates/slicer-core/tests/voronoi_stress.rs` | NEW | Step 2 | AC-2/AC-3/AC-N1 |
| `crates/slicer-core/tests/fixtures/voronoi/` | NEW | Step 2 | Golden JSON files |
| `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` | NEW | Step 3 | Re-exports |
| `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` | NEW | Step 3 | `SkeletalTrapezoidationGraph` |
| `crates/slicer-core/tests/skt_graph_golden.rs` | NEW | Step 3 | AC-4 |
| `crates/slicer-core/tests/fixtures/skt/` | NEW | Step 3 | Golden JSON |
| `crates/slicer-core/src/skeletal_trapezoidation/discretize.rs` | NEW | Step 4 | Parabolic edge math |
| `crates/slicer-core/tests/parabolic_discretize.rs` | NEW | Step 4 | AC-5 |
| `crates/slicer-core/src/arachne/mod.rs` | NEW | Step 5 | Re-exports |
| `crates/slicer-core/src/arachne/preprocess.rs` | NEW | Step 5 | 9-stage + T-P96-E |
| `crates/slicer-core/tests/preprocess_golden.rs` | NEW | Step 5 | AC-6 + AC-7 + AC-N3 |
| `crates/slicer-core/tests/fixtures/arachne_preprocess/` | NEW | Step 5 | Raw-outline + 4-color cells |
| `modules/core-modules/arachne-perimeters/` (new dir + manifest + src/lib.rs + Cargo.toml) | NEW (P108 emptied the path) | Step 6 | Fresh skeleton: `incompatible-with = ["com.core.classic-perimeters"]` only; empty `LayerModule` impl + `warn!`; add workspace member in root `Cargo.toml` |
| `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs` | EDIT | Step 6 | AC-N2 test `dag_rejects_arachne_and_classic_coexistence`; file already exists at `tests/unit/`; already registered in `tests/unit/main.rs:15`; use `--test unit` |
| `docs/01_system_architecture.md` | EDIT | Step 7 | slicer-core sub-module entries |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | EDIT | Step 7 | Flip rows to DONE |

## Read-Only Context

| File | Range | Purpose |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | Phase 10 rows + Inherited-from-P96 T-P96-E row | Task definitions |
| `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` | full | T-P96-E tie-break semantics |
| `docs/03_wit_and_manifest.md` | §"Module Manifest TOML" + §"incompatible-with" | T-205 manifest format |
| `docs/05_module_sdk.md` | §"#[slicer_module]" + `LayerModule` trait | T-205 placeholder shape |
| `docs/08_coordinate_system.md` | full | Unit conversion for OrcaSlicer constants |
| `docs/01_system_architecture.md` | §slicer-core section | Existing sub-module pattern |
| `crates/slicer-core/src/lib.rs` | full (current state) | Existing `pub mod` declarations to extend |
| `modules/core-modules/classic-perimeters/Cargo.toml` | full | Template for new module's Cargo.toml |
| `modules/core-modules/classic-perimeters/classic-perimeters.toml` | full | Template for the NEW `arachne-perimeters.toml` skeleton |
| `modules/core-modules/classic-perimeters/src/lib.rs` | lines 1–50 | `#[slicer_module]` invocation pattern |
| `docs/specs/orca-mmu-perimeter-investigation.md` | full (≤ 200 lines) — PRESENT in tree (authored by T-P96-A0/P105, `implemented`). Use a FACT dispatch for the tie-break rule + Arachne MMU citation; fall back to a `MultiMaterialSegmentation.cpp` LOCATIONS dispatch only if a citation is missing. | T-P96-E Arachne MMU path citations |

## Out-of-Bounds Files

The implementer MUST NOT directly read:
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` (~3000 LOC) — use LOCATIONS dispatch only.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` (~2500 LOC) — use SUMMARY dispatch only.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` — citations already in `docs/specs/orca-mmu-perimeter-investigation.md`.
- Any vendored boostvoronoi source — use https://docs.rs/boostvoronoi/ via WebFetch SUMMARY only.
- `crates/slicer-ir/src/slice_ir.rs` (~1700 LOC) — this packet does NOT touch IR; do not open.
- Other M1 packet directories (`.ralph/specs/102_*/...108_*/`) — they're closed.
- `target/`, lockfiles, generated bindgen output.

## Expected Sub-Agent Dispatches

| Step | Dispatch | Scope | Return format |
| --- | --- | --- | --- |
| Step 2 | WebFetch boostvoronoi docs | https://docs.rs/boostvoronoi/ | SUMMARY ≤ 200 words: `VoronoiBuilder` + `VoronoiDiagram` API surface |
| Step 3 | OrcaSlicer LOCATIONS for SKT graph layout | `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` | LOCATIONS ≤ 20 entries: half-edge struct + `r_min`/`r_max`/`central` field defs |
| Step 4 | OrcaSlicer LOCATIONS for parabolic discretization | `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` | LOCATIONS ≤ 10 entries: `discretize_parabolic_edge` signature + tessellation constants |
| Step 5 | OrcaSlicer SUMMARY for 9-stage preprocess | `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:590-604` | SUMMARY ≤ 150 words: each stage's offset + simplify epsilon + epsilon_offset hazard |
| Step 5 | Investigation one-pager for T-P96-E | `docs/specs/orca-mmu-perimeter-investigation.md` | FACT: tie-break rule (1 sentence) + Arachne MMU citation (file:line) |
| Step 6 | Confirm deletion before create | `! test -d modules/core-modules/arachne-perimeters` | FACT: directory is ABSENT (P108 completed); use `modules/core-modules/classic-perimeters/` as template for new skeleton |
| All steps | `cargo test -p slicer-core <pattern>` | n/a | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |

## Data & Contract Notes

- **`Segment`**: `{ a: Point2, b: Point2 }` with `Point2 { x: i64, y: i64 }` in slicer units. Uses `slicer_ir::Point2` (the existing i64 struct at `crates/slicer-ir/src/slice_ir.rs:81`). NOTE: `slicer_core::geometry::Point2` does NOT exist — `geometry.rs` imports `slicer_ir::Point2` via `use slicer_ir::{ExPolygon, Point2}` (verified line 16). Do NOT define a new Point2 in `voronoi.rs`.
- **`HalfEdgeGraph`**: `{ vertices: Vec<Vertex>, edges: Vec<HalfEdge> }` where `HalfEdge { start_vertex: usize, twin: usize, next: usize, prev: usize, cell: usize, is_primary: bool, is_curved: bool }`. The `is_curved` flag drives discretization (Step 4).
- **`SkeletalTrapezoidationGraph`**: extends `HalfEdgeGraph` with per-edge `r_min: f64`, `r_max: f64`, `central: bool` (default false). R-values are slicer-unit distances.
- **`PreprocessParams`**: bundles `epsilon_offset: f64`, `simplify_epsilon: f64`, `min_feature_size: f64`. Defaults match OrcaSlicer (translated through the /100 rule).
- **`TieBreakRule`**: enum `{ LowerToolIndexWins, HigherToolIndexWins, Custom(fn(ToolIndex, ToolIndex) -> ToolIndex) }`. Default `LowerToolIndexWins` per ADR-0013.
- **`VoronoiError`**: `{ EmptyInput, DegenerateInput(String), InternalBoostError(String) }`. Not panicking — every error path returns `Err`.

## Locked Assumptions and Invariants

- Voronoi/SKT/arachne::preprocess placed in `slicer-core` per docs/13 §Out of Scope (per-layer geometry operations belong in slicer-core). boostvoronoi added to `slicer-core/Cargo.toml`; slicer-helpers no-FFI question resolved by the rename. Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT`.
- boostvoronoi's output preserves input segment order via cell indexing — the wrapper relies on this for the SKT graph construction.
- The 9-stage preprocess pipeline destroys features smaller than `epsilon_offset` (~11.5 µm in real, 115 units in slicer space). This is documented Orca behavior; the doc-comment hazard string is REQUIRED so downstream M2 callers don't assume preservation.
- The `arachne-perimeters` skeleton's `incompatible-with` declarations are enforced by the host scheduler's DAG validation (existing host code from P104 / D-X — confirm during Step 6). If the validation isn't wired, AC-N2 fails and the implementer files a bug — does NOT silently drop the assertion.
- The placeholder `run_perimeters` returns `Ok(())` — this makes any test that loads the module BUT expects walls to fail silently. Mitigation: the `warn!` is mandatory so `Grep` on test logs catches the placeholder path during P112.

## Risks and Tradeoffs

- **boostvoronoi maintenance.** Single-author crate, ~6 contributors total. Risk: stale crate could block M2 if bugs surface. Mitigation: ADR-0023 records pinned version + a fallback escape hatch (if boostvoronoi proves blocking, swap to voronator with a 2-week budget — recorded as a residual deviation).
- **Goldens recorded per crate vs from OrcaSlicer.** This packet records goldens from boostvoronoi's own output (not OrcaSlicer reference). Reason: testing the wrapper, not parity. OrcaSlicer-parity verification lands in P112's T-231. Risk: goldens drift if boostvoronoi changes; mitigation: pin the version.
- **T-205 placeholder masking.** A module that returns `Ok(())` could be loaded in real slice runs and silently produce no walls. The `warn!` is the canary; tests in P112 that activate `arachne-perimeters` MUST assert walls present (not just `Ok`).
- **Schema invariance.** This packet does NOT bump any IR schema version. T-202's `SkeletalTrapezoidationGraph` is INTERNAL to slicer-core — not in IR. T-224 (in P112) is what adds `ExtrusionLine`/`ExtrusionJunction` to IR.

## Context Cost Estimate

- Aggregate: M.
- Largest single step: Step 5 (9-stage preprocess + T-P96-E per-color + 2 golden fixtures). Sub-step budget: M. If implementation reaches 60% context, the implementer hands off (T-P96-E can be split into its own follow-on if Step 5 alone hits the threshold).
- Highest-risk dispatch: Step 3's SkeletalTrapezoidation.cpp LOCATIONS — the file is ~3000 LOC; an unscoped read would burn 40k+ tokens. The dispatch contract caps at 20 entries. If a single dispatch returns > 20, re-dispatch tighter and proceed.

## Open Questions

- **[FWD]** What's the exact pinned `boostvoronoi` version (0.10 vs 0.11)? Resolve in Step 1 by reading https://docs.rs/boostvoronoi/ for the latest 0.x — record in ADR-0023.
- **Resolved** — There is no separate `slicer_core::geometry::Point2`; `geometry.rs` re-exports `slicer_ir::Point2` (the i64 struct at `slice_ir.rs:81`). `voronoi.rs` uses `slicer_ir::Point2` directly (already i64, which is what boostvoronoi requires) — no new Point2 type is defined.
- **[FWD]** Is there an existing `dag_validation.rs` test file under `crates/slicer-runtime/tests/contract/`? Resolve in Step 6 via `Glob` before writing. If absent, create.
- **None [BLOCK].** Every blocking question is resolved by ADR-0023 (D-7) or by `docs/specs/orca-mmu-perimeter-investigation.md` (D-13, D-15 — closed by T-P96-A0 in P105). The packet is fully unblocked.
