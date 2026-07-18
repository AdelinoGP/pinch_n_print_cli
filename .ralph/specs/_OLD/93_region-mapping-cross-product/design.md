# Design: 93_region-mapping-cross-product

## Controlling Code Paths

- Primary entry: `execute_region_mapping_inner` at `crates/slicer-core/src/algos/region_mapping.rs:384` (the kernel's existing public entry; this packet extends it).
- Companion helpers in the same file that the cross-product loop composes with:
  - `overlay_resolved` at line 110 — applies a paint-semantic `ResolvedConfig` on top of a base `ResolvedConfig`. The chain fold calls this once per `(semantic, value)` pair in each chain.
  - `stamp_modifier_config_deltas` at line 217 — stamps `config_delta.fields` from `ModifierVolume` entries. The chain fold's base is `stamp_modifier_config_deltas`'s output; modifier stamping happens once per `(layer, ActiveRegion)`, before the chain fold inner loop.
- DELETED in this packet:
  - `overlapping_semantics_for_region` (line 286) — fully subsumed by the chain-derived overlay path. The empty-chain case (`chain = []`) reproduces this function's output exactly when `aggregated_region_split.is_empty()`.
  - The call site at line 494 (the `let semantics = overlapping_semantics_for_region(...)` line and its subsequent `if semantics.is_empty()` branch) — replaced by the chain fold.
- Producer wrapper: `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`.
- New file: `crates/slicer-ir/src/region_split_registry.rs` for the `enumerate_canonical_chains` helper.
- Neighboring tests / fixtures: `crates/slicer-core/tests/algo_region_mapping_tdd.rs` (extend with the six AC-9 net-new tests + the AC-7b overlay-equivalence test + the cross-product / scan / enumerator tests). The cube_4color test file is out of scope.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Determinism invariant: `enumerate_canonical_chains` MUST produce the same output sequence for the same input. Use BTreeMap for the canonical-order axis; use a stable `PaintValue` comparator for tied semantics. Without determinism, `RegionMapIR.entries` becomes order-dependent and tests flake.
- Cross-product cardinality invariant: for an object with N opted-in semantics each carrying K_i distinct values, the per-(layer, ActiveRegion) cross-product cardinality is `∏(1 + K_i)` (the +1 accounts for "this semantic not present in this chain"). For a 16-color object (1 semantic, 16 values), that's 17 chains per region per layer. For 16-color × fuzzy_skin flag, that's 17 × 2 = 34 chains.
- Empty-aggregation invariant: when `aggregated_region_split.is_empty()`, the cross-product collapses to one element (the empty chain) per region. `RegionMapIR.entries` cardinality matches the pre-packet pre-cross-product behavior. The chain-derived overlay path's empty-chain case reproduces the deleted `overlapping_semantics_for_region` output (AC-7b, AC-10).
- D15: emit empty-polygon `RegionPlan` entries unconditionally. The kernel does NOT gate by geometric coverage; **empty-polygon filtering is P95's responsibility** — polygons live on `SlicedRegion`, populated by P95 via `replace_slice_ir`, and P95 has the polygons in hand to decide emptiness.

## Code Change Surface

- Selected approach: extract `enumerate_canonical_chains` into a small `slicer-ir` helper (testable in isolation), implement the per-object paint scan as a private kernel helper, extend the body of `execute_region_mapping_inner` to drive the cross-product loop and the chain-derived overlay fold, delete `overlapping_semantics_for_region` along with its line-494 call site, preserve `stamp_modifier_config_deltas` (called before the chain fold as the base), update the producer wrapper, raise the cap, and add tests.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-ir/src/region_split_registry.rs`** (NEW, ≤ 100 LOC):
    - `pub fn enumerate_canonical_chains(variants: &HashMap<String, Vec<PaintValue>>, canonical_order: &[String]) -> Vec<Vec<(String, PaintValue)>>`.
    - Returns every subset of `(semantic, value)` pairs across the input map; subset elements are ordered by the canonical order; the empty subset is the first element.
    - Implementation: recursive enumerator over `canonical_order` with branches `(include nothing for this semantic, include each value of this semantic)`.
    - Unit tests inside `slicer-ir` for the 2×1 case (6 chains), 0-semantic case (1 chain), 1×4 case (5 chains).
  - **`crates/slicer-core/src/algos/region_mapping.rs`** (extend + delete):
    - New private function `fn scan_paint_data(objects: &[Object], aggregated: &BTreeMap<String, AggregatedRegionSplitEntry>) -> HashMap<ObjectId, HashMap<String, Vec<PaintValue>>>`. Iterates each object's `paint_data.layers[*].facet_values`; for each semantic in `aggregated`, collects distinct PaintValues. Single pass. Deterministic ordering via BTreeSet / sort-after-collection.
    - Defensive runtime guard: reject any `PaintValue::Scalar(_)` encountered during scan with `RegionMappingError::ScalarInRegionSplitFacetValue { object_id, semantic, scalar }`. (P92's manifest validator already rejects `value_type = "scalar"`, but a hostile mesh could carry Scalar paint where a Flag was declared; defensive guard.)
    - Kernel main loop (inside `execute_region_mapping_inner` at line 384):
      ```rust
      let painting_variants_per_object = scan_paint_data(&mesh.objects, &aggregated_region_split);
      let canonical_order: Vec<String> = aggregated_region_split.keys().cloned().collect();
      let mut entries = HashMap::new();
      let mut configs = Vec::new();  // interner Vec
      for layer in layer_plan.global_layers.iter() {
          for active_region in layer.active_regions.iter() {
              let variants = painting_variants_per_object.get(&active_region.object_id).cloned().unwrap_or_default();
              let chains = enumerate_canonical_chains(&variants, &canonical_order);
              // Modifier stamping is the base on which paint-semantic overlays fold.
              let modifier_stamped_base = stamp_modifier_config_deltas(&active_region.resolved_config, &active_region.modifier_volumes);
              for chain in chains {
                  let key = RegionKey { global_layer_index: layer.index, object_id: active_region.object_id.clone(), region_id: active_region.region_id, variant_chain: chain.clone() };
                  // Chain-derived overlay: fold matching paint-semantic ResolvedConfigs onto the modifier-stamped base.
                  // When chain is empty (the only chain when aggregated_region_split.is_empty()), this fold is a no-op
                  // and the result equals what the deleted overlapping_semantics_for_region path produced pre-packet.
                  let plan_config = chain.iter().fold(modifier_stamped_base.clone(), |acc, (sem, val)| overlay_resolved(&acc, paint_semantic_configs.get(sem, val)));
                  let config_id = intern_config_in_vec(&mut configs, plan_config);
                  if entries.len() >= DEFAULT_REGION_MAP_CAP {
                      return Err(RegionMappingError::CapExceeded { actual: entries.len(), cap: DEFAULT_REGION_MAP_CAP, top_contributor_object_id: identify_top_contributor(&entries) });
                  }
                  entries.insert(key, RegionPlan { config: config_id, /* stage_modules derived as today */ });
              }
          }
      }
      RegionMapIR { entries, configs }
      ```
    - **DELETE `fn overlapping_semantics_for_region` (line 286) and its call site at line 494** (the `let semantics = overlapping_semantics_for_region(...)` line and its subsequent `if semantics.is_empty()` branch). The chain-derived overlay fold with `chain = []` reproduces the deleted code's behavior when `aggregated_region_split.is_empty()` — AC-7b's unit test asserts this equivalence; AC-10's byte-identical g-code check verifies it integration-wide.
    - **PRESERVE `stamp_modifier_config_deltas` (line 217)**: called once per `(layer, ActiveRegion)` before the chain fold inner loop. No change to its body.
    - `fn identify_top_contributor(entries: &HashMap<RegionKey, RegionPlan>) -> ObjectId`: groups by `object_id`, returns the one with the highest entry count.
  - **`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`** (small edit):
    - Wrapper signature now reads `aggregated_region_split` from the scheduler-provided context and threads it into the kernel call.
    - If the wrapper currently does not have access to the scheduler context, extend its constructor / commit signature minimally to take a `&BTreeMap<String, AggregatedRegionSplitEntry>` reference.
  - **`crates/slicer-ir/src/slice_ir.rs`** (constant raise):
    - `DEFAULT_REGION_MAP_CAP: usize` from current value `1_000` (at line 1196) to `750_000`. 750× jump from baseline.
    - Doc-comment updated to note the rationale (16-color × 1000-layer × 16-region headroom without modifier stamping) and the 750× factor.
  - **`crates/slicer-core/tests/algo_region_mapping_tdd.rs`** (extend):
    - Six AC-9 net-new tests (enumerated in `packet.spec.md` AC-9).
    - AC-7b overlay-equivalence test: `region_mapping_chain_derived_overlay_matches_layer_wide_overlay_when_aggregation_empty`.
    - Unit tests for `scan_paint_data` (4-distinct-tool-index case), the cross-product cardinality on a 1-semantic-4-value scenario, the empty-aggregation invariant (AC-N1), the scalar-paint defensive guard (AC-N3).
  - **`crates/slicer-runtime/tests/integration/region_map_cap_overflow_tdd.rs`** (NEW):
    - Synthetic scenario producing > 750k entries; assert `CapExceeded` is returned with the worst-contributor `ObjectId`.
- Rejected alternatives that were considered and why they were not chosen:
  - **Retain `overlapping_semantics_for_region` as a fallback for the empty-aggregation case**: rejected. The chain-derived path with `chain = []` produces the same output by design (AC-7b asserts this; AC-10 verifies it). Keeping the old function as a fallback duplicates code paths and creates a second source of truth for overlay derivation.
  - **Pre-compute `painting_variants_per_object` outside the kernel** (e.g., in the producer wrapper): rejected because it forces the producer wrapper to know about paint-data semantics, violating layer separation. The kernel is the right place to scan paint data.
  - **Filter empty-polygon `RegionPlan` entries in the kernel**: rejected. Polygons live on `SlicedRegion`, populated by P95 via `replace_slice_ir`; the kernel cannot predict polygon emptiness from `ActiveRegion` alone. P95 owns the empty-polygon gate.
  - **Allocate `entries` with capacity = `DEFAULT_REGION_MAP_CAP`**: would over-allocate for empty-aggregation production runs. Use `HashMap::new()` and grow on demand.
  - **Use a HashMap<RegionKey, ConfigId> for the interner**: faster O(1) intern, but requires `RegionKey: Hash` (already true) AND a separate forward Vec<ResolvedConfig> for `config_for`. The Vec-scan approach is O(N) per intern; profile-driven upgrade is deferred.
  - **Move `DEFAULT_REGION_MAP_CAP` into a config**: rejected — the cap is a guardrail, not a tunable. Pathological scenes should be surfaced with the diagnostic, not silently expanded.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/region_split_registry.rs` (NEW) — role: `enumerate_canonical_chains` helper; expected change: new file (≤ 100 LOC).
- `crates/slicer-ir/src/lib.rs` — role: module declaration; expected change: one line `pub mod region_split_registry;`.
- `crates/slicer-core/src/algos/region_mapping.rs` — role: kernel; expected change: extend `execute_region_mapping_inner` with the cross-product loop; delete `overlapping_semantics_for_region` and its line-494 call site; add `scan_paint_data` + defensive Scalar guard; preserve `stamp_modifier_config_deltas` and `overlay_resolved`.
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — role: producer wrapper; expected change: thread `aggregated_region_split`.
- `crates/slicer-ir/src/slice_ir.rs` — role: `DEFAULT_REGION_MAP_CAP` constant at line 1196; expected change: raise from `1_000` to `750_000`.
- `crates/slicer-core/tests/algo_region_mapping_tdd.rs` — role: kernel unit tests; expected change: extend with the six AC-9 net-new tests, the AC-7b overlay-equivalence test, plus scan/enumerator/cap-derivation tests.
- `crates/slicer-runtime/tests/integration/region_map_cap_overflow_tdd.rs` (NEW) — role: cap diagnostic; expected change: new file (≤ 80 LOC).

Multiple files but each delta is small. Per-step plan keeps each step to ≤ 3 files.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c" — read directly.
- `docs/02_ir_schemas.md` — RegionMapIR, RegionPlan, RegionKey sections only.
- `docs/04_host_scheduler.md` — RegionMapping stage if present (range-read).
- `crates/slicer-core/src/algos/region_mapping.rs` — range-read by symbol (535 lines).
- `crates/slicer-scheduler/src/region_split.rs` (created in P92) — read for the `aggregate_region_splits` signature.
- An existing core-module manifest as TOML template (for synthetic-manifest authoring in tests).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/src/dispatch.rs`, `wasm_host.rs` — not in scope.
- `modules/core-modules/*/src/**` — no module code change.
- Binary 3MF files — never `Read`.
- `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` — paint-segmentation territory (P95).
- `crates/slicer-runtime/src/layer_executor.rs` — touched in P92; not edited here. If a kernel-level lookup needs metadata that lives there, ESCALATE (likely a design oversight).

## Expected Sub-Agent Dispatches

- "Re-verify LOCATIONS in `crates/slicer-core/src/algos/region_mapping.rs` for: `execute_region_mapping_inner` (line 384), `overlay_resolved` (line 110), `stamp_modifier_config_deltas` (line 217), `overlapping_semantics_for_region` (line 286). Cap at 8 entries" — purpose: pinpoint edit + delete targets.
- "Locate `DEFAULT_REGION_MAP_CAP` in `crates/slicer-ir/src/`; return FILE:LINE" — purpose: cap-raise edit site (expected `slice_ir.rs:1196`).
- "Open `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` (full read OK; ≤ 60 LOC) and return SNIPPETS (≤ 30 lines) showing the wrapper's commit signature and access to scheduler context" — purpose: producer-wrapper edit scope.
- "Run `cargo test -p slicer-ir region_split_registry 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: validate the helper.
- "Run `cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown" — purpose: kernel-test gate (six AC-9 tests + AC-7b + AC-N1 + AC-N3).
- "Run `! rg -q 'overlapping_semantics_for_region' crates/slicer-core/src/algos/region_mapping.rs`; return FACT pass/fail" — purpose: AC-7 deletion verification.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-wedge.gcode && sha256sum /tmp/p93-wedge.gcode`; return FACT (sha256)" — purpose: AC-10 byte-identical g-code.
- "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-11.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156`; return SUMMARY ≤ 150 words" — purpose: parity confirmation.

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

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 3 — kernel extension + deletion).
- Highest-risk dispatch: AC-7b's overlay-equivalence test return. The dispatch must return both the AC-7b test's pass/fail AND the AC-10 sha256 so equivalence is verified at unit and integration levels in the same step.

## Open Questions

- `[FWD]` — Where does `aggregated_region_split` enter the producer wrapper (constructor, commit context, scheduler-provided slot)? Step 1 dispatch confirms.
- `[FWD]` — Does the kernel's `paint_semantic_configs` source (the per-semantic `ResolvedConfig` map the chain fold consults) already exist in the kernel scope post-P91, or does Step 3 need to wire it from `aggregated_region_split`? Step 3 dispatch confirms.
- `[BLOCK]` — None.
