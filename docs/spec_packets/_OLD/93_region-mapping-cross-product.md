---
status: implemented
packet: 93
task_ids: [TASK-243]
---

# 93_region-mapping-cross-product

## Goal

Add cross-product expansion of `RegionMapIR.entries` keyed by `(layer × ActiveRegion × variant_chain)`, populating `RegionKey.variant_chain` from the per-object cross-product of paint values present in `aggregated_region_split` (D2 + D5 + D15). Migrate the existing per-region config-overlay flow at `region_mapping.rs:494` to derive overlays from the chain instead of from `overlapping_semantics_for_region`'s layer-wide semantic stamping; that function and its call site are deleted, with the new chain-derived path's empty-chain case standing in for the empty-aggregation fallback.

## Problem Statement

After P91 (schema) and P92 (manifest + dispatch), the IR has a `variant_chain` slot on `RegionKey` and the host knows how to dispatch by it — but no code populates it. `execute_region_mapping_inner` at `crates/slicer-core/src/algos/region_mapping.rs:384` produces one `RegionPlan` per `(layer, ActiveRegion)` with empty `variant_chain` for every entry. The chain dimension simply does not exist in the kernel today.

Today's `overlapping_semantics_for_region` (lines 286-319) computes config overlays at layer-wide granularity — it returns the set of paint semantics present anywhere on that layer, and the caller at line 494 applies those overlays to every `ActiveRegion` of every object on that layer. This is fit-for-purpose under the current single-entry-per-region model, but the cross-product model (D2) demands chain-keyed overlays: a `RegionPlan` with `variant_chain = [(material, ToolIndex(2))]` needs the `material:2` overlay specifically, NOT every `material:*` overlay present on the layer. The existing function's layer-wide return shape cannot express that.

OrcaSlicer's `PrintApply.cpp:1138-1156` solves the symmetric problem by emitting one `PaintedRegion` per `(volume_region × extruder_index)` combination present on the object. The shape we adopt is analogous: one `RegionPlan` per `(ActiveRegion × variant_chain)` element, where `variant_chain` ranges over the canonical cross-product of paint values present on the object across all semantics in `aggregated_region_split`.

This packet extends the kernel to:
- Accept `aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>` from the scheduler (P92's output).
- Scan each `mesh.objects[*].paint_data` once, collecting distinct `PaintValue`s per opted-in semantic per object.
- For each `(layer, ActiveRegion)`, enumerate the canonical cross-product (including the empty subset = base region) and emit one `RegionPlan` per chain. Per-variant polygons remain empty in this packet — P95 fills them later.
- Intern `ResolvedConfig`s via the P91 helper so the 16-color × 16-variant case does not replicate full configs in `RegionMapIR.configs`.
- Derive each chain's overlay by folding the matching paint-semantic `ResolvedConfig`s onto the modifier-stamped base via `overlay_resolved` (line 110). This is the chain-derived overlay path that replaces `overlapping_semantics_for_region`'s layer-wide derivation.
- Delete `overlapping_semantics_for_region` and its call site at line 494. The chain-derived path with `chain = []` (the empty chain emitted for every region when `aggregated_region_split.is_empty()`) reproduces the deleted path's output exactly — there is no need for a separate fallback.
- Preserve `stamp_modifier_config_deltas` (line 217); modifier-volume stamping composes with the chain dimension as the base on which paint-semantic overlays fold.

Because no production module declares `[[region_split]]` in P93's scope (P95's task), `aggregated_region_split` is empty by default and the cross-product collapses to the empty chain only — production behavior is preserved. The byte-identical g-code check (AC-10) is the integration-level verification of that equivalence.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Determinism invariant: `enumerate_canonical_chains` MUST produce the same output sequence for the same input. Use BTreeMap for the canonical-order axis; use a stable `PaintValue` comparator for tied semantics. Without determinism, `RegionMapIR.entries` becomes order-dependent and tests flake.
- Cross-product cardinality invariant: for an object with N opted-in semantics each carrying K_i distinct values, the per-(layer, ActiveRegion) cross-product cardinality is `∏(1 + K_i)` (the +1 accounts for "this semantic not present in this chain"). For a 16-color object (1 semantic, 16 values), that's 17 chains per region per layer. For 16-color × fuzzy_skin flag, that's 17 × 2 = 34 chains.
- Empty-aggregation invariant: when `aggregated_region_split.is_empty()`, the cross-product collapses to one element (the empty chain) per region. `RegionMapIR.entries` cardinality matches the pre-packet pre-cross-product behavior. The chain-derived overlay path's empty-chain case reproduces the deleted `overlapping_semantics_for_region` output (AC-7b, AC-10).
- D15: emit empty-polygon `RegionPlan` entries unconditionally. The kernel does NOT gate by geometric coverage; **empty-polygon filtering is P95's responsibility** — polygons live on `SlicedRegion`, populated by P95 via `replace_slice_ir`, and P95 has the polygons in hand to decide emptiness.

## Data and Contract Notes

- IR or manifest contracts touched: `RegionMapIR.entries` cardinality changes when `aggregated_region_split` is non-empty (cross-product expansion). No new IR type. No WIT change.
- WIT boundary considerations: none. Guest WASM rebuilds because `slicer-ir` is a guest dep (the `DEFAULT_REGION_MAP_CAP` constant change propagates), but no contract reshapes.
- Determinism or scheduler constraints: cross-product enumeration order is the contract. Test fixtures lock specific orderings; flipping the order in a future refactor breaks tests.

## Locked Assumptions and Invariants

- **Empty aggregation → empty chains → equivalent overlay**: when `aggregated_region_split.is_empty()`, the cross-product produces exactly one element (the empty chain) per region. The chain fold with `chain = []` produces a `ResolvedConfig` byte-identical to the deleted `overlapping_semantics_for_region`-driven path. Production behavior is preserved (AC-7b at the unit level, AC-10 at the g-code level).
- **Cross-product cardinality bound**: `∏(1 + K_i)` over opted-in semantics, where K_i is distinct values present on the object.
- **DEFAULT_REGION_MAP_CAP = 750_000**: admits 16-color × 1000-layer × 16-region scenes without modifier stamping. Pathological scenes beyond this surface the top-contributor diagnostic per AC-N2. Modifier stamping can multiply by a small constant (each modifier-stamped variant interns its own `ResolvedConfig`); revisit the cap if profiling shows realistic scenes hitting it.
- **Empty-polygon entries persist (D15)**: P93's kernel emits `RegionPlan`s with empty per-variant polygons unconditionally. The filter lives at P95 where polygons are in hand (`replace_slice_ir` populates them).
- **`overlapping_semantics_for_region` is fully deleted**: no fallback retained. The chain-derived overlay path's empty-chain case IS the fallback.
- **Defensive Scalar rejection at runtime**: the kernel rejects `PaintValue::Scalar` in the scan even if the manifest validator was bypassed. Double-defense against D13 violation.
- **Interner identity**: two `ResolvedConfig`s that are `==` produce the same `ConfigId`. The linear-scan interner is the agreed implementation (HashSet upgrade deferred).

## Risks and Tradeoffs

- **Risk: the AC-7b overlay-equivalence assertion fails** because the chain fold differs from `overlapping_semantics_for_region` in a subtle way (e.g., paint-semantic priority handling). Mitigation: the unit test compares against a recorded pre-packet fixture; any divergence is investigated, not waved off.
- **Risk: AC-10 byte-identical check fails** because deletion changes iteration order in `RegionMapIR.entries`. Mitigation: the kernel asserts on key membership, not iteration order; the g-code emission is order-stable by design.
- **Risk: the cap diagnostic identifies the wrong top contributor** when entries are evenly distributed. Mitigation: ties broken by `ObjectId` lex order; documented in the diagnostic helper's doc-comment.
- **Tradeoff: O(N) intern vs O(1) HashSet intern.** Linear scan is simpler and correct; profile-driven upgrade deferred until measurements justify.
