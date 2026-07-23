# Requirements: support-planner-smooth-nodes

## Packet Metadata

- Grouped task IDs:
  - `TASK-286` (renumbered from source-plan `TASK-262`; `TASK-262` is now used by `docs/07_implementation_status.md` for `PrePass::LightningTreeGen` per packet 137).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`support-planner`'s top-down MST propagation emits raw stairstep branch geometry. Each layer's branch position is whichever `clamp_to_avoidance` snap produced (see `modules/core-modules/support-planner/src/lib.rs:707`); the per-layer positions form a column across the support-plan entries (one per layer), and consecutive layers' positions are not smoothed. OrcaSlicer's `TreeSupport::smooth_nodes` (TreeSupport.cpp ~3153) runs 100 iterations of three-point Laplacian smoothing over each branch chain after `drop_nodes` completes; the visual difference is significant (smooth teardrops vs zigzag stairsteps). This packet ports the smoothing pass to the planner.

The validation harness (packet 119) does NOT currently include a curvature invariant. Without one, future algorithmic changes can silently re-introduce stairsteps as long as the existing 7 invariants pass. This packet adds the invariant alongside the algorithmic change.

## In Scope

- Implement `fn smooth_branches(entries: &mut Vec<SupportPlanEntry>, iterations: usize)` in `modules/core-modules/support-planner/src/lib.rs`. The function groups `SupportPlanEntry` rows by `(object_id, region_id)`, sorts each group by `global_layer_index` descending (top-to-bottom), and treats the per-layer (x, y) positions as a chain. Endpoint indices (highest z and lowest z, i.e., the tip and the root) are NOT modified. Width (radius) is smoothed the same way.
- Per iteration: for each chain of ≥ 3 points, for each non-endpoint index `i`, compute `p[i] = (p[i-1] + p[i] + p[i+1]) / 3`. Same for `width`. Width clamped to `[0.0, MAX_BRANCH_RADIUS_MM = 6.0]` after each iteration (per `modules/core-modules/support-planner/src/lib.rs:66`).
- Default iteration count: 100 (matches Orca).
- Run the smoothing pass at the end of `plan_for_object` (after the propagation loop at line 742 completes, before the final `entries_in_order` emit at line 750).
- Add `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs::branch_curvature_below_threshold` invariant (≤ 30° max turn-angle per consecutive segment pair).
- Add `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` with AC-2, AC-3, AC-N1, AC-N2.
- Regenerate `resources/golden/support_regression_wedge_branch_count.txt` and `..._endpoints.txt` (intentional re-anchor; the regen is triggered by setting the env var `SUPPORT_WEDGE_REGEN_GOLDEN=1` and running `support_golden_regression_wedge_tdd::current_wedge_output_stays_within_self_capture_tolerance` — see `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs:65`).
- Update `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list to add `branch_curvature_below_threshold`.

## Out of Scope

- Porting Orca's `need_extra_wall` flag interactions (tall-branch dual-wall path). Future work.
- Configurable iteration count from the manifest. The 100-iteration default ships hardcoded; configurability is a future packet.
- Smoothing across MERGED chains (where two branches share a child via multi-neighbour aggregation). The current planner does not yet emit merged chains; sibling packet `122_support-planner-multi-neighbour-mst` introduces them. Smoothing across merges is part of that packet.
- Performance optimization (the 100-iteration pass over moderate chain counts is fast enough at the wedge fixture scale).
- Touching `tree-support` or `traditional-support` — they consume the smoothed `SupportPlanIR` via the existing `support_plan_segments_for` path (see `modules/core-modules/tree-support/src/lib.rs:159`).

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C3 — directly.
- `docs/specs/support-modules-orca-port.md` §Validation Strategy — invariant list grows here.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::smooth_nodes` — confirm 100-iteration count, three-point Laplacian formula, endpoint-fixed convention.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-7`.
  - AC-1: function exists with correct signature shape.
  - AC-2, AC-3: unit-test the function in isolation.
  - AC-4, AC-5: extend the wedge harness with the curvature invariant.
  - AC-6: re-anchor self-capture goldens.
  - AC-7: smoothing does not regress the existing 7 wedge invariants.
- Negative cases: AC-N1 (chain shorter than 3), AC-N2 (empty input).
- Cross-packet impact: future Block C packets re-anchor goldens after their changes; the curvature invariant becomes part of the permanent invariant set.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Guest WASM current. | FACT pass/fail |
| `cargo test -p support-planner --test smooth_nodes_tdd 2>&1 \| tee target/test-output.log` | AC-2, AC-3, AC-N1, AC-N2. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-4, AC-5, AC-7. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance 2>&1 \| tee target/test-output.log` | AC-6 re-anchor (regen happens here). | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-6 (without env, verifies tolerance). | FACT pass/fail |
| `cargo clippy -p support-planner -p slicer-runtime --all-targets -- -D warnings` | Lint gate. | FACT pass/fail |

## Step Completion Expectations

- The goldens MUST be re-captured AFTER the smoothing pass is integrated into `plan_for_object`. If they are captured before integration, AC-6 will pass against the unsmoothed baseline and the invariant check (AC-4) will catch the inconsistency.
- The curvature invariant threshold (30° max turn-angle per consecutive segment pair) is a packet-defined number. It must be loose enough to allow legitimately-curved smoothed branches on the wedge and tight enough to catch unsmoothed stairsteps. The implementer picks the exact number empirically by running the harness against pre-smoothing and post-smoothing planner output; the chosen number lands in the invariant test's assertion message.
- Smoothing must NOT change the *number* of `branch_segments` or their *connectivity* (start/end indices). Step 4 of the implementation plan asserts the segment count is preserved within the bounds of the count tolerance.
- Smoothing operates on per-layer (x, y) positions; the per-layer (z, width) are NOT smoothed. Only x and y and width participate in the three-point Laplacian. This matches Orca's `smooth_nodes` (which only smooths the planar coordinates; z is fixed by the layer plan).

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `modules/core-modules/support-planner/src/lib.rs` — 1590 lines; range-read around `plan_for_object`'s end (lines 740-756, the propagation-loop tail + the `entries_in_order` emit). The smoothing integration point is between line 742 and line 750.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — 261 lines; read the 7 existing test functions to copy the setup pattern; the new test fits at the bottom of the file.
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — delegate `smooth_nodes` SUMMARY.
- Likely temptation reads (skip these):
  - Other Orca smoothing utilities (Curvature.cpp, etc.) — out of scope; the smoothing here is the Laplacian-on-chain only.
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo test -p slicer-runtime --test support_invariants_wedge_tdd` — FACT (per-test pass/fail).
  - "Summarize OrcaSlicer smooth_nodes" — SUMMARY ≤ 200 words.
