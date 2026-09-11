---
status: draft
packet: test-quality-remediation_03_core-bridge-dup-merge
task_ids:
  - core/DUP-CORE
backlog_source: docs/specs/test-quality-remediation-plan.md
context_cost_estimate: S
copy_note: Approved core-bridge-dup-merge queue entry; task IDs are program item IDs, not TASK-### mappings.
---

# Packet Contract: core-bridge-dup-merge

## Goal

Merge the textually duplicated supported-bridge rejection test in `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` into its named survivor while preserving every distinct unsupported-span and ungated-candidate guard exactly.

## Scope Boundaries

Edit only `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`: delete the single duplicate `#[test]` function and leave the five distinct guards untouched. No production code, other core test files, new tests, or behavior changes are authorized; the implementation-time §7 `core` ledger row bookkeeping in `docs/specs/test-quality-remediation-plan.md` is the only documentation surface.

## Prerequisites and Blockers

- Depends on: None as an implementation prerequisite; `core-flow-consolidation` is a generation-order dependency only and exports no API.
- Unblocks: `core-region-mapping-dup-merges` packet generation only.
- Activation blockers: Independent `spec-review --preflight`; no scope expansion.

## Acceptance Criteria

- **AC-1. Given** the merged target retains five distinct guards, **when** the feature-correct target is listed and executed, **then** `solid_underneath_span_produces_no_bridge_area`, `unsupported_span_retains_bridge_area`, `ungated_candidates_cannot_silently_return`, `no_lower_layer_clears_bridge_areas`, and `existing_empty_lower_layer_retains_bridge_area` are each discovered and pass, and the absorbed name `fully_supported_candidate_rejected_zero_bridge_area` no longer appears in the file. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; rg -q ": test$" target/test-output.log; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; for n in solid_underneath_span_produces_no_bridge_area unsupported_span_retains_bridge_area ungated_candidates_cannot_silently_return no_lower_layer_clears_bridge_areas existing_empty_lower_layer_retains_bridge_area; do rg -q "test ${n} \.\.\. ok" target/test-output.log; done; ! rg -q -F "fully_supported_candidate_rejected_zero_bridge_area" crates/slicer-core/tests/bridge_false_site_gating_tdd.rs'`
- **AC-2. Given** the duplicate was the only redundant case in the target, **when** discovery is re-derived after the merge, **then** the target discovers exactly `5` tests, zero of which match `fully_supported_candidate_rejected_zero_bridge_area`, and exactly one of which matches `solid_underneath_span_produces_no_bridge_area`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 5; test "$(grep -c "^fully_supported_candidate_rejected_zero_bridge_area: test$" target/test-output.log || true)" -eq 0; test "$(grep -c "^solid_underneath_span_produces_no_bridge_area: test$" target/test-output.log || true)" -eq 1'`
- **AC-3. Given** the lower-layer span shapes the gate consumes, **when** the three span-shape witnesses run, **then** `unsupported_span_retains_bridge_area` still asserts the retained area equals `difference(&[bridge.clone()], &[anchor.clone()])` exactly and is disjoint from the anchor, `no_lower_layer_clears_bridge_areas` clears bridge areas on `None`, and `existing_empty_lower_layer_retains_bridge_area` retains the full bridge polygon for `Some(&[])`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; for n in unsupported_span_retains_bridge_area no_lower_layer_clears_bridge_areas existing_empty_lower_layer_retains_bridge_area; do rg -q "${n}: test" target/test-output.log; done; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; for n in unsupported_span_retains_bridge_area no_lower_layer_clears_bridge_areas existing_empty_lower_layer_retains_bridge_area; do rg -q "test ${n} \.\.\. ok" target/test-output.log; done'`
- **AC-4. Given** the ungated guard starts with empty `bridge_areas` so the candidate provably originates from `assemble_bridge_areas` stamping, **when** the ungated guard runs, **then** the stamped population is non-empty, `gate_bridge_areas_by_unsupported_span` over the fully supported span empties it, and gated differs from ungated. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; rg -q "ungated_candidates_cannot_silently_return: test" target/test-output.log; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test ungated_candidates_cannot_silently_return \.\.\. ok" target/test-output.log'`
- **AC-5. Given** the bridge slice is implemented without closing unrelated core work, **when** the narrow ledger update is recorded, **then** the `core` row in §7 is the sole selected row, its table header has exactly six columns `Wave`, `State`, `Retired/changed symbols`, `Surviving/new coverage`, `Validation`, and `Remaining gap`, its row has exactly six cells, has state `partial`, names `core-bridge-dup-merge`, `fully_supported_candidate_rejected_zero_bridge_area`, `solid_underneath_span_produces_no_bridge_area`, and `bridge_false_site_gating_tdd` in its changed-symbols cell, names `unsupported_span_retains_bridge_area`, `ungated_candidates_cannot_silently_return`, and `existing_empty_lower_layer_retains_bridge_area` in its coverage cell, records `cargo test`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `check-test-quality` in its validation cell, and names the remaining non-bridge gap `core-region-mapping-dup-merges` in its final cell. | `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-bridge-dup-merge\",\"fully_supported_candidate_rejected_zero_bridge_area\",\"solid_underneath_span_produces_no_bridge_area\",\"bridge_false_site_gating_tdd\")); assert all(x in coverage for x in (\"unsupported_span_retains_bridge_area\",\"ungated_candidates_cannot_silently_return\",\"existing_empty_lower_layer_retains_bridge_area\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-region-mapping-dup-merges\" in gap and \"non-bridge\" in gap; print(\"PASS: anchored core ledger row has six validated cells\")"'`

## Negative Test Cases

- **AC-N1. Given** the fully supported `10×10` span rejection must remain an exact empty result, **when** the surviving rejection witness runs, **then** `region.bridge_areas.is_empty()` is asserted (never weakened to a merely-non-empty or size-reduced check) and the witness passes. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test solid_underneath_span_produces_no_bridge_area \.\.\. ok" target/test-output.log; rg -q -F "assert!(region.bridge_areas.is_empty());" crates/slicer-core/tests/bridge_false_site_gating_tdd.rs'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 5; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; for n in solid_underneath_span_produces_no_bridge_area unsupported_span_retains_bridge_area ungated_candidates_cannot_silently_return no_lower_layer_clears_bridge_areas existing_empty_lower_layer_retains_bridge_area; do rg -q "test ${n} \.\.\. ok" target/test-output.log; done'` — the primary contract: exact five-count discovery with the absorbed name absent, all five survivors passing (same command as AC-1/AC-2's shared execution core, repeated rather than referenced).

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` §1, §4, §5.1, §6, Packet Queue, and the `core-bridge-dup-merge` continuation-approval paragraphs - direct bounded reads.
- `docs/22_test_quality.md` §§1–5 - direct bounded read; duplicate-merge and survivor-map discipline.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct read; the absorbed duplicate needs a named survivor before removal.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - direct read; report-mode gate status.
- `docs/00_project_overview.md` normative document map and `.agents/doc-index.md` - direct reads.
- `.agents/skills/spec-review/references/preflight-gate.md` S0–S8 - direct read; symbol and target checks.

## Doc Impact Statement (Required)

Specific implementation-time documentation impact: `docs/specs/test-quality-remediation-plan.md` §7 Ledger - the implementation worker must update only the `core` ledger row with this bridge slice's partial state, retired/changed symbols (`fully_supported_candidate_rejected_zero_bridge_area` absorbed by `solid_underneath_span_produces_no_bridge_area` in `bridge_false_site_gating_tdd`), surviving coverage, actual validations, and the remaining non-bridge `core-region-mapping-dup-merges` gap; it must not close the core wave or rewrite the queue/other waves. Verification is the AC-5 ledger parser command defined below. This is bookkeeping, not a production contract change.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).