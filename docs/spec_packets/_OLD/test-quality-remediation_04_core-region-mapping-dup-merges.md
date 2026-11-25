---
status: implemented
packet: test-quality-remediation_04_core-region-mapping-dup-merges
task_ids:
  - core/DUP-CORE
---

# test-quality-remediation_04_core-region-mapping-dup-merges

## Goal

Merge the two textually duplicated region-mapping cross-product tests `region_mapping_enumerate_chains` and `region_mapping_cross_product_entry_count` in `crates/slicer-core/tests/algo_region_mapping_tdd.rs` into the plan-named survivor `region_mapping_two_semantics_produces_cross_product_cardinality`, preserving the exact chain-set, cardinality, and ordering coverage of the file while retaining the absorbed AC-4 formula and AC-3 enumeration alias prose.

## Problem Statement

The Wave-0 audit confirmed `region_mapping_enumerate_chains` and `region_mapping_cross_product_entry_count` (`crates/slicer-core/tests/algo_region_mapping_tdd.rs`) are textually duplicated case-subsets of `region_mapping_two_semantics_produces_cross_product_cardinality` in the same file. All three build the identical fixture — `single_region_plan("obj_a")`, empty stage invocations, `no_paint_configs`, `aggregated(&["material", "fuzzy_skin"])`, and `painted_object("obj_a", [("material", ToolIndex(1), ToolIndex(2)), ("fuzzy_skin", Flag(true))])` — and drive the same `execute_region_mapping_with_cap` call. The survivor already asserts the analytic cardinality `entries.len() == 6` and the full six-chain set equality with the `"expected cross-product chains"` message, so each duplicate's assertion set is a strict subset of the survivor's. The only candidate-unique prose is AC-4's abstract formula `entries.len() == layers × active_regions × ∏(1 + K_i)` from the entry-count duplicate's doc comment; the AC-3 enumeration wording (`Verifies SET membership of the enumerated chains.`) already appears in the survivor's own doc comment and is retained. The formula's provenance is the `DEFAULT_REGION_MAP_CAP` doc comment in `crates/slicer-ir/src/slice_ir.rs` and the Cross-product cardinality invariant in `docs/spec_packets/_OLD/93_region-mapping-cross-product.md`. The approved `core-region-mapping-dup-merges` slice (plan Packet Queue row #4) merges both duplicates into the plan-named survivor while preserving the exact chain-set, cardinality, and ordering coverage of the file — notably the distinct ordering witness `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order` and the distinct fixture variants in the 21 sibling tests — without touching production code or any other core test.

## Architecture Constraints

- The sole code edit surface is `crates/slicer-core/tests/algo_region_mapping_tdd.rs`; the only additional edit is the required §7 `core` ledger-row bookkeeping in `docs/specs/test-quality-remediation-plan.md`. No production behavior or export changes.
- The single test binary is `algo_region_mapping_tdd`; its named `[[test]]` stanza in `crates/slicer-core/Cargo.toml` carries `required-features = ["host-algos"]`, so every command uses `--features host-algos` to avoid feature-blind verification under the core-wave policy.
- Preserve the merge direction pinned by the plan §5.1 DUP-CORE row and the audit directive: survivor `region_mapping_two_semantics_produces_cross_product_cardinality`, absorbed `region_mapping_enumerate_chains` and `region_mapping_cross_product_entry_count`.
- Preserve the survivor's assertion union verbatim: the analytic `assert_eq!(region_map.entries.len(), 6);` and the full six-chain `HashSet` set equality with the `"expected cross-product chains"` message. The two absorbed bodies are strict assertion subsets of this union (one asserts only the count; the other asserts only the chain set), verified against the pre-edit file.
- Extend the survivor's doc comment so the absorbed AC-4 alias is not lost: add the candidate-unique formula prose `entries.len() == layers × active_regions × ∏(1 + K_i)`. The AC-3 enumeration wording (`SET membership of the enumerated chains`) already appears in the survivor's doc comment and is retained verbatim; the existing `AC-9 (c) / AC-3 / AC-4:` line already names the aliases and stays.
- Preserve all 21 sibling tests byte-identically — in particular `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order`'s strict-ascending canonical-order witness (the file's ordering coverage), `region_mapping_two_objects_with_disjoint_paint_emit_per_object_chains`, `region_mapping_cap_exceeded_returns_error`, and `region_mapping_config_interning`. Preserve the survivor body byte-identically while extending only its doc comment.
- The `∏(1 + K_i)` invariant is the survivor's claim provenance from `docs/spec_packets/_OLD/93_region-mapping-cross-product.md` §Architecture Constraints ("Cross-product cardinality invariant"); retaining the formula in the survivor's doc comment keeps that locked invariant's test-side witness wording after the merge.
- No schema, WIT, coordinate conversion, scheduler, manifest, or WASM contract is touched. The test fixtures construct paint semantics and values by name only (`"material"`, `"fuzzy_skin"`, `ToolIndex`, `Flag`); no unit conversion is authored by this packet.

## Data and Contract Notes

- IR/manifest contracts: none.
- WIT boundary: none.
- Determinism/scheduler constraints: none; the retained tests keep deterministic fixture inputs (fixed semantic names and `PaintValue` variants) and exact chain-set assertions; the survivor's `HashSet`-based set equality is order-insensitive by design, while the ordering witness retains order sensitivity for the canonical-order contract.

## Locked Assumptions and Invariants

- The survivor is `region_mapping_two_semantics_produces_cross_product_cardinality`; both absorbed names are removed from the file and from discovery.
- Post-merge discovery is expected to be exactly 22 tests; implementation must re-derive the live 24-test baseline and stop if the expected two-test retirement does not reconcile.
- The survivor keeps its exact assertion strength: analytic `entries.len() == 6` plus full six-chain set equality, its doc comment now additionally carrying the absorbed AC-4 formula prose alongside its pre-existing AC-3 enumeration wording.
- No new public symbol is introduced and no net-new export is expected.

## Risks and Tradeoffs

- A careless deletion could orphan imports or helpers; both absorbed fns are self-contained (no helpers of their own) and use only imports/helpers already used by the survivor, so no import or helper is expected to become unused. The `HashSet` import and `PaintValue`/`SemVer` usages remain exercised by retained tests.
- Merging by test name alone could lose a distinct input; the pre-grounded survivor map shows both duplicates are strict assertion subsets (one count-only, one set-only) of the survivor on the identical fixture, and the 21 sibling tests are distinct in fixture or assertion (ordering witness, disjoint objects, cap error, config interning, modifier and variant-id guards, per-tool overlays).
- Feature flags can make a narrow run appear green without compiling the intended target; `--features host-algos` and exact-name guards are mandatory in every command.
- The doc-comment extension could drift the `AC-9 (c) / AC-3 / AC-4:` alias line; the edit adds prose without rewording the existing line, and AC-3 of `packet.spec.md` pins both retained fragments.
