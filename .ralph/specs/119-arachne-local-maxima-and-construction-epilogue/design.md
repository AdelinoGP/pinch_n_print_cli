# Design: 119-arachne-local-maxima-and-construction-epilogue

## Controlling Code Paths

- Primary code path (N9): `crates/slicer-core/src/arachne/generate_toolpaths.rs` — the end of `generate_toolpaths` (where `generateLocalMaximaSingleBeads` is appended as the final step, after A2's `connectJunctions` emission).
- Primary code path (N10): `crates/slicer-core/src/skeletal_trapezoidation/graph.rs:269-327` (`from_polygons` — where the three-pass epilogue is appended after 113c's per-edge radius bounds).
- Neighboring code path: `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (the `isLocalMaximum` predicate may live here alongside `updateIsCentral`, or in `graph.rs`).
- Neighboring tests/fixtures: `arachne_local_maxima_single_beads.rs` (NEW — AC-1), `arachne_construction_epilogue.rs` (NEW — AC-2), `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` + `toolpaths_tapered_wedge.json` (re-baseline).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **D's micro-loops are `is_odd = true` closed `ExtrusionLine`s** (canonical N4 semantics — the centerline bead of an odd-bead-count region). This is consistent with A2's `is_odd` fix (A2 owns the per-segment `is_odd` rule for the main walls; D's micro-loops are a separate emission with `is_odd = true` by construction, not a per-segment computation).
- Packet-specific constraint: **D's epilogue is additive** — three passes appended after 113c's existing per-edge radius bounds. 113c's `from_polygons` Steps 1-3 remain canonical and untouched. D does not re-derive 113c's per-cell construction.
- Packet-specific constraint: **`collapseSmallEdges`'s zero-length ε is a small constant in slicer units.** The implementer confirms the exact value via a delegated SUMMARY of `SkeletalTrapezoidationGraph.cpp`'s `collapseSmallEdges`.
- Packet-specific constraint: **WASM staleness does NOT apply** — D's change surface is `slicer-core`-internal; no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.

## Code Change Surface

- Selected approach: port `generateLocalMaximaSingleBeads` (N9) as the final step of `generate_toolpaths` and port the `constructFromPolygons` epilogue (N10) as three additive passes appended to `from_polygons`. The two are bundled because they are both "cleanup" passes (local-maxima emission + graph degeneracy cleanup) that are low-risk and share the graph-shape context.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — NEW `generate_local_maxima_single_beads` function (6-segment hexagonal micro-loop, radius `width/8`, `is_odd = true`); called as the final step of `generate_toolpaths` after A2's `connectJunctions` emission.
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — NEW `separate_pointy_quad_end_nodes`, `collapse_small_edges`, incident-edge normalization functions; appended to `from_polygons` after 113c's per-edge radius bounds (`:269-327`). NEW `is_local_maximum` predicate (or in `centrality.rs`).
  - `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs` (NEW) — AC-1.
  - `crates/slicer-core/tests/arachne_construction_epilogue.rs` (NEW) — AC-2.
  - `crates/slicer-core/tests/fixtures/arachne/centrality_*.json` + `toolpaths_tapered_wedge.json` — re-baselined via self-capture.
- Rejected alternatives:
  - **Split N9 and N10 into two packets** — rejected (both are low-risk cleanup passes sharing graph-shape context; bundling as one M packet avoids packet-management overhead).
  - **Put `isLocalMaximum` in `generate_toolpaths.rs`** — rejected (it's a graph predicate, not a toolpath-emission concern; belongs in `graph.rs` or `centrality.rs` alongside `updateIsCentral`).
  - **Make `collapseSmallEdges`'s ε configurable** — rejected (canonical hardcodes it; D should match unless a delegated SUMMARY reveals a config key).

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — role: N9 `generateLocalMaximaSingleBeads`; expected change: add `generate_local_maxima_single_beads` function + call it as the final step of `generate_toolpaths`.
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — role: N10 epilogue + `isLocalMaximum` predicate; expected change: add `separate_pointy_quad_end_nodes`/`collapse_small_edges`/incident-edge normalization, append to `from_polygons`, add `is_local_maximum`.
- `crates/slicer-core/tests/arachne_local_maxima_single_beads.rs` — role: AC-1; expected change: NEW file, near-square odd-bead-count region + hexagonal micro-loop assertion.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — full (202 lines); purpose: the `run_arachne_pipeline` + `inset0_lines` helper pattern D's tests mirror.
- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` — range-read `:100-200` (`updateIsCentral` predicate convention — D's `is_local_maximum` may mirror this style); read-only.
- `docs/02_ir_schemas.md` lines ~1091-1150 — purpose: `ExtrusionLine::is_odd` field shape.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — purpose: addendum target.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — delegate parity checks via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-core/src/arachne/pipeline.rs` — A1/A2/B/C's scope; D does not touch the pipeline stages.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — A1/B's scope.
- `crates/slicer-core/src/beading/*` — B's scope.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` — Packet F.

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "SUMMARY of `SkeletalTrapezoidation.cpp:2383-2413` `generateLocalMaximaSingleBeads` — explicitly ask for the hexagonal micro-loop geometry (6 segments, radius `width/8`, `is_odd = true`) + the `isLocalMaximum(true)` + not-central + odd-bead-count conditions; return ≤ 200 words, no code unless asked" — purpose: confirm Step 1's emission.
- "SUMMARY of `SkeletalTrapezoidation.cpp:538-546` `constructFromPolygons` epilogue — ask for the three-pass order (`separatePointyQuadEndNodes` → `collapseSmallEdges` → incident-edge normalization); return ≤ 200 words" — purpose: confirm Step 2's epilogue.
- "SUMMARY of `SkeletalTrapezoidationGraph.cpp` `collapseSmallEdges` — ask for the zero-length ε constant + the endpoint-merge rule; return ≤ 200 words" — purpose: confirm Step 2's `collapse_small_edges`.
- "SUMMARY of `SkeletalTrapezoidationGraph.cpp` `separatePointyQuadEndNodes` — ask for the node-duplication rule (which nodes are duplicated, how incident edges repoint); return ≤ 200 words" — purpose: confirm Step 2's `separate_pointy_quad_end_nodes`.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --nocapture`; return FACT pass/fail or SNIPPETS on failure" — purpose: validate AC-1.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --nocapture`; return FACT pass/fail" — purpose: validate AC-2.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --test arachne_parity_red_perimeter_index --test arachne_parity_red_is_odd_semantics --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT pass (expected — N1/N2/N4/N3 stay green)" — purpose: gate D didn't regress A1/A2/B.
- "Run `cargo test -p slicer-core --features host-algos --test centrality --test bead_count --test propagation --test generate_toolpaths 2>&1`; return FACT pass/fail (fixtures re-baselined)" — purpose: regression gate.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. D's surface is `slicer-core`-internal; no WIT/IR change. D's micro-loops are `ExtrusionLine` with `is_odd = true` (existing field shape).
- WIT boundary considerations: **none**. No WIT/IR schema change.
- Determinism: D's changes preserve determinism (graph walks are index-ordered; the micro-loop emission is a deterministic per-node predicate; `collapseSmallEdges`'s endpoint merge is deterministic under ties via index-ascending).

## Locked Assumptions and Invariants

- D's micro-loops are `is_odd = true` closed `ExtrusionLine`s (canonical N4 semantics). Consistent with A2's `is_odd` fix.
- D's epilogue is additive — three passes appended after 113c's existing per-edge radius bounds. 113c's `from_polygons` Steps 1-3 remain canonical.
- `collapseSmallEdges`'s zero-length ε is a small constant in slicer units (the implementer confirms via delegated SUMMARY).
- D keeps N1, N2, N3, N4 red tests GREEN (gated).
- D's `isLocalMaximum` predicate: a node is a local maximum if all its neighbors have `distance_to_boundary <=` its own.
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.

## Risks and Tradeoffs

- **`collapseSmallEdges`'s endpoint merge could ripple into A1's junction fans.** Merging endpoints changes edge topology; A1's `generate_junctions` walks edges. Risk is contained by the N1 red tests (AC-N1 stays green) + the `generate_toolpaths` regression suite.
- **`separatePointyQuadEndNodes`'s node duplication changes the graph's vertex count.** Downstream stages (centrality, bead_count, propagation) must handle the duplicated nodes. Risk is contained by the regression suite (centrality/bead_count/propagation fixtures re-baselined).
- **`generateLocalMaximaSingleBeads`'s micro-loops interact with E's `removeSmallLines`.** D's micro-loops are `is_odd = true` closed lines; E's `removeSmallLines` only removes `is_odd && !is_closed` lines, so closed micro-loops survive. But E's post-process order change (N11) could affect them. D's commit message must flag this for E.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — the epilogue's three passes + the `isLocalMaximum` predicate + fixture re-baseline, the bulk of the work).
- Highest-risk dispatch: the `collapseSmallEdges` SUMMARY — its return could blow budget if it returns code instead of prose. Required return format: `SUMMARY ≤ 200 words, no code unless asked`.

## Open Questions

- [FWD] Does `isLocalMaximum` belong in `graph.rs` (as a standalone graph predicate) or `centrality.rs` (alongside `updateIsCentral`)? The implementer decides based on which file's conventions are a better fit; `centrality.rs` already has `updateIsCentral`'s neighbor-walk pattern.
- [FWD] Does `collapseSmallEdges`'s zero-length ε need to match the `SNAP_FRAC` constant in `propagation.rs:49` (1e-6 as a fraction of edge length), or is it an absolute slicer-unit constant? The delegated SUMMARY should clarify.
- [FWD] Should the hexagonal micro-loop's 6 segments be computed in slicer units or mm? `width/8` is in slicer units (the `Beading::bead_widths` are in slicer units per `beading/mod.rs`); D should keep the computation in slicer units and convert to mm only at the `ExtrusionLine` emission boundary if needed.

None activation-blocking.