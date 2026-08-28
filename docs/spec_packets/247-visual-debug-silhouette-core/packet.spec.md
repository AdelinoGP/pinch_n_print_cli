---
status: implemented
packet: 247-visual-debug-silhouette-core
task_ids:
  - TASK-442
  - TASK-443
  - TASK-444
  - TASK-445
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
plan_source: docs/specs/visual-debug-silhouette-side-views-plan.md (Packet Queue row #1)
---

# Packet Contract: 247-visual-debug-silhouette-core

## Goal

Add the `silhouette` visualization kind (schema 1.2.0) to `pnp_cli visual-debug`: schema gate and full fail-closed validation matrix, one composite X–Z/Y–Z image per (tap, view) rendered through the existing `Projector`/`Canvas` via exact interval projection, model-wide Z framing, the 1.2.0 manifest shape (`view`, `layers_rendered`, optional `layer_index`/`layer_z`), and the first two tap families — the `CapturedIr::Slice` taps and `PrePass::SupportGeometry` (`SupportPlanIR` roles with raft/coarse warnings).

## Scope Boundaries

Model source only; the `CapturedIr::Slice`-payload taps (`Layer::Slice`, `PrePass::PaintSegmentation`, `Layer::PaintRegionAnnotation`, `Layer::SlicePostProcess`) plus `PrePass::SupportGeometry`. Everything else in the plan's D8 table — gcode source, `PostPass::*`, `PrePass::RegionMapping`, `PrePass::OverhangAnnotation`, multicolor, seam overlays — is rejected fail-closed here and owned by queue rows #2–#5 (packets 248–251). Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: nothing (queue row #1; plan approved 2026-08-27).
- Unblocks: packets 248 (gcode-source silhouette), 249 (postpass + multicolor), 250 (GCodeEmit), 251 (seam overlays).
- Activation blockers: none known.

## Acceptance Criteria

- **AC-1. Given** a `schema_version: "1.2.0"` model-source request over `resources/regression_wedge.stl` with tap `Layer::Slice`, a layer range, and one `{"type": "silhouette"}` visualization (no `options.view`), **when** `run_visual_debug` succeeds, **then** the bundle contains exactly one silhouette PNG named `images/Layer__Slice_silhouette_front.png`, and its manifest entry has `"visualization": "silhouette"`, `"view": "front"`, a `"layers_rendered"` list of inclusive `{"start", "end"}` ranges covering exactly the resolved layer indices, a `"world_bounds_mm"` object, and **no** `"layer_index"` or `"layer_z"` key. | `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd -- silhouette_bundle_entry_shape_and_default_front_view 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a `CapturedIr::Slice` capture at layer top `z` whose `SliceIR.regions` contains two `SlicedRegion`s with distinct `effective_layer_height` values — one normal, one catch-up-sized (its `effective_layer_height` reaches down past the previous layer's top, the shape the layer planner produces for a catch-up region; `SlicedRegion` itself carries only `effective_layer_height`, the catch-up flags live on `ActiveRegion`), **when** `render_silhouette_composite` renders it, **then** each region's rectangle bottom row corresponds to its own `z − effective_layer_height` — the catch-up-sized region's bottom landing strictly below the other's, never merged to one uniform slab — verified by decoded-pixel assertions at the two distinct slab bottoms. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- region_slab_bottoms_follow_effective_layer_height 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** slice regions forming (a) one contour with a hole, (b) two disjoint islands, and (c) two touching-interval islands, **when** rendered as a silhouette, **then** (a) yields one unbroken horizontal run (holes never split a projection interval), (b) yields two runs separated by background, and (c) yields one merged run. Cases (a) and (b) are asserted on decoded pixels; case (c)'s merge is witnessed by a direct assertion on the interval-union helper (`union_silhouette_intervals`, `crates/slicer-runtime/src/visual_debug_render.rs`), because two abutting rectangles rasterize identically to one merged rectangle and decoded pixels therefore cannot distinguish the merged case from the unmerged one. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- interval_union_holes_islands_and_touching_merge 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a `CapturedIr::SupportGeometry` capture whose `SupportPlanIR` entry carries `SupportPlanRole::SupportBody` and `SupportPlanRole::TopInterface` role regions overlapping in X on one layer, **when** rendered, **then** the overlap paints `palette::SUPPORT_INTERFACE` (interface classes paint after body per the fixed class order), the non-overlapping body run paints `palette::SUPPORT`, and the entry's warnings contain one occlusion warning naming the affected layer count. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- support_role_paint_order_and_occlusion_warning 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a `SupportPlanIR` with two entries at `global_layer_index: -2` and `-1` and a `SupportGeometryIR` with three coarse `entries`, **when** rendered as a silhouette, **then** the returned warnings contain (a) one raft warning naming the count `2` and the dropped index range `-2..-1` and (b) one coarse-entry warning naming the count `3` and stating the entries are skipped, and no raft or coarse geometry contributes pixels. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- raft_and_coarse_entries_skip_with_named_warnings 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** the same capture group, view, scale, slab schedule, and viewport, **when** `render_silhouette_composite` runs twice, **then** the two `RenderedImage.png_bytes` are byte-identical and the two warning lists are equal element-for-element. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- silhouette_composite_is_deterministic 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** two 1.2.0 silhouette requests over the same wedge model, one selecting a layer subset and one selecting all layers, **when** both bundles render, **then** both silhouette entries record byte-identical `world_bounds_mm` (the Z frame is model-wide from `MeshIR::build_volume`, never selection-wide). | `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd -- z_frame_is_model_wide_not_selection_wide 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** a `schema_version: "1.0.0"` gcode-source request whose fixture layer has **no** `;Z:` marker, **when** the bundle renders after this packet's `ImageEntry` change, **then** the manifest entry still serializes the key `"layer_index"` with its integer value and the key `"layer_z"` with JSON `null` (byte-compatible 1.0/1.1 serialization output, pinned on serialization, not parsing). | `cargo test -p pnp-cli --test visual_debug_request_bundle_tdd -- legacy_entries_keep_layer_index_and_null_layer_z_serialization 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** a 1.2.0 request with two identical silhouette specs and taps `Layer::Slice` + `Layer::SlicePostProcess` (two always-available `CapturedIr::Slice` taps — the wedge fixture commits no support plan, so end-to-end support coverage stays at the renderer level per AC-4/AC-5), **when** the bundle renders, **then** exactly two silhouette images exist (one per tap; duplicate specs collapse into one (tap, view) group) and no two manifest entries share a `png_path`. | `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd -- one_image_per_tap_view_group_and_unique_filenames 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10 (docs).** `docs/19_visual_debug.md` gains a silhouette section carrying the D4 scale guidance verbatim as the phrase `for interface-band inspection on tall models, raise` (followed by `resolution_scale`; this exact phrase does not exist in the doc today, so the grep fails until the guidance is written) and the paint-order occlusion caveat (D2), and `docs/02_ir_schemas.md` IR 9a states the key is a model-layer index on the emit schedule. | `rg -q 'silhouette' docs/19_visual_debug.md && rg -qi 'occlu' docs/19_visual_debug.md && rg -q 'for interface-band inspection on tall models, raise' docs/19_visual_debug.md && rg -q 'model-layer' docs/02_ir_schemas.md && echo PASS`
- **AC-11 (deviation row).** `docs/DEVIATION_LOG.md` gains an open row (ID re-derived at write time, next free `DEV-###`) tracking raft side-view rendering as the follow-up W1 names. | `rg -q 'raft side' docs/DEVIATION_LOG.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a request declaring `schema_version: "1.1.0"` (and separately `"1.0.0"`) with a `silhouette` visualization, **when** validated, **then** it is rejected with `ValidationError::SilhouetteRequiresSchema12` whose Display names `"1.2.0"` — not `UnknownVisualizationKind` — and no bundle is written. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_under_pre_1_2_schema_names_the_required_version 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a 1.2.0 request mixing one `silhouette` and one `filled_areas` visualization, **when** validated, **then** it is rejected with `ValidationError::SilhouetteMixedWithOtherKinds` naming `filled_areas`, and no bundle is written. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_mixing_with_topdown_kinds_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** a 1.2.0 silhouette request with `frame: "plate"`, **when** validated, **then** it is rejected with `ValidationError::SilhouettePlateFrameUnsupported`. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_plate_frame_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** (a) a silhouette spec with `options.view: "top"`, (b) a `filled_areas` spec with `options.view: "front"` under 1.2.0, and (c) a `filled_areas` spec carrying an explicit `options.view` key under declared `"1.0.0"` and `"1.1.0"`, **when** validated, **then** all are rejected with `ValidationError::InvalidSilhouetteView` (unknown value; view on a non-silhouette kind; view requires 1.2.0 — the message for (c) names `"1.2.0"`, never a silent stray-key tolerance). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_view_unknown_value_and_wrong_kind_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N5. Given** a 1.2.0 silhouette request naming each unsupported tap in turn — `Layer::Perimeters` (arena), `PrePass::MeshAnalysis`, `PrePass::SeamPlanning`, `PrePass::RegionMapping`, `PrePass::OverhangAnnotation`, `PostPass::LayerFinalization`, `PostPass::GCodeEmit` — **when** validated, **then** each is rejected with `ValidationError::SilhouetteUnsupportedForTap` carrying that tap name and a non-empty reason. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_unsupported_taps_rejected_with_reasons 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N6. Given** a 1.2.0 gcode-source request with a silhouette visualization, **when** validated, **then** it is rejected with `ValidationError::SilhouetteUnsupportedOnGcodeSource` (interim; packet 248 lifts it). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_on_gcode_source_rejected_interim 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N7. Given** a 1.2.0 silhouette spec carrying `options.composited_overlays: ["seams"]`, **when** validated, **then** it is rejected with `ValidationError::InvalidVisualizationOptions` (the strict typed parse's `deny_unknown_fields`; packet 251 introduces the option). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- composited_overlays_not_accepted_by_247 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N8. Given** a 1.2.0 silhouette spec with `options.color_by: "tool"`, **when** validated, **then** it is rejected with `ValidationError::InvalidColorBy` (no 247-supported silhouette tap carries a tool assignment; packet 249 relaxes this for tool-carrying captures), while `color_by: "role"` on a silhouette is accepted. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_tool_coloring_rejected_role_accepted 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N9. Given** a 1.2.0 request with two silhouette specs resolving to different views (`"front"` and `"side"`), **when** validated, **then** it is rejected with `ValidationError::InvalidSilhouetteView` stating one silhouette plane per bundle (preserves the pinned bundle-wide `world_bounds_mm` byte-identity). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- one_silhouette_plane_per_bundle 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N10. Given** a `schema_version: "1.2.0"` request naming an unknown visualization kind, **when** validated, **then** it is still rejected with `ValidationError::UnknownVisualizationKind` (1.2.0 loosens nothing else). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- unknown_kind_still_rejected_under_1_2 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Deviations

Six deviations from `design.md` shipped in Steps 1–6. All are recorded here as accepted; none was silent.

| # | What deviated | From which `design.md` statement | Why accepted |
| --- | --- | --- | --- |
| D-1 | `union_silhouette_intervals` is `pub` and re-exported from `crates/slicer-runtime/src/lib.rs` — a sixth item beyond `design.md`'s five-item re-export list. | The re-export list in `design.md` names exactly five items. | Interval merging has no pixel-level witness: two abutting rectangles rasterize identically to one merged rectangle. AC-3's mandated touch-merge binding check needed a direct assertion target. **Consequence:** the crate's public API is one item wider than the design declared. |
| D-2 | AC-3's touching-merge case (c) is proven by a helper unit assertion on `union_silhouette_intervals`, not by decoded pixels. | `design.md` (and AC-3 as originally written) required all three cases asserted on decoded pixels. | Same root cause as D-1 — pixels cannot distinguish merged from abutting-unmerged. AC-3's assertion tail was repaired to state this truthfully; cases (a) and (b) remain pixel-asserted and nothing was weakened. |
| D-3 | W2's warning text uses an ASCII hyphen where `design.md` pins an em dash. | `design.md`'s verbatim W2 string. | Matches the surrounding file's ASCII convention. The AC-5-asserted substring `coarse SupportGeometryIR entries skipped` does not straddle the punctuation and is intact. |
| D-4 | AC-5's fixtures carry one extra non-negative body entry beyond the design's fixture description. | `design.md`'s AC-5 fixture shape (two raft entries + three coarse entries only). | `render_silhouette_composite` fails closed with `Err` on a zero-rectangle group, so the designed fixture would return an error instead of the warnings under test. Review confirmed the assertion is not weakened: the no-pixels checks are positional samples inside the raft band and the coarse rects, disjoint from the body entry, and `geometry_points_mm` still ingests the skipped geometry so those samples are in-canvas rather than off-image. |
| D-5 | `RenderError::MissingGeometryField` named `regions[].polygons` even for a support-only capture group. | `design.md`'s error contract — the field string should name the field actually missing on the capture arm taken. | Reported by review as a finding and fixed: the field string is now selected per capture arm rather than hard-coded to the Slice-family field. |
| D-6 | `legend_version_for` was widened so schema `1.2.0` maps to `LEGEND_VERSION` (`"1.1.0"`). | `design.md` scopes Step 5 to bundle assembly and forbids bumping `LEGEND_VERSION`. | Without the widening a 1.2.0 bundle recorded `legend_version: "1.0.0"` — a pre-existing defect surfaced during Step 1 and fixed in Step 5. `LEGEND_VERSION` itself was **not** bumped, and the 1.0.0 branch is byte-identical. |

**Forward-looking item for packets 248–251 (not a deviation).** `Layer::Slice` is not a `STAGE_ORDER` member — the scheduler stage is `PrePass::Slice` — so the silhouette group comparator's `unwrap_or(usize::MAX)` fallback sorts `Layer::Slice` *after* `Layer::SlicePostProcess`. Ordering stays total and deterministic (a `BTreeMap` keyed on a `(usize, String)` tuple), but every future non-`STAGE_ORDER` tap ties at `usize::MAX` and falls back to tap-string order. Packets 248–251 should expect this `STAGE_ORDER` / `BLACKBOARD_TAP_STAGE_IDS` naming gap when they add taps.

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — normative plan; long (~800 lines): ranged reads only (§3 facts, §4.1–4.3, §5–§7 for this packet).
- `docs/19_visual_debug.md` — current user-facing contract (232 lines; direct read).
- `docs/specs/_OLD/visual-pipeline-debug.md` — archived v1 contract (Projector single-owner rule, bundle contract); delegate — only those two sections apply.
- `docs/08_coordinate_system.md` — units and Z convention; direct read of the summary sections.

## Doc Impact Statement (Required)

- `docs/19_visual_debug.md` — new "Silhouette Side Views (schema 1.2.0)" section: request/manifest shape, view semantics, single-plane-per-bundle rule, `world_bounds_mm` plane semantics, model-wide Z framing + D4 scale guidance (must contain the exact phrase `for interface-band inspection on tall models, raise` followed by `resolution_scale` — AC-10 pins this verbatim string), fixed paint order + D2 occlusion caveat, W1/W2 warnings inventory, supported/rejected taps, filename scheme — `rg -q 'silhouette' docs/19_visual_debug.md && rg -q 'for interface-band inspection on tall models, raise' docs/19_visual_debug.md && rg -qi 'occlu' docs/19_visual_debug.md`
- `docs/02_ir_schemas.md` section "IR 9a — SupportGeometryIR" — correct "keyed by support-layer index" to the producer-verified semantics (model-layer/global index on the support emit schedule; `u32::MAX` sentinel unchanged) — `rg -q 'model-layer' docs/02_ir_schemas.md`
- `docs/DEVIATION_LOG.md` — one new open row for the raft side-view follow-up (W1) — `rg -q 'raft side' docs/DEVIATION_LOG.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
