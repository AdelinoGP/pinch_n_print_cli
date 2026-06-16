# Requirements: 110_arachne-voronoi-skt-foundations

## Packet Metadata

- Grouped task IDs:
  - `T-200` — ADR `0010-arachne-port-strategy.md`: Voronoi crate selection (boostvoronoi), pure-Rust constraints, degeneracy handling expectations. Closes D-7.
  - `T-201` — Vendor / depend on `boostvoronoi`; wrap in `slicer_core::voronoi` with Orca-shaped API surface `voronoi_from_segments(&[Segment]) -> Result<HalfEdgeGraph, VoronoiError>`.
  - `T-202` — Port `SkeletalTrapezoidationGraph` (half-edge graph storing R-values per edge).
  - `T-203` — Discretize parabolic VD edges to line segments via `discretize_parabolic_edge(focus, line_a, line_b, max_segment_len)`.
  - `T-204` — Port the 9-stage pre-processing pipeline from `WallToolPaths.cpp:590-604` (triple-offset, simplify, fixSelfIntersections, removeSmallAreas, etc.) into `arachne::preprocess::preprocess_input_outline`.
  - `T-205` — Create new `modules/core-modules/arachne-perimeters/` skeleton with manifest + empty `LayerModule` impl. Module loads under `com.core.arachne-perimeters`; `incompatible-with` declares `com.core.classic-perimeters` AND `com.core.variable-width-perimeters`.
  - `T-P96-E` — `[blocked: D-15]` Arachne MMU dedup at boundary level (NOT per-edge wall mask). Preprocessing of per-color input contour before SkeletalTrapezoidation: each color's input cell has bisector edges with neighboring different-color cells contracted/removed per the tie-break rule. The result is per-color preprocessed input cells that Arachne ingests normally. Adds `preprocess_per_color_inputs` to `arachne/preprocess.rs`.
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The current `arachne-perimeters` module is an iterative-inset width approximation — NOT real Arachne. M1's truthful-rename pass (T-090–T-092 in P108) renamed it to `variable-width-perimeters` and clearly stated this in its doc-comment. M2 reintroduces a NEW `arachne-perimeters` module that implements real OrcaSlicer Arachne: Voronoi diagram + skeletal trapezoidation + BeadingStrategy stack + extrusion generation. P110 lays the foundation: it brings in the Voronoi crate (D-7 must close — `boostvoronoi` per https://docs.rs/boostvoronoi/), ports the half-edge `SkeletalTrapezoidationGraph` that Arachne anchors bead-count assignment on, discretizes parabolic edges so downstream code consumes line segments only, ports the 9-stage input preprocess (a destructive but documented pipeline that prepares the polygon outline for VD construction), and creates the empty-but-loadable `arachne-perimeters/` module skeleton so P112 has a target to wire into.

P110 also absorbs T-P96-E: Arachne's MMU strategy is fundamentally different from Classic's. Classic uses an edge-level skip-mask (T-P96-C0..C2 in P104/P105). Arachne instead preprocesses the input contour itself — per-color cells get their bisector edges (against neighboring different-color cells) contracted/removed per ADR-0013's tie-break rule BEFORE SkeletalTrapezoidation runs. That preprocessing extension naturally lives in `arachne/preprocess.rs` alongside the 9-stage pipeline, so it's bundled here rather than in P112. Verification (cube_4color parity for Arachne) lands in P112's T-231.

This is a NEW-CODE-HEAVY packet: every task creates files (ADR, voronoi.rs, skeletal_trapezoidation/, arachne/preprocess.rs, arachne-perimeters/). There are no existing-code edits beyond two Cargo.toml dependency adds (`slicer-core/Cargo.toml` for boostvoronoi, workspace `Cargo.toml` for the new module) and the docs index `docs/01_system_architecture.md`.

## In Scope

- `docs/adr/0010-arachne-port-strategy.md` (NEW) — ADR closing D-7. Records boostvoronoi selection + degeneracy strategy.
- `docs/14_deviation_audit_history.md` — D-7 closure entry pointing at ADR-0010.
- `crates/slicer-core/Cargo.toml` — add `boostvoronoi = "0.x"` dependency.
- `crates/slicer-core/src/lib.rs` — `pub mod voronoi;` + `pub mod skeletal_trapezoidation;` + `pub mod arachne;` registration.
- `crates/slicer-core/src/voronoi.rs` (NEW) — `voronoi_from_segments`, `Segment`, `HalfEdgeGraph`, `VoronoiError`.
- `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (NEW) + `graph.rs` (NEW) + `discretize.rs` (NEW).
- `crates/slicer-core/src/arachne/mod.rs` (NEW) + `preprocess.rs` (NEW — both 9-stage pipeline + T-P96-E `preprocess_per_color_inputs`).
- `crates/slicer-core/tests/voronoi_stress.rs` (NEW) — 3 stress fixtures.
- `crates/slicer-core/tests/skt_graph_golden.rs` (NEW) — square + wedge golden fixtures.
- `crates/slicer-core/tests/preprocess_golden.rs` (NEW) — raw-outline fixture + per-color MMU dedup fixture.
- `modules/core-modules/arachne-perimeters/` (NEW DIRECTORY) — Cargo.toml, src/lib.rs (placeholder `run_perimeters` returning `Ok(())`), `arachne-perimeters.toml` manifest with `incompatible-with` declarations, `wit-guest/` shim.
- `crates/slicer-runtime/tests/contract/dag_validation.rs` (EDIT) — new test `dag_rejects_arachne_and_classic_coexistence` for AC-N2.
- `Cargo.toml` (workspace) — add `modules/core-modules/arachne-perimeters` to members.
- `docs/01_system_architecture.md` — register the three new slicer-core sub-modules.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — flip T-200..T-205 + T-P96-E to DONE.

## Out of Scope

- BeadingStrategy stack (Phase 11 / P111) — trait, 5 strategies, factory, strip-pass, 11 m_params config keys.
- Extrusion generation (Phase 12 / P112) — centrality, bead-count propagation, `generateToolpaths`, `ExtrusionLine` IR, stitch/simplify/removeSmall.
- Wire-up of `slicer_core::*` into `arachne-perimeters::run_perimeters` (P112 / T-230).
- Parity harness extension with Arachne fixtures (P112 / T-231).
- Real `arachne-perimeters::run_perimeters` implementation — this packet ships a placeholder that returns `Ok(())` and logs a `warn!` so a DAG validation test can still load the module.
- M1 implementation packets (P102..P108) — those must already be shipped.
- M1 verification (P109) — separate predecessor.
- Sibling overhang roadmap (P106/P107) — orthogonal.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~400 lines | Range-read Phase 10 rows (T-200..T-205) + Inherited-from-P96 (T-P96-E row). |
| `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` | ~80 lines | Read full — guides T-P96-E tie-break semantics. |
| `docs/03_wit_and_manifest.md` | ~600 lines | Range-read §"Module Manifest TOML" + §"incompatible-with" for T-205. |
| `docs/05_module_sdk.md` | ~700 lines | Range-read §"#[slicer_module]" + `LayerModule` trait surface for T-205. |
| `docs/01_system_architecture.md` | ~300 lines | Read §slicer-core section — where new sub-modules land. |
| `https://docs.rs/boostvoronoi/` | n/a | Delegate fetch + SUMMARY (≤ 200 words) of `voronoi_builder` + `output` API surface. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` — graph struct + edge fields (`r_min`, `r_max`, `central` markers). Delegate ONE LOCATIONS dispatch (≤ 20 entries) for: half-edge struct definition, `discretize_parabolic_edge` function, and the parabolic-tessellation math constants.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:590-604` — the 9-stage preprocess sequence. Delegate ONE SUMMARY (≤ 150 words) listing each stage's offset distance + simplify epsilon + the `epsilon_offset ~ 11.5 µm` hazard rationale.
- `docs/specs/orca-mmu-perimeter-investigation.md` (already authored by T-P96-A0 in P102) — Arachne MMU path citations for T-P96-E. NO direct OrcaSlicer read needed; use the investigation one-pager.

## Acceptance Summary

- Positive cases: `AC-1` (ADR-0010 + D-7 closure), `AC-2` (`voronoi_from_segments` square fixture), `AC-3` (Voronoi stress fixtures: collinear / T-junction / duplicate-vertex), `AC-4` (SKT graph: square + wedge golden), `AC-5` (parabolic discretization within 0.005 mm Hausdorff), `AC-6` (9-stage preprocess + hazard doc string), `AC-7` (T-P96-E per-color MMU dedup), `AC-8` (skeleton module loads with both `incompatible-with` declarations + WASM builds).
- Negative cases: `AC-N1` (empty input → `Err(EmptyInput)`), `AC-N2` (DAG rejects arachne+classic coexistence), `AC-N3` (sub-epsilon features dropped + `warn!` emitted).
- Refinements not captured in Given/When/Then:
  - `epsilon_offset` is `115` units (per coordinate system: 1 unit = 100 nm; 11.5 µm = 11500 nm = 115 units, but `WallToolPaths.cpp` uses `SCALED_EPSILON * 0.5` in the OrcaSlicer scale — confirm during T-204 implementation via the SUMMARY dispatch).
  - boostvoronoi version: pin to the latest 0.x at packet activation; the ADR records the pinned version + an audit-note line for future bumps.
  - For T-202's `r_min`/`r_max` field types: f64 (matches OrcaSlicer); for `central`: `bool`, default `false`.
  - The placeholder `run_perimeters` in T-205 MUST trace a `warn!` so test infra can detect "arachne-perimeters skeleton loaded but no walls produced" without the test reading guest logs.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile after slicer-core additions + new module | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo test -p slicer-core voronoi 2>&1 \| tee target/test-output.log` | AC-2 + AC-3 + AC-N1 | FACT pass/fail per test |
| `cargo test -p slicer-core skt_graph 2>&1 \| tee target/test-output.log` | AC-4 | FACT pass/fail |
| `cargo test -p slicer-core parabolic_discretize 2>&1 \| tee target/test-output.log` | AC-5 | FACT pass/fail |
| `cargo test -p slicer-core preprocess 2>&1 \| tee target/test-output.log` | AC-6 + AC-7 + AC-N3 | FACT pass/fail |
| `cargo xtask build-guests --check` | AC-8 WASM skeleton | FACT clean / STALE list |
| `cargo test -p slicer-runtime --test contract dag_rejects_arachne_and_classic_coexistence 2>&1 \| tee target/test-output.log` | AC-N2 | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: until T-205 (Step 6) lands, the workspace will not compile if the new module is added to `members` early — the implementer MUST add `arachne-perimeters` to the workspace `Cargo.toml` only at Step 6, not earlier. Steps 1–5 are slicer-core-internal and don't touch workspace membership.
- Step ordering rationale: ADR (Step 1) → boostvoronoi wrapper (Step 2) → SKT graph (Step 3) → discretize (Step 4) → 9-stage + per-color preprocess (Step 5) → module skeleton (Step 6) → docs (Step 7). Each step's tests must go GREEN before the next step starts. The reason: SKT graph depends on Voronoi output; discretize is consumed by SKT graph construction (parabolic edges become line segments before the graph is built); preprocess feeds the input cells that voronoi_from_segments will eventually receive (in P112 wire-up).
- Shared scratch state: the recorded golden fixtures (square / wedge / 3-collinear / T-junction / 4-color cube per-color cells / raw-outline) live under `crates/slicer-core/tests/fixtures/` and are written once in Steps 2/3/5. Subsequent steps must not edit them. Regenerating a golden during this packet masks a regression — the implementer halts if a golden fixture would need editing post-Step-5.

## Context Discipline Notes

- This packet has 7 steps. The largest is Step 5 (9-stage preprocess + T-P96-E per-color extension + 2 golden fixtures).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` is ~3000 LOC — **do not direct-read**. Use the LOCATIONS dispatch contract from the OrcaSlicer Reference Obligations section.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` is ~2500 LOC — range-read lines 590–604 ONLY via SUMMARY dispatch.
- boostvoronoi crate docs (https://docs.rs/boostvoronoi/) — delegate WebFetch + SUMMARY (≤ 200 words). Never paste the full page body.
- Likely temptation: re-read OrcaSlicer half-edge graph layout to disambiguate field semantics. **Use the LOCATIONS dispatch** (≤ 20 entries) — that's enough for Rust translation; field semantics will surface from the test failure mode if any guess is wrong.
- Sub-agent return-format for the heaviest dispatch: SkeletalTrapezoidation.cpp LOCATIONS must be ≤ 20 entries. If the dispatch returns > 20, re-dispatch tighter (narrow to graph struct definition + one function).
