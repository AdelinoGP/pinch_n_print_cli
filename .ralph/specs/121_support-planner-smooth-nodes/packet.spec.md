---
status: draft
packet: 121
task_ids:
  - TASK-286
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-smooth-nodes

## Goal

Port OrcaSlicer's `TreeSupport::smooth_nodes` (100-iteration three-point Laplacian smoothing on branch chains) to `support-planner` so the per-layer (x, y) positions of each branch column are smoothed into continuous curves rather than the raw stairstep positions produced by the per-layer `clamp_to_avoidance` snap. Add a `branch_curvature_below_threshold` invariant to the wedge harness (`crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`) that asserts no consecutive segment pair across all `SupportPlanEntry.branch_segments[*]` exceeds 30° turn angle — the smoothing is gate-locked against regression by this invariant.

## Scope Boundaries

Touches `modules/core-modules/support-planner/src/lib.rs` (new `smooth_branches` function + integration into `plan_for_object` after the propagation loop) and extends the wedge harness with one new invariant. No IR change, no WIT change, no manifest change. `branch_segments: Vec<ExtrusionPath3D>` (per `crates/slicer-ir/src/slice_ir.rs:1125`) carries per-layer 2-point segments; the smoothing operates across layers, collecting each branch column's per-layer (x, y) sequence and applying Laplacian in place. Endpoint columns (root and tip) are held fixed. Branch *count* and *connectivity* are preserved.

The packet does NOT touch `tree-support` or `traditional-support` — they consume the smoothed `SupportPlanIR` via the existing `support_plan_segments_for` path.

## Prerequisites and Blockers

- Depends on: `119_support-validation-wedge-harness` wedge harness (the `support_invariants_wedge_tdd.rs` file already exists with 7 invariants; this packet adds the 8th). `117_support-planner-geometric-correctness` implemented (TASK-281 + TASK-282 closed 2026-07-19; the radii that get smoothed are post-tip-cone). `120_support-modules-paint-segment-annotations-migration` (TASK-285) is a soft dependency — the elligibility of nodes doesn't change with smoothing, but the soft dependency keeps the spec-packet dependency graph intact for review.
- Unblocks: `122_support-planner-multi-neighbour-mst`, `123_support-planner-to-buildplate-pruning` — both inherit smoothed branch output and rely on this packet's curvature invariant as a regression gate.
- Activation blockers: the wedge harness file `support_invariants_wedge_tdd.rs` must be present and the existing 7 invariants must be GREEN. Confirmed in the codebase survey.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** searched, **then** a function `fn smooth_branches(entries: &mut Vec<SupportPlanEntry>, iterations: usize)` exists that performs Laplacian smoothing on per-layer (x, y) positions of each branch column using the three-point average `p[i] = (p[i-1] + p[i] + p[i+1]) / 3` for `iterations` iterations (default 100), holding the column's tip and root points fixed. Width (radius) at each layer is also smoothed via the same formula, clamped to `[0.0, MAX_BRANCH_RADIUS_MM = 6.0]` after each iteration. | `rg -q 'fn smooth_branches' modules/core-modules/support-planner/src/lib.rs && rg -q 'iterations: usize' modules/core-modules/support-planner/src/lib.rs`
- **AC-2. Given** a synthetic branch column `[(0.0, 0.0, 5.0), (1.0, 0.0, 4.0), (1.0, 1.0, 3.0), (2.0, 1.0, 2.0), (2.0, 0.0, 1.0)]` (5 points; tip and root have zero z-displacement) with `iterations = 100`, **when** `smooth_branches(...)` is called, **then** the result's middle-three points have lower maximum-consecutive-segment turn-angle than the input (curvature measured as the maximum turn-angle between consecutive segment vectors). | `cargo test -p support-planner --test smooth_nodes_tdd -- smoothing_reduces_curvature --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the same synthetic branch column, **when** `smooth_branches(...)` is called, **then** the first and last points are unchanged (tip and root held fixed). | `cargo test -p support-planner --test smooth_nodes_tdd -- endpoints_held_fixed --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support-planner::plan_for_object` running on the wedge fixture, **when** the planner emits `SupportPlanIR.entries`, **then** the emitted `branch_segments[*].points[*]` carry smoothed (x, y) positions — i.e., the new invariant `branch_curvature_below_threshold` in `support_invariants_wedge_tdd.rs` PASSES with a packet-defined threshold (≤ 30° max turn-angle per consecutive segment pair). | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd -- branch_curvature_below_threshold --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`, **when** searched, **then** a new `#[test] fn branch_curvature_below_threshold` exists that asserts no consecutive (x, y) position pair across all `SupportPlanEntry.branch_segments[*].points[*]` exceeds 30° turn angle. The threshold 30° is empirical — it must be loose enough to allow legitimately-curved smoothed branches on the wedge and tight enough to catch unsmoothed stairsteps. | `rg -q 'fn branch_curvature_below_threshold' crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- **AC-6. Given** the regenerated `resources/golden/support_regression_wedge_branch_count.txt` and `..._endpoints.txt` (re-captured by this packet after smoothing lands, via the `SUPPORT_WEDGE_REGEN_GOLDEN=1` env var — the test at `support_golden_regression_wedge_tdd.rs:65` handles the regen), **when** `support_golden_regression_wedge_tdd::current_wedge_output_stays_within_self_capture_tolerance` runs, **then** the goldens are within tolerance of the smoothed-output baseline (count drift ≤ 10%, endpoint Hausdorff ≤ 0.5 mm — both tolerances are already encoded in the test at lines 110-111). The commit message documents the algorithmic shift; reviewers verify the new shape is "intended different, not regression." | `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- **AC-7. Given** the existing 7 wedge invariants in `support_invariants_wedge_tdd.rs` (`support_plan_has_finite_branch_paths`, `branch_endpoints_are_outside_support_collision_outlines`, `branch_points_match_entry_layer_z`, `overhang_facets_have_wedge_layer_contacts`, `branch_radii_stay_within_current_bounds`, `disabled_raft_has_no_negative_entries`, `support_disabled_produces_explicit_empty_plan`), **when** run after smoothing is enabled, **then** ALL 7 still pass. | `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a branch column with only two points (root + tip, no middle layer), **when** `smooth_branches(...)` is called, **then** the column is returned unchanged (smoothing requires ≥ 3 points; columns shorter than 3 are no-op). | `cargo test -p support-planner --test smooth_nodes_tdd -- columns_below_three_points_unchanged --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** an empty `entries: Vec<SupportPlanEntry>`, **when** `smooth_branches(...)` is called, **then** the function returns without panicking (graceful empty handling). | `cargo test -p support-planner --test smooth_nodes_tdd -- empty_entries_no_panic --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p support-planner --test smooth_nodes_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo clippy -p support-planner -p slicer-runtime --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C3 — directly. Note the invariant list grows with this packet.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::smooth_nodes` (~line 3153 in the Orca tree) — confirm the 100-iteration count, the three-point Laplacian formula, the endpoint-fixed convention, and the `need_extra_wall` flag interactions (this packet does NOT port `need_extra_wall`; that flag is future work). DELEGATED per OrcaSlicer Reference Obligations.
- `crates/slicer-ir/src/slice_ir.rs:1113-1126` — `SupportPlanEntry` definition; `branch_segments: Vec<ExtrusionPath3D>`.
- `crates/slicer-ir/src/slice_ir.rs:1780-1788` — `ExtrusionPath3D` definition; `points: Vec<Point3WithWidth>`.
- `modules/core-modules/support-planner/src/lib.rs:313-756` — `plan_for_object` function (integration point: at the end of the function, after the propagation loop, before the `entries_in_order` final emit at line 750).

## Doc Impact Statement (Required)

- `docs/specs/support-modules-orca-port.md` §Validation Strategy — append the `branch_curvature_below_threshold` invariant to the v1 invariant list (Step 4 of the implementation plan). Verification: `rg -q 'branch_curvature_below_threshold' docs/specs/support-modules-orca-port.md`.

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
