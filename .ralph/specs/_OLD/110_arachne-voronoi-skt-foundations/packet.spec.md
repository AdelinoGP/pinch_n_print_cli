---
status: implemented
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

Land the M2 foundations layer for real Arachne: write ADR-0023 (D-7 is already CLOSED in the roadmap — the ADR documents the crate selection decision that closes it), add `slicer_core::voronoi` as the Orca-shaped wrapper around boostvoronoi, port `SkeletalTrapezoidationGraph` (half-edge graph storing per-edge R-values), discretize parabolic Voronoi edges to line segments, port the 9-stage Arachne input pre-processing pipeline from `WallToolPaths.cpp:590-604` (including the T-P96-E per-color boundary-level MMU dedup), and CREATE the new `arachne-perimeters/` core-module skeleton (new directory, manifest, empty `LayerModule` impl) with `incompatible-with = ["com.core.classic-perimeters"]`.

**Predecessor P108 (`status: implemented`) already deleted the old fake `arachne-perimeters/`** (the 512-line iterative-inset approximation, removed under D-110-DROP-VARIABLE-WIDTH; `modules/core-modules/arachne-perimeters/` is confirmed absent from the tree). P110/T-205 creates the real-Arachne skeleton FRESH in that now-empty path — this is NOT an edit of an existing module. Real wire-up of `run_perimeters` against the new slicer-core modules is P112's job (T-230).

## Scope Boundaries

Touches `docs/adr/0023-arachne-port-strategy.md` (new ADR — slot 0023 is unused and reserved for this packet; the tree's highest committed ADR is 0031, and 0022/0024–0031 are already taken, so 0023 is the free gap this packet fills), `crates/slicer-core/src/voronoi.rs` (new boostvoronoi wrapper), `crates/slicer-core/src/skeletal_trapezoidation/{graph.rs,discretize.rs}` (new sub-module), `crates/slicer-core/src/arachne/preprocess.rs` (new — 9-stage pipeline + per-color MMU dedup for T-P96-E), and the NEW `modules/core-modules/arachne-perimeters/` skeleton (NEW directory, manifest, empty `LayerModule` impl — P108 already deleted the old fake and the path is confirmed absent; see T-205). No BeadingStrategy work (P111), no extrusion generation (P112), no wire-up (P112).

## Prerequisites and Blockers

- Depends on (all `status: implemented` at refinement — the M1 packets P102–P109 have shipped):
  - **P103** (polygon primitives T-040/T-043/T-044/T-045) — `implemented` ✓ (`crates/slicer-core/src/polygon_ops.rs`, `polygon_tree.rs`, `medial_axis.rs` present). `preprocess.rs` calls into these.
  - **P102** — `implemented` ✓.
  - **P104** — `implemented` ✓ — the surface-rule primitives `preprocess.rs` calls into are in tree.
  - **P105** — `implemented` ✓ — `docs/specs/orca-mmu-perimeter-investigation.md` (cited by T-P96-E, authored by P105/T-P96-A0) is committed and present in the tree.
  - **P108** — `implemented` ✓ — deleted the old fake `arachne-perimeters/` (D-110-DROP-VARIABLE-WIDTH); `modules/core-modules/arachne-perimeters/` is confirmed absent, so T-205's fresh-create precondition holds.
  - **P109 (M1 verification)** — `implemented` ✓ — the parity harness from T-100 (`crates/slicer-runtime/tests/integration/perimeter_parity.rs`) is present; it is the regression bed P112's T-231 extends.
- Unblocks:
  - **P111 (BeadingStrategy stack)** — needs `SkeletalTrapezoidationGraph` from T-202 to anchor bead-count assignment.
  - **P112 (extrusion + wire-up)** — needs the module skeleton from T-205 and the full pipeline from T-204 to wire `run_perimeters` against.
- Activation blockers: D-7 closure depends on T-200's ADR — the ADR itself is part of this packet, so no external blocker. The implementer drafts ADR-0023 first.

## Acceptance Criteria

- **AC-1. Given** `docs/adr/0023-arachne-port-strategy.md`, **when** the ADR is inspected, **then** it (a) records `boostvoronoi` v0.x as the selected Voronoi crate with one-line rationale citing https://docs.rs/boostvoronoi/, (b) lists the degeneracy classes Arachne must handle (collinear input, T-junctions, duplicate vertices, near-collinear within `epsilon_offset ≈ 11.5 µm` per `WallToolPaths.cpp` hazard), (c) defines the strategy for each (pre-snap, Boost-VD's built-in handling, or explicit rejection), and (d) cross-references the existing D-7 CLOSED entry in `docs/specs/perimeter-modules-orca-parity-roadmap.md` (D-7 is already marked CLOSED there; this ADR is its rationale document). NOTE: do NOT add a second D-7 closure entry to `docs/14_deviation_audit_history.md` — D-7 lives in the roadmap, not the audit log (verified). | `rg -q 'boostvoronoi' docs/adr/0023-arachne-port-strategy.md && rg -q 'epsilon_offset' docs/adr/0023-arachne-port-strategy.md`
- **AC-2. Given** the new `slicer_core::voronoi` module, **when** `voronoi_from_segments(segments: &[Segment]) -> Result<HalfEdgeGraph, VoronoiError>` is called with a square's four segments, **then** the returned graph has the expected vertex count (5: 4 corners + 1 centroid) and the expected edge count derived from boostvoronoi's output. The function MUST NOT panic on empty input — it returns `Err(VoronoiError::EmptyInput)`. | `cargo test -p slicer-core --features host-algos voronoi_square_four_segments -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the Voronoi stress fixtures (3-collinear-point input + T-junction input + duplicate-vertex input), **when** `voronoi_from_segments` runs against each, **then** each returns a valid `HalfEdgeGraph` (no panic) and the half-edge count matches the recorded boostvoronoi reference for that fixture. | `cargo test -p slicer-core --features host-algos voronoi_stress -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the new `SkeletalTrapezoidationGraph` in `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`, **when** the square + wedge golden fixtures are inputs to its construction, **then** the resulting graph (a) carries `r_min` and `r_max` floats per edge (the Voronoi-derived radius bounds), (b) reproduces the expected half-edge / twin / next / prev wiring from a recorded JSON reference, and (c) preserves Orca's `central` boolean field per edge (default `false`; filled by P112's T-220 centrality pass). | `cargo test -p slicer-core --features host-algos skt_graph_square_and_wedge -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the new `discretize_parabolic_edge(parabola_focus: Point2, line_a: Point2, line_b: Point2, max_segment_len: f64) -> Vec<Point2>` in `skeletal_trapezoidation/discretize.rs`, **when** called against a recorded parabolic VD edge from OrcaSlicer's `SkeletalTrapezoidation.cpp` reference, **then** the returned polyline lies within 0.005 mm Hausdorff distance of the OrcaSlicer-discretized polyline for the same parabola. | `cargo test -p slicer-core --features host-algos parabolic_discretize_matches_orca -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the new 9-stage preprocess pipeline at `crates/slicer-core/src/arachne/preprocess.rs` (`preprocess_input_outline(polys: &Polygons, params: &PreprocessParams) -> Polygons` implementing offset+simplify+offset+simplify+offset+fixSelfIntersections+removeSmallAreas+offsetExtra+simplifyExtra per `WallToolPaths.cpp:590-604`), **when** the pipeline runs against a recorded raw-outline fixture, **then** the output matches the recorded reference polygons within tolerance per stage. The doc-comment for `preprocess_input_outline` MUST contain the verbatim hazard string `destroys features < epsilon_offset ~11.5 µm`. | `cargo test -p slicer-core preprocess_nine_stage_pipeline -- --nocapture 2>&1 | tee target/test-output.log && rg -q 'destroys features < epsilon_offset ~11\.5 µm' crates/slicer-core/src/arachne/preprocess.rs`
- **AC-7. Given** the T-P96-E preprocessing extension `preprocess_per_color_inputs(painted_cells: &[(ToolIndex, Vec<ExPolygon>)]) -> Vec<(ToolIndex, Vec<ExPolygon>)>` in `arachne/preprocess.rs`, **when** the 4-color cube fixture's per-color cells are inputs, **then** the function is a validated pass-through: each output cell's boundary is preserved unmodified from its input (no bisector-edge contraction), the non-overlap invariant across all output cells is validated, and a violation beyond ε is logged (not silently repaired) rather than causing a panic; per-color outputs are deterministic. Per-color boundary isolation is the responsibility of the upstream paint/region-split pipeline (P91-94), not of Arachne preprocessing — confirmed against ADR-0013's current doctrine (lines 9, 29, 32, 40: "no skip mask, no per-edge ownership, and no tie-break rule") and against canonical OrcaSlicer source (`PerimeterGenerator.cpp:2600-2653` `process_arachne()`; `Arachne/WallToolPaths.hpp:63-83`), which contains zero color/extruder/material-aware logic — Arachne is color-blind by design. NOTE: this corrects the packet's original AC-7 text, which specified a bisector-edge contraction algorithm driven by a `TieBreakRule` enum; that design was based on a since-retired tie-break model (see closure-log.md item 3 for full detail). | `cargo test -p slicer-core preprocess_per_color_mmu_dedup -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** the NEW `modules/core-modules/arachne-perimeters/` skeleton created by T-205 (P108 already deleted the old fake; the path is confirmed absent), **when** the host's module loader is told to load it, **then** (a) the manifest field `id = "com.core.arachne-perimeters"` is present, (b) the manifest's `incompatible-with` lists `com.core.classic-perimeters` ONLY — no other incompatibility entries, (c) `run_perimeters` is an empty `LayerModule` impl (returns `Ok(())` + emits a `warn!` so test infra can detect the skeleton path), (d) the WASM guest builds clean via `cargo xtask build-guests`. PRECONDITION (satisfied at refinement — verified absent): `! test -d modules/core-modules/arachne-perimeters` must be true BEFORE T-205 creates the new directory. | `cargo xtask build-guests --check 2>&1 | tee target/test-output.log && rg -q '"com\.core\.classic-perimeters"' modules/core-modules/arachne-perimeters/arachne-perimeters.toml`

## Negative Test Cases

- **AC-N1. Given** an empty segment list passed to `voronoi_from_segments(&[])`, **when** the call returns, **then** the result is `Err(VoronoiError::EmptyInput)` and the underlying boostvoronoi call is never made (no panic, no allocation past the error path). | `cargo test -p slicer-core --features host-algos voronoi_empty_input_returns_err -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a DAG that attempts to load BOTH `arachne-perimeters` AND `classic-perimeters`, **when** the host scheduler validates the DAG, **then** validation fails with an error citing the `incompatible-with` clause (no silent both-load). PRECONDITION: the NEW T-205 skeleton module (created by this packet) must be present; P108 (`implemented`) already removed the old fake. | `cargo test -p slicer-runtime --test unit dag_rejects_arachne_and_classic_coexistence -- --nocapture 2>&1 | tee target/test-output.log` (S7 FIX: DAG tests live at `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs`, aggregated in `tests/unit/main.rs` — NOT under `tests/contract/`. New test registers as `mod` in `tests/unit/main.rs`.)
- **AC-N3. Given** the 9-stage preprocess pipeline run against an input with features smaller than `epsilon_offset` (~11.5 µm), **when** the output is inspected, **then** those features are absent from the output (the documented hazard is observed) AND the function emits a tracing `warn!` for each dropped feature listing its centroid. | `cargo test -p slicer-core preprocess_drops_tiny_features_with_warn -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos 2>&1 | tee target/test-output.log` (T-201..T-204 + T-P96-E unit suites)
- `cargo xtask build-guests --check` (T-205 — manifest edit coherence; existing arachne-perimeters WASM must still be valid)

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 10 (T-200..T-205) + Inherited from P96 (T-P96-E row). Range-read those rows.
- `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — guides T-P96-E tie-break rule semantics.
- `docs/03_wit_and_manifest.md` — module manifest TOML schema + `incompatible-with` semantics for T-205.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro + `LayerModule` trait surface for T-205's placeholder.
- `docs/01_system_architecture.md` — where new slicer-core sub-modules are documented.
- `CLAUDE.md` — §"Guest WASM Staleness" (T-205 triggers a rebuild; the implementer MUST run `--check`).

## Doc Impact Statement (Required)

- `docs/adr/0023-arachne-port-strategy.md` — new ADR file — `rg -q '^# ADR-0023' docs/adr/0023-arachne-port-strategy.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — D-7 is ALREADY marked CLOSED in this file (verified: line 92). The ADR-0023 reference should be added inline to the existing D-7 row — `rg -q 'D-7.*CLOSED.*boostvoronoi' docs/specs/perimeter-modules-orca-parity-roadmap.md` (NOTE: `docs/14_deviation_audit_history.md` does NOT contain D-7 — do not add a duplicate entry there; D-7 lives in the roadmap only).
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

## Deviations

- `[AC-7 / T-P96-E]` — Specified: bisector-edge contraction via a `TieBreakRule` enum (default `LowerToolIndexWins`) per ADR-0013 | Implemented: validated pass-through — no `TieBreakRule` type, cells unmodified, non-overlap violations logged not repaired | Reason: ADR-0013's current doctrine (lines 9,29,32,40) retired the tie-break model; canonical OrcaSlicer source (`PerimeterGenerator.cpp:2600-2653`) confirms Arachne has zero color-aware logic — isolation happens upstream. Confirmed with user mid-session against canonical source.
- `[AC-5 / discretize_parabolic_edge signature]` — Specified: 4-param signature implied to mirror OrcaSlicer | Implemented: `line_a`/`line_b` serve double duty as directrix-line definition AND arc bounds (Orca's real function takes 6 params incl. explicit `start`/`end`/`transitioning_angle`) | Reason: packet's stated signature has no room for separate arc bounds; `transitioning_angle` bead-marker logic is P111/P112 territory.
- `[AC-5 / "OrcaSlicer-discretized polyline" reference]` — Specified: compare against "the OrcaSlicer-discretized polyline" | Implemented: compared against an independently-computed dense resampling of the same closed-form parabola equation | Reason: this environment cannot execute OrcaSlicer C++; same precedent as Steps 2-3's boostvoronoi-derived goldens. Numeric OrcaSlicer parity deferred to P112/T-231.
- `[AC-6 / AC-1 / epsilon_offset value]` — Specified: `epsilon_offset ≈ 11.5 µm` | Implemented: mandatory doc-string retains the literal "~11.5 µm" text (fixed by AC-6's `rg` contract) but the actual formula-computed runtime constant is ≈12.499 µm (≈125 units) | Reason: doc-string text is a frozen test-contract string; the real OrcaSlicer formula was implemented literally rather than force-fit. Reconciled with a note in ADR-0023.
- `[AC-N3 / "tracing warn!"]` — Specified: function emits a `tracing warn!` | Implemented: real `log::warn!` (the `log` crate, made unconditional in `slicer-core`) | Reason: `tracing` doesn't exist anywhere in this workspace; `log` is the established unconditional convention in 5 other host crates.
- `[design.md Step 6 Code Change Surface]` — Specified: module dir + manifest + `src/lib.rs` + `Cargo.toml` only | Implemented: also created `modules/core-modules/arachne-perimeters/wit-guest/{Cargo.toml,src/lib.rs}` | Reason: every other core-module has an identical `wit-guest/` companion subcrate — the actual wasm32/cdylib compile target; the top-level crate alone can't produce a guest.
- `[Doc Impact Statement]` — Specified: `docs/01_system_architecture.md` + roadmap edits only | Implemented: also flipped the P110 checkbox in `docs/07_implementation_status.md` | Reason: that file names this packet's task IDs directly with an unchecked box; leaving it stale would contradict the roadmap's DONE flip.
- `[packet.spec.md verification commands]` — Specified: bare `cargo test -p slicer-core <pattern>` commands for AC-2/AC-3/AC-4/AC-5/AC-N1 | Implemented: corrected to include `--features host-algos`, since those test binaries are feature-gated and silently report 0 tests run (exit 0) without it | Reason: caught by a post-implementation spec audit — the original commands were not actually falsifiable.
