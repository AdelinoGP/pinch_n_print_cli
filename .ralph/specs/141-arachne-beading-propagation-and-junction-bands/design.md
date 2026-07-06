# Design: 141-arachne-beading-propagation-and-junction-bands

## Controlling Code Paths

- Primary code path: `crates/slicer-core/src/arachne/generate_toolpaths.rs:192-334` (`generate_junctions`) — the divergent centrality-gated / both-half-edges / clamp-out-of-band / per-endpoint-beading scheme A1 rewrites to canonical.
- Neighboring code paths: `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:120-160` (`upward_central_edges` centrality gate), `:810-860` (`primary_source_vertices`), `:980-1100` (`propagate_beadings_downward` + `interpolate_bead_counts` rounded-integer blend).
- Neighboring tests/fixtures: `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` (N1 red tests — A1's oracle), `crates/slicer-core/tests/centrality.rs`/`bead_count.rs`/`propagation.rs`/`generate_toolpaths.rs` (self-capture fixtures that will re-baseline).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **`BeadingPropagation` is a side table, not a field on `STVertex`.** Upstream keeps the full `Beading` per node in a side structure (`SkeletalTrapezoidation.cpp:2091-2127`), not on the vertex itself. A1 must match this layout — putting a `Beading` on `STVertex` would bloat the struct and break the existing `STVertex: PartialEq` derive across centrality/bead_count/propagation fixtures. The side table is owned by `SkeletalTrapezoidationGraph` (a `Vec<Beading>` indexed by vertex, with a sentinel for "no beading yet", or `HashMap<usize, Beading>` if sparse — the implementer decides based on upstream's actual density, which the audit summary says is full per-node).
- Packet-specific constraint: **A1 must not remove the π hack (`pipeline.rs:334`) or the 0.1× filter-dist fudge (`pipeline.rs:272-277`).** Those are Packet C's (`144`) scope, strictly after A2. A1's rewrite is gated on the centrality scheme the π hack sustains.
- Packet-specific constraint: **A1 must not touch `arachne_pipeline.rs:122` or delete `assign_perimeter_indices`.** Both are A2's scope. A1 leaves `perimeter_index = 0` at junction generation.
- Packet-specific constraint: **WASM staleness does NOT apply** — A1's change surface is `slicer-core`-internal (`arachne/`, `skeletal_trapezoidation/`); no path feeds the guest WASM build (`wit/`, `slicer-macros`, `slicer-sdk`, `slicer-ir`, `slicer-schema`, core-modules). The `wasm-staleness` snippet is intentionally omitted.
- Packet-specific constraint (added 2026-07-06, see `packet.spec.md`'s "Known
  Implementation Hazard"): **the beading MUST be resolved at the peak
  (`edge.to`) always — never at `edge.from` as a primary path with the peak
  as a fallback — and each junction's width MUST come from that ONE resolved
  beading's own `bead_widths[idx]`, never a fresh per-bead
  `strategy.compute()` call.** `generate_junctions` must have NO
  `edge.central`/`edge.edge_type == EdgeType::EXTRA_VD` gate, and the domain
  seeding in `generate_toolpaths` must NOT gain a matching filter either —
  canonical has zero such checks. Run
  `cargo test -p slicer-core --features host-algos --test arachne_generate_junctions_canonical_regression`
  after any change to `generate_junctions` to confirm.

## Code Change Surface

- Selected approach: faithful port of canonical `generateJunctions` + `getBeading`/`BeadingPropagation`, replacing PNP's centrality-gated / both-half-edges / clamp / per-endpoint-beading scheme atomically. N7 (side table) lands first as Step 1, gated by a structural test; N1 (junction rewrite) lands as Step 2, gated by the N1 red tests. The two are bundled because N7 has no dedicated red test (only N1's tests validate the combined system).
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — `upward_central_edges` (drop centrality filter), `primary_source_vertices` (relax centrality gate), `propagate_beadings_downward` + `interpolate_bead_counts` (interpolate widths/locations, not rounded integers, into the side table). NEW: `BeadingPropagation` side-table type + `get_beding`/`get_nearest_beding` (0.1 mm radius in slicer units = 1000 units).
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — `SkeletalTrapezoidationGraph` gains a `beading_propagation: Vec<Beading>` (or `HashMap`) field, initialized empty by `from_polygons`, populated by propagation passes.
  - `crates/slicer-core/src/arachne/generate_toolpaths.rs` — `generate_junctions` (`:192-334`) rewritten: no centrality gate, upward-half-edge-only skip, flat/same-bead-count skip, single `get_beding` at peak node, in-band beads only (middle-index start, break on `bead_R < end_R`), no clamping, near-`start_R` snap.
  - `crates/slicer-core/tests/arachne_junction_upward_half_edge_only.rs` (NEW) — AC-N1 structural test.
  - `crates/slicer-core/tests/arachne_generate_junctions_canonical_regression.rs` (NEW, added 2026-07-06 as part of the hazard correction) — AC-3, 3 bug-regression locks isolated from the chain walk.
  - `crates/slicer-core/tests/fixtures/arachne/{centrality_*.json, propagation_*.json, bead_count_tapered_wedge.json, toolpaths_tapered_wedge.json}` — re-baselined via self-capture.
- Rejected alternatives:
  - **Split N7 into a standalone A0 with a structural test suite** — rejected during grilling (user decision: keep bundled in A1). N7's structural tests are a weak oracle (no parity check); bundling with N1 gives N7 a real acceptance oracle via the N1 red tests.
  - **Put `Beading` on `STVertex`** — rejected (bloats the struct, breaks `PartialEq` derives on existing fixtures). Side table matches upstream.
  - **Remove the π hack in A1** — rejected (load-bearing for A1's centrality-gated scheme until A1's rewrite lands; Packet C owns removal strictly after A2).

## Files in Scope (read + edit)

- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` — role: N7 side table + propagation gating; expected change: add `BeadingPropagation` side table, `get_beding`/`get_nearest_beding`, drop centrality gate from `upward_central_edges`/`primary_source_vertices`, replace `interpolate_bead_counts` with width/location blend into side table.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — role: N1 junction rewrite; expected change: rewrite `generate_junctions:192-334` to canonical scheme (upward-only, in-band, no clamp, single `get_beding` at peak).
- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` — role: side-table field on `SkeletalTrapezoidationGraph`; expected change: add `beading_propagation` field, init empty in `from_polygons`, preserve `PartialEq` (likely `#[derive(PartialEq)]` on the struct needs the field to be `PartialEq` — `Beading` already derives `PartialEq`).

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-core/src/beading/mod.rs` — read full (108 lines); purpose: confirm `Beading` struct shape (`bead_widths`, `toolpath_locations`, `left_over`, `total_thickness`) and its `PartialEq` derive for the side table.
- `crates/slicer-core/tests/arachne_parity_red_junction_bands.rs` — read full (202 lines); purpose: A1's acceptance oracle — understand the exact assertions (≤ 0.6 mm boundary distance for rectangle, ≤ 0.15 mm deviation from 0.2 mm for square).
- `crates/slicer-core/tests/arachne_parity_red_transition_ends.rs` — read full (217 lines); purpose: AC-N1's fixture shape (single central twin-pair edge) — A1's structural test reuses this topology.
- `docs/02_ir_schemas.md` lines ~1091-1150 — purpose: `ExtrusionJunction`/`ExtrusionLine` field shapes.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — purpose: substrate A1 builds on; supersession addendum target.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks.

- `OrcaSlicerDocumented/...` — delegate parity checks via the `orca-delegation` contract; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs:640-740` (`apply_transitions` body) — Packet B's scope; A1 does not touch it.
- `crates/slicer-core/src/beading/{distributed,widening,redistribute,outer_wall_inset,limited,factory}.rs` — A1 does not extend the `BeadingStrategy` trait (Packet B); read-only for `Beading` shape only.
- `crates/slicer-core/src/arachne/pipeline.rs:334` (π hack) and `:272-277` (0.1× fudge) — Packet C's scope; A1 leaves them in place.
- `crates/slicer-core/tests/arachne_pipeline.rs:122` — A2's scope; A1 leaves it red.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/*` — large JSONs; re-record via `#[ignore]`d `record_*` functions only (Packet F owns the cross-crate batch; A1 re-baselines only `slicer-core` fixtures).

## Expected Sub-Agent Dispatches

List the dispatches the implementer is expected to make.

- "SUMMARY of `SkeletalTrapezoidation.cpp:2013-2079` `generateJunctions` — explicitly ask for the upward-skip / in-band-break / middle-index-start loop structure, NOT just a callee summary; return ≤ 200 words, no code unless asked" — purpose: confirm Step 2's rewrite shape.
- "SUMMARY of `SkeletalTrapezoidation.cpp:2091-2127` `getBeading`/`getNearestBeading` — ask for the 0.1 mm radius constant in slicer units and the nearest-lookup algorithm; return ≤ 200 words" — purpose: confirm Step 1's side-table lookup.
- "SUMMARY of `SkeletalTrapezoidation.cpp:1833-1899` `propagateBeadingsDownward` — ask for the `ratio_of_top` blend over bead widths/locations (not integer counts) and the central-edge skip; return ≤ 200 words" — purpose: confirm Step 1's interpolation fix.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_rectangle_outer_wall_junctions_stay_near_boundary --nocapture`; return FACT (pass) or SNIPPETS (fail with assertion + ≤ 20 lines)" — purpose: validate Step 2's AC-1.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands -- n1_square_outer_wall_junctions_at_outer_bead_radius --nocapture`; return FACT (pass) or SNIPPETS (fail)" — purpose: validate Step 2's AC-2.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_perimeter_index --no-fail-fast`; return FACT fail (expected — confirms N2 stayed red)" — purpose: gate A1's scope boundary.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_is_odd_semantics --no-fail-fast`; return FACT fail (expected — confirms N4 stayed red)" — purpose: gate A1's scope boundary.
- "Run `cargo test -p slicer-core --features host-algos --test arachne_parity_red_transition_ends --no-fail-fast`; return FACT fail (expected — confirms N3 stayed red)" — purpose: gate A1's scope boundary.
- "Find all callers of `upward_central_edges` and `interpolate_bead_counts`; return LOCATIONS" — purpose: confirm no orphan call sites after the signature changes.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. `ExtrusionJunction::perimeter_index` stays `u32` and stays `0` at A1's layer (A2 sets it to `bead_idx`). `ExtrusionLine`/`ExtrusionJunction` field shapes unchanged.
- WIT boundary considerations: **none**. A1's change surface is `slicer-core`-internal; no WIT/IR schema change. The `perimeter_index` semantic change is A2's scope decision (wire-type-transparent).
- Determinism: A1's rewrite preserves determinism (index-ordered traversal; the upward-half-edge skip and in-band bead filter are deterministic given the graph). The `getNearestBeading` 0.1 mm radius lookup must be deterministic under ties (index-ascending tiebreak, matching upstream's `BTreeSet`/`std::map` ordering).

## Locked Assumptions and Invariants

- `BeadingPropagation` is a side table on `SkeletalTrapezoidationGraph`, not a field on `STVertex` — keeps the vertex struct small, matches upstream, preserves `STVertex: PartialEq` derives.
- `getBeading`'s `getNearestBeading` radius is 0.1 mm = 1000 slicer units (1 unit = 100 nm per `docs/08_coordinate_system.md`).
- A1 leaves the π hack (`pipeline.rs:334`), the 0.1× filter-dist fudge (`pipeline.rs:272-277`), `arachne_pipeline.rs:122`, and `assign_perimeter_indices` (`pipeline.rs:384-390`) untouched — all are downstream packets' scope.
- A1 keeps N2, N3, N4 red tests RED (gated by the "stays red" verification commands).
- `Beading` invariant `bead_widths.len() == toolpath_locations.len()` preserved on every side-table entry; debug-assert in `get_beding`'s hot path.
- Fixture re-baseline uses the self-capture pattern (first-run writes if missing, subsequent compare) — never read the JSONs directly.

## Risks and Tradeoffs

- **N7's structural test is a weak oracle.** The side table's correctness is only fully validated by N1's red tests (the combined system). If Step 1's structural test passes but Step 2's N1 tests fail, the bug could be in either N7 or N1 — the implementer must bisect via the structural test's invariants. This is the accepted tradeoff of bundling (user decision).
- **`upward_central_edges` signature change ripples into `propagate_beadings_upward`/`downward` and `compute_dist_to_bottom_source`.** The implementer must find all callers (dispatch listed) and update call sites. Risk is contained (the function is private to `propagation.rs`).
- **Fixture re-baseline may mask regressions.** The self-capture pattern locks in *this* implementation's behavior, not OrcaSlicer ground truth. The N1 red tests are the real parity oracle; the fixtures guard self-regression only.
- **Bisect confusion across A1→A2 boundary.** Between A1 and A2, N2/N4 red tests stay red. The "stays red" verification commands gate this, but a future bisect across the boundary will see red tests that are "expected red" — the implementer must record the A1/A2 boundary in commit messages.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — the `generate_junctions` rewrite, which is the bulk of the work and requires the heaviest OrcaSlicer dispatch).
- Highest-risk dispatch: the `generateJunctions` SUMMARY dispatch — its return could blow budget if it returns code instead of prose. Required return format: `SUMMARY ≤ 200 words, no code unless asked`; if the sub-agent returns code, re-dispatch with a tighter scope.

## Open Questions

- [FWD] Should `BeadingPropagation` be `Vec<Beading>` (indexed by vertex, sentinel for "no beading yet") or `HashMap<usize, Beading>` (sparse)? The audit summary says upstream keeps a full `Beading` per node, suggesting `Vec`, but the implementer should confirm density via a delegated read of `SkeletalTrapezoidation.cpp:2091-2127` and choose the cheaper representation. Either preserves the `Beading: PartialEq` invariant.
- [FWD] Does `get_beding` need to handle the case where the side table is empty (no propagation has run yet)? Upstream's `getBeading` falls back to `compute(2R, bead_count)` for primary sources; A1's `get_beding` should match. The implementer confirms via the delegated SUMMARY.

None activation-blocking.