# Design: 247-visual-debug-silhouette-core

## Controlling Code Paths

- Primary code path: `validate_request` → `run_visual_debug` → `run_model_source` (`crates/pnp-cli/src/visual_debug.rs`) → `execute_blackboard_taps` (`crates/slicer-runtime/src/layer_executor.rs`) → **new** `render_silhouette_composite` (`crates/slicer-runtime/src/visual_debug_render.rs`) → `Canvas::fill_polygon` via `Projector`.
- Neighboring tests/fixtures: `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` (library-call validation harness, `unreachable_model_request` pattern), `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` (CLI manifest-shape harness incl. `ac_manifest_serializes_required_index_and_entry_fields`), `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` (wedge-model end-to-end pattern: `wedge_path`/`module_dir`/`write_bounded_config` helpers), `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs` (direct `StageCapture` fixtures + `decode_rgb` PNG decoding), `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` (`seeded_support_geometry_and_plan` fixture pattern).
- OrcaSlicer comparison: none — this is a PnP-native tool; no parity obligations.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Projector single-owner rule (archived spec, binding): the silhouette path feeds `Projector::project(x_or_y_mm, z_mm)` and never defines its own world→pixel transform. `Projector`'s built-in y-flip makes larger Z render toward the top — correct orientation for free.
- Z is mm floats end-to-end (`docs/08_coordinate_system.md`) and must never round-trip through `mm_to_units`; polygon X/Y is read via `Point2::to_mm` only.
- Version-locking (mandatory pattern from the template): 1.0.0/1.1.0 manifests must stay byte-identical. `legend_version_for` (`crates/pnp-cli/src/visual_debug.rs`) already pins 1.0.0 to the literal `"1.0.0"`; this packet extends `schema_supported` without touching either existing branch's output, and pins byte-compat with serialization tests (AC-8), not just parsing tests. `LEGEND_VERSION` is deliberately not bumped (fills, not glyphs).
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): new test literals of watched types (`SliceIR`, `SlicedRegion`, `SupportPlanEntry`, `SupportPlanIR`, `VisualDebugRequest`, …) need `..` FRU or an `// exhaustive: <reason>` waiver; `SupportPlanEntry` has no `Default` — existing suites use the exhaustive waiver, follow them.

## Code Change Surface

- Selected approach: silhouette is a fourth visualization kind validated schema-aware in `validate_request`; the composite path branches inside `run_model_source` **before** the existing per-capture render loop (a silhouette request never enters that loop — the mixing ban guarantees the two paths are disjoint per bundle); all pixel math lives in one new renderer entry point that reuses the private `Canvas`/`Shape`/`draw_shapes` machinery in-module.

- Exact surface, per file:
  - `crates/pnp-cli/src/visual_debug.rs`
    - `const VERSION_1_2: &str = "1.2.0"`; `schema_supported` accepts it; `is_v1_1` generalizes to a `strict_options` predicate true for 1.1.0 and 1.2.0; `effective_visualization_options` uses the strict typed parse for both.
    - `VisualizationOptions` gains `#[serde(default)] pub view: Option<String>` (deny_unknown_fields already rejects `composited_overlays` — AC-N7 needs no code, only a pinning test).
    - `ValidationError` gains: `SilhouetteRequiresSchema12`, `SilhouetteMixedWithOtherKinds { other_kind: String }`, `SilhouettePlateFrameUnsupported`, `InvalidSilhouetteView { message: String }`, `SilhouetteUnsupportedForTap { tap: String, reason: String }`, `SilhouetteUnsupportedOnGcodeSource` — plus their `Display` arms (each message names the fix or the reason, `OptionRequiresSchema11` style).
    - `validate_request`: kind whitelist grows to accept `"silhouette"` only under 1.2.0 (else `SilhouetteRequiresSchema12`); mixing ban; one-view-per-bundle; view value/kind checks; `frame == FrameMode::Plate` + silhouette → reject; per-tap silhouette whitelist check (`SILHOUETTE_TAP_STAGE_IDS`, a new module-local const: the four `CapturedIr::Slice` taps + `PrePass::SupportGeometry`) with per-class reasons; gcode source + silhouette → reject; `color_by: "tool"` on silhouette → `InvalidColorBy`.
    - `ImageEntry`: `layer_index: Option<i64>` + `skip_serializing_if = "Option::is_none"`; `layer_z: Option<Option<f64>>` + `skip_serializing_if = "Option::is_none"` (tri-state: `Some(Some(z))` → number, `Some(None)` → `null`, `None` → absent); new `pub view: Option<String>` and `pub layers_rendered: Option<Vec<LayerRangeEntry>>`, both `skip_serializing_if`; new `pub struct LayerRangeEntry { pub start: i64, pub end: i64 }` (Serialize). Blast radius: exactly four `ImageEntry` literal sites, all in this file (the `typed_ir` no-visualization arm, the isolated-overlay arm, the geometry arm, and the gcode arm) — wrap existing values `Some(...)`/`Some(Some(...))`; no test constructs `ImageEntry` as a Rust literal (verified: only JSON-value assertions).
    - `run_model_source`: silhouette branch — resolve the bundle view once; build the slab schedule from `ctx.blackboard.layer_plan()` (`z_bottom` = previous `GlobalLayer.z`, `0.0` for index 0); compute the silhouette viewport (below); group `output.captures` by tap (view is bundle-wide); call `render_silhouette_composite` per group; emit one `ImageEntry` per group (`view`, `layers_rendered` from the group's capture layer indices compressed to maximal ranges, `warnings` from the renderer, `layer_index: None`, `layer_z: None`, `typed_capture: None`, shared `world_bounds_mm`); filenames `format!("{}_silhouette_{}.png", sanitize_path_component(tap), view.name())`; groups ordered by `STAGE_ORDER` position then tap string.
  - `crates/slicer-runtime/src/visual_debug_render.rs`
    - `pub enum SilhouetteView { Front, Side }` with `pub fn name(self) -> &'static str` (`"front"`/`"side"`) and `pub fn parse(&str) -> Option<Self>`.
    - `pub struct SilhouetteScheduleSlab { pub index: u32, pub z_bottom: f32, pub z_top: f32 }` and `pub struct SilhouetteSlabSchedule { pub slabs: Vec<SilhouetteScheduleSlab> }` (caller-built; the renderer never reads a Blackboard).
    - `pub fn compute_silhouette_viewport_bounds(captures: &[StageCapture], view: SilhouetteView, schedule: &SilhouetteSlabSchedule, model_extent: Option<ViewportBoundsMm>) -> ViewportBoundsMm` — horizontal = per-view axis of `geometry_points_mm` unioned with `model_extent`'s horizontal; vertical = slab z-range unioned with `model_extent`'s vertical; `.with_margin()`. `model_extent` carries plane semantics (min_x/max_x = X-or-Y extent, min_y/max_y = Z extent), built by pnp-cli from `MeshIR::build_volume`.
    - `pub fn render_silhouette_composite(captures: &[StageCapture], view: SilhouetteView, resolution_scale: u32, viewport: ViewportBoundsMm, schedule: &SilhouetteSlabSchedule) -> Result<(RenderedImage, Vec<String>), RenderError>` — scale check as `render_stage_capture_styled`; per capture (already sorted ascending by layer): extract (class, interval, slab) triples; union intervals per (layer, class); emit rectangles ascending layer → fixed class order → ascending interval start; draw via `Shape::Fill` + `draw_shapes`; fail closed `RenderError::MissingGeometryField` when zero rectangles exist across the whole group; return warnings (W1, W2, occlusion — fixed order, deduped per group).
    - Class extraction: `CapturedIr::Slice` → one body class from `SliceIR.regions[].polygons` (contour min/max via `Point2::to_mm`; `infill_areas` deliberately not a class — decision below), slab per region `[capture.layer_z − region.effective_layer_height, capture.layer_z]`. `CapturedIr::SupportGeometry` → classes per `SupportPlanRole` from `entry.roles[].regions` for entries with `global_layer_index >= 0` matching the capture's layer; slab from `schedule` (the capture's layer index); raft entries counted for W1 from any one capture's whole-plan payload; `geometry.entries` counted for W2.
    - Paint order (D2, pinned): body classes first — `SLICE_REGION` (Slice family) / `SUPPORT` (`SupportBody`) — then `SUPPORT_RAFT` (`RaftRelated`), `SUPPORT_BASE_INTERFACE` (`BaseInterface`), `SUPPORT_BOTTOM_INTERFACE` (`BottomInterface`), `SUPPORT_INTERFACE` (`TopInterface`) last. New `palette` constants: `SUPPORT_BASE_INTERFACE`, `SUPPORT_BOTTOM_INTERFACE`, `SUPPORT_RAFT` — RGB values chosen at implementation, pairwise distinct from each other, from `SUPPORT`/`SUPPORT_INTERFACE`/`SLICE_REGION`, and from `BACKGROUND`.
    - Interval union: sorted endpoint sweep over `(f32, f32)` intervals, merging when `next.start <= current.end` (touch merges; exact comparison, no epsilon).
  - `crates/slicer-runtime/src/lib.rs` — re-export `SilhouetteView`, `SilhouetteScheduleSlab`, `SilhouetteSlabSchedule`, `render_silhouette_composite`, `compute_silhouette_viewport_bounds`, and `union_silhouette_intervals` alongside the existing `visual_debug_render` re-exports. (The sixth name, `union_silhouette_intervals`, was added during implementation: AC-3 mandates a touch-merge binding check that decoded pixels cannot witness, since two abutting rectangles rasterize identically to one merged rectangle, and a test binary can only reach the public API. Recorded as deviation D-1 in `packet.spec.md`.)
  - Tests: new `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`, new `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs`, edits to `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` and `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`.
  - Docs: `docs/19_visual_debug.md`, `docs/02_ir_schemas.md` (IR 9a wording), `docs/DEVIATION_LOG.md` (raft row).

- Rejected alternatives and reasons:
  - Per-entry `projection` field / re-scoped bounds instead of the mixing ban — rewrites the pinned `world_bounds_mm` byte-identity contract and every consumer assertion (plan D6).
  - Plain `#[serde(skip_serializing_if)]` on `layer_z: Option<f64>` (the plan's literal D7 text) — **falsified during grounding**: `visual_debug_gcode::ParsedLayer.layer_z` is `None` for a layer with no `;Z:` comment (`ensure_layer` constructs it `None`), and such entries serialize `"layer_z": null` today, so a plain skip would flip `null` → absent on existing 1.0/1.1 gcode bundles. The tri-state `Option<Option<f64>>` achieves the plan's stated outcome (absent on silhouette entries) while byte-preserving the `null`.
  - Uniform per-layer slabs from consecutive `GlobalLayer.z` for Slice-family taps — lies on catch-up layers (plan D1; `GlobalLayer` carries no height field — verified).
  - Per-region heights for the support tap's slabs — support columns span air where no `ActiveRegion` is active, and per-region heights disagree across objects on a shared layer; the schedule z-diff is the only height a plan entry can honestly attest (mirrors the plan's D8 slab-source note for postpass taps).
  - Rendering both views by default, or allowing front+side in one bundle — one visualization spec producing two images breaks the request-to-entry mapping (plan D5), and two planes in one bundle breaks the byte-identical `world_bounds_mm` invariant; two bundles cover the workflow.
  - Min-1px inflation of sub-pixel bands — the image lies about Z geometry (plan D4); docs guidance on `resolution_scale` instead.
  - A new transform for (x_or_y, z) — violates the Projector single-owner rule the archived spec records both prior paths drifting on.

### Delegated decisions (plan §10 items 9 and the queue directive), resolved here

1. **`Layer::Slice` classes — `polygons` only (plan D8 default adopted).** `infill_areas` lie inside the region's outer contour, so their projection interval is a subset of the `polygons` interval: a second class adds zero silhouette information while introducing an occlusion pairing that could hide real body extent. The top-down view already distinguishes them.
2. **The five extra D8 SliceIR-family taps.** The three whose capture payload is literally `CapturedIr::Slice` (`PrePass::PaintSegmentation`, `Layer::PaintRegionAnnotation`, `Layer::SlicePostProcess`) are **included** — they cost one match-arm membership, nothing else. `PrePass::RegionMapping` (needs the render-time join + dynamic `config_tint` classes with a determinism story for class ordering) and `PrePass::OverhangAnnotation` (its `CapturedIr::SurfaceClassification` payload carries **no** per-region height source — an honest slab needs a capture-shape change) are **rejected** in 247 with `SilhouetteUnsupportedForTap` ("not yet supported for silhouette") and flagged `[FWD]` below for explicit assignment.
3. **W2 wording — producer-verified.** The in-tree contradiction (plan fact 4 caveat) resolves in favor of the field's own doc comment: `execute_support_geometry`/`build_emit_schedule` (`crates/slicer-core/src/algos/support_geometry.rs`) key `SupportGeometryKey.global_support_layer_index` with `global_layer.index` — a **model-layer (global) index** — emitting only at layers where accumulated per-object height crosses `support_layer_height_mm`, plus a `u32::MAX` sentinel bucket for intermediate model-resolution layers. `docs/02_ir_schemas.md` IR 9a's "keyed by support-layer index" is the imprecise side (corrected in this packet). W2's honest skip reason is therefore **not** "own grid" but: each emitted entry aggregates polygons across the model-layer span since the previous emit layer (and the sentinel bucket has no layer at all), so drawing them on a single model-layer slab would misstate their vertical extent. W2 text: `support geometry: {n} coarse SupportGeometryIR entries skipped — emit-schedule entries span multiple model layers (u32::MAX sentinel = intermediate layers) and cannot be honestly drawn on single-layer slabs; inspect them via the top-down view`.
4. **`layers_rendered` encoding (plan D7 defers):** `Vec<LayerRangeEntry { start: i64, end: i64 }>` — maximal runs of consecutive rendered indices, ascending, non-overlapping, inclusive; matches `LayerSelector::Range`'s field names; lossless by construction.
5. **Occlusion caveat placement (plan §10 item 2):** in `docs/19_visual_debug.md` **and**, when occlusion actually occurs (a later class's union interval overlaps an earlier class's on the same layer), as a deterministic per-entry manifest warning naming the affected layer count — the manifest is the PNG-reading agent's first read.

## Files in Scope (read + edit)

- `crates/pnp-cli/src/visual_debug.rs` — validation, manifest shape, composite assembly; the packet's largest edit.
- `crates/slicer-runtime/src/visual_debug_render.rs` — composite renderer, viewport helper, palette constants.
- `crates/slicer-runtime/src/lib.rs` — re-exports only (one small block).
- Tests (per-step, listed in `implementation-plan.md`): `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`, `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`, new `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`, new `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs`.
- Docs: `docs/19_visual_debug.md`, `docs/02_ir_schemas.md` (one sentence), `docs/DEVIATION_LOG.md` (one row).

Justification for exceeding three primaries: the packet spans a CLI crate, a runtime crate, and their pinning tests by design (the plan's §9 scope names exactly these files); each step stays ≤3 edits.

## Read-Only Context

- `crates/slicer-runtime/src/layer_executor.rs` — `execute_blackboard_taps`, `CapturedIr`, `StageCapture`, `BLACKBOARD_TAP_STAGE_IDS` region only — purpose: capture shapes; never edited.
- `crates/slicer-ir/src/slice_ir.rs` — `SliceIR`/`SlicedRegion`/`GlobalLayer`/`SupportPlanIR`/`SupportPlanEntry`/`SupportPlanRole`/`SupportGeometryIR`/`BoundingBox3` definitions only — purpose: field shapes; never edited.
- `crates/pnp-cli/src/visual_debug_gcode.rs` — `ParsedLayer.layer_z` construction only — purpose: the AC-8 null case; never edited.
- `crates/slicer-core/src/algos/support_geometry.rs` — `build_emit_schedule`/`execute_support_geometry` only — purpose: W2 wording evidence; never edited.
- `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` — fixture helpers (`wedge_path`, `module_dir`, `write_bounded_config`) only — purpose: copy the established wedge harness into the new bundle test.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not applicable (no parity), never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `modules/core-modules/**` sources — not part of this change; guest artifacts matter only as prebuilt test dependencies (`cargo xtask build-guests --check` before blaming Step-5 failures).
- The other queue rows' future packet dirs (248–251) — do not create or reference files there.
- `docs/07_implementation_status.md` — worker-dispatch updates only at the completion gate, never a full read.

## Expected Sub-Agent Dispatches

- Question: does `cargo xtask check-literals` pass after the new test fixtures land, and if not which literal/type trips it?; scope: repo root command; return: `FACT` (exit code + offending file list ≤5); purpose: Steps 3–5.
- Question: run the packet's per-step test command and report pass/fail with failing test names; scope: the step's single `cargo test -p <crate> --test <file>` invocation tee'd to `target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure; purpose: every step's verification.
- Question: current highest `DEV-###` in `docs/DEVIATION_LOG.md` (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`); scope: that file; return: `FACT`; purpose: Step 6 allocates the next free ID at write time (ledger fact — never trust a number frozen in this packet).
- Question: does any doc besides `docs/02_ir_schemas.md` repeat the "support-layer index" wording for `SupportGeometryKey`?; scope: `docs/`; return: `LOCATIONS ≤10`; purpose: Step 6 (fix only the IR 9a sentence; report others, do not mass-edit).

## Data and Contract Notes

- IR/manifest contracts: the manifest mirrors the request's declared `schema_version` (existing comment in `run_visual_debug`); 1.2.0 entries may carry `view`/`layers_rendered` and omit `layer_index`/`layer_z`; 1.0/1.1 output is byte-frozen (AC-8). `world_bounds_mm` reuses `ViewportBoundsMm` — inside a silhouette bundle `min_y`/`max_y` carry Z millimeters; legality rests on the mixing ban + one-view-per-bundle + the per-entry `view` field (plan §10 item 1, confirmed).
- WIT boundary: none — no WIT, IR struct, or guest-facing type changes; `ImageEntry` is a Serialize-only CLI type.
- Determinism/scheduler constraints: captures arrive sorted (`STAGE_ORDER` position, then layer) from `execute_blackboard_taps`; rectangle emission is ascending layer → fixed class order → ascending interval start; warnings order W1, W2, occlusion; group order `STAGE_ORDER` position then tap; all sources are `Vec`s (no `HashMap` iteration reaches the silhouette path — `SupportPlanIR.entries` is a `Vec`; entries within a layer sort by `(object_id, region_id)` like `support_geometry_shapes`).

## Locked Assumptions and Invariants

- One silhouette plane per bundle; every silhouette entry in a bundle shares one byte-identical `world_bounds_mm` (extends the pinned fact-6 invariant unchanged for 1.0/1.1 consumers).
- Slabs are `[z − effective_layer_height, z]` per region for `CapturedIr::Slice` taps; `[previous global z (0.0 for the first), z]` for the support tap. `GlobalLayer.z` is the layer **top**, and on a catch-up region the height reaches the catch-up bottom (layer-planner pinned: `effective_layer_height == z − catchup_z_bottom` — an `ActiveRegion`-level invariant; `SlicedRegion` carries only `effective_layer_height`, no catch-up flags, so the renderer needs nothing beyond that one field).
- The silhouette never draws raft entries, coarse `SupportGeometryIR.entries`, sub-pixel inflation, or inferred geometry; every omission is a named warning or a named rejection.
- `SILHOUETTE_TAP_STAGE_IDS` = the four `CapturedIr::Slice` taps + `PrePass::SupportGeometry`; later packets extend it rather than bypassing validation.

## Risks and Tradeoffs

- `execute_blackboard_taps` clones the whole `SupportGeometryIR`+`SupportPlanIR` composite per selected layer (pre-existing behavior); an all-layers support silhouette multiplies that clone by layer count. Accepted for 247 (plans are far smaller than the postpass whole-print IR that motivated D10); if profiling ever shows it matters, a dedup shaped like D10 is the follow-up — do not fix it speculatively here.
- The occlusion warning fires only when overlap actually occurs; a reader of an overlap-free bundle relies on the docs/19 caveat alone. Accepted: an always-on warning would train agents to ignore warnings.
- f32 interval endpoints: exact-comparison unions may leave a sub-pixel seam between two regions that abut at nearly-but-not-exactly equal coordinates; slabs tile exactly by construction (same `layer_z` source), so only horizontal seams from IR-level near-touches can occur, and they render honestly (the gap exists in the IR).
- AC-8 freezes `"layer_z": null` for markerless gcode layers; if a future packet wants absent-when-unknown it must version-gate it.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5 — bundle assembly + end-to-end tests over the wedge pipeline)
- Highest-risk dispatch and required return format: the Step-5 end-to-end test run (needs fresh guest WASMs; `FACT pass/fail` + `cargo xtask build-guests --check` exit code first).

## Open Questions

- `[FWD to the batch orchestrator / queue owner]` `PrePass::RegionMapping` and `PrePass::OverhangAnnotation` silhouettes are rejected here and owned by **no** queue row. RegionMapping is mechanically ready (its capture retains `slice_ir` for slabs; needs a deterministic class-order rule for `config_tint` classes). OverhangAnnotation needs a capture-shape decision (its capture carries no per-region heights). Recommend: assign both to a small follow-up packet or absorb RegionMapping into row #3; do not leave them permanently rejected.
- `[FWD to packet 248]` AC-N6's `SilhouetteUnsupportedOnGcodeSource` variant and its pinning test `silhouette_on_gcode_source_rejected_interim` are yours to remove/retarget when the gcode source lands.
- `[FWD to packet 249]` AC-N8 pins `color_by: "tool"` + silhouette → `InvalidColorBy` at validation. When LayerFinalization/GCodeEmit silhouettes land, replace the blanket validation rejection with the per-capture `ToolColorUnavailable` contract (plan R6/D17) and retarget `silhouette_tool_coloring_rejected_role_accepted`.
- `[FWD to packet 251]` AC-N7 relies on `deny_unknown_fields` rejecting `composited_overlays`; when you add the field to `VisualizationOptions`, keep a named validation error for non-silhouette kinds/gcode sources (plan R9) and retarget `composited_overlays_not_accepted_by_247`.
- `[FWD to packets 248–250]` Exports you consume: `SilhouetteView`, `SilhouetteSlabSchedule`/`SilhouetteScheduleSlab`, `render_silhouette_composite(captures, view, resolution_scale, viewport, schedule) -> Result<(RenderedImage, Vec<String>), RenderError>`, `compute_silhouette_viewport_bounds`, manifest fields `view` + `layers_rendered` (`LayerRangeEntry {start, end}`), filename scheme `{sanitized_tap}_silhouette_{view}.png` (insert `_tool` before `.png` for tool-colored variants, `_overlay_{kind}` for isolated overlays, mirroring the existing scheme).
- No `[BLOCK]` items.
