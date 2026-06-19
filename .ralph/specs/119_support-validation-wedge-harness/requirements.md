# Requirements: support-validation-wedge-harness

## Packet Metadata

- Grouped task IDs:
  - `TASK-260` — Validation harness on `regression_wedge.stl`: invariants + self-capture golden regression (C1 from `docs/specs/support-modules-orca-port.md`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Subsequent Block C support packets (`121_support-planner-smooth-nodes`, `122_support-planner-multi-neighbour-mst`, `123_support-planner-to-buildplate-pruning`) ship algorithmic changes that can regress branch quality in ways that are invisible to unit tests. Without an oracle, those packets land "against vibes" — exactly the failure mode that bit packet 31b. Real OrcaSlicer reference output is blocked indefinitely on `TASK-163b-orca-ref` (fixture + Orca-runner infrastructure that does not exist). The realistic correctness gate is invariants + self-capture, evaluated on a small engineered fixture.

`resources/regression_wedge.stl` (≈50 KB, ≈45° overhang, deliberate bridge, top + bottom surfaces) is the engineered fixture P0b standardized on. It produces non-trivial support output with default config and is small enough to run inside the integration-test budget.

This packet stands up the harness once. Subsequent C-block packets re-use it without modification. The invariant list is documented as v1; each future C-item that introduces a new invariant adds it to the same test file.

## In Scope

- Author `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` with the six invariant tests + AC-N1 short-circuit case.
- Implement an introspection helper used by the six tests to reconstruct parent-child chains from `SupportPlanIR.entries[*].branch_segments[*]` (shared endpoints between segments establish the implicit MST topology the planner emitted).
- Author `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` with the count-tolerance + Hausdorff comparison.
- Capture `resources/golden/support_regression_wedge_branch_count.txt` (single integer; total `branch_segments` count summed across all entries) by running the planner on the wedge fixture once and writing the value.
- Capture `resources/golden/support_regression_wedge_endpoints.txt` (sorted `(x, y, z)` triples; one per line; mm-valued; `f32` formatted to 6 decimal places) by the same run.
- Add a `tools/` or `xtask` recipe (whichever convention the workspace already uses for golden-regen) that re-captures both files. The recipe is invoked once during initial capture and rarely thereafter (only when a sibling packet's algorithmic change is intentional and re-anchored).
- Add `AC-N2` test that mutates the golden file in-memory (or temp-file overlays the path) and asserts the harness detects the drift — confirms the tolerance arithmetic, not the I/O path.

## Out of Scope

- Test infrastructure for fixtures other than `regression_wedge.stl`. Cube fixtures are paint-pipeline focused.
- Comparison against real OrcaSlicer output. Blocked on `TASK-163b-orca-ref`.
- Adding the future invariants from C3/C4/C5 (curvature, multi-neighbour symmetry, build-plate-only enforcement). Those land with their respective packets.
- Raft entry-count assertion for the non-zero-raft case. Sibling packet `124_support-plan-raft-plan-and-raftinfill-role` lands that.
- Performance benchmarking of the planner on the wedge fixture.
- A GUI-side visualization of the goldens.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` — §C1, §Validation Strategy, §D3. Read directly (≤ 50 lines combined).
- `docs/02_ir_schemas.md` §"SupportPlanIR" — read lines 862-921 directly. The harness asserts against these field paths.
- `crates/slicer-runtime/tests/common/` — existing fixture-loading + slicer-cache helpers. Delegate `LOCATIONS` if > 300 lines.
- `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs` — neighboring integration test for an example of how integration tests bootstrap the runtime + assert against IR; range-read for pattern, do not copy verbatim.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-7` from `packet.spec.md`.
  - AC-1 through AC-6 are the six v1 invariants on the wedge fixture.
  - AC-7 gates the self-capture golden tolerance arithmetic.
- Negative cases: `AC-N1` (empty input short-circuits, not silent-pass on real regression) and `AC-N2` (intentional drift is detected, not silently accepted).
- Cross-packet impact: every subsequent Block C support packet adds an entry to either the invariants file (new invariant) or regenerates the goldens (intentional algorithmic shift). The harness becomes a long-lived shared dependency.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Guest WASM current before running integration tests. | FACT pass/fail |
| `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 \| tee target/test-output.log` | Six invariant gates + AC-N1. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 \| tee target/test-output.log` | Golden tolerance + AC-N2. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `test -s resources/golden/support_regression_wedge_branch_count.txt && test -s resources/golden/support_regression_wedge_endpoints.txt` | Goldens captured and non-empty. | FACT pass/fail |
| `cargo clippy -p slicer-runtime --all-targets -- -D warnings` | Test code lint gate. | FACT pass/fail |

## Step Completion Expectations

- The goldens MUST be captured AFTER `117_support-planner-geometric-correctness` lands. If a sub-agent confirms the tip cone fix has not landed at the time this packet's Step 4 runs, the implementer STOPS and surfaces the dependency violation. The wrong-baseline failure mode is exactly what bit packet 31b.
- The introspection helper in `support_invariants_wedge_tdd.rs` is the only non-trivial code in this packet. It reconstructs parent-child chains from `branch_segments` by treating segments as edges in an undirected graph keyed by `(x, y, z, ε)` and computing connected components. The helper MUST handle the case where the same `(x, y, z)` point appears in multiple segments (which is how the planner records branch merges); the test's chain traversal does not need to disambiguate which segment is "parent" vs "child" — `dist_to_top` monotonicity is asserted against the implicit tree structure.
- The packet does NOT modify `support-planner`. If any AC-1 through AC-6 fails, that is a real regression discovered by the harness; resolution is to fix the planner in the appropriate sibling packet, not to weaken the invariant.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `docs/02_ir_schemas.md` — read only the `SupportPlanIR` section (≤ 80 lines).
  - `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs` — range-read the test setup + IR-assertion pattern (≤ 100 lines).
  - `modules/core-modules/support-planner/src/lib.rs` — NOT in scope. The harness asserts against the IR contract; it does NOT introspect the planner's internals. Do not open the planner source from this packet.
- Likely temptation reads (skip these):
  - `OrcaSlicerDocumented/**` — no Orca behavior is being asserted; the harness is project-internal.
  - `resources/regression_wedge.stl` — binary; sub-agents return summaries (e.g., "the wedge has N triangles, N overhang facets at threshold 45°"); the implementer never opens the STL.
  - All other resource files in `resources/golden/` — not in scope.
- Sub-agent return-format hints for heaviest dispatches:
  - "Run the planner on `regression_wedge.stl` and print the IR field counts (entries.len, branch_segments[*].len, raft_plan.len); return FACT (n entries, m segments, k raft)" — for golden capture. The full IR is NEVER returned through this dispatch.
  - `cargo test -p slicer-runtime --test support_invariants_wedge_tdd` — FACT all-pass / which-test-failed; on fail SNIPPETS ≤ 30 lines with the failing assertion + offending data point.
