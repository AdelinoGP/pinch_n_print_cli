# Proptest pre-pass evidence (packet `proptest-openrscad_11_pbt-prepass`)

Measured, source-revision-stamped evidence for the four assigned families
(`H-REGIONMAP`, `H-PAINT-*`, `H-SUPPORT-*`) and the delegated
`H-LIGHTNING-*` module obligation. This file records what was **executed**.
Anything not executed is listed under [Not yet executed](#not-yet-executed)
rather than described as passing, and the packet remains `draft`.

## Source and environment provenance

- `git rev-parse HEAD` at baseline capture:
  `61fb47bf9f4f706f07e74a862f654b367054306e`, working tree clean
  (`git status --porcelain` empty) before command 1 of the baseline.
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`,
  `cargo 1.96.0 (30a34c682 2026-05-25)`.
- Platform: Windows (win32, `x86_64-pc-windows-msvc`).
- Target dir: the workspace default `target/`; guest artifacts under the shared
  `target/guests/`.
- Working-tree state during the packet: the ordering correction and its tests
  described below are uncommitted changes on top of `61fb47bf`. Every number in
  this file was produced by the tree as it stands, not by `61fb47bf`.

## Baseline (pre-change) ledger

Captured before any edit with `set -o pipefail` and one log per command under
`target/pbt11-baseline/`. All twenty narrow test commands selected exactly one
test and reported `1 passed; 0 failed`.

| # | Test | Result |
|---:|---|---|
| 1 | `algo_region_mapping_tdd::region_mapping_two_semantics_produces_cross_product_cardinality` | PASS |
| 2 | `algo_region_mapping_tdd::modifier_parent_bound_only_applies_to_effective_parameter_footprints` | PASS |
| 3 | `algo_region_mapping_tdd::region_mapping_chains_ordered_by_aggregated_region_split_canonical_order` | PASS |
| 4 | `algo_region_mapping_tdd::region_mapping_per_tool_config_overrides_paint_semantic_on_same_key` | PASS |
| 5 | `region_split_registry_tdd::enumerate_canonical_chains_empty_input_returns_single_empty_chain` | PASS |
| 6 | `region_split_aggregation_tdd::region_split_aggregation_canonical_order` | PASS |
| 7 | `driver_v2_tests::declared_region_split_composition_preserves_ordered_chains` | PASS |
| 8 | `compose_variants::compose_two_semantics_produces_4_variants_at_full_overlap` | PASS |
| 9 | `compose_variants::compose_variant_keys_are_deterministic_via_btreemap` | PASS |
| 10 | `paint_segmentation_base_fallback_tdd::paint_base_fallback_uses_own_object_contours` | PASS |
| 11 | `paint_segmentation_multi_object_isolation_tdd::base_region_must_not_carry_other_objects_contours` | PASS |
| 12 | `overhang_annotation_ramp_tdd::ramp_overhang_partitions_into_correctly_ordered_bands` | PASS |
| 13 | `algo_support_geometry_tdd::build_emit_schedule_two_objects_per_object_semantics` | PASS |
| 14 | `support_overhang_detection_tdd::a_region_sitting_on_a_different_region_below_is_not_a_contact` | PASS |
| 15 | `support_interface_regularize_tdd::roof_and_floor_do_not_overlap` | PASS |
| 16 | `region_mapping_resolved_config_tdd::commit_stamps_per_object_resolved_config` | PASS |
| 17 | `support_analysis_producer::tests::support_analysis_populates_all_derivable_inputs` | PASS |
| 18 | `kernel::tests::lightning_generator_overhangs` | PASS |
| 19 | `kernel::tests::source_ordered_chains_remain_distinct_in_generated_lightning` | PASS |
| 20 | `property_domain_contracts::domain_exact_predicates_counterexamples` | PASS |
| 21 | `cargo xtask check-literals` | PASS — `0 violation(s) in 0 file(s) (watchlist: 196 types)` |
| 22 | `cargo xtask build-guests --check` | exit 1 — 49 `STALE:` lines, every one `fingerprint mismatch`, zero lock-divergence lines |

Commands 1–9 finished before the first source edit and are certified
pre-change. Commands 10–20 finished while edits were landing; the two
ordering-sensitive ones (7, 8, 9) are in the certified pre-change set, and the
rest are re-run below on the final tree. Command 22 is **not** a usable
baseline: the guest fingerprint hashes `crates/*/src` of each guest's path
closure, so it registered the in-flight `slicer-core/src` edits. Its
post-change state is what matters and is reported under Gates.

## Live seams confirmed on disk (Step 1)

- `commit_region_mapping_builtin` (`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`)
  → `execute_region_mapping_inner` (`crates/slicer-core/src/algos/region_mapping.rs`)
  → `enumerate_canonical_chains` (`crates/slicer-ir/src/region_split_registry.rs`).
- `canonical_variant_chain_order` (`crates/slicer-scheduler/src/region_split.rs`)
  still has no production caller; it returns the same `(priority, name)` sequence
  the kernel now computes locally.
- `execute_paint_segmentation` / `compose_variants`
  (`crates/slicer-core/src/algos/paint_segmentation/`) and
  `region_map_key_for_sliced_region` / `build_layer_selection_index` /
  `config_for` (`crates/slicer-wasm-host/src/selection.rs`).
- `resolve_scope_stack` (`crates/slicer-config/src/resolution.rs`) applies
  paint semantics in sorted lexical order and the tool scope last — the
  independent reason the compatibility overlay order must not follow chain order.
- `CORE_REGION_SPLIT_PRIORITIES` (`crates/slicer-schema/src/lib.rs`) is
  `material = 100`, `fuzzy_skin = 200`; `COMMUNITY_PRIORITY_FLOOR = 1000`.
- ADR-0018 already carried packet #7's 2026-11-13 amendment, so this packet took
  the "clarification landed first" branch of the spec's #7 sequencing: the
  2026-11-19 amendment supersedes its *current-driver* observation and preserves
  its container, helper and tied-priority-payload facts.

## The ordering correction (Step 3)

Three coordinated production changes, all source-derived (not from an executed
failure):

1. `execute_region_mapping_inner` sorts the aggregate's `(entry.priority, name)`
   pairs locally before calling `enumerate_canonical_chains`. No new
   `slicer-core` → `slicer-scheduler` dependency, no call to the helper.
2. The compatibility paint-overlay loop applies `paint_semantic_configs` in
   **lexical semantic namespace order** (one key per namespace spelling, first in
   `BTreeMap` order), with the tool overlay still last. Before this, the overlay
   order came from the chain, so reordering the chain alone would have flipped
   the resolved width.
3. `compose_variants` groups axes in the caller's **first-occurrence** order
   instead of name order, and `execute_paint_segmentation` supplies each parent's
   inputs in the order recovered from that object's maximal committed chains on
   that layer (longest-first, deterministic tie-break on the chain vector, once
   per `(layer, object)` so the join is `O(entries)` and not
   `O(regions × entries)`). Axes are an object-level declaration and every
   maximal chain under one object carries the same sequence, so this never
   borrows another object's axes; a map with no chains at all falls back to the
   per-semantic discovery order rather than a name sort.

Fixture updates that make the expectation discriminating (the old
equal-priority fixtures could not distinguish the orders):

- `region_mapping_two_semantics_produces_cross_product_cardinality` now declares
  `material` 100 / `fuzzy_skin` 200 and expects the combined chain
  material-then-fuzzy.
- `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order` now
  declares `zeta` 1000 / `alpha` 1100, so the canonical order is the reverse of
  the name order.
- `declared_region_split_composition_preserves_ordered_chains` uses
  material-first overlap vectors and an independent `(priority, name)` rank
  roster instead of the lexical `windows(2)` assertion.

## Deterministic controls

All four core controls pass in one target:

```text
cargo test -p slicer-core --features property_testing,host-algos --test property_prepass_controls
test prepass_model_controls ... ok
test prepass_owned_shrink_and_bound_controls ... ok
test prepass_regionmap_rejection_controls ... ok
test prepass_regionmap_order_and_cap_controls ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

What they establish, with the real system under test:

- `prepass_model_controls`: the checker accepts the live material/fuzzy
  observation and rejects nine named mutations — dropped empty chain, dropped
  combined chain, name-order substitution, substituted value, duplicated key,
  wrong config, non-idempotent interner, truncated pool, cap success.
- `prepass_regionmap_order_and_cap_controls`: the four named chains exactly;
  community order `zeta(1000) → alpha(1100) → beta(1100)` with the lexical tie;
  compatibility overlay width `0.75` on the combined key (fuzzy 0.5 applied
  before material 0.75) and `1.0` with a tool-0 overlay (tool last); exact-count
  cap succeeds and `cap = total - 1` reports `entry_count = total`, `cap =
  total - 1`, plus the empty-plan/`cap = 0` mode and a unique top contributor.
- `prepass_regionmap_rejection_controls`: `ScalarInRegionSplitFacetValue` with
  its exact `object_id`/`semantic`/`scalar_bits` fields; a duplicated full
  active identity reporting `DuplicateRegionKey` with the offending key;
  identical modifier footprints grouping into exactly one minted sub-region; an
  unsupported base parent (`1 << 50`) reporting
  `ModifierParentRegionIdOutOfRange`; a **real** distinct-footprint
  `modifier_sub_region_id` collision found by a birthday search over 4096
  sliced cube footprints (payload space 1_000_002) and rejected as
  `DuplicateRegionKey`, with the pair retained as concrete coordinates (two
  0.5 mm cubes at x = 20.87 and x = 24.32 mm, see `packet.spec.md` deviation 4); and the painted-id contract — the region map keeps both
  `material`/`ToolIndex(0)` and `material`/`ToolIndex(999_999)` because its keys
  carry the full chain, while `execute_paint_segmentation` rejects the shared
  payload 1 with `PaintVariantRegionIdCollision` naming both chains.

That control also corrected a stale source comment in `paint_variant_region_id`
(`crates/slicer-core/src/algos/paint_segmentation/mod.rs`): it claimed both the
region-map and segmentation callers reject colliding synthesized ids, but only
the segmentation caller does.
- `prepass_owned_shrink_and_bound_controls`: `(1+3)^3 = 64` and `(1+4)^5 = 3125`
  are accepted as exact bounds and one below each is rejected before sampling.

Runtime controls (the cross-stage join):

```text
cargo test -p slicer-runtime --features property_testing --test property_prepass_controls
test prepass_regionmap_runtime_wrapper ... ok
test prepass_paint_map_identity_integration ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`prepass_paint_map_identity_integration` builds a real open square tube with
`material`/`ToolIndex(0)` and `fuzzy_skin`/`Flag(true)` on every wall facet,
commits the map through `commit_region_mapping_builtin` with a real
`RegionResolutionAuthority` (layer range covering `z = 0.5`), runs
`execute_paint_segmentation`, and then `build_layer_selection_index`. It asserts:
the combined painted region is nonempty and its chain is exactly
`[("material", ToolIndex(0)), ("fuzzy_skin", Flag(true))]`; its map-space key
exists and is the base address `region_id = 0`; the source-space selected key
keeps the minted id and the unchanged chain; the selected config view carries
`wall_count = 7` and `infill_density = 0.5` while BASE carries `2` / `0.25`.
Both negative mutations — reversing the combined chain in the real slice, and
removing only the combined map entry — return
`SelectionError::MissingModelConfig` with the exact map-space key and the
`Layer::Infill` / `com.pbt.identity` identity.

## Registry execution (`H-LIGHTNING-SEEDS` and `H-LIGHTNING-TREES`)

```text
cargo xtask test --summary --properties --property H-LIGHTNING-SEEDS --tier routine|extended
test result: ok. 1 passed; 0 failed; ... 0.03s | 0.08s
VERDICT: PASS

cargo xtask test --summary --properties --property H-LIGHTNING-TREES --tier routine|extended
test result: ok. 1 passed; 0 failed; ... 0.05s | 0.17s
VERDICT: PASS
```

Both run as **lib** tests in the module-private kernel
(`kernel::property_tests`), gated by `#[cfg(all(test, feature = "property_testing"))]` so the guest
build and a default native test run never compile them. The seed model re-derives
`erode(current) - above` analytically and classifies the kernel's flat ring list with an even-odd
test; the tree model walks the forest with a pointer-identity visited set, checks root ownership,
termination, the `max_tree_nodes` budget and repeated-run equality, and bounds points by a declared
4x4-domain margin rather than the outline (trees branch outside their outline). Three controls pass
in the lib target: `lightning_model_controls`, `lightning_seed_tree_controls` and
`lightning_owned_shrink_controls`.

## Registry execution (`H-SUPPORT-ANALYSIS`)

```text
cargo xtask test --summary --properties --property H-SUPPORT-ANALYSIS --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.04s
VERDICT: PASS

cargo xtask test --summary --properties --property H-SUPPORT-ANALYSIS --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 0.12s
VERDICT: PASS
```

The real producer runs on a fresh blackboard with layered rectangles and exact-chain regions.
Occupancy keys and polygons, the per-object feasible envelope rectangles, the top-layer termination
sets and the literal territory clearance all agree with an independent model; the disabled mode
publishes settings only. Candidate selection is deliberately not asserted (it depends on surface
classification this property does not fabricate), and the percent-mode clearance is checked
separately against the documented percentage-of-fallback formula.

## Registry execution (`H-SUPPORT-TERRITORY`)

```text
cargo xtask test --summary --properties --property H-SUPPORT-TERRITORY --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.48s
VERDICT: PASS

cargo xtask test --summary --properties --property H-SUPPORT-TERRITORY --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 2.13s
VERDICT: PASS
```

The host clipper is driven through `from_ir`/`clip` with one to three territory rectangles, a
separate query body, clearances `0`/`0.2`/`0.4` mm and both minted-origin and base-region queries.
Membership is checked analytically (own intersection, or body minus every inflated foreign
rectangle) at sample points offset 2500 units from every edge, so no sample can land on a
boundary. `from_ir` on an empty analysis is `None`, a territory-free layer clips to `None`, and a
distinct region id does not match a minted entry.

## Registry execution (`H-SUPPORT-CONTACT`)

```text
cargo xtask test --summary --properties --property H-SUPPORT-CONTACT --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.03s
VERDICT: PASS

cargo xtask test --summary --properties --property H-SUPPORT-CONTACT --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 0.09s
VERDICT: PASS
```

On the enforced-layer plain-difference branch the contact set is
`upper - lower - blockers`, checked cell by cell on a millimetre-scale lattice. The deterministic
modes cover the `0.01 mm` sliver (dropped by the tiny-spot erosion), a fully supported column, a
blocker-covered contact and an empty upper region. A first attempt placed the cells in raw
canonical units, where the `0.04 mm` erosion emptied every fixture — the fixture is now 4 mm x 8 mm
per cell, and that modelling error is recorded here rather than hidden.

## Registry execution (`H-SUPPORT-ROLES`)

```text
cargo xtask test --summary --properties --property H-SUPPORT-ROLES --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.14s
VERDICT: PASS

cargo xtask test --summary --properties --property H-SUPPORT-ROLES --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 0.27s
VERDICT: PASS
```

Grid-style (`smooth_supports = false`) set algebra: each cell of an even lattice is owned by exactly
its declared role, the output role order follows the input's first-appearance order with the body
remainder appended when it was absent, and the three documented `None` modes (nonpositive spacing,
body-only, no interface) return `None`. Top/base aliasing is pinned separately.

## Registry execution (`H-SUPPORT-SCHEDULE`)

```text
cargo xtask test --summary --properties --property H-SUPPORT-SCHEDULE --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.03s
VERDICT: PASS

cargo xtask test --summary --properties --property H-SUPPORT-SCHEDULE --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 0.31s
VERDICT: PASS
```

The schedule, the exact key set, the empty-source present keys, the `u32::MAX` sentinel entries and
the IR-wide settings all agree with an independently re-derived model. The empty plan produces no
schedule and no entries. `prepass_support_schedule_controls` refutes a dropped emission, a wrong
object, a wrong chain and a false empty map, and confirms that a scheduled region with no committed
source really is a present empty key.

## Registry execution (`H-PAINT-DRIVER`)

```text
cargo xtask test --summary --properties --property H-PAINT-DRIVER --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.22s
VERDICT: PASS

cargo xtask test --summary --properties --property H-PAINT-DRIVER --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 1.05s
VERDICT: PASS
```

Objects are disjoint 2 mm square tubes, each painting a drawn subset of `material` and
`fuzzy_skin`; the checker proves every output point stays inside its own object's square, each
object's chain set is its own widest chain plus BASE, BASE and the painted chain are disjoint on a
4x4 interior grid, ids follow the documented scheme (BASE keeps the base id; `material`
`ToolIndex(0)` mints payload 1), repeated runs agree, and the three unchanged-input paths return
the caller's `Arc`. The deterministic block also drives the real
`PaintVariantRegionIdCollision` with `ToolIndex(0)`/`ToolIndex(999_999)`.

## Registry execution (`H-PAINT-OVERHANG`)

```text
cargo xtask test --summary --properties --property H-PAINT-OVERHANG --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.33s
VERDICT: PASS

cargo xtask test --summary --properties --property H-PAINT-OVERHANG --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 1.24s
VERDICT: PASS
```

The X-strip fixture puts `current \ previous` on a strip whose band boundaries are exact X
intervals. Every classified cell midpoint is inside exactly its analytic band, the quartile set
equals the scene's observable set, band 4 extends past `2.0 x line width`, and the three
no-overhang modes leave both maps without the layer key. `prepass_overhang_band_controls` refutes
swapped quartiles, a substituted previous-boundary footprint and a fourth band capped at `2.0 x`.

## Registry execution (`H-PAINT-COMPOSE`)

```text
cargo xtask test --summary --properties --property H-PAINT-COMPOSE --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.05s (0.64 s on an earlier run)
VERDICT: PASS

cargo xtask test --summary --properties --property H-PAINT-COMPOSE --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 0.21s
VERDICT: PASS
```

The model builds an even-coordinate cell grid, derives each cell's chain from the declared labels
and an independently sorted `(priority, name)` order, and classifies every returned chain with
`oracles::point_in_region` at the cell midpoint; a `Boundary` outcome is a failure. Cell ownership,
the full supplied cross-product key set, and the total-extent check all agree on every case.
`prepass_paint_partition_and_driver_controls` refutes a lost BASE chain, a filled unpainted cell, a
name-ordered key, a false all-empty result and swapped ownership.

## Registry execution (`H-REGIONMAP`)

```text
cargo xtask test --summary --properties --property H-REGIONMAP --tier routine
test result: ok. 1 passed; 0 failed; ... finished in 0.10s
VERDICT: PASS

cargo xtask test --summary --properties --property H-REGIONMAP --tier extended
test result: ok. 1 passed; 0 failed; ... finished in 0.99s
VERDICT: PASS
```

Both tiers ran with zero local/global rejections and zero flat-map regenerations
(the strategy is total). No failure occurred, so no shrink ran and no failure
artifact was written. Artifacts live under
`target/property-testing/<run-token>/H-REGIONMAP/` with `context.json`,
`case.txt`, `observations.jsonl` and the controller `begin`/`end` events. Logs
are archived as `target/pbt11-h-regionmap-routine.log` and
`target/pbt11-h-regionmap-extended.log`.

Measured profile choice: routine 64 cases in 0.10 s and extended 256 cases in
0.99 s are far below the tier budgets, so the extended count was left at 256 with
the wider domain (`max_semantics = 4`) rather than raised for its own sake; the
widest reachable product (256) equals `max_chains`, so a well-formed case is
always compared in full.

## Ordering fallout re-runs on the final tree

| Test | Result |
|---|---|
| `algo_region_mapping_tdd` (22 tests) | PASS — 22 passed, 0 failed |
| `algos::paint_segmentation` lib (96 tests, 3 ignored) | PASS — 96 passed, 3 ignored |
| `paint_segmentation_base_fallback_tdd` | PASS — 1 passed |
| `paint_segmentation_multi_object_isolation_tdd` | PASS — 2 passed |
| `region_split_registry_tdd` (ir) | PASS — 3 passed |
| `region_split_aggregation_tdd` (scheduler) | PASS — 4 passed |
| `property_prepass_controls` (core) | PASS — 8 passed |
| `property_prepass_controls` (runtime) | PASS — 9 passed |

## Gates for the changed surface

| Gate | Result |
|---|---|
| `cargo clippy -p slicer-core --all-targets --features host-algos,property_testing -- -D warnings` | exit 0 |
| `cargo clippy -p slicer-runtime --all-targets --features property_testing -- -D warnings` | exit 0 |
| `cargo clippy -p slicer-wasm-host --all-targets -- -D warnings` | exit 0 |
| `cargo xtask check-literals` | exit 0 — `0 violation(s) in 0 file(s) (watchlist: 196 types)` |
| `cargo xtask check-fmt` | exit 0 — `62 package(s) clean` |
| `cargo xtask build-guests --check` | exit 0, zero `STALE:` lines after `cargo xtask build-guests` rebuilt all 49 guests |
| `cargo build --workspace` | exit 0 |

## Ownership handoffs

- **#13 (`pbt-infill`)** receives `H-LIGHTNING-SEEDS` / `H-LIGHTNING-TREES` as the delegated
  generator coverage its packet explicitly credits to #11; #13 adds no lightning IDs of its own.
- **#14 (`pbt-support-modules`)** receives the host analysis/territory evidence
  (`H-SUPPORT-ANALYSIS`, `H-SUPPORT-TERRITORY`) as an input to its planner/renderer families. It
  owns module-local `SupportGrid`, planner clipping/emission and renderers; this packet makes no
  planner-routing, decline-reason or impossibility claim.
- **#17 (`pbt-ci-closure`)** receives all eleven records and this file for its final audit: the
  registry now holds every ID its table lists for #11.

## Second review round

A delta re-review confirmed the seven earlier findings fixed and raised the lightning and runtime
gaps:

- **AC-8 lightning driver** — `kernel::property_tests` now drives
  `generate_lightning_trees` on two sibling sources that share `(object, region)` but differ by
  ordered chain: every plan entry names a declared source, the chained sibling keeps its own entry,
  and a non-`lightning-infill` `sparse_fill_holder` publishes nothing. A dedicated positive witness
  requires nonempty `tree_edge_segments` and confines every endpoint to the source extent, so a
  fake all-empty generator fails there.
- **AC-N2 lightning and runtime shrink walks** — `lightning_owned_shrink_controls` and the runtime
  `prepass_owned_shrink_and_bound_controls` now drive their real value trees through `complicate`
  and `simplify`, re-verifying every exposed state and requiring at least one simplification.
- **Runtime control inventory** — `prepass_model_controls` and
  `prepass_owned_shrink_and_bound_controls` now exist in the runtime target (9 controls total).
- **Analysis assertions** — top-layer termination entries must contain both their own footprint and
  the object's plate rectangle, and no candidate vertex may sit inside the supported lower region.
- **Module gating** — the lightning test module uses separately stacked `#[cfg(test)]` and
  `#[cfg(feature = "property_testing")]` attributes.
- **Driver identity oracle** — the synthesized payload of arbitrary chains is no longer compared
  against the production helper; only the namespace and the documented `ToolIndex(0) → 1` fast path
  are asserted.

## Review-driven corrections

An independent review of the first closure attempt returned CHANGES REQUESTED with seven findings.
All were addressed:

1. **AC-N2 shrink evidence** — `prepass_owned_shrink_and_bound_controls` now drives the real value
   trees (`Strategy::new_tree`) for all seven core families, calls `complicate` up to four times and
   `simplify` up to twelve, re-runs the same checker on every exposed state, and asserts that at
   least one simplification was observed. The narrowing-bound rejections remain.
2. **AC-7 candidate/no-candidate coverage** — the analysis model now derives the candidate set from
   same-object lower-layer coverage, asserts each candidate's full source identity and that its
   geometry stays inside its own source region, and three deterministic witnesses cover the
   same-object-overhang vs misleading-foreign-object scene, the fully-covered no-candidate mode and
   the surface-classification no-support mode. `prepass_support_analysis_territory_controls` now
   rejects four mutated real observations (wrong occupancy object, altered clearance, wrong
   candidate source, dropped candidate) instead of asserting positive cases.
3. **AC-2 overhang delivery** — `prepass_overhang_object_delivery` now asserts each object's own
   previous-boundary rectangle (bounds compared in millimetres) and that every band polygon stays
   inside that object's current rectangle, so a foreign-object or current-layer substitute fails.
4. **AC-N1 painted parent** — `prepass_regionmap_rejection_controls` exercises the painted-parent
   `ModifierParentRegionIdOutOfRange` branch with a base whose own id fits while the minted painted
   parent does not, asserting the reported key carries the chain and the minted id.
5. **AC-9 registry deviation** — recorded as `## Deviations` item 1 in `packet.spec.md`.
6. **AC-3 regression home** — `prepass_paint_community_priority_witness` was added beside
   `prepass_paint_map_identity_integration`, with the root-cause description in its doc comment.
7. **Collision witness** — the found pair is now pinned with retained coordinates (deviation 4).

## Remaining caveats

The packet is closed on the evidence above. Two limitations are recorded rather than hidden:

- `H-LIGHTNING-SEEDS` models axis-aligned rectangles (including the erosion-inside-outline and
  empty-above reset cases), not general polygons with holes; see `packet.spec.md` deviation 3.
- `docs/02_ir_schemas.md` carries the `H-SUPPORT-*` / `H-LIGHTNING-*` links; a deeper editorial
  pass over the retired-lightning section is not done.

The per-run artifacts are retained under `target/property-testing/<run-token>/<ID>/`
(`context.json`, `observations.jsonl`, `case.txt`, controller events) and the tier logs as
`target/pbt11-*.log`. One retained routine seed was replayed successfully
(`--seed 9339635094170354160`, `target/pbt11-replay.log`).
