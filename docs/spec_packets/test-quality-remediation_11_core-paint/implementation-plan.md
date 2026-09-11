# Implementation Plan: core-paint

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Strengthen driver short-circuit triple

- Task IDs: `core/PAINT`
- Objective: make each `driver_v2_*` test fail on an empty/vacuous fixture while keeping its short-circuit intent and existing name.
- Precondition: the three test names resolve only in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`.
- Postcondition: each body asserts its input slice/regions/polygons are nonempty before the production call and pins the unchanged path after it.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - lines `1881-1920`
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - lines `2081-2130`
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs`
  - `crates/slicer-core/tests/*`
  - `docs/specs/test-quality-remediation-plan.md`
- Expected sub-agent dispatches:
  - Question: run the AC-1 loop and report pass/fail from the log; scope: `cargo test -p slicer-core --features host-algos --lib`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PAINT driver entry plus §6 log discipline
  - `docs/22_test_quality.md` - delegated SUMMARY for R3 non-vacuity
- Verification:
  - `set -euo pipefail; mkdir -p target; for t in driver_v2_empty_mesh_returns_input_slice_ir driver_v2_no_paint_data_short_circuits driver_v2_empty_region_map_short_circuits; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || exit 1; done` - FACT pass/fail
- Exit condition: the AC-1 loop prints one passing test per filter and a bare-feature run of the first filter prints `0 passed`.

### Step 2: Pin prune post-state

- Task IDs: `core/PAINT`
- Objective: replace the no-panic ending of `prune_one_arc_preserves_border_nodes` with the exact `deleted` plus `arc_indices` pins derived from `remove_nodes_with_one_arc`.
- Precondition: Step 1 filter loop passes; the prune name resolves only in `voronoi_prune.rs`.
- Postcondition: the test asserts both node structs survive, the sole arc's `deleted` flag is `true`, and both endpoints' `arc_indices` are empty; MAY/no-panic comments are gone.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` - lines `242-260`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` - lines `539-580`
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`
  - `crates/slicer-core/tests/*`
  - `docs/specs/test-quality-remediation-plan.md`
- Expected sub-agent dispatches:
  - Question: state the deletion rule for a degree-1 interior node joined by a `Border` arc with the supporting lines; scope: `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs`; return: `SNIPPETS`
  - Question: run the AC-2 filter and report pass/fail; scope: cargo only; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 PAINT prune entry
  - `docs/22_test_quality.md` - delegated SUMMARY for R4 success-only
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib prune_one_arc_preserves_border_nodes -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail
- Exit condition: the single prune filter passes and the body contains no `MAY` hedge on the asserted outcome.

### Step 3: Twin Err arm plus probe dispositions

- Task IDs: `core/PAINT`
- Objective: give the twin constructor both an `Ok` and an `Err` witness and close the probe earn-their-keep review with exactly one compile-witness waiver.
- Precondition: Steps 1–2 filters pass; all four affected names resolve only in `voronoi_graph.rs`.
- Postcondition: the retained twin test pins the valid-input `Ok` plus nonempty diagram; the new `from_colored_lines_invalid_input_returns_err_without_panic` feeds first coordinate `2_147_483_648` and pins `Err(MmuGraphError::CoordinateOverflow(2147483648))` with no panic; the color `42`/`0` plus isolation pins and the `is_primary` pin are retained; the builder probe carries a `// test-quality:` waiver in the 8 lines immediately above its fn naming the protected surface.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` - lines `1360-1380`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` - lines `1533-1610`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` - lines `1649-1670`
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs`
  - `crates/slicer-core/tests/*`
  - `docs/specs/test-quality-remediation-plan.md`
- Expected sub-agent dispatches:
  - Question: inventory `return Err` sites in `MMU_Graph::from_colored_lines` and state the smallest reaching bad input; scope: `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`; return: `LOCATIONS`
  - Question: run the AC-3 and AC-4 commands and report pass/fail; scope: cargo only; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/22_test_quality.md` - delegated SUMMARY for R4 plus §3 legitimate weak forms
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - delegated SUMMARY for earn-their-keep
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY for watched-struct rest/waiver on any new literal
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib from_colored_lines_twin_propagation_is_question_mark -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; cargo test -p slicer-core --features host-algos --lib from_colored_lines_invalid_input_returns_err_without_panic -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail
  - `set -euo pipefail; mkdir -p target; for t in boostvoronoi_supports_line_segment_sites vertex_color_get_set_round_trip edge_is_primary_callable; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || exit 1; done; python3 -c 'import pathlib; ls=pathlib.Path("crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs").read_text(encoding="utf-8").splitlines(); i=next(n for n,l in enumerate(ls) if "fn boostvoronoi_supports_line_segment_sites" in l); w="\n".join(ls[max(0,i-8):i]); assert "test-quality:" in w and any(k in w for k in ("boostvoronoi","with_segments","compile")); print("WAIVER-PASS")'` - FACT pass/fail
- Exit condition: both twin filters and all three probe filters print `1 passed`, and the waiver grep succeeds.

### Step 4: Record the §7 core ledger row

- Task IDs: `core/PAINT`
- Objective: append this packet's dispositions and evidence to the accumulated `core` row without touching the queue or any other row.
- Precondition: Steps 1–3 filters pass; the §7 heading and single `core` row exist.
- Postcondition: the `core` row state is `partial`, its cells carry the packet token, all nine names (eight existing plus the new Err test), oracle tokens, representative validation plus four gates, and the remaining gap lists `core-brittle`, `core-cross`, `core-parity`, `census-target-scope` without `core-paint`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger span only
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs`
  - `docs/specs/test-quality-remediation-plan.md` ## Packet Queue span
- Expected sub-agent dispatches:
  - Question: run the AC-5 python predicate and report its verdict; scope: plan file only; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §7 ownership plus Queue row #11 boundary
- Verification:
  - AC-5 python predicate from `packet.spec.md` - prints `core-paint ledger predicate: PASS`
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --lib driver_v2_empty_mesh_returns_input_slice_ir -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 0 passed; 0 failed;' target/test-output.log` - FACT pass/fail for AC-N1
- Exit condition: the predicate prints PASS and the bare-feature control prints `0 passed`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | one test module, three bodies |
| Step 2 | S | one test body plus rule lookup |
| Step 3 | S | one file, twin discovery plus probes |
| Step 4 | S | one ledger row append |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check` and `cargo clippy` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile. `cargo test` invocations use the narrowest target selector instead (`--lib` with a substring filter, or `--test <file>`), which is mutually exclusive with `--all-targets`; the AC commands intentionally select targets this way.
