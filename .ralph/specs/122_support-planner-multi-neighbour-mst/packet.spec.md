---
status: implemented
packet: 122
task_ids:
  - TASK-287
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-multi-neighbour-mst

## Goal

Replace `support-planner`'s single-neighbour MST propagation — the `nearest_neighbour` + `nearest_distance` lookup at `modules/core-modules/support-planner/src/lib.rs:669-682` (current line numbers have drifted; symbol names are stable identifiers) that picks exactly one MST neighbour per node (the lowest-distance endpoint) and uses it as the move target at lines 688-704 — with multi-neighbour target synthesis matching OrcaSlicer's `TreeSupport::drop_nodes`: each node's move target is the reciprocal-distance-squared (1/d²) weighted aggregate of ALL its MST neighbours. Add a `merge_geometry_symmetric_for_n_branches` invariant to the wedge harness asserting that for every node with ≥ 3 incoming MST edges (a merge point), the merge is approximately equidistant from contributing branches.

## Scope Boundaries

Touches `modules/core-modules/support-planner/src/lib.rs` — the per-neighbour lookup at lines 671-682 and the move-target synthesis at lines 688-704 are replaced. The `mst_edges: Vec<(a_idx, b_idx, distance)>` source data is unchanged. No IR change, no manifest change, no WIT change. Branch *connectivity* may change (different nodes become merge points); the self-capture goldens are re-anchored.

The packet does NOT touch `tree-support` or `traditional-support` — they consume the planner output via the existing `support_plan_segments_for` path.

## Prerequisites and Blockers

- Depends on: `119_support-validation-wedge-harness` wedge harness; `117_support-planner-geometric-correctness` implemented (TASK-281 + TASK-282 closed 2026-07-19); `120_support-modules-paint-segment-annotations-migration` (TASK-285) for the elligibility path; `121_support-planner-smooth-nodes` (TASK-286) for the smoothed chain output (this packet builds on the smoothed-chain output but is independent enough to land before smoothing too — the symmetry invariant only checks the merge geometry, not the smoothness).
- Unblocks: `123_support-planner-to-buildplate-pruning` (TASK-288) — relies on this packet's symmetric merge for unsupported-branch pruning.
- Activation blockers: the wedge harness file `support_invariants_wedge_tdd.rs` must be present and the existing 7 + AC-5 (curvature, added by packet 121) invariants must be GREEN.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/support-planner/src/lib.rs` propagation block (originally lines 669-704, current line numbers have drifted), **when** searched, **then** the per-node move-target synthesis iterates over ALL MST neighbours of the node (not just `nearest_neighbour`), producing a target XY that is the weighted aggregate of all neighbour positions. Weights are `1.0 / (distance_j * distance_j)` (reciprocal squared distance — 1/d² — matching OrcaSlicer's `TreeSupport::drop_nodes` non-`is_strong` path), normalized so weights sum to 1. The degenerate `distance_j < 1e-6 mm` case collapses to that neighbour's position (the dominant weight saturates; no division by zero). | `rg -q 'fn aggregate_neighbour_targets\|fn multi_neighbour_aggregate' modules/core-modules/support-planner/src/lib.rs && rg -q 'all_neighbours\|aggregate.*neighbour' modules/core-modules/support-planner/src/lib.rs`
- **AC-2. Given** a synthetic 3-neighbour fan (one central node with three MST neighbours at equal distance, symmetric arrangement), **when** the propagation pass runs for one step, **then** the central node's new position is the centroid of its three neighbours (within `1e-3 mm` of the geometric centroid). | `cargo test -p support-planner --test multi_neighbour_mst_tdd -- symmetric_3_neighbour_centroid --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the same fan with asymmetric arrangement (one neighbour at 1 mm, two neighbours at 5 mm), **when** the propagation runs for one step, **then** the central node's new position weights the closer neighbour more heavily — the new position is closer to the 1 mm neighbour than to the 5 mm cluster's midpoint (the 1 mm neighbour's reciprocal-squared weight `1.0/1.0² = 1.0` dominates the 5 mm neighbours' `1.0/5.0² = 0.04`). | `cargo test -p support-planner --test multi_neighbour_mst_tdd -- asymmetric_neighbours_weighted_by_reciprocal_squared --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support_invariants_wedge_tdd.rs`, **when** searched, **then** a new `#[test] fn merge_geometry_symmetric_for_n_branches` exists that asserts: for every merge point (a node with ≥ 3 incoming MST edges in the planner's internal data, OR equivalently, every `SupportPlanEntry.branch_segments` tuple where three or more segments share a common endpoint), the standard deviation of distances from the merge point to its contributing endpoint XYs is ≤ 30% of the mean distance. The threshold 30% is empirical — it must be loose enough to allow legitimately-asymmetric smoothed branches and tight enough to catch the old single-neighbour asymmetry. | `rg -q 'fn merge_geometry_symmetric_for_n_branches' crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- **AC-5. Given** the new wedge invariant, **when** run after multi-neighbour propagation lands, **then** it PASSES. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- merge_geometry_symmetric_for_n_branches --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the regenerated wedge goldens (re-anchored by this packet via `SUPPORT_WEDGE_REGEN_GOLDEN=1`), **when** the golden-regression test runs without the env var, **then** the tolerance check PASSES (count drift ≤ 10%, endpoint Hausdorff ≤ 0.5 mm — tolerances already encoded in `support_golden_regression_wedge_tdd.rs:110-111`). | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- **AC-7. Given** all previous wedge invariants (the 7 from packet 119 + the curvature invariant from packet 121), **when** run after this packet's algorithm change, **then** ALL PASS. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a synthetic single-neighbour case (a chain with only one MST neighbour at each step), **when** the propagation runs, **then** the behavior matches the old single-neighbour algorithm (degenerate case: reciprocal-distance weighted aggregate over 1 element is that element). | `cargo test -p support-planner --test multi_neighbour_mst_tdd -- single_neighbour_degenerate_case_matches_old --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a node with multiple MST neighbours where one has `distance = 0` (zero distance — they coincide), **when** the aggregate is computed, **then** the result is the coincident neighbour's position (no division-by-zero, the dominant weight saturates to 1.0). | `cargo test -p support-planner --test multi_neighbour_mst_tdd -- zero_distance_neighbour_does_not_panic --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p support-planner --test multi_neighbour_mst_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C4 — directly.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::drop_nodes` (~line 2625) — confirm multi-neighbour aggregation formula; identify the weighting scheme Orca uses (reciprocal distance vs. equal weight vs. other). DELEGATED per OrcaSlicer Reference Obligations.
- `modules/core-modules/support-planner/src/lib.rs:671-704` — the propagation block being replaced.

## Doc Impact Statement (Required)

- `docs/specs/support-modules-orca-port.md` §Validation Strategy — append the `merge_geometry_symmetric_for_n_branches` invariant to the v1 invariant list (Step 4 of the implementation plan). Verification: `rg -q 'merge_geometry_symmetric_for_n_branches' docs/specs/support-modules-orca-port.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::drop_nodes` (~line 2625) — confirm multi-neighbour aggregation formula; identify the weighting scheme Orca uses (reciprocal distance vs. equal weight vs. other).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
