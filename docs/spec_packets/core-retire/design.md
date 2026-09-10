# Design: core-retire

## Controlling Code Paths

- Primary code path: the production gate `if slice_closing_radius_mm > 0.0 { apply_slice_closing_radius(raw_polygons, slice_closing_radius_mm) } else { raw_polygons }` inside `execute_prepass_slice_single_layer_impl` (`crates/slicer-core/src/algos/prepass_slice.rs`), reached through the public `execute_prepass_slice_single_layer`. Production code is read-only; the packet adds a test that drives this branch for the first time.
- Neighboring tests/fixtures: the two retirement candidates (`clip_operation_variants_are_distinct` in the inline `tests` module of `crates/slicer-core/src/polygon_ops.rs`; `slice_closing_radius_zero_is_noop` in `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs`), their surviving neighbours (`slice_closing_radius_fuses_gap_within_two_r`, which keeps `apply_slice_closing_radius` and `unit_square_expolygon` live), and the gated target `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` with its local `cube_mesh` / `make_global_layer` / `p3` / `identity_transform` / `build_volume` / `sv` helpers. No fixture file is added; the new mesh fixture is a local helper fn.
- OrcaSlicer comparison: none. `apply_slice_closing_radius` ports OrcaSlicer's `slice_closing_radius` round-trip, but this packet asserts only the local gate contract and adds no parity claim.

## Architecture Constraints

- Test-only change: no production function, signature, public API, manifest entry, feature, Cargo target, or fixture file is modified. `ClipOperation` and `apply_slice_closing_radius` both remain production-referenced after the deletions.
- Earn-their-keep discipline (ADR-0064): each retirement records the specific regression input that would otherwise slip through, and each survivor names what it protects. Neither candidate qualifies for the compile-witness carve-out, because both surfaces are already compile-checked by surviving code in the same file.
- Feature gating is load-bearing: `algo_prepass_slice_tdd` carries `required-features = ["host-algos"]` and has no `#![cfg(feature = ...)]` inner attribute, so the stanza is the only gate and Cargo **skips the target outright** without the feature — an explicit `--test algo_prepass_slice_tdd` fails with `requires the features: host-algos`, and a bare `cargo test -p slicer-core` silently omits it. This is AGENTS.md's first false-green mechanism ("Cargo skips them building outright"), not the second (`#![cfg(feature)]`, where "the file compiles to an empty binary"); the two must not be conflated. By contrast `triangle_mesh_slicer_tdd` is auto-discovered and ungated, which is exactly why the replacement test cannot live there.
- Watched-struct literal rule (`docs/21_data_defaults_and_fixtures.md` §3): the watched types this packet's fixture constructs are `ObjectMesh` (7 named `pub` fields, `crates/slicer-ir/src/slice_ir.rs`) and `ActiveRegion` (8 named `pub` fields). Both must carry a `..Default::default()` rest, matching the existing `cube_mesh` and `make_global_layer` helpers in the same file. `ResolvedConfig` is **not** watched: §7.1 blind spot 2 names it explicitly, because macro-generated struct definitions are invisible to the syn-based watchlist scanner — the FRU idiom is still used for it, following the in-crate precedent `build_region_map` in `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs`, for churn resistance rather than gate compliance. `RegionKey` (4 fields), `RegionPlan` (3), `RegionMapIR` (3), `MeshIR` (3), and `IndexedTriangleSet` (2) are all below the five-field threshold.
- Independent-oracle discipline (`docs/22_test_quality.md` §4 Q2): the expected island count, areas, and bounds are derived analytically from the fixture rectangle dimensions, never by calling `apply_slice_closing_radius` or re-running the slicer to produce the expected side of an assertion.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: delete the two unfalsifiable tests; re-home the NEG-3 intent as one contrast-pair test in the gated prepass target that calls `execute_prepass_slice_single_layer` twice against a two-island mesh, varying only the interned `ResolvedConfig.slice_closing_radius` (`0.0` vs `0.04`) behind an identical `RegionKey`; then record the retirement deltas in the real §7 `core` ledger row.
- Exact functions, traits, manifests, tests, and fixtures: delete `clip_operation_variants_are_distinct`; delete `slice_closing_radius_zero_is_noop`; add `prepass_slice_closing_radius_gate_applies_only_when_positive` plus local helpers `two_island_mesh()` and a region-map builder to `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`; reuse the file's existing `sv`, `p3`, `identity_transform`, `build_volume`, and `make_global_layer`. No production symbol is added or changed.
- Rejected alternatives and reasons: rewriting `slice_closing_radius_zero_is_noop` in place was rejected because importing the `host-algos`-gated `prepass_slice` into the ungated `triangle_mesh_slicer_tdd` breaks the default-feature build, and gating that whole file would silently zero out its other 11 tests; a sentinel-only move (r=0 alone) was rejected because a deleted guard degrades to `apply_slice_closing_radius(raw, 0.0)`, whose `offset(+0)`/`offset(-0)` round-trip may be geometrically identical, which would rebuild a false green while removing one; retiring the clip-op test without naming a survivor was rejected because ADR-0064 requires the regression input to be named; extending the census JSON to cover lib targets was rejected as a program-level change that would mutate a committed artifact every remaining wave depends on.

## Files in Scope (read + edit)

Four files rather than three: all three code sites live in `slicer-core` but in different homes by construction - one inline `src` test module and one ungated integration target hold the retirements, while the replacement must land in a third, `host-algos`-gated target - and the progress row is separately owned. The implementation plan splits them so no step edits more than two files.

- `crates/slicer-core/src/polygon_ops.rs` - role: inline test module holding the unfalsifiable clip-op assertion; expected change: delete one `#[test]` fn, nothing else.
- `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` - role: ungated integration target holding the dead NEG-3 test; expected change: delete one `#[test]` fn and its doc comment, retaining the section banner and `unit_square_expolygon`.
- `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - role: `host-algos`-gated prepass target; expected change: append the contrast-pair test plus its local two-island mesh fixture and region-map builder, and extend the `slicer_ir` import with `RegionKey`, `RegionMapIR`, `RegionPlan`, `ResolvedConfig`.
- `docs/specs/test-quality-remediation-plan.md` - role: progress ledger only; expected change: update the real `core` row in §7 with the retirements, the replacement, and the three function-count deltas; do not edit `## Packet Queue`.

## Read-Only Context

- `crates/slicer-core/src/algos/prepass_slice.rs` - symbols `execute_prepass_slice_single_layer`, `execute_prepass_slice_single_layer_impl`, and `is_modifier_namespace_id` skip - read the `RegionKey` construction, the `debug_assert!` on lookup miss, and the closing-radius gate only. Do not edit.
- `crates/slicer-core/src/triangle_mesh_slicer.rs` - symbol `apply_slice_closing_radius` - read its inflate/deflate `Round`-join body and the "must NOT be called when r == 0.0" contract only.
- `crates/slicer-ir/src/slice_ir.rs` - symbols `RegionKey`, `RegionMapIR`, `RegionPlan`, `ConfigId`, `config_for`, `intern_config`, `ActiveRegion`, `GlobalLayer`, `SliceIR`, `SlicedRegion`, `Point2::from_mm` - read field names, the `entries: HashMap<RegionKey, RegionPlan>` type, and the `regions[i].polygons` assertion path only. Large file; ranged reads by symbol only.
- `crates/slicer-ir/src/resolved_config.rs` - the `declare_resolved_config!` declaration lines for `slice_closing_radius` (default `0.049`) and `flat_bridge_closing_join` (default `"miter"`) only.
- `crates/slicer-runtime/tests/executor/prepass_slice_and_shell_tdd.rs` - symbol `make_region_map` - the canonical intern-and-insert idiom to mirror. Read the helper only.
- `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs` - symbol `build_region_map` - the in-crate FRU idiom for `ResolvedConfig` literals.
- `crates/slicer-core/Cargo.toml` - the `[[test]] name = "algo_prepass_slice_tdd"` stanza with `required-features = ["host-algos"]`, and the absence of a `triangle_mesh_slicer_tdd` stanza.
- `docs/specs/test-quality-remediation-plan.md` - delegated sections §7 Ledger, `## Packet Queue`, the row #10 approval note, and the dependency-export note.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` and any external OrcaSlicer checkout - no parity behavior is claimed.
- `target/**`, `Cargo.lock`, generated code, vendored dependencies, and large fixture files - never load.
- All production implementation files for editing; `prepass_slice.rs`, `triangle_mesh_slicer.rs`, and the `polygon_ops.rs` production half are read-only context.
- `docs/specs/test-quality-remediation-census.json` - the census is target-scoped and this packet adds and removes no target; extending it is a program-level change deferred to the final wave.
- Any `slicer-core` test file not listed above, any new test target or file, `crates/slicer-core/src/flow.rs` retirement rows (owned by `core-flow-consolidation`), and all other crate waves.
- `docs/specs/test-quality-remediation-plan.md` outside the §7 `core` row and its heading-to-queue boundary; the parent owns queue-row bookkeeping.
- `docs/spec_packets/core-strengthen/**` for edits; its exports are reconstructed only through the delegated summary.

## Expected Sub-Agent Dispatches

- Question: after deleting the named fn, does the file's `#[test]` count equal the expected value and does the surviving roster match in order?; scope: the two edited test files; return: `FACT` with the count and any roster mismatch.
- Question: does the exact gated filter run one test with zero failures?; scope: `cargo test -p slicer-core --features host-algos --test algo_prepass_slice_tdd`; return: `FACT` on success or bounded failure `SNIPPETS`.
- Question: verify the real §7 heading-to-queue span and update one accumulated `core` row without touching queue rows; scope: `docs/specs/test-quality-remediation-plan.md`; return: `FACT` with the real-plan predicate result.
- Question: run `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `cargo xtask check-test-quality --report`; scope: workspace gates; return: `FACT` pass/fail.

## Data and Contract Notes

- IR/manifest contracts: none changed. The test constructs `RegionMapIR` through its `Default` plus `intern_config`, and inserts one `RegionPlan` under a `RegionKey`; no schema version is touched.
- WIT boundary: none.
- Determinism/scheduler constraints: the `RegionKey` the production code builds hard-codes `variant_chain: Vec::new()`, so the test's key must use an empty chain or the lookup misses. A miss triggers `debug_assert!(false, ...)`, and tests compile with `debug_assertions` on, so a mismatched key panics rather than silently falling back. `region_id` must not have bit 63 set, or `is_modifier_namespace_id` skips the region and the call returns zero regions instead of a miss.

## Locked Assumptions and Invariants

- `ResolvedConfig::default().slice_closing_radius` is `0.049`, not `0.0`; the gate-off arm must set `0.0` explicitly, and `RegionMapIR::default()` pre-seeds `configs[0]` with that default, so `RegionPlan::default()` already resolves to `0.049`.
- `config_for` panics via `.expect` on an unknown key; both the `entries.get` probe and the later `config_for` call require the same key.
- The gate compares `slice_closing_radius_mm > 0.0`; `0.04` is above it and `0.0` is not, and `2r = 0.08` exceeds the fixture's 0.05 mm gap, matching the surviving fuses-test arithmetic.
- `SliceIR.regions[i].polygons` is the assertion path; `infill_areas` is a clone of the same vector in the impl and is therefore not an independent surface.
- Both retired names occur exactly once each under `crates/` today, so after deletion neither resolves anywhere in that tree.
- The discovery census counts targets, not functions; this packet adds and removes no target, so the committed census stays byte-identical and reconciliation happens at function granularity in the ledger.

## Risks and Tradeoffs

- **Stated falsifiability limit.** The r>0 arm catches an inverted gate, a hard-coded radius, and broken `ResolvedConfig` plumbing. The pure "guard deleted" defect degrades to `apply_slice_closing_radius(raw, 0.0)`, an `offset(+0)`/`offset(-0)` `Round`-join round-trip; whether Clipper perturbs the contour enough to fail the r=0 arm's exact area and bounds assertions is **unmeasured** in this session. The r=0 arm is written to maximise the chance of catching it (exact island count, combined area, and both bounding boxes) but the packet does not claim that case is proven caught. This is still strictly stronger than the retired test, which caught nothing.
- Asserting exact contour vertex counts would be more sensitive to a zero-radius round-trip but would pin Clipper output order as a false contract; area, bounds, and island count were chosen instead, consistent with `core-strengthen`'s boolean-geometry approach.
- A two-island mesh fixture is new to this target. Building it as one `ObjectMesh` with two disjoint boxes keeps a single `object_id` and therefore a single `RegionKey`, avoiding a multi-object region-map matrix.
- The ledger row is shared with `core-strengthen`. This packet's predicate deliberately does not require that packet's tokens, so the two do not impose an implementation order on each other; preserving accumulated prior content is enforced as a step postcondition and diff review, not by the predicate.
- Deleting an inline `src` test is invisible to the target-scoped census. The mitigation is an explicit function-count delta in the ledger plus a named `census-target-scope` program-level gap; the alternative, editing the committed census, was rejected above.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (the contrast-pair test: new fixture, region-map construction, and two-arm geometry assertions)
- Highest-risk dispatch and required return format: the gated exact-filter run; return `FACT` with the anchored one-passed/zero-failed line, or bounded failure `SNIPPETS` naming the failing assertion.

## Open Questions

None. The packet is intentionally `draft` pending independent preflight; no activation or implementation decision is made here.
