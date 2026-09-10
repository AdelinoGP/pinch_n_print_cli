# Requirements: core-region-mapping-dup-merges

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE (region mapping)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The Wave-0 audit confirmed `region_mapping_enumerate_chains` and `region_mapping_cross_product_entry_count` (`crates/slicer-core/tests/algo_region_mapping_tdd.rs`) are textually duplicated case-subsets of `region_mapping_two_semantics_produces_cross_product_cardinality` in the same file. All three build the identical fixture — `single_region_plan("obj_a")`, empty stage invocations, `no_paint_configs`, `aggregated(&["material", "fuzzy_skin"])`, and `painted_object("obj_a", [("material", ToolIndex(1), ToolIndex(2)), ("fuzzy_skin", Flag(true))])` — and drive the same `execute_region_mapping_with_cap` call. The survivor already asserts the analytic cardinality `entries.len() == 6` and the full six-chain set equality with the `"expected cross-product chains"` message, so each duplicate's assertion set is a strict subset of the survivor's. The only candidate-unique prose is AC-4's abstract formula `entries.len() == layers × active_regions × ∏(1 + K_i)` from the entry-count duplicate's doc comment; the AC-3 enumeration wording (`Verifies SET membership of the enumerated chains.`) already appears in the survivor's own doc comment and is retained. The formula's provenance is the P93 invariant prose carried today on the `DEFAULT_REGION_MAP_CAP` doc comment at `crates/slicer-ir/src/slice_ir.rs` (lines 1695–1711) and in `docs/spec_packets/_OLD/93_region-mapping-cross-product.md`. The approved `core-region-mapping-dup-merges` slice (plan Packet Queue row #4) merges both duplicates into the plan-named survivor while preserving the exact chain-set, cardinality, and ordering coverage of the file — notably the distinct ordering witness `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order` and the distinct fixture variants in the 21 remaining tests — without touching production code or any other core test.

## In Scope

- Delete exactly two duplicate tests from `crates/slicer-core/tests/algo_region_mapping_tdd.rs`: `region_mapping_enumerate_chains` and `region_mapping_cross_product_entry_count`, merging into the surviving `region_mapping_two_semantics_produces_cross_product_cardinality` (the direction pinned by the plan's §5.1 DUP-CORE row and the audit directive `region_mapping_enumerate_chains and region_mapping_cross_product_entry_count → region_mapping_two_semantics_produces_cross_product_cardinality`).
- Preserve the union of the absorbed tests' assertions on the survivor: the full six-chain set equality and the analytic cardinality `assert_eq!(region_map.entries.len(), 6);` are already present in the survivor's body and must remain.
- Extend the survivor's doc comment to retain the absorbed AC-4 formula prose (`entries.len() == layers × active_regions × ∏(1 + K_i)`); the AC-3 enumeration wording already in the survivor's doc comment stays verbatim. The existing `AC-9 (c) / AC-3 / AC-4:` alias line already covers the survivor's own claims and may be kept verbatim.
- Preserve all 21 distinct tests byte-identically, including `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order`'s strict-ascending canonical-order witness, the disjoint-objects per-object-chain case, the paint-scan and per-tool-overlay cases, the cap-exceeded error case, the modifier and variant-id guards, and the config-interning case.
- Keep the target's imports, helpers (`sv`, `make_layer_plan`, `no_objects`, `no_paint_configs`, `empty_aggregated`, `semantic_for`, `painted_object`, `modifier_cube_mesh`, `modifier_volume`, `aggregated`, `single_region_plan`, `unencodable_modifier_parent_id`, `map_single_region_with_object`), and the survivor's inline canonical-order comment block intact unless a deletion makes an item unused; if a deletion makes an item unused, delete that item only. Neither absorbed test defines a helper, so no helper deletion is expected.
- Record before/after discovery and execution census reconciliation at implementation time without freezing counts in this packet.
- The post-merge target discovers exactly `22` tests (down from `24`); the delta `2` is the accounted retirement.

## Out of Scope

- Any production code or behavior change in `crates/slicer-core/src/**`, including `algos/region_mapping.rs` (`execute_region_mapping_with_cap` and the `∏(1 + K_i)` cross-product kernel), and in `crates/slicer-ir/**`, including `region_split_registry.rs` (`enumerate_canonical_chains`) and `slice_ir.rs` (`DEFAULT_REGION_MAP_CAP`).
- All other `slicer-core` test files and all other queue IDs, including pending support, wall-sequence, geometry, beading, strengthen, retire, paint, brittle, cross, and parity items.
- `core-flow-consolidation`'s two test homes (`flow.rs`, `flow_tdd.rs`) and `core-bridge-dup-merge`'s test home (`bridge_false_site_gating_tdd.rs`); both packets are generation-order only and export no API, no test files, and no symbols.
- New tests, renamed survivors (other than deleting the absorbed names), strengthened oracles, parameterization of the 21 distinct tests, or WIT/IR/schema/coordinate/WASM/OrcaSlicer changes.
- Editing the parent plan during generation. The implementation worker must make the narrow §7 `core` ledger update required by the AC-4 ledger parser command in `packet.spec.md`, but must not rewrite queue entries, other waves, or claim the whole core wave closed.
- Task-### mapping; the approved program IDs are authoritative for this packet.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §1 non-negotiables; §4 wave exit gates; §5.1 DUP-CORE region-mapping item; §6 command pattern; Packet Queue rows #3–#5 and the resume/continuation-approval paragraphs. Bounded direct reads.
- `docs/22_test_quality.md` - census reconciliation and duplicate-merge survivor discipline; §2.1 names the survivor as the in-tree repaired-shape example. Bounded direct read.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - every retired test must name the survivor protecting its regression input. Direct read.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - `check-test-quality` remains report mode until final program promotion. Direct read.
- `docs/spec_packets/_OLD/93_region-mapping-cross-product.md` - provenance of the `AC-3`/`AC-4`/`AC-9` acceptance aliases the survivor's doc comment cites, and of the `∏(1 + K_i)` cardinality invariant whose live doc-comment home is the `DEFAULT_REGION_MAP_CAP` doc comment at `crates/slicer-ir/src/slice_ir.rs` (lines 1695–1711, read-only). Direct read (63 lines).
- `.agents/skills/spec-packet-generator/references/templates/*.md` and `.agents/skills/spec-review/references/preflight-gate.md` - packet contract and S0–S8 inventory.

## Acceptance Summary

- Positive: `AC-1` through `AC-3` in `packet.spec.md`.
- Negative: `AC-N1` in `packet.spec.md`; it pins the survivor's exact assertion strength (full set equality + analytic cardinality) and the no-new-tests invariant.
- Ledger evidence: the AC-4 ledger parser command in `packet.spec.md`; it owns the mandatory §7 partial-row evidence.
- Cross-packet impact: generation unblocks `core-support-dup-merges` only after independent preflight. The packet exports no API or implementation prerequisite. `core-flow-consolidation` and `core-bridge-dup-merge` are not implementation dependencies.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test algo_region_mapping_tdd -- --list 2>&1 \| tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log \|\| true)" -eq 22; test "$(grep -c "^region_mapping_enumerate_chains: test$" target/test-output.log \|\| true)" -eq 0; test "$(grep -c "^region_mapping_cross_product_entry_count: test$" target/test-output.log \|\| true)" -eq 0; cargo test -p slicer-core --features host-algos --test algo_region_mapping_tdd -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok\. 22 passed; 0 failed" target/test-output.log; rg -q "test region_mapping_two_semantics_produces_cross_product_cardinality \.\.\. ok" target/test-output.log'` | Prove the feature-correct target discovers exactly 22 tests, both absorbed names are absent from discovery, the full target passes, and the survivor passes without a zero-test false green. | FACT pass/fail; inspect `target/test-output.log` on failure. |
| `cargo check --workspace --all-targets` | Compile all workspace targets after the test-file edit. | FACT pass/fail. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Preserve lint cleanliness after the deletion. | FACT pass/fail. |
| `cargo xtask check-literals` | Confirm no struct-literal rule is introduced or violated. | FACT pass/fail. |
| `cargo xtask check-test-quality --report crates/slicer-core/tests/algo_region_mapping_tdd.rs` | Report touched-scope quality findings; implementation must close or justify any remaining findings for this file. | FACT pass/fail plus filtered findings. |
| `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"\|\") and not x.startswith(\"\|---\")]; headers=[x for x in rows if x.split(\"\|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"\|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"\|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"\|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-region-mapping-dup-merges\",\"region_mapping_enumerate_chains\",\"region_mapping_cross_product_entry_count\",\"region_mapping_two_semantics_produces_cross_product_cardinality\",\"algo_region_mapping_tdd\")); assert all(x in coverage for x in (\"region_mapping_two_semantics_produces_cross_product_cardinality\",\"region_mapping_chains_ordered_by_aggregated_region_split_canonical_order\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-support-dup-merges\" in gap; print(\"PASS: anchored core ledger row has six validated cells\")"'` | Parse §7, select exactly the `core` row, enforce the exact six-column header and row shape, and validate every AC-4 field without global keyword matches. | FACT pass/fail; parser emits one PASS line or exits nonzero. |

Every future cargo test command uses a self-contained `bash -lc 'set -euo pipefail; ...'` wrapper, creates `target`, captures combined output with `tee target/test-output.log >/dev/null`, checks the completed log rather than piping `rg` directly from `tee`, and rejects zero discovery before execution. The implementation must re-derive current counts from `cargo test ... -- --list` and the current discovery census rather than use packet-time counts. `algo_region_mapping_tdd` is feature-gated on `host-algos` (verified against the `[[test]]` stanza in `crates/slicer-core/Cargo.toml` and `docs/specs/test-quality-remediation-census.json`); every invocation MUST carry `--features host-algos`.

## Step Completion Expectations

The deletion step must produce the survivor map (both duplicates → `region_mapping_two_semantics_produces_cross_product_cardinality`) before removing the duplicates; the final validation must show exactly 22 discovered tests, no absorbed name, the full 22/22 pass, and the survivor's union assertions retained. The implementation must also complete the narrow §7 ledger update; no other queue item or the whole core wave is implicitly closed.

## Context Discipline Notes

Read only the bounded source ranges named by `design.md`; do not inspect unrelated core tests or production modules. No OrcaSlicerDocumented inspection is applicable because this packet does not port or modify production canonical behavior.
