# Design: support-validation-wedge-harness

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — NEW file. Six invariant tests + AC-N1 short-circuit.
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` — NEW file. Count tolerance + Hausdorff comparison + AC-N2 drift detector.
  - `resources/golden/support_regression_wedge_branch_count.txt` — NEW file (single integer).
  - `resources/golden/support_regression_wedge_endpoints.txt` — NEW file (sorted f32 triples).
- Neighboring tests/fixtures:
  - `crates/slicer-runtime/tests/common/` — existing fixture-loading helpers; the new tests reuse them.
  - `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs` — pattern reference for runtime-bootstrap-then-assert-IR integration tests.
- OrcaSlicer comparison surface: not consulted. This packet is project-internal validation infrastructure.

## Architecture Constraints

- The harness asserts against `SupportPlanIR` contract surfaces (`entries[*].branch_segments[*]` shape) — NOT against `support-planner` internals. The packet must NOT introspect `crates/slicer-runtime/...` or `modules/core-modules/support-planner/src/lib.rs` to assert behavior; that couples the test to implementation details.
- Goldens are CAPTURED, not authored. The implementer writes a small capture recipe (xtask or shell script — confirm convention via dispatch) that runs the planner once on the wedge fixture and emits the two files. Hand-editing the golden values is a sign the test is wrong.
- Tolerance numbers (`±10%` branch count, `0.5 mm` Hausdorff) come from `docs/specs/support-modules-orca-port.md` §Validation Strategy. The implementer MUST NOT widen them without an explicit packet review.
- The integration tests live under `crates/slicer-runtime/tests/integration/`. The crate-level test bucket (Test Discipline in `CLAUDE.md`) routes them via `tests/integration/main.rs`; the implementer confirms registration in that mod file.
- No new dependency is needed. f32 sorting + Hausdorff distance + branch-count count are pure-stdlib. If the implementer is tempted to pull `approx` or `rstest`, refuse — the tolerance arithmetic is trivial enough to inline.

## Code Change Surface

- Selected approach: two narrow integration test files + two golden text files + one capture recipe (xtask).
- Exact functions/structs/manifests/tests to change:
  - `support_invariants_wedge_tdd.rs` (new) — seven test functions (six invariants + AC-N1) + one introspection helper `fn reconstruct_chains(plan: &SupportPlanIR) -> Vec<Vec<EndpointKey>>` that returns connected components keyed by `(x_rounded, y_rounded, z_rounded)`.
  - `support_golden_regression_wedge_tdd.rs` (new) — two test functions (AC-7 + AC-N2) + one helper `fn endpoints_hausdorff(a: &[Point3], b: &[Point3]) -> f32`.
  - `crates/slicer-runtime/tests/integration/main.rs` — register the two new test modules (one-line additions each).
  - `resources/golden/support_regression_wedge_branch_count.txt` (new) — single integer.
  - `resources/golden/support_regression_wedge_endpoints.txt` (new) — sorted endpoints.
  - `xtask/src/` (or wherever the workspace's golden-regen recipes live) — confirm via dispatch; if no convention exists, add a small `capture_support_goldens` xtask command.
- Rejected alternatives:
  - **Run the planner inline in the golden-regen test and write the files on-the-fly** — rejected: tests should be deterministic across runs, not side-effecting on `resources/`.
  - **Use snapshot-test crates (`insta`, etc.)** — rejected: adds a dependency for tolerance arithmetic that fits in 20 lines of code.
  - **Capture the full `SupportPlanIR` as JSON golden** — rejected: too brittle. The whole point of the tolerance gate is to allow legitimate small drift while catching gross regressions. A JSON snapshot makes every reformat a failure.
  - **Assert invariants on the cube_4color or benchy fixtures** — rejected: cube is paint-pipeline focused (no overhangs), benchy is retired.

## Files in Scope (read + edit)

The packet edits 2 new test files + 2 new golden files + 1 main.rs registration + 1 xtask recipe (6 total). Files are small and disjoint; the count is justified by the natural shape of test-infrastructure packets.

- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` — role: AC-1 through AC-6, AC-N1; expected change: file created.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` — role: AC-7, AC-N2; expected change: file created.
- `crates/slicer-runtime/tests/integration/main.rs` — role: module registration; expected change: 2 lines added.
- `resources/golden/support_regression_wedge_branch_count.txt` — role: golden artifact; expected change: file created.
- `resources/golden/support_regression_wedge_endpoints.txt` — role: golden artifact; expected change: file created.
- `xtask/src/...` (path to be confirmed via Step 1 dispatch) — role: capture recipe; expected change: one new subcommand or one small recipe addition.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C1, §Validation Strategy — read directly.
- `docs/02_ir_schemas.md` §"SupportPlanIR" — read lines 862-921 directly; the test asserts these field paths.
- `crates/slicer-runtime/tests/integration/main.rs` — read fully (small).
- `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs` — range-read for pattern; do not copy verbatim.
- `crates/slicer-runtime/tests/common/` — delegate `LOCATIONS` for fixture-loading helpers.
- `crates/slicer-ir/src/slice_ir.rs` — read only the `SupportPlanIR`, `SupportPlanEntry`, `Point3WithWidth` definitions to confirm field paths.

## Out-of-Bounds Files

- `modules/core-modules/support-planner/src/lib.rs` — NOT read by this packet. The harness asserts against the IR contract.
- `OrcaSlicerDocumented/**` — not consulted.
- `target/`, `Cargo.lock`, generated code — never load.
- `resources/regression_wedge.stl` — binary; delegate any introspection.
- All other crates outside `slicer-runtime` and the new test files.

## Expected Sub-Agent Dispatches

- "Find the workspace's golden-regen recipe convention. Return LOCATIONS ≤ 5 entries showing the `xtask` subcommand pattern OR confirm no existing recipe and surface as a new subcommand." — purpose: confirm Step 5 surface.
- "Locate fixture-loading helpers in `crates/slicer-runtime/tests/common/`; return LOCATIONS for `cached_load_model`, `cached_run` (if they exist) or equivalents." — purpose: bootstrap pattern.
- "Run the planner via a sub-agent on `resources/regression_wedge.stl` with `support_enabled = true` and default other config; return FACT (`SupportPlanIR.entries.len()` value, total `branch_segments` count, first 10 endpoint coordinates as `(x, y, z)` triples). Use the existing integration-test harness pattern (delegate the cargo run; do NOT paste the full IR)." — purpose: Step 5 initial capture.
- "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT (per-test pass/fail) — expected: all GREEN once written correctly. SNIPPETS ≤ 30 lines on failure showing the failing test name + assertion text." — purpose: gate AC-1 through AC-6 + AC-N1.
- "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure." — purpose: gate AC-7 + AC-N2.
- "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`)." — purpose: confirm WASM ready before running integration tests.

## Data and Contract Notes

- IR contracts touched: none. The harness READS `SupportPlanIR` as currently defined.
- WIT boundary considerations: none.
- Determinism: every test assertion is deterministic given a fixed input fixture. The planner is documented as deterministic (`docs/02_ir_schemas.md:917-921`); if the harness reveals non-determinism, that is a real bug and resolution is in the planner, not the harness.
- Tolerance arithmetic: branch count uses signed `(actual - baseline).abs() as f32 / baseline as f32 <= 0.10`. Hausdorff is `max_a∈A min_b∈B dist(a,b)`, then `max(forward, reverse)`.

## Locked Assumptions and Invariants

- `regression_wedge.stl` produces ≥ 5 `SupportPlanIR.entries` and ≥ 10 `branch_segments` total under default-plus-support config. If the wedge is later modified and these counts drop below the threshold, the test is no longer meaningful — surface as a packet-author note in the next packet that touches the wedge.
- The introspection helper rounds coordinates to 6 decimal places (`f32` → `i64` after `* 1_000_000`) for endpoint key equality. Floating-point exact equality is not used.
- Goldens are committed to git as text files (one-line for count, line-per-endpoint for endpoints). They are NOT generated at test time; the test asserts against the committed content.
- The harness does NOT depend on any sibling packet's runtime behavior beyond the planner contract. It IS the gate that sibling packets pass through.

## Risks and Tradeoffs

- **Risk**: the initial golden capture encodes whatever the planner produces post-Packet 2. If Packet 2 (geometric correctness) has not landed, the goldens encode broken behavior. **Mitigation**: Step 4 of the implementation plan EXPLICITLY dispatches a check for Packet 2 closure; if not closed, the packet halts.
- **Risk**: the tolerance gate's `0.5 mm` Hausdorff is sensitive on a small fixture — a 0.5 mm shift on a wedge whose bounding box is ≈ 50 mm is 1% drift. A future C-block change might exceed it for legitimate reasons. **Mitigation**: failure forces re-anchoring (re-capture the goldens with the algorithmic change documented in the commit message). The harness is opinionated about "no silent drift."
- **Risk**: the introspection helper's `f32` rounding to 6 decimal places might not match the planner's emission precision (mm-scale). **Mitigation**: tested at the AC-1 invariant — if rounding causes an invariant to incorrectly fail, the rounding factor is widened in the helper before the goldens are captured.
- **Tradeoff**: hand-rolling Hausdorff is 20 lines but adds maintenance surface vs. pulling `kiddo` or `nalgebra`. Acceptable: 20 lines of pure stdlib code is reviewable; a dependency is forever.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 5 — initial golden capture; requires running the planner and reading its output via dispatch).
- Highest-risk dispatch: golden capture. Required return format: FACT (counts + first-10 endpoints) — NEVER the full IR. If the dispatch returns the full IR it MUST be re-dispatched with tighter scope.

## Open Questions

None.
