# Implementation Plan: core-support-dup-merges

## Execution Rules

- Work one atomic support slice at a time and map it to the approved program wave/item ID.
- Re-derive counts and assertion relationships before deletion; never treat a packet-time count as execution evidence.
- Delegate cargo and OrcaSlicer work; all test output is tee'd to `target/test-output.log` and read there rather than rerun.
- No step may edit more than three files; the two merge steps edit one and two files respectively, and the ledger step edits one bounded row.

## Steps

### Step 1: Fold the coplanar-step subset into the exact layer-and-area survivor

- Task IDs: `core/DUP-CORE (support overhang)`
- Objective: Preserve the RC-1 coplanar-step rationale in `overhang_is_detected_once_at_the_step_layer`, then delete only the weaker duplicate `coplanar_step_does_not_hide_the_contact`.
- Precondition: Fresh source inventory finds exactly 19 `#[test]` fns in file order as listed in `requirements.md`; both candidate tests use `pillar_then_cap()` and `params(45.0, 0.2)`; the absorbed body asserts only non-empty sweep output, while the survivor asserts exact layer `3` and expanded-back area `64.0 mm² ± 0.1 mm²`; the target requires `host-algos`.
- Postcondition: Exactly 18 `#[test]` fns remain; `overhang_is_detected_once_at_the_step_layer` contains the identical fixture, exact `vec![3_usize]` equality, `2.0 * 8.0 * 4.0` expected area, `< 0.1` area tolerance, and RC-1 rationale; the absorbed name is absent; every other 17 test is still discovered once and passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs` - `rect` through `sweep` and the contiguous `overhang_is_detected_once_at_the_step_layer` / `coplanar_step_does_not_hide_the_contact` blocks only.
  - `crates/slicer-core/Cargo.toml` - `support_overhang_detection_tdd` `[[test]]` stanza only.
  - `docs/specs/test-quality-remediation-census.json` - `support_overhang_detection_tdd` entry only.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs` - move/preserve the RC-1 rationale in the survivor and delete only `coplanar_step_does_not_hide_the_contact`.
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/overhang_annotation.rs`, both support-geometry files owned by Step 2, all sibling-packet files, all other crates/modules, and all existing docs.
  - Generated code, vendored dependencies, `target/`, and `Cargo.lock` as edit surfaces.
- Blast-radius discipline:
  - No struct field or schema/version constant changes. Repository-wide exact-name grounding found no test or source consumer of either candidate name beyond its definition; the census manifest records only the target and feature, not a 19-test count. Therefore no additional edit file is authorized.
  - The full 18-name survivor roster is pre-baked into AC-1's discovery/run command; any sibling deletion or rename fails before cargo check can mask it.
- Expected sub-agent dispatches:
  - Question: confirm the current `19`-test inventory and strict-subset relationship on the identical fixture; scope: named helper and two candidate blocks in `support_overhang_detection_tdd.rs`; return: `FACT` with count, names, fixture equality, and assertion relationship.
  - Question: confirm canonical lower-layer growth/difference/expand-back remains the behavior pinned by the survivor; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp`; return: `SUMMARY` ≤200 words, no code.
  - Question: run the feature-correct target and static survivor checks below; scope: Step 1 verification; return: `FACT` pass/fail, ≤20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` §1, §4, §5.1 DUP-CORE support-overhang item, §6, and Packet Queue row #5.
  - `docs/22_test_quality.md` §§1–5 and ADR-0064 - named regression input, independent oracle, and accounted retirement.
  - `docs/08_coordinate_system.md` §The Rule and conversion sections - `Point2::from_mm`, millimeter area, and scaling constraints.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - delegated `detect_overhangs` parity confirmation only.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 18; test "$(grep -c "^overhang_is_detected_once_at_the_step_layer: test$" target/test-output.log || true)" -eq 1; test "$(grep -c "^coplanar_step_does_not_hide_the_contact: test$" target/test-output.log || true)" -eq 0; cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. 18 passed; 0 failed" target/test-output.log; rg -q "test overhang_is_detected_once_at_the_step_layer \.\.\. ok" target/test-output.log'` - FACT pass/fail; exact feature-correct count and survivor execution.
  - `bash -lc 'set -euo pipefail; mkdir -p target; F=crates/slicer-core/tests/support_overhang_detection_tdd.rs; awk "/^fn overhang_is_detected_once_at_the_step_layer\(\)/{flag=1} flag{print} flag&&/^}$/{exit}" "$F" > target/support-overhang-survivor.txt; rg -q -F "vec![3_usize]" target/support-overhang-survivor.txt; rg -q -F "let expected = 2.0 * 8.0 * 4.0;" target/support-overhang-survivor.txt; rg -q -F "(got - expected).abs() < 0.1" target/support-overhang-survivor.txt; rg -q -F "Regression pin for RC-1" target/support-overhang-survivor.txt; ! rg -q "fn coplanar_step_does_not_hide_the_contact" "$F"; test "$(grep -Ec "^#\[test\]" "$F" || true)" -eq 18'` - FACT pass/fail; exact survivor-scoped assertion/rationale preservation and source census.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/support_overhang_detection_tdd.rs 2>&1 | tee target/test-output.log >/dev/null; rg -q -F "check-test-quality: 0 finding(s) in 0 file(s) [report mode]" target/test-output.log'` - FACT pass/fail; touched-file report.
- Exit condition: Stop with failure if the live fixture or assertion relationship differs from the precondition; the command does not discover exactly 18 tests; any named sibling disappears; the absorbed name survives; the survivor loses exact layer, area, tolerance, or RC-1 rationale; canonical confirmation contradicts the retained assertion; or production code/another file would need editing.

### Step 2: Move both inline support-geometry twins to the integration-test survivor home

- Task IDs: `core/DUP-CORE (support geometry)`
- Objective: Preserve all distinct support-geometry cases in `algo_support_geometry_tdd.rs`, enrich the schedule survivor with the inline twins' `got` diagnostics, and delete the complete duplicate `#[cfg(test)] mod tests` from `support_geometry.rs`.
- Precondition: Fresh inventories find exactly three integration tests (`emits_for_2_layer_fixture`, `build_emit_schedule_two_objects_per_object_semantics`, `empty_plan_produces_empty_support`) and exactly two inline tests (`support_geometry_emits_for_2_layer_fixture`, `build_emit_schedule_two_objects_per_object_semantics`); the emission and schedule pairs have identical fixtures/assertion operands; the inline code is a `#[cfg(test)]` module, not a doc example; the target requires `host-algos`.
- Postcondition: The integration file still has exactly three tests in the same order and all pass; emission retains successful/non-empty `entries`; schedule retains exact `obj-A == {1,3,5}` and `obj-B == {0..5}` plus both `got` diagnostics; the empty-plan successful-empty-`entries` witness is unchanged; `support_geometry.rs` has zero tests and no `#[cfg(test)] mod tests`; production code is byte-identical.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/algo_support_geometry_tdd.rs` - full 105-line file.
  - `crates/slicer-core/src/algos/support_geometry.rs` - `build_emit_schedule` / `execute_support_geometry` signatures and `#[cfg(test)] mod tests` only.
  - `crates/slicer-core/Cargo.toml` and `docs/specs/test-quality-remediation-census.json` - `algo_support_geometry_tdd` entries only.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/algo_support_geometry_tdd.rs` - add only `got {a_sched:?}` and `got {b_sched:?}` to the existing schedule assertion messages; retain all three tests/helpers.
  - `crates/slicer-core/src/algos/support_geometry.rs` - delete only the entire `#[cfg(test)] mod tests` block.
- Files explicitly out of bounds:
  - Every production statement/doc in `support_geometry.rs` before `#[cfg(test)] mod tests`, the Step 1 overhang file, all other core tests/source, sibling packet files, other crates/modules, and all existing docs.
  - Generated code, vendored dependencies, `target/`, and `Cargo.lock` as edit surfaces.
- Blast-radius discipline:
  - No struct field or schema/version constant changes. Repository-wide exact-name grounding found no source/test consumers of the inline-only emission name; the same schedule name exists only in the two definitions being consolidated. No test elsewhere asserts these per-file counts or names, and the census manifest records only target/feature registration.
  - The integration-only empty-plan case is explicitly in the allowed verification roster, preventing a superficially correct `3 → 2` drop. No extra edit file is authorized to repair count drift.
- Expected sub-agent dispatches:
  - Question: confirm live `3` integration / `2` inline inventories, test-module (not doc-example) disposition, identical fixture/assertion operands, and the integration-only empty-plan case; scope: the two Step 2 files; return: `FACT` with names and subset/equality result.
  - Question: run the feature-correct target, inline-source census, and closure gates; scope: Step 2 verification; return: `FACT` pass/fail, ≤20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` §1, §4, §5.1 DUP-CORE support-geometry item, §6, and Packet Queue row #5.
  - `docs/22_test_quality.md` §§1–5 and ADR-0064 - union-preserving consolidation and census accountability.
  - `docs/02_ir_schemas.md` §IR 9a `SupportGeometryIR` - object/layer schedule semantics.
- OrcaSlicer refs: None; schedule consolidation preserves existing PnP behavior and does not port canonical code.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; T=crates/slicer-core/tests/algo_support_geometry_tdd.rs; S=crates/slicer-core/src/algos/support_geometry.rs; cargo test -p slicer-core --features host-algos --test algo_support_geometry_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 3; for n in emits_for_2_layer_fixture build_emit_schedule_two_objects_per_object_semantics empty_plan_produces_empty_support; do test "$(grep -c "^${n}: test$" target/test-output.log || true)" -eq 1; done; test "$(grep -Ec "^[[:space:]]*#\[test\]" "$T" || true)" -eq 3; test "$(grep -Ec "^[[:space:]]*#\[test\]" "$S" || true)" -eq 0; ! rg -q "#\[cfg\(test\)\]|fn support_geometry_emits_for_2_layer_fixture|fn build_emit_schedule_two_objects_per_object_semantics" "$S"; cargo test -p slicer-core --features host-algos --test algo_support_geometry_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. 3 passed; 0 failed" target/test-output.log'` - FACT pass/fail; exact integration `3 → 3`, inline `2 → 0`, names, and behavior.
  - `bash -lc 'set -euo pipefail; mkdir -p target; F=crates/slicer-core/tests/algo_support_geometry_tdd.rs; awk "/^fn emits_for_2_layer_fixture\(\)/{flag=1} flag{print} flag&&/^}$/{exit}" "$F" > target/support-geometry-emission-survivor.txt; awk "/^fn build_emit_schedule_two_objects_per_object_semantics\(\)/{flag=1} flag{print} flag&&/^}$/{exit}" "$F" > target/support-geometry-schedule-survivor.txt; awk "/^fn empty_plan_produces_empty_support\(\)/{flag=1} flag{print} flag&&/^}$/{exit}" "$F" > target/support-geometry-empty-survivor.txt; rg -q -F "assert!(result.is_ok());" target/support-geometry-emission-survivor.txt; rg -q -F "assert!(!ir.entries.is_empty());" target/support-geometry-emission-survivor.txt; rg -q -F "[1u32, 3, 5]" target/support-geometry-schedule-survivor.txt; rg -q -F "(0u32..6).collect::<BTreeSet<u32>>()" target/support-geometry-schedule-survivor.txt; rg -q -F "got {a_sched:?}" target/support-geometry-schedule-survivor.txt; rg -q -F "got {b_sched:?}" target/support-geometry-schedule-survivor.txt; rg -q -F "assert!(result.is_ok());" target/support-geometry-empty-survivor.txt; rg -q -F "assert!(result.unwrap().entries.is_empty());" target/support-geometry-empty-survivor.txt'` - FACT pass/fail; exact named-survivor ownership of emitted-entry, schedule, diagnostics, and empty-plan assertions.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/algo_support_geometry_tdd.rs crates/slicer-core/src/algos/support_geometry.rs 2>&1 | tee target/test-output.log >/dev/null; rg -q -F "check-test-quality: 0 finding(s) in 0 file(s) [report mode]" target/test-output.log'` - FACT pass/fail; touched-file report.
  - `cargo check --workspace --all-targets` - FACT pass/fail; proves deletion leaves no test-only import/helper fallout across all targets.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT pass/fail.
- Exit condition: Stop with failure if either pair is not identical in fixture/assertion operands; inline code is a doc example; integration discovery changes from three; the empty-plan case, exact object schedules, successful/non-empty/empty `entries`, or `got` diagnostics are absent; any inline `#[test]` remains; production code changes; or another file would need editing.

### Step 3: Record the partial core ledger evidence

- Task IDs: `core/DUP-CORE (support overhang)`; `core/DUP-CORE (support geometry)`
- Objective: Update only the §7 `core` ledger row with this packet's absorbed/surviving tests, actual validation classes, retained `partial` state, and `core-wall-sequence-dup-merges` as remaining work.
- Precondition: Steps 1 and 2 and their feature-correct verification commands pass; no whole-core closure or Packet Queue implementation edit is justified.
- Postcondition: The sole §7 `core` row has exactly six cells, remains `partial`, records this packet's merge and validations, and names the next remaining core slice; the Packet Queue and every other plan line are unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger table only.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` - §7 `core` row only.
- Files explicitly out of bounds:
  - The Packet Queue and all plan content outside §7's `core` row; `docs/07_implementation_status.md`; all source/test files completed in Steps 1–2; all sibling packet directories and other docs.
- Blast-radius discipline:
  - No code/schema blast radius. The six-column row parser rejects malformed cells and anchors extraction between `## 7. Ledger` and the next `##` heading so queue text cannot satisfy the criterion.
- Expected sub-agent dispatches:
  - Question: verify the proposed diff changes only the §7 `core` row and run AC-7's parser; scope: `docs/specs/test-quality-remediation-plan.md` §7; return: `FACT` pass/fail.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` §4 crate-wave contract and §7 Ledger.
  - `docs/22_test_quality.md` §5 and ADR-0065 - report-mode evidence does not close the full crate wave.
- OrcaSlicer refs: None; this is evidence bookkeeping.
- Verification:
  - `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(encoding=\"utf-8\"); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-support-dup-merges\",\"coplanar_step_does_not_hide_the_contact\",\"overhang_is_detected_once_at_the_step_layer\",\"support_geometry_emits_for_2_layer_fixture\",\"build_emit_schedule_two_objects_per_object_semantics\")); assert all(x in coverage for x in (\"overhang_is_detected_once_at_the_step_layer\",\"emits_for_2_layer_fixture\",\"build_emit_schedule_two_objects_per_object_semantics\",\"empty_plan_produces_empty_support\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-wall-sequence-dup-merges\" in gap; q=t.split(\"## Packet Queue\",1)[1]; qrows=[x for x in q.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; assert [x.strip() for x in qrows[0].split(\"|\")[1:-1]]==[\"#\",\"packet slug\",\"goal (one sentence)\",\"task ids\",\"depends on\",\"status\",\"packet dir\"]; assert any(\"core-support-dup-merges\" in x for x in qrows); print(\"PASS: anchored core ledger row has six partial-state evidence cells; Packet Queue table intact\")"'` - FACT pass/fail; confirms the anchored six-cell partial ledger evidence.
- Exit condition: Stop with failure if the row claims full core closure, has other than six cells, omits this packet's survivor/validation evidence, omits `core-wall-sequence-dup-merges`, or any Packet Queue/other plan content changes.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One bounded integration-test edit plus delegated canonical confirmation. |
| Step 2 | S | Two small test-only edits; one is a `#[cfg(test)]` block in a host-only source module. |
| Step 3 | S | One bounded six-cell ledger-row edit after both code steps pass. |

Aggregate remains `S`: both steps use pre-grounded symbols, bounded windows, and narrow target commands.

## Packet Completion Gate

- Both survivor maps hold at implementation time; any drift stops rather than broadens the packet.
- Feature-correct discovery and execution reconcile overhang `19 → 18`, support integration `3 → 3`, and support inline `2 → 0` with every named survivor passing.
- Exact layer `3`, `64.0 mm² ± 0.1 mm²` area, successful/non-empty/empty `entries`, and `{1,3,5}` / `{0..5}` schedules remain asserted.
- Touched-scope `cargo xtask check-test-quality --report` reports zero findings; `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask check-literals` pass.
- No sibling-packet file, production body, existing doc beyond the §7 `core` ledger row, or guest artifact changes.
- The §7 `core` ledger row remains partial and satisfies AC-7; no Packet Queue or other plan content changes.
- `packet.spec.md` is ready for `status: implemented` only after implementation acceptance; packet generation itself leaves it `draft`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command; return only FACT pass/fail and bounded failure snippets.
- Re-derive all three source/discovery counts immediately before reporting; do not quote packet-time counts as current ledger facts.
- Record any packet-local risk, especially survivor-map drift or an unexpected inline-module boundary.
- Confirm no new production/test symbols are exported and the next packet consumes only queue status.
- Confirm the implementation diff changes only §7's `core` row in the parent plan and leaves Packet Queue generation status to the orchestrator.
