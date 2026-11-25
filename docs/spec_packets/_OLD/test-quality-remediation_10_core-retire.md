---
status: implemented
packet: test-quality-remediation_10_core-retire
task_ids:
  - 'core/RETIRE (excluding flow)'
---

# test-quality-remediation_10_core-retire

## Goal

Retire the two `slicer-core` tests that no production defect can falsify, and re-home the slice-closing-radius no-op intent as a real contrast pair that drives the production gate in the `host-algos`-gated prepass target.

## Problem Statement

Two `slicer-core` tests carry names that claim behavioral protection their bodies cannot deliver, and ADR-0064 puts the burden of proof on the test. `clip_operation_variants_are_distinct` asserts `assert_ne!(ClipOperation::Union, ClipOperation::Difference)` over a fieldless enum with a derived `PartialEq`, so discriminant inequality holds for every possible compilation and no production defect can falsify it. `slice_closing_radius_zero_is_noop` sets `let r = 0.0_f32` and branches on `if r > 0.0`, which is statically false, so it compares `polygons.clone()` against `polygons` and never calls `apply_slice_closing_radius`; deleting the real gate in `crates/slicer-core/src/algos/prepass_slice.rs` leaves it green, and its own doc comment concedes it simulates the caller. This is one coherent slice because both are the same failure mode under `docs/22_test_quality.md` gate rule R1, whose legitimate-form column reads "none - retire instead", and because the second one's intent is worth preserving rather than discarding: no test in `slicer-core` drives the closing-radius gate today. The `flow.rs` retirement rows stay with `core-flow-consolidation`.

## Architecture Constraints

- Test-only change: no production function, signature, public API, manifest entry, feature, Cargo target, or fixture file is modified. `ClipOperation` and `apply_slice_closing_radius` both remain production-referenced after the deletions.
- Earn-their-keep discipline (ADR-0064): each retirement records the specific regression input that would otherwise slip through, and each survivor names what it protects. Neither candidate qualifies for the compile-witness carve-out, because both surfaces are already compile-checked by surviving code in the same file.
- Feature gating is load-bearing: `algo_prepass_slice_tdd` carries `required-features = ["host-algos"]` and has no `#![cfg(feature = ...)]` inner attribute, so the stanza is the only gate and Cargo **skips the target outright** without the feature — an explicit `--test algo_prepass_slice_tdd` fails with `requires the features: host-algos`, and a bare `cargo test -p slicer-core` silently omits it. This is AGENTS.md's first false-green mechanism ("Cargo skips them building outright"), not the second (`#![cfg(feature)]`, where "the file compiles to an empty binary"); the two must not be conflated. By contrast `triangle_mesh_slicer_tdd` is auto-discovered and ungated, which is exactly why the replacement test cannot live there.
- Watched-struct literal rule (`docs/21_data_defaults_and_fixtures.md` §3): the watched types this packet's fixture constructs are `ObjectMesh` (7 named `pub` fields, `crates/slicer-ir/src/slice_ir.rs`) and `ActiveRegion` (8 named `pub` fields). Both must carry a `..Default::default()` rest, matching the existing `cube_mesh` and `make_global_layer` helpers in the same file. `ResolvedConfig` is **not** watched: §7.1 blind spot 2 names it explicitly, because macro-generated struct definitions are invisible to the syn-based watchlist scanner — the FRU idiom is still used for it, following the in-crate precedent `build_region_map` in `crates/slicer-core/tests/paint_segmentation_per_region_shell_config_tdd.rs`, for churn resistance rather than gate compliance. `RegionKey` (4 fields), `RegionPlan` (3), `RegionMapIR` (3), `MeshIR` (3), and `IndexedTriangleSet` (2) are all below the five-field threshold.
- Independent-oracle discipline (`docs/22_test_quality.md` §4 Q2): the expected island count, areas, and bounds are derived analytically from the fixture rectangle dimensions, never by calling `apply_slice_closing_radius` or re-running the slicer to produce the expected side of an assertion.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

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
