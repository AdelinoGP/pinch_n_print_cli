# Design: support-planner-smooth-nodes

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs::plan_for_object` — call `smooth_chains` once after MST propagation completes, before emitting `SupportPlanEntry.branch_segments`.
  - `modules/core-modules/support-planner/src/lib.rs::smooth_chains` (NEW function).
  - `modules/core-modules/support-planner/src/lib.rs::extract_chains_for_object` (NEW helper that reconstructs parent-child chains).
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` (NEW) — AC-2, AC-3, AC-N1.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — extended with AC-5's curvature invariant.
  - `resources/golden/support_regression_wedge_*.txt` — regenerated.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Smoothing operates on mm-valued `f32` coordinates (per the planner's existing convention). No unit conversion is introduced.
- The smoothing pass MUST be deterministic. Given identical input chains, the output is identical. No random initialization, no thread-local state.
- The smoothing pass MUST NOT increase the number of points in any chain. Three-point Laplacian replaces interior points in place.
- Radii are clamped to `[0.0, MAX_BRANCH_RADIUS_MM]` after each iteration to preserve the existing invariant.

## Code Change Surface

- Selected approach: dedicated `smooth_chains` function called once at the tail of `plan_for_object`. Chains are extracted, smoothed, and re-injected before emission.
- Exact functions/structs/tests to change:
  - `support_planner::smooth_chains(chains: &mut Vec<Vec<PlannedSupportNode>>, iterations: usize)` (new).
  - `support_planner::extract_chains_for_object(...)` (new helper).
  - `support_planner::plan_for_object` (integration: extract → smooth → re-inject).
  - `support_invariants_wedge_tdd::branch_curvature_below_threshold` (new test in the existing harness file).
  - `smooth_nodes_tdd.rs` (new test file with three tests).
  - Goldens (regenerated).
- Rejected alternatives:
  - **Smooth inline during propagation** — rejected: smoothing is a final-pass operation per Orca's design; mixing it with propagation makes the chain mutation semantics murky.
  - **Make iteration count a config key** — rejected per Out of Scope; future packet.
  - **Use a higher-order smoothing kernel (Gaussian, cubic spline)** — rejected: deviates from Orca's three-point Laplacian without rationale.

## Files in Scope (read + edit)

The packet edits 1 source file + 2 new/extended test files + 2 regenerated goldens (5 total).

- `modules/core-modules/support-planner/src/lib.rs` — role: smoothing + extraction + integration; expected change: ≈80 lines added.
- `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` — role: AC-2, AC-3, AC-N1; expected change: file created.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — role: AC-5 invariant added; expected change: 1 new test function (~30 lines).
- `resources/golden/support_regression_wedge_branch_count.txt` — role: regenerated baseline.
- `resources/golden/support_regression_wedge_endpoints.txt` — role: regenerated baseline.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C3 — directly.
- Existing `extract_chains_for_object`-equivalent code in the validation harness (`reconstruct_chains` helper from packet 4) — pattern reference; if directly reusable, lift to a shared test-support module or duplicate (≤ 30 lines).
- Existing `support-planner` `plan_for_object` tail (the emission loop) — range-read.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/**`, `Cargo.lock`, generated code — never load.
- Other modules — not edited.
- Other infill / perimeter algorithms — not touched.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `TreeSupport::smooth_nodes` from `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return SUMMARY ≤ 200 words confirming iteration count, formula, endpoint convention." — purpose: validate Orca port.
- "Run `cargo test -p support-planner --test smooth_nodes_tdd`; return FACT pass/fail per-test." — purpose: AC-2, AC-3, AC-N1.
- "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT pass/fail per-test." — purpose: AC-4, AC-5, AC-7.
- "Run the xtask golden-regen command for support; return FACT (goldens were updated with which line counts)." — purpose: re-anchor.
- "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail." — purpose: AC-6.
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE." — purpose: WASM gate.

## Data and Contract Notes

- IR contracts touched: none. `SupportPlanEntry.branch_segments` shape unchanged; values within them shift.
- WIT boundary considerations: none.
- Determinism: smoothing is deterministic.
- Curvature threshold (30°) is encoded in the AC-5 test, not a config key.

## Locked Assumptions and Invariants

- Endpoint indices (root, tip) are never mutated by smoothing.
- Chains shorter than 3 points are no-op.
- Radii after smoothing are clamped `[0.0, MAX_BRANCH_RADIUS_MM]`.
- Existing wedge invariants 1-5 continue to hold.

## Risks and Tradeoffs

- **Risk**: regenerating goldens after every C-block packet is operational overhead. **Mitigation**: documented in each packet's commit message; the harness's tolerance gate catches large regressions even when the baseline shifts.
- **Risk**: the curvature threshold (30°) may be too tight for some legitimate sharp turns at the wedge's overhang corners. **Mitigation**: Step 3 empirically picks the threshold; if too tight, document in the test's assertion message and widen.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (smoothing implementation + integration).

## Open Questions

None.
