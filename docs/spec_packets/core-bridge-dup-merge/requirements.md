# Requirements: core-bridge-dup-merge

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE (bridge)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The Wave-0 audit confirmed `fully_supported_candidate_rejected_zero_bridge_area` (`crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`) is textually identical to `solid_underneath_span_produces_no_bridge_area` in the same file: both build the same `(0.0, 0.0)–(10.0, 10.0)` bridge polygon via `region_with_bridge`, call `gate_bridge_areas_by_unsupported_span(&mut region, Some(&[bridge]))` over a fully supported span, and assert `region.bridge_areas.is_empty()`. The approved `core-bridge-dup-merge` slice (plan Packet Queue row #3) merges the duplicate into the plan-named survivor while preserving the five distinct guards — the exact difference-and-disjointness unsupported-span assertion, the ungated-stamping control, the `None` lower-layer clearing, and the empty-`Some(&[])` retention — without touching production code or any other core test.

## In Scope

- Delete the single duplicate test `fully_supported_candidate_rejected_zero_bridge_area` from `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`, merging into the surviving `solid_underneath_span_produces_no_bridge_area` (the direction pinned by the plan's §5.1 DUP-CORE row).
- Preserve all five distinct guards and their exact assertions: `solid_underneath_span_produces_no_bridge_area` (fully supported span → empty bridge areas), `unsupported_span_retains_bridge_area` (retained area equals `difference(&[bridge.clone()], &[anchor.clone()])` exactly and is disjoint from the anchor via `intersection`), `ungated_candidates_cannot_silently_return` (empty-start `bridge_areas` proving `assemble_bridge_areas` stamping, then gating to empty), `no_lower_layer_clears_bridge_areas` (`None` clears), and `existing_empty_lower_layer_retains_bridge_area` (`Some(&[])` retains the full bridge polygon).
- Keep the target's imports, `square` helper, `region_with_bridge` helper, and the `// AC-2:` retention comment in `unsupported_span_retains_bridge_area` intact unless a removal makes them unused; if a removal makes an item unused, delete that item only.
- Record before/after discovery and execution census reconciliation at implementation time without freezing counts in this packet.
- The post-merge target discovers exactly `5` tests (down from `6`); the delta `1` is the accounted retirement.

## Out of Scope

- Any production code or behavior change in `crates/slicer-core/src/**`, including `algos::prepass_slice::{assemble_bridge_areas, gate_bridge_areas_by_unsupported_span}`.
- All other `slicer-core` test files and all other queue IDs, including pending region-mapping, support, wall-sequence, geometry, beading, strengthen, retire, paint, brittle, cross, and parity items.
- `core-flow-consolidation`'s two test homes (`flow.rs`, `flow_tdd.rs`); that packet is generation-order only and exports no API.
- New tests, renamed survivors (other than deleting the absorbed name), strengthened oracles, or WIT/IR/schema/coordinate/WASM/OrcaSlicer changes.
- Editing the parent plan during generation. The implementation worker must make the narrow §7 `core` ledger update required by the AC-5 ledger parser command in `packet.spec.md`, but must not rewrite queue entries, other waves, or claim the whole core wave closed.
- Task-### mapping; the approved program IDs are authoritative for this packet.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §1 non-negotiables; §4 wave exit gates; §5.1 DUP-CORE bridge item; §6 command pattern; Packet Queue row #3 and the continuation-approval paragraphs. Bounded direct reads.
- `docs/22_test_quality.md` - census reconciliation and duplicate-merge survivor discipline. Bounded direct read.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - every retired test must name the survivor protecting its regression input. Direct read.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - `check-test-quality` remains report mode until final program promotion. Direct read.
- `.agents/skills/spec-packet-generator/references/templates/*.md` and `.agents/skills/spec-review/references/preflight-gate.md` - packet contract and S0–S8 inventory.

## Acceptance Summary

- Positive: `AC-1` through `AC-4` in `packet.spec.md`.
- Negative: `AC-N1` in `packet.spec.md`; it pins the surviving rejection witness's exact empty assertion.
- Ledger evidence: the AC-5 ledger parser command in `packet.spec.md`; it owns the mandatory §7 partial-row evidence.
- Cross-packet impact: generation unblocks `core-region-mapping-dup-merges` only after independent preflight. The packet exports no API or implementation prerequisite. `core-flow-consolidation` is not an implementation dependency.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --list 2>&1 \| tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log \|\| true)" -eq 5; cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; for n in solid_underneath_span_produces_no_bridge_area unsupported_span_retains_bridge_area ungated_candidates_cannot_silently_return no_lower_layer_clears_bridge_areas existing_empty_lower_layer_retains_bridge_area; do rg -q "test ${n} \.\.\. ok" target/test-output.log; done'` | Prove the feature-correct target discovers exactly five tests and every named survivor passes without a zero-test false green. | FACT pass/fail; inspect `target/test-output.log` on failure. |
| `cargo check --workspace --all-targets` | Compile all workspace targets after the test-file edit. | FACT pass/fail. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Preserve lint cleanliness after the deletion. | FACT pass/fail. |
| `cargo xtask check-literals` | Confirm no struct-literal rule is introduced or violated. | FACT pass/fail. |
| `cargo xtask check-test-quality --report crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` | Report touched-scope quality findings; implementation must close or justify any remaining findings for this file. | FACT pass/fail plus filtered findings. |
| `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"\|\") and not x.startswith(\"\|---\")]; headers=[x for x in rows if x.split(\"\|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"\|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"\|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"\|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-bridge-dup-merge\",\"fully_supported_candidate_rejected_zero_bridge_area\",\"solid_underneath_span_produces_no_bridge_area\",\"bridge_false_site_gating_tdd\")); assert all(x in coverage for x in (\"unsupported_span_retains_bridge_area\",\"ungated_candidates_cannot_silently_return\",\"existing_empty_lower_layer_retains_bridge_area\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-region-mapping-dup-merges\" in gap and \"non-bridge\" in gap; print(\"PASS: anchored core ledger row has six validated cells\")"'` | Parse §7, select exactly the `core` row, enforce the exact six-column header and row shape, and validate every AC-5 field without global keyword matches. | FACT pass/fail; parser emits one PASS line or exits nonzero. |

Every future cargo test command uses a self-contained `bash -lc 'set -euo pipefail; ...'` wrapper, creates `target`, captures combined output with `tee target/test-output.log >/dev/null`, checks the completed log rather than piping `rg` directly from `tee`, and rejects zero discovery before execution. The implementation must re-derive current counts from `cargo test ... -- --list` and the current discovery census rather than use packet-time counts. `bridge_false_site_gating_tdd` is feature-gated on `host-algos` (verified against the `[[test]]` stanza in `crates/slicer-core/Cargo.toml` and `docs/specs/test-quality-remediation-census.json`); every invocation MUST carry `--features host-algos`.

## Step Completion Expectations

The deletion step must produce the survivor map (duplicate → `solid_underneath_span_produces_no_bridge_area`) before removing the duplicate; the final validation must show exactly five discovered tests, no absorbed name, and five named passing survivors. The implementation must also complete the narrow §7 ledger update; no other queue item or the whole core wave is implicitly closed.

## Context Discipline Notes

Read only the bounded source range named by `design.md`; do not inspect unrelated core tests or production modules. No OrcaSlicerDocumented inspection is applicable because this packet does not port or modify production canonical behavior.