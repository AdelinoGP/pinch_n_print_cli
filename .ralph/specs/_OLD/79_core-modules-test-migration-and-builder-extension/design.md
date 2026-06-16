# Design — Packet 79

## Controlling Code Paths

This packet has two halves that share `crates/slicer-sdk/src/test_support/fixtures.rs` as the central code surface but touch different module crates downstream.

**Half one — `test_support` builder extensions** (single primary file):

- **`crates/slicer-sdk/src/test_support/fixtures.rs`** — the existing file (moved here from `slicer-test/src/fixtures.rs` in packet 78) gains seven new public surfaces:
  1. `pub fn print_entity(entity_id: u64, role: ExtrusionRole, points: Vec<Point3WithWidth>, region_key: RegionKey, topo_order: u32) -> PrintEntity` — constructs a `PrintEntity` per `docs/02_ir_schemas.md` IR-9. The body sets `path: ExtrusionPath3D { points, role, speed_factor: 1.0 }` and the supplied scalars; leaves any other fields at `Default`.
  2. `pub fn tool_change(after_entity_index: u32, tool_index: u32) -> ToolChange` — minimal `ToolChange` per the IR; defaults the other fields.
  3. `pub fn seam_candidate(position: Point3WithWidth, score: f32, reason: SeamReason) -> SeamCandidate` — constructs a `SeamCandidate`.
  4. `pub struct LayerCollectionFixtureBuilder { global_layer_index: u32, z: f32, entities: Vec<PrintEntity>, tool_changes: Vec<ToolChange> }` with a consuming `mut self -> Self` builder pattern. `build()` returns `LayerCollectionIR { global_layer_index, z, ordered_entities: entities, tool_changes, ..Default::default() }`.
  5. `impl PerimeterRegionViewBuilder { pub fn add_outer_wall_with_flags(mut self, path: ExtrusionPath3D, feature_flags: Vec<WallFeatureFlag>, boundary_type: WallBoundaryType) -> Self }` — mirrors the existing `add_outer_wall` but threads `feature_flags` and `boundary_type` through to the inner `WallLoop`.
  6. `pub fn rect_polygon(cx_mm: f32, cy_mm: f32, width_mm: f32, height_mm: f32) -> ExPolygon` — axis-aligned rectangle ExPolygon constructor mirroring `square_polygon`'s style; corners at `(cx ± w/2, cy ± h/2)` in mm-space converted via `mm_to_units`, CCW winding, empty `holes`. Closes the `make_narrow_rect`-style gap surfaced by packet 78's arachne-perimeters migration where `square_polygon` was too symmetric (single side parameter) and `rect_path` returned the wrong type (`ExtrusionPath3D`, not `ExPolygon`).
  7. `impl SliceRegionViewBuilder { pub fn top_shell_index(self, idx: Option<u32>) -> Self; pub fn top_solid_fill(self, fills: Vec<ExPolygon>) -> Self; pub fn bottom_shell_index(self, idx: Option<u32>) -> Self; pub fn bottom_solid_fill(self, fills: Vec<ExPolygon>) -> Self; pub fn is_bridge(self, on: bool) -> Self; pub fn bridge_areas(self, areas: Vec<ExPolygon>) -> Self; pub fn bridge_orientation_deg(self, deg: f32) -> Self }` — seven new setter methods on the existing `SliceRegionViewBuilder` so post-build `r.set_*()` chains (used by `rectilinear-infill::make_test_region` / `make_bridge_region` in packet 78's migration) collapse to single-expression builder chains. Each setter writes only its target field; unset setters leave the field at `SliceRegionViewBuilder::new()`'s default. The production `SliceRegionView` type itself is unchanged.

- **`crates/slicer-sdk/src/test_prelude.rs`** — re-export the new freestanding fixtures (`pub use crate::test_support::fixtures::{print_entity, tool_change, seam_candidate, LayerCollectionFixtureBuilder, rect_polygon};`). The new `PerimeterRegionViewBuilder::add_outer_wall_with_flags` method and the seven new `SliceRegionViewBuilder` setters are reached via the existing builder re-exports.

- **Seven new TDD files** under `crates/slicer-sdk/tests/` — `test_support_print_entity_tdd.rs`, `test_support_tool_change_tdd.rs`, `test_support_seam_candidate_tdd.rs`, `test_support_layer_collection_fixture_builder_tdd.rs`, `test_support_wall_loop_with_flags_tdd.rs`, `test_support_rect_polygon_tdd.rs`, `test_support_slice_region_view_builder_setters_tdd.rs`. Each is a small round-trip locker (≈ 20-40 lines each).

**Half two — Module migrations**:

- 10 modules in Groups A+B get `Cargo.toml` dev-dep addition + test-file edits replacing `make_*` helper bodies with builder chains. The edits are mechanical per the per-module recon in §Data and Contract Notes.
- 3 modules in Group C get a per-module decision (migrate cosmetic-only OR leave untouched). The decision is documented per module in the implementation log.
- 2 P78-migrated modules (Group D: `arachne-perimeters`, `rectilinear-infill`) get exemplar tightening — adopt the new `rect_polygon` and `SliceRegionViewBuilder` setters from half-one to replace the workaround forms recorded in packet 78's closure deviations (arachne's inline `ExPolygon` literal in `make_narrow_rect`; rectilinear's post-build `r.set_*()` chains in `make_test_region` / `make_bridge_region`). Dogfoods the new builders against the workloads that originally surfaced their need. No `Cargo.toml` changes (both modules already carry the dev-dep from P78).
- 1 module (gyroid-infill) gets a regression run only — `cargo test -p gyroid-infill` to confirm packet 78's migration still passes.

The hard sequencing constraint: every Group-B migration depends on at least one half-one builder extension being green. The `implementation-plan.md` orders steps so half-one TDD completes before any Group-B step starts.

## Architecture Constraints

- **`LayerCollectionFixtureBuilder` must not collide with `LayerCollectionBuilder`.** The production builder at `crates/slicer-sdk/src/layer_collection_builder.rs` is unchanged in this packet. The new fixture builder lives in `crates/slicer-sdk/src/test_support/fixtures.rs` (different module path) and has a distinct type name. Tests that need both can `use slicer_sdk::test_prelude::LayerCollectionFixtureBuilder;` alongside `use slicer_sdk::LayerCollectionBuilder;`.
- **New `SliceRegionViewBuilder` setters MUST preserve default-field behaviour.** Each new setter writes only its target field; unset setters MUST leave the field at `SliceRegionViewBuilder::new()`'s initial-state default. Pre-existing tests that don't call the new setters must behave identically to today. The TDD locks this with a "construct without new setters → compare against baseline default-built region" assertion before exercising any setter call.
- **`LayerCollectionIR` field defaults are load-bearing.** `LayerCollectionFixtureBuilder::build()` populates only `global_layer_index`, `z`, `ordered_entities`, `tool_changes`; the rest (`z_hops`, `annotations`, `retracts`, `travel_moves`, `schema_version`) come from `..Default::default()`. This relies on the `Default` derive on `LayerCollectionIR` added in TASK-200b. If any of those fields' default values differ from what the original `make_layer` helpers set, the migration would silently change test inputs. The recon at packet generation time confirmed all four Group-B `make_layer` variants use the same `vec![], vec![], vec![], vec![]` literal pattern — matching `Default`.
- **Assertion preservation is non-negotiable.** AC-N1's snapshot ceremony is the audit trail. No test's assertion line may change wording, tolerance, or scope as a side effect of the migration. The implementer captures the snapshots before touching test files, then re-verifies post-migration.
- **`Cargo.lock` MUST regenerate cleanly.** 10 modules gain dev-dep entries on `slicer-sdk` with `features = ["test"]`. Cargo regenerates the lockfile to account for the new feature edges. Commit the diff in the same commit as the `Cargo.toml` edits, not separately. If the diff includes upstream package version changes (unlikely — all deps are internal), audit briefly.
- **Per-feature TDD precedes consumer migration.** Half one's five TDDs (AC-1..AC-5) MUST go green before half two's group-B steps start. This is enforced by `implementation-plan.md` step preconditions; the implementer must not parallelize across the half boundary.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

Primary edits (≤ 3 conceptual surfaces):

1. **Builder extensions** — `crates/slicer-sdk/src/test_support/fixtures.rs` (≈ 180 LoC additions; 5 new free functions + 1 new struct + 8 new methods on existing structs), `crates/slicer-sdk/src/test_prelude.rs` (≈ 6 line additions), 7 new `test_support_*_tdd.rs` files under `crates/slicer-sdk/tests/` (≈ 20-40 LoC each).
2. **Group A + Group B migrations** — 10 modules × 2 files average per module = ~20 test files edited; each edit is a helper-body rewrite. Each module also gets one `Cargo.toml` dev-dep line addition.
3. **Group C decisions + Group D tightening + Doc update** — 3 Group-C modules' test files reviewed (edited only if shortening); 2 P78-migrated Group-D modules' test files tightened to adopt new builders (3 helper bodies total — `make_narrow_rect`, `make_test_region`, `make_bridge_region`); `docs/05_module_sdk.md` §Test Support gains 1-2 lines listing the new fixtures.

Secondary edits (mechanical follow-on):

4. `Cargo.lock` (regenerated; committed alongside Cargo.toml diffs).
5. Implementation log file containing the 10-11 assertion snapshots (per AC-N1) — kept in the packet's commit message or attached to the closure ceremony, not in a checked-in file.

## Files in Scope (read+edit)

Edit-allowed:

- `crates/slicer-sdk/src/test_support/fixtures.rs`
- `crates/slicer-sdk/src/test_prelude.rs`
- `crates/slicer-sdk/tests/test_support_print_entity_tdd.rs` (new)
- `crates/slicer-sdk/tests/test_support_tool_change_tdd.rs` (new)
- `crates/slicer-sdk/tests/test_support_seam_candidate_tdd.rs` (new)
- `crates/slicer-sdk/tests/test_support_layer_collection_fixture_builder_tdd.rs` (new)
- `crates/slicer-sdk/tests/test_support_wall_loop_with_flags_tdd.rs` (new)
- `crates/slicer-sdk/tests/test_support_rect_polygon_tdd.rs` (new)
- `crates/slicer-sdk/tests/test_support_slice_region_view_builder_setters_tdd.rs` (new)
- For each Group-A module (`layer-planner-default`, `lightning-infill`, `mesh-segmentation`, `traditional-support`, `tree-support`, `classic-perimeters`): the module's `Cargo.toml` and every file under `tests/`. The set varies per module; the recon enumerates 1-2 test files per module typically.
- For each Group-B module (`path-optimization-default`, `seam-placer`, `skirt-brim`, `wipe-tower`): same pattern.
- For each Group-C module (`fuzzy-skin`, `support-surface-ironing`, `top-surface-ironing`): test files inspected; edited only on the cosmetic-shorter rule.
- For each Group-D module (`arachne-perimeters`, `rectilinear-infill`): test files only (no `Cargo.toml` — already wired in P78). Specifically: `modules/core-modules/arachne-perimeters/tests/arachne_perimeters_tdd.rs` (`make_narrow_rect` body); `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` (`make_test_region` body); `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (`make_bridge_region` body).
- `docs/05_module_sdk.md` (only §Test Support).
- `Cargo.lock` (auto-regenerated).

To-be-deleted: nothing (this packet is purely additive in the SDK and purely substitutive in the modules).

## Read-Only Context

- `crates/slicer-sdk/src/layer_collection_builder.rs` — confirmed 97 lines, entity-ordering only. Read once to confirm there is no overlap with the new `LayerCollectionFixtureBuilder`.
- `crates/slicer-sdk/src/test_support/fixtures.rs` (post-packet-78 location) — the existing file being extended. Read the existing `ConfigViewBuilder`, `SliceRegionViewBuilder`, `PerimeterRegionViewBuilder` impls to mirror their style.
- `docs/02_ir_schemas.md` — read only the field surfaces being constructed: `PrintEntity` (IR-9), `ToolChange` (IR-12 subsection), `LayerCollectionIR` (IR-12), `WallLoop` (IR-6 subsection), `SeamCandidate`. Use line-range loads; never load the whole file (> 600 lines).
- For each Group-A/B module: that module's `src/lib.rs` config-key surface (the strings passed to `config.get_*("...")` calls in `on_print_start` or per-stage entry points) — needed to confirm migration setter keys match production code. Read only the lines surrounding `config.get_*` calls.
- Per-module test files (for Group A/B): read before migration to capture the assertion snapshot (AC-N1).
- `docs/05_module_sdk.md` §Test Support (≈ lines 445-560 post-packet-78) — read once to know where to append the new helper list.

## Out-of-Bounds Files

- All other module source files (anything in `modules/core-modules/*/src/`) except for the narrow `config.get_*` line ranges.
- `crates/slicer-runtime/**` (packet 80).
- `crates/slicer-ir/**` (IR-shape changes are out of scope; this packet constructs existing shapes, doesn't modify them).
- `crates/slicer-core/**`, `crates/slicer-schema/**`, `crates/slicer-helpers/**`, `xtask/**`.
- `crates/pnp-cli/**` (scaffold already correct from packet 78).
- `crates/slicer-macros/**` (macro shape locked since packet 77).
- `crates/slicer-sdk/src/host.rs` (thread-locals frozen since well before packet 77).
- `OrcaSlicerDocumented/**` — never load.
- All `target/`, lockfile internals beyond mechanical regeneration, all `*.wasm` artifacts.
- `crates/slicer-runtime/test-guests/**`.

## Expected Sub-Agent Dispatches

The implementer should plan for these (each with explicit return-format):

1. **IR field-surface confirmation** — `Question: list the field names of PrintEntity, ToolChange, SeamCandidate, ExPolygon (contour + holes), and the SliceRegionView field surface for top_shell_index / top_solid_fill / bottom_shell_index / bottom_solid_fill / is_bridge / bridge_areas / bridge_orientation_deg per docs/02_ir_schemas.md (cross-reference slicer-ir/src/lib.rs for the Rust type definitions if the doc is silent on a field's exact Rust type). Scope: docs/02_ir_schemas.md plus narrow line ranges in slicer-ir/src/lib.rs. Return: FACT (≤ 5 lines per type).`
2. **Per-module config-key extraction (Group A)** — for each Group-A module, `Question: list every config.get_*("key") string used by the module's src/lib.rs. Scope: that file. Return: FACT (≤ 5 lines).` (6 dispatches.)
3. **Per-module config-key extraction (Group B)** — same for the 4 Group-B modules. (4 dispatches.)
4. **Helper-body extraction (Group A)** — for each Group-A module, `Question: verbatim show every fn make_* body in modules/core-modules/<name>/tests/*.rs. Return: SNIPPETS (≤ 1 snippet per helper, ≤ 25 lines each).` (6 dispatches.) NOTE: the Group-B helper bodies are already captured in this packet's §Data and Contract Notes from the generation-phase recon; do not re-dispatch for those.
5. **Per-module test count and pre-migration line count** — for each migrated module, `Question: how many tests in modules/core-modules/<name>/tests/, total LoC across that directory. Scope: that directory. Return: FACT (≤ 3 lines).` Used for AC-7's "≤ original line count" metric.
6. **Builder-extension TDD verification** — `Question: do cargo test -p slicer-sdk --test test_support_<name>_tdd pass for all five new TDD files? Scope: slicer-sdk. Return: FACT: pass count / first failure.`
7. **Workspace test ceremony** — `Question: does cargo test --workspace pass with all packages green? Scope: workspace. Return: FACT: total tests / pass count / fail count / per-package breakdown.` This is the AC-11 closure gate.
8. **Guest staleness recheck** — `Question: does cargo xtask build-guests --check pass after all Cargo.toml dev-dep additions? Scope: xtask. Return: FACT: clean / list of STALE guests.`
9. **Wasm-target gate (3 representative modules)** — `Question: do cargo check --target wasm32-unknown-unknown -p {skirt-brim, seam-placer, classic-perimeters} all pass, AND does cargo tree show none of them activate slicer-sdk's test feature? Scope: workspace. Return: FACT: clean / first violation.`
10. **AC-11 test-count audit** — `Question: capture the pre-packet-79 cargo test --workspace count (from packet 78's closure log) and the post-packet-79 count. Are they identical? Scope: implementation log. Return: FACT: pre N1 post N2 delta D, identical yes/no.` Audit detects regressions.

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

## Context Cost Estimate

- **Aggregate**: L. 15+ steps; bigger blast radius than packets 77/78. Use the natural handoff boundary between half-one (steps 1-7: extensions + TDDs) and half-two (steps 8-onwards: migrations). Implementer should drop to a fresh context at the boundary if utilization is over 50%.
- **Largest single step**: any Group-B migration step (steps 11-14, one per module). Each is M because the helper-body rewrite + assertion-snapshot pre-capture + narrow test run reads a non-trivial number of files. None reaches L individually.
- **Highest-risk dispatch**: dispatch 7 (workspace test ceremony). Long runtime; many files exercised. Return FACT only — never absorb the per-test output.

## Open Questions

None. The grilling session resolved every design decision; the recon at packet generation time confirmed all Group-B helper bodies verbatim (captured in §Data and Contract Notes); the classic-perimeters gap was identified and folded into Group A. Packet 78's exemplar migration subsequently surfaced two additional builder gaps (the `rect_polygon` ExPolygon helper for `make_narrow_rect`-style shapes, and seven `SliceRegionViewBuilder` setters for top / bottom / bridge fields used by `rectilinear-infill::make_test_region` / `make_bridge_region`); both have been folded into half-one's extension list (surfaces 6 and 7), into the implementation plan (new TDD steps 7 and 8, plus a Group-D tightening step 10), and into packet.spec.md's AC list (AC-12 and AC-13). No remaining open questions.
