# Design: support-planner-multi-neighbour-mst

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs` propagation block (lines 586-660 area) — `nearest_neighbour` / `nearest_distance` lookups replaced with all-neighbours aggregation.
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` (new) — AC-2, AC-3, AC-N1.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — extended with AC-4 invariant.
  - Goldens regenerated.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The reciprocal-distance weighting MUST handle the degenerate case `D_j = 0` (zero distance) without dividing by zero. Implementation: when any `D_j < 1e-6 mm`, the target collapses to that neighbour's position (the weight is dominant).
- The aggregation is deterministic: same input MST, same output target.
- Existing `max_move_xy` and `clamp_to_avoidance` enforcement is preserved.

## Code Change Surface

- Selected approach: in-place modification of the propagation block; the `nearest_neighbour` Vec is replaced by a per-node `Vec<(neighbour_idx, distance)>` lookup.
- Exact functions to change:
  - The block in `plan_for_object` that computes `nearest_neighbour` and `nearest_distance` (lines 586-599 area).
  - The downstream block that uses them (lines 601-662 area) to synthesize the move target.
- Rejected alternatives:
  - **Equal-weight averaging (no reciprocal)** — rejected: Orca uses distance-weighted aggregation per the survey SUMMARY; equal weighting would lose the "closer matters more" property.
  - **Limit aggregation to top-3 nearest neighbours** — rejected: not Orca's behavior; arbitrary cutoff.

## Files in Scope (read + edit)

- `modules/core-modules/support-planner/src/lib.rs` — propagation block rewrite.
- `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` — new test file.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — new test added.
- Goldens regenerated.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C4 — directly.
- Existing propagation block — range-read.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- Other modules; the `prim_mst` function (the MST builder stays unchanged).
- `target/**`, generated code.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `TreeSupport::drop_nodes` aggregation formula; return SUMMARY ≤ 200 words. Confirm reciprocal-distance weighting vs alternatives."
- "Run `cargo test -p support-planner --test multi_neighbour_mst_tdd`; return FACT per-test."
- "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT per-test."
- "Run xtask golden-regen; return FACT."
- "Run `cargo xtask build-guests --check`; return FACT."

## Data and Contract Notes

- IR contract: unchanged.
- WIT: none.
- Determinism: preserved.

## Locked Assumptions and Invariants

- All existing wedge invariants continue to hold post-change.
- `max_move_xy` cap and `clamp_to_avoidance` enforcement are preserved.
- Degenerate `D_j = 0` does not panic.

## Risks and Tradeoffs

- **Risk**: aggregation may produce targets outside avoidance polys more often than single-neighbour did. **Mitigation**: existing `clamp_to_avoidance` post-cap handles it; invariant 2 (no-collision) catches regressions.
- **Risk**: symmetry threshold (±15%) may be too tight on real wedge merges. **Mitigation**: Step 4 empirically picks the threshold.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`

## Open Questions

None.
