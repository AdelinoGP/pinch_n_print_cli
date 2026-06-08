---
status: draft
packet: 93
task_ids: [TASK-243]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 93 — RegionMapping Cross-Product Expansion (variant_chain populated; polygons empty)

## Goal

Add cross-product expansion of `RegionMapIR.entries` keyed by `(layer × ActiveRegion × variant_chain)`, populating `RegionKey.variant_chain` from the per-object cross-product of paint values present in `aggregated_region_split` (D2 + D5 + D15). Migrate the existing per-region config-overlay flow at `region_mapping.rs:494` to derive overlays from the chain instead of from `overlapping_semantics_for_region`'s layer-wide semantic stamping; that function and its call site are deleted, with the new chain-derived path's empty-chain case standing in for the empty-aggregation fallback.

## Scope Boundaries

This packet extends `execute_region_mapping_inner` with the cross-product loop, updates the producer wrapper to thread `aggregated_region_split` from the scheduler into the kernel, and deletes `overlapping_semantics_for_region` along with its call site at line 494 (subsumed by the chain-derived overlay path). The existing `stamp_modifier_config_deltas` (line 217) and `overlay_resolved` (line 110) helpers are preserved and composed with the new chain dimension. No paint-segmentation, no mesh-segmentation, no module-manifest changes. Empty-polygon `RegionPlan` filtering is **out of scope** — P95 owns it (polygons live on `SlicedRegion`, populated by paint-segmentation). With no production module declaring `[[region_split]]` yet (P95's job), `aggregated_region_split` is empty by default and the cross-product collapses to the empty chain only, preserving production behavior. Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 91 (schema scaffolding) `implemented` and packet 92 (manifest + dispatch) `implemented`. P93 reads `aggregated_region_split` (P92) and interns configs via the P91 helper.
- Unblocks: P95 (paint-segmentation port) fills per-variant polygons via `replace_slice_ir` into the entries P93 emits, and owns the empty-polygon filter decision (deferred from this packet).
- Activation blockers: P91 and P92 both `implemented`. No internal blockers — `AUDIT.md`'s three findings were resolved in the refinement pass that produced this packet (§Audit 1: additive framing; §Audit 2: empty-polygon filter deferred to P95; §Audit 3: AC-9 rescoped to net-new kernel tests, AC-10 dropped).

## Acceptance Criteria

### AC-1 — `execute_region_mapping_inner` reads `aggregated_region_split` from the scheduler

**Given** the kernel extension,
**When** `execute_region_mapping_inner` at `crates/slicer-core/src/algos/region_mapping.rs:384` is inspected,
**Then** its signature (or a context object it consumes) carries `aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>` produced by `slicer-scheduler::region_split::aggregate_region_splits`. The kernel does NOT hardcode a list of opted-in semantics; the registry is the authoritative source.

| `rg -q 'aggregated_region_split|AggregatedRegionSplitEntry' crates/slicer-core/src/algos/region_mapping.rs && cargo check -p slicer-core 2>&1 | tee target/test-output.log`

### AC-2 — Per-object paint scan produces `HashMap<ObjectId, HashMap<String, Vec<PaintValue>>>` of distinct paint values present per opted-in semantic

**Given** the scan step,
**When** the kernel runs against a mesh whose object[0] has `paint_data.layers[*]` containing 4 distinct `PaintValue::ToolIndex` values for semantic `material`,
**Then** `painting_variants_per_object[object[0]]["material"]` contains exactly those 4 values (de-duplicated, deterministically ordered).

| `cargo test -p slicer-core region_mapping_paint_scan 2>&1 | tee target/test-output.log`

### AC-3 — `enumerate_canonical_chains` produces every subset of (semantic, value) pairs in canonical order, including the empty subset

**Given** an object with `material` values `{ToolIndex(1), ToolIndex(2)}` and `fuzzy_skin` values `{Flag(true)}`,
**When** `enumerate_canonical_chains` is invoked with the canonical order `["material", "fuzzy_skin"]`,
**Then** the returned `Vec<Vec<(String, PaintValue)>>` contains exactly the 6 chains: `[]`, `[("material", ToolIndex(1))]`, `[("material", ToolIndex(2))]`, `[("fuzzy_skin", Flag(true))]`, `[("material", ToolIndex(1)), ("fuzzy_skin", Flag(true))]`, `[("material", ToolIndex(2)), ("fuzzy_skin", Flag(true))]`. Order within each chain follows the canonical order (material before fuzzy_skin); order across chains is deterministic.

| `cargo test -p slicer-core region_mapping_enumerate_chains 2>&1 | tee target/test-output.log`

### AC-4 — `RegionMapIR.entries` contains one `RegionPlan` per (layer, ActiveRegion, variant_chain) cross-product element

**Given** a single-layer, single-active-region scenario with the chains from AC-3,
**When** RegionMapping runs,
**Then** `RegionMapIR.entries.len() == 6 × 1 × 1 = 6`; each `RegionPlan` is keyed by a `RegionKey` whose `variant_chain` matches one of the 6 chains from AC-3.

| `cargo test -p slicer-core region_mapping_cross_product_entry_count 2>&1 | tee target/test-output.log`

### AC-5 — Config interning: distinct ResolvedConfigs go into `configs: Vec`; equivalent configs reuse the same `ConfigId`

**Given** a scenario where two variant chains derive identical `ResolvedConfig`s (e.g., neither painted variant overrides any config key),
**When** the kernel interns,
**Then** both `RegionPlan.config` fields hold the SAME `ConfigId`; `RegionMapIR.configs.len()` does NOT count duplicates. The `intern_config` helper from P1a is the only path to populate the Vec.

| `cargo test -p slicer-core region_mapping_config_interning 2>&1 | tee target/test-output.log`

### AC-6 — Per-variant polygons remain empty (P3 fills them)

**Given** the cross-product expansion,
**When** each entry in `RegionMapIR.entries` is inspected,
**Then** any per-variant polygon field is empty / default. (Polygons live on `SlicedRegion`, not on `RegionPlan`; the assertion is that no `SlicedRegion` is created or modified by this packet — the SliceIR.regions remain in their pre-paint-segmentation shape.)

| `cargo test -p slicer-runtime --test executor cube_4color_paint_region_map_empty_polygons 2>&1 | tee target/test-output.log`

### AC-7 — `overlapping_semantics_for_region` and its call site at line 494 are DELETED

**Given** that the chain-derived overlay path subsumes the existing layer-wide overlay derivation,
**When** `crates/slicer-core/src/algos/region_mapping.rs` is inspected after this packet,
**Then** the function `overlapping_semantics_for_region` no longer exists in the file, and its call site at line 494 (the `let semantics = overlapping_semantics_for_region(...)` line) is gone — the chain-derived overlay path is the only remaining path that produces `effective_config` / `paint_overrides` on each `RegionPlan`.

| `! rg -q 'overlapping_semantics_for_region' crates/slicer-core/src/algos/region_mapping.rs && cargo test -p slicer-core region_mapping_chain_derived_overlay 2>&1 | tee target/test-output.log`

### AC-7b — Empty-aggregation overlay equivalence

**Given** `aggregated_region_split.is_empty()` (the production default until P95 declares a `[[region_split]]` semantic),
**When** the chain-derived overlay path runs with the only chain being `[]` (the empty chain),
**Then** the resulting `ResolvedConfig` per `RegionPlan` matches the `ResolvedConfig` that the deleted layer-wide `overlapping_semantics_for_region`-driven path produced for the same input pre-packet. The byte-identical g-code check in AC-10 (formerly AC-11) is the integration-level verification of this equivalence; a kernel-level unit test asserts the per-region `ConfigId` interner produces the same `ResolvedConfig` content.

| `cargo test -p slicer-core region_mapping_chain_derived_overlay_matches_layer_wide_overlay_when_aggregation_empty 2>&1 | tee target/test-output.log`

### AC-8 — `DEFAULT_REGION_MAP_CAP` raised from 1_000 to 750_000 with overflow diagnostic

**Given** that today's constant at `crates/slicer-ir/src/slice_ir.rs:1196` is `pub const DEFAULT_REGION_MAP_CAP: usize = 1_000` and cross-product expansion can multiply entry counts by `∏(1 + K_i)` per region,
**When** the constant location is inspected after this packet,
**Then** the value is `750_000`, the doc-comment explains the 750× headroom rationale (16-color × 1000-layer × 16-region × 3-modifier scenes), and the overflow diagnostic names the worst-contributing `object_id` in the structured-event output.

| `rg -q 'DEFAULT_REGION_MAP_CAP\s*[:=]\s*750_000' crates/slicer-ir/src/slice_ir.rs && cargo test -p slicer-runtime region_map_cap_overflow_diagnostic 2>&1 | tee target/test-output.log`

### AC-9 — Net-new kernel unit tests assert variant_chain shape against synthetic input

**Given** six net-new kernel unit tests under `crates/slicer-core/tests/algo_region_mapping_tdd.rs` that drive `execute_region_mapping_inner` with a synthetic mesh + synthetic `BTreeMap<String, AggregatedRegionSplitEntry>`,
**When** `cargo test -p slicer-core region_mapping` runs,
**Then** all six tests pass with exact `variant_chain` shape assertions on `RegionMapIR.entries`:
- `region_mapping_emits_empty_chain_for_unpainted_object` — object with no `paint_data` and any non-empty aggregation produces exactly one chain `[]` per `(layer, ActiveRegion)`.
- `region_mapping_emits_n_plus_1_chains_for_single_semantic_n_distinct_values` — object with `material` carrying `{ToolIndex(1), ToolIndex(2), ToolIndex(3)}` produces exactly 4 chains: `[]`, `[material:1]`, `[material:2]`, `[material:3]` per `(layer, ActiveRegion)`.
- `region_mapping_two_semantics_produces_cross_product_cardinality` — object with `material` × `fuzzy_skin` produces `∏(1 + K_i)` chains per `(layer, ActiveRegion)`; exact key set is enumerated and asserted.
- `region_mapping_chains_ordered_by_aggregated_region_split_canonical_order` — given two semantics whose BTreeMap order differs from their declaration order, each chain's `(semantic, value)` pairs appear in BTreeMap iteration order.
- `region_mapping_two_objects_with_disjoint_paint_emit_per_object_chains` — object A carries only `material`; object B carries only `fuzzy_skin`. Object A's `RegionPlan` entries have no `fuzzy_skin` element in any chain; symmetrically for object B.
- `region_mapping_chain_derived_overlay_matches_layer_wide_overlay_when_aggregation_empty` — paired positive assertion for AC-7b: empty-chain config equals what the deleted layer-wide path produced pre-packet.

Cube_4color tests (`cube_4color_paint_tdd.rs`) remain P95's acceptance concern and are not gated by this packet.

| `cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log`

### AC-10 — Behavior preservation when `aggregated_region_split` is empty

**Given** that no production module declares `[[region_split]]` in this packet (P95's job),
**When** the default production `pnp_cli slice` runs on `resources/regression_wedge.stl`,
**Then** the produced g-code is byte-identical to the post-P92 baseline captured in Step 0 (recorded as `P92_BASELINE_SHA=<hex>` in `.ralph/specs/93_region-mapping-cross-product/closure-log.md`); `RegionMapIR.entries` cardinality is unchanged vs pre-packet (the cross-product collapses to the empty chain only). This is the integration-level verification of AC-7b's overlay equivalence. The comparison shell command exits 0 only on match.

| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p93-wedge-post.gcode && test "$(sha256sum target/p93-wedge-post.gcode | awk '{print $1}')" = "$(grep -oE 'P92_BASELINE_SHA=[a-f0-9]+' .ralph/specs/93_region-mapping-cross-product/closure-log.md | head -1 | cut -d= -f2)"`

### AC-11 — Guest WASM rebuild clean

**Given** the IR-aware kernel extension,
**When** `cargo xtask build-guests && cargo xtask build-guests --check` runs,
**Then** `--check` reports zero `STALE:` entries.

| `cargo xtask build-guests && cargo xtask build-guests --check`

## Negative Test Cases

### AC-N1 — Empty `aggregated_region_split` produces only base (empty) variant_chains

**Given** the production default (no opted-in semantics),
**When** the kernel runs,
**Then** every `RegionKey.variant_chain` in `RegionMapIR.entries` is empty. The `RegionMapIR.entries` keyset equals what the pre-packet kernel produced (modulo any benign hash-order differences).

| `cargo test -p slicer-core region_mapping_empty_aggregation_no_variants 2>&1 | tee target/test-output.log`

### AC-N2 — Cap overflow halts with a structured error naming the worst object

**Given** a synthetic test scenario where the cross-product would exceed `DEFAULT_REGION_MAP_CAP`,
**When** the kernel runs,
**Then** the kernel returns a structured error `RegionMappingError::CapExceeded { actual, cap, top_contributor_object_id }`; no `RegionMapIR` is materialized; the host surfaces the error as fatal per the existing `RegionMappingError` handling.

| `cargo test -p slicer-runtime region_map_cap_exceeded_named_contributor 2>&1 | tee target/test-output.log`

### AC-N3 — A variant chain in an entry NEVER contains a `Scalar` `PaintValue`

**Given** D13 (Scalar forbidden in variant_chain),
**When** any `RegionPlan` in `RegionMapIR.entries` is inspected,
**Then** no element of `variant_chain` carries a `PaintValue::Scalar(_)`. (P1b's manifest validator rejects `value_type = "scalar"`, so this is doubly enforced — at manifest-load time and as a defensive runtime check in the scan step.)

| `cargo test -p slicer-core region_mapping_no_scalar_in_variant_chain 2>&1 | tee target/test-output.log`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log` (the kernel-level tests, including the six AC-9 net-new tests and AC-7b's overlay-equivalence test)
4. `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-wedge.gcode && sha256sum /tmp/p93-wedge.gcode` (AC-10 byte-identical g-code)
5. `cargo xtask build-guests && cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c — RegionMapping cross-product expansion" (~140 lines; read directly).
- `docs/02_ir_schemas.md` — sections describing `RegionMapIR`, `RegionPlan`, `RegionKey` (post-P91 shape; range-read).
- `docs/04_host_scheduler.md` §"RegionMapping" stage if it exists (range-read).
- `crates/slicer-core/src/algos/region_mapping.rs` — primary edit site (535 lines; range-read by symbol: `execute_region_mapping_inner` line 384, `overlay_resolved` line 110, `stamp_modifier_config_deltas` line 217, `overlapping_semantics_for_region` line 286).

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `crates/slicer-core/src/algos/region_mapping.rs` doc-comments for the cross-product expansion function and the new `enumerate_canonical_chains` helper — `rg -q 'enumerate_canonical_chains' crates/slicer-core/src/algos/region_mapping.rs`.
- `crates/slicer-ir/src/region_split_registry.rs` (NEW) with the `enumerate_canonical_chains` helper extracted from the kernel — `test -f crates/slicer-ir/src/region_split_registry.rs`.

`docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` updates deferred to packet 99 (P5c).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156` — cross-product expansion; SUMMARY confirming the algorithm shape (one entry per `(volume_region × extruder_index)` combination present on the object).
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — `PaintedRegion`/`FuzzySkinPaintedRegion` structures; SUMMARY confirming parent-region pointer + paint-value pair.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
