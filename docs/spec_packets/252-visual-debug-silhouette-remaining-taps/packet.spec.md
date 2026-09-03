---
status: implemented
packet: 252-visual-debug-silhouette-remaining-taps
task_ids:
  - TASK-458
  - TASK-459
  - TASK-460
  - TASK-461
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
plan_source: docs/specs/visual-debug-silhouette-side-views-plan.md (Packet Queue row #6)
---

# Packet Contract: 252-visual-debug-silhouette-remaining-taps

## Goal

Close the plan's D8 silhouette tap whitelist: add `PrePass::RegionMapping` (joined `SliceIR` rows, per-region slabs, deterministic `config_tint` interval classes) and `PrePass::OverhangAnnotation` (`overhang_quartile_polygons[layer]` bands on honest SliceIR-derived slabs via a new per-layer height index — never schedule z-diffs) to the silhouette renderer and validation surface, retiring packet 247's interim `SilhouetteUnsupportedForTap` rejections for exactly these two taps.

## Scope Boundaries

Model source only; the two taps above. `PrePass::MeshAnalysis`, `PrePass::SeamPlanning`, and every arena tap stay rejected (plan §8 — no Z attribution / execution cost); the bridge/overhang `xy_footprint` shapes of `CapturedIr::SurfaceClassification` are **not** drawn on silhouettes (no Z or layer field — plan fact 5; only the per-layer-keyed quartile bands are). No `CapturedIr` shape change, no manifest shape change, no schema bump, no change to any top-down renderer or to packets 248–251's surfaces. Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 247 (`247-visual-debug-silhouette-core`, currently `draft` — FORWARD-DEP; every step consuming its exports states "packet 247 implemented" as a precondition; the swarm executes queue order, so 248–251 will also precede this row's implementation).
- Unblocks: nothing (last silhouette queue row; D8 whitelist closed after this packet).
- Activation blockers: none known.

## Acceptance Criteria

- **AC-1. Given** a `CapturedIr::RegionMapping` capture whose retained `slice_ir` row at layer top `z` carries two regions with distinct `effective_layer_height` values (one catch-up-sized) and a `RegionMapIR` joining both, **when** `render_silhouette_composite` renders it, **then** each joined region's rectangle bottom corresponds to its own `z − effective_layer_height` (the catch-up-sized region's bottom strictly below the other's, never one uniform slab) and each paints its own `config_tint` color — verified by decoded-pixel assertions at the two distinct slab bottoms. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- region_mapping_slabs_follow_joined_effective_layer_height 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** two `RegionMapIR` entries on one layer resolving to distinct `ResolvedConfig` contents (distinct `config_tint` RGB triples) whose regions overlap in the projected axis, **when** rendered twice, **then** the overlap paints the lexicographically-larger `(r, g, b)` tint (tint classes paint in ascending RGB order, later class wins), 247's occlusion warning fires naming the affected layer count, and the two `RenderedImage.png_bytes` are byte-identical. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- region_mapping_tint_class_order_and_determinism 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a `RegionMapIR` entry on a selected layer with no matching `SlicedRegion` in the retained `slice_ir` (same skip case `region_mapping_shapes` tolerates top-down), **when** rendered, **then** the entry contributes no pixels and the returned warnings contain one warning naming the unjoined-entry count — never a silent drop. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- region_mapping_unjoined_entries_warn_and_skip 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a `CapturedIr::SurfaceClassification` capture with quartile bands 1–4 at one layer and a `SilhouetteSliceHeightIndex` whose layer has a single height class `h`, **when** `render_silhouette_overhang_composite` renders it, **then** every band rectangle spans `[z − h, z]`, each quartile paints its own palette constant, and where bands' projected intervals overlap the highest quartile's color wins (paint order ascending quartile, Q4 last) — decoded-pixel assertions. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- overhang_bands_single_height_slabs_and_quartile_order 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a layer whose height index carries two height classes (two objects with distinct `effective_layer_height`, disjoint XY footprints) and one quartile band containing one polygon inside each footprint, **when** rendered, **then** the band yields rectangles with two distinct bottoms — each polygon's rectangle spanning its containing class's `[z − h, z]` via the `slicer_core::polygon_ops::intersection` partition — never one merged slab. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- overhang_bands_partition_across_mixed_height_classes 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** the same overhang capture group, view, scale, viewport, and height index, **when** `render_silhouette_overhang_composite` runs twice, **then** the two `RenderedImage.png_bytes` are byte-identical and the warning lists are equal element-for-element (band iteration is a per-layer keyed `get`, bands sorted ascending by `quartile` — no `HashMap` iteration order reaches the output). | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- overhang_composite_is_deterministic 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** two 1.2.0 model-source silhouette requests over `resources/regression_wedge.stl` with tap `PrePass::RegionMapping`, one selecting a layer subset and one all layers, **when** both bundles render, **then** each bundle contains exactly one PNG named `images/PrePass__RegionMapping_silhouette_front.png`, each manifest entry carries `"visualization": "silhouette"`, `"view": "front"`, `"layers_rendered"`, no `"layer_index"`/`"layer_z"` key, and both entries record byte-identical `world_bounds_mm` (model-wide Z framing, selection-independent). | `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd -- region_mapping_bundle_entry_and_model_wide_frame 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** 1.2.0 model-source silhouette requests naming `PrePass::RegionMapping` and `PrePass::OverhangAnnotation`, **when** validated via the library-call harness, **then** both pass validation with no `SilhouetteUnsupportedForTap` (the whitelist lift), while the same request shape naming `PrePass::MeshAnalysis` is still rejected. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_region_mapping_and_overhang_taps_accepted 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9 (docs).** `docs/19_visual_debug.md`'s silhouette tap table (authored by packet 247) gains one row per new tap: the RegionMapping row naming its deterministic `tint class` ordering and joined per-region slabs, and the OverhangAnnotation row naming per-`quartile` band colors and the SliceIR-derived height classes (neither anchor exists in the doc today; both greps fail until the rows are written). | `rg -q 'quartile' docs/19_visual_debug.md && rg -q 'tint class' docs/19_visual_debug.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** the tree after this packet, **when** packet 247's `silhouette_unsupported_taps_rejected_with_reasons` runs, **then** it passes exercising only its remaining rejected arms — the arena taps, `PrePass::MeshAnalysis`, and `PrePass::SeamPlanning` (the `PostPass::LayerFinalization` and `PostPass::GCodeEmit` arms fall away when packets 249/250 lift those taps — 250's AC-N2 pins the GCodeEmit retirement explicitly; 249's lift implies the LayerFinalization one) — with the `PrePass::RegionMapping` and `PrePass::OverhangAnnotation` arms removed by this packet; the test is never deleted (packet 250's AC-N2 re-runs it). The absence-of-arm clause is vacuously true until packet 247 is implemented — a queue-order artifact; it becomes a meaningful pin once 247 is in the tree. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_unsupported_taps_rejected_with_reasons 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a 1.2.0 model-source silhouette request with `options.color_by: "tool"` and tap `PrePass::RegionMapping` (and separately `PrePass::OverhangAnnotation`), **when** it reaches rendering, **then** it fails closed with `RenderError::ToolColorUnavailable` naming the tap (blackboard captures carry no tool assignment — the pinned contract; the whitelist lift must not silently render role colors instead). Meaningful only once packets 247 and 249 are implemented (until 249's per-capture contract lands, 247's blanket validation `InvalidColorBy` makes the render path unreachable) — a queue-order artifact; the swarm executes queue order, so both precede this packet. The rejection fires at group assembly/extraction before any geometry is read, so the test is deterministic regardless of the wedge's overhang-band content. | `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd -- silhouette_tool_on_remaining_taps_fails_tool_color_unavailable 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** a quartile band whose `quartile` value is outside `1..=4`, **when** rendered, **then** `render_silhouette_overhang_composite` fails closed with the new named `RenderError::InvalidQuartile` carrying the tap, layer, and offending value — never guessed into a palette bucket. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- overhang_invalid_quartile_fails_closed 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** an overhang capture group whose selected layers have no quartile bands at all, **when** rendered, **then** it fails closed with `RenderError::MissingGeometryField` (never a blank image), matching the top-down empty-shapes contract and 247's zero-rectangle rule. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- overhang_empty_bands_fail_closed 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N5. Given** an `PrePass::OverhangAnnotation` capture group and a height index missing an entry for a layer that carries bands, **when** rendered, **then** it fails closed with `RenderError::MissingGeometryField` naming the height-index field — the renderer never substitutes a schedule z-diff or any other guessed slab. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- overhang_missing_height_index_layer_fails_closed 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — normative; long (~820 lines): ranged reads only (fact 5, D1, D2, D8's two tap rows, §6, §7, Packet Queue row #6).
- `docs/spec_packets/247-visual-debug-silhouette-core/design.md` — the foundation contract (exports, `SILHOUETTE_TAP_STAGE_IDS`, warning machinery, filename scheme); direct read.
- `docs/19_visual_debug.md` — user-facing contract (edited here); direct read.
- `docs/08_coordinate_system.md` — units and Z convention; direct read of the summary sections.

## Doc Impact Statement (Required)

- `docs/19_visual_debug.md` — extend the packet-247 silhouette tap table with the `PrePass::RegionMapping` row (joined per-region slabs; deterministic `tint class` paint order — ascending RGB; unjoined-entry warning) and the `PrePass::OverhangAnnotation` row (per-`quartile` colors, ascending paint order; SliceIR-derived height classes; bridge/overhang footprints deliberately excluded) — `rg -q 'quartile' docs/19_visual_debug.md && rg -q 'tint class' docs/19_visual_debug.md`
- No deviation-log row: this packet closes 247's `[FWD]` coverage gap rather than deferring anything new.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
