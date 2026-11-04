# ADR-0073 — Full-source variant-chain identity and versioned specialist query packages

Status: **Accepted.** Window W governance, step G1 of packet
`layer-module-preparation_01_shared-selection-prerequisite` (2026-10-11). The
contract below is recorded **as implemented** by that packet: every symbol,
key and version named here was verified against the migrated tree at
authoring time. It supersedes the contested identity clauses of
[ADR-0059](./0059-support-families-and-anchored-entities.md) (Rulings 2–3) and
one factual location claim in
[ADR-0022](./0022-explicit-per-region-origin-for-perimeter-output-builders.md);
those supersessions are declared explicitly and dated in the amended ADRs
themselves, not silently rewritten.

## Context and contested clauses

[ADR-0059](./0059-support-families-and-anchored-entities.md) Ruling 2 keys host
merges on the declared identity `(family_id, global_layer_index, object_id,
region_id, anchor_z)` and resolves family ownership through
`family_assignments[(object_id, region_id)]`; Ruling 3 admits exactly one
entry per `(global_layer_index, object_id, region_id)`. Paint segmentation
(`paint_variant_region_id`,
`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) lets one base region
carry many ordered paint chains, and config/map-space addressing
(`map_space_region_id`, `crates/slicer-wasm-host/src/selection.rs`) folds
painted ids back onto the base id — sources that differ only by their ordered
chain are distinct full origins that none of those clause keys can express.

Separately, the shared core handles package was unversioned
(`slicer:ir-handles`), and the core view `paint-region-layer-view` carried the
support/lightning specialist accessors, so a shared record-shape change had no
versioned surface and every guest of the shared view acquired the specialist
accessors whether or not its stage consumed them. The exact-Z query seam
(`ExactZQueryService::query` / `QueryKey`,
`crates/slicer-wasm-host/src/exact_z_query.rs`) identified a cached query by
object/region/Z only, exposing no source chain in the query identity even
though collision occupancy and the baseline envelope are object-wide.

## Decision — full source identity and the versioned split, as implemented

1. **Aggregate/family/admission identities retain every former dimension plus
   the ordered variant chain.**
   - `SupportAnalysisIR::family_assignments` is
     `BTreeMap<SupportSourceKey, String>` keyed on the full source origin
     `(object_id, region_id, variant_chain)` via `SupportSourceKey`
     (`crates/slicer-ir/src/slice_ir.rs`; the key deliberately carries no
     layer coordinate). `check_ownership`
     (`crates/slicer-wasm-host/src/support_aggregation.rs`) resolves the owner
     by that full key and admits an entry only when the assigned family
     matches the entry's `family_id` AND the producing module's manifest
     claims hold `support-family:<family_id>`; anything else is dropped with
     warn code `1206` — default-deny ownership, never arrival-order
     arbitration.
   - `union_same_family_entries` (same crate) merges same-family
     contributions on the declared identity `(family_id, global_layer_index,
     object_id, region_id, variant_chain, anchor_z)`: every former dimension
     retained, with the ordered chain added as one more dimension. Scalar
     attribution comes from the first entry; geometry, body identities and
     demands accumulate without duplicates.
   - Admission at the host aggregation seam
     (`try_aggregate_support_plans_with_policy`, same crate) keys the
     conflict/admission identity on the full source origin
     `(global_layer_index, object_id, region_id, variant_chain)`: entries for
     two chain-differing regions of one object never collapse into one row.
     Repeated identities within one family combine into exactly one retained
     entry; cross-family claims on one source origin are settled by
     default-deny ownership. The chain-independent whole-plan duplicate guard
     `SupportPlanIR::duplicate_region_identity`
     (`crates/slicer-ir/src/slice_ir.rs`), checked by
     `Blackboard::commit_support_plan`
     (`crates/slicer-runtime/src/blackboard.rs`), is unchanged as the prepass
     commit-seam invariant.

> **Amendment — 2026-10-13 (packet `layer-module-preparation_01_shared-selection-prerequisite`,
> final-review remediation F3).** The bullet ending above ("The
> chain-independent whole-plan duplicate guard … is unchanged as the prepass
> commit-seam invariant") is superseded by the same packet's IR mandate that
> `duplicate_region_identity` include the ordered chain in the returned
> duplicate identity. The guard is now chain-bearing
> (`crates/slicer-ir/src/slice_ir.rs`), both `Blackboard::commit_support_plan`
> branches (`crates/slicer-runtime/src/blackboard.rs`) enforce it, and
> `BlackboardError::DuplicateSupportPlanEntry` surfaces the rejection.
> Chain-empty sources keep the exact prior pair-keyed semantics (empty chain
> is the identity of a chain-less source, never a wildcard). Evidence:
> `support_plan_merge_rejects_duplicate_region_identity`
> (`crates/slicer-runtime/tests/unit/blackboard_support_geometry_slot_tdd.rs`)
> and the packet's AC-R8 chain-distinct witness. Recorded explicitly per the
> no-silent-ADR-rewrite rule.
2. **Object-wide exact-Z geometry vs source-specific ownership.** The exact-Z
   query now carries the ordered source chain as identity while the geometry
   it returns stays object-wide: `exact-z-support-query.query` takes
   `(object-id, region-id, variant-chain, physical-z-mm)` (the four-argument
   form in `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`),
   and `ExactZQueryService::query` / `QueryKey`
   (`crates/slicer-wasm-host/src/exact_z_query.rs`) include the ordered chain
   — order-sensitive — in the cache identity; an empty chain is the exact
   identity of a genuinely chain-less source, never a wildcard. Occupancy and
   the bounding envelope remain object-wide computations over the object's
   world mesh. `validate_entry`
   (`crates/slicer-wasm-host/src/support_aggregation.rs`) supplies the plan's
   full source identity to the query; `TerritoryClipper`
   (`crates/slicer-wasm-host/src/support_territory.rs`) separately enforces
   source-specific territory ownership by full source key. Object-wide
   collision geometry is not permission for pair-only ownership or config
   lookup.
3. **Typed shared/specialist version split with a clean break.** Core handles
   live in versioned `slicer:ir-handles@1.0.0`
   (`crates/slicer-schema/wit/deps/ir-types.wit`); specialists own separate,
   independent versioned packages `slicer:support-queries@1.0.0`
   (`support-queries` interface, `support-query-view` resource,
   `crates/slicer-schema/wit/deps/support-queries/support-queries.wit`) and
   `slicer:lightning-queries@1.0.0` (`lightning-queries` interface,
   `lightning-query-view` resource,
   `crates/slicer-schema/wit/deps/lightning-queries/lightning-queries.wit`),
   importing version-qualified core `object-id` / `region-id` / `paint-value`
   plus the existing unversioned geometry package (whose version is not
   changed). Core does not import specialists. Breaking record-shape changes
   take MAJOR versions: `slicer:layer-infill@2.0.0`,
   `slicer:layer-support@2.0.0` and `slicer:prepass-support-geometry@2.0.0`,
   matching the canonical package declarations and the `StageSpec` rows in
   `crates/slicer-schema/src/lib.rs`. `paint-region-layer-view` no longer
   carries any specialist accessor; it exposes only core accessors
   (`get-regions`, `get-custom-regions`, `layer-index`, `is-raft`), and the
   specialist query resources are allocated separately per layer with
   full-origin keys — `build_paint_layer_data_with_plan`
   (`crates/slicer-wasm-host/src/dispatch.rs`) returns the paint data plus
   `SupportQueryViewData` and `LightningQueryViewData`, whose maps are keyed
   by the full origin `(object_id, region_id, variant_chain)` with the chain
   taken from the IR entries' own `variant_chain` fields, never guessed or
   derived. The prepass region-origin record
   `region-source-origin { region-id, variant-chain }` and its
   `region-origins` list inside `region-segmentation-view-entry` (same WIT)
   are mirrored SDK-side by `RegionSourceOrigin` and `region_origins`
   (`crates/slicer-sdk/src/prepass_types.rs`).
4. **Clean-break evidence, as committed.** Three receipts make the break
   observable: (i) the EB01 decode receipt — the rebuilt dragon-curve
   example component decodes with `slicer:ir-handles@1.0.0` +
   `slicer:support-queries@1.0.0` + `slicer:lightning-queries@1.0.0` imports
   and the `slicer:layer-infill@2.0.0` `run` export (recorded in the packet's
   committed step history); (ii) the W11 OLD-rejection witness — the frozen
   pre-split fixture
   `crates/slicer-wasm-host/tests/fixtures/pre-split-layer-infill.component.wasm`
   (unversioned core imports, same-stage export, captured immutable before
   migration with a wasm-tools provenance receipt) rejects the migrated
   production host at typed instantiation before invocation, proven by
   `versioned_query_resources_load_and_old_components_reject`
   (`crates/slicer-wasm-host/tests/contract/source_identity_repair_tdd.rs`);
   (iii) the committed step receipts for E1, W10–W13 and the STAGES metadata
   migration, each recording its own verification exit codes, re-derivable
   from the packet's `layer-module-preparation_01` commit history. No
   dual-version adapter or old-version runtime compatibility exists: the
   frozen fixture is test evidence, not a supported ABI.

## What this decision does not decide

- **Decision vs historical report.** The step receipts record what the packet
  did and how each step was verified; this decision states the standing
  contract. Only the clauses above are normative — receipt wording is not.
- **No normative source-clipping geometry mandate.** Object-wide physical
  collision occupancy, anchor and global-layer policy, and opaque
  body/demand/entity identities are unchanged ([ADR-0059](./0059-support-families-and-anchored-entities.md),
  retained). Source-specific ownership is enforced by the full-source keys —
  assignment, admission identity, territory and query identity — never by
  clipping or source-partitioning the object-wide geometry algorithm.
- **No preparation-lifecycle scope.** Private Layer-module preparation
  remains governed by
  [ADR-0066](./0066-private-layer-preparation-capability.md). This decision
  touches only shared-query and production-output identity/versioning
  contracts.

## Cross-references

- [ADR-0059](./0059-support-families-and-anchored-entities.md), Ruling 4 —
  explicitly supersedes the Ruling 2/Ruling 3 identity dimensions with the
  full-chain replacements recorded here (every former dimension retained; the
  ordered chain added).
- [ADR-0022](./0022-explicit-per-region-origin-for-perimeter-output-builders.md),
  amendment 2026-10-11 — factual correction: the output builders live in
  versioned `slicer:ir-handles@1.0.0`, not an unversioned package; the
  full-origin principle of that ADR is retained, fuller now (chains carried
  everywhere).
- Packet contract:
  `docs/spec_packets/layer-module-preparation_01_shared-selection-prerequisite/design.md`
  — §"Coordinated WIT/SDK/IR contracts" and §"Additional confirmed support
  identity seams".