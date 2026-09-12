# Implementation Plan: core-flow-consolidation

## Execution Rules

- Work one atomic step at a time; map every step to the approved program IDs.
- Preserve tests first, then remove the inline owner, then run the narrowest falsifying validation.
- Each edit step allows at most three files; this packet has two source test homes plus one scoped §7 ledger document surface, with Step 1 editing only the two source test homes and Step 2 editing only the ledger row.

## Steps

### Step 1: Build and apply the flow survivor map

- Task IDs: `core/DUP-CORE (flow)`, `core/RETIRE (flow)`, `core/DUP-CORE-strengthen (wider-bead)`
- Objective: Compare the ten inline cases with the integration cases, then relocate every inline-only assertion to `flow_tdd`, merge exact duplicates without losing inputs, and preserve and verify the existing wider-bead independent-formula oracle and strict wider-than-canonical comparison.
- Precondition: `flow_tdd` is the public integration target; `flow` APIs and `RoleWidthContext` fields are publicly accessible; `flow_correction_stays_positive_for_vertical_input` is confirmed out of scope.
- Postcondition: `flow_tdd` contains the union of all distinct flow cases, including `(0.4, 1.0)` spacing `0.1854 ± 1e-3` and threshold `boundary * 0.5` rejection, preserves and verifies the existing wider-bead independent-formula oracle and strict comparison, and `flow.rs` contains no inline test module or `#[test]`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/flow.rs` - public flow API and inline `#[cfg(test)] mod tests` only.
  - `crates/slicer-core/tests/flow_tdd.rs` - complete integration test file only.
  - `crates/slicer-core/Cargo.toml` - `[features]` table and complete `[[test]]` stanza named `flow_tdd` only.
  - `crates/slicer-core/src/lib.rs` - symbol lookup only for the excluded vertical correction test.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/flow.rs` - delete only the inline `#[cfg(test)] mod tests` block.
  - `crates/slicer-core/tests/flow_tdd.rs` - add/merge survivor tests and the independent wider-bead oracle.
- Files explicitly out of bounds:
  - `crates/slicer-core/src/lib.rs` and every source/test file other than the two listed edit files.
  - Parent plan, canonical backlog, `docs/07_implementation_status.md`, other packets, generated files, `target/`, and `Cargo.lock`.
- Expected sub-agent dispatches:
  - Question: produce the exact inline-to-integration survivor crosswalk before deletion; scope: the two edit files; return: `SUMMARY` ≤200 words.
  - Question: verify public visibility and target wiring after edits; scope: `flow.rs`, `flow_tdd.rs`, `Cargo.toml`; return: `LOCATIONS` ≤20 entries.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §1, §4, §5.1, §6, `core-flow-consolidation` queue entry and boundary.
  - `docs/22_test_quality.md` - §§1–5.
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` and `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md`.
- OrcaSlicer refs: None; no canonical source inspection is needed for this pre-existing API test move.

#### Discovery and reconciliation procedure

- Command IDs `packet01-inline-list-before`, `packet01-inline-exec-before`, `packet01-integration-list-before`, and `packet01-integration-exec-before` are already-successful baseline commands. Their copied logs exist at `target/packet01-inline-list-before.log`, `target/packet01-inline-exec-before.log`, `target/packet01-integration-list-before.log`, and `target/packet01-integration-exec-before.log`; inspect those logs and do not rerun or overwrite them.
- `packet01-inline-list-before`:

  ```bash
  bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib flow::tests:: -- --list 2>&1 | tee target/test-output.log >/dev/null; cp -f target/test-output.log target/packet01-inline-list-before.log; rg -q ": test$" target/packet01-inline-list-before.log'
  ```

- `packet01-inline-exec-before` (the preceding copied list log is the nonzero-discovery guard):

  ```bash
  bash -lc 'set -euo pipefail; mkdir -p target; rg -q ": test$" target/packet01-inline-list-before.log; cargo test -p slicer-core --features host-algos --lib flow::tests:: -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; cp -f target/test-output.log target/packet01-inline-exec-before.log; rg -q "test result: ok" target/packet01-inline-exec-before.log'
  ```

- `packet01-integration-list-before`:

  ```bash
  bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test flow_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; cp -f target/test-output.log target/packet01-integration-list-before.log; rg -q ": test$" target/packet01-integration-list-before.log'
  ```

- `packet01-integration-exec-before` (the preceding copied list log is the nonzero-discovery guard):

  ```bash
  bash -lc 'set -euo pipefail; mkdir -p target; rg -q ": test$" target/packet01-integration-list-before.log; cargo test -p slicer-core --features host-algos --test flow_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; cp -f target/test-output.log target/packet01-integration-exec-before.log; rg -q "test result: ok" target/packet01-integration-exec-before.log'
  ```

- After the edit, run `packet01-inline-list-after`, `packet01-integration-list-after`, and `packet01-integration-exec-after`; each command copies its completed output to its corresponding `target/packet01-*-after.log` file. The inline list must be empty, while integration discovery must be nonzero and integration execution must report a passing result:

  ```bash
  bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib flow::tests:: -- --list 2>&1 | tee target/test-output.log >/dev/null; cp -f target/test-output.log target/packet01-inline-list-after.log; ! rg -q ": test$" target/packet01-inline-list-after.log'
  ```

  ```bash
  bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test flow_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; cp -f target/test-output.log target/packet01-integration-list-after.log; rg -q ": test$" target/packet01-integration-list-after.log'
  ```

  ```bash
  bash -lc 'set -euo pipefail; mkdir -p target; rg -q ": test$" target/packet01-integration-list-after.log; cargo test -p slicer-core --features host-algos --test flow_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; cp -f target/test-output.log target/packet01-integration-exec-after.log; rg -q "test result: ok" target/packet01-integration-exec-after.log'
  ```

- `packet01-survivor-name-reconcile` is a names-only check; it preserves every preexisting integration registration and maps each original inline name to a surviving integration name without inspecting test bodies:

  ```bash
  bash -lc 'set -euo pipefail; python - <<PY
  from pathlib import Path

  def names(path):
      return {
          line.strip()[:-6].rsplit("::", 1)[-1]
          for line in Path(path).read_text().splitlines()
          if line.strip().endswith(": test")
      }

  inline_to_survivor = {
      "classic_0p4mm_bead_0p2mm_layer": "canonical_0p4mm_bead_0p2mm_layer_spacing",
      "width_at_or_above_nozzle_produces_larger_spacing": "wider_bead_spacing_is_larger_than_canonical",
      "width_below_layer_height_still_has_positive_spacing": "width_below_layer_height_still_has_positive_spacing",
      "spacing_errors_at_canonicals_actual_threshold": "spacing_errors_at_the_canonical_threshold",
      "zero_or_negative_inputs_error": "zero_or_negative_width_errors",
      "error_message_names_inputs_and_fix": "production_reachable_negative_spacing_errors_with_actionable_message",
      "roundtrip_spacing_to_width": "roundtrip_spacing_to_width_recovers_original",
      "bridging_flow_non_thick_returns_ratio_unchanged": "bridging_flow_non_thick_returns_ratio_unchanged",
      "bridging_flow_thick_round_cross_section_factor": "bridging_flow_thick_round_cross_section_factor",
      "bridging_flow_thick_degenerate_inputs_fall_back_to_ratio": "bridging_flow_thick_degenerate_inputs_fall_back_to_ratio",
  }
  before_inline = names("target/packet01-inline-list-before.log")
  before_integration = names("target/packet01-integration-list-before.log")
  after_integration = names("target/packet01-integration-list-after.log")
  assert before_inline and before_integration and after_integration
  assert set(inline_to_survivor) <= before_inline
  assert before_integration <= after_integration
  assert set(inline_to_survivor.values()) <= after_integration
  print("PASS: names-only inline survivor and integration-name reconciliation")
  PY'
  ```

- These procedures use no frozen counts and no source-grep assertions. Content preservation remains governed by the survivor inventory and review crosswalk above; the existing 17 named survivor guards in the verification command remain unchanged.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test flow_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; rg -q ": test$" target/test-output.log; cargo test -p slicer-core --features host-algos --test flow_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; for n in canonical_0p4mm_bead_0p2mm_layer_spacing wider_bead_spacing_is_larger_than_canonical width_below_layer_height_still_has_positive_spacing spacing_errors_at_the_canonical_threshold zero_or_negative_width_errors production_reachable_negative_spacing_errors_with_actionable_message roundtrip_spacing_to_width_recovers_original spacing_is_monotone_in_line_width flow_to_width_zero_inputs_return_zero flow_to_width_result_is_at_least_spacing bridging_flow_non_thick_returns_ratio_unchanged bridging_flow_thick_round_cross_section_factor bridging_flow_thick_degenerate_inputs_fall_back_to_ratio resolve_role_width_bridge_and_first_layer_precedence_matrix resolve_role_width_bridge_fallback_covers_zero_and_absent_widths resolve_role_width_role_zero_and_positive_matrix resolve_role_width_auto_sentinel_uses_line_width_then_nozzle_width; do rg -q "test ${n} \.\.\. ok" target/test-output.log; done'` - FACT pass/fail; proves nonzero discovery and every planned survivor pass line.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/src/flow.rs crates/slicer-core/tests/flow_tdd.rs 2>&1 | tee target/test-output.log >/dev/null; ! rg -q "crates/slicer-core/(src/flow.rs|tests/flow_tdd.rs)" target/test-output.log'` - FACT pass/fail; touched-scope report is filtered and does not claim other core findings are closed.
- Exit condition: Exit with failure if the survivor map cannot name each inline case's surviving test/assertion, including `(0.4, 1.0)` at `0.1854 ± 1e-3` and `boundary * 0.5` rejection, discovery is zero, any exact flow test fails, inline `#[test]` remains, or the quality report identifies an unaddressed touched-scope finding. Do not expand the edit surface.

### Step 2: Record the mandatory partial core ledger evidence

- Task IDs: `core/DUP-CORE (flow)`, `core/RETIRE (flow)`, `core/DUP-CORE-strengthen (wider-bead)`
- Objective: Update only §7's `core` ledger row with the flow slice's partial status, retired/changed symbols, surviving coverage, actual validations, and a final cell containing `remaining non-flow §5.1 work stays open; next queued gap: core-bridge-dup-merge`.
- Precondition: Step 1's flow tests and gates pass; no whole-core closure is justified.
- Postcondition: The §7 `core` row records the flow slice evidence and its final cell contains `remaining non-flow §5.1 work stays open; next queued gap: core-bridge-dup-merge`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger only.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` - §7 `core` row only.
- Files explicitly out of bounds:
  - Both source/test homes, queue sections, other waves, `docs/07_implementation_status.md`, and every other packet.
- Expected sub-agent dispatches: None; parent-provided facts govern this bookkeeping edit.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §4 exit-gate distinction and §7 ledger.
  - `docs/22_test_quality.md` - evidence/quality standard.
- OrcaSlicer refs: None.
- Verification:
  - `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(encoding=\"utf-8\"); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-flow-consolidation\",\"flow.rs\",\"flow_tdd\")); assert all(x in coverage for x in (\"flow_tdd\",\"width_below_layer_height_still_has_positive_spacing\",\"spacing_errors_at_the_canonical_threshold\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"remaining non-flow §5.1 work stays open; next queued gap: core-bridge-dup-merge\" in gap.casefold(); print(\"PASS: anchored core ledger row has six validated cells\")"'` - FACT pass/fail; confirms exactly one §7 `core` row, exact six-column shape, and all required evidence cells.
- Exit condition: Exit with failure if the row claims the entire core wave is closed, rewrites queue/other-wave state, freezes audit counts, omits actual validation evidence, or omits the exact final-cell phrase `remaining non-flow §5.1 work stays open; next queued gap: core-bridge-dup-merge`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two bounded source files plus manifest wiring; no production or external canonical read. |
| Step 2 | S | One bounded §7 ledger row; no queue or other-wave edits. |

## Packet Completion Gate

- All distinct flow cases have a named survivor in the single integration home.
- Exact feature-correct target discovery is nonzero and all targeted tests pass with tee'd output.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and touched-scope `check-test-quality --report` pass.
- Step 1's `packet01-inline-list-before`, `packet01-inline-exec-before`, `packet01-integration-list-before`, and `packet01-integration-exec-before` logs are inspected without rerunning or overwriting the already-successful baseline; `packet01-inline-list-after`, `packet01-integration-list-after`, `packet01-integration-exec-after`, and `packet01-survivor-name-reconcile` prove empty inline discovery, nonzero integration discovery/execution, preservation of every preexisting integration name, and name-only inline survivor mapping. No frozen counts are accepted.
- The implementation ledger update is mandatory and limited to §7's `core` row; its final cell must contain `remaining non-flow §5.1 work stays open; next queued gap: core-bridge-dup-merge`, without closing the core wave or changing queue/other-wave state.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record any remaining packet-local risk, especially whether exact duplicate cases were merged without assertion loss.
- Keep `core-bridge-dup-merge` and all other core work pending as applicable; do not claim the wave-0 core zero-findings gate is closed until the full core wave reaches its own exit gate.
