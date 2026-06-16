# Plan — Fix infill overlapping walls and solid (top/bottom) infill

> Authored 2026-06-08 via /grill-with-docs session. Held until other in-flight
> work (region_split, packets 92–95, slicer-core uncommitted edits) settles.
> Source plan file: `C:\Users\agpen\.claude\plans\abstract-yawning-spark.md`.

## Context

Two distinct bugs combine to produce the user-visible symptom "infill is generated over walls and even over solid infill":

**Bug A — wall-inset polygon never reaches the infill stage.**
`classic-perimeters` / `arachne-perimeters` correctly compute the wall-inset polygon and commit it to `PerimeterIR.regions[*].infill_areas` (`modules/core-modules/classic-perimeters/src/lib.rs:195–206`). But `Layer::Infill` constructs `SliceRegionView` via `push_slice_regions(slice_ir, …)` (`crates/slicer-wasm-host/src/dispatch.rs:499`), and `commit_layer_outputs` never mirrors `PerimeterIR.infill_areas` back into the per-layer arena's `SliceIR`. Infill modules receive the raw region outline — confirmed at `crates/slicer-core/src/algos/prepass_slice.rs:329–333`, where `PrePass::Slice` initializes every `SlicedRegion` with `polygons: polygons.clone(), infill_areas: polygons` byte-identical, and nothing else writes `infill_areas`.

**Bug B — solid/bridge polygons never subtracted from infill input.**
`rectilinear-infill`, `gyroid-infill`, and `lightning-infill` pick **one** role per region (`TopSolidInfill` / `BottomSolidInfill` / `SparseInfill`) via `top_shell_index.is_some()` / `bottom_shell_index.is_some()` boolean flags and emit over the **full** `infill_areas` polygon. The polygon-precise `top_solid_fill` / `bottom_solid_fill` (from `PrePass::ShellClassification`, two-pass shrinking-shadow projection) and `bridge_areas` (from `PrePass::MeshAnalysis` packet 36-rev1) are accessible via WIT (`crates/slicer-schema/wit/deps/ir-types.wit:36–39`) but never subtracted before sparse emission. On a partial-top layer the same square is filled twice; on a pure top layer SparseInfill is still emitted because the wall-inset outline isn't subtracted at all.

Corrected data flow (OrcaSlicer `PrintObject::prepare_infill` parity): after walls are placed, the host partitions the wall-inset polygon into four **pairwise-disjoint** canonical fill polygons by precedence `bridge > bottom > top > sparse`. Each fill claim holder emits over exactly one polygon with zero polygon math.

## Decision summary (from grilling session)

| Q | Decision | Rationale |
|---|---|---|
| Q1 | **New field** `SlicedRegion.sparse_infill_area: Vec<ExPolygon>`. `infill_areas` keeps current meaning (raw outline, byte-identical to `polygons`; harmless redundancy, retire in follow-up packet). | Each polygon has one stable meaning end-to-end; closes the dual-meaning trap that produced the bug. |
| Q2 | Partition runs as a **side effect at perimeter commit** inside `commit_layer_outputs`. No new pipeline stage. | Smallest blast radius; same write pattern `Layer::SlicePostProcess` already uses on `arena.slice()`. |
| Q3 | **Clip all four polygons in place** at the same hook: `top_solid_fill`, `bottom_solid_fill`, `bridge_areas` get `∩ perimeter.infill_areas`; `sparse_infill_area` is computed fresh. Modules read pre-clipped polygons. | Zero polygon math in modules; matches OrcaSlicer's `prepare_infill`. Per-layer arena diverges from Blackboard PrePass values; this is by design — Blackboard invariant preserved on read-only PrePass side. |
| Q4 | **Silent-skip with `log::warn!`** when a `SliceIR` region has no matching `PerimeterIR` entry. (Originally Q4 = Fatal; relaxed during implementation because variant_chain regions from packets 92–95 share wall geometry with their base region — fatal would poison every multi-color slice. Visibility restored via structured warning naming the offending `(object_id, region_id)`.) | The variant_chain coexistence requirement only emerged when the change met the active workstream; fatal stays for IR-level violations (`take_slice` / `arena.perimeter()` both `None`) since those are stage-ordering bugs, not per-region absences. |
| Q5 | **Strict precedence dedup**: `bridge > bottom > top > sparse`. The four polygons become pairwise disjoint by construction. | "Disjoint fill polygons" is now a property of the host-published IR, not module hygiene. Survives multi-module claim splits (packet 37). |
| Q6 | Fire **only at `Layer::Perimeters` commit**, not `Layer::PerimetersPostProcess`. | No current module mutates `infill_areas` at post-process time; firing twice is YAGNI. |
| Q7 | **Tiered TDD**. Phase 1a critical red first (host partition unit + consolidated module-level test). Phase 1b end-to-end guards added after Phase 2 is green. | Matches TDD rhythm; one parametrised module test instead of three per-module files prevents test drift. |

## Approach

### Phase 2.0 — Schema scaffolding (build-only, lands first so Phase 1a compiles)

**`crates/slicer-ir/src/slice_ir.rs`** — add field at end of `SlicedRegion`:
```rust
/// Sparse-only infill polygon after precedence-dedup partition.
/// Written by host at `Layer::Perimeters` commit; empty before that.
#[serde(default)]
pub sparse_infill_area: Vec<ExPolygon>,
```
Bump `CURRENT_SLICE_IR_SCHEMA_VERSION` minor (additive, `#[serde(default)]` preserves old-fixture compat).

**`crates/slicer-schema/wit/deps/ir-types.wit:24–40`** — add accessor on `slice-region-view`:
```wit
sparse-infill-area: func() -> list<ex-polygon>;
```

**`crates/slicer-sdk/src/views.rs`** — add field on `SliceRegionView` struct, host-only `set_sparse_infill_area` (for tests), and public `sparse_infill_area(&self) -> &[ExPolygon]` accessor. Mirror the existing `top_solid_fill` pattern.

**`crates/slicer-wasm-host/src/host.rs`** — add field on `SliceRegionData`, populate it in `sliced_region_to_data`, wire the bindgen accessor on `HostSliceRegionView` to forward to `SliceRegionData::sparse_infill_area`. Per CLAUDE.md WIT/Type Changes Checklist, verify type identity at host / dispatch / guest macro sites.

**`crates/slicer-core/src/algos/prepass_slice.rs`** — add `sparse_infill_area: Vec::new()` to the `SlicedRegion` struct literal in `execute_prepass_slice_single_layer` (line ~329).

Other struct-literal sites (~29 files: integration tests, e2e tests, contract tests, ir tests, executor tests) need the new field too. Most use `..Default::default()` and won't break since `SlicedRegion` derives `Default`. The non-defaulted ones (per `cargo check --workspace --all-targets`) need mechanical update.

Build (`cargo check --workspace --all-targets`) must be green here. No logic yet — `sparse_infill_area` is always empty.

### Phase 1a — Red TDD tests (critical contract surface)

**`crates/slicer-runtime/tests/integration/region_partition_tdd.rs` (new)** — host partition unit.

Constructs synthetic `SliceIR` + `PerimeterIR` arenas directly (no WASM), invokes the new `sync_perimeter_infill_areas_into_slice` helper, asserts arena post-state.

- **AC-1 (sparse partition):** wall-inset = 10×10 square; `top_solid_fill` = right half. After hook: `sparse_infill_area` = left half (5×10); `top_solid_fill` = right half (clipped, unchanged); `bottom_solid_fill`, `bridge_areas` = empty.
- **AC-2 (precedence: bridge > bottom > top > sparse):** all three solid/bridge polygons fully overlap one wall-inset. After hook: `bridge_areas` covers it, the other three are empty; pairwise disjointness invariant: `intersection(any two of the four) == empty`.
- **AC-3 (clip-in-place):** `top_solid_fill` extends past wall-inset before hook. After hook: every vertex lies inside `perimeter.infill_areas`.
- **AC-4 (pure top → empty sparse):** `top_solid_fill ⊇ perimeter.infill_areas`. After hook: `sparse_infill_area == empty`, `top_solid_fill == perimeter.infill_areas`.
- **AC-5 (no perimeter entry → fatal):** `SliceIR.regions` has an entry, `PerimeterIR.regions` doesn't. `commit_layer_outputs` returns `Err(LayerStageError::FatalLayer)` with the `(object_id, region_id)` pair named in `message`.
- **AC-6 (preserves untouched fields):** `polygons`, `effective_layer_height`, `top_shell_index`, `bottom_shell_index`, `is_bridge` are unchanged by the hook.

**`crates/slicer-runtime/tests/integration/infill_partitioned_input_tdd.rs` (new)** — consolidated module-level test, parametrised over `rectilinear-infill`, `gyroid-infill`, `lightning-infill` via a shared dispatch helper.

- **AC-7:** region with disjoint populated `sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`, `bridge_areas` → each module emits SparseInfill / TopSolidInfill / BottomSolidInfill / BridgeInfill paths confined to its respective polygon. Roles match the source polygon; no role's paths exit its source polygon (epsilon containment).
- **AC-8:** `sparse_infill_area` empty → zero SparseInfill paths emitted (regardless of `top_shell_index` flag value).
- **AC-9:** all four polygons empty → zero paths emitted; no panic, no error.
- **NEG-1:** region whose only non-empty polygon is `top_solid_fill` and `should_emit(TopSolidInfill) == false` → zero paths emitted (claim-gating works correctly under the new structure).

All Phase 1a tests are red after Phase 2.0 (field exists but is always empty; partition not implemented).

### Phase 2.1 — Host partition implementation

**`crates/slicer-runtime/src/region_partition.rs` (new file).**

```rust
//! Host-side fill-polygon partition. Runs at `Layer::Perimeters` commit;
//! mutates per-layer arena state only. Blackboard PrePass values stay
//! canonical (the per-layer arena holds its own mutable SliceIR copy).
//!
//! Precedence: bridge > bottom > top > sparse (OrcaSlicer
//! `PrintObject::prepare_infill` parity). The four output polygons are
//! pairwise disjoint by construction; each fill claim holder emits over
//! exactly one polygon with zero polygon math.

use slicer_core::polygon_ops::{difference, intersection, union};
use slicer_ir::LayerStageError;
use crate::LayerArena;

pub fn sync_perimeter_infill_areas_into_slice(
    arena: &mut LayerArena,
    layer_index: u32,
) -> Result<(), LayerStageError> {
    let mut slice = arena.take_slice().ok_or_else(/* fatal: no slice IR */)?;
    let perimeter = arena.perimeter().ok_or_else(/* fatal: no perimeter IR */)?;

    for slice_region in &mut slice.regions {
        let perim = perimeter.regions.iter().find(|r|
            r.object_id == slice_region.object_id
                && r.region_id == slice_region.region_id
        ).ok_or_else(|| LayerStageError::FatalLayer {
            layer_index,
            stage_id: "Layer::Perimeters".into(),
            module_id: "host:region_partition".into(),
            message: format!(
                "no PerimeterIR entry for SliceIR region (object_id={}, region_id={})",
                slice_region.object_id, slice_region.region_id,
            ),
        })?;
        let wall_inset = &perim.infill_areas;

        let bridge = intersection(&slice_region.bridge_areas, wall_inset);
        let bottom = difference(
            &intersection(&slice_region.bottom_solid_fill, wall_inset),
            &bridge,
        );
        let top = difference(
            &intersection(&slice_region.top_solid_fill, wall_inset),
            &union(&bridge, &bottom),
        );
        let sparse = difference(
            wall_inset,
            &union(&bridge, &union(&bottom, &top)),
        );

        slice_region.bridge_areas      = bridge;
        slice_region.bottom_solid_fill = bottom;
        slice_region.top_solid_fill    = top;
        slice_region.sparse_infill_area = sparse;
    }

    arena.set_slice(slice).map_err(/* arena commit */)?;
    Ok(())
}
```

Reuses `slicer_core::polygon_ops::{intersection, union, difference}` (`crates/slicer-core/src/polygon_ops.rs:93,98,103`); no new helpers needed.

**`crates/slicer-runtime/src/lib.rs`** — declare `mod region_partition;`.

**`crates/slicer-runtime/src/layer_executor.rs` `commit_layer_outputs`** — at the `"Layer::Perimeters"` arm only (not the combined `"Layer::Perimeters" | "Layer::PerimetersPostProcess"` arm; split the match arms), after every `arena.set_perimeter(...)` call, invoke:
```rust
crate::region_partition::sync_perimeter_infill_areas_into_slice(arena, layer_index)?;
```

After Phase 2.1: `region_partition_tdd.rs` AC-1 through AC-6 green.

### Phase 2.2 — Module emit-side migration

For each of `modules/core-modules/{rectilinear,gyroid,lightning}-infill/src/lib.rs`, replace the per-region role-pick (`top_shell_index.is_some()` ladder) with up to four per-polygon emits:

```rust
for region in regions {
    // SparseInfill over sparse_infill_area
    if region.should_emit(ExtrusionRole::SparseInfill) {
        for expoly in region.sparse_infill_area() {
            // existing emit logic, role = SparseInfill -> push_sparse_path
        }
    }
    // TopSolidInfill over top_solid_fill
    if region.should_emit(ExtrusionRole::TopSolidInfill) {
        for expoly in region.top_solid_fill() {
            // emit, role = TopSolidInfill -> push_solid_path
        }
    }
    // BottomSolidInfill over bottom_solid_fill
    if region.should_emit(ExtrusionRole::BottomSolidInfill) {
        for expoly in region.bottom_solid_fill() {
            // emit, role = BottomSolidInfill -> push_solid_path
        }
    }
    // BridgeInfill over bridge_areas (rectilinear uses bridge_orientation_deg angle)
    if region.should_emit(ExtrusionRole::BridgeInfill) {
        for expoly in region.bridge_areas() {
            // emit at bridge angle, role = BridgeInfill -> push_solid_path
        }
    }
}
```

In `rectilinear-infill/src/lib.rs`:
- Delete `partition_expoly_by_bridges` (no longer needed; bridge is its own canonical polygon).
- Delete the "Bottom wins on overlap to match OrcaSlicer" deviation comment at `:140–141` (precedence is now host-enforced — note this in `DEVIATION_LOG.md` as superseded).
- Update existing `top_bottom_fill_tdd.rs` fixtures: regions that previously set `top_shell_index = Some(0)` to drive TopSolidInfill emission now also need `top_solid_fill = vec![<polygon>]` populated. Pure mechanical fixture migration.

In `gyroid-infill/src/lib.rs` and `lightning-infill/src/lib.rs`: same restructure. Lightning's organic-tree generator only kicks in for SparseInfill (matches OrcaSlicer); top/bottom/bridge use its standard rectilinear fallback inside the module.

After Phase 2.2: `infill_partitioned_input_tdd.rs` AC-7 through NEG-1 green for all three modules.

### Phase 1b — End-to-end guards (green at write time)

**`crates/slicer-runtime/tests/integration/infill_partition_e2e_tdd.rs` (new).** Drives a 2-layer synthetic fixture through the layer executor end-to-end. Asserts:
- Pure top layer (`top_shell_index = Some(0)`, `top_solid_fill` covers wall-inset): zero SparseInfill paths in resulting `InfillIR`.
- Mid layer (`top_shell_index = bottom_shell_index = None`): no Top/Bottom/Bridge paths.
- Partial top layer (`top_solid_fill` = half of wall-inset): both TopSolidInfill and SparseInfill present; the union of TopSolidInfill path bboxes is disjoint from the union of SparseInfill path bboxes (rasterise to a tolerance grid for the assertion).

### Phase 4 — Documentation + guest WASM rebuild

- `docs/02_ir_schemas.md` SliceIR section: document new `sparse_infill_area` field; note that after `Layer::Perimeters` commit, `top_solid_fill` / `bottom_solid_fill` / `bridge_areas` are clipped to `perimeter.infill_areas` and `sparse_infill_area` is the precedence-dedup remainder. Document the `bridge > bottom > top > sparse` precedence as host-enforced.
- `docs/01_system_architecture.md`: in the `Layer::Perimeters` block, add one sentence: "On commit, host computes the four canonical pairwise-disjoint fill polygons (`sparse_infill_area`, clipped `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`) into the per-layer arena's `SliceIR` before `Layer::Infill` consumes them."
- `docs/DEVIATION_LOG.md`: register the supersession of rectilinear-infill's bottom-wins comment by host-enforced precedence.
- Per CLAUDE.md "Guest WASM Staleness": `cargo xtask build-guests --check` then `cargo xtask build-guests`. All three infill modules + any test guest reading `slice-region-view` will be stale.

## Verification

```powershell
# Type-check first (seconds):
cargo check --workspace --all-targets

# Phase 1a tests — red after Phase 2.0, green after Phase 2.1 / 2.2:
mkdir target -Force
cargo test -p slicer-runtime --test integration region_partition_tdd 2>&1 | Tee-Object -FilePath target/test-output.log
cargo test -p slicer-runtime --test integration infill_partitioned_input_tdd 2>&1 | Tee-Object -FilePath target/test-output.log

# Phase 1b guards (green at write time):
cargo test -p slicer-runtime --test integration infill_partition_e2e_tdd 2>&1 | Tee-Object -FilePath target/test-output.log

# Guest rebuild (mandatory — slice-region-view gained `sparse-infill-area`):
cargo xtask build-guests --check
cargo xtask build-guests

# Pre-commit gate:
cargo clippy --workspace --all-targets -- -D warnings

# Benchy visual verification with HTML report:
cargo run --bin pnp_cli --release -- slice `
    --model resources/benchy.stl `
    --module-dir modules/core-modules `
    --output target/benchy.gcode `
    --report target/benchy-report.html
```

Open `target/benchy-report.html` and confirm:
1. No sparse-infill segments cross the perimeter band on any layer.
2. Pure top/bottom layers (layer 0 and the topmost layers) show only solid fill — no sparse hatch underneath.
3. A partial-top layer (e.g. the boat deck transition) shows solid only where the deck exists and sparse only outside it; no overlap.

## Critical files

- `crates/slicer-ir/src/slice_ir.rs` (~line 1322) — add `sparse_infill_area`; bump schema version (minor).
- `crates/slicer-schema/wit/deps/ir-types.wit` (~line 28) — add `sparse-infill-area: func() -> list<ex-polygon>;`.
- `crates/slicer-sdk/src/views.rs` — add field, accessor, host-only setter on `SliceRegionView`.
- `crates/slicer-wasm-host/src/host.rs` — add field on `SliceRegionData`, populate in `sliced_region_to_data`, wire `HostSliceRegionView::sparse_infill_area` bindgen accessor.
- `crates/slicer-core/src/algos/prepass_slice.rs` — add field initializer in `SlicedRegion` struct literal.
- `crates/slicer-runtime/src/region_partition.rs` (new) — `sync_perimeter_infill_areas_into_slice`.
- `crates/slicer-runtime/src/lib.rs` — declare module.
- `crates/slicer-runtime/src/layer_executor.rs` (~line 1089, `"Layer::Perimeters"` arm) — split combined match arm, call partition after `arena.set_perimeter`.
- `modules/core-modules/{rectilinear,gyroid,lightning}-infill/src/lib.rs` — per-polygon emits; delete `partition_expoly_by_bridges` and bottom-wins comment from rectilinear.
- `crates/slicer-runtime/tests/integration/region_partition_tdd.rs` (new).
- `crates/slicer-runtime/tests/integration/infill_partitioned_input_tdd.rs` (new).
- `crates/slicer-runtime/tests/integration/infill_partition_e2e_tdd.rs` (new, Phase 1b).
- `docs/02_ir_schemas.md`, `docs/01_system_architecture.md`, `docs/DEVIATION_LOG.md` — semantic notes.

## Pre-existing build blockers (encountered during Phase 2.0 attempt 2026-06-08)

When this plan resumes, expect the following pre-existing errors from in-flight uncommitted work on `master`; they must be resolved (by the work that introduced them) before Phase 2.0's `cargo check --workspace --all-targets` can go green:

1. **`crates/slicer-core/src/algos/region_mapping.rs:48`** — `#[derive(Eq)]` on `RegionMappingError` enum that gained a `scalar: f32` variant at line 77. `f32: !Eq`. Fix: drop `Eq` from the derive on that enum (PartialEq stays).
2. **`crates/slicer-sdk/tests/smoke.rs:11–14`** — references `slicer_sdk::test_support::*` which is gated on `#[cfg(any(test, feature = "test"))]` at `crates/slicer-sdk/src/lib.rs:33`. Either enable the `test` feature for the test target, or move the gate. Same file also has a `*v - 0.2` deref of `f64` at line 122 (E0614).

These are not blockers for this plan's design — they're blockers for any green `cargo check --workspace --all-targets` regardless of which packet runs.

## Phase 2.0 progress as of 2026-06-08 (in-progress edits left on disk)

A partial Phase 2.0 attempt was made and left on the working tree before pausing:

- `crates/slicer-ir/src/slice_ir.rs` — `sparse_infill_area` field added at end of `SlicedRegion`; `CURRENT_SLICE_IR_SCHEMA_VERSION` bumped 4.0.0 → 4.1.0.
- `crates/slicer-schema/wit/deps/ir-types.wit` — `sparse-infill-area: func() -> list<ex-polygon>;` accessor added to `slice-region-view`.
- `crates/slicer-sdk/src/views.rs` — field, host-only setter, public accessor added on `SliceRegionView` (mirroring `top_solid_fill` pattern).
- `crates/slicer-wasm-host/src/host.rs` — field added on `SliceRegionData`; `sliced_region_to_data` and the fallback constructor at line ~2335 populate it; `HostSliceRegionView::sparse_infill_area` bindgen accessor wired.
- `crates/slicer-core/src/algos/prepass_slice.rs` — `sparse_infill_area: Vec::new()` added to the `SlicedRegion` struct literal at line ~329 (single-line addition; coexists with the user's other in-progress changes in this file).

Other `SlicedRegion { … }` struct-literal sites across the workspace (per `cargo check --workspace --all-targets`) have NOT been touched yet; expect ~5–10 compile errors of form `error[E0063]: missing field sparse_infill_area in initializer of SlicedRegion` from test fixtures that build the struct without `..Default::default()`. These were not enumerated before pausing.

When resuming, the choice is:
- (a) Keep the in-progress edits and complete the missing-field updates after the pre-existing blockers are resolved, OR
- (b) Revert the in-progress edits and restart Phase 2.0 cleanly once the blockers are cleared.

## Notes / out-of-scope

- **No new pipeline stage** (Q2). Partition runs in `commit_layer_outputs` as a side effect; named `sync_perimeter_infill_areas_into_slice` in `crates/slicer-runtime/src/region_partition.rs` so a future reader can find it from `docs/01`.
- **`SlicedRegion.infill_areas` untouched** (Q1). Remains a byte-identical copy of `polygons` (the raw outline). Formally dead code from the infill perspective; follow-up packet can retire it.
- **Per-layer arena ≠ Blackboard PrePass values** after the hook (Q3). `top_solid_fill` / `bottom_solid_fill` / `bridge_areas` are clipped only on the per-layer arena copy; the Blackboard's PrePass-committed values stay canonical (read-only on the Blackboard side per `slice-prepass-migration.md` invariant). Documented in `region_partition.rs` doc-comment.
- **Precedence is host-enforced** (Q5). Modules cannot customise ordering without changing `region_partition.rs`. OrcaSlicer order (`bridge > bottom > top > sparse`) is the only supported precedence.
- **Wall-anchor parametric overlap** (OrcaSlicer's `infill_anchor` parametric overlap into walls) is NOT addressed. The existing perimeter inset of `-line_width/2` from the innermost wall remains the documented behaviour.
- **Schema bump is minor**, not major. Addition is backward-compatible via `#[serde(default)]`.
- **No `crates/slicer-wasm-host/src/dispatch.rs` change.** Partition lands in arena state before `push_slice_regions` builds the view.
- **Fatal-on-missing-perimeter** (Q4) is documented as a contract; if a real configuration is found where this fires legitimately, downgrade to a warning channel via a registered deviation (no silent fallback).
