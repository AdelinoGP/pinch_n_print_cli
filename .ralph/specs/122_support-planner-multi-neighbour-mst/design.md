# Design: support-planner-multi-neighbour-mst

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs:671-682` — `nearest_neighbour` / `nearest_distance` lookups replaced with a per-node all-neighbours scan.
  - `modules/core-modules/support-planner/src/lib.rs:684-704` — the move-target synthesis updated to use the new aggregate.
  - `modules/core-modules/support-planner/src/lib.rs::aggregate_neighbour_targets` (NEW helper; pure function: `fn aggregate_neighbour_targets(neighbours: &[(usize, f32)], active_nodes: &[PlannedSupportNode]) -> Option<(f32, f32)>`).
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` (new) — AC-2, AC-3, AC-N1, AC-N2.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — extended with AC-4 invariant (the 9th).
  - Goldens regenerated via `SUPPORT_WEDGE_REGEN_GOLDEN=1`.
  - `docs/specs/support-modules-orca-port.md` §Validation Strategy — invariant list extended.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The reciprocal-distance weighting MUST handle the degenerate case `D_j = 0` (zero distance) without dividing by zero. Implementation: when any `D_j < 1e-6 mm`, the target collapses to that neighbour's position (the weight is dominant).
- The aggregation is deterministic: same input MST, same output target.
- Existing `max_move_xy` cap (line 695-704) and `clamp_to_avoidance` enforcement (line 707) are preserved.
- The `aggregate_neighbour_targets` helper is a pure function (no side effects), making it directly unit-testable without planner setup.

## Code Change Surface

- Selected approach: extract the aggregate into a pure helper `aggregate_neighbour_targets`; rewrite the propagation block to call it; the `nearest_neighbour` + `nearest_distance` Vec allocations are replaced with a `neighbours_of: Vec<Vec<(usize, f32)>>` lookup (one inner Vec per active node containing all incident MST edges).
- Exact functions/structs/tests to change:
  - The block in `plan_for_object` that computes `nearest_neighbour` and `nearest_distance` (lines 671-682).
  - The downstream block that uses them (lines 688-704) to synthesize the move target.
  - New `aggregate_neighbour_targets` helper.
  - `support_invariants_wedge_tdd::merge_geometry_symmetric_for_n_branches` (new test).
  - `multi_neighbour_mst_tdd.rs` (new test file with four tests).
  - Goldens (regenerated).
  - `docs/specs/support-modules-orca-port.md` (one line in the invariant list).
- Rejected alternatives:
  - **Equal-weight averaging (no reciprocal)** — rejected: Orca uses distance-weighted aggregation per the survey SUMMARY; equal weighting would lose the "closer matters more" property.
  - **Limit aggregation to top-3 nearest neighbours** — rejected: not Orca's behavior; arbitrary cutoff.
  - **Smoothing across multi-neighbour merges** — out of scope for this packet; the merging rule (which nodes are dropped) is preserved; only the *direction* of the move changes.

## Files in Scope (read + edit)

- `modules/core-modules/support-planner/src/lib.rs` — propagation block rewrite.
- `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` — new test file.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — new test added.
- `resources/golden/support_regression_wedge_branch_count.txt` and `..._endpoints.txt` — regenerated.
- `docs/specs/support-modules-orca-port.md` — invariant list extension (1 line).

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C4 — directly.
- Existing propagation block (lines 669-704) — range-read.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- Other modules; the `prim_mst` function (the MST builder stays unchanged).
- `target/**`, generated code.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `TreeSupport::drop_nodes` aggregation formula; return SUMMARY ≤ 200 words. Confirm reciprocal-distance weighting vs alternatives."
- "Run `cargo test -p support-planner --test multi_neighbour_mst_tdd`; return FACT per-test."
- "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT per-test."
- "Run `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance`; return FACT (regen happened)."
- "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail."
- "Run `cargo xtask build-guests --check`; return FACT."

## Data and Contract Notes

- IR contract: unchanged.
- WIT: none.
- Determinism: preserved.

## Locked Assumptions and Invariants

- All existing wedge invariants (7 from packet 119 + 1 curvature from packet 121) continue to hold post-change.
- `max_move_xy` cap and `clamp_to_avoidance` enforcement are preserved.
- Degenerate `D_j = 0` does not panic.

## Risks and Tradeoffs

- **Risk**: aggregation may produce targets outside avoidance polys more often than single-neighbour did. **Mitigation**: existing `clamp_to_avoidance` post-cap handles it; invariant 2 (no-collision) catches regressions.
- **Risk**: symmetry threshold (30% stddev/mean) may be too tight on real wedge merges. **Mitigation**: Step 4 empirically picks the threshold.
- **Risk**: re-anchored goldens shift significantly because branch *connectivity* changes (different nodes become merge points). **Mitigation**: AC-6 tolerance check captures the shift; if > 10% drift, the shift is intentional and the goldens are re-anchored with documentation in the commit message.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 — propagation block rewrite + helper).

## Open Questions

None.
