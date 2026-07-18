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
---

# 110_arachne-voronoi-skt-foundations

## Goal

Land the M2 foundations layer for real Arachne: write ADR-0023 (D-7 is already CLOSED in the roadmap — the ADR documents the crate selection decision that closes it), add `slicer_core::voronoi` as the Orca-shaped wrapper around boostvoronoi, port `SkeletalTrapezoidationGraph` (half-edge graph storing per-edge R-values), discretize parabolic Voronoi edges to line segments, port the 9-stage Arachne input pre-processing pipeline from `WallToolPaths.cpp:590-604` (including the T-P96-E per-color boundary-level MMU dedup), and CREATE the new `arachne-perimeters/` core-module skeleton (new directory, manifest, empty `LayerModule` impl) with `incompatible-with = ["com.core.classic-perimeters"]`.

**Predecessor P108 (`status: implemented`) already deleted the old fake `arachne-perimeters/`** (the 512-line iterative-inset approximation, removed under D-110-DROP-VARIABLE-WIDTH; `modules/core-modules/arachne-perimeters/` is confirmed absent from the tree). P110/T-205 creates the real-Arachne skeleton FRESH in that now-empty path — this is NOT an edit of an existing module. Real wire-up of `run_perimeters` against the new slicer-core modules is P112's job (T-230).

## Problem Statement

**Predecessor P108 (`implemented`) already deleted the old fake `arachne-perimeters/`** (a 512-line iterative-inset approximation that is NOT real Arachne). P110/T-205 now creates a FRESH `arachne-perimeters/` skeleton for real Arachne — new directory, new manifest, empty `LayerModule` impl — in the path P108 emptied (confirmed absent).

P110 lays the M2 foundation: it brings in the Voronoi crate (D-7 was already marked CLOSED in `docs/specs/perimeter-modules-orca-parity-roadmap.md` — ADR-0023 documents the rationale), ports the half-edge `SkeletalTrapezoidationGraph` that Arachne anchors bead-count assignment on, discretizes parabolic edges so downstream code consumes line segments only, ports the 9-stage input preprocess (a destructive but documented pipeline that prepares the polygon outline for VD construction), and CREATES the new `arachne-perimeters/` skeleton declaring `incompatible-with = ["com.core.classic-perimeters"]`. Real wire-up of `run_perimeters` against the new slicer-core modules lands in P112/T-230.

P110 also absorbs T-P96-E: Arachne's MMU strategy is fundamentally different from Classic's. Classic uses an edge-level skip-mask (T-P96-C0..C2 in P104/P105). Arachne instead preprocesses the input contour itself — per-color cells get their bisector edges (against neighboring different-color cells) contracted/removed per ADR-0013's tie-break rule BEFORE SkeletalTrapezoidation runs. That preprocessing extension naturally lives in `arachne/preprocess.rs` alongside the 9-stage pipeline, so it's bundled here rather than in P112. Verification (cube_4color parity for Arachne) lands in P112's T-231.

This is a NEW-CODE-HEAVY packet: every task creates files (ADR, voronoi.rs, skeletal_trapezoidation/, arachne/preprocess.rs, arachne-perimeters/). There are no existing-code edits beyond two Cargo.toml dependency adds (`slicer-core/Cargo.toml` for boostvoronoi, workspace `Cargo.toml` for the new module) and the docs index `docs/01_system_architecture.md`.

## Architecture Constraints

<!-- snippet: coord-system -->
- **Coordinate system hazard.** All Voronoi/SKT geometry passes through `slicer_core::voronoi::Segment` keyed in slicer units (1 unit = 100 nm). OrcaSlicer's `epsilon_offset` is `SCALED_EPSILON * 0.5` ≈ 11.5 µm in real space; in slicer units that's `115` (not `11500` — divide OrcaSlicer constants by 100 per `docs/08_coordinate_system.md`). The implementer MUST translate every OrcaSlicer scale constant through `mm_to_units` or the explicit `/100` rule before pasting into Rust. Any constant > 100000 in geometry code is a red flag.

<!-- snippet: wasm-staleness -->
- **Guest WASM staleness.** P110 triggers a guest rebuild on TWO fronts: (1) Step 6 CREATES a new core module `modules/core-modules/arachne-perimeters/` (a fresh guest with its own manifest + `src/lib.rs`, in the path P108 emptied); and (2) Steps 2/5 add new modules to `crates/slicer-core/**`, a **universal guest dependency baked into every guest WASM** (per CLAUDE.md §"Guest WASM Staleness"). So after the Step 2/5 `slicer-core` edits AND after the Step 6 module creation the implementer MUST run `cargo xtask build-guests --check` and rebuild if `STALE:`; otherwise AC-N2's DAG-validation test can fail with a typed-instantiation error masquerading as a packet bug. **OPEN RISK (implementer must verify in Step 2):** the new Voronoi code wraps `boostvoronoi`, a C++-FFI crate that does NOT target `wasm32`. It MUST stay behind the host-only `host-algos` feature so it is never compiled into a guest; if `voronoi.rs`/`skeletal_trapezoidation`/`arachne::preprocess` leak into the default (guest) feature set, every guest build breaks. Gate them host-only and confirm `cargo xtask build-guests` still compiles.

- **boostvoronoi licensing & versioning.** boostvoronoi is BSL-1.0; the workspace is MIT/Apache-2.0. The ADR (T-200) MUST record the license decision; the dependency is added to `slicer-core/Cargo.toml` only. No other crate depends on it.
- **Determinism.** `voronoi_from_segments` MUST be deterministic for the same input segment order. boostvoronoi's underlying Boost.Polygon algorithm is deterministic; the wrapper does not introduce any HashMap/HashSet over float keys (use sorted Vec or BTreeMap if needed).
- **No panics in geometry code.** Every wrapper function returns `Result<_, VoronoiError>` or `Result<_, PreprocessError>`. Internal `unwrap()` is forbidden in `voronoi.rs`, `graph.rs`, `discretize.rs`, `preprocess.rs` — invariants get explicit error variants.

## Data & Contract Notes

- **`Segment`**: `{ a: Point2, b: Point2 }` with `Point2 { x: i64, y: i64 }` in slicer units. Uses `slicer_ir::Point2` (the existing i64 struct at `crates/slicer-ir/src/slice_ir.rs:81`). NOTE: `slicer_core::geometry::Point2` does NOT exist — `geometry.rs` imports `slicer_ir::Point2` via `use slicer_ir::{ExPolygon, Point2}` (verified line 16). Do NOT define a new Point2 in `voronoi.rs`.
- **`HalfEdgeGraph`**: `{ vertices: Vec<Vertex>, edges: Vec<HalfEdge> }` where `HalfEdge { start_vertex: usize, twin: usize, next: usize, prev: usize, cell: usize, is_primary: bool, is_curved: bool }`. The `is_curved` flag drives discretization (Step 4).
- **`SkeletalTrapezoidationGraph`**: extends `HalfEdgeGraph` with per-edge `r_min: f64`, `r_max: f64`, `central: bool` (default false). R-values are slicer-unit distances.
- **`PreprocessParams`**: bundles `epsilon_offset: f64`, `simplify_epsilon: f64`, `min_feature_size: f64`. Defaults match OrcaSlicer (translated through the /100 rule).
- **`TieBreakRule`**: enum `{ LowerToolIndexWins, HigherToolIndexWins, Custom(fn(ToolIndex, ToolIndex) -> ToolIndex) }`. Default `LowerToolIndexWins` per ADR-0013.
  > **Correction (post-closure):** no `TieBreakRule` type exists in the shipped code. ADR-0013 (lines 9, 29, 32, 40) retired the tie-break model; OrcaSlicer's `PerimeterGenerator.cpp:2600-2653` confirms Arachne has no color-aware logic. `preprocess_per_color_inputs` is a pass-through. See `closure-log.md` §3.
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
