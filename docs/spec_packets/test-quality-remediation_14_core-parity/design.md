# Design: core-parity

## Controlling Code Paths

- Primary code path: the nine additive `#[test]` functions in `crates/slicer-core/tests/` driving public production entry points: `SkeletalTrapezoidationGraph::from_polygons` + `populate_beading_propagation` + `get_nearest_beding` (side table), `apply_transitions` (transition splits), `generate_toolpaths` (junction chains), `run_arachne_pipeline` (dumbbell/postprocess), and `LimitedBeadingStrategy::compute`/`compute_and_strip` (cap boundary).
- Neighboring tests/fixtures: the existing locks in the same nine files (sixteen anchors listed in AC-N2) are read-only references for style and must survive byte-for-byte assertion-identical; `crates/slicer-core/tests/fixtures/beading/limited_cap_boundary.json` is read but never edited (the at-cap case is constructed inline).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules. Item 8 has no Orca counterpart.

## Architecture Constraints

- **Additive-only, test-only:** no production edit, no threshold/tolerance change, no exact-pin removal. Every step adds `#[test]` functions (or replaces the explicitly-marked `let _ = max_deviation_x; // placeholder` line with a real assertion); existing assertion bodies are not weakened.
- **Feature-correct invocation:** the eight arachne files are auto-discovered integration targets gated in-file by `#![cfg(feature = "host-algos")]`; `beading_limited` is an explicit `[[test]]` stanza with no `required-features`. All AC runs use `--features host-algos`; AC-N1 proves the gate matters. (Census: all nine targets recorded with `required: []`; the gates live in the files, not the manifest.)
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): new test literals of watched types (`SkeletalTrapezoidationGraph`, `STVertex`, `STHalfEdge`, `Beading`, `ExtrusionLine`, `ArachneParams`, `Point2`) must use `..` rest (FRU) or an `// exhaustive:` waiver; the touched files' existing FRU style is the template. `cargo xtask check-literals` runs before committing.
- Test-quality gate (`docs/22_test_quality.md`, report mode): every new assertion must be independently falsifiable — exact value/array pins, no self-referential oracles. Where a measured baseline is required (items 5, 7), the measured values are pinned as literal expected arrays with the canonical claim documented in a comment.
- Red/parity discipline: assertions encode canonical contract; where current behavior is documented as divergent (item 2's non-retained source projection), the test pins exactly what the implementation guarantees and documents the divergence in a comment — it never pins a wrong value as canonical.

## Code Change Surface

- Selected approach: one new test (or a tight pair in the dumbbell file) per step, written in the existing file's fixture style, reusing the file's own builders (`make_f2_target_graph`, `make_split_target_graph`, `make_f3_target_graph`, `l_shape`, `dumbbell_polygon`, `load_fixture`/`build_strategy`, `p_mm`, `junction`, `expoly`) so every step is a single-file diff.
- Exact functions, traits, manifests, tests, and fixtures:
  - S1: add `get_nearest_beding_includes_vertex_at_exact_radius_boundary` — hand-built 3-vertex chain (`STVertex`/`STHalfEdge` FRU style of `make_f3_target_graph`), populated side-table entries via direct `beading_propagation` assignment (`pub` field), distances exactly `1000.0`/`2000.0` units; oracle contract from `get_nearest_beding` (`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`): inclusive `edge_len <= radius_units` seeding, BFS-by-cumulative-distance nearest.
  - S2: add `apply_transitions_diagonal_source_mid_r_and_foot_sentinels` — diagonal-edge twin-fixture variant of `make_f2_target_graph(mid_r_units)`; asserts mid node `distance_to_boundary == mid_r` (`750_000.0`), two feet `0.0`/`None`, cross-side foot equality, mid on the edge line; divergence comment per `insert_node` (`crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`): source provenance not retained — feet sit at the projected-to-line position, not the true source-segment foot.
  - S3: add `apply_transitions_split_topology_exact_counts_and_cross_twin_patch` on `make_split_target_graph` — exact counts: 3 new vertices (1 mid `Some(lower_bc)` + `mid_r`, 2 feet `0.0`/`None`), 6 new edges (4 `EXTRA_VD` ribs in 2 pairs, 2 `NORMAL` second fragments), cross-twin twin-links consistent, exact split position; replace the `let _ = max_deviation_x;` placeholder in `apply_transitions_split_position_coincides_on_both_sides` (line 332) with `assert!(max_deviation_x <= 1.0)` (units).
  - S4: add `f5_invariant_node_distances_match_rib_geometry_and_boundary` on `l_shape` via `from_polygons` — per-node oracle: boundary/rib-foot nodes (`distance == 0.0`) lie on a polygon segment (independent point-to-segment distance `<= 1` unit); no `NaN`, nothing `< 0.0`; every `EXTRA_VD` rib pair's spine endpoint distance equals the independently computed rib length (endpoint positions); semantics anchored to `make_rib`/`make_node_vd` (`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`) and `edge_radius_bounds`' NAN handling.
  - S5: add `dumbbell_wide_gap_not_dissolved_pins_exact_ring_topology` (existing `dumbbell_polygon`, gap `1.5` mm > `0.4` mm — canonical `filterNoncentralRegions` does NOT dissolve) and `dumbbell_narrow_gap_dissolves_to_single_closed_ring` (variant with central-region separation `<= 0.4` mm — canonical dissolution → exactly one closed inset-0 ring); both pin measured deterministic counts with the canonical rule documented.
  - S6: add `limited_inserts_single_centre_sentinel_at_cap_boundary` — uses `load_fixture()`+`build_strategy()` and `compute(24000.0, 6)`: exact 7-entry arrays (widths `[4000,4000,4000,0,4000,4000,4000]`, locations `[2000,6000,10000,12000,14000,18000,22000]`), `total_thickness 24000.0`, `left_over 0.0`; `compute_and_strip` returns the six uniform beads; exercises the at-cap branch `actual_count.is_multiple_of(2) && actual_count == self.max_bead_count` of `crates/slicer-core/src/beading/limited.rs` (canonical `LimitedBeadingStrategy::compute` odd-centre cap-boundary shape).
  - S7: add `chain_junctions_land_at_documented_interpolated_positions` on `make_f3_target_graph` + `SymmetricBeadingStrategy` — `xs[0] == 1.25` mm, `xs[2] == 8.75` mm (values documented in the file's own traversal doc comment, `t = 0.75`), `xs[1]` measured-pinned strictly between, all `y == 0.0`, `len == 3`.
  - S8: add `arachne_params_absent_keys_fall_back_to_defaults_per_key` — empty `ConfigView`; per wired key (`min_central_distance`, `min_width`, `min_bead_width`, `wall_transition_length`, `wall_transition_angle`, `initial_layer_min_bead_width`, `outer_wall_offset` + bool `detect_thin_wall`), assert `None`/absent and the exact `ArachneParams::default()` fallback value (`0.0`, `0.4`, `0.4`, `0.4`, `10.0_f64.to_radians()`, `0.34`, `0.0`, `false`); key list verified against `arachne_params_from_config` in `modules/core-modules/arachne-perimeters/src/lib.rs` (ranged read only).
  - S9: add `canonical_remove_small_first_keeps_line_simplify_first_drops_it` — odd open `ExtrusionLine`, min junction width `0.4` (threshold `0.2` mm), out-and-back micro-segment polyline length `>= 0.2` mm with chord `< 0.2` mm; canonical branch `remove_small_lines(line, 0.5, 0.4, false, false)` → `len == 1`; old-order branch `simplify_toolpaths(line, 0.0025, 0.000025, 2e-6)` then `remove_small_lines` → empty; header pin (`WallToolPaths::generate` removal-before-simplification order) untouched.
  - S10: ledger step — plan §7 `core` row only.
- Rejected alternatives and reasons:
  - Editing the beding-radius/side-table fixture JSON to add boundary cases — rejected: keeps the edit surface to the nine approved files.
  - Adding the at-cap case to `limited_cap_boundary.json` — rejected for the same reason; the inline construction mirrors the fixture's analytic derivation verbatim.
  - Asserting the diagonal transition's true projected foot position (item 2) — rejected as knowingly red: `insert_node` documents that source provenance is not retained; the test pins what is guaranteed and documents the divergence.
  - A WASM module-fallback-table test for item 8 — rejected at grounding: the module performs per-key `unwrap_or(defaults.X)` reads, not a table; the local mirror is the §5.1 reading.
  - Touching `arachne_beading_is_odd_semantics.rs` for item 6 — rejected: §5.1 says "odd-cap boundary", not is-odd line-marking.

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting. This packet touches exactly one file per step plus one shared ledger doc — no extras.

- `crates/slicer-core/tests/arachne_beding_propagation_side_table.rs` - role: S1 home; expected change: one new `#[test]` + hand-built chain helpers if needed
- `crates/slicer-core/tests/arachne_construction_apply_transitions_mirror_fix.rs` - role: S2 home; expected change: one new `#[test]` + diagonal fixture variant
- `crates/slicer-core/tests/arachne_construction_insert_node_rib_split.rs` - role: S3 home; expected change: one new `#[test]` + placeholder replacement in the existing split-position test
- `crates/slicer-core/tests/arachne_construction_node_distance_perp_foot.rs` - role: S4 home; expected change: one new `#[test]` with per-node oracle
- `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs` - role: S5 home; expected change: two new `#[test]`s + a narrow-gap fixture builder
- `crates/slicer-core/tests/beading/limited.rs` - role: S6 home; expected change: one new `#[test]` (inline at-cap case, fixture untouched)
- `crates/slicer-core/tests/arachne_stitch_chain_junctions_t_to_fix.rs` - role: S7 home; expected change: one new `#[test]`
- `crates/slicer-core/tests/arachne_pipeline.rs` - role: S8 home; expected change: one new `#[test]`
- `crates/slicer-core/tests/arachne_postprocess_order.rs` - role: S9 home; expected change: one new `#[test]` with the divergence fixture
- `docs/specs/test-quality-remediation-plan.md` - role: shared ledger; expected change: §7 `core` row only (S10)

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` - lines 620-716 only - purpose: `get_nearest_beding` BFS contract (inclusive radius, cumulative distance); lines 935-1060 only - purpose: `make_rib`/`make_node_vd` F5 semantics; lines 1519-1592 only - purpose: retained `nearest_boundary_distance` oracle + `edge_radius_bounds` NAN handling
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` - lines 178-460 only - purpose: `generate_transition_mids` (`mid_r = get_transition_thickness / 2`) and `insert_node` faithful split contract
- `crates/slicer-core/src/beading/limited.rs` - lines 100-225 only - purpose: at-cap single-sentinel branch and over-cap block placement to be pinned by S6
- `crates/slicer-core/src/arachne/pipeline.rs` - lines 76-231 and 369-380 only - purpose: `ArachneParams::default()` values and the canonical post-process order in `run_arachne_pipeline`
- `crates/slicer-core/src/arachne/remove_small.rs` - lines 45-112 only - purpose: removal threshold (`min_junction_width * min_length_factor`) and XY polyline-length metric for the S9 fixture
- `modules/core-modules/arachne-perimeters/src/lib.rs` - lines 280-340 only - purpose: wired config keys for the S8 fallback contract (read-only, different workspace)
- `crates/slicer-core/tests/fixtures/beading/limited_cap_boundary.json` - 41 lines, direct - purpose: parent params and sentinel derivation style for the inline S6 case
- `docs/specs/test-quality-remediation-plan.md` - lines 155-166, 269-289, 291-311, 312-352 only - purpose: §5.1 PARITY row, §6 invocation patterns, §7 ledger, queue row #14

## Out-of-Bounds Files

- Canonical OrcaSlicer upstream (pinned at `40eab797c6a60a5949c0f92d00798da414c4b44a`, see dispatches below) and any local `OrcaSlicerDocumented/` checkout - delegate; never load directly
- `docs/specs/test-quality-remediation-census.json` - never touch, never full-load (targeted greps only)
- Other `docs/spec_packets/*` directories - never modify
- `crates/slicer-core/src/**` production files - read-only ranges above; never edited
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-wasm-host/`, `modules/core-modules/*/src/**` (except the ranged read above) - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

All Orca refs below are canonical `https://github.com/OrcaSlicer/OrcaSlicer` file paths pinned at commit `40eab797c6a60a5949c0f92d00798da414c4b44a` (main, 2026-08-04); cite file + function, never line numbers.

- Question: `getNearestBeading` — inclusive or exclusive radius boundary, and how `connectJunctions` seeds junction chains from incident edges; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`; return: `LOCATIONS` (<=20); purpose: S1/S7 oracle confirmation
- Question: `LimitedBeadingStrategy::compute` odd-centre cap-boundary branch — where the single centre sentinel is placed and its location formula; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp`; return: `SUMMARY` (<=200 words); purpose: S6 canonical shape
- Question: `filterNoncentralRegions` gap-dissolve distance rule and its effects on inset-0 topology; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`; return: `SUMMARY`; purpose: S5 canonical claims
- Question: `run_post_process_scripts` and `WallToolPaths::generate` post-process call sequence — removal before simplification; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/slic3r/GUI/PostProcessor.cpp`, `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/WallToolPaths.cpp`; return: `LOCATIONS`; purpose: S9 order claim (header pin preserved)
- Question: `makeRib`/`distance_to_boundary` sentinel semantics for un-ribbed nodes; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp`, `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationJoint.hpp`; return: `SUMMARY`; purpose: S4 oracle
- Question: `getTransitionThickness`/`getNonlinearThicknesses` return shapes; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.hpp`; return: `LOCATIONS`; purpose: S2/S3 anchors

## Data and Contract Notes

- IR/manifest contracts: none — test-only additions; no IR schema, manifest, or WIT surface changes.
- WIT boundary: none; no guest WASM artifacts are touched (wasm-staleness considerations intentionally omitted).
- Determinism/scheduler constraints: the added tests must stay deterministic — no timing, no iteration-order dependence; all fixtures are hand-built or fixed polygons.

## Locked Assumptions and Invariants

- Canonical parity supremacy: existing exact pins are never removed or loosened; all sixteen pre-existing test-name anchors in the nine files must still resolve (AC-N2).
- The §5.1 PARITY row is authoritative over the older audit text; items are grounded to on-disk symbols only.
- Ambiguity commitments: item 6 = at-cap odd-centre cap boundary in `beading/limited.rs`; item 8 = local fallback contract in `arachne_pipeline.rs` (no Orca counterpart).
- The plan §7 `core` row is the only ledger mutation; `Packet Queue` and the census never change.

## Risks and Tradeoffs

- Measured-baseline pins (S5, S7's `xs[1]`): the exact numbers are captured at implementation time; the canonical *claims* (dissolve rule, interpolation formula) are fixed by the file docs — if a measured value contradicts the documented expectation, the implementation must stop and record the divergence in the ledger rather than silently pin it.
- F-family (S2/S3/S4) pass/fail state: grounded green against the landed fixes (`insert_node` faithful port, F5 fix in `make_node_vd`/`make_rib`); if any assertion proves red at implementation, the step records the measured state (self-captured baseline with divergence comment) and the ledger row notes it — the ACs must then still pass with the pinned baseline.
- Shared ledger row: later core-wave packets may touch the same §7 row; AC-10 accepts any `core` row carrying this packet's tokens.

## Context Cost Estimate

- Aggregate: `M` (never L)
- Largest step: `S` (step 4 — per-node oracle over the L-shape with builder-range reads)
- Highest-risk dispatch and required return format: Orca delegation for S5/S6/S7 canonical claims; `SUMMARY` (<=200 words) / `LOCATIONS` (<=20 entries) per the orca-delegation snippet.

## Open Questions

None. Both grounding ambiguities (odd-cap home, module-fallback reading) were resolved against on-disk evidence and are committed in `requirements.md` §In Scope. Tag implementer-resolvable questions `[FWD]`; tag activation blockers `[BLOCK]`. Scope/interface/verification questions keep the packet `draft`. Delegate answers requiring out-of-bounds reads. Write `None.` when absent.
