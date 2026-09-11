# Design: core-region-mapping-dup-merges

## Controlling Code Paths

- Primary code path: existing public `execute_region_mapping_with_cap` in `crates/slicer-core/src/algos/region_mapping.rs` (the kernel whose cardinality contract the tests lock); the canonical-chain enumeration helper `enumerate_canonical_chains` lives in `crates/slicer-ir/src/region_split_registry.rs` and the `∏(1 + K_i)` doc prose on `DEFAULT_REGION_MAP_CAP` at `crates/slicer-ir/src/slice_ir.rs`. No function bodies change and this packet edits none of those files.
- Neighboring tests/fixtures: integration tests only in `crates/slicer-core/tests/algo_region_mapping_tdd.rs` (1524 lines, 24 `#[test]` fns plus 13 module-level non-test helpers). There is no inline twin in production source for this target.
- OrcaSlicer comparison: not applicable; this test-only merge does not port canonical code. See `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The exact edit surface is `crates/slicer-core/tests/algo_region_mapping_tdd.rs` alone; no production behavior or export changes.
- The single test binary is `algo_region_mapping_tdd`; `crates/slicer-core/Cargo.toml` carries `required-features = ["host-algos"]` for it (the `[[test]]` stanza at lines 58–60 at generation time), so every command uses `--features host-algos` to avoid feature-blind verification under the core-wave policy.
- Preserve the merge direction pinned by the plan §5.1 DUP-CORE row and the audit directive: survivor `region_mapping_two_semantics_produces_cross_product_cardinality`, absorbed `region_mapping_enumerate_chains` and `region_mapping_cross_product_entry_count`.
- Preserve the survivor's assertion union verbatim: the analytic `assert_eq!(region_map.entries.len(), 6);` and the full six-chain `HashSet` set equality with the `"expected cross-product chains"` message. The two absorbed bodies are strict assertion subsets of this union (one asserts only the count; the other asserts only the chain set), verified against the pre-edit file.
- Extend the survivor's doc comment so the absorbed AC-4 alias is not lost: add the candidate-unique formula prose `entries.len() == layers × active_regions × ∏(1 + K_i)`. The AC-3 enumeration wording (`SET membership of the enumerated chains`) already appears in the survivor's doc comment and is retained verbatim; the existing `AC-9 (c) / AC-3 / AC-4:` line already names the aliases and stays.
- Preserve all 21 distinct tests byte-identically — in particular `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order`'s strict-ascending canonical-order witness (the file's ordering coverage), `region_mapping_two_objects_with_disjoint_paint_emit_per_object_chains`, `region_mapping_cap_exceeded_returns_error`, and `region_mapping_config_interning`.
- The `∏(1 + K_i)` invariant is the survivor's claim provenance from `docs/spec_packets/_OLD/93_region-mapping-cross-product.md` §Architecture Constraints ("Cross-product cardinality invariant"); retaining the formula in the survivor's doc comment keeps that locked invariant's test-side witness wording after the merge.
- No schema, WIT, coordinate conversion, scheduler, manifest, or WASM contract is touched. The test fixtures construct paint semantics and values by name only (`"material"`, `"fuzzy_skin"`, `ToolIndex`, `Flag`); no unit conversion is authored by this packet.

## Code Change Surface

- Selected approach: apply the pre-grounded survivor map — delete the two `#[test]` fns `region_mapping_cross_product_entry_count` (lines 1432–1466 at generation time, asserting only `entries.len() == 6` on the identical fixture) and `region_mapping_enumerate_chains` (lines 1468–1524 at generation time, asserting only the chain-set equality on the identical fixture) — and extend the survivor's doc comment with the one candidate-unique prose piece (the AC-4 formula); touch nothing else.
- Exact functions/types/fields/tests: delete `region_mapping_cross_product_entry_count` and `region_mapping_enumerate_chains`; retain the survivor `region_mapping_two_semantics_produces_cross_product_cardinality` and the 21 distinct tests; retain all imports and the 13 module-level non-test helpers (`sv`, `make_layer_plan`, `no_objects`, `no_paint_configs`, `empty_aggregated`, `semantic_for`, `painted_object`, `modifier_cube_mesh`, `modifier_volume`, `aggregated`, `single_region_plan`, `unencodable_modifier_parent_id`, `map_single_region_with_object` — the survivor uses `single_region_plan`, `no_paint_configs`, `aggregated`, `painted_object`, and `RegionMappingPlanProjection` construction inline); the only prose edit is the AC-4 formula addition to the survivor's doc comment.
- Rejected alternatives: renaming the survivor instead of deleting (the plan pins the survivor name; renaming would churn AC discovery commands for no behavioral gain); parameterizing the cross-product fixture into a shared helper (out of scope — the plan's FIX disposition is "merge preserving union of cases", not refactor; `core-parity`/`core-strengthen` own oracle changes); merging the ordering witness `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order` into the survivor (it asserts a materially different contract — strict-ascending canonical order over the `alpha_semantic`/`zeta_semantic` fixture with a different aggregation map — and merging would weaken a distinct regression input); touching production `region_mapping.rs` (outside the approved test-only scope).

## Files in Scope (read + edit)

- `crates/slicer-core/tests/algo_region_mapping_tdd.rs` - role: sole edit surface, the duplicates' owner; expected change: delete the two duplicate `#[test]` fns and extend the survivor's doc comment.
- `docs/specs/test-quality-remediation-plan.md` §7 only - role: mandatory program bookkeeping; expected change: update only the `core` ledger row with partial region-mapping status, the survivor map, actual validations, and the next pending `core-support-dup-merges` gap.

## Read-Only Context

- `crates/slicer-core/tests/algo_region_mapping_tdd.rs` - lines 1–200 (helpers and imports), 1025–1150 (the survivor and the ordering witness), 1430–1524 (the two absorbed fns) - current 24-test inventory and the duplicate pairs.
- `crates/slicer-core/Cargo.toml` - lines 58–60 only - the `algo_region_mapping_tdd` `[[test]]` stanza with `required-features = ["host-algos"]`.
`crates/slicer-ir/src/slice_ir.rs` - lines 1695–1711 only - the `DEFAULT_REGION_MAP_CAP` constant whose doc comment carries the `∏(1 + K_i)` cardinality prose (provenance context for the absorbed formula; read-only, never edited).
- `docs/specs/test-quality-remediation-plan.md` - ranges §1, §4, §5.1, §6, §7 Ledger, Packet Queue/continuation-approval paragraphs only.

## Out-of-Bounds Files

- All `crates/slicer-core/src/**`, including `algos/region_mapping.rs`, `flow.rs`, and `lib.rs`.
- All other `crates/slicer-core/tests/*` files, including `flow_tdd.rs` and `bridge_false_site_gating_tdd.rs`.
- Parent plan sections other than the implementation-time §7 Ledger edit explicitly allowed above, `docs/07_implementation_status.md`, canonical backlog, other packet directories (including `core-flow-consolidation` and `core-bridge-dup-merge`), generated artifacts, `target/`, and `Cargo.lock`.
- `OrcaSlicerDocumented/...` - no inspection is applicable; if implementation discovers a parity claim requiring external inspection, stop and request a delegated reference lookup rather than inventing a path.

## Expected Sub-Agent Dispatches

- Question: confirm pre-edit discovery is exactly 24 tests including both duplicates and post-edit exactly 22 with both absorbed names absent; scope: `crates/slicer-core/tests/algo_region_mapping_tdd.rs`; return: `FACT` with the two counts; purpose: survivor-map proof and census reconciliation.
- Question: run the feature-correct exact target checks and gates; scope: commands in `requirements.md`; return: `FACT` pass/fail, with at most 20 failure lines; purpose: implementation validation. (No generation-time test execution.)
- Question: confirm both duplicates' bodies are strict assertion subsets of the survivor's body at implementation time (same fixture statements, `assert_eq!(region_map.entries.len(), 6);` and the six-chain `HashSet` equality with the `"expected cross-product chains"` message present in the survivor); scope: lines 1025–1150 and 1430–1524 of the target file; return: `FACT` listing which assertion lines are subsets; purpose: merge-direction proof.

## Data and Contract Notes

- IR/manifest contracts: none.
- WIT boundary: none.
- Determinism/scheduler constraints: none; the retained tests keep deterministic fixture inputs (fixed semantic names and `PaintValue` variants) and exact chain-set assertions; the survivor's `HashSet`-based set equality is order-insensitive by design, while the ordering witness retains order sensitivity for the canonical-order contract.

## Locked Assumptions and Invariants

- The survivor is `region_mapping_two_semantics_produces_cross_product_cardinality`; both absorbed names are removed from the file and from discovery.
- Post-merge discovery is exactly 22 tests; the two-test delta is the accounted census retirement, recorded in the ledger row rather than frozen here.
- The survivor keeps its exact assertion strength: analytic `entries.len() == 6` plus full six-chain set equality, its doc comment now additionally carrying the absorbed AC-4 formula prose alongside its pre-existing AC-3 enumeration wording.
- No new public symbol is introduced and no net-new export is expected.

## Risks and Tradeoffs

- A careless deletion could orphan imports or helpers; both absorbed fns are self-contained (no helpers of their own) and use only imports/helpers already used by the survivor, so no import or helper is expected to become unused. The `HashSet` import and `PaintValue`/`SemVer` usages remain exercised by retained tests.
- Merging by test name alone could lose a distinct input; the pre-grounded survivor map shows both duplicates are strict assertion subsets (one count-only, one set-only) of the survivor on the identical fixture, and the 21 remaining tests are provably distinct in fixture or assertion (ordering witness, disjoint objects, cap error, config interning, modifier and variant-id guards, per-tool overlays).
- Feature flags can make a narrow run appear green without compiling the intended target; `--features host-algos` and exact-name guards are mandatory in every command.
- The doc-comment extension could drift the `AC-9 (c) / AC-3 / AC-4:` alias line; the edit adds prose without rewording the existing line, and AC-3 of `packet.spec.md` pins both retained fragments.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: pre/post discovery census, `FACT` with two counts.

## Open Questions

None. `[FWD]` None — the survivor name, absorbed names, and merge direction are pinned by the plan and the audit directive; no implementer choice is required. `[BLOCK]` If either duplicate is discovered to differ from its packet-time premise at implementation time — a body that is not a strict assertion subset of the survivor, a divergent fixture, or an absorbed assertion not present in the survivor — stop the packet rather than silently changing either body; that would be a scope-changing premise error in the plan.
