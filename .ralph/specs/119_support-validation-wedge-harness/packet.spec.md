---
status: draft
packet: 119
task_ids:
  - TASK-260
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-validation-wedge-harness

## Goal

Stand up the validation harness that gates every subsequent Block C support packet: six invariant tests asserted on `resources/regression_wedge.stl` against the planner's output, plus a self-capture golden regression that captures branch count and endpoints as `resources/golden/support_regression_wedge_*.txt` and asserts future drift stays within ±10% on count and ≤ 0.5 mm Hausdorff on endpoints.

## Scope Boundaries

Touches only test files and golden artifacts. No production code change. No IR change, no WIT change, no manifest change. The invariant list is documented as v1 and expected to grow as future C-block items land (smooth_nodes adds a curvature invariant; multi-neighbour-MST adds a symmetry invariant; etc.). The self-capture goldens are written from the planner's CURRENT post-Packet-2 output, so this packet MUST land after `117_support-planner-geometric-correctness` to avoid baking the broken `tapered_radius` tip behavior into the baseline.

## Prerequisites and Blockers

- Depends on: `117_support-planner-geometric-correctness` (tip cone + inflate_polygon fix). Without it, the goldens encode the broken floor-at-`branch_radius` behavior and the radius-monotone invariant would be coincidentally satisfied by an algorithm-level bug.
- Unblocks: `121_support-planner-smooth-nodes`, `122_support-planner-multi-neighbour-mst`, `123_support-planner-to-buildplate-pruning` — all four ship algorithmic changes that need this harness as their correctness gate.
- Activation blockers: regression_wedge.stl must produce a non-empty support plan with default config. If a sub-agent confirms it does not, the implementer must adjust config (e.g., enable supports, set `support_threshold_angle = 45`) inside the test setup to ensure the harness has something to assert.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`, **when** the test `reachability_every_chain_terminates_at_buildplate_or_contact` runs against the wedge fixture with default-plus-support config, **then** every `SupportPlanIR.entries[*].branch_segments[*]` endpoint chain terminates at either `z ≤ 1e-3 mm` (build plate) or at an overhang facet's contact point at the facet's origin layer (within `branch_distance` tolerance). | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- reachability_every_chain_terminates_at_buildplate_or_contact --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** the same fixture and config, **when** the test `no_endpoint_inside_collision_polys` runs, **then** no `branch_segment` endpoint lies inside any `collision_polys` for its layer (as computed by the planner's avoidance cache). | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- no_endpoint_inside_collision_polys --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the same fixture, **when** the test `dist_to_top_monotone_along_chain` runs, **then** `dist_to_top` is monotone non-decreasing along every parent-child chain in the planner's output (verified by reconstructing parent links from `branch_segments` shared endpoints and the implicit MST topology recorded in the test fixture's introspection helper). | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- dist_to_top_monotone_along_chain --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the same fixture, **when** the test `overhang_facet_has_contact_at_origin_layer` runs, **then** for every overhang facet (triangle whose normal z-component is ≤ `-sin(45°)`) whose centroid passes the `support_threshold_angle` check, at least one contact point exists at the facet's origin layer within `tree_support_branch_distance` mm tolerance. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- overhang_facet_has_contact_at_origin_layer --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the same fixture, **when** the test `branch_radius_monotone_along_chain` runs, **then** along every parent-child chain the `width / 2` of each `Point3WithWidth` is monotone non-decreasing with `dist_to_top` (clamped to `[0, MAX_BRANCH_RADIUS_MM = 6.0]`). | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- branch_radius_monotone_along_chain --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the wedge fixture with `support_raft_layers = 0` (default), **when** the test `raft_plan_count_zero_when_disabled` runs, **then** the planner emits NO raft entries (current `support-planner` placeholder code path; this invariant evolves into "exactly `support_raft_layers × n_objects_needing_raft` `RaftPlan` rows when `> 0`" after sibling packet `124_support-plan-raft-plan-and-raftinfill-role` lands). The test as written for v1 asserts the zero-raft case only; the documented evolution path is recorded inline. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- raft_plan_count_zero_when_disabled --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** `resources/golden/support_regression_wedge_branch_count.txt` (newly captured by this packet) and `resources/golden/support_regression_wedge_endpoints.txt` (newly captured), **when** `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` runs, **then** the planner's current branch count is within ±10% of the captured baseline AND the Hausdorff distance between the current endpoint set and the captured endpoint set is ≤ 0.5 mm. Either failure fails the test. | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** the wedge fixture with `support_enabled = false`, **when** any invariant test runs, **then** the test detects an empty support plan and short-circuits to PASS (no false positives from empty input). The test must NOT silently pass if `support_enabled = true` but the planner's output is empty — that's a regression, not an empty-input case. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- empty_support_plan_short_circuits_under_disabled --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a deliberately-mutated golden file (e.g., `branch_count.txt` set to a value > 25% off the captured baseline), **when** AC-7's test runs, **then** the test fails with an assertion message naming `branch count drift > 10%`. The test does not silently accept large drift. | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- detects_intentional_branch_count_drift --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` — §C1, §Validation Strategy, §D3. Read directly; defines the six invariants and the tolerance numbers.
- `crates/slicer-runtime/tests/common/` — confirm the fixture-loading and slicer-cache patterns used by neighboring integration tests (e.g. `region_mapping_tdd.rs`, `cube_4color_paint_tdd.rs`). Delegate `LOCATIONS` if not obvious.
- `docs/02_ir_schemas.md` §"SupportPlanIR" — read lines 862-921 directly; the test code asserts against these field paths.

## Doc Impact Statement (Required)

`none` — this packet adds test files and resource files. The Validation Strategy section in `docs/specs/support-modules-orca-port.md` already documents the invariant list and tolerance numbers. No public surface, IR, WIT, claim, or manifest schema change.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
