# Design: 93_region-mapping-cross-product

## Controlling Code Paths

- Primary code paths: `crates/slicer-core/src/algos/region_mapping.rs` (kernel rewrite), `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` (wrapper update to thread `aggregated_region_split`), `crates/slicer-ir/src/region_split_registry.rs` (NEW — extracts `enumerate_canonical_chains` helper).
- Neighboring tests or fixtures: `crates/slicer-core/tests/algo_region_mapping_tdd.rs` (extend with new unit tests), `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` (5 RED tests turn GREEN after assertion update — but the *test bodies* are touched only if the assertions explicitly need rewording; if they already target the right shape, no edit).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Determinism invariant: `enumerate_canonical_chains` MUST produce the same output sequence for the same input. Use BTreeMap for the canonical-order axis; use a stable `PaintValue` comparator for tied semantics. Without determinism, `RegionMapIR.entries` becomes order-dependent and tests flake.
- Cross-product cardinality invariant: for an object with N opted-in semantics each carrying K_i distinct values, the per-(layer, ActiveRegion) cross-product cardinality is `∏(1 + K_i)` (the +1 accounts for "this semantic not present in this chain"). For a 16-color object (1 semantic, 16 values), that's 17 chains per region per layer. For 16-color × fuzzy_skin flag, that's 17 × 2 = 34 chains. The new `DEFAULT_REGION_MAP_CAP = 750_000` admits ~16-color × 1000-layer × 16-region × 3-modifier scenes (≈ 1.6M entries before modifier stamping; the cap is conservative; the diagnostic surfaces top-contributor).
- Empty-aggregation invariant: when `aggregated_region_split.is_empty()`, the cross-product collapses to one element (the empty chain) per region. `RegionMapIR.entries` cardinality matches the pre-packet pre-cross-product behavior. Tests assert on this invariant (AC-N1).
- D15: emit empty-polygon `RegionPlan` entries unconditionally. The kernel does NOT gate by geometric coverage; that's P3's job (paint-segmentation produces the per-variant polygons, possibly empty, and writes via `replace_slice_ir`).

## Code Change Surface

- Selected approach: extract `enumerate_canonical_chains` into a small `slicer-ir` helper (testable in isolation), implement the per-object paint scan as a private kernel helper, rewrite the kernel's main loop to use both, replace the broken `overlapping_semantics_for_region`, update the producer wrapper, raise the cap, and update tests.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **`crates/slicer-ir/src/region_split_registry.rs`** (NEW, ≤ 100 LOC):
    - `pub fn enumerate_canonical_chains(variants: &HashMap<String, Vec<PaintValue>>, canonical_order: &[String]) -> Vec<Vec<(String, PaintValue)>>`.
    - Returns every subset of `(semantic, value)` pairs across the input map; subset elements are ordered by the canonical order; the empty subset is the first element.
    - Implementation: recursive enumerator over `canonical_order` with branches `(include nothing for this semantic, include each value of this semantic)`.
    - Unit tests inside `slicer-ir` for the 2×1 case (6 chains), 0-semantic case (1 chain), 1×4 case (5 chains).
  - **`crates/slicer-core/src/algos/region_mapping.rs`** (rewrite):
    - New private function `fn scan_paint_data(objects: &[Object], aggregated: &BTreeMap<String, AggregatedRegionSplitEntry>) -> HashMap<ObjectId, HashMap<String, Vec<PaintValue>>>`. Iterates each object's `paint_data.layers[*].facet_values`; for each semantic in `aggregated`, collects distinct PaintValues. Single pass. Deterministic ordering via BTreeSet / sort-after-collection.
    - Defensive runtime guard: reject any `PaintValue::Scalar(_)` encountered during scan with `RegionMappingError::ScalarInRegionSplitFacetValue { object_id, semantic, scalar }`. (P1b's manifest validator already rejects `value_type = "scalar"`, but a hostile mesh could carry Scalar paint where a Flag was declared; defensive guard.)
    - Kernel main loop (replaces the existing `commit_region_mapping_builtin` body):
      ```rust
      let painting_variants_per_object = scan_paint_data(&mesh.objects, &aggregated_region_split);
      let canonical_order: Vec<String> = aggregated_region_split.keys().cloned().collect();
      let mut entries = HashMap::new();
      let mut configs = Vec::new();  // interner Vec
      for layer in layer_plan.global_layers.iter() {
          for active_region in layer.active_regions.iter() {
              let variants = painting_variants_per_object.get(&active_region.object_id).cloned().unwrap_or_default();
              let chains = enumerate_canonical_chains(&variants, &canonical_order);
              for chain in chains {
                  let key = RegionKey { global_layer_index: layer.index, object_id: active_region.object_id.clone(), region_id: active_region.region_id, variant_chain: chain.clone() };
                  let plan_config = derive_resolved_config(&active_region.resolved_config, &chain, &paint_semantic_configs);
                  let config_id = intern_config_in_vec(&mut configs, plan_config);
                  let stage_modules = derive_stage_modules(&active_region, &chain, &stage_invocations);
                  if entries.len() >= DEFAULT_REGION_MAP_CAP {
                      return Err(RegionMappingError::CapExceeded { actual: entries.len(), cap: DEFAULT_REGION_MAP_CAP, top_contributor_object_id: identify_top_contributor(&entries) });
                  }
                  entries.insert(key, RegionPlan { config: config_id, stage_modules });
              }
          }
      }
      RegionMapIR { entries, configs }
      ```
    - Delete `fn overlapping_semantics_for_region` (the `return true` bug). If any other caller exists (unlikely; the function was named for one consumer), update it to use the per-object lookup `painting_variants_per_object[object_id]`.
    - `fn derive_resolved_config(base: &ResolvedConfig, chain: &[(String, PaintValue)], paint_semantic_configs: &PaintSemanticConfigs) -> ResolvedConfig`: applies chain's overlays to the base. For now, this is a stub that returns `base.clone()` unchanged because no opted-in semantic has overlay config keys yet (that's P3 / P4 territory once `material` declares its tool-specific feedrate overrides). Documented in the function's doc-comment as "P1c stub; P3+ extends overlay logic per-semantic".
    - `fn identify_top_contributor(entries: &HashMap<RegionKey, RegionPlan>) -> ObjectId`: groups by `object_id`, returns the one with the highest entry count.
  - **`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`** (small edit):
    - Wrapper signature now reads `aggregated_region_split` from the scheduler-provided context and threads it into the kernel call.
    - If the wrapper currently does not have access to the scheduler context, extend its constructor / commit signature minimally to take a `&BTreeMap<String, AggregatedRegionSplitEntry>` reference.
  - **`crates/slicer-ir/src/slice_ir.rs`** (constant raise):
    - `DEFAULT_REGION_MAP_CAP: usize` from current value (presumably 250_000 or similar) to `750_000`.
    - Doc-comment updated to note the rationale (16-color × 1000-layer × 16-region × 3-modifier headroom).
  - **`crates/slicer-core/tests/algo_region_mapping_tdd.rs`** (extend):
    - New unit tests for `scan_paint_data`, `enumerate_canonical_chains`, the cross-product cardinality on a 1-semantic-4-value scenario, the empty-aggregation invariant, the scalar-paint defensive guard.
  - **`crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs`** (assertion review):
    - For each of the 12 tests, confirm whether the assertion targets `RegionMapIR.entries` shape (GREEN after this packet) or per-variant polygons (still RED). 5 GREEN, 7 RED expected per the roadmap. If the cherry-pick's authors used a different assertion style than the kernel produces, adjust the assertion (with closure-log justification per AC-N1 of the migration packets).
  - **`crates/slicer-runtime/tests/integration/region_map_cap_overflow_tdd.rs`** (NEW):
    - Synthetic scenario producing > 750k entries; assert `CapExceeded` is returned with the worst-contributor `ObjectId`.
- Rejected alternatives that were considered and why they were not chosen:
  - **Pre-compute `painting_variants_per_object` outside the kernel** (e.g., in the producer wrapper): rejected because it forces the producer wrapper to know about paint-data semantics, violating layer separation. The kernel is the right place to scan paint data.
  - **Allocate `entries` with capacity = `DEFAULT_REGION_MAP_CAP`**: would over-allocate for empty-aggregation production runs. Use `HashMap::new()` and grow on demand.
  - **Use a HashMap<RegionKey, ConfigId> for the interner**: faster O(1) intern, but requires `RegionKey: Hash` (already true) AND a separate forward Vec<ResolvedConfig> for `config_for`. The Vec-scan approach is O(N) per intern; profile-driven upgrade is deferred.
  - **Move `DEFAULT_REGION_MAP_CAP` into a config**: rejected — the cap is a guardrail, not a tunable. Pathological scenes should be surfaced with the diagnostic, not silently expanded.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/region_split_registry.rs` (NEW) — role: `enumerate_canonical_chains` helper; expected change: new file (≤ 100 LOC).
- `crates/slicer-ir/src/lib.rs` — role: module declaration; expected change: one line `pub mod region_split_registry;`.
- `crates/slicer-core/src/algos/region_mapping.rs` — role: kernel; expected change: rewrite the cross-product expansion, delete the broken function, add the paint scan + defensive Scalar guard.
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — role: producer wrapper; expected change: thread `aggregated_region_split`.
- `crates/slicer-ir/src/slice_ir.rs` — role: `DEFAULT_REGION_MAP_CAP` constant; expected change: raise to 750_000.
- `crates/slicer-core/tests/algo_region_mapping_tdd.rs` — role: kernel unit tests; expected change: extend with cross-product + scan tests.
- `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` — role: cube acceptance; expected change: minimal — only if the assertion shape needs alignment with kernel output (closure log justifies any change).
- `crates/slicer-runtime/tests/integration/region_map_cap_overflow_tdd.rs` (NEW) — role: cap diagnostic; expected change: new file (≤ 80 LOC).

Multiple files but each delta is small. Per-step plan keeps each step to ≤ 3 files.

## Read-Only Context

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P1c" — read directly.
- `docs/02_ir_schemas.md` — RegionMapIR, RegionPlan, RegionKey sections only.
- `docs/04_host_scheduler.md` — RegionMapping stage if present (range-read).
- `crates/slicer-core/src/algos/region_mapping.rs` — full read OK if ≤ 500 lines; otherwise range-read by symbol.
- `crates/slicer-scheduler/src/region_split.rs` (created in P1b) — read for the `aggregate_region_splits` signature.
- An existing core-module manifest as TOML template (for synthetic-manifest authoring in tests).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/src/dispatch.rs`, `wasm_host.rs` — not in scope.
- `modules/core-modules/*/src/**` — no module code change.
- Binary 3MF files — never `Read`.
- `crates/slicer-runtime/src/layer_executor.rs` — touched in P1b; not edited here. If a kernel-level lookup needs metadata that lives there, ESCALATE (likely a design oversight).

## Expected Sub-Agent Dispatches

- "Open `crates/slicer-core/src/algos/region_mapping.rs` and return LOCATIONS for: `commit_region_mapping_builtin`, `overlapping_semantics_for_region`, `derive_resolved_config` (if exists), `derive_stage_modules` (if exists). Cap at 10 entries" — purpose: pinpoint edit targets.
- "Locate `DEFAULT_REGION_MAP_CAP` in `crates/slicer-ir/src/`; return FILE:LINE" — purpose: cap-raise edit site.
- "Run `cargo test -p slicer-ir region_split_registry 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: validate the helper.
- "Run `cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown" — purpose: kernel-test gate.
- "Run `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; return FACT pass-count + FAILED-count breakdown" — purpose: AC-9 + AC-10.
- "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p93-wedge.gcode && sha256sum /tmp/p93-wedge.gcode`; return FACT (sha256)" — purpose: AC-11.
- "Run `cargo xtask build-guests && cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-12.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:1138-1156`; return SUMMARY ≤ 150 words" — purpose: parity confirmation.

## Data and Contract Notes

- IR or manifest contracts touched: `RegionMapIR.entries` cardinality changes when `aggregated_region_split` is non-empty (cross-product expansion). No new IR type. No WIT change.
- WIT boundary considerations: none. Guest WASM rebuilds because `slicer-ir` is a guest dep (the `DEFAULT_REGION_MAP_CAP` constant change propagates), but no contract reshapes.
- Determinism or scheduler constraints: cross-product enumeration order is the contract. Test fixtures lock specific orderings; flipping the order in a future refactor breaks tests.

## Locked Assumptions and Invariants

- **Empty aggregation → empty chains**: when `aggregated_region_split.is_empty()`, the cross-product produces exactly one element (the empty chain) per region. Production behavior is preserved (AC-11).
- **Cross-product cardinality bound**: `∏(1 + K_i)` over opted-in semantics, where K_i is distinct values present on the object. `DEFAULT_REGION_MAP_CAP = 750_000` is the hard guardrail.
- **Defensive Scalar rejection at runtime**: the kernel rejects `PaintValue::Scalar` in the scan even if the manifest validator was bypassed. Double-defense against D13 violation.
- **Interner identity**: two `ResolvedConfig`s that are `==` produce the same `ConfigId`. The linear-scan interner is the agreed implementation (HashSet upgrade deferred).

## Risks and Tradeoffs

- **Risk: the kernel rewrite introduces a g-code diff against the post-P92 baseline** because a previously-implicit ordering in `RegionMapIR.entries` iteration changes. Mitigation: tests assert on KEY membership, not on iteration order; AC-11 confirms emitted g-code is unchanged. Any diff is investigated, not waved off.
- **Risk: 5/12 GREEN ratio doesn't match cherry-pick authors' expectations.** Mitigation: each test's classification is re-examined; if a "GREEN" test asserts on something this packet doesn't deliver, escalate before forcing a pass.
- **Risk: the cap diagnostic identifies the wrong top contributor** when entries are evenly distributed. Mitigation: ties broken by `ObjectId` lex order; documented in the diagnostic helper's doc-comment.
- **Tradeoff: O(N) intern vs O(1) HashSet intern.** Linear scan is simpler and correct; profile-driven upgrade deferred until measurements justify.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 3 — kernel rewrite).
- Highest-risk dispatch: the cube-tests pass/FAIL breakdown (Step 6). The dispatch must return BOTH counts (passed + failed) + the names of each failed test so the closure log can record the GREEN/RED split.

## Open Questions

- `[FWD]` — Does the existing `derive_resolved_config` function exist with a callable signature in `region_mapping.rs`? If yes, extend in place; if no, write the stub. Step 1 dispatch confirms.
- `[FWD]` — Where does `aggregated_region_split` enter the producer wrapper (constructor, commit context, scheduler-provided slot)? Step 4 dispatch confirms.
- `[FWD]` — The cherry-pick (5c272ef) authors classified 5/7 GREEN/RED; if the kernel produces a different distribution, document the divergence in the closure log.
- `[BLOCK]` — None.
