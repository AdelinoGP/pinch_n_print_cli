# Implementation Plan: core-wall-sequence-dup-merges

## Execution Rules

- Work one atomic wall-sequence step at a time and map it to the approved program wave/item ID.
- Re-derive counts and assertion relationships before deletion; never treat a packet-time count as execution evidence.
- Delegate cargo and OrcaSlicer work; all test output is tee'd to `target/test-output.log` and read there rather than rerun.
- No step may edit more than three files; the merge step edits two files, and the ledger step edits one bounded row.

## Steps

### Step 1: Migrate the N==2 sandwich case and delete the inline twin module

- Task IDs: `core/DUP-CORE (wall sequence)`
- Objective: Add `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner` to `wall_sequence_reorder_tdd.rs`, then delete only the complete `#[cfg(test)] mod wall_sequence_reorder_tests` block from `perimeter_utils.rs`.
- Precondition: Fresh source inventory finds exactly 5 `#[test]` fns in `wall_sequence_reorder_tdd.rs` in file order as listed in `requirements.md`, and exactly 4 in the inline `#[cfg(test)] mod wall_sequence_reorder_tests` of `perimeter_utils.rs`; the three absorbed pairs construct the identical `[Outer(0), Inner(1), Inner(2)]` fixture and each survivor asserts strictly more (loop types plus per-slot identity) than its inline twin; the target is ungated (census `required: []`, no `required-features`).
- Postcondition: Exactly 6 `#[test]` fns remain in the TDD file (the 5 pre-edit names plus the migrated one, whose doc comment contains `N == 2`); `perimeter_utils.rs` contains zero `#[test]` attributes, no `#[cfg(test)]` module, and byte-identical production code; every TDD test is discovered once and passes ungated.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` - full 122-line file.
  - `crates/slicer-core/src/perimeter_utils.rs` - `wall_sequence_reorder` signature, `WallSequence` enum, and the `#[cfg(test)]` block (lines 886–1054) only.
  - `crates/slicer-core/Cargo.toml` - the `wall_sequence_reorder_tdd` `[[test]]` stanza only.
  - `docs/specs/test-quality-remediation-census.json` - the `wall_sequence_reorder_tdd` entry only.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` - add only the migrated N==2 test using the existing `make_wall` helper.
  - `crates/slicer-core/src/perimeter_utils.rs` - delete only the entire `#[cfg(test)] mod wall_sequence_reorder_tests` block (including the inline `make_wall`/`wall_loop_base` helpers and the module's `use slicer_ir::{...}` import).
- Files explicitly out of bounds:
  - All production statements in `perimeter_utils.rs` above the test module, the `wall_sequence_reorder` call sites in `modules/core-modules/classic-perimeters/src/lib.rs` and `modules/core-modules/arachne-perimeters/src/lib.rs`, all sibling-packet files, all other crates/modules, and all existing docs.
  - Generated code, vendored dependencies, `target/`, and `Cargo.lock` as edit surfaces.
- Blast-radius discipline:
  - No struct field or schema/version constant changes. Repository-wide exact-name grounding found the three absorbed inline names and `mod wall_sequence_reorder_tests` referenced only inside `perimeter_utils.rs` itself; no test elsewhere asserts these per-file counts or names, and the census manifest records only the target and its empty feature list. Therefore no additional edit file is authorized.
  - The migrated N==2 test is explicitly in the allowed verification roster, preventing a superficially correct `6 → 5` drop; the two distinct edge cases are roster-pinned so an edge-case loss fails before cargo check can mask it.
- Expected sub-agent dispatches:
  - Question: confirm the current `5`-test TDD / `4`-test inline inventories and the strict-subset relationship on the identical fixture; scope: `wall_sequence_reorder_tdd.rs` and the inline module block in `perimeter_utils.rs`; return: `FACT` with counts, names, fixture equality, and assertion relationship.
  - Question: confirm canonical wall-emission sequencing remains the behavior pinned by the survivors; scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SUMMARY` ≤200 words, no code.
  - Question: run the ungated target and static survivor checks below; scope: Step 1 verification; return: `FACT` pass/fail, ≤20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` §1, §4, §5.1 DUP-CORE wall-sequence item, §6, and Packet Queue row #6.
  - `docs/22_test_quality.md` §§1–5 and ADR-0064 - union-preserving consolidation, named regression input, and accounted retirement.
  - `docs/01_system_architecture.md` wall-sequence range - shared-helper contract consumed by both perimeter modules.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegated `PerimeterGenerator::process` parity confirmation only.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --test wall_sequence_reorder_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 6; for n in inner_outer_canonical_order outer_inner_reversed_order inner_outer_inner_sandwich_order empty_walls_is_noop single_wall_unchanged_for_all_modes inner_outer_inner_with_two_walls_swaps_outer_and_first_inner; do test "$(grep -c "^${n}: test$" target/test-output.log || true)" -eq 1; done; test "$(grep -Ec "^[[:space:]]*#\[test\]" crates/slicer-core/src/perimeter_utils.rs || true)" -eq 0; if rg -q "#\[cfg\(test\)\]|mod wall_sequence_reorder_tests" crates/slicer-core/src/perimeter_utils.rs; then echo "inline test module still present"; exit 1; fi; cargo test -p slicer-core --test wall_sequence_reorder_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. 6 passed; 0 failed" target/test-output.log'` - FACT pass/fail; exact ungated `6` count, names, inline `4 → 0`, and execution.
  - `bash -lc 'set -euo pipefail; F=crates/slicer-core/tests/wall_sequence_reorder_tdd.rs; awk "/^fn inner_outer_inner_with_two_walls_swaps_outer_and_first_inner\(\)/{flag=1} flag{print} flag&&/^}$/{exit}" "$F" > target/wall-seq-two-wall.txt; test -s target/wall-seq-two-wall.txt; rg -q -F "make_wall(0, LoopType::Outer" target/wall-seq-two-wall.txt; rg -q -F "make_wall(1, LoopType::Inner" target/wall-seq-two-wall.txt; rg -q -F "WallSequence::InnerOuterInner" target/wall-seq-two-wall.txt; rg -q -F "walls[0].perimeter_index, 1" target/wall-seq-two-wall.txt; rg -q -F "walls[1].perimeter_index, 0" target/wall-seq-two-wall.txt; rm -f target/wall-seq-two-wall.txt; test "$(grep -Ec "^#\[test\]" "$F" || true)" -eq 6'` - FACT pass/fail; migrated N==2 case ownership and TDD source census.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/wall_sequence_reorder_tdd.rs crates/slicer-core/src/perimeter_utils.rs 2>&1 | tee target/test-output.log >/dev/null; rg -q -F "check-test-quality: 0 finding(s) in 0 file(s) [report mode]" target/test-output.log'` - FACT pass/fail; touched-file report.
  - `cargo check --workspace --all-targets` - FACT pass/fail; proves deletion leaves no test-only import/helper fallout across all targets.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT pass/fail.
- Exit condition: Stop with failure if the live fixture or assertion relationship differs from the precondition; the command does not discover exactly 6 tests; any pre-edit TDD test disappears or is renamed; the migrated name is absent or duplicated; any inline `#[test]` or `#[cfg(test)]` module survives in `perimeter_utils.rs`; production code changes; canonical confirmation contradicts a retained order witness; or another file would need editing.

### Step 2: Record the partial core ledger evidence

- Task IDs: `core/DUP-CORE (wall sequence)`
- Objective: Update only the §7 `core` ledger row with this packet's absorbed/surviving tests, actual validation classes, retained `partial` state, and `core-geometry-dup-review` as remaining work.
- Precondition: Step 1 and its verification commands pass; no whole-core closure or Packet Queue implementation edit is justified.
- Postcondition: The sole §7 `core` row has exactly six cells, remains `partial`, records this packet's merge and validations, and names the next remaining core slice; the Packet Queue and every other plan line are unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger table only.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` - §7 `core` row only.
- Files explicitly out of bounds:
  - The Packet Queue and all plan content outside §7's `core` row; `docs/07_implementation_status.md`; all source/test files completed in Step 1; all sibling packet directories and other docs.
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
  - `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(encoding=\"utf-8\"); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-wall-sequence-dup-merges\",\"inner_outer_is_canonical_no_reorder\",\"outer_inner_reverses\",\"inner_outer_inner_sandwich\",\"inner_outer_inner_with_two_walls_swaps_outer_and_first_inner\")); assert all(x in coverage for x in (\"inner_outer_canonical_order\",\"outer_inner_reversed_order\",\"inner_outer_inner_sandwich_order\",\"inner_outer_inner_with_two_walls_swaps_outer_and_first_inner\",\"empty_walls_is_noop\",\"single_wall_unchanged_for_all_modes\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-geometry-dup-review\" in gap; q=t.split(\"## Packet Queue\",1)[1]; qrows=[x for x in q.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; assert [x.strip() for x in qrows[0].split(\"|\")[1:-1]]==[\"#\",\"packet slug\",\"goal (one sentence)\",\"task ids\",\"depends on\",\"status\",\"packet dir\"]; assert any(\"core-wall-sequence-dup-merges\" in x for x in qrows); print(\"PASS: anchored core ledger row has six partial-state evidence cells; Packet Queue table intact\")"'` - FACT pass/fail; single PASS line.
- Exit condition: Stop with failure if the row claims full core closure, has other than six cells, omits this packet's survivor/validation evidence, omits `core-geometry-dup-review`, or any Packet Queue/other plan content changes.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two bounded test-only edits; the source edit deletes only the file-tail `#[cfg(test)]` block. |
| Step 2 | S | One bounded six-cell ledger-row edit after the code step passes. |

Aggregate remains `S`: the step uses pre-grounded symbols, bounded windows, and a narrow ungated target command.

## Packet Completion Gate

- The survivor map holds at implementation time; any drift stops rather than broadens the packet.
- Ungated discovery and execution reconcile TDD `5 → 6` and inline `4 → 0` with every named survivor passing.
- The three mode orderings (`[0,1,2]`, `[2,1,0]`, `[1,0,2]`), the N==2 `[1,0]` sandwich swap, and the empty/single-wall edge cases remain asserted.
- Touched-scope `cargo xtask check-test-quality --report` reports zero findings; `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask check-literals` pass.
- No sibling-packet file, production body, perimeter-module call site, existing doc beyond the §7 `core` ledger row, or guest artifact changes.
- The §7 `core` ledger row remains partial and satisfies AC-7; no Packet Queue or other plan content changes.
- `packet.spec.md` is ready for `status: implemented` only after implementation acceptance; packet generation itself leaves it `draft`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command; return only FACT pass/fail and bounded failure snippets.
- Re-derive both source/discovery counts immediately before reporting; do not quote packet-time counts as current ledger facts.
- Record any packet-local risk, especially survivor-map drift or an unexpected inline-module boundary.
- Confirm no new production/test symbols are exported and the next packet consumes only queue status.
- Confirm the implementation diff changes only §7's `core` row in the parent plan and leaves Packet Queue generation status to the orchestrator.