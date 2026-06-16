---
status: draft
packet: 110_arachne-voronoi-skt-foundations
task_ids:
  - T-200
  - T-201
  - T-202
  - T-203
  - T-204
  - T-205
  - T-P96-E
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 110_arachne-voronoi-skt-foundations

## Goal

Land the M2 foundations layer for real Arachne: write ADR-0010 closing D-7 (boostvoronoi crate selection), add `slicer_core::voronoi` as the Orca-shaped wrapper around boostvoronoi, port `SkeletalTrapezoidationGraph` (half-edge graph storing per-edge R-values), discretize parabolic Voronoi edges to line segments, port the 9-stage Arachne input pre-processing pipeline from `WallToolPaths.cpp:590-604` (including the T-P96-E per-color boundary-level MMU dedup), and create the new `arachne-perimeters/` core-module skeleton with `incompatible-with` declarations against both `classic-perimeters` and `variable-width-perimeters`.

## Scope Boundaries

Touches `docs/adr/0010-arachne-port-strategy.md` (new ADR closing D-7), `crates/slicer-core/src/voronoi.rs` (new boostvoronoi wrapper), `crates/slicer-core/src/skeletal_trapezoidation/{graph.rs,discretize.rs}` (new sub-module), `crates/slicer-core/src/arachne/preprocess.rs` (new — 9-stage pipeline + per-color MMU dedup for T-P96-E), and a fresh `modules/core-modules/arachne-perimeters/` directory (manifest + empty-but-loadable `LayerModule` impl). No BeadingStrategy work (P111), no extrusion generation (P112), no wire-up (P112). The skeleton module ships with a placeholder `run_perimeters` that returns `Ok(())` and traces a single warning — the real implementation lands in P112's T-230.

## Prerequisites and Blockers

- Depends on:
  - **P102, P103, P104, P105, P108** — M1 implementation packets must be `status: implemented` so `slicer-core` carries the polygon primitives (T-040/T-041/T-043/T-044/T-045) that `preprocess.rs` calls into.
  - **P109 (M1 verification)** — the parity harness from T-100 is the regression bed that P112's T-231 extends; this packet doesn't depend on it for compile but the implementer should not start until M1 is closed (otherwise classic regressions during M2 work go undetected).
- Unblocks:
  - **P111 (BeadingStrategy stack)** — needs `SkeletalTrapezoidationGraph` from T-202 to anchor bead-count assignment.
  - **P112 (extrusion + wire-up)** — needs the module skeleton from T-205 and the full pipeline from T-204 to wire `run_perimeters` against.
- Activation blockers: D-7 closure depends on T-200's ADR — the ADR itself is part of this packet, so no external blocker. The implementer drafts ADR-0010 first.

## Acceptance Criteria

- **AC-1. Given** `docs/adr/0010-arachne-port-strategy.md`, **when** the ADR is inspected, **then** it (a) records `boostvoronoi` v0.x as the selected Voronoi crate with one-line rationale citing https://docs.rs/boostvoronoi/, (b) lists the degeneracy classes Arachne must handle (collinear input, T-junctions, duplicate vertices, near-collinear within `epsilon_offset ≈ 11.5 µm` per `WallToolPaths.cpp` hazard), (c) defines the strategy for each (pre-snap, Boost-VD's built-in handling, or explicit rejection), and (d) closes D-7 with a status line `D-7: CLOSED — boostvoronoi v0.x`. | `rg -q 'D-7.*CLOSED.*boostvoronoi' docs/adr/0010-arachne-port-strategy.md && rg -q 'epsilon_offset' docs/adr/0010-arachne-port-strategy.md`
- **AC-2. Given** the new `slicer_core::voronoi` module, **when** `voronoi_from_segments(segments: &[Segment]) -> Result<HalfEdgeGraph, VoronoiError>` is called with a square's four segments, **then** the returned graph has the expected vertex count (5: 4 corners + 1 centroid) and the expected edge count derived from boostvoronoi's output. The function MUST NOT panic on empty input — it returns `Err(VoronoiError::EmptyInput)`. | `cargo test -p slicer-core voronoi_square_four_segments -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the Voronoi stress fixtures (3-collinear-point input + T-junction input + duplicate-vertex input), **when** `voronoi_from_segments` runs against each, **then** each returns a valid `HalfEdgeGraph` (no panic) and the half-edge count matches the recorded boostvoronoi reference for that fixture. | `cargo test -p slicer-core voronoi_stress -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the new `SkeletalTrapezoidationGraph` in `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`, **when** the square + wedge golden fixtures are inputs to its construction, **then** the resulting graph (a) carries `r_min` and `r_max` floats per edge (the Voronoi-derived radius bounds), (b) reproduces the expected half-edge / twin / next / prev wiring from a recorded JSON reference, and (c) preserves Orca's `central` boolean field per edge (default `false`; filled by P112's T-220 centrality pass). | `cargo test -p slicer-core skt_graph_square_and_wedge -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the new `discretize_parabolic_edge(parabola_focus: Point2, line_a: Point2, line_b: Point2, max_segment_len: f64) -> Vec<Point2>` in `skeletal_trapezoidation/discretize.rs`, **when** called against a recorded parabolic VD edge from OrcaSlicer's `SkeletalTrapezoidation.cpp` reference, **then** the returned polyline lies within 0.005 mm Hausdorff distance of the OrcaSlicer-discretized polyline for the same parabola. | `cargo test -p slicer-core parabolic_discretize_matches_orca -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the new 9-stage preprocess pipeline at `crates/slicer-core/src/arachne/preprocess.rs` (`preprocess_input_outline(polys: &Polygons, params: &PreprocessParams) -> Polygons` implementing offset+simplify+offset+simplify+offset+fixSelfIntersections+removeSmallAreas+offsetExtra+simplifyExtra per `WallToolPaths.cpp:590-604`), **when** the pipeline runs against a recorded raw-outline fixture, **then** the output matches the recorded reference polygons within tolerance per stage. The doc-comment for `preprocess_input_outline` MUST contain the verbatim hazard string `destroys features < epsilon_offset ~11.5 µm`. | `cargo test -p slicer-core preprocess_nine_stage_pipeline -- --nocapture 2>&1 | tee target/test-output.log && rg -q 'destroys features < epsilon_offset ~11\.5 µm' crates/slicer-core/src/arachne/preprocess.rs`
- **AC-7. Given** the T-P96-E preprocessing extension `preprocess_per_color_inputs(painted_cells: &[(ToolIndex, Polygons)], tie_break: TieBreakRule) -> Vec<(ToolIndex, Polygons)>` in `arachne/preprocess.rs`, **when** the 4-color cube fixture's per-color cells are inputs, **then** each output cell has bisector edges with neighboring different-color cells contracted/removed per the tie-break rule from ADR-0013 / T-P96-A0; the union of all output cells covers the original union exactly within ε; per-color outputs are deterministic. | `cargo test -p slicer-core preprocess_per_color_mmu_dedup -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** the new `modules/core-modules/arachne-perimeters/` directory, **when** the host's module loader is told to load it, **then** (a) the manifest declares `name = "com.core.arachne-perimeters"`, (b) the manifest's `incompatible-with` lists BOTH `com.core.classic-perimeters` AND `com.core.variable-width-perimeters`, (c) `LayerModule::run_perimeters` is a valid implementation that compiles, runs without panic, and returns `Ok(())` for any input (placeholder — real wiring lands in P112), (d) the WASM guest builds clean via `cargo xtask build-guests`. | `cargo xtask build-guests --check 2>&1 | tee target/test-output.log && rg -q 'incompatible-with.*classic-perimeters' modules/core-modules/arachne-perimeters/arachne-perimeters.toml && rg -q 'incompatible-with.*variable-width-perimeters' modules/core-modules/arachne-perimeters/arachne-perimeters.toml`

## Negative Test Cases

- **AC-N1. Given** an empty segment list passed to `voronoi_from_segments(&[])`, **when** the call returns, **then** the result is `Err(VoronoiError::EmptyInput)` and the underlying boostvoronoi call is never made (no panic, no allocation past the error path). | `cargo test -p slicer-core voronoi_empty_input_returns_err -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a DAG that attempts to load BOTH `arachne-perimeters` AND `classic-perimeters`, **when** the host scheduler validates the DAG, **then** validation fails with an error citing the `incompatible-with` clause (no silent both-load). | `cargo test -p slicer-runtime --test contract dag_rejects_arachne_and_classic_coexistence -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** the 9-stage preprocess pipeline run against an input with features smaller than `epsilon_offset` (~11.5 µm), **when** the output is inspected, **then** those features are absent from the output (the documented hazard is observed) AND the function emits a tracing `warn!` for each dropped feature listing its centroid. | `cargo test -p slicer-core preprocess_drops_tiny_features_with_warn -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core 2>&1 | tee target/test-output.log` (T-201..T-204 + T-P96-E unit suites)
- `cargo xtask build-guests --check` (T-205 skeleton coherence)

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 10 (T-200..T-205) + Inherited from P96 (T-P96-E row). Range-read those rows.
- `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — guides T-P96-E tie-break rule semantics.
- `docs/03_wit_and_manifest.md` — module manifest TOML schema + `incompatible-with` semantics for T-205.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro + `LayerModule` trait surface for T-205's placeholder.
- `docs/01_system_architecture.md` — where new slicer-core sub-modules are documented.
- `CLAUDE.md` — §"Guest WASM Staleness" (T-205 triggers a rebuild; the implementer MUST run `--check`).

## Doc Impact Statement (Required)

- `docs/adr/0010-arachne-port-strategy.md` — new ADR file — `rg -q '^# ADR-0010' docs/adr/0010-arachne-port-strategy.md`
- `docs/14_deviation_audit_history.md` — record D-7 closure pointing at ADR-0010 — `rg -q 'D-7.*CLOSED.*ADR-0010' docs/14_deviation_audit_history.md`
- `docs/01_system_architecture.md` — add sub-section entries for `voronoi`, `skeletal_trapezoidation`, `arachne::preprocess` under `slicer-core` — `rg -q 'voronoi' docs/01_system_architecture.md && rg -q 'skeletal_trapezoidation' docs/01_system_architecture.md && rg -q 'arachne::preprocess' docs/01_system_architecture.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — flip T-200/T-201/T-202/T-203/T-204/T-205/T-P96-E rows to DONE in the milestone tracker — `rg -q 'T-200.*DONE' docs/specs/perimeter-modules-orca-parity-roadmap.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` — half-edge graph data layout (T-202), parabolic-edge discretization math (T-203). One LOCATIONS dispatch (≤ 20 entries) for graph struct + discretize_parabolic_edge function signatures.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` lines 590–604 — the 9-stage preprocess sequence (T-204). One SUMMARY (≤ 150 words) listing each stage's offset distance + simplify epsilon.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` — per-color bisector contraction reference for T-P96-E. Cite the investigation one-pager from T-P96-A0 (`docs/specs/orca-mmu-perimeter-investigation.md`) — that already names the lines.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
