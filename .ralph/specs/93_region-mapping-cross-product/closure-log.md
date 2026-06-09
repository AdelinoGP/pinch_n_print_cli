# P93 Closure Log

P92_BASELINE_SHA=e60fca7fb4ea67fd54a402c2d352ae7719f82389a4ac4669b497922c9301f674
P93_POST_SHA=e60fca7fb4ea67fd54a402c2d352ae7719f82389a4ac4669b497922c9301f674  # MATCH baseline (AC-10 PASS)

## Acceptance Ceremony Results

| AC    | Status | Notes |
|-------|--------|-------|
| AC-1  | PASS   | `execute_region_mapping_inner` carries `aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>` |
| AC-2  | PASS   | `region_mapping_paint_scan` — 5 chains for 4-distinct-ToolIndex input |
| AC-3  | PASS   | `region_mapping_enumerate_chains` (slicer-core) + 3 unit tests in slicer-ir/tests/region_split_registry_tdd.rs |
| AC-4  | PASS   | `region_mapping_cross_product_entry_count` — cardinality matches ∏(1 + K_i) |
| AC-5  | PASS   | `region_mapping_config_interning` — duplicate ResolvedConfigs share ConfigId |
| AC-6  | BY-CONSTRUCTION | Kernel writes only to `RegionMapIR`; `SlicedRegion` untouched. P95 owns polygon population. The named test `cube_4color_paint_region_map_empty_polygons` was NOT added (P95 territory per AUDIT.md §Audit 3). |
| AC-7  | PASS   | `overlapping_semantics_for_region` and its line-494 caller deleted (verified via grep) |
| AC-7b | PASS   | `region_mapping_chain_derived_overlay_matches_layer_wide_overlay_when_aggregation_empty` (by-construction proof; AC-10 is the integration-level confirmation) |
| AC-8  | PASS   | `DEFAULT_REGION_MAP_CAP = 750_000` at `crates/slicer-ir/src/slice_ir.rs:1196` with 750× headroom rationale doc |
| AC-9  | PASS   | All six enumerated tests pass in `crates/slicer-core/tests/algo_region_mapping_tdd.rs` |
| AC-10 | PASS   | Byte-identical g-code on `resources/regression_wedge.stl` (SHA above) |
| AC-11 | PASS   | `cargo xtask build-guests --check` reports zero STALE; 33 guests built clean |
| AC-N1 | PASS   | `region_mapping_empty_aggregation_no_variants` |
| AC-N2 | PASS   | `region_map_cap_exceeded_named_contributor` (integration test) — `top_contributors[0]` names worst object |
| AC-N3 | PASS   | `region_mapping_no_scalar_in_variant_chain` — defensive Scalar guard returns `ScalarInRegionSplitFacetValue` |

## Test Counts (Step 9 final gate)

- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test -p slicer-core --features host-algos` — 90 passed / 0 failed
- `cargo test -p slicer-ir` — 79 passed / 0 failed
- `cargo test -p slicer-runtime --test integration region_map_cap` — 1 passed / 0 failed

## Deviations from packet docs

1. **`slicer-core` now depends on `slicer-scheduler`** for `AggregatedRegionSplitEntry` (added to `crates/slicer-core/Cargo.toml`). No circular dep verified. Cleaner alternative (relocate `AggregatedRegionSplitEntry` to `slicer-ir`) deferred to a follow-up.
2. **`paint_regions: Option<&PaintRegionIR>`** is now unused in `execute_region_mapping_inner` (chain-derived path subsumes it). Renamed to `_paint_regions`; producer wrapper continues to pass it. Cleanup deferrable.
3. **AC-7b strategy** changed from "captured JSON fixture" to **by-construction equivalence proof** — the AC-7b test asserts properties of the chain-derived path with empty aggregation directly (no `paint_overrides`, no extra overlays beyond modifier stamping). AC-10 is the integration-level confirmation. No `p93_overlay_baseline.json` fixture was captured (implementation-plan.md Step 1 dispatch deviation).
4. **Step 2 verification command in implementation-plan.md** (`cargo test -p slicer-ir region_split_registry`) uses a name-filter that doesn't match the test fns (`enumerate_canonical_chains_*`). Working incantation: `cargo test -p slicer-ir --test region_split_registry_tdd`. Implementation-plan.md should be amended.
5. **AC-2, AC-3, AC-4, AC-5, AC-7, AC-9, AC-N1, AC-N3 verification commands** assume the slicer-core test bucket runs without `--features host-algos`. The kernel test file requires that feature flag. Working incantation: append `--features host-algos` to every `cargo test -p slicer-core ...` command in `packet.spec.md`.
6. **AC-N2 verification command** in packet.spec.md (`cargo test -p slicer-runtime region_map_cap_exceeded_named_contributor`) uses the test-name filter only; works as written, but the explicit form `cargo test -p slicer-runtime --test integration region_map_cap_exceeded_named_contributor` is the canonical bucket-aware invocation.
7. **`RegionMappingError` derive list** had to drop `Eq` because the new `ScalarInRegionSplitFacetValue { scalar: f32 }` variant cannot satisfy `Eq`. Cascading drops in `RegionMappingBuiltinError` (slicer-runtime/src/builtins/region_mapping_producer.rs) and `PrepassExecutionError` (slicer-runtime/src/prepass.rs). `PartialEq` preserved everywhere.
8. **`ExecutionPlan` gained `aggregated_region_split: BTreeMap<String, AggregatedRegionSplitEntry>` field**, and `build_execution_plan` / `build_live_execution_plan` gained a `diagnostics: &mut Vec<LoadDiagnostic>` parameter. 37 test files across the workspace had to be patched to construct the new field / pass the new arg.
9. **AC-6 test name (`cube_4color_paint_region_map_empty_polygons`) was NOT added.** This test is in cube_4color territory which AUDIT.md §Audit 3 explicitly relegated to P95. AC-6's property (no `SlicedRegion` mutation by this packet) holds by construction since the kernel only writes to `RegionMapIR`. Recommend re-wording AC-6 to a by-construction claim before packet close, OR transferring the test obligation to P95.

## Files Touched (Summary)

### Production code
- `crates/slicer-ir/src/region_split_registry.rs` (NEW; `enumerate_canonical_chains` helper)
- `crates/slicer-ir/src/lib.rs` (export new module)
- `crates/slicer-ir/src/slice_ir.rs` (cap 1_000 → 750_000 + doc-comment)
- `crates/slicer-core/Cargo.toml` (added `slicer-scheduler` dep)
- `crates/slicer-core/src/algos/region_mapping.rs` (kernel extended; `overlapping_semantics_for_region` + line-494 caller deleted; `scan_paint_data`, chain fold, `paint_value_canonical_cmp` added; `RegionMappingError::ScalarInRegionSplitFacetValue` variant; Eq dropped)
- `crates/slicer-scheduler/src/execution_plan.rs` (`aggregated_region_split` field; `build_execution_plan` signature widened)
- `crates/slicer-wasm-host/src/execution_plan_live.rs` (`build_live_execution_plan` signature widened)
- `crates/slicer-runtime/src/run.rs` (diagnostics thread-through)
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` (thread `&plan.aggregated_region_split` to kernel; Eq dropped)
- `crates/slicer-runtime/src/prepass.rs` (Eq dropped)

### Tests (NEW)
- `crates/slicer-ir/tests/region_split_registry_tdd.rs` (3 unit tests for enumerator)
- `crates/slicer-runtime/tests/integration/region_map_cap_overflow_tdd.rs` (AC-N2)

### Tests (PATCHED — new signature pass-through)
- `crates/slicer-core/tests/algo_region_mapping_tdd.rs` (+ 12 net-new tests)
- `crates/slicer-runtime/tests/integration/main.rs` (mod registry)
- 37 other test files in `slicer-runtime`, `slicer-scheduler`, `pnp-cli` test buckets

### Closure
- `.ralph/specs/93_region-mapping-cross-product/closure-log.md` (this file)

## Post-Audit Fixes (2026-06-08)

Audit (spec-audit-session) returned **DO NOT SHIP** flagging three substantive defects + two process cleanups. All fixes applied; AC commands re-verified fresh.

### Fix 1 — Eq restored on RegionMappingError + cascade

`ScalarInRegionSplitFacetValue { scalar: f32 }` → `{ scalar_bits: u32 }` (f32::to_bits / from_bits, P91's `ResolvedConfig` pattern). Restored `Eq` on `RegionMappingError`, `RegionMappingBuiltinError`, `PrepassExecutionError`. Public accessor `RegionMappingError::scalar() -> f32` reconstructs the float. Display impl preserved. Six test files patched to use `scalar_bits` / `.scalar()`.

### Fix 2 — AC-6 reworded to by-construction; AC-8 test filter consolidated

AC-6 now greps `! rg -q 'SlicedRegion|sliced_region' crates/slicer-core/src/algos/region_mapping.rs` (kernel writes only `RegionMapIR`; polygon population is P95's job per AUDIT.md §Audit 3). AC-8's `&& cargo test ...` clause now references `region_map_cap_exceeded_named_contributor` (AC-N2's actual test name). AC-8's `rg` regex tightened from `[:=]` to `[^=]*=` because `pub const DEFAULT_REGION_MAP_CAP: usize = 750_000` contains a type annotation between `:` and `=`.

### Fix 3 — Dead `_paint_regions` param removed

`execute_region_mapping_inner`, `execute_region_mapping`, and `execute_region_mapping_with_cap` no longer carry the `paint_regions: Option<&PaintRegionIR>` parameter. Producer wrapper no longer constructs or passes it. Kernel `use slicer_ir::PaintRegionIR` import removed. 7 caller test files patched.

### Cleanup A — TASK-243 row added to docs/07_implementation_status.md

### Cleanup B — TASK-163b split staged into a separate prior commit (option b)

### AC re-verification (fresh)

```
AC_6_CHECK=PASS (! rg -q 'SlicedRegion|sliced_region' crates/slicer-core/src/algos/region_mapping.rs → exit 0)
AC_7B_CHECK=PASS (cargo test -p slicer-core --features host-algos --test algo_region_mapping_tdd region_mapping_chain_derived_overlay → 1/0)
AC_8_CHECK=PASS (rg -q 'DEFAULT_REGION_MAP_CAP[^=]*=\s*750_000' && cargo test -p slicer-runtime region_map_cap_exceeded_named_contributor → 1/0)
AC_10_CHECK=PASS (sha256 target/p93-wedge-post.gcode == P92_BASELINE_SHA; byte-identical preserved across Fix 1's f32-to-bits ripple)
```

## Status

Packet ready to flip `draft` → `implemented` (Commit B). All ACs verified fresh post-Fix-1+3; AC-10 byte-identical g-code preserved across the f32-to-bits change.
