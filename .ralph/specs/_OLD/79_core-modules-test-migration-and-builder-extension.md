---
status: implemented
packet: 79
task_ids: [TASK-227, TASK-228]
---

# 79_core-modules-test-migration-and-builder-extension

## Goal

Make every core-module test under `modules/core-modules/*/tests/` that can use shared builders do so — extend `slicer_sdk::test_support` builders with seven new fixture surfaces covering the IR shapes and `SliceRegionView` field surfaces the existing fixtures don't reach (`LayerCollectionIR`, `PrintEntity`, `ToolChange`, variant `WallLoop` flag combos, `SeamCandidate`, axis-aligned-rectangle `ExPolygon` constructor, and `SliceRegionView` top/bottom/bridge fields), then bulk-migrate 13 modules using the consolidated `slicer_sdk::test_prelude` so that no core-module retains hand-rolled `make_*` fixture helpers when a shared builder covers the shape — and adopt the new builders in the two P78-migrated exemplar modules (arachne, rectilinear) whose closure deviations recorded the workaround forms.

## Problem Statement

After packet 78 lands, `slicer_sdk::test_support` owns the canonical module-testing fixture API behind a `test` feature, exposed via `slicer_sdk::test_prelude`. Two exemplar modules (`arachne-perimeters`, `rectilinear-infill`, joined by the P78 packet's continuity check on `gyroid-infill`) prove the consolidated builders cover diverse `ConfigViewBuilder` / `SliceRegionViewBuilder` shapes. But 13 other core-modules with tests still carry hand-rolled `make_*` fixture helpers, and 4 of those (the "Group B" modules in this packet's classification) construct IR types the existing builders don't yet cover: `path-optimization-default` and `seam-placer` build variant `WallLoop` shapes; `skirt-brim` builds `LayerCollectionIR` with `PrintEntity` lists; `wipe-tower` builds `LayerCollectionIR` with `ToolChange` entries. Recon (in this packet's generation phase) confirmed that `crates/slicer-sdk/src/layer_collection_builder.rs` — the production builder — covers a different concern (entity ordering for finalization) and is not the right surface to extend; this packet adds a parallel `LayerCollectionFixtureBuilder` to `test_support/fixtures.rs` instead.

This packet does two things in sequence. **Half one** extends the test_support builders behind the existing feature gate: new `LayerCollectionFixtureBuilder`, freestanding `print_entity(...)`, `tool_change(...)`, `seam_candidate(...)` helpers, and one new method on the existing `PerimeterRegionViewBuilder` (`add_outer_wall_with_flags`) to cover seam-placer's specialized wall-loop shape. Each new fixture surface lands with a `crates/slicer-sdk/tests/test_support_*_tdd.rs` round-trip test before any consumer touches it. **Half two** migrates the 13 modules. Group A (7 modules — `layer-planner-default`, `lightning-infill`, `mesh-segmentation`, `traditional-support`, `tree-support`, `classic-perimeters`, plus `gyroid-infill` verified from P78) maps cleanly to the existing builders. Group B (4 modules — `path-optimization-default`, `seam-placer`, `skirt-brim`, `wipe-tower`) requires the half-one extensions. Group C (3 modules — `fuzzy-skin`, `support-surface-ironing`, `top-surface-ironing`) have tests but no `make_*` helpers; verify they still pass with the post-78 test_prelude available, and add the import only when it shortens the file. 4 modules with no tests (`machine-gcode-emit`, `part-cooling`, `seam-planner-default`, `support-planner`) are skipped — support-planner gains its first test in packet 80 via the relocation of `prepass_support_generation_orca_parity_tdd.rs` from runtime.

The migration discipline is strict: every original assertion must survive the migration verbatim (looser tolerances or skipped checks are rejected). The implementer captures a pre/post assertion snapshot for one representative test per migrated module as a human-readable audit trail (AC-N1).

Without this packet, the consolidation that packets 77 and 78 begin remains half-finished: the builders exist but most modules don't use them; documentation describes a single test surface but the codebase still has 13 modules independently reinventing it.

## Architecture Constraints

- **`LayerCollectionFixtureBuilder` must not collide with `LayerCollectionBuilder`.** The production builder at `crates/slicer-sdk/src/layer_collection_builder.rs` is unchanged in this packet. The new fixture builder lives in `crates/slicer-sdk/src/test_support/fixtures.rs` (different module path) and has a distinct type name. Tests that need both can `use slicer_sdk::test_prelude::LayerCollectionFixtureBuilder;` alongside `use slicer_sdk::LayerCollectionBuilder;`.
- **New `SliceRegionViewBuilder` setters MUST preserve default-field behaviour.** Each new setter writes only its target field; unset setters MUST leave the field at `SliceRegionViewBuilder::new()`'s initial-state default. Pre-existing tests that don't call the new setters must behave identically to today. The TDD locks this with a "construct without new setters → compare against baseline default-built region" assertion before exercising any setter call.
- **`LayerCollectionIR` field defaults are load-bearing.** `LayerCollectionFixtureBuilder::build()` populates only `global_layer_index`, `z`, `ordered_entities`, `tool_changes`; the rest (`z_hops`, `annotations`, `retracts`, `travel_moves`, `schema_version`) come from `..Default::default()`. This relies on the `Default` derive on `LayerCollectionIR` added in TASK-200b. If any of those fields' default values differ from what the original `make_layer` helpers set, the migration would silently change test inputs. The recon at packet generation time confirmed all four Group-B `make_layer` variants use the same `vec![], vec![], vec![], vec![]` literal pattern — matching `Default`.
- **Assertion preservation is non-negotiable.** AC-N1's snapshot ceremony is the audit trail. No test's assertion line may change wording, tolerance, or scope as a side effect of the migration. The implementer captures the snapshots before touching test files, then re-verifies post-migration.
- **`Cargo.lock` MUST regenerate cleanly.** 10 modules gain dev-dep entries on `slicer-sdk` with `features = ["test"]`. Cargo regenerates the lockfile to account for the new feature edges. Commit the diff in the same commit as the `Cargo.toml` edits, not separately. If the diff includes upstream package version changes (unlikely — all deps are internal), audit briefly.
- **Per-feature TDD precedes consumer migration.** Half one's five TDDs (AC-1..AC-5) MUST go green before half two's group-B steps start. This is enforced by `implementation-plan.md` step preconditions; the implementer must not parallelize across the half boundary.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

### Group-B helper bodies (verbatim — captured in recon, do NOT re-dispatch)

These are the exact bodies the migration must replicate via builder chains. Field-for-field invariant preservation is mandatory.

**`path-optimization-default/seam_consumption_tdd.rs::make_wall_loop`** — `WallLoop { perimeter_index: 0, loop_type: Outer, path: ExtrusionPath3D { points: 2 × Point3WithWidth at given (x,y,z,width), flow_factor: 1.0, overhang_quartile: None; role: OuterWall; speed_factor: 1.0 }, width_profile: WidthProfile { widths: vec![width; 2] }, feature_flags: vec![], boundary_type: Interior }`. **Migration**: `PerimeterRegionViewBuilder::add_outer_wall(rect_path-style)` already produces this; the helper can disappear entirely or shrink to a one-line forward to `PerimeterRegionViewBuilder::add_outer_wall(...)`.

**`path-optimization-default/travel_policy_tdd.rs::make_wall_loop`** — same as seam_consumption variant but with hard-coded `width: 0.4`. Same migration.

**`path-optimization-default/retract_mode_propagation_tdd.rs::make_wall_loop`** — same body, same migration.

**`seam-placer/seam_placer_tdd.rs::candidate`** — `SeamCandidate { position: Point3WithWidth { x, y, z, width: 0.4, flow_factor: 1.0, overhang_quartile: None }, score, reason }`. **Migration**: replace body with `seam_candidate(Point3WithWidth { x, y, z, width: 0.4, flow_factor: 1.0, overhang_quartile: None }, score, reason)` from the new `seam_candidate` fixture helper (AC-5).

**`seam-placer/seam_placer_tdd.rs::wall_at_z`** — `WallLoop` with 3 points along (0,0)-(1,0)-(2,0) at given z + width 0.4, role OuterWall, non-empty `feature_flags: vec![WallFeatureFlag { tool_index: None, fuzzy_skin: false, is_bridge: false, is_thin_wall: false, skip_ironing: false, custom: empty }, ...]` (3 entries to match the 3 points), `boundary_type: WallBoundaryType::ExteriorSurface`. **Migration**: helper builds the `ExtrusionPath3D` directly, then passes to `PerimeterRegionViewBuilder::add_outer_wall_with_flags(path, feature_flags, WallBoundaryType::ExteriorSurface)` (AC-4 method).

**`skirt-brim/skirt_brim_tdd.rs::make_entity_at(x, y, z)`** — `PrintEntity { entity_id: 1, path: ExtrusionPath3D { points: [Point3WithWidth { x, y, z, width: 0.4, flow_factor: 1.0, overhang_quartile: None }], role: OuterWall, speed_factor: 1.0 }, role: OuterWall, region_key: RegionKey { global_layer_index: 0, object_id: "obj1", region_id: 1 }, topo_order: 0 }`. **Migration**: `print_entity(1, OuterWall, vec![Point3WithWidth { x, y, z, width: 0.4, ... }], RegionKey { global_layer_index: 0, object_id: "obj1".to_string(), region_id: 1 }, 0)`.

**`skirt-brim/finalization_live_tdd.rs::make_entity_at(layer_index, x, y, z)`** — same as above but `region_key.global_layer_index = layer_index` (not 0). Different parameter list — preserve the (layer_index, x, y, z) signature; body uses `print_entity` with `region_key.global_layer_index: layer_index`.

**`skirt-brim/{skirt_brim_tdd,finalization_live_tdd}.rs::make_layer{,_with_entities}(...)`** — both construct `LayerCollectionIR { schema_version: semver(), global_layer_index: index, z, ordered_entities: entities, tool_changes: vec![], z_hops: vec![], annotations: vec![], retracts: vec![], travel_moves: vec![] }`. **Migration**: `LayerCollectionFixtureBuilder::new().global_layer_index(index).z(z).build()` then chain `.add_entity(...)` for each entity in the input `Vec`.

**`wipe-tower/{wipe_tower_tdd,finalization_live_tdd}.rs::make_layer(index, z, tool_changes)`** — `LayerCollectionIR { schema_version: semver(), global_layer_index: index, z, ordered_entities: vec![dummy_entity(z, index)], tool_changes, ..rest vec![]... }`. **Migration**: a `fold` over the `tool_changes` input keeps the body a single expression:

```rust
fn make_layer(index: u32, z: f32, tool_changes: Vec<ToolChange>) -> LayerCollectionIR {
    tool_changes.into_iter().fold(
        LayerCollectionFixtureBuilder::new().global_layer_index(index).z(z).add_entity(dummy_entity(z, index)),
        |b, tc| b.add_tool_change(tc),
    ).build()
}
```

That body is ~7 lines from `fn` to `}` — fits AC-7's `≤ 8` threshold. The same fold pattern applies to `skirt-brim::make_layer_with_entities` (fold `add_entity` over the `entities: Vec<PrintEntity>` input) and to `skirt-brim::make_layer` whenever its `entities` arg is a runtime `Vec`. The `dummy_entity` helper (in scope but not surveyed) can stay or migrate to `print_entity` at the implementer's discretion.

### Cargo dev-dep line format (verified in packet 78)

For modules under `modules/core-modules/<name>/`, the dev-dep entry is:

```toml
[dev-dependencies]
slicer-sdk = { path = "../../../crates/slicer-sdk", features = ["test"] }
```

Three `../` because the module's `Cargo.toml` is two directory levels below the workspace root (`modules/core-modules/<name>/`).

## Locked Assumptions and Invariants

- **Invariant A**: The five builder extensions (AC-1..AC-5) are all gated by the existing `test_support` feature umbrella from packet 77. No new feature flag is introduced.
- **Invariant B**: `LayerCollectionFixtureBuilder::build()` returns a `LayerCollectionIR` whose unset fields are `Default`. Specifically: `z_hops`, `annotations`, `retracts`, `travel_moves` are empty `Vec`s; `schema_version` is `LayerCollectionIR::default().schema_version` (the `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` constant per TASK-200b). Tests that previously relied on `semver()` to produce a specific version are silently fine because that helper also produces the canonical constant.
- **Invariant C**: Every original assertion in every migrated test file survives the migration verbatim. AC-N1's snapshot ceremony is the audit trail.
- **Invariant D**: Group-C migrations that add the `slicer-sdk` dev-dep MUST use the prelude productively (not import-then-don't-use). Cosmetic imports without usage are rejected at AC-8.
- **Invariant E**: `cargo test --workspace` (AC-11) passes with zero regressions from the pre-packet-79 baseline. The implementer captures the pre-baseline test count from packet 78's closure log; post-packet count must be identical or higher (only if Group-C migrations add new tests, which they shouldn't per scope).
- **Invariant F**: `rect_polygon(cx, cy, w, h)` returns an `ExPolygon` with `contour: Polygon { points }` where `points` is a 4-vertex sequence in CCW winding order at corners `(cx ± w/2, cy ± h/2)`, all in mm-space converted via `mm_to_units`. `holes: vec![]`. Style mirrors `square_polygon` verbatim (same `mm_to_units` call shape, same winding convention).
- **Invariant G**: Each new `SliceRegionViewBuilder` setter is idempotent (calling it twice with the same value yields the same final state) and last-write-wins (calling with different values yields the second value). Unset setters MUST leave the field at `SliceRegionViewBuilder::new()`'s default; the TDD asserts this via a default-built region equality probe before any setter call.
- **Invariant H**: The Group-D tightening (arachne `make_narrow_rect` → `rect_polygon`; rectilinear `r.set_*()` chains → builder setter chains) preserves every assertion in arachne-perimeters and rectilinear-infill tests verbatim. AC-N1's snapshot ceremony extends to cover these two modules in addition to Groups A+B.

## Risks and Tradeoffs

- **Risk: a builder extension's default-field choice silently changes test inputs.** Example: if `print_entity(...)` defaults `topo_order: 0` but a caller expected `topo_order: 5`, the migrated test asserts on stale state. Mitigation: every helper accepts every materially-varying input as an explicit parameter, even if the original `make_*` helper hard-coded it. AC-1 / AC-2 / AC-3 / AC-5 each name the minimum input set; the implementer may add MORE parameters but must not reduce.
- **Risk: assertion-preservation discipline is hard to enforce automatically.** AC-N1's manual snapshot ceremony catches the obvious cases (tolerance loosening, comparator swaps) but a subtle bug — e.g., the migration changes which `assert!` line runs first due to setup ordering — is harder to catch. Mitigation: per-module narrow `cargo test -p <module>` after each migration; halt on first failure; only proceed once green.
- **Risk: `cargo test --workspace` takes ≥ 11 minutes and may surface unrelated flakes.** Mitigation: AC-10's `delta D, identical yes/no` audit dispatches detect regressions vs pre-baseline; any flake that surfaces during this packet's test sweep is investigated for relevance, then either fixed (if related) or documented as pre-existing (if not).
- **Risk: 10 simultaneous Cargo.toml dev-dep additions trigger lockfile churn that includes upstream package changes.** Mitigation: review `Cargo.lock` diff; expect only internal-path-dep changes; if upstream versions move, freeze them via `cargo update --precise` or accept after audit.
- **Tradeoff: extending the production `LayerCollectionBuilder` instead of adding `LayerCollectionFixtureBuilder`.** Rejected: the production builder serves entity ordering for finalization; conflating its API with fixture construction would force test-only fields onto the production type or require feature-gating individual methods, both of which are uglier than a separate fixture builder.
