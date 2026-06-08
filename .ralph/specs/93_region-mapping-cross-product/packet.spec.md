---
status: draft
packet: 93
task_ids: [TASK-243]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 93 — RegionMapping Cross-Product Expansion (variant_chain populated; polygons empty)

## Goal

Rewrite the RegionMapping kernel in `crates/slicer-core/src/algos/region_mapping.rs` to expand each `ActiveRegion` into N variant entries — one per element of the canonical cross-product of paint values present on that object across all opted-in region-split semantics (D2 + D5 + D15). The expansion uses `aggregated_region_split_semantics` (P1b's BTreeMap) as the registry of opted-in semantics, scans each object's `paint_data.layers[*].facet_values` once to discover the distinct `PaintValue`s present per semantic per object, then for each `(layer, ActiveRegion)` emits one `RegionPlan` keyed by `(global_layer_index, object_id, region_id, variant_chain)` for every element of `enumerate_canonical_chains(variants, &canonical_order)` — including the empty (base) chain. Each `RegionPlan.config` is set via the P1a interner (`region_map.intern_config(...)`) so a 16-color object's 16 variants don't replicate full ResolvedConfigs. Per-variant polygons stay empty in this packet — they are populated by paint-segmentation in P3 via `replace_slice_ir`. The `DEFAULT_REGION_MAP_CAP` constant is raised to 750_000 (with the existing top-contributor-diagnostic message updated to surface the worst object on overflow). The previously-broken `overlapping_semantics_for_region` at `region_mapping.rs:286-319` (the hardcoded `return true` that stamped every paint semantic onto every region regardless of object) is replaced with the per-object cross-product expansion — a region's variant entries are now bounded by what that specific object's paint actually carries. Five GREEN cube_4color tests assert the new shape; seven RED cube_4color tests stay RED (they assert on variant polygons, which P3 fills).

## Scope Boundaries

This packet rewrites the RegionMapping kernel and the producer wrapper to thread `aggregated_region_split` from the scheduler into the kernel. It does NOT change paint-segmentation, mesh-segmentation, or any module's manifest. With no core module declaring `[[region_split]]` yet (still P3's job), `aggregated_region_split` is empty by default — and the cross-product collapses to the empty chain for every region, leaving every `RegionMapIR.entries` shape identical to pre-packet. Cube test fixtures (P0a authored) carry painted facets, but they only opt-in to region-splitting once a paint-aware module declares the relevant semantics in its manifest. To exercise the cross-product in this packet's tests, synthetic manifests (from P1b's fixture directory) declaring `material` are loaded for the cube tests; production behavior is unchanged. Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 91 (P1a — schema scaffolding) and packet 92 (P1b — manifest + dispatch) must be `implemented`. P1c reads `aggregated_region_split` (P1b) and writes via the P1a `intern_config`/`config_for` accessors.
- Unblocks: P2 (94, mesh-segmentation wiring) is independent of this packet but recommended to land after P1c so the prepass driver shape stabilises. P3 (95, paint-segmentation port) depends on P1c (fills variant polygons via `replace_slice_ir` into the entries P1c emits).
- Activation blockers: P91 and P92 both `implemented`.

## Acceptance Criteria

### AC-1 — `RegionMapping` kernel reads `aggregated_region_split` from the scheduler and threads it through

**Given** the kernel rewrite,
**When** `crates/slicer-core/src/algos/region_mapping.rs` is inspected,
**Then** the kernel's public entry point accepts (or reads from a context object) the `aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>` produced by `slicer-scheduler::region_split::aggregate_region_splits`. The kernel does NOT hardcode a list of opted-in semantics; the registry is the authoritative source.

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

### AC-7 — `overlapping_semantics_for_region`'s `return true` bug is removed

**Given** the broken implementation at `region_mapping.rs:286-319`,
**When** the file is grepped,
**Then** the function either no longer exists OR has been replaced with a per-object lookup that returns the actual cross-product variants for the asking region's object_id. No `return true` remains as a paint-overlap shortcut.

| `! rg -nE 'fn overlapping_semantics_for_region[^}]*\n[^}]*return true' crates/slicer-core/src/algos/region_mapping.rs`

### AC-8 — `DEFAULT_REGION_MAP_CAP` raised to 750_000 with overflow diagnostic

**Given** the increased cardinality of `RegionMapIR.entries` under cross-product,
**When** `crates/slicer-ir/src/slice_ir.rs` (or wherever the constant lives) is inspected,
**Then** `DEFAULT_REGION_MAP_CAP` is `750_000`; the overflow diagnostic message (already a top-contributor surface) names the worst-contributing object_id in the structured-event output on overflow.

| `rg -q 'DEFAULT_REGION_MAP_CAP\s*[:=]\s*750_000' crates/slicer-ir/src/ && cargo test -p slicer-runtime region_map_cap_overflow_diagnostic 2>&1 | tee target/test-output.log`

### AC-9 — 5 GREEN cube_4color RED-tests turn GREEN: variant_chain assertions

**Given** the 5 cube_4color tests that assert on `RegionMapIR.entries` containing specific variant_chain shapes (from cherry-pick 5c272ef's RED suite),
**When** the migrated kernel runs against `cube_4color.3mf` with a synthetic manifest declaring `[[region_split]] semantic = "material"`,
**Then** all 5 tests pass. Specifically (per the cube_4color authoring convention face +X = ToolIndex 1, face -X = ToolIndex 2, face +Y = ToolIndex 3, face -Y = ToolIndex 4):
- A test asserts that `RegionMapIR.entries` contains a key with `variant_chain = [("material", ToolIndex(1))]`.
- Same for ToolIndex 2, 3, 4.
- A test asserts the base region (empty variant_chain) entry also exists.

| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log | grep -E '^test result' | head -1`

### AC-10 — 7 RED cube_4color tests stay RED: variant-polygon assertions

**Given** the 7 cube_4color tests that assert on per-variant polygon coverage (P3 territory),
**When** the test bucket runs after this packet,
**Then** these 7 tests are still RED (FAILED). Their failure assertions update to reference variant polygons (P3 will populate); the test names and intent are preserved. Acceptable failure: `cargo test ... 2>&1 | grep "test result"` reports a non-zero failed count corresponding to these 7 tests.

Manual check — the test bucket's pass-count is documented in the closure log: 5 passing + 7 failing (or whatever distribution P0a's cube authoring produces).

| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log | grep -qE 'test result: (FAILED|ok)\.'`

### AC-11 — Behavior preservation when `aggregated_region_split` is empty

**Given** that no production module declares `[[region_split]]` in this packet (P3's job),
**When** the default production `pnp_cli slice` runs on `resources/regression_wedge.stl`,
**Then** the produced g-code is byte-identical to the post-P92 baseline; `RegionMapIR.entries` cardinality is unchanged vs pre-packet (the cross-product collapses to the empty chain only).

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-wedge.gcode && sha256sum /tmp/p93-wedge.gcode`

### AC-12 — Guest WASM rebuild clean

**Given** the IR-aware kernel rewrite,
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
3. `cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log` (the kernel-level tests)
4. `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log` (5 GREEN + 7 RED expected)
5. `cargo xtask build-guests && cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c — RegionMapping cross-product expansion" (~140 lines; read directly).
- `docs/02_ir_schemas.md` — sections describing `RegionMapIR`, `RegionPlan`, `RegionKey` (post-P1a shape; range-read).
- `docs/04_host_scheduler.md` §"RegionMapping" stage if it exists (range-read).
- `crates/slicer-core/src/algos/region_mapping.rs` — primary edit site; read in full as needed (likely > 300 lines, range-read by symbol).

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
