# Implementation Plan: core-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task ID `core/PARITY` (plan wave/item IDs replace `TASK-###` per the plan's packet-queue exemption).
- Each step is a single-file test addition (S5 adds two tests in its one file); validate with the narrowest falsifying filter after writing (`--exact`, `--features host-algos`, tee to `target/test-output.log`).
- Additive-only: never remove or loosen an existing assertion; never touch production code.
- Every new struct literal of a watched type uses `..` rest (FRU) or an `// exhaustive:` waiver — follow the touched file's own FRU style.
- Read the plan §5.1 PARITY row and §6 pattern block from disk at the start of work; never from memory.

## Steps

### Step 1: beding side-table exact-radius boundary test

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 1 — beding side table radius boundary)
- Objective: add `get_nearest_beding_includes_vertex_at_exact_radius_boundary` pinning the inclusive radius boundary and nearest-populated semantics of `get_nearest_beding`.
- Precondition: `crates/slicer-core/tests/arachne_beding_propagation_side_table.rs` builds green (baseline); `get_nearest_beding` (`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`) seeded by twin/next-prev with cumulative distances.
- Postcondition: new test passes; a hand-built chain proves radius `1000.0` includes a populated neighbour exactly `1000` units away, `999.5` excludes it, zero-radius self/None semantics hold, and the nearest (not merely any) populated entry is returned; no existing test modified.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_beding_propagation_side_table.rs` - lines 113-130 (fixture builders), 285-384 (radius contract locks) — direct
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` - lines 620-716 (`get_nearest_beding` BFS) — direct
  - `crates/slicer-core/tests/arachne_stitch_chain_junctions_t_to_fix.rs` - lines 133-226 (hand-built `STVertex`/`STHalfEdge` FRU style to copy)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_beding_propagation_side_table.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`, `docs/specs/test-quality-remediation-census.json`, other packet dirs
- Blast-radius discipline (mandatory when adding a new struct field or schema constant): N/A — no struct-field or schema additions; hand-built literals use FRU per the churn gate.
- Expected sub-agent dispatches:
  - Question: `getNearestBeading` radius boundary — inclusive or exclusive at exactly the radius? scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`; return: `SUMMARY` (<=200 words); purpose: confirm the inclusive-boundary assertion
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row, §6 invocation patterns (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test arachne_beding_propagation_side_table -- --exact get_nearest_beding_includes_vertex_at_exact_radius_boundary 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-1)
- Exit condition: AC-1's grep matches `ok. 1 passed; 0 failed`.

### Step 2: diagonal-source transition perpendicular-foot contract

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 2 — transitions perpendicular foot)
- Objective: add `apply_transitions_diagonal_source_mid_r_and_foot_sentinels` pinning the mid-node radius (`mid_r`) and foot-sentinel contract on a diagonal source edge where linear interpolation diverges from a true projection.
- Precondition: `apply_transitions` on a hand-built twin pair with distinct endpoint radii behaves per the faithful `insert_node` (`crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`).
- Postcondition: new test passes asserting mid node `distance_to_boundary == mid_r` (`750_000.0` units), two feet with `0.0`/`None`, cross-side foot equality, mid position on the edge line; divergence comment references `insert_node`'s non-retained-source note; existing `apply_transitions_new_vertex_position_is_perpendicular_foot` unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_construction_apply_transitions_mirror_fix.rs` - lines 93-195 (`make_f2_target_graph`), 195-428 (existing F2 locks) — direct
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` - lines 178-460 (`generate_transition_mids`, `insert_node`) — direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_construction_apply_transitions_mirror_fix.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**`, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: `getTransitionThickness` semantics and `generateTransitionMids` mid placement (radius = thickness / 2); scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.hpp`, `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`; return: `LOCATIONS` (<=20); purpose: confirm `mid_r` anchor
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.hpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test arachne_construction_apply_transitions_mirror_fix -- --exact apply_transitions_diagonal_source_mid_r_and_foot_sentinels 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-2)
- Exit condition: AC-2's grep matches `ok. 1 passed; 0 failed`.

### Step 3: rib-split exact topology and placeholder replacement

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 3 — rib-split geometry)
- Objective: add `apply_transitions_split_topology_exact_counts_and_cross_twin_patch` pinning exact split vertex/edge counts and replace the `let _ = max_deviation_x;` placeholder in `apply_transitions_split_position_coincides_on_both_sides` with a real position assertion.
- Precondition: `make_split_target_graph` fixture builds; `apply_transitions` produces the faithful insertNode shape on symmetric twin pairs.
- Postcondition: new test passes asserting 3 new vertices (1 mid: `bead_count Some(lower_bc)`, `distance_to_boundary == mid_r`; 2 feet: `0.0`/`None`), 6 new edges (4 `EXTRA_VD` ribs in two pairs, 2 `NORMAL` second fragments), intact cross-twin twin-links; placeholder line replaced by `assert!(max_deviation_x <= 1.0)` (1 unit = 100 nm tolerance) with the `let _ =` removed; tests at file lines 160/219/250/297 unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_construction_insert_node_rib_split.rs` - lines 100-160 (fixture), 160-384 (existing split locks) — direct
  - `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` - lines 240-460 (`insert_node` topology contract) — direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_construction_insert_node_rib_split.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**`, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: `insertNode`+`insertRib` new-edge/new-vertex counts and cross-twin patching; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp`; return: `SUMMARY` (<=200 words); purpose: confirm the exact 3-vertex/6-edge counts
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test arachne_construction_insert_node_rib_split -- --exact apply_transitions_split_topology_exact_counts_and_cross_twin_patch 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-3)
- Exit condition: AC-3's grep matches `ok. 1 passed; 0 failed` and `rg -n 'let _ = max_deviation_x' crates/slicer-core/tests/arachne_construction_insert_node_rib_split.rs` returns nothing.

### Step 4: F5 per-node distance oracle

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 4 — node distances)
- Objective: add `f5_invariant_node_distances_match_rib_geometry_and_boundary` asserting the F5-fix semantics per node on the L-shape graph.
- Precondition: `from_polygons` on `l_shape` builds with current F5-fix semantics (boundary `0.0`, ribbed spine = perpendicular foot, no plausible-wrong global-min values on this fixture, no `NaN`).
- Postcondition: new test passes asserting (a) every vertex with `distance_to_boundary == 0.0` lies on a polygon segment (independent point-to-segment distance `<= 1` unit); (b) no vertex has `NaN` or `< 0.0` on the L-shape; (c) for every `EXTRA_VD` rib pair, the spine endpoint's distance equals the geometric length between the two endpoint positions (independent oracle); existing locks at file lines 117/147/184 unchanged. If a measured divergence appears, record it as a self-captured baseline with a divergence comment naming the F5 section of the audit, and note it in the §7 ledger — never adjust the existing locks.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_construction_node_distance_perp_foot.rs` - lines 50-204 (fixture + existing locks) — direct
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` - lines 935-1060 (`make_rib`/`make_node_vd`), 1519-1592 (`nearest_boundary_distance`, `edge_radius_bounds`) — direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_construction_node_distance_perp_foot.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**`, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: `makeRib` boundary-node distance semantics and un-ribbed node sentinel handling; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp`, `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationJoint.hpp`; return: `SUMMARY` (<=200 words); purpose: confirm the per-node oracle classes
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test arachne_construction_node_distance_perp_foot -- --exact f5_invariant_node_distances_match_rib_geometry_and_boundary 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-4)
- Exit condition: AC-4's grep matches `ok. 1 passed; 0 failed`; any measured divergence is recorded in the ledger row.

### Step 5: dumbbell ring-topology pair

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 5 — dumbbell topology)
- Objective: add `dumbbell_wide_gap_not_dissolved_pins_exact_ring_topology` (existing fixture, gap `1.5` mm) and `dumbbell_narrow_gap_dissolves_to_single_closed_ring` (narrow-gap variant, separation `<= 0.4` mm), pinning exact deterministic inset-0 topology per the canonical dissolve rule.
- Precondition: `run_arachne_pipeline` on the dumbbell polygon is deterministic and currently produces the weak-presence result the existing test locks.
- Postcondition: both new tests pass; the wide-gap test pins the measured inset-0 line count, per-line `is_closed`, and junction counts with the documented claim that canonical `filterNoncentralRegions` does NOT dissolve a `> max_dist` (`0.4` mm) gap; the narrow-gap test asserts exactly one closed inset-0 ring (`is_closed == true`, junction count pinned) with the documented claim that a `<= 0.4` mm separation is dissolved; existing `dumbbell_single_central_region_inset0_ring_pair` unchanged. If the measured wide-gap topology contradicts "not dissolved" (e.g. already one ring), stop and record the divergence in the ledger rather than pinning it silently.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs` - lines 27-87 (fixture + existing test) — direct
  - `crates/slicer-core/tests/arachne_pipeline.rs` - lines 23-50, 104-238 (fixture style, existing pipeline tests) — direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_filter_noncentral_regions.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**`, other test files, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: `filterNoncentralRegions` max-distance rule and dissolve topology; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`; return: `SUMMARY` (<=200 words); purpose: confirm both canonical claims
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; for t in dumbbell_wide_gap_not_dissolved_pins_exact_ring_topology dumbbell_narrow_gap_dissolves_to_single_closed_ring; do cargo test -p slicer-core --features host-algos --test arachne_filter_noncentral_regions -- --exact "$t" 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || exit 1; done` - FACT pass/fail (AC-5)
- Exit condition: AC-5's loop greps both `ok. 1 passed; 0 failed`.

### Step 6: odd-cap at-cap single-centre-sentinel test

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 6 — odd-cap boundary; committed reading: the at-cap `bead_count == max_bead_count` branch of `LimitedBeadingStrategy::compute`, NOT is-odd line-marking)
- Objective: add `limited_inserts_single_centre_sentinel_at_cap_boundary` pinning the single centre-sentinel shape (odd total) on the at-cap boundary case.
- Precondition: `load_fixture()`/`build_strategy()` yield the fixture strategy; `compute(24000.0, 6)` takes the at-cap branch (`actual_count.is_multiple_of(2) && actual_count == max_bead_count`).
- Postcondition: new test passes with the exact analytic arrays — `bead_widths == [4000.0, 4000.0, 4000.0, 0.0, 4000.0, 4000.0, 4000.0]`, `toolpath_locations == [2000.0, 6000.0, 10000.0, 12000.0, 14000.0, 18000.0, 22000.0]`, `len == 7`, `total_thickness == 24000.0`, `left_over == 0.0`; `compute_and_strip` returns the six uniform `4000.0`-wide beads; fixture JSON untouched; existing `limited_inserts_sentinels_at_cap`/`limited_raw_compute_retains_sentinels` unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/beading/limited.rs` - lines 85-289 (fixture loader + existing locks) — direct
  - `crates/slicer-core/src/beading/limited.rs` - lines 100-225 (at-cap branch) — direct
  - `crates/slicer-core/tests/fixtures/beading/limited_cap_boundary.json` - 41 lines, direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/beading/limited.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/tests/fixtures/beading/limited_cap_boundary.json`, `crates/slicer-core/src/**`, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: `LimitedBeadingStrategy::compute` odd-centre cap-boundary branch — sentinel placement and location formula for the at-cap case; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp`; return: `LOCATIONS` (<=20); purpose: confirm the single-centre sentinel shape
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/BeadingStrategy/LimitedBeadingStrategy.cpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test beading_limited -- --exact limited_inserts_single_centre_sentinel_at_cap_boundary 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-6)
- Exit condition: AC-6's grep matches `ok. 1 passed; 0 failed`.

### Step 7: junction reachability exact positions

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 7 — junction reachability)
- Objective: add `chain_junctions_land_at_documented_interpolated_positions` pinning the exact interpolated chain positions on the F3 fixture.
- Precondition: `generate_toolpaths` on `make_f3_target_graph` with `SymmetricBeadingStrategy` produces the 3-junction chains the existing locks already pin.
- Postcondition: new test passes asserting `xs[0] == 1.25` mm and `xs[2] == 8.75` mm (values documented in the file's own traversal doc comment, `t = (1.5-3.0)/(1.0-3.0) = 0.75`), `xs[1]` pinned to the measured value strictly between them (merge of e0's `to` and rib_back's `from`), all three `y == 0.0`, `len == 3`; existing count lock (line 235) and traversal lock (line 309) unchanged. If the measured `xs[1]` contradicts the merge-point expectation (~`5` mm, v1's x), stop and record the divergence.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_stitch_chain_junctions_t_to_fix.rs` - lines 133-347 (fixture + locks + interpolation doc) — direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_stitch_chain_junctions_t_to_fix.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**`, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: `connectJunctions` incident-edge seeding so a chain reaches every endpoint vertex once; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp`; return: `LOCATIONS` (<=20); purpose: confirm reachability semantics
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test arachne_stitch_chain_junctions_t_to_fix -- --exact chain_junctions_land_at_documented_interpolated_positions 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-7)
- Exit condition: AC-7's grep matches `ok. 1 passed; 0 failed`.

### Step 8: module-fallback per-key contract

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 8 — module fallback; committed reading: local fallback contract mirrored in slicer-core, NOT a WASM fallback table; no Orca counterpart)
- Objective: add `arachne_params_absent_keys_fall_back_to_defaults_per_key` asserting, on an empty `ConfigView`, every wired key's exact `ArachneParams::default()` fallback.
- Precondition: the seven wired float keys and the `detect_thin_wall` bool in the module's `arachne_params_from_config` are identified (ranged read of `modules/core-modules/arachne-perimeters/src/lib.rs`); `ArachneParams::default()` values confirmed in `crates/slicer-core/src/arachne/pipeline.rs`.
- Postcondition: new test passes asserting per key: absent (`None`) in the empty config AND exact default fallback — `min_central_distance → 0.0`, `min_width → 0.4`, `min_bead_width → 0.4`, `wall_transition_length → 0.4`, `wall_transition_angle → 10.0_f64.to_radians()`, `initial_layer_min_bead_width → 0.34`, `outer_wall_offset → 0.0`, `detect_thin_wall → print_thin_walls == false`; existing `arachne_params_defaults_when_keys_absent` unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_pipeline.rs` - lines 353-472 (existing fallback test + context) — direct
  - `crates/slicer-core/src/arachne/pipeline.rs` - lines 76-231 (`ArachneParams` + `Default`) — direct
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - lines 280-340 (wired keys) — direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_pipeline.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (read-only), `crates/slicer-core/src/**`, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: none required — local behavior; a confirmation `LOCATIONS` read of `arachne_params_from_config` in the module is permitted only if the ranged read above is insufficient
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs: none — item 8 is local fallback behavior with NO Orca counterpart; do not invent parity.
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test arachne_pipeline -- --exact arachne_params_absent_keys_fall_back_to_defaults_per_key 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-8)
- Exit condition: AC-8's grep matches `ok. 1 passed; 0 failed`.

### Step 9: postprocess order-divergence fixture

- Task IDs: `core/PARITY` (plan §5.1 PARITY, item 9 — postprocess order divergence fixture)
- Objective: add `canonical_remove_small_first_keeps_line_simplify_first_drops_it` proving the canonical order (`remove_small` before `simplify`) is observable on a line the old order would drop.
- Precondition: `run_arachne_pipeline`'s canonical post-process order (removal first, per `crates/slicer-core/src/arachne/pipeline.rs`) is in place; the existing degenerate fixture confirms the old-order branch is untested today.
- Postcondition: new test passes asserting, on an odd open line (min junction width `0.4`, threshold `0.2` mm) with an out-and-back micro-segment shape (polyline length `>= 0.2` mm, chord `< 0.2` mm): canonical branch `remove_small_lines(line, 0.5, 0.4, false, false)` → `len == 1`; old-order branch `simplify_toolpaths(line, 0.0025, 0.000025, 2e-6)` then `remove_small_lines` → `is_empty()`; coordinates are adjusted at implementation only to satisfy the two properties (never to fake a pass); existing `pipeline_smoke_after_order_swap` and `remove_small_before_simplify_short_odd_line_removed` unchanged; file-header pin preserved.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/arachne_postprocess_order.rs` - lines 1-123 (header pin, smoke, degenerate order test) — direct
  - `crates/slicer-core/src/arachne/remove_small.rs` - lines 45-112 (threshold + length metric) — direct
  - `crates/slicer-core/src/arachne/pipeline.rs` - lines 369-380 (canonical order invocation) — direct
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/arachne_postprocess_order.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**`, `docs/specs/test-quality-remediation-census.json`
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: `run_post_process_scripts` sequential execution order and `WallToolPaths::generate`'s post-process call sequence; scope: `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/slic3r/GUI/PostProcessor.cpp`, `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/WallToolPaths.cpp`; return: `LOCATIONS` (<=20); purpose: confirm the order claim (the local header pin stays authoritative)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PARITY row (ranged)
- OrcaSlicer refs:
  - `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/slic3r/GUI/PostProcessor.cpp`, `https://github.com/OrcaSlicer/OrcaSlicer/blob/40eab797c6a60a5949c0f92d00798da414c4b44a/src/libslic3r/Arachne/WallToolPaths.cpp` - delegate; never load
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test arachne_postprocess_order -- --exact canonical_remove_small_first_keeps_line_simplify_first_drops_it 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail (AC-9)
- Exit condition: AC-9's grep matches `ok. 1 passed; 0 failed`.

### Step 10: plan §7 ledger row update

- Task IDs: `core/PARITY` (ledger step per the approved M downstream context)
- Objective: record core-parity's additive progress in the §7 `core` ledger row only.
- Precondition: Steps 1-9 complete; the ledger table in `docs/specs/test-quality-remediation-plan.md` lines 291-311 is the current state (one `core` row, state `open`).
- Postcondition: the existing `core` row (never a second row) reads state `partial (core-parity)`; Retired/changed symbols stays `—`; Surviving/new coverage appends the ten new test names and oracle tokens (`exact_radius_boundary`, `mid_r`, `cross_twin`, `single_closed_ring`, `centre_sentinel`, `1.25mm`, `8.75mm`, `0.34`, `unwrap_or`, `remove_small`); Validation appends the representative command `cargo test -p slicer-core --features host-algos --test arachne_postprocess_order` plus the four gate tokens (`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`); Remaining gap no longer contains `core-parity`; `Packet Queue` and every other plan section untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - lines 291-311 (ledger) only
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md`
- Files explicitly out of bounds:
  - `docs/specs/test-quality-remediation-census.json`, `docs/specs/test-quality-remediation-plan.md` lines outside 291-311, all other plan sections
- Blast-radius discipline: N/A — no struct-field or schema additions.
- Expected sub-agent dispatches:
  - Question: none — direct ranged read and single-row edit; if the row already carries other packets' tokens (joint ownership), append, never overwrite
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §7 (ranged)
- OrcaSlicer refs: none.
- Verification:
  - AC-10's python ledger predicate from `packet.spec.md` - FACT pass/fail
  - `rg -n '^## 7\. Ledger\b|^\| core \|' docs/specs/test-quality-remediation-plan.md | tail -5` - SNIPPETS (<=10 lines) sanity
- Exit condition: AC-10 prints `core ledger predicate: PASS`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | hand-built chain + BFS range read |
| Step 2 | S | diagonal fixture + propagation range read |
| Step 3 | S | topology counts + propagation range read |
| Step 4 | S | per-node oracle + builder range reads |
| Step 5 | S | measured baseline + contrast fixture |
| Step 6 | S | inline at-cap case, analytic arrays |
| Step 7 | S | exact positions from file's own docs |
| Step 8 | S | key list + defaults ranged reads |
| Step 9 | S | divergence fixture + threshold metric read |
| Step 10 | S | single-row ledger append |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate: M. Largest step: S.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command (AC-1..AC-10, AC-N1, AC-N2) returns PASS.
- Update `docs/specs/test-quality-remediation-plan.md` §7 `core` row (Step 10) through the ranged read, never a full backlog read.
- Reconcile reopened/superseded status transitions: none — this packet reopens no prior packet.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command (AC-1..AC-10, AC-N1, AC-N2; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask check-test-quality --report`), each tee'd to its dedicated log under `target/`.
- Record remaining packet-local risk: measured-baseline values (S5, S7) and any F-family divergence recorded in the ledger.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
