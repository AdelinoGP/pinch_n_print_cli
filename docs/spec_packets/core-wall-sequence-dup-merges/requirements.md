# Requirements: core-wall-sequence-dup-merges

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE (wall sequence)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md` §5.1 DUP-CORE wall-sequence slice
- Packet status: `draft`
- Dependency: `core-support-dup-merges` (queue-generation order only; no exported symbols)
- Aggregate context cost: `S`

## Problem Statement

`perimeter_utils.rs` repeats three wall-sequence tests inside a production file's `#[cfg(test)]` module that the standalone `wall_sequence_reorder_tdd.rs` already asserts strictly stronger on an identical 3-wall fixture: the TDD tests check both loop-type slots and per-slot `perimeter_index` identity, while the inline twins check only `perimeter_index`. Conversely, the inline module owns one case the standalone home lacks — the N==2 `InnerOuterInner` swap — so deleting by file alone would silently lose the two-wall sandwich witness, and deleting by name alone could lose the loop-type assertions or the distinct edge cases.

## In Scope

- Apply this exact survivor map:
  - `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` pre-edit inventory, in file order (`5`): `inner_outer_canonical_order`, `outer_inner_reversed_order`, `inner_outer_inner_sandwich_order`, `empty_walls_is_noop`, `single_wall_unchanged_for_all_modes`.
  - Inline `#[cfg(test)] mod wall_sequence_reorder_tests` inventory in `crates/slicer-core/src/perimeter_utils.rs`, in file order (`4`): `inner_outer_is_canonical_no_reorder`, `outer_inner_reverses`, `inner_outer_inner_sandwich`, `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner`.
  - Absorb `inner_outer_is_canonical_no_reorder` → `inner_outer_canonical_order`, `outer_inner_reverses` → `outer_inner_reversed_order`, and `inner_outer_inner_sandwich` → `inner_outer_inner_sandwich_order`; all three pairs construct the identical `[Outer(0), Inner(1), Inner(2)]` fixture (outer wall first, then two inner walls) and assert the same per-slot `perimeter_index` values; each survivor additionally asserts the loop types, so each survivor strictly subsumes its inline twin.
  - Migrate `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner` into the TDD home unchanged in intent: `[Outer(0), Inner(1)]` sandwich input asserting the `[Inner_0, Outer]` result (`walls[0].perimeter_index == 1`, `walls[1].perimeter_index == 0`), preserving the `For N == 2` result statement in the survivor's doc comment (AC-3 greps for `N == 2` inside the extracted body).
- Post-edit counts: TDD `5 → 6`, inline `4 → 0`; after the merge `perimeter_utils.rs` contains zero `#[test]` attributes and no `#[cfg(test)]` module.
- Keep the production symbols under test unchanged: `slicer_core::perimeter_utils::wall_sequence_reorder` and `slicer_core::perimeter_utils::WallSequence` (all three variants), including the signature's `tree` parameter and its documented M2 grouping fallback.
- Reconcile the source counts and discovery counts. The durable census manifest lists `wall_sequence_reorder_tdd` with `required: []`; the target stays ungated and every command runs without a feature flag. The census records targets/features, not per-function counts.
- After the merge and its gates pass, update only the §7 `core` ledger row in `docs/specs/test-quality-remediation-plan.md`: preserve `partial` state, record this packet's absorbed/surviving tests and actual validations, and name `core-geometry-dup-review` as remaining work without editing the Packet Queue.

## Out of Scope

- Any production statement in `crates/slicer-core/src/perimeter_utils.rs`, including `wall_sequence_reorder`, `WallSequence`, `close_loop`, and every helper above the `#[cfg(test)]` module; the module header comment is retained only as an edit-boundary landmark.
- Every other `slicer-core` test file, especially sibling-packet homes `crates/slicer-core/tests/flow_tdd.rs`, `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`, `crates/slicer-core/tests/algo_region_mapping_tdd.rs`, `crates/slicer-core/tests/support_overhang_detection_tdd.rs`, and `crates/slicer-core/tests/algo_support_geometry_tdd.rs`.
- The `wall_sequence_reorder` call sites in `modules/core-modules/classic-perimeters/src/lib.rs` and `modules/core-modules/arachne-perimeters/src/lib.rs`, all other module and crate code, new tests, renamed survivors, changed fixtures, new helpers, public APIs, IR/WIT/schema/config/manifest changes, and guest WASM artifacts.
- Existing docs except the implementation-time `docs/specs/test-quality-remediation-plan.md` §7 `core` ledger-row edit above; `docs/07_implementation_status.md`, sibling packet directories, the Packet Queue, every other ledger row, and all other documentation remain out of scope. The independent orchestrator owns post-preflight queue status.
- The crate-wide zero-findings wave exit; this packet proves zero findings only in its two touched Rust files.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` §§1–4, §5.1, §§6–7, and Packet Queue - survivor-union, census, feature, wave, and queue requirements.
- `docs/22_test_quality.md` §§1–5 - falsifiable-oracle and report-mode gate standard.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - named-regression-input and accounted-retirement standard.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - report-mode status and crate-wave zero-findings rule.
- `docs/01_system_architecture.md` - wall-sequence range only; `perimeter_utils` (`wall_sequence_reorder`) as the shared helper both perimeter modules call.
- `docs/specs/test-quality-remediation-census.json` - the `wall_sequence_reorder_tdd` entry requires no features; no per-test-function counts are stored there.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — confirm canonical `PerimeterGenerator::process` wall-emission sequencing (outer-first canonical, reversed outer-last, and inner-outer-inner sandwich) remains the behavior pinned by the three surviving order tests; no canonical code is ported or changed.

## Acceptance Summary

- Positive: `AC-1` reconciles TDD `5 → 6` and inline `4 → 0` with feature-correct (ungated) discovery and execution; `AC-2` pins the three mode survivors' exact per-slot identity and loop-type assertions; `AC-3` pins the migrated N==2 two-wall sandwich case; `AC-4` pins the distinct empty and single-wall edge cases; `AC-5` proves ungated registration; `AC-6` proves touched-scope report-mode findings are zero; `AC-7` verifies the partial six-column §7 `core` ledger row.
- Negative: `AC-N1` fails on an unaccounted retirement, renamed/missing survivor, lost assertion fragment, or residual inline definition.
- Cross-packet impact: no symbol or file export. `core-geometry-dup-review` depends only on this packet's independently generated queue status.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --test wall_sequence_reorder_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 6; cargo test -p slicer-core --test wall_sequence_reorder_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. 6 passed; 0 failed" target/test-output.log'` | Ungated discovery and execution reconciliation `5 → 6`. | FACT pass/fail; ≤20 failure lines. |
| `bash -lc 'set -euo pipefail; test "$(grep -Ec "^[[:space:]]*#\[test\]" crates/slicer-core/src/perimeter_utils.rs || true)" -eq 0; if rg -q "#\[cfg\(test\)\]|mod wall_sequence_reorder_tests" crates/slicer-core/src/perimeter_utils.rs; then echo "inline test module still present"; exit 1; fi'` | Inline `4 → 0` disposition and no residual test module. | FACT pass/fail. |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/wall_sequence_reorder_tdd.rs crates/slicer-core/src/perimeter_utils.rs 2>&1 | tee target/test-output.log >/dev/null; rg -q -F "check-test-quality: 0 finding(s) in 0 file(s) [report mode]" target/test-output.log'` | Touched-scope false-green gate. | FACT pass/fail. |
| `cargo check --workspace --all-targets` | Compile every target after deleting the inline module. | FACT pass/fail. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Detect unused imports/helpers and lint regressions. | FACT pass/fail. |
| `cargo xtask check-literals` | Preserve watched fixture-literal discipline. | FACT pass/fail. |
| `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(encoding=\"utf-8\"); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-wall-sequence-dup-merges\",\"inner_outer_is_canonical_no_reorder\",\"outer_inner_reverses\",\"inner_outer_inner_sandwich\",\"inner_outer_inner_with_two_walls_swaps_outer_and_first_inner\")); assert all(x in coverage for x in (\"inner_outer_canonical_order\",\"outer_inner_reversed_order\",\"inner_outer_inner_sandwich_order\",\"inner_outer_inner_with_two_walls_swaps_outer_and_first_inner\",\"empty_walls_is_noop\",\"single_wall_unchanged_for_all_modes\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-geometry-dup-review\" in gap; q=t.split(\"## Packet Queue\",1)[1]; qrows=[x for x in q.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; assert [x.strip() for x in qrows[0].split(\"|\")[1:-1]]==[\"#\",\"packet slug\",\"goal (one sentence)\",\"task ids\",\"depends on\",\"status\",\"packet dir\"]; assert any(\"core-wall-sequence-dup-merges\" in x for x in qrows); print(\"PASS: anchored core ledger row has six partial-state evidence cells; Packet Queue table intact\")'` | Verify the §7 `core` bookkeeping row without touching the Packet Queue. | FACT pass/fail; single PASS line. |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step 2 starts only after Step 1 has reconciled TDD `5 → 6` and inline `4 → 0` and proved the migrated N==2 case.
- Neither step may compensate for a failed count or assertion check by changing production behavior, adding a replacement test, or editing an unrelated test.
- Full-crate `check-test-quality` zero findings remain a later core-wave closure expectation; packet completion requires only the touched-scope zero recorded above.
- The §7 ledger edit occurs only after the merge step passes and may touch only the `core` row; Packet Queue state remains orchestrator-owned.

## Context Discipline Notes

- Read only the named windows of the 1054-line `perimeter_utils.rs` (the `#[cfg(test)]` module block and the `wall_sequence_reorder` signature) and the 122-line TDD file; inventories and survivor mappings above avoid exploratory whole-crate reads.
- OrcaSlicer confirmation is delegated and bounded; no canonical source enters the implementer's context.
- Cargo runs are delegated and always tee test output to `target/test-output.log`; inspect that log rather than rerunning.