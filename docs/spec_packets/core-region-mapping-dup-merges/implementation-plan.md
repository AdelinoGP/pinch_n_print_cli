# Implementation Plan: core-region-mapping-dup-merges

## Execution Rules

- Work one atomic step at a time; map every step to the approved program IDs.
- Confirm the survivor map (both duplicates → `region_mapping_two_semantics_produces_cross_product_cardinality`) before deleting; the deletion plus the survivor doc-comment extension is the implementation.
- Each edit step allows at most three files; this packet has one source test home plus one scoped §7 ledger document surface, with Step 1 editing only the test home and Step 2 editing only the ledger row.

## Steps

### Step 1: Delete both duplicates against the pre-grounded survivor map and extend the survivor's doc comment

- Task IDs: `core/DUP-CORE (region mapping)`
- Objective: Delete the two duplicate `#[test]` fns `region_mapping_cross_product_entry_count` and `region_mapping_enumerate_chains` from `crates/slicer-core/tests/algo_region_mapping_tdd.rs`, leaving the survivor `region_mapping_two_semantics_produces_cross_product_cardinality` and the 21 distinct tests byte-identical except for the survivor's doc-comment extension, which absorbs the candidate-unique AC-4 formula prose (the AC-3 enumeration wording is already on the survivor and is retained verbatim).
- Precondition: Both duplicates are strict assertion subsets of the survivor on the identical fixture (the survivor's body already contains `assert_eq!(region_map.entries.len(), 6);` and the six-chain `HashSet` equality with the `"expected cross-product chains"` message); discovery is 24 tests including both duplicates; `algo_region_mapping_tdd` is wired with `required-features = ["host-algos"]`.
- Postcondition: The file contains 22 `#[test]` fns — the survivor plus the 21 distinct tests — with all imports and helpers intact; both absorbed names appear nowhere in the file; the survivor's doc comment retains the absorbed AC-4 formula prose (`entries.len() == layers × active_regions × ∏(1 + K_i)`) and its pre-existing AC-3 enumeration wording (`SET membership of the enumerated chains`); discovery is exactly 22 tests and all pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/algo_region_mapping_tdd.rs` - lines 1–200, 1025–1150, and 1430–1524.
  - `crates/slicer-core/Cargo.toml` - lines 58–60 only.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/algo_region_mapping_tdd.rs` - delete only the two `region_mapping_cross_product_entry_count` and `region_mapping_enumerate_chains` fns (lines 1432–1466 and 1468–1524 at generation time) and extend the survivor's doc comment (lines 1025–1027 at generation time).
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**` and every source/test file other than the one listed edit file.
  - Parent plan, canonical backlog, `docs/07_implementation_status.md`, other packets, generated files, `target/`, and `Cargo.lock`.
- Blast-radius discipline: not applicable — no struct field, schema constant, or public constant is added or bumped; the deletion removes whole test functions and the only other edit is a doc comment. The step still verifies no import or helper becomes unused via the compile gates below.
- Expected sub-agent dispatches:
  - Question: confirm pre-edit discovery is exactly 24 tests including both duplicates and post-edit exactly 22 with both absorbed names absent; scope: `crates/slicer-core/tests/algo_region_mapping_tdd.rs`; return: `FACT` with the two counts.
  - Question: confirm both duplicates' bodies are strict assertion subsets of the survivor's body; scope: lines 1025–1150 and 1430–1524 of the target file; return: `FACT` listing which assertion lines are subsets.
  - Question: run the feature-correct exact target checks and gates; scope: commands below; return: `FACT` pass/fail with at most 20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §1, §4, §5.1 DUP-CORE region-mapping item, §6, Packet Queue row #4.
  - `docs/22_test_quality.md` - §§1–5 census-reconciliation and survivor-map discipline; §2.1 names the survivor as the repaired-shape example.
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` and `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md`.
  - `docs/spec_packets/_OLD/93_region-mapping-cross-product.md` - lines 32–56; AC-alias provenance for the retained formula prose.
- OrcaSlicer refs: None; no canonical source inspection is needed for this pre-existing API test merge.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test algo_region_mapping_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 22; test "$(grep -c "^region_mapping_enumerate_chains: test$" target/test-output.log || true)" -eq 0; test "$(grep -c "^region_mapping_cross_product_entry_count: test$" target/test-output.log || true)" -eq 0; cargo test -p slicer-core --features host-algos --test algo_region_mapping_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. 22 passed; 0 failed" target/test-output.log; rg -q "test region_mapping_two_semantics_produces_cross_product_cardinality \.\.\. ok" target/test-output.log; rg -q "test region_mapping_chains_ordered_by_aggregated_region_split_canonical_order \.\.\. ok" target/test-output.log'` - FACT pass/fail; proves exact 22-count discovery, both absorbed names absent from discovery, the full target passing, and the survivor plus ordering witness pass lines.
  - `bash -lc 'set -euo pipefail; F=crates/slicer-core/tests/algo_region_mapping_tdd.rs; rg -q -F "assert_eq!(region_map.entries.len(), 6);" "$F"; rg -q -F "assert_eq!(actual, expected, \"expected cross-product chains\");" "$F"; rg -q -F "entries.len() == layers" "$F"; rg -q -F "SET membership of the enumerated chains" "$F"; test "$(rg -c -F "entries.len() == layers" "$F" || echo 0)" -eq 1; test "$(grep -c "^#\[test\]" "$F" || true)" -eq 22; ! rg -q "region_mapping_enumerate_chains|region_mapping_cross_product_entry_count" "$F"'` - FACT pass/fail; proves the survivor's union assertions and the absorbed AC-4 formula prose survive in the file (the formula in exactly one place, the survivor), both absorbed names are gone, and the pre-existing AC-3 wording is retained.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/algo_region_mapping_tdd.rs 2>&1 | tee target/test-output.log >/dev/null; ! rg -q "crates/slicer-core/tests/algo_region_mapping_tdd.rs" target/test-output.log'` - FACT pass/fail; touched-scope report is filtered and does not claim other core findings are closed.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT pass/fail.
- Exit condition: Exit with failure if either duplicate differs from its packet-time premise — a body that is not a strict assertion subset of the survivor, a divergent fixture, or an absorbed assertion not present in the survivor (do not silently change either body — stop per `design.md`'s `[BLOCK]`); discovery is not exactly 22 post-edit; either absorbed name survives in file or discovery; any distinct test fails; the survivor's union assertions, its pre-existing AC-3 wording, or the absorbed AC-4 formula prose is missing; or a helper/import is left unused. Do not expand the edit surface.

### Step 2: Record the mandatory partial core ledger evidence

- Task IDs: `core/DUP-CORE (region mapping)`
- Objective: Update only §7's `core` ledger row with the region-mapping slice's partial status, the survivor map, surviving coverage, actual validations, and the next pending `core-support-dup-merges` gap.
- Precondition: Step 1's region-mapping merge and gates pass; no whole-core closure is justified.
- Postcondition: The §7 `core` row records the region-mapping slice evidence and explicitly leaves non-region-mapping core work open. The row must satisfy the AC-4 ledger parser command in `packet.spec.md`, repeated verbatim in this step's Verification below.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger only.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` - §7 `core` row only (per the AC-4 ledger parser command in `packet.spec.md`).
- Files explicitly out of bounds:
  - Both source/test homes (the edit file above), queue sections, other waves, `docs/07_implementation_status.md`, and every other packet.
- Expected sub-agent dispatches: None; parent-provided facts govern this bookkeeping edit.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §4 exit-gate distinction and §7 ledger.
  - `docs/22_test_quality.md` - evidence/quality standard.
- OrcaSlicer refs: None.
- Verification:
  - `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-region-mapping-dup-merges\",\"region_mapping_enumerate_chains\",\"region_mapping_cross_product_entry_count\",\"region_mapping_two_semantics_produces_cross_product_cardinality\",\"algo_region_mapping_tdd\")); assert all(x in coverage for x in (\"region_mapping_two_semantics_produces_cross_product_cardinality\",\"region_mapping_chains_ordered_by_aggregated_region_split_canonical_order\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-support-dup-merges\" in gap; print(\"PASS: anchored core ledger row has six validated cells\")"'` - FACT pass/fail; confirms exactly one §7 `core` row, exact six-column shape, and all required evidence cells.
- Exit condition: Exit with failure if the row claims the entire core wave is closed, rewrites queue/other-wave state, freezes audit counts, omits actual validation evidence, or omits the remaining `core-support-dup-merges` gap.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Three bounded ranges of one 1524-line test file plus manifest wiring; no production or external canonical read. |
| Step 2 | S | One bounded §7 ledger row; no queue or other-wave edits. |

## Packet Completion Gate

- Both duplicates have a named survivor (`region_mapping_two_semantics_produces_cross_product_cardinality`) and all 21 distinct tests survive with exact assertion strength.
- Exact feature-correct target discovery is exactly 22, all targeted tests pass with tee'd output, and both absorbed names are absent.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and touched-scope `check-test-quality --report` pass.
- Before/after target discovery and execution census are reconciled from fresh commands (24 → 22, the two accounted retirements); no packet-time counts are copied as current truth.
- The implementation ledger update is mandatory and limited to §7's `core` row; it records partial region-mapping evidence and the next pending `core-support-dup-merges` gap, without closing the core wave or changing queue/other-wave state.
- The ledger row must satisfy the AC-4 ledger parser command in `packet.spec.md`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record any remaining packet-local risk, especially whether the duplicates' strict-subset premise held at implementation time.
- Keep `core-support-dup-merges` and all other core work pending as applicable; do not claim the wave-0 core zero-findings gate is closed until the full core wave reaches its own exit gate.
