# Design: 96_paint-segmentation-phase5-width-limit

## Controlling Code Paths

- **Primary code paths** (Phase 5 kernel + integration):
  - `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — driver `pub fn execute_paint_segmentation` at line 393; integration inserted between the inlined variant-composition block end at line 802 (the `working[i].regions = new_regions;` write inside the `if !new_regions.is_empty()` guard at line 801) and the final `Ok(Arc::new(working))` return at line 999.
  - `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` — schema landing site (see §"Schema Landing Site" below).
- **Bisector-edge ownership surface** (TASK-246-BISECTOR; AC-22b):
  - `crates/slicer-ir/src/slice_ir.rs:1273` — `SlicedRegion` struct gains a new `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` field.
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — driver populates the mask between the variant-composition block end (`mod.rs:802`) and the Phase 5 call.
  - `modules/core-modules/classic-perimeters/src/lib.rs` — `run_perimeters` at line 85; polygons consumed at line 94; outer-wall loop at lines 111–118; edge iteration at line 153 (apply skip mask for outer walls only).
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs:337` — remove `#[ignore]` from `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one`.
- **Neighboring tests/fixtures**:
  - Six kernel unit tests in `width_limit.rs` `#[cfg(test)] mod tests`.
  - One driver-level unit test asserting `!beam` skip behavior.
  - Three NEW integration test files under `crates/slicer-runtime/tests/executor/` (named in the In-Scope list).
  - Potential small `resources/cube_4color_tall.3mf` (≤ 100 KB) if existing fixture is too short for layer-alternation visibility.
- **OrcaSlicer comparison surface**:
  - `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` line 1294 (depth-selection ternary; STANDALONE `interlocking_depth` on even layers, NOT additive with `cut_width`).
  - Same file line 2452 (caller-side guard: `cut_segmented_layers` is invoked only when `!interlocking_beam`).
  - These two FACTs were validated during the spec-review-fix recon; their effect is encoded in the corrected kernel sketch and driver integration below.

## Schema Landing Site

The original P96 draft proposed two alternatives: host effective-config schema OR a `modules/core-modules/paint-segmentation-default/<name>.toml` module manifest. The spec-review-fix recon (Q5/Q7) confirmed neither alternative is reachable as-stated: `paint-segmentation-default` does NOT exist as a core-module, and there is no existing paint-segmentation host-side schema declaration site.

**Decided landing site**: `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`.

Rationale:
- `mesh-segmentation` is the existing core-module that governs painted-mesh ingest; its `[config.schema]` is the semantically-adjacent home for MMU/paint config keys.
- All 21 core-modules have `[config.schema]` sections (recon Q4); adding three entries to `mesh-segmentation.toml` is a pure-additive change consistent with the existing pattern.
- Avoids creating a new core-module (out of scope per `requirements.md` In Scope).

**Parser FACT (verified during P96 review)**: the manifest parser at `crates/slicer-scheduler/src/manifest.rs:1034` (`fn read_config_schema`) deserializes `config.schema` as a flat `BTreeMap<String, ConfigSchemaEntry>` and iterates keys at lines 1046–1050. TOML normalizes `[config.schema.X]` as `config.schema.X = { ... }` inside the surrounding table, so dotted sub-block headers and inline-table entries coexist transparently. Three other core-modules (`classic-perimeters`, `fuzzy-skin`, `rectilinear-infill`) already use the dotted `[config.schema.<key>]` form, so P96's additions follow an established pattern; the existing `"mesh_seg_mark:*"` inline entry in `mesh-segmentation.toml` remains untouched and continues to parse.

**Coordination caveat**: P5a (97) is scoped to delete the WASM mesh-segmentation module. If P5a removes `mesh-segmentation/mesh-segmentation.toml`, the three `[config.schema.mmu_segmented_region_*]` entries MUST migrate to the surviving owner of paint-segmentation host config. P5a is responsible for this migration as part of its own scope.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. The `SlicedRegion` IR field addition (TASK-246-BISECTOR) GUARANTEES guest staleness; rebuild is mandatory.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Short-circuit invariant (kernel)**: when both `mmu_segmented_region_max_width = 0` and `mmu_segmented_region_interlocking_depth = 0` (in i64 units), the kernel returns `Ok(())` immediately. Default-config slices produce byte-identical g-code via this path (AC-8).
- **Beam-flag invariant (driver-level skip, OrcaSlicer parity)**: `mmu_segmented_region_interlocking_beam = true` causes the DRIVER to skip the entire `cut_segmented_layers` call. The kernel itself does NOT take a `beam` parameter. This matches `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2452` (the call is gated on `!segmentation_interlocking_beam`). AC-7 and AC-N3 both verify this driver-level skip. The original P96 draft assumed `beam = true` produced constant-depth bands — this was incorrect.
- **Per-layer depth invariant (OrcaSlicer parity)**: `depth_for_layer = (layer_idx % 2 == 0 && interlocking_depth != 0) ? interlocking_depth : region_width`. Per `MultiMaterialSegmentation.cpp:1294`, the even-layer branch is STANDALONE `interlocking_depth`, NOT additive with `region_width`. The original P96 draft showed an additive sketch (`interlocking_depth_units + region_width_units`) — this was incorrect.
- **Negative-value invariant**: any of the depth/width keys with negative value triggers `PaintSegmentationError::InvalidPhase5Config { key, value }` at runtime (AC-N1). The config-schema declares both as `minimum = 0.0`; the runtime guards against schema validator bypass.
- **Empty-output invariant**: width larger than the smallest variant footprint correctly produces empty per-variant polygons; entries persist in `SliceIR` (D15 compatible).
- **Bisector-edge-mask additivity invariant**: the new `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` field on `SlicedRegion` defaults to `None`. For unpainted slices, the field is ALWAYS `None`. AC-10 regression (11/11 + 10/10 GREEN) confirms unpainted-shape and existing-cube-paint paths are unaffected when no bisector edges exist. Outer-wall emission in `classic-perimeters` only consults the mask when `Some(_)`; `None` behaves exactly as the pre-AC-22b code.

## Code Change Surface

- Selected approach: implement the Phase 5 kernel as a single function in a new file with synthetic-polygon unit tests; integrate at the driver level (`execute_paint_segmentation`) guarded by `!interlocking_beam`; add cube_4color integration tests with non-default config. Separately, implement the bisector-edge ownership mechanism (TASK-246-BISECTOR) as an additive IR field plus a driver tagging pass plus a classic-perimeters consumer change.

### Phase 5 Kernel + Integration

- **`crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`** (NEW, ≤ 200 LOC):
  ```rust
  use std::collections::BTreeMap;
  use slicer_ir::PaintValue;
  use crate::algos::paint_segmentation::compose_variants::ChainKey; // = Vec<(String, PaintValue)>
  use crate::polygon_ops::{offset, difference_ex, OffsetJoinType};
  use crate::algos::paint_segmentation::PaintSegmentationError;
  use geo::ExPolygon;

  // 1 unit = 100 nm = 1e-4 mm (see `docs/08_coordinate_system.md`).
  const UNITS_PER_MM: f32 = 10_000.0;
  // Offset arc tolerance for inward offset in mm. Matches existing P95-era callers.
  const OFFSET_ARC_TOLERANCE_MM: f32 = 0.01;

  /// Phase 5 kernel — width-limiting + interlocking erosion.
  ///
  /// Driver-side caller: `execute_paint_segmentation` in `mod.rs`, guarded by `!interlocking_beam`.
  /// Per-layer depth selection (OrcaSlicer parity, `MultiMaterialSegmentation.cpp:1294`):
  ///   depth = (layer_idx % 2 == 0 && interlocking_depth_units != 0)
  ///           ? interlocking_depth_units      // STANDALONE; NOT additive with region_width
  ///           : region_width_units;
  /// If depth == 0 for a layer, the layer is skipped.
  pub fn cut_segmented_layers(
      variants_per_layer: &mut [BTreeMap<ChainKey, Vec<ExPolygon>>],
      input_expolygons_per_layer: &[Vec<ExPolygon>],
      region_width_units: i64,
      interlocking_depth_units: i64,
  ) -> Result<(), PaintSegmentationError> {
      if region_width_units < 0 {
          return Err(PaintSegmentationError::InvalidPhase5Config {
              key: "mmu_segmented_region_max_width".into(),
              value: region_width_units,
          });
      }
      if interlocking_depth_units < 0 {
          return Err(PaintSegmentationError::InvalidPhase5Config {
              key: "mmu_segmented_region_interlocking_depth".into(),
              value: interlocking_depth_units,
          });
      }
      if region_width_units == 0 && interlocking_depth_units == 0 {
          return Ok(()); // kernel-level short-circuit
      }
      for (layer_idx, variants) in variants_per_layer.iter_mut().enumerate() {
          let depth_units = if layer_idx % 2 == 0 && interlocking_depth_units != 0 {
              interlocking_depth_units
          } else {
              region_width_units
          };
          if depth_units == 0 { continue; } // per-layer skip
          let layer_input = &input_expolygons_per_layer[layer_idx];
          // Inward offset = call `offset` with a NEGATIVE delta_mm. The existing API
          // `pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join, arc_tolerance) -> Vec<ExPolygon>`
          // at crates/slicer-core/src/polygon_ops.rs:195 is the only offset primitive
          // — no dedicated `offset_expolygons_inward` helper exists.
          let delta_mm = -(depth_units as f32) / UNITS_PER_MM;
          let inner = offset(layer_input, delta_mm, OffsetJoinType::Miter, OFFSET_ARC_TOLERANCE_MM);
          for (chain, expolys) in variants.iter_mut() {
              if chain.is_empty() { continue; } // base/unpainted region unchanged
              *expolys = difference_ex(expolys, &inner);
              // D15-compatible: empty result is OK; entries persist in SliceIR.
          }
      }
      Ok(())
  }
  ```
  Helper API anchors (verified during P96 review):
  - `pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join: OffsetJoinType, arc_tolerance_mm: f32) -> Vec<ExPolygon>` at `crates/slicer-core/src/polygon_ops.rs:195` — invoked with a NEGATIVE `delta_mm` to erode inward. No standalone `offset_expolygons_inward` helper exists; the kernel sketch was previously written against a fictitious name.
  - `pub fn difference_ex(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon>` at `crates/slicer-core/src/polygon_ops.rs:266` — takes slice references, not owned vectors.
  - Step 1 confirms the exact `OffsetJoinType` variant + arc-tolerance to use (the sketch uses `Miter` + `0.01 mm` as the conservative default; if existing P95 paint-segmentation callers use a different combination, mirror them).

- **`crates/slicer-core/src/algos/paint_segmentation/mod.rs`** (integration inside `pub fn execute_paint_segmentation` at line 393):

  Inserted AFTER the inlined variant-composition block end at `mod.rs:802` (the `working[i].regions = new_regions;` write) and BEFORE the final `Ok(Arc::new(working))` return at `mod.rs:999`:
  ```rust
  // ── Phase 5: width-limit + interlocking erosion ──
  // OrcaSlicer parity: when `interlocking_beam == true`, the entire kernel is
  // skipped at the driver level (matches MultiMaterialSegmentation.cpp:2452).
  // Config keys are read per-RegionKey via RegionMapIR::config_for (P1a interner).
  let region_key = /* the primary RegionKey for paint config; see Step 1 dispatch */;
  let cfg = region_map.config_for(&region_key);
  let width_units = mm_to_units(cfg.get_f32("mmu_segmented_region_max_width").unwrap_or(0.0));
  let depth_units = mm_to_units(cfg.get_f32("mmu_segmented_region_interlocking_depth").unwrap_or(0.0));
  let beam = cfg.get_bool("mmu_segmented_region_interlocking_beam").unwrap_or(false);

  if !beam {
      // Convert assembled variants from `working` back into the kernel's
      // `&mut [BTreeMap<ChainKey, Vec<ExPolygon>>]` view, then write back.
      // (Exact adapter shape resolved by Step 4a; the variant block at mod.rs:577-804
      //  already produces compatible per-layer maps internally.)
      let mut variants_per_layer = collect_variants_per_layer(&working);
      let input_expolygons_per_layer = collect_input_layer_expolygons(&slice_ir);
      cut_segmented_layers(
          &mut variants_per_layer,
          &input_expolygons_per_layer,
          width_units,
          depth_units,
      )?;
      apply_variants_back_into_working(&mut working, &variants_per_layer);
  }
  ```

- **Schema entries** in `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`:
  ```toml
  [config.schema.mmu_segmented_region_max_width]
  type = "f32"
  default = 0.0
  units = "mm"
  minimum = 0.0
  description = "Maximum width of a paint-segmented region (mm). 0 disables width limiting (OrcaSlicer parity)."

  [config.schema.mmu_segmented_region_interlocking_depth]
  type = "f32"
  default = 0.0
  units = "mm"
  minimum = 0.0
  description = "Interlocking-beam depth between adjacent layers (mm). 0 disables interlocking. When >0, even layers use this depth; odd layers fall back to mmu_segmented_region_max_width."

  [config.schema.mmu_segmented_region_interlocking_beam]
  type = "bool"
  default = false
  description = "When true, the entire Phase 5 (cut_segmented_layers) pass is SKIPPED at the driver. OrcaSlicer parity: see MultiMaterialSegmentation.cpp:2452."
  ```

- **Six kernel unit tests** in `width_limit.rs` `#[cfg(test)] mod tests`:
  - `width_limit_only_no_interlocking_erodes_to_band` (AC-1 positive a).
  - `interlocking_alternates_when_depth_nonzero` (AC-1 positive b).
  - `interlocking_depth_zero_degenerates_to_width_limit` (AC-1 positive c — replaces the misnamed "constant when beam=true" test from the original draft).
  - `width_limit_negative_rejected` (AC-N1).
  - `width_limit_oversize_yields_empty` (AC-N2).
  - `kernel_short_circuits_when_both_keys_zero` (regression-guard for AC-8).
  Synthetic input: 2–3 layers, simple `ExPolygon` squares, known expected outputs.

- **One driver-level unit test** in `paint_segmentation/mod.rs` `#[cfg(test)]`:
  - `interlocking_beam_true_skips_phase5_driver` (AC-N3) — asserts the kernel is NOT invoked when `beam = true`, even with `depth = 0.5` and `width = 2.0` in config.

- **Three integration tests** under `crates/slicer-runtime/tests/executor/`:
  - `cube_4color_phase5_width_limit_bands_tdd.rs` (NEW; AC-5).
  - `cube_4color_phase5_interlocking_alternates_tdd.rs` (NEW; AC-6).
  - `cube_4color_phase5_interlocking_beam_skips_phase5_tdd.rs` (NEW; AC-7 — asserts byte-identicality vs baseline, NOT constant-depth bands).

- **Optional `resources/cube_4color_tall.3mf`** (only if the existing cube is too short for layer-alternation visibility — likely 30 mm + tall).

### Bisector-Edge Ownership (AC-22b) Code Change Surface

> **AS-BUILT (supersedes the per-edge bool-mask surface below; deviation `D-96-AC22-EXTERNAL-CONTOUR`).**
> The drafted mechanism — a per-edge `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` consumed by the perimeter guest — was found unimplementable and was replaced. Why it failed: (1) the WASM perimeter guest cannot reconstruct the clean model boundary because boolean polygon ops (`union_ex`/`closing_ex`) are effectively no-ops in the guest; (2) Arachne's variable-width **medial-axis** walls do not map 1:1 onto original polygon edges, so a per-edge mask cannot be indexed onto the emitted wall; and (3) per-cell outer-wall tracing fragments the model perimeter across colour cells (each cell emitting its slice as a separate loop), which can never match the single-loop unpainted baseline count.
>
> **As-built mechanism:**
> 1. **IR field** — `SlicedRegion.external_contour: Option<Vec<ExPolygon>>` (`#[serde(default)]`, default `None`): the gap-free outer boundary of the region's painted cell group. Plumbed across WIT (`ir-types.wit` `external-contour`), host (`host.rs`), SDK view (`views.rs`), macro adapter (`lib.rs`), mirroring `polygons`.
> 2. **Tagging** — `bisector_ownership::populate_external_contours(&mut working, &slice_ir)`, called from `execute_paint_segmentation` after variant-composition and before Phase 5. Per object, `union_ex` of the **pre-segmentation** slice polygons (HOST-side, where boolean ops are reliable) is the clean model perimeter; it is attached to every painted cell of that object. Unpainted layers/objects → `None`.
> 3. **Consumer** — `arachne-perimeters` (active) and `classic-perimeters` group regions by object and trace the OUTER wall **once** per painted object from the shared `external_contour` (`emit_outer=true, emit_inner=false`); each cell adds only inner walls + infill (`emit_outer=false, emit_inner=true`). The shared outer wall (width `line_width`, centerline `line_width/2`) abuts each cell's first inner wall — no gap. Unpainted regions emit in full (`true, true`).
> 4. **Test counter** — the AC-22b move counter was refined to count only real extrusion segments (`G1` with `E` AND `X`/`Y`), excluding `E`-only retract/unretract moves (deviation `D-96-AC22-RETRACT-COUNTER`); symmetric across painted and unpainted. Result: 124/124 layers at exactly 4 outer-wall extrusion moves.
>
> The remainder of this subsection is the ORIGINAL draft surface, retained for historical context only.

This subsection declares the production-code surface required to drive the inherited deferred test `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` GREEN. The mechanism corresponds to TASK-246-BISECTOR (see `task-map.md`); Steps 4b and 4c in `implementation-plan.md` execute this work.

- **IR change** — `crates/slicer-ir/src/slice_ir.rs:1273` (`pub struct SlicedRegion`):
  Add a new field:
  ```rust
  /// Per-polygon, per-edge skip mask used by classic-perimeters outer-wall emission.
  /// `None` = no skipping (default; unpainted regions and pre-AC-22b code paths).
  /// `Some(mask)` = mask[poly_idx][edge_idx]: `true` means this edge is OWNED by a
  /// DIFFERENT (lower color-id) region; skip emitting it as an outer wall.
  /// Set by paint-segmentation Phase 7 → tagging stage; consumed by classic-perimeters.
  /// Default: None (preserves AC-10 regression on unpainted slices).
  #[serde(default)]
  pub bisector_edge_skip_mask: Option<Vec<Vec<bool>>>,
  ```
  Field is additive; default `None`. All existing serialized SliceIR remains forward-compatible.

- **Tagging stage** — `crates/slicer-core/src/algos/paint_segmentation/mod.rs` inside `execute_paint_segmentation`, AFTER the variant-composition block ends at `mod.rs:802` and BEFORE the Phase 5 integration block:
  ```rust
  // ── Bisector-edge ownership tagging (TASK-246-BISECTOR; AC-22b) ──
  // For each pair of adjacent variant regions sharing an edge whose geometry
  // coincides across the two polygons, the cell with the HIGHER color-id marks
  // the edge skip = true. The cell with the LOWER color-id leaves skip = false
  // (it owns the edge). Result: each shared edge is emitted by exactly ONE side
  // at outer-wall time.
  populate_bisector_edge_skip_masks(&mut working);
  ```
  The helper `populate_bisector_edge_skip_masks(working: &mut [SliceIR])` lives in a new submodule `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs`.

  **Why geometric edge-coincidence (chosen approach):** by the time the tagging stage runs (after Phase 7's inlined variant-composition block at `mod.rs:802`), the Voronoi diagram that drove cell assignment is no longer reachable — composition has already produced geometric `Vec<ExPolygon>` per region. Re-deriving the original Voronoi adjacency would either require modifying Phase 7 (rejected, OUT OF SCOPE per `requirements.md`) or re-running Voronoi (rejected, expensive duplicate work). Instead the helper identifies bisector edges directly from polygon geometry: an edge of polygon A coincides with an edge of polygon B (within `i64` integer-coordinate tolerance) iff A and B share that boundary segment, which by construction means the edge sits on a Voronoi bisector between A's and B's cells.

  **Algorithm:**
  1. For each `SliceIR` in `working` (one per layer), initialize `bisector_edge_skip_mask = Some(vec![vec![false; edges_in_poly]; polys_in_region])` for every painted region (regions whose `variant_chain` is non-empty). Unpainted regions keep `bisector_edge_skip_mask = None`.
  2. Resolve `color_id_of(region: &SlicedRegion) -> u16`: take `min` over the chain — `region.variant_chain.iter().map(|(_, paint_value)| paint_value.color_id()).min().unwrap_or(0)`. This is canonical and deterministic across multi-extruder regions. The base/unpainted region has chain-empty and is excluded from tagging (already `None`).
  3. Build an `EdgeKey` = `(p_min, p_max)` where `p_min` / `p_max` are the two `Point2<i64>` endpoints of an edge sorted lexicographically (so `(A,B)` and `(B,A)` hash to the same key). Insert into a `HashMap<EdgeKey, Vec<(region_idx, poly_idx, edge_idx, color_id)>>`. Iterate every painted region's every polygon's every edge once.
  4. For each bucket with `≥ 2` entries — these are geometric bisector edges:
     - Find `min_color = entries.iter().map(|e| e.color_id).min()`.
     - For each entry: if `entry.color_id > min_color`, set `mask[region_idx][poly_idx][edge_idx] = true`. The entry (or entries) with `color_id == min_color` keep `false` (owners).
     - Ties (two entries with the same `color_id`) cannot occur because they would mean two regions share the same `variant_chain` after Phase 7's chain-keyed grouping; if they DO occur for any reason, fall back to deterministic ordering by `region_idx`: lowest region_idx is owner.
  5. Buckets with exactly 1 entry are boundary edges (not bisectors); their `mask` stays `false`. Buckets with `≥ 3` entries (3-color corners or shared-vertex artifacts) follow the same `min_color`-wins rule — if two regions tie at min_color while a third has higher color_id, the two min-color regions both own (`false`); they will collectively emit one duplicate edge, which is acceptable (the AC-22b assertion is `±1` per-layer wall count, not exact).

  **Tolerance:** endpoints are exact `Point2<i64>` integers in the 100-nm coordinate system (no floating-point rounding). Quantization is implicit in the `i64` type. If a future change converts endpoints to floating-point, replace `EdgeKey` with `(quantize(p_min, 1_i64), quantize(p_max, 1_i64))` where `quantize` rounds to the nearest 1-unit grid (100 nm). The 1-unit grid is small enough that no real geometric divergence aliases to a coincident bucket and large enough that exact-equal output of the same compose_variants Voronoi step lands on the same key.

  **3-color corner case:** a Voronoi vertex shared by three differently-colored cells has three edges radiating from it, each between a different color pair. Each edge gets its own `EdgeKey` bucket of size 2; the algorithm resolves each independently. The shared vertex itself is just an endpoint and is not an edge — no special handling needed.

  **Complexity:** O(N_edges) hashing + O(N_buckets) resolution per layer; dominated by the polygon-edge count, which is bounded by the existing polygon-iteration cost in `classic-perimeters`.

  **Submodule docs:** `bisector_ownership.rs` documents the `EdgeKey` quantization grid (1 unit = 100 nm), the `min(color_id)` ownership rule (with 3-color and tie tie-breaks), and a worked example using a synthetic 2-cell pair as a doctest.

- **Consumer change** — `modules/core-modules/classic-perimeters/src/lib.rs`:
  - `run_perimeters` at line 85: pass through the `region.bisector_edge_skip_mask` reference.
  - Polygon consumption at line 94: keep `let polygons = region.polygons();` but add `let skip_mask = region.bisector_edge_skip_mask.as_ref();`.
  - Outer-wall loop at lines 111–118 (`i == 0` branch only — inner walls and infill are unaffected): when iterating edges at line 153, gate the emit on `!skip_mask.map_or(false, |m| m.get(poly_idx).and_then(|p| p.get(edge_idx)).copied().unwrap_or(false))`.
  - The mask is read only when present; absent (`None`) means "emit all edges" — identical to pre-AC-22b behavior.

- **Test unignore** — `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs:337`: remove the `#[ignore]` attribute. Test fn at line 338. After the mechanism above lands, the test drives GREEN.

### Rejected Alternatives

- **Make Phase 5 a separate sub-driver invoked from prepass directly**: rejected — the algorithm is internal to paint-segmentation; surfacing it as a stage would over-expose the implementation.
- **Read config keys via direct `plan.config` (pre-P1a shape)**: rejected — that shape no longer exists after P1a. Use `region_map.config_for(&region_key)`.
- **Make `interlocking_beam` an integer enum (depth-per-layer offset)**: rejected — OrcaSlicer's API is a bool; the user-facing semantics are `false = run Phase 5`, `true = skip Phase 5`. Don't over-engineer.
- **Implement bisector-edge ownership as a SliceIR-side filter without modifying classic-perimeters**: rejected — the polygon stream consumed at `classic-perimeters/src/lib.rs:94` is the natural enforcement point; pre-filtering polygons would lose per-edge granularity (a polygon can have SOME bisector edges and SOME boundary edges).
- **Use a `HashSet<(poly_idx, edge_idx)>` instead of `Option<Vec<Vec<bool>>>`**: rejected — the bool-matrix shape is dense and small (≪ KB per layer), and matches the perimeter iteration shape exactly. HashSet would require hashing per edge.
- **Track bisector-edge provenance inside compose_variants Phase 7 instead of re-deriving it geometrically**: rejected — Phase 7 is OUT OF SCOPE per `requirements.md`. Geometric edge-coincidence (chosen approach) recovers the same adjacency from the resulting polygons in O(N_edges) without invasive changes to a stable pipeline stage.
- **Re-run a Voronoi pass after Phase 7 to recover bisector adjacency**: rejected — duplicates work already done in compose_variants and adds non-trivial CPU. Geometric edge-coincidence is cheaper and uses information already present in the produced polygons.

## Files in Scope (read + edit)

**Phase 5 (TASK-246):**
- `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (NEW).
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (integration point at lines 393–999).
- `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (three `[config.schema.*]` entries added).
- 3 new integration test files under `crates/slicer-runtime/tests/executor/` (names in §"Code Change Surface").
- Optional `resources/cube_4color_tall.3mf` (≤ 100 KB) only if needed.

**Bisector-edge ownership (TASK-246-BISECTOR):**
- `crates/slicer-ir/src/slice_ir.rs` (additive field on `SlicedRegion` at line 1273).
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (tagging insertion point, same file as Phase 5 integration — separate Step).
- `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs` (NEW).
- `modules/core-modules/classic-perimeters/src/lib.rs` (consumer change in `run_perimeters` at lines 85–192).
- `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (remove `#[ignore]` at line 337).

≤ 3 files per step in implementation-plan. Steps 4b and 4c split the bisector-edge work to stay within that budget.

## Read-Only Context

- `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 (50-80 lines; range-read).
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4".
- `docs/08_coordinate_system.md` — coordinate conversion table.
- `docs/02_ir_schemas.md` §"SlicedRegion" (for the new field's doc placement; updated on closure per Doc Impact Statement).
- `crates/slicer-core/src/polygon_ops.rs` — `offset` (`:195`) + `difference_ex` (`:266`) signatures (post-P95). Inward erosion is `offset(_, delta_mm, _, _)` with a NEGATIVE `delta_mm`; no dedicated `offset_expolygons_inward` helper exists.
- `crates/slicer-core/src/algos/paint_segmentation/compose_variants.rs` — `ChainKey` typedef at line 45 (the kernel re-exports this).
- `crates/slicer-ir/src/slice_ir.rs:1230` — `RegionMapIR::config_for` signature.
- `crates/slicer-ir/src/slice_ir.rs:1273` — existing `SlicedRegion` struct shape (field-addition target).
- `modules/core-modules/classic-perimeters/src/lib.rs` — `run_perimeters` body (lines 85–192); consumer change target.
- `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` — existing `[config.schema]` blocks for syntax mirror.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated code — never load.
- The other paint_segmentation sub-modules (`phase3.rs`, `colorize.rs`, `compose_variants.rs`) — P95 territory; not edited.
- `crates/slicer-runtime/src/prepass.rs` — not edited (Phase 5 is internal to paint-segmentation).
- Binary fixtures — never `Read`.

## Expected Sub-Agent Dispatches

- "Return the EXACT signature of the inward-offset helper `offset` (at `crates/slicer-core/src/polygon_ops.rs:195`; invoked with NEGATIVE `delta_mm`) and `difference_ex` (at `:266`); FACT" — purpose: Step 2 kernel; confirms `OffsetJoinType` variant + arc-tolerance constant to mirror existing P95 paint-segmentation callers.
- "Open `crates/slicer-core/src/algos/paint_segmentation/mod.rs` lines 795–815 and lines 990–1000; return SNIPPETS — purpose: confirm the insertion point for Phase 5 (post variant-block-end at line 802 under the `if !new_regions.is_empty()` guard at line 801, pre return at line 999).
- "Confirm the `RegionKey` used to look up paint-related config in `RegionMapIR`; FACT (≤ 5 lines) showing which `region_key` constant the paint pipeline uses to call `config_for`."
- "Open `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` and return one existing `[config.schema.*]` block; SNIPPETS — purpose: syntax mirror for the three new entries."
- "Open `modules/core-modules/classic-perimeters/src/lib.rs` lines 85–160 and return SNIPPETS — purpose: identify the exact loop-index + edge-index variable names used in the outer-wall emission for AC-22b consumer change."
- "Open `crates/slicer-ir/src/slice_ir.rs` lines 1265–1310 (around `pub struct SlicedRegion`) and return SNIPPETS — purpose: confirm `#[derive(...)]` and `#[serde(...)]` attributes for the additive field placement."
- "Run `mkdir -p target && cargo test -p slicer-core paint_segmentation::width_limit 2>&1 | tee target/test-output.log`; FACT" — Step 2.
- "Run `mkdir -p target && cargo test -p slicer-core interlocking_beam_true_skips_phase5_driver 2>&1 | tee target/test-output.log`; FACT" — AC-N3 (filter has no `paint_segmentation::` prefix so it matches as a substring whether the test is at the file root or nested in `mod tests`).
- "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log`; FACT" — Steps 5.
- "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one 2>&1 | tee target/test-output.log`; FACT" — AC-22b (Step 4c).
- "Run AC-8 SHA-equality commands (wedge + cube) from `packet.spec.md` AC-8 verification; FACT" — Step 6.
- "Run `cargo xtask build-guests --check`; FACT" — Step 8.
- "Helper-vs-driver wire-in: in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` does `pub fn execute_paint_segmentation` invoke `cut_segmented_layers` on the production path? LOCATIONS." — explicit P95 W6/W8 trap guard, run during Step 4a verification.

## Data and Contract Notes

- IR contracts touched: `SlicedRegion` gains additive field `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` (default `None`). This is the only IR-shape change. Existing serialized `SliceIR` remains forward-compatible via `#[serde(default)]`. Doc Impact: `docs/02_ir_schemas.md` §"SlicedRegion" updated on closure.
- WIT boundary considerations: the `SlicedRegion` field addition propagates through guest bindgen. ALL guest WASMs are stale after this change; `cargo xtask build-guests --check` MUST be run after Step 4b (IR change) and any guest rebuild needed before Step 4c can be tested against a real `classic-perimeters` module. Schema entries in `mesh-segmentation.toml` ALSO require a rebuild of that guest. Both invalidations are covered by Step 8.
- Determinism or scheduler constraints: Phase 5's even/odd layer alternation is deterministic by layer index (same input → same output). Bisector-edge ownership is deterministic by lowest-color-id rule (no ambiguity when two cells differ in color).

## Locked Assumptions and Invariants

- **Default config → no-op**: with all three keys at declared defaults (`max_width = 0.0`, `interlocking_depth = 0.0`, `interlocking_beam = false`), Phase 5 short-circuits in the kernel (both depth/width = 0). AC-8 byte-identicality contract.
- **Driver-level beam guard (OrcaSlicer parity)**: `interlocking_beam = true` causes the DRIVER to skip the entire `cut_segmented_layers` call. The kernel does not consult the beam flag. AC-7 and AC-N3 both verify this. Per `MultiMaterialSegmentation.cpp:2452`.
- **Standalone even-layer depth (OrcaSlicer parity)**: `depth_for_layer = interlocking_depth` STANDALONE on even layers (not additive with `region_width`). Per `MultiMaterialSegmentation.cpp:1294`.
- **Negative values rejected**: schema-level (`minimum = 0.0`) + runtime defense (kernel returns `InvalidPhase5Config` error).
- **Bisector-mask additivity**: `SlicedRegion.bisector_edge_skip_mask` defaults to `None`. Unpainted slices and pre-AC-22b code paths never construct a `Some(_)` mask. AC-10 regression confirms unpainted-shape and existing-cube-paint paths unchanged.
- **Lowest-color-id ownership rule**: bisector edges between two differently-colored cells are owned by the cell with the LOWER color-id (canonical, deterministic). Documented in `bisector_ownership.rs` module-level comment on closure.

## Risks and Tradeoffs

- **Risk: Phase 5 interacts badly with downstream perimeter generation** (the variant polygons it produces have eroded inner boundaries; perimeter generator might produce duplicate / overlapping perimeters). Mitigation: AC-9's visual report check; if visual confirms banding without perimeter artifacts, packet ships.
- **Risk: integration test for "alternating bands across adjacent layers" requires careful Z-layer pair selection**. Mitigation: pick layers far from top/bottom (avoid edge effects); document the chosen layer indices in the closure log.
- **Risk: bisector-edge ownership rule mis-resolves the lower-color-id when `variant_chain` is multi-valued** (a chain like `[(extruder, 1), (extruder, 2)]` has more than one "color"). Mitigation: the resolution rule explicitly takes `min(color_id)` over the chain; documented in `bisector_ownership.rs` module-level docs.
- **Risk: AC-22b (bisector dedup) over-deletes edges** (e.g. a region's edge that does NOT touch a differently-colored neighbor gets masked). Mitigation: tagging stage only sets the mask when the geometric `EdgeKey` bucket contains ≥ 2 distinct-color entries; boundary edges with bucket size 1 keep `false` and stay emitted. The chosen algorithm is described in §"Bisector-Edge Ownership (AC-22b) Code Change Surface" → "Algorithm" with the worked 3-color-corner case.
- **Risk: geometric `EdgeKey` mis-buckets two near-coincident but not equal edges** (e.g. one came from compose_variants and the other from Phase 5 erosion). Mitigation: tagging stage runs BEFORE Phase 5 (Step 4b precedes Step 4a's call site is inside `!beam`, but Phase 5 only mutates polygons under `!beam`; tagging needs the pre-Phase-5 polygons to see true Voronoi-bisector geometry). Both edges originate from the same compose_variants Voronoi step and share exact `Point2<i64>` endpoints — no rounding drift.
- **Risk: schema landing in `mesh-segmentation.toml` becomes wrong if P5a deletes that module**. Mitigation: explicit coordination note in §"Schema Landing Site"; P5a takes responsibility for migration.
- **Tradeoff: `0.0` defaults vs. OrcaSlicer's non-zero defaults**. `0.0` is conservative and preserves byte-identical regression; users opt in via config. Register deviation `D-96-DEFAULT-ZERO` on closure (see Doc Impact Statement).
- **Tradeoff: corrected `beam = true` semantics deviate from the original P96 draft** (which assumed constant-depth bands). The new semantic matches OrcaSlicer's actual behavior verified at `MultiMaterialSegmentation.cpp:2452`. Register deviation `D-96-BEAM-FLAG-SKIPS` on closure recording the semantic correction.

## Context Cost Estimate

- Aggregate: `M`.
- Largest single step: `M` (Step 2 — kernel + 6 unit tests).
- Bisector-edge work (Steps 4b + 4c) is split across two steps to keep each ≤ M; the IR-field addition (4b) and the consumer change (4c) are independently small.
- Highest-risk dispatch: Step 4a's helper-vs-driver wire-in dispatch (explicit P95 W6/W8 trap guard).

## Open Questions

- `[RESOLVED]` — `polygon_ops.rs` exposes `offset(polygons, delta_mm, join, arc_tolerance) -> Vec<ExPolygon>` at line 195 (inward = negative `delta_mm`) and `difference_ex(subject, clip) -> Vec<ExPolygon>` at line 266. There is NO dedicated `offset_expolygons_inward` helper. Kernel sketch above uses the real `offset` signature directly; Step 2's only open question is the `OffsetJoinType` variant + arc-tolerance constant to mirror existing P95 callers (sketch defaults to `Miter` + `0.01 mm`).
- `[FWD]` — Step 4a dispatch resolves the EXACT `RegionKey` used by paint config (which key to pass to `region_map.config_for(...)` for MMU lookups).
- `[FWD]` — Step 5 dispatch confirms whether `cube_4color.3mf` is tall enough for layer-alternation visibility; if not, author `cube_4color_tall.3mf` (≤ 100 KB).
- `[FWD]` — Step 4b dispatch resolves the EXACT serde derive macros / field ordering convention used by `SlicedRegion` (for the additive field placement).
- `[BLOCK]` — None.

(Note: the original P96 draft's open question about the schema landing site is RESOLVED to `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` in §"Schema Landing Site" above. The fictitious `paint-segmentation-default` module alternative is removed.)
