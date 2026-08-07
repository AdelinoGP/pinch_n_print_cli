# Implementation Plan: 210a-support-planner-coord-t

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **`smooth_branches` is rewritten exactly once, in Step 3.** No other step in this packet, and no step in `210b`, may modify it except to fix a lint.
- This packet runs as a **single** swarm session. The mandatory mid-packet checkpoint that the merged `210` carried is dissolved by the split; `210b` is the hand-off.

## Steps

### Step 1: Author the RED integer-contract tests

- Task IDs: `TASK-326`
- Objective: write the six tests that only an integer node representation can pass — the rewritten `multi_neighbour_mst_tdd.rs` cases (AC-3) and the five new in-file `#[cfg(test)]` cases `smooth_branches_uses_truncating_integer_average` (AC-4), `point_in_polygon_units_is_exact_on_contour_vertex` (AC-N1), `point_in_any_expoly_excludes_points_inside_holes` (AC-N8), `node_position_roundtrips_beyond_f32_integer_ceiling` (AC-N2) and `mm_unit_round_trip_envelope_is_5_120_003_units` (AC-N7) — and record their RED state.
- AC-N8's case is written against the **new** `point_in_any_expoly(polygons: &[ExPolygon], p: Point2)` signature, so it fails to compile on arrival exactly like AC-N1's. Build one `ExPolygon` whose `contour` is a square and whose `holes` holds one smaller square strictly inside it; assert `false` for a `Point2` inside the hole and `true` for a `Point2` inside the contour but outside the hole. It exists because the retype description ("drop the `* SCALING_FACTOR as f32` scalings and the `p.x as f32` maps") does not by itself force the hole term to survive, and AC-7's clauses are all satisfied by a contour-only body.
- AC-4's fixture, stated exactly because its discriminating power depends on the numbers: a 5-entry column whose chain-order X positions are `50_000`, `50_001`, `50_003`, `50_006`, `50_010` internal units (`y = 0`, equal widths, distinct `z` per layer), built through `units_to_mm` so the stored millimetres round-trip exactly (all five are far inside the measured 5 120 003-unit envelope). Smooth with `iterations = 1` and assert the **first interior** point (chain index 1, computed from the two unmodified originals at indices 0 and 2) satisfies `assert_eq!(pt.x, units_to_mm(50_001))` — the truncating integer average of `(50_000 + 50_001 + 50_003) / 3`. The pre-migration `f32` path yields `5.000133…` mm, ~0.33 unit ≈ 70 `f32` ULPs away at this magnitude. Do **not** use a straight, evenly-spaced chain: on one, the three-point average equals `cur` exactly in `f32` too, and the test can never go RED.
- AC-N7's case is the odd one out: it exercises only `mm_to_units` / `units_to_mm` and therefore **passes on the pre-migration tree**. That is intended — it is a fact-pin against a number this slice has had wrong three times, not a migration test. Record it as GREEN-on-arrival and do not "strengthen" it into a RED case by coupling it to the node type.
- AC-N2's case must construct and read `PlannedSupportNode`'s `i64` field directly; it must **not** route through `units_to_mm` / `mm_to_units`. Its 17 000 000-unit value is deliberately outside the mm round-trip envelope, and passing it through the boundary would turn it into a (failing) test of AC-N7's subject.
- Precondition: working tree clean; `cargo test -p support-planner` green on the pre-migration code.
- Postcondition: `cargo check -p support-planner --all-targets` FAILS to compile `multi_neighbour_mst_tdd.rs` (unresolved `Point2` argument types against the still-`f32` `aggregate_neighbour_targets`) and `point_in_polygon_units_is_exact_on_contour_vertex` / `point_in_any_expoly_excludes_points_inside_holes` / `node_position_roundtrips_beyond_f32_integer_ceiling` (unresolved `point_in_polygon_units`, the two-`f32` `point_in_any_expoly` arity, `i64` node fields). `smooth_branches_uses_truncating_integer_average` compiles and **fails at its assertion** — that runtime failure, not a compile error, is its RED state and must be recorded verbatim (the observed `f32` value vs the expected `units_to_mm(50_001)`).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` — whole file; it is short
  - `modules/core-modules/support-planner/src/lib.rs` — the `#[cfg(test)] mod tests` block only, plus the `aggregate_neighbour_targets` and `point_in_polygon` doc comments
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs`
  - `modules/core-modules/support-planner/src/lib.rs` (test block only)
- Files explicitly out of bounds:
  - every other `tests/*.rs` in this module
  - `crates/**`, `docs/**`, `resources/golden/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: not applicable — this step adds no struct field and bumps no constant. The struct-literal work is Step 2's.
- Expected sub-agent dispatches:
  - Question: does `cargo check -p support-planner --all-targets` fail, on which symbols, and separately does `smooth_branches_uses_truncating_integer_average` compile-and-fail rather than fail to compile?; scope: `modules/core-modules/support-planner`; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — §"SDK Helpers" and §"Point2 Wrapper" ranges, for the exact helper names used in the new assertions
- OrcaSlicer refs: none for this step
- Verification:
  - `cargo check -p support-planner --all-targets 2>&1 | rg -c '^error'` — FACT: non-zero error count is the expected RED
  - `cargo test -p support-planner --lib mm_unit_round_trip_envelope_is_5_120_003_units 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT: AC-N7 is green on arrival
- Exit condition: all six tests exist; four fail to compile, AC-4's case fails at runtime with its observed-vs-expected values recorded, and AC-N7's case passes. If `cargo check` unexpectedly SUCCEEDS, or if AC-4's case passes, the tests were written too weakly — strengthen them; do not proceed. If AC-N7's case *fails*, the measured envelope in `design.md` is wrong for this toolchain — stop and report rather than editing the expected values.

### Step 2: Retype `PlannedSupportNode` and every node-geometry consumer

- Task IDs: `TASK-326`
- Objective: land the atomic retype — `PlannedSupportNode { x: i64, y: i64, … }`, `prim_mst → Vec<(usize, usize, i64)>`, `euclidean_distance → i64` via `.isqrt()`, an **explicitly annotated** `let mut neighbours_of: Vec<Vec<(usize, i64)>>`, `aggregate_neighbour_targets(&[Point2], &[i64]) -> Option<Point2>`, net-new `point_in_polygon_units`, `point_in_any_expoly(&[ExPolygon], Point2)`, `clamp_to_avoidance` / `closest_point_on_polygon` / `closest_point_on_segment` on `Point2`/`i64`, `max_move_xy: i64` via `mm_to_units`, merge comparison against `mm_to_units(merge_distance_mm)`, `push_interface_scan_lines` on `Point2`/`i64`, `first_point_xyw → Option<(Point2, f32)>`, and `units_to_mm` at every `Point3WithWidth` construction. **`smooth_branches`' body is not touched here** beyond whatever `first_point_xyw`'s new return type forces at its call sites — it is Step 3's.
- Precondition: Step 1's RED state recorded.
- Postcondition: `cargo check -p support-planner --all-targets` clean; `cargo test -p support-planner --lib`, `--test multi_neighbour_mst_tdd`, `--test to_buildplate_tdd`, `--test diagnostics_tdd` all pass. `smooth_nodes_tdd` passes. `AC-4`'s case may still be RED — its subject moves in Step 3.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` — four ranged reads: the const/struct header, `plan_for_object`, the free-function helper block, the `#[cfg(test)] mod tests` block. Never open in full (2 058 lines).
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` — `unreachable_buildplate_node_pruned` and the `multi_overhang_grid` / `make_layer_plan` helpers only, to confirm the collision fixture's expectations are membership-based, not coordinate-based
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**`, `crates/slicer-core/**` — editing any of these stales all 34 guests
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — Step 4 owns any fallout there
  - `resources/golden/**` — Step 4 owns regeneration
  - the code-1003 block in `run_support_geometry`, `support-planner.toml`, `tests/diagnostics_tdd.rs` — `210b` owns all three; leave code 1003 firing
  - every other `modules/core-modules/*`
- Blast-radius discipline (mandatory — this step retypes a struct's fields):
  - `PlannedSupportNode` is **private** to `modules/core-modules/support-planner/src/lib.rs` (declared `struct PlannedSupportNode`, no `pub`), so its struct-literal blast radius is confined to that one file. The complete, grep-verified inventory is **six literal sites**: four in `plan_for_object` (the overhang-facet contact push, the paint-enforcer contact push, the no-MST-neighbour propagate-unchanged branch, and the moved-node branch) and two inside the in-file `#[cfg(test)] mod tests` case `prim_mst_on_two_nodes_returns_one_edge`. Zero sites under `tests/`. All six are in this step's single edited file — that is why the file cap is 1, not 3.
  - Test-assertion fallout in the same step: `prim_mst_on_two_nodes_returns_one_edge` currently asserts `(edges[0].2 - 5.0).abs() < 1e-4` (an `f32` millimetre distance). It must become the exact `assert_eq!(edges[0].2, 50_000)` — 3 mm/4 mm legs give a 5 mm hypotenuse = 50 000 units, and `(30_000² + 40_000²).isqrt()` is exactly `50_000`. Do not weaken it to a tolerance.
  - `aggregate_neighbour_targets` is `pub` with exactly one external consumer, `tests/multi_neighbour_mst_tdd.rs`, rewritten in Step 1. `point_in_polygon`, `tapered_radius`, `smooth_branches`, `group_branches_into_columns` **and `pub struct SupportPlanner` itself** are the module's other `pub` items; none of their signatures change, so `orca_parity_tdd.rs` and `smooth_nodes_tdd.rs` need no signature-driven edit.
  - **`pub struct SupportPlanner`'s inventory, completed for the record (no fallout expected in this packet).** It is the one `pub` item in this module with external consumers — five files reference it: `modules/core-modules/support-planner/wit-guest/src/lib.rs`, `tests/to_buildplate_tdd.rs`, `tests/slicer_module_binding_tdd.rs`, `tests/diagnostics_tdd.rs` and `tests/orca_parity_tdd.rs`. **This packet adds no field to it and changes no field's type**, so its struct-literal blast radius is empty here and none of those five files needs a signature-driven edit — but the inventory is recorded because packet `210b` *does* add a field to it (`support_interface_bottom_layers: i32`), and an implementer who reads only the `PlannedSupportNode` inventory above could mistake "the struct with the blast radius" for the wrong struct. If any edit in this step touches `SupportPlanner`'s fields, stop: that is `210b`'s surface.
- Expected sub-agent dispatches:
  - Question: what exact type is `SupportNode::position`, and does the class carry any floating-point position member?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp`; return: `FACT` (≤5 lines)
  - Question: does `cargo test -p support-planner` pass, and which test binaries report failures?; scope: `modules/core-modules/support-planner`; return: `FACT` pass/fail + failing test names
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — §"Conversion & Determinism (Normative)", §"Conversion When Porting OrcaSlicer Code", §"Constant Conversion Table" ranges
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` — delegate; never load
- Verification:
  - `cargo check -p support-planner --all-targets` — FACT pass/fail
  - `cargo test -p support-planner --lib prim_mst_on_two_nodes_returns_one_edge 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail
  - `cargo test -p support-planner --test multi_neighbour_mst_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-3 goes GREEN here)
  - `cargo test -p support-planner --test to_buildplate_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail
  - `! rg -q 'SCALING_FACTOR as f32' modules/core-modules/support-planner/src/lib.rs` — FACT: the lossy round trip is gone
- Exit condition: AC-1, AC-2, AC-3, AC-6, AC-7, AC-N1, AC-N2 and AC-N8 all PASS, and no `f32` field remains on `PlannedSupportNode` or in any retyped helper signature. **AC-N8 specifically:** `point_in_any_expoly`'s hole term survived the retype — the migrated body must be `point_in_polygon_units(&ex.contour.points, p) && !ex.holes.iter().any(|h| point_in_polygon_units(&h.points, p))`. Satisfying AC-7 with a contour-only body is a silent correctness regression that lets a branch sit inside a model hole. **If a case in `orca_parity_tdd.rs` or either golden suite flips, do not touch it here — record the failure and hand it to Step 4, which owns those files.** Never loosen an assertion or a tolerance anywhere.

### Step 3: Rewrite `smooth_branches` once — extract `split_column_into_chains` and move to integer averaging

- Task IDs: `TASK-326`
- Objective: perform **both** edits to `smooth_branches` in a single rewrite, because they are edits to the same twenty lines and the split of the merged packet depends on this half owning them:
  1. **Extract** the sub-chain gap walk into a private `split_column_into_chains(entries: &[SupportPlanEntry], column: &[usize]) -> Vec<(usize, usize)>` returning half-open ranges into `column`, and rewrite `smooth_branches` to consume it. `210b`'s `densify_bottom_interface` becomes its second caller — in `210b`, not here.
  2. **Retype** the averaging: one scratch `Vec<Point2>` per sub-chain, `iterations` passes of `(prev + cur + next) / 3` in `i64` (truncating, matching canonical `TreeSupport::smooth_nodes`), one `units_to_mm` write-back per point after the final pass. Replace the fn-local `const CHAIN_BREAK_THRESHOLD_MM: f32 = 5.0` with a fn-local `const CHAIN_BREAK_THRESHOLD_UNITS: i64 = 50_000` **inside `split_column_into_chains`**, comparing squared units (`dx*dx + dy*dy > CHAIN_BREAK_THRESHOLD_UNITS.pow(2)`) instead of taking a square root.
- Behaviour-preservation contract for the extraction — all three must hold or `smooth_nodes_tdd` will not stay green:
  - `split_column_into_chains` returns **every** sub-chain range, including ranges shorter than 3, and is callable on a column of any length.
  - The `e - s < 3` skip and the `column.len() < 3` early-continue stay in `smooth_branches`. They do **not** move into the helper: `210b` needs floor bands on short chains.
  - The current walk's `None ⇒ break` on a malformed entry is preserved verbatim (it terminates the split loop, leaving remaining indices inside the final chain). `continue` would change chain boundaries.
- Widths keep their `f32` averaging and `MAX_BRANCH_RADIUS_MM` clamp; `z`, `role`, `speed_factor`, layer index, ids and counts are preserved as today.
- Precondition: Step 2 complete and green; the emission boundary is `units_to_mm`.
- Postcondition: `cargo test -p support-planner --test smooth_nodes_tdd` passes with its four assertions **byte-unchanged**; `smooth_branches_uses_truncating_integer_average` goes GREEN.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` — the `group_branches_into_columns` / `first_point_xyw` / `smooth_branches` block only
  - `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` — the four `#[test]` fns `smoothing_reduces_curvature`, `endpoints_held_fixed`, `columns_below_three_points_unchanged`, `empty_entries_no_panic` and the helper fns `pt` / `entry` / `build_column` / `read_column` / `max_turn_angle`. The tests sit *after* the helpers in the file; locate by name, not by line.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` — the guard; if it needs editing, the rewrite went wrong
  - `resources/golden/**`, `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — Step 4
  - `crates/**`, `docs/**`, `OrcaSlicerDocumented/**` (delegate)
- Blast-radius discipline: not applicable — no struct field is added and no public constant's value changes. `CHAIN_BREAK_THRESHOLD_MM` is a fn-local `const` inside `smooth_branches` with no other *reader* (grep-verified); replacing it and relocating it into `split_column_into_chains` has no compile-time fallout outside these two functions.
  - **Known stale reference, flagged not fixed (INFO).** The name `CHAIN_BREAK_THRESHOLD_MM` also appears in a **comment** in `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — the line reading "Mirror the smoother's CHAIN_BREAK_THRESHOLD_MM = 5.0 in …", inside the chain-grouping helper that `branch_curvature_below_threshold` uses. It is prose, not code, so nothing breaks and AC-5's negative clause (`! rg -q 'CHAIN_BREAK_THRESHOLD_MM' modules/core-modules/support-planner/src/lib.rs`) is **file-scoped to `src/lib.rs`** and still passes. `crates/**` is out of bounds for this step and that file is the wedge oracle, so **do not edit it here.** Record the staleness in the Step 5b report so a later packet that legitimately owns `crates/slicer-runtime/tests/` can correct the comment to `CHAIN_BREAK_THRESHOLD_UNITS = 50_000`. The mirrored *value* is unchanged (5.0 mm = 50 000 units), so the oracle's behaviour is unaffected; only the name in the comment rots.
- Expected sub-agent dispatches:
  - Question: in `TreeSupport::smooth_nodes`, is the three-point position average integer `Point` arithmetic with a truncating `/3`, is the radius average `double`, and what is `max_move` derived from?; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY` (≤200 words, no code)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — §"Conversion & Determinism (Normative)" range, for the rounding rule at the write-back boundary
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — delegate; never load
- Verification:
  - `cargo test -p support-planner --test smooth_nodes_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail
  - `cargo test -p support-planner --lib smooth_branches_uses_truncating_integer_average 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-4)
  - `rg -q 'const CHAIN_BREAK_THRESHOLD_UNITS: i64 = 50_000;' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'CHAIN_BREAK_THRESHOLD_MM' modules/core-modules/support-planner/src/lib.rs` — FACT: AC-5
  - `[ "$(rg -c '^\s*fn split_column_into_chains' modules/core-modules/support-planner/src/lib.rs)" = "1" ] && [ "$(rg -c 'split_column_into_chains\(' modules/core-modules/support-planner/src/lib.rs)" -ge 2 ]` — FACT: AC-19 (exactly one declaration, at least two occurrences)
- Exit condition: AC-4, AC-5 and AC-19 PASS and `smooth_nodes_tdd` is green with unmodified assertions. If the extraction changes any smoothing output, it was not behaviour-preserving — **revert and redo the whole step**, never adjust the guard. If the canonical dispatch reveals the `pts` / `pts1` idempotence quirk, record it per `design.md` `[FWD-1]` and do not change PnP's iteration semantics here.

### Step 4: Reconcile the frozen goldens and the parity fixture

- Task IDs: `TASK-326`
- Objective: run both frozen-golden suites and `orca_parity_tdd` against the migrated geometry and resolve any drift now, attributed to the retype. This step is the sole owner of `resources/golden/**` and of `overhang_plate_fixture`.
- Resolution ladder, in order — stop at the first that applies:
  1. Both suites green against the committed goldens ⇒ done; record "no drift".
  2. A case in `orca_parity_tdd.rs` other than the golden comparison flips (candidates: `radius_tapers_with_distance_to_top` at `< 1e-6` / `< 1e-4`, `wall_count_scales_max_move_distance` at `< 1e-6`, `avoidance_keeps_branches_inside_support_outline`, `node_dropped_when_avoidance_rejects_all_moves`) because a node sat within sub-unit distance of a fixture boundary ⇒ **widen `overhang_plate_fixture`'s geometric margin** so the case is no longer marginal. The assertion and its tolerance constant stay byte-identical.
  3. A golden comparison exceeds its bound ⇒ establish, by naming the mechanism (`isqrt` truncation in an MST weight, exact-integer `point_in_polygon_units` on a boundary node, the integer `max_move_xy` cap), that the new output is the canonical-correct one, then regenerate that pair and commit it. **The justification and the regenerated file's basename must be carried into Step 5b's `DEV-128` closure text**, because AC-8 clause (c) greps `docs/DEVIATION_LOG.md` for that basename and fails the packet otherwise. Clause (c) inspects the **working tree** (a merge-base diff taken *without* `..HEAD`, unioned with `git status --porcelain -- resources/golden/`), so it fires the moment a golden is regenerated here — before the commit — not only after. Do not expect to be able to run the packet gates with a regenerated-but-unrecorded golden sitting in the tree.
- **Prohibited:** widening `let tolerance_mm = 0.5_f32;` or `let tolerance_fraction = 0.10_f32;` in `orca_parity_tdd.rs`, or either `0.10, 0.5` argument pair in the wedge comparator; editing `detects_intentional_branch_count_drift`; regenerating a golden without a named mechanism.
- Precondition: Steps 2 and 3 complete and green.
- Postcondition: `cargo test -p support-planner --test orca_parity_tdd` and `cargo test -p slicer-runtime --test integration support_golden_regression_wedge` both green, with tolerance constants unchanged. Any regeneration is committed together with its justification.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — the six fns named in `design.md` §Read-Only Context plus `overhang_plate_fixture`, located by name
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` — `current_wedge_output_stays_within_self_capture_tolerance` and the `SUPPORT_WEDGE_REGEN_GOLDEN` gate only
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` (fixture margin only — never an assertion or a tolerance)
  - `resources/golden/benchy_tree_support_orca_endpoints.txt` + `..._branch_count.txt` (via `SUPPORT_PLANNER_REGEN_GOLDEN=1`, never hand-edited)
  - `resources/golden/support_regression_wedge_endpoints.txt` + `..._branch_count.txt` (via `SUPPORT_WEDGE_REGEN_GOLDEN=1`, never hand-edited)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` — the comparator is the oracle; its tolerances and its self-test are frozen
  - `modules/core-modules/support-planner/src/lib.rs` — the migration is settled; if a golden failure implies a code bug, reopen Step 2 or Step 3 rather than editing here
- Blast-radius discipline: not applicable — no struct field, no constant. The *fixture* blast radius is stated instead: `overhang_plate_fixture` is shared by `avoidance_keeps_branches_inside_support_outline`, `benchy_orca_parity_within_tolerance` and `node_dropped_when_avoidance_rejects_all_moves`; widening its margin must be re-verified against all three.
- Expected sub-agent dispatches:
  - Question: do `benchy_orca_parity_within_tolerance` and `current_wedge_output_stays_within_self_capture_tolerance` pass, and if not what Hausdorff distance and branch counts are reported?; scope: the two suites; return: `FACT` (≤5 lines)
  - Question: does `cargo test -p support-planner --test orca_parity_tdd` pass, and which cases fail with what observed-vs-expected values?; scope: `modules/core-modules/support-planner`; return: `FACT` pass/fail + failing case names + ≤10 lines of assertion detail
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §Test Discipline — canonical-correct output wins; fixtures may be re-recorded to match, assertions may not be weakened. Already summarised here; no read needed.
- OrcaSlicer refs: none for this step
- Verification:
  - `cargo test -p support-planner --test orca_parity_tdd 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail
  - `cargo test -p slicer-runtime --test integration support_golden_regression_wedge 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail
  - `rg -q 'let tolerance_mm = 0\.5_f32;' modules/core-modules/support-planner/tests/orca_parity_tdd.rs && rg -q 'let tolerance_fraction = 0\.10_f32;' modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — FACT: the parity tolerances are byte-identical
  - `[ "$(rg -c '^\s+0\.10,$' crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs)" = "2" ] && [ "$(rg -c '^\s+0\.5,$' crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs)" = "2" ]` — FACT: the wedge tolerances are byte-identical
- Exit condition: AC-8 clauses (a) and (b) PASS with both suites green and both tolerance constants byte-identical, and every regeneration performed is recorded with its named mechanism *and its file basename*, staged for Step 5b so clause (c) can pass. If a golden moved and no mechanism can be named, that is an unexplained behaviour change — stop and report; do not regenerate.

### Step 5: Rebuild the guest and clear the whole-packet gates

- Task IDs: `TASK-326`
- Objective: prove the migration end-to-end through the real dispatch path — rebuild `support-planner.wasm`, run the wedge invariant suite (AC-17), run the whole-crate sweep (AC-18), and clear the workspace gates.
- Precondition: Steps 2, 3 and 4 complete; `cargo test -p support-planner` green.
- Postcondition: `cargo xtask build-guests --check` exits 0 with no `STALE:`; the four named tests in `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` pass; at least 7 `support-planner` test binaries report `ok` with zero failures; `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- Files allowed to read, with ranges when over 300 lines:
  - none — this is a verification step. Read only the delegated FACT returns.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs` (only to fix a lint or a failure surfaced by this step's gates)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/**` — the invariant tests are the oracle; fixing the oracle to make the migration pass is prohibited
  - `resources/golden/**` — Step 4 owned it and is closed
  - `target/**`, `modules/core-modules/support-planner/support-planner.wasm` (regenerated, never hand-edited)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests --check` exit 0 and report no `STALE:` line, and after a rebuild does it come back clean?; scope: workspace; return: `FACT` (≤5 lines, include the exit code)
  - Question: do `branch_endpoints_are_outside_support_collision_outlines`, `branch_points_match_entry_layer_z`, `branch_radii_stay_within_current_bounds` and `branch_curvature_below_threshold` pass?; scope: `cargo test -p slicer-runtime --test integration support_invariants_wedge`; return: `FACT` pass/fail + ≤20 lines of the first failure
  - Question: how many `test result: ok. N passed; 0 failed` lines does `cargo test -p support-planner` print, and is there any `test result: FAILED` line?; scope: `modules/core-modules/support-planner`; return: `FACT` (≤5 lines)
  - Question: does `cargo clippy --workspace --all-targets -- -D warnings` pass, and which lints fire in `support-planner`?; scope: workspace; return: `FACT` pass/fail + lint names
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Guest WASM Staleness" — already summarised in `design.md`; no read needed
- OrcaSlicer refs: none for this step
- Verification:
  - `cargo xtask build-guests --check` — FACT: exit 0, no `STALE:` (AC-N6)
  - `cargo test -p slicer-runtime --test integration support_invariants_wedge 2>&1 | rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed'` — FACT pass/fail (AC-17)
  - `out=$(cargo test -p support-planner 2>&1); ! printf '%s' "$out" | rg -q '^test result: FAILED' && [ "$(printf '%s' "$out" | rg -c '^test result: ok\. [1-9][0-9]* passed; 0 failed')" -ge 7 ]` — FACT pass/fail (AC-18)
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: AC-17, AC-18 and AC-N6 PASS. A wedge failure is this packet's bug until `build-guests --check` has been shown clean — the deflections listed in `CLAUDE.md` §"Guest WASM Staleness" are prohibited.

### Step 5b: Close the ledger

- Task IDs: `TASK-326`
- Objective: land this packet's two doc edits plus the module header, with re-derived rather than quoted values.
- Precondition: Step 5 green.
- Ledger obligations, in the order they must be performed:
  1. Delegate a `FACT` for `DEV-128`'s current status cell and for whether `TASK-326` is still absent from `docs/07_implementation_status.md`. Do not trust any value written in this packet.
  2. `DEV-128` → `Closed`, citing this packet and the invariant-2 evidence, and — if Step 4 regenerated either golden pair — naming the mechanism **and the regenerated file basenames** (`benchy_tree_support_orca_endpoints.txt`, `support_regression_wedge_branch_count.txt`, …). AC-8 clause (c) greps for whichever basenames actually changed.
  3. `docs/07_implementation_status.md`: register `TASK-326` under §"Workstream 3 — Benchy parity and missing OrcaSlicer behavior", noting that the bottom-interface bands are `TASK-327` in packet `210b` and are **not** part of this row.
  4. `modules/core-modules/support-planner/src/lib.rs` `//!` header: state that node positions are `i64` internal units and name the two conversion boundaries.
  5. Report (do not edit) the known-stale cross-file comment flagged in Step 3: `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` still says "Mirror the smoother's CHAIN_BREAK_THRESHOLD_MM = 5.0". `crates/**` is out of bounds for this packet and the mirrored value (5.0 mm = 50 000 units) is unchanged, so nothing breaks; the name in the comment is simply stale and should be corrected by a later packet that owns that directory.
- **Not edited here:** `docs/15_config_keys_reference.md`, `docs/adr/0010-typed-diagnostic-channel.md` (both `210b`'s, including ADR-0010 §Context's "All three shipped via packet 118 with the codes `1001` / `1002` / `1003`" sentence), and `docs/specs/deviation-remediation-206-212-plan.md` (already reconciled by the split decision).
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — the `DEV-128` row only, located by grep
  - `docs/07_implementation_status.md` — the Workstream 3 heading only, located by grep
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `docs/specs/deviation-remediation-206-212-plan.md` — already reconciled
  - `docs/15_config_keys_reference.md`, `docs/adr/**` — `210b`'s
  - `.ralph/specs/211-support-interface-bottom-layers/`, `.ralph/specs/210b-support-interface-bottom-layers/` — never edited from this packet
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: what is the current status cell of `DEV-128`, and is `TASK-326` still absent from `docs/07_implementation_status.md`?; scope: both files; return: `FACT` (≤5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` (`DEV-128`), `docs/07_implementation_status.md` (Workstream 3) — both ranged or delegated
- OrcaSlicer refs: none for this step
- Verification:
  - `rg -q 'DEV-128.*Closed' docs/DEVIATION_LOG.md` — FACT pass/fail
  - `rg -q 'TASK-326' docs/07_implementation_status.md` — FACT pass/fail
  - `rg -q 'internal units' modules/core-modules/support-planner/src/lib.rs` — FACT pass/fail
- Exit condition: every Doc Impact grep in `packet.spec.md` returns PASS, and AC-8 clause (c) passes with the deviation log naming any regenerated golden. If `TASK-326` turns out to be taken by a parallel packet, take the next free ID, update `packet.spec.md`'s frontmatter and `task-map.md` in the same edit, and report the change — do not double-book.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two short test files; RED is three compile errors plus one recorded assertion failure, and one deliberately-green fact-pin (AC-N7) |
| Step 2 | M | The atomic retype; six struct literals + nine helper signatures in one 2 058-line file, read in four ranges |
| Step 3 | M | The seam. One function rewritten once: extraction + integer averaging together |
| Step 4 | S | Golden and parity-fixture reconciliation; owns `resources/golden/**` |
| Step 5 | S | Verification only, all output delegated as FACT |
| Step 5b | S | Two ledger files plus the module header |

Aggregate `M`. No individual step is `L`. The merged packet's `L` rating and its mandatory mid-packet checkpoint are dissolved by the split into `210a` / `210b`.

## Packet Completion Gate

- All six steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read; the update registers `TASK-326` and points at `210b` for `TASK-327`.
- `DEV-128` is `Closed`. `DEV-129` is untouched and still `Open` — it belongs to `210b`.
- `.ralph/specs/211-support-interface-bottom-layers/packet.spec.md` remains `status: superseded` and its directory untouched.
- `packet.spec.md` is ready for `status: implemented`. **Only then may `210b` be activated.**

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (`AC-1` … `AC-8`, `AC-17` … `AC-19`, `AC-N1` … `AC-N3`, `AC-N6`, `AC-N7`) and the three packet-level gate commands.
- Record which frozen goldens were regenerated and the named mechanism for each. A regeneration with no named mechanism is a finding, not a closure; a regeneration not named in `docs/DEVIATION_LOG.md` fails AC-8 clause (c).
- Confirm no tolerance constant moved: `let tolerance_mm = 0.5_f32;` and `let tolerance_fraction = 0.10_f32;` in `orca_parity_tdd.rs`; both `0.10, 0.5` argument pairs in the wedge comparator; the four float tolerances in `orca_parity_tdd.rs` (`radius_tapers_with_distance_to_top` ×2, `raft_and_interface_layers_emit_expected_entry_count`, `wall_count_scales_max_move_distance`). Confirm any behaviour-test flip was resolved by widening a *fixture's* geometric margin, never by loosening an assertion.
- Confirm the exported surface `210b` depends on is exactly as promised in `packet.spec.md` §Exports Consumed by 210b. If any signature drifted during implementation, update `210b`'s `design.md` §Prerequisites in the same session and say so in the closure report.
- Record remaining packet-local risk: sub-unit rounding may have shifted a fixture node by up to 0.5 unit (5 × 10⁻⁵ mm); above 512.0003 mm the mm round trip quantises to 100 nm (AC-N7).
- Confirm context stayed at or below 150k, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.
- `cargo test --workspace` is **not** required for this packet's closure. The wedge invariant suite, both golden suites, `cargo test -p support-planner`, and `cargo check/clippy --workspace --all-targets` are the closure bar. Run the full suite only if the user asks — and if you do, use `cargo xtask test --summary --workspace` per `CLAUDE.md`, dispatched to a sub-agent returning `FACT pass/fail`.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
