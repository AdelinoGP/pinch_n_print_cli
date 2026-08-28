# Design: 252-visual-debug-silhouette-remaining-taps

## Controlling Code Paths

- Primary code path: `validate_request` → `run_model_source` silhouette branch (`crates/pnp-cli/src/visual_debug.rs`, authored by packet 247) → `execute_blackboard_taps` (`crates/slicer-runtime/src/layer_executor.rs`) → packet 247's `render_silhouette_composite` (RegionMapping groups) / **new** `render_silhouette_overhang_composite` (OverhangAnnotation groups) in `crates/slicer-runtime/src/visual_debug_render.rs` → `Canvas::fill_polygon` via `Projector`.
- Capture shapes (verified, unchanged by this packet): `CapturedIr::RegionMapping { region_map: RegionMapIR, slice_ir: Vec<SliceIR> }` — the whole-print slice rows are retained in the capture for the render-time join; `CapturedIr::SurfaceClassification(SurfaceClassificationIR)` — shared by `PrePass::MeshAnalysis` and `PrePass::OverhangAnnotation`, carries `overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` (per-layer keyed) and **no** per-region heights.
- Top-down siblings mirrored, never edited: `region_mapping_shapes` (full-tuple join, `(object_id, region_id, variant_chain)` sort, skip-on-miss, `config_tint(region_map.config_for(key))`), `surface_classification_shapes` (keyed `overhang_quartile_polygons.get(&layer_index)` lookup; note it paints all bands uniform `palette::SURFACE_OVERHANG` — there is **no** existing per-quartile palette; this packet introduces one for silhouettes only).
- Producer invariants relied on: `overhang_annotation_producer.rs` (`crates/slicer-runtime/src/builtins/`) derives per-object footprints from committed `SliceIR` region polygons and diffs consecutive layers, then merges objects **by quartile** (band polygons concatenated in `mesh.objects` order — a `QuartileBand` carries no object/region identity, at most 4 bands per layer, producer-sorted by quartile). Hence: band polygons ⊆ that layer's region polygons (same integer coordinate space), and band→height attribution must be geometric, not by identity.
- OrcaSlicer comparison: none — PnP-native tool; no parity obligations.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Projector single-owner rule (archived spec, binding): all silhouette pixels go through `Projector::project(x_or_y_mm, z_mm)` — this packet adds no transform and reuses 247's rectangle-emission machinery.
- Z is mm floats end-to-end; polygon X/Y read via `Point2::to_mm` only. `polygon_ops::intersection` operates in the integer polygon space *before* projection — no mm round-trip.
- D1 (binding): slabs are `[z − effective_layer_height, z]` per region for SliceIR-height-capable taps. Both taps here are SliceIR-height-capable (RegionMapping joins slice rows; overhang bands derive from slice footprints), so schedule z-diffs are **prohibited** as their slab source.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): new test literals of watched types (`SliceIR`, `SlicedRegion`, `RegionMapIR`, `SurfaceClassificationIR`, …) need `..` FRU or an `// exhaustive: <reason>` waiver — follow `visual_debug_blackboard_tap_tdd.rs`'s seeded fixtures (`seeded_region_map`, `seeded_surface_classification`).

## Code Change Surface

- Selected approach — two asymmetric halves, matching what each capture can honestly attest:
  1. **RegionMapping: self-contained extraction arm.** The capture already retains the whole-print `Vec<SliceIR>`, so the arm joins at render time exactly like the top-down and sources each rectangle's slab from the joined `SlicedRegion.effective_layer_height`. No new inputs, no signature change to any 247 export.
  2. **OverhangAnnotation: renderer-side height index, no capture change.** A new caller-built input (`SilhouetteSliceHeightIndex`) carries, per layer, the layer's region footprints grouped by exact `effective_layer_height`; the assembly builds it once from the Blackboard's committed slice rows (`Blackboard::slice_ir`, `crates/slicer-runtime/src/blackboard.rs` — the same slot the capture adapter reads). A new entry point consumes it; slabs come only from height classes, never from the 247 slab schedule.

- Exact surface, per file:
  - `crates/slicer-runtime/src/visual_debug_render.rs`
    - RegionMapping extraction arm in the composite extraction (in a 247-only tree this is inside `render_silhouette_composite`; once packet 249's styled delegation lands the arm sits in the shared internals both entry points use — the contract is behavioral, seam-agnostic). Per capture: filter `region_map.entries` to `capture.layer_index`, sort by `(object_id, region_id, variant_chain)`, resolve against the capture's own `slice_ir` by the full tuple (`global_layer_index`, `object_id`, `region_id`, `variant_chain`); unjoined keys are counted and skipped; joined regions emit `(tint-class, interval, [capture.layer_z − region.effective_layer_height, capture.layer_z])` triples from `regions[].polygons` contours.
    - Tint classes: key = `config_tint(...)` RGB triple; per-(layer, tint) interval union (247's exact-comparison sweep); class paint order ascending `(r, g, b)` lexicographic; rectangle emission stays ascending layer → class order → interval start. New warning (deterministic, deduped per group, appended after 247's W1/W2/occlusion slots): `region mapping: {n} entries had no joined SliceIR region and were skipped`.
    - `pub struct SilhouetteLayerHeightClass { pub effective_layer_height: f32, pub footprint: Vec<ExPolygon> }`; `pub struct SilhouetteSliceHeightIndex { pub layers: BTreeMap<u32, Vec<SilhouetteLayerHeightClass>> }`; `pub fn build_silhouette_slice_height_index(slice_rows: &[SliceIR]) -> SilhouetteSliceHeightIndex` — per row, group regions by `effective_layer_height.to_bits()` (exact, no epsilon — the same discipline as 247's interval comparisons), classes sorted ascending by height, footprint = the class's regions' `polygons` concatenated (used only as a clip input; no union needed).
    - `pub fn render_silhouette_overhang_composite(captures: &[StageCapture], view: SilhouetteView, resolution_scale: u32, viewport: ViewportBoundsMm, height_index: &SilhouetteSliceHeightIndex) -> Result<(RenderedImage, Vec<String>), RenderError>` — scale check as 247; per capture: `bands = sc.overhang_quartile_polygons.get(&capture.layer_index)` (keyed lookup, no map iteration; absent → contributes nothing), bands sorted ascending by `quartile` (defensive re-sort; producer already sorts); `quartile ∉ 1..=4` → `RenderError::InvalidQuartile`; height classes from `height_index.layers[&capture.layer_index]` — absent while bands exist → `RenderError::MissingGeometryField` with `field` naming the height index (fail closed, never a substituted slab). One class → all band polygons project to intervals with slab `[capture.layer_z − h, capture.layer_z]`. Multiple classes → per band polygon, `slicer_core::polygon_ops::intersection(&[band_poly], &class.footprint)` per class in ascending-height order; each piece projects with its class's slab. Zero rectangles across the group → `RenderError::MissingGeometryField` (247's rule). `per_object` footprints and `prev_layer_boundaries` are never read.
    - `pub enum RenderError` gains `InvalidQuartile { tap: String, layer_index: u32, quartile: u8 }` + Display arm. Blast radius: `RenderError` is matched by variant only in its own Display impl (this file); pnp-cli consumes it through error conversion at its `render_stage_capture_styled` call sites, never a variant `match` (verified by grep — the only variant-matched render error in pnp-cli is the unrelated `GcodeRenderError`). Adding the variant therefore touches the Display impl plus its pinning test only.
    - Four new `palette` constants for quartiles 1–4 — RGB chosen at implementation, pairwise distinct from each other, from `BACKGROUND`, and from every color the silhouette path can paint (`SLICE_REGION`, `SUPPORT*` family, 247's new constants, and plausible `config_tint` outputs are unconstrained — tint range is `60..=239` per channel, so quartile constants should use at least one channel outside that range to be unambiguous in decoded-pixel tests).
  - `crates/slicer-runtime/src/lib.rs` — re-export `render_silhouette_overhang_composite`, `build_silhouette_slice_height_index`, `SilhouetteSliceHeightIndex`, `SilhouetteLayerHeightClass` beside the 247 re-exports.
  - `crates/pnp-cli/src/visual_debug.rs`
    - `SILHOUETTE_TAP_STAGE_IDS` += `"PrePass::RegionMapping"`, `"PrePass::OverhangAnnotation"`; delete their `SilhouetteUnsupportedForTap` reason arms (MeshAnalysis/SeamPlanning/arena reasons untouched).
    - `run_model_source` silhouette branch: OverhangAnnotation groups build the height index once via `build_silhouette_slice_height_index(ctx.blackboard.slice_ir())` and call the overhang entry point; RegionMapping groups flow through the existing composite call unchanged. Entry emission (`view`, `layers_rendered` from group capture indices, warnings, filename via `sanitize_path_component`) identical to 247's groups. Under 249's (tap, view, color mode) grouping, tool-mode groups for either tap fail with the existing `RenderError::ToolColorUnavailable` naming the tap — same contract as every blackboard capture (the seam — assembly guard or extraction-arm check — follows wherever 249 put the `CapturedIr::Slice` rejection).
  - Tests: extend `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` (AC-1..6, AC-N3..N5), `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` (AC-8, AC-N1, AC-N2 — retire the two arms of `silhouette_unsupported_taps_rejected_with_reasons`, keep its remaining arms passing), `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs` (AC-7).
  - Docs: `docs/19_visual_debug.md` — two tap-table rows (anchors `quartile`, `tint class`; both verified absent from the doc today).

- Rejected alternatives and reasons:
  - **`CapturedIr` shape change** (add `slice_ir` to the `SurfaceClassification` variant, or a new `OverhangAnnotation` composite variant) — `CapturedIr` is `#[serde(tag = "kind", content = "value")]` and serialized into every top-down entry's `typed_capture`; changing the variant's payload shape (tuple → struct) or its tag rewrites the serialized 1.0/1.1 output for `PrePass::MeshAnalysis`/`PrePass::OverhangAnnotation` top-down bundles — a break of the pinned byte-compat invariant 247's AC-8 freezes. Blast radius would also touch every `CapturedIr` match arm (`schema_version_string`, the render dispatch, the capture adapter, and the `visual_debug_blackboard_tap_tdd.rs` assertions that destructure `CapturedIr::SurfaceClassification`).
  - **Schedule z-diff slabs from 247's `SilhouetteSlabSchedule`** — D1's rejection applies: overhang bands derive from slice footprints, and on a catch-up layer the material truly reaches `catchup_z_bottom`, below the previous global z; the schedule diff draws a thinner band than exists. The plan's D8 row explicitly names "the same layer's SliceIR region heights" as this tap's slab source.
  - **Extending 247's `SilhouetteScheduleSlab`/`SilhouetteSlabSchedule` or `render_silhouette_composite`'s signature with height data** — those are exports packets 249/250 build against (struct literals and call sites in their designs); an additive field or parameter invalidates two read-only sibling contracts. A separate entry point + separate index type composes instead.
  - **Representative-point band→region attribution** (assign each band polygon to the region containing its first vertex) — diff-derived band vertices routinely lie *on* region contours, making point-in-polygon on a boundary vertex implementation-defined; the boolean intersection partition is exact and robust, and runs only on mixed-height layers.
  - **Uniform `SURFACE_OVERHANG` for all bands** (the top-down status quo) — flattens the quartile severity signal that is the tap's entire content on a silhouette (D8 row says "bands"; the packet directive requires per-quartile classes with D2 paint-order discipline).
  - **Tint-class paint order by first-contributing join key** — ties class order to entry layout rather than content; ascending RGB is a total, content-derived order and needs no tie-break rule.
  - **Skipping band-less layers via a warning** — a layer with no overhang is normal geometry, not an omission; only the all-layers-empty group fails closed (mirrors the top-down empty-shapes contract).

### Delegated decisions (from the queue directive), resolved here

1. **OverhangAnnotation slab source**: renderer-side `SilhouetteSliceHeightIndex` built by the assembly from `Blackboard::slice_ir` — no capture-shape change, no schedule z-diffs (rationale above; this is the design that "does not lie about slabs").
2. **Mixed-height partition semantics**: exact geometric partition via `polygon_ops::intersection`; residue is impossible by producer construction (band ⊆ current-layer footprint), so no residue warning exists — a nonempty symmetric difference would be a producer bug, not a renderer condition.
3. **Quartile paint order**: ascending 1→4, Q4 (most severe) painted last so severity survives projected overlaps; bands within a layer re-sorted by `quartile` defensively.
4. **Warning inventory**: one new warning (unjoined RegionMapping entries); 247's occlusion warning fires for both taps' class overlaps unchanged; warning order stays deterministic — 247's slots first, the new warning after.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/visual_debug_render.rs` — both renderer halves, palette constants, `RenderError::InvalidQuartile`; the packet's largest edit.
- `crates/pnp-cli/src/visual_debug.rs` — whitelist lift + assembly routing.
- `crates/slicer-runtime/src/lib.rs` — re-exports only (one small block).
- Tests: `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`, `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`, `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs`.
- Docs: `docs/19_visual_debug.md` (two table rows).

Justification for exceeding three primaries: the packet spans the same CLI-crate/runtime-crate/test seam 247 established; each step stays ≤3 edits.

## Read-Only Context

- `crates/slicer-runtime/src/layer_executor.rs` — `CapturedIr` variants + the `execute_blackboard_taps` adapter arms only — capture shapes; never edited.
- `crates/slicer-runtime/src/blackboard.rs` — `slice_ir()` accessor signature only; never edited.
- `crates/slicer-ir/src/slice_ir.rs` — `RegionMapIR`/`RegionKey`/`RegionPlan`/`config_for`, `SurfaceClassificationIR`/`QuartileBand`, `SlicedRegion.effective_layer_height` definitions only; never edited.
- `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` — module docs for the band⊆footprint and merge-by-quartile invariants; never edited.
- `crates/slicer-core/src/polygon_ops.rs` — `intersection` signature only; never edited.
- `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` — `seeded_region_map`/`seeded_surface_classification`/`seeded_slice_ir` fixture patterns to copy; never edited.
- `docs/spec_packets/247-visual-debug-silhouette-core/design.md` — foundation exports, warning machinery, filename scheme.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not applicable (no parity), never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `modules/core-modules/**` sources — guest artifacts matter only as prebuilt test dependencies (`cargo xtask build-guests --check` before blaming Step-4 failures).
- Packet dirs 247–251 beyond the two 247 files named above — read-only siblings; never create or edit files there.
- `docs/07_implementation_status.md` — worker-dispatch updates only at the completion gate, never a full read.

## Expected Sub-Agent Dispatches

- Question: run the step's test command and report pass/fail with failing test names; scope: the step's single `cargo test -p <crate> --test <file>` invocation tee'd to `target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure; purpose: every step's verification.
- Question: does `cargo xtask check-literals` pass after the new fixtures, and if not which literal/type trips it?; scope: repo root command; return: `FACT` (exit code + offending files ≤5); purpose: Steps 1–3.
- Question: list every match/destructure site of `RenderError` outside `visual_debug_render.rs`'s Display impl; scope: `crates/`; return: `LOCATIONS ≤10`; purpose: Step 2 re-verifies the `InvalidQuartile` blast radius at implementation time (ledger-adjacent — re-derive, don't trust this design's snapshot).
- Question: exact current shape of the composite extraction seam (did 249's styled delegation land?); scope: `crates/slicer-runtime/src/visual_debug_render.rs` symbol scan for `render_silhouette_composite_styled`; return: `FACT`; purpose: Step 1 places the RegionMapping arm at the live seam.

## Data and Contract Notes

- No WIT, IR-struct, schema-version, or manifest-shape changes. `typed_capture` stays absent on silhouette entries (D7); 1.0/1.1 serialization output is untouched (no `CapturedIr` or `ImageEntry` edits).
- Determinism: captures arrive sorted (STAGE_ORDER position, then layer); rectangle emission ascending layer → class order (RGB-ascending tints / quartile-ascending bands) → interval start; band lookup is keyed (`get(&layer_index)`), height-index layers are a `BTreeMap`, band lists re-sorted by `quartile` — no `HashMap` iteration order reaches any output. `RegionMapIR.entries` is a `HashMap`: the join-key sort is what launders it (same discipline as `region_mapping_shapes`).
- `config_tint` is a pure FNV-1a function of the `ResolvedConfig` Debug form — stable across processes/builds (its doc comment pins this), so RGB-ascending class order is deterministic.
- Warnings: element-for-element deterministic; new unjoined-entry warning appended after 247's slots, deduped per group.

## Locked Assumptions and Invariants

- Band polygons are subsets of their layer's `SliceIR` region polygons (producer: footprint diffs of committed region polygons; same integer coordinate space) — the partition is exact, residue impossible.
- A `QuartileBand` carries no object/region identity and may mix objects (producer merges by quartile) — attribution is geometric only.
- Regions of one object on one global layer share `effective_layer_height` (heights are per-object layer schedule state); distinct heights on one layer imply distinct objects with physically disjoint XY footprints, so per-class intersections never double-attribute.
- Slabs: RegionMapping `[capture.layer_z − joined region's effective_layer_height, capture.layer_z]`; OverhangAnnotation `[capture.layer_z − class height, capture.layer_z]`. Neither tap ever reads `SilhouetteSlabSchedule`.
- `SILHOUETTE_TAP_STAGE_IDS` after this packet = 247's five + 249's `PostPass::LayerFinalization` + 250's `PostPass::GCodeEmit` + these two; the D8 whitelist is closed — remaining rejections (MeshAnalysis, SeamPlanning, arena) are permanent plan §8 exclusions, not interim.

## Risks and Tradeoffs

- The RegionMapping capture clones the whole-print `Vec<SliceIR>` per selected layer (pre-existing adapter behavior, same acceptance as 247's support-tap clone note); the height index instead reads the Blackboard slot once per bundle — no new per-layer clones.
- `polygon_ops::intersection` runs only on mixed-height layers with bands; single-height layers (the common case, and all single-object prints) take the no-boolean fast path. Cost unmeasured; bounded by (bands × height classes) per mixed layer.
- Two distinct `ResolvedConfig`s can hash-collide to one tint (top-down accepts the same collision); the silhouette then merges them into one class — pixels are identical either way, so nothing is hidden that the top-down would show.
- 249's styled-entry refactor moves the extraction seam this packet's RegionMapping arm lands in; the dispatch in Expected Sub-Agent Dispatches re-derives the live seam at implementation time rather than freezing it here.
- AC-N2's tool-rejection pin is authored against the post-249 tree (queue order); if this packet were ever implemented against a 247-only tree, that one test would need its interim `InvalidColorBy` form — the AC's annotation records this.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — overhang renderer + height index + fail-closed arms)
- Highest-risk dispatch and required return format: Step 4's end-to-end wedge bundle run (`FACT pass/fail`, preceded by `cargo xtask build-guests --check` exit code).

## Open Questions

- `[FWD to the batch orchestrator]` This packet closes 247's unowned-tap `[FWD]` (echoed by 249/250/251's batch-report notes): after row #6, every D8 tap is owned and the remaining rejections are permanent plan §8 exclusions. Record the closure in the batch report.
- No `[BLOCK]` items.
