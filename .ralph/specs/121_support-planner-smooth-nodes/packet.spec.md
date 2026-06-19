---
status: draft
packet: 121
task_ids:
  - TASK-262
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-smooth-nodes

## Goal

Port OrcaSlicer's `TreeSupport::smooth_nodes` (100-iteration three-point Laplacian smoothing) to `support-planner`'s branch-chain emission so the resulting `SupportPlanEntry.branch_segments` show smoothed branches instead of stairstep XY positions, and add a curvature invariant to the validation harness so the smoothing is gate-locked against regression.

## Scope Boundaries

Touches `support-planner/src/lib.rs` (new `smooth_chains` function + integration into the top-down propagation finalization) and extends the wedge harness (`support_invariants_wedge_tdd.rs`) with one new invariant. No IR change, no WIT change, no manifest change. The smoothing is a final pass after the MST propagation completes — node connectivity is preserved; only XY positions and radii are updated.

## Prerequisites and Blockers

- Depends on: `119_support-validation-wedge-harness` implemented; `117_support-planner-geometric-correctness` implemented (tip cone semantics underpin the radii being smoothed); `120_support-modules-paint-segment-annotations-migration` implemented (so the enforcer/blocker eligibility is correct before smoothing assumptions kick in).
- Unblocks: `122_support-planner-multi-neighbour-mst`, `123_support-planner-to-buildplate-pruning` — both inherit smoothed branch output and rely on this packet's curvature invariant as a regression gate.
- Activation blockers: packet 4 (validation harness) must report `status: implemented`.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** searched, **then** a function `fn smooth_chains(...)` exists that performs Laplacian smoothing on branch-chain points using the three-point average `p[i] = (p[i-1] + p[i] + p[i+1]) / 3` for `iterations` iterations (default 100), holding endpoints (root and tip) fixed. The same loop smooths the radii via `r[i] = (r[i-1] + r[i] + r[i+1]) / 3`. | `rg -q 'fn smooth_chains' modules/core-modules/support-planner/src/lib.rs && rg -q 'iterations: usize' modules/core-modules/support-planner/src/lib.rs`
- **AC-2. Given** a synthetic chain `[(0, 0, 1), (1, 0, 0.8), (1, 1, 0.6), (2, 1, 0.4), (2, 0, 0.2)]` with `iterations = 100`, **when** `smooth_chains(...)` is called, **then** the result's middle-three points have lower maximum-curvature than the input (curvature measured as the maximum turn-angle between consecutive segment vectors). | `cargo test -p support-planner --test smooth_nodes_tdd -- smoothing_reduces_curvature --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the same synthetic chain, **when** `smooth_chains(...)` is called, **then** the first and last points are unchanged (endpoints held fixed). | `cargo test -p support-planner --test smooth_nodes_tdd -- endpoints_held_fixed --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support-planner::plan_for_object` running on the wedge fixture, **when** the planner emits `SupportPlanIR.entries`, **then** the emitted `branch_segments` have been smoothed — i.e., the new invariant in `support_invariants_wedge_tdd.rs` (`branch_curvature_below_threshold`) passes with a packet-defined threshold (≤ 30° max-turn-angle per consecutive segment pair). | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- branch_curvature_below_threshold --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`, **when** searched, **then** a new `#[test] fn branch_curvature_below_threshold` exists that asserts no consecutive segment pair across all `SupportPlanIR.entries[*].branch_segments[*]` exceeds 30° turn angle. | `rg -q 'fn branch_curvature_below_threshold' crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- **AC-6. Given** the regenerated `resources/golden/support_regression_wedge_branch_count.txt` and `..._endpoints.txt` (re-captured by this packet after smoothing lands), **when** `support_golden_regression_wedge_tdd` runs, **then** the goldens are within tolerance of the smoothed-output baseline (re-anchor; the commit message documents the algorithmic shift). | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- **AC-7. Given** invariants 1-5 from the wedge harness (`support_invariants_wedge_tdd.rs`), **when** run after smoothing is enabled, **then** ALL still pass — smoothing must not regress reachability, collision-free, monotone `dist_to_top`, overhang coverage, or radius monotone. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a chain with only two points (root + tip, no middle), **when** `smooth_chains(...)` is called, **then** the chain is returned unchanged (smoothing requires ≥ 3 points; chains shorter than 3 are no-op). | `cargo test -p support-planner --test smooth_nodes_tdd -- chains_below_three_points_unchanged --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p support-planner --test smooth_nodes_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C3 — directly.
- `docs/specs/support-modules-orca-port.md` §Validation Strategy — note the invariant list grows with this packet.

## Doc Impact Statement (Required)

`none` — this packet adds a new invariant test, an algorithm-internal smoothing pass, and regenerates the self-capture goldens. The spec at `docs/specs/support-modules-orca-port.md` §C3 already describes the smoothing port; no public surface, IR, WIT, claim, or manifest schema change.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::smooth_nodes` (line ~3153 in the Orca tree) — confirm the 100-iteration count, the three-point Laplacian formula, the endpoint-fixed convention, and the `need_extra_wall` flag interactions (this packet does NOT port `need_extra_wall`; that flag is future work).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
