# Implementation Plan: core-retire

This packet retires two unfalsifiable tests, re-homes one of their intents as a real contrast pair in the `host-algos`-gated prepass target, and records the deltas in the real §7 ledger row.

## Execution Rules

- Work one atomic step at a time; map every step to `core/RETIRE (excluding flow)`.
- Before deleting anything, re-derive the target file's `#[test]` count and roster from disk. The counts quoted here (21, 12, 6) were captured at authoring time and are ledger-style facts that can rot.
- Every Cargo invocation against `algo_prepass_slice_tdd` passes `--features host-algos`, uses an exact filter, checks an anchored one-passed/zero-failed result, and tees combined output to `target/test-output.log`; cargo runs are delegated.
- No step edits production implementation, adds a fixture file or Cargo target, or changes the approved queue. The final step edits only the §7 `core` progress row; the packet author does not edit the plan during generation.

## Steps

### Step 1: Retire the two unfalsifiable tests

- Task IDs: `core/RETIRE (excluding flow)`
- Objective: delete `clip_operation_variants_are_distinct` and `slice_closing_radius_zero_is_noop`, leaving every neighbouring test, helper, import, and section banner intact.
- Precondition: `crates/slicer-core/src/polygon_ops.rs` holds 21 `#[test]` fns and `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` holds 12; each retired name occurs exactly once under `crates/`, at its own definition.
- Postcondition: the counts are 20 and 11; neither name resolves anywhere under `crates/`; `ClipOperation` is still referenced by `clip_polygons`'s `op: ClipOperation` parameter; `apply_slice_closing_radius` and `unit_square_expolygon` each still have exactly two callers inside `triangle_mesh_slicer_tdd.rs`, all within `slice_closing_radius_fuses_gap_within_two_r`; the `// AC-7 / NEG-3` section banner is retained.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/polygon_ops.rs` - the inline `#[cfg(test)] mod tests` only; locate `clip_operation_variants_are_distinct` by symbol, not by line.
  - `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` - the closing-radius section: the banner, `unit_square_expolygon`, `slice_closing_radius_fuses_gap_within_two_r`, and `slice_closing_radius_zero_is_noop`.
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` and `docs/22_test_quality.md` - the earn-their-keep rule and gate rule R1.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/polygon_ops.rs`
  - `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs`
- Files explicitly out of bounds:
  - every production implementation body, including `clip_polygons`, `ClipOperation`, and `apply_slice_closing_radius`
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` (owned by Step 2)
  - `docs/specs/test-quality-remediation-plan.md` (owned by Step 3), the queue, the census JSON, fixtures, manifests, and new targets
- Expected sub-agent dispatches:
  - Question: after the deletions, what is each file's `#[test]` count and full roster in file order, and does either retired name still resolve under `crates/`?; scope: the two edited files plus a `crates/` grep; return: `FACT` with two counts, two rosters, and the grep result.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct read; the burden of proof and both carve-outs.
  - `docs/22_test_quality.md` - direct read of §2.8 and the §5 R1 row.
- OrcaSlicer refs:
  - None; no parity behavior is asserted.
- Verification:
  - AC-1's command - FACT pass/fail.
  - AC-2's command - FACT pass/fail.
- Exit condition: both AC-1 and AC-2 pass, the survivor test still passes, and the dispatched rosters show exactly one name removed from each file and nothing else changed.

### Step 2: Re-home the NEG-3 intent as a contrast pair

- Task IDs: `core/RETIRE (excluding flow)`
- Objective: append `prepass_slice_closing_radius_gate_applies_only_when_positive` and its local fixtures to the gated prepass target so the closing-radius gate is driven for both the zero and positive radius, with independently derived geometry expectations.
- Precondition: `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` holds 6 `#[test]` fns ending with `prepass_slice_caches_bottom_surface_footprint_across_layers`; the target carries `required-features = ["host-algos"]`; `execute_prepass_slice_single_layer` is already imported there.
- Postcondition: the file holds 7 `#[test]` fns with the new one last; it defines `two_island_mesh()` producing a single `ObjectMesh` with `id` `"islands"` whose two disjoint boxes cross `z = 5.0` as the unit squares `x in [0.0, 1.0]` and `x in [1.05, 2.05]`, both `y in [0.0, 1.0]`; it builds a `RegionMapIR` with one entry keyed `RegionKey { global_layer_index: 0, object_id: "islands".to_string(), region_id: 0, variant_chain: Vec::new() }`; it calls `execute_prepass_slice_single_layer` twice with `Some(&region_map)`, differing only in the interned `ResolvedConfig.slice_closing_radius` (`0.0` then `0.04`); the `0.0` arm asserts exactly 2 islands, combined area `200_000_000` unit-squared, and bounds `(0,0)`-`(1,1)` and `(1.05,0)`-`(2.05,1)` mm; the `0.04` arm asserts exactly 1 island with bounds `(0,0)`-`(2.05,1)` mm; the watched-type literals it introduces — `ObjectMesh` (7 named `pub` fields) and `ActiveRegion` (8) — each carry a `..Default::default()` rest, as `cube_mesh` and `make_global_layer` already do; `ResolvedConfig` is a documented watchlist blind spot (macro-generated) and uses the FRU idiom for churn resistance rather than gate compliance.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - imports, `sv`, `p3`, `identity_transform`, `build_volume`, `cube_mesh`, `make_global_layer`, and `slice_at_mid_height_produces_nonempty_polygons` as the call template.
  - `crates/slicer-core/src/algos/prepass_slice.rs` - `execute_prepass_slice_single_layer` and the `RegionKey` construction, `debug_assert!`, and closing-radius gate inside `execute_prepass_slice_single_layer_impl` only.
  - `crates/slicer-ir/src/slice_ir.rs` - by symbol only: `RegionKey`, `RegionMapIR`, `RegionPlan`, `ConfigId`, `config_for`, `intern_config`, `ActiveRegion`, `GlobalLayer`, `SliceIR`, `SlicedRegion`, `Point2::from_mm`.
  - `crates/slicer-ir/src/resolved_config.rs` - the `slice_closing_radius` and `flat_bridge_closing_join` declaration lines only.
  - `crates/slicer-runtime/tests/executor/prepass_slice_and_shell_tdd.rs` - `make_region_map` only, as the intern-and-insert idiom.
  - `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs` - `build_region_map` only, as the in-crate FRU idiom.
  - `docs/21_data_defaults_and_fixtures.md` and `docs/08_coordinate_system.md` - the watched-literal rule and the mm/unit conversion.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`
- Files explicitly out of bounds:
  - all production files, including `prepass_slice.rs` and `triangle_mesh_slicer.rs`
  - the two files edited in Step 1, and every other `slicer-core` test file
  - `docs/specs/test-quality-remediation-plan.md` (owned by Step 3), the census JSON, `crates/slicer-core/Cargo.toml`, fixtures, and new targets
- Expected sub-agent dispatches:
  - Question: does the exact gated filter run one test with zero failures, and does `-- --list` report 7 tests for the target?; scope: `cargo test -p slicer-core --features host-algos --test algo_prepass_slice_tdd`; return: `FACT` pass/fail plus the count, or bounded failure `SNIPPETS` naming the failing assertion.
  - Question: does `cargo xtask check-literals` pass after the new fixture literals?; scope: workspace gate; return: `FACT` pass/fail.
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - direct read of §3 watchlist derivation and §4 waiver format.
  - `docs/08_coordinate_system.md` - direct read of the 100 nm unit and `Point2::from_mm`.
  - `docs/22_test_quality.md` - direct read of §4 question 2; expected values are derived analytically, never by calling the function under test.
- OrcaSlicer refs:
  - None; the gate contract is local and no parity claim is made.
- Verification:
  - AC-3's command - FACT pass/fail.
  - AC-4's command - FACT pass/fail.
  - `set -euo pipefail; cargo test -p slicer-core --features host-algos --test algo_prepass_slice_tdd -- --list 2>&1 | grep -c ': test$'` - FACT: must equal 7; a smaller number means the new test never compiled.
- Exit condition: AC-3 and AC-4 pass, `-- --list` reports 7, `check-literals` exits 0, and the two arms differ only in the interned `slice_closing_radius` value.

### Step 3: Record the retirement deltas in the core ledger row

- Task IDs: `core/RETIRE (excluding flow)`
- Objective: update the existing `core` row only, inside the real §7 Ledger span, preserving accumulated prior content and recording this packet's retirements, replacement, count deltas, validation command, and updated remaining gap.
- Precondition: Steps 1 and 2 have passed their narrow filters, so the three post-edit counts are known from disk rather than from this document; the plan contains `## 7. Ledger (progress record; rows store re-derivable facts, never frozen counts)` followed later by `## Packet Queue`; the queue remains parent-owned.
- Postcondition: exactly one six-column `core` row in that span has state `partial` or `partial (parenthesized detail)`; its changed cell contains `core-retire` and both retired names; its surviving cell contains `prepass_slice_closing_radius_gate_applies_only_when_positive`, `slice_closing_radius_fuses_gap_within_two_r`, and the whitespace-normalized deltas `21->20`, `12->11`, and `6->7`; its Validation cell preserves accumulated prior commands and contains the token-bounded representative invocation `cargo test -p slicer-core --features host-algos --test algo_prepass_slice_tdd -- --exact prepass_slice_closing_radius_gate_applies_only_when_positive`, with optional trailing flags or a closing code-span backtick allowed; its gap cell contains `core-paint`, `core-brittle`, `core-cross`, `core-parity`, and `census-target-scope`, and no longer contains `core-retire`. Any content contributed by `core-strengthen` is preserved verbatim. The next queue heading and all queue rows are unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - the real §7 Ledger heading through the next `## Packet Queue`, including accumulated `core` row content only.
  - `docs/specs/test-quality-remediation-plan.md` - the queue boundary for a no-edit comparison; do not rewrite unrelated plan sections.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` (the single §7 `core` row only)
- Files explicitly out of bounds:
  - the `## Packet Queue` table and every other plan row or section
  - `docs/specs/test-quality-remediation-census.json` - target-scoped and unchanged by this packet
  - all source and test files, fixtures, manifests, generated artifacts, lockfiles, and other packet directories
- Expected sub-agent dispatches:
  - Question: does the real heading-to-queue span contain exactly one updated `core` row satisfying the six-column, partial-state, retired-name, delta-token, representative-command, and gap grammar while preserving prior content and leaving queue text untouched?; scope: `docs/specs/test-quality-remediation-plan.md`; return: `FACT` with the real-plan predicate result.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - delegated ledger and queue contract.
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct read of the census-reconciliation clause that this row satisfies at function granularity.
- OrcaSlicer refs:
  - None.
- Verification:
  - AC-5's real-plan predicate, run against `docs/specs/test-quality-remediation-plan.md` directly; it must print one `core-retire ledger predicate: PASS` line only after the actual row is updated. Do not manufacture a row or use a temporary copy for closure.
  - `set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report 2>&1 | tee target/core-retire-quality.log >/dev/null` - FACT pass/fail.
- Exit condition: the real-plan predicate passes against the implemented row; the plan diff contains only the `core` row inside §7, with no queue change and no census-file change; `core-retire` no longer appears in the remaining-gap cell.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two single-function deletions; roster and count checks. |
| Step 2 | M | New two-island fixture, region-map construction, and two-arm geometry assertions in a gated target. |
| Step 3 | S | One bounded ledger-row edit and the real-plan predicate. |

Aggregate context cost: `M`.

## Packet Completion Gate

- All three steps and exits are complete.
- Every pipe-suffixed acceptance command, including AC-N1, returns PASS through its delegated runtime check.
- The `core` ledger row is updated by the implementation worker through the bounded plan dispatch; AC-5 proves the actual row.
- The queue remains parent-owned and unchanged by this packet's implementation, and `docs/specs/test-quality-remediation-census.json` is byte-identical.
- `packet.spec.md` remains `draft` until independent preflight and later acceptance; no implementation result is claimed here.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and the three packet-level gate commands.
- Record any remaining packet-local risk, in particular the stated falsifiability limit in `design.md`: the r>0 arm proves the gate reads the config, but whether a pure guard deletion is caught depends on an unmeasured `offset(+0)`/`offset(-0)` round-trip.
- Confirm the implementation worker used only the listed files, that no neighbouring test was removed or reordered, and that the ledger row is `partial`, not a premature wave closure.

All `cargo check` and `cargo clippy` invocations in gate and verification commands use `--all-targets`, and every `algo_prepass_slice_tdd` invocation passes `--features host-algos`.
