---
status: draft
packet: 122
task_ids:
  - TASK-263
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-multi-neighbour-mst

## Goal

Replace `support-planner`'s single-neighbour MST propagation (each node moves toward its lowest-distance MST neighbour, producing asymmetric chains when ≥ 3 MST neighbours exist) with multi-neighbour target synthesis matching OrcaSlicer's `TreeSupport::drop_nodes` (each node's target is synthesized from ALL its MST neighbours), and add a symmetry invariant to the wedge harness that asserts merge geometry is symmetric.

## Scope Boundaries

Touches `support-planner/src/lib.rs` (the propagation block around line 586-660 — `nearest_neighbour` lookup replaced with multi-neighbour aggregation) and extends the wedge harness with one new invariant. No IR change, no manifest change. Branch *connectivity* changes — different nodes end up as merge points; the self-capture goldens are re-anchored.

## Prerequisites and Blockers

- Depends on: `119_support-validation-wedge-harness`, `117_support-planner-geometric-correctness`, `120_support-modules-paint-segment-annotations-migration`, AND `121_support-planner-smooth-nodes` (this packet builds on the smoothed-chain output).
- Unblocks: `123_support-planner-to-buildplate-pruning` (relies on this packet's symmetric merge for unsupported-branch pruning).
- Activation blockers: predecessor packets `status: implemented`.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/support-planner/src/lib.rs` propagation block, **when** searched, **then** the per-node move-target synthesis iterates over ALL MST neighbours of the node (not just `nearest_neighbour`), producing a target XY that is the weighted aggregate (by reciprocal distance) of all neighbour positions. | `rg -q 'all_neighbours' modules/core-modules/support-planner/src/lib.rs && rg -q 'reciprocal' modules/core-modules/support-planner/src/lib.rs`
- **AC-2. Given** a synthetic 3-neighbour fan (one central node with three MST neighbours at equal distance, symmetric arrangement), **when** the propagation pass runs for one step, **then** the central node's new position is the centroid of its three neighbours (within `1e-3 mm`). | `cargo test -p support-planner --test multi_neighbour_mst_tdd -- symmetric_3_neighbour_centroid --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the same fan with asymmetric arrangement (one neighbour at 1 mm, two neighbours at 5 mm), **when** the propagation runs for one step, **then** the central node's new position weights the closer neighbour more heavily (reciprocal-distance weighting) — the new position is closer to the 1 mm neighbour than to the 5 mm cluster's midpoint. | `cargo test -p support-planner --test multi_neighbour_mst_tdd -- asymmetric_neighbours_weighted_by_reciprocal --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support_invariants_wedge_tdd.rs`, **when** searched, **then** a new `#[test] fn merge_geometry_symmetric_for_n_branches` exists that asserts: for every node with ≥ 3 incoming MST edges (a merge point), the average squared distance from the merge point to its parent endpoints is within ±15% of the squared-distance variance (i.e., the merge is approximately equidistant from contributing branches). | `rg -q 'fn merge_geometry_symmetric_for_n_branches' crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- **AC-5. Given** the new wedge invariant, **when** run after multi-neighbour propagation lands, **then** it PASSES. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- merge_geometry_symmetric_for_n_branches --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the regenerated wedge goldens (re-anchored by this packet), **when** the golden-regression test runs, **then** the tolerance check PASSES. | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- **AC-7. Given** all previous wedge invariants (reachability, no-collision, dist_to_top monotone, overhang coverage, radius monotone, curvature from packet 6), **when** run after this packet's algorithm change, **then** ALL PASS. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a synthetic single-neighbour case (a chain with only one MST neighbour at each step), **when** the propagation runs, **then** the behavior matches the old single-neighbour algorithm (degenerate case: reciprocal-distance weighted aggregate over 1 element is that element). | `cargo test -p support-planner --test multi_neighbour_mst_tdd -- single_neighbour_degenerate_case_matches_old --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p support-planner --test multi_neighbour_mst_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C4 — directly.

## Doc Impact Statement (Required)

`none` — algorithm-internal change with re-anchored goldens. The spec already documents the multi-neighbour intent.

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
