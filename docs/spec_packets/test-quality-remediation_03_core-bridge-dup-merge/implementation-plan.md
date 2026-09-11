# Implementation Plan: core-bridge-dup-merge

## Execution Rules

- Work one atomic step at a time; map every step to the approved program IDs.
- Confirm the survivor map (duplicate → `solid_underneath_span_produces_no_bridge_area`) before deleting; the deletion is the implementation.
- Each edit step allows at most three files; this packet has one source test home plus one scoped §7 ledger document surface, with Step 1 editing only the test home and Step 2 editing only the ledger row.

## Steps

### Step 1: Delete the duplicate against the pre-grounded survivor map

- Task IDs: `core/DUP-CORE (bridge)`
- Objective: Delete the single duplicate `#[test]` fn `fully_supported_candidate_rejected_zero_bridge_area` from `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`, leaving `solid_underneath_span_produces_no_bridge_area` and the four other distinct guards byte-identical.
- Precondition: The duplicate is textually identical to the survivor (same `region_with_bridge` fixture, same `Some(&[bridge])` fully supported span argument, same `assert!(region.bridge_areas.is_empty());`); discovery is six tests including the duplicate; `bridge_false_site_gating_tdd` is wired with `required-features = ["host-algos"]`.
- Postcondition: The file contains five `#[test]` fns — `solid_underneath_span_produces_no_bridge_area`, `unsupported_span_retains_bridge_area`, `ungated_candidates_cannot_silently_return`, `no_lower_layer_clears_bridge_areas`, `existing_empty_lower_layer_retains_bridge_area` — with all imports, the `square` and `region_with_bridge` helpers, and the `// AC-2:` retention comment intact; the absorbed name appears nowhere in the file; discovery is exactly five tests and all pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` - lines 1–115.
  - `crates/slicer-core/Cargo.toml` - lines 113–117 only.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` - delete only the `fully_supported_candidate_rejected_zero_bridge_area` fn (lines 58–64 at generation time).
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**` and every source/test file other than the one listed edit file.
  - Parent plan, canonical backlog, `docs/07_implementation_status.md`, other packets, generated files, `target/`, and `Cargo.lock`.
- Expected sub-agent dispatches:
  - Question: confirm pre-edit discovery is exactly six tests including the duplicate and post-edit exactly five with the absorbed name absent; scope: `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`; return: `FACT` with the two counts.
  - Question: run the feature-correct exact target checks; scope: commands below; return: `FACT` pass/fail with at most 20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §1, §4, §5.1 DUP-CORE bridge item, §6, Packet Queue row #3.
  - `docs/22_test_quality.md` - §§1–5 census-reconciliation and survivor-map discipline.
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` and `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md`.
- OrcaSlicer refs: None; no canonical source inspection is needed for this pre-existing API test merge.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 5; test "$(grep -c "^fully_supported_candidate_rejected_zero_bridge_area: test$" target/test-output.log || true)" -eq 0; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; for n in solid_underneath_span_produces_no_bridge_area unsupported_span_retains_bridge_area ungated_candidates_cannot_silently_return no_lower_layer_clears_bridge_areas existing_empty_lower_layer_retains_bridge_area; do rg -q "test ${n} \.\.\. ok" target/test-output.log; done'` - FACT pass/fail; proves exact five-count discovery, the absorbed name absent from discovery, and every named survivor pass line.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/bridge_false_site_gating_tdd.rs 2>&1 | tee target/test-output.log >/dev/null; ! rg -q "crates/slicer-core/tests/bridge_false_site_gating_tdd.rs" target/test-output.log'` - FACT pass/fail; touched-scope report is filtered and does not claim other core findings are closed.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT pass/fail.
- Exit condition: Exit with failure if the duplicate differs textually from `solid_underneath_span_produces_no_bridge_area` in fixture, span argument, or assertion (do not silently change either body — stop per `design.md`'s `[BLOCK]`), discovery is not exactly five post-edit, the absorbed name survives in file or discovery, any named survivor fails, or a helper/import is left unused. Do not expand the edit surface.

### Step 2: Record the mandatory partial core ledger evidence

- Task IDs: `core/DUP-CORE (bridge)`
- Objective: Update only §7's `core` ledger row with the bridge slice's partial status, the survivor map, surviving coverage, actual validations, and the remaining `core-region-mapping-dup-merges` gap.
- Precondition: Step 1's bridge merge and gates pass; no whole-core closure is justified.
- Postcondition: The §7 `core` row records the bridge slice evidence and explicitly leaves non-bridge core work open. The row must satisfy the AC-5 ledger parser command in `packet.spec.md`, repeated verbatim in this step's Verification below.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger only.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` - §7 `core` row only (per the AC-5 ledger parser command in `packet.spec.md`).
- Files explicitly out of bounds:
  - Both source/test homes (the edit file above), queue sections, other waves, `docs/07_implementation_status.md`, and every other packet.
- Expected sub-agent dispatches: None; parent-provided facts govern this bookkeeping edit.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §4 exit-gate distinction and §7 ledger.
  - `docs/22_test_quality.md` - evidence/quality standard.
- OrcaSlicer refs: None.
- Verification:
  - `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-bridge-dup-merge\",\"fully_supported_candidate_rejected_zero_bridge_area\",\"solid_underneath_span_produces_no_bridge_area\",\"bridge_false_site_gating_tdd\")); assert all(x in coverage for x in (\"unsupported_span_retains_bridge_area\",\"ungated_candidates_cannot_silently_return\",\"existing_empty_lower_layer_retains_bridge_area\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-region-mapping-dup-merges\" in gap and \"non-bridge\" in gap; print(\"PASS: anchored core ledger row has six validated cells\")"'` - FACT pass/fail; confirms exactly one §7 `core` row, exact six-column shape, and all required evidence cells.
- Exit condition: Exit with failure if the row claims the entire core wave is closed, rewrites queue/other-wave state, freezes audit counts, omits actual validation evidence, or omits the remaining `core-region-mapping-dup-merges` gap.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One bounded 115-line test file plus manifest wiring; no production or external canonical read. |
| Step 2 | S | One bounded §7 ledger row; no queue or other-wave edits. |

## Packet Completion Gate

- The duplicate has a named survivor (`solid_underneath_span_produces_no_bridge_area`) and all five distinct guards survive with exact assertion strength.
- Exact feature-correct target discovery is exactly five, all targeted tests pass with tee'd output, and the absorbed name is absent.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and touched-scope `check-test-quality --report` pass.
- Before/after target discovery and execution census are reconciled from fresh commands (6 → 5, the one accounted retirement); no packet-time counts are copied as current truth.
- The implementation ledger update is mandatory and limited to §7's `core` row; it records partial bridge evidence and the remaining non-bridge gap, without closing the core wave or changing queue/other-wave state.
- The ledger row must satisfy the AC-5 ledger parser command in `packet.spec.md`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record any remaining packet-local risk, especially whether the duplicate's textual-identity premise held at implementation time.
- Keep `core-region-mapping-dup-merges` and all other core work pending as applicable; do not claim the wave-0 core zero-findings gate is closed until the full core wave reaches its own exit gate.