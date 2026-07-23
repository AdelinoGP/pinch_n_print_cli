# Design: support-planner-smooth-nodes

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs::plan_for_object` — call `smooth_branches` once after the propagation loop completes (between line 742 and line 750), before the final `entries_in_order` emit.
  - `modules/core-modules/support-planner/src/lib.rs::smooth_branches` (NEW function).
  - `modules/core-modules/support-planner/src/lib.rs::group_branches_into_columns` (NEW helper; groups `SupportPlanEntry` rows by `(object_id, region_id)` and sorts each group by `global_layer_index` descending).
- Neighboring tests/fixtures:
  - `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` (NEW) — AC-2, AC-3, AC-N1, AC-N2.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — extended with AC-5's curvature invariant (currently 7 tests; this packet adds the 8th).
  - `resources/golden/support_regression_wedge_branch_count.txt` and `..._endpoints.txt` — regenerated via `SUPPORT_WEDGE_REGEN_GOLDEN=1`.
  - `docs/specs/support-modules-orca-port.md` §Validation Strategy — invariant list extended.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Smoothing operates on mm-valued `f32` coordinates (per the planner's existing convention; the planner uses `na.x` / `na.y` as raw f32 in mm throughout `plan_for_object`).
- The smoothing pass MUST be deterministic. Given identical input entries, the output is identical. No random initialization, no thread-local state.
- The smoothing pass MUST NOT increase the number of `ExtrusionPath3D` records or the number of `points` within any path. Three-point Laplacian replaces interior (x, y) in place.
- Radii (i.e., `Point3WithWidth.width`) are clamped to `[0.0, MAX_BRANCH_RADIUS_MM = 6.0]` after each iteration to preserve the existing invariant.
- Smoothing operates on per-layer (x, y) and width only. Z, role, speed_factor are NOT smoothed (z is determined by the layer plan; role + speed_factor are module-level, not per-point).

## Code Change Surface

- Selected approach: dedicated `smooth_branches` function called once at the tail of `plan_for_object`. The function takes the already-emitted `entries_in_order` (which is moved into a `Vec<SupportPlanEntry>`) and smooths in place.
- Exact functions/structs/tests to change:
  - `support_planner::smooth_branches(entries: &mut Vec<SupportPlanEntry>, iterations: usize)` (new).
  - `support_planner::group_branches_into_columns(entries: &mut [SupportPlanEntry]) -> Vec<Vec<usize>>` (new helper; returns indices into `entries` grouped by `(object_id, region_id)`).
  - `support_planner::plan_for_object` (integration: after line 742, before line 750, call `smooth_branches`).
  - `support_invariants_wedge_tdd::branch_curvature_below_threshold` (new test in the existing harness file).
  - `smooth_nodes_tdd.rs` (new test file with four tests).
  - Goldens (regenerated via `SUPPORT_WEDGE_REGEN_GOLDEN=1`).
  - `docs/specs/support-modules-orca-port.md` (one line in the invariant list).
- Rejected alternatives:
  - **Smooth inline during propagation** — rejected: smoothing is a final-pass operation per Orca's design; mixing it with propagation makes the chain mutation semantics murky (propagation builds the entries; smoothing transforms them after the fact).
  - **Make iteration count a config key** — rejected per Out of Scope; future packet.
  - **Use a higher-order smoothing kernel (Gaussian, cubic spline)** — rejected: deviates from Orca's three-point Laplacian without rationale.
  - **Re-derive the chain structure from the planner's internal `active_nodes` state** — rejected: `active_nodes` is layer-scoped; chain reconstruction from per-layer states is complex and unnecessary. Grouping the already-emitted `SupportPlanEntry` rows by `(object_id, region_id)` is simpler and equivalent.

## Files in Scope (read + edit)

The packet edits 1 source file + 2 new/extended test files + 2 regenerated goldens + 1 doc file (6 total).

- `modules/core-modules/support-planner/src/lib.rs` — role: smoothing + grouping + integration; expected change: ≈80 lines added.
- `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` — role: AC-2, AC-3, AC-N1, AC-N2; expected change: file created.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — role: AC-5 invariant added; expected change: 1 new test function (~40 lines).
- `resources/golden/support_regression_wedge_branch_count.txt` — role: regenerated baseline.
- `resources/golden/support_regression_wedge_endpoints.txt` — role: regenerated baseline.
- `docs/specs/support-modules-orca-port.md` — role: invariant list extension (1 line).

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C3 — directly.
- Existing `plan_for_object` tail (lines 740-756) — range-read.
- Existing 7 invariants in `support_invariants_wedge_tdd.rs` — read for the setup pattern (e.g., `prepare_ctx` and `plan_entries` helpers at lines 11-30).
- `ExtrusionPath3D` and `SupportPlanEntry` definitions in `crates/slicer-ir/src/slice_ir.rs` (lines 1113-1126, 1780-1788) — read for the data shape.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/**`, `Cargo.lock`, generated code — never load.
- `tree-support`, `traditional-support` — consume the smoothed plan via the existing `support_plan_segments_for` path; do not edit.
- Other modules — not edited.
- Other infill / perimeter algorithms — not touched.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `TreeSupport::smooth_nodes` from `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return SUMMARY ≤ 200 words confirming iteration count, formula, endpoint convention." — purpose: validate Orca port.
- "Run `cargo test -p support-planner --test smooth_nodes_tdd`; return FACT pass/fail per-test." — purpose: AC-2, AC-3, AC-N1, AC-N2.
- "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT pass/fail per-test." — purpose: AC-4, AC-5, AC-7.
- "Run `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance`; return FACT (regen happened, file sizes + line counts)." — purpose: re-anchor.
- "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail." — purpose: AC-6 (without env, verifies tolerance).
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE." — purpose: WASM gate.

## Data and Contract Notes

- IR contracts touched: none. `SupportPlanEntry.branch_segments` shape unchanged; values within them shift (x, y, width of interior points are smoothed).
- WIT boundary considerations: none.
- Determinism: smoothing is deterministic.
- Curvature threshold (30°) is encoded in the AC-5 test, not a config key.

## Locked Assumptions and Invariants

- Endpoint indices (highest z and lowest z, i.e., the tip and the root) are never mutated by smoothing.
- Columns shorter than 3 points are no-op.
- Width (radius) after smoothing is clamped `[0.0, MAX_BRANCH_RADIUS_MM = 6.0]`.
- Existing 7 wedge invariants continue to hold.
- Smoothing operates on per-layer (x, y) and width only. z, role, speed_factor are NOT smoothed.

## Risks and Tradeoffs

- **Risk**: regenerating goldens after every C-block packet is operational overhead. **Mitigation**: documented in each packet's commit message; the harness's tolerance gate catches large regressions even when the baseline shifts.
- **Risk**: the curvature threshold (30°) may be too tight for some legitimate sharp turns at the wedge's overhang corners. **Mitigation**: Step 3 empirically picks the threshold; if too tight, document in the test's assertion message and widen.
- **Risk**: a single `SupportPlanEntry.branch_segments` is `Vec<ExtrusionPath3D>`, and each `ExtrusionPath3D` is typically a 2-point segment (one MST edge per layer). The "chain" the smoother operates on is across layers, not within a single `ExtrusionPath3D`. If the implementer confuses these and smooths within a single path, the result is degenerate. **Mitigation**: the `group_branches_into_columns` helper explicitly groups by `(object_id, region_id)` and orders by `global_layer_index`; the test AC-2 / AC-3 exercise a 5-point synthetic column.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (smoothing implementation + integration).
- Highest-risk dispatch: "Re-anchor goldens" — set `SUPPORT_WEDGE_REGEN_GOLDEN=1`, run the test, then unset; the env-var pattern is built into the test at `support_golden_regression_wedge_tdd.rs:65`.

## Open Questions

None.
