---
status: implemented
packet: 249-visual-debug-silhouette-postpass-multicolor
task_ids:
  - TASK-449
  - TASK-450
  - TASK-451
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
plan_source: docs/specs/visual-debug-silhouette-side-views-plan.md (Packet Queue row #3)
---

# Packet Contract: 249-visual-debug-silhouette-postpass-multicolor

## Goal

Extend the `silhouette` visualization kind (packet 247, schema 1.2.0) to `PostPass::LayerFinalization` via the plan's D10 single whole-print capture shape (one `StageCapture` carrying the finalized `Vec<LayerCollectionIR>` once — never one whole-print clone per layer), rendering typed `Point3WithWidth` segment projections inflated by `width / 2` on schedule-z-diff slabs, and bring `color_by: "tool"` to silhouettes (D17): per-(layer, tool) interval classes on tool-carrying captures, ascending-tool paint order, `tool_palette` manifest emission, and the per-capture `RenderError::ToolColorUnavailable` fail-closed contract replacing packet 247's blanket validation rejection.

## Scope Boundaries

Surface: `run_postpass_taps`/`run_model_source`/`validate_request` in `crates/pnp-cli/src/visual_debug.rs` and the silhouette composite in `crates/slicer-runtime/src/visual_debug_render.rs` (a styled entry point plus the `CapturedIr::LayerFinalization` extraction arm). `PostPass::GCodeEmit` silhouettes stay rejected (packet 250); the standalone gcode source, seam overlays, and every existing top-down render path — including the per-layer postpass capture rows the top-down consumers read — stay byte-untouched. This packet depends on packet 247 only (not on 248). Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 247 (`247-visual-debug-silhouette-core`, currently `draft` — FORWARD-DEP; every step consuming its exports states "packet 247 implemented" as a precondition; the swarm executes queue order). Explicitly **not** dependent on packet 248.
- Unblocks: packet 250 (GCodeEmit silhouette — reuses the whole-print capture shape and the styled composite entry point).
- Activation blockers: packet 247 not yet `implemented`.

## Acceptance Criteria

- **AC-1. Given** a `PostPassCapture` fixture with three finalized `LayerCollectionIR` rows and `stage_ids = ["PostPass::LayerFinalization"]`, **when** `postpass_stage_captures` runs with `PostpassCaptureShape::WholePrint`, **then** it returns exactly **one** `StageCapture` whose `CapturedIr::LayerFinalization` payload contains all three layers; **and when** it runs with `PostpassCaptureShape::PerLayer` over three applicable layers, **then** it returns three captures, each carrying the full three-layer payload (the existing top-down shape, pinned unchanged). | `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- postpass_whole_print_shape_one_capture_per_tap 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a `schema_version: "1.2.0"` model-source request over `resources/regression_wedge.stl` with tap `PostPass::LayerFinalization`, a layer range, and one `{"type": "silhouette"}` visualization, **when** `run_visual_debug` succeeds, **then** the bundle contains exactly one silhouette PNG named `images/PostPass__LayerFinalization_silhouette_front.png`, and its manifest entry has `"view": "front"`, a `"layers_rendered"` list equal to the resolved selection intersected with the finalized layer indices (maximal inclusive ranges), and **no** `"layer_index"` or `"layer_z"` key. | `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- postpass_silhouette_bundle_entry_shape 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a whole-print `CapturedIr::LayerFinalization` fixture with two layers at `z = 0.2` and `z = 0.4` and an entity path of known `Point3WithWidth.width = 0.4`, **when** `render_silhouette_composite_styled` renders it, **then** layer slabs are `[0, 0.2]` and `[0.2, 0.4]` (consecutive finalized z-diffs, first from 0) and the drawn horizontal run spans the segment's projected extent inflated by each endpoint's own `width / 2` — verified by decoded-pixel extents against a half-width-narrower control. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- finalized_layer_slabs_and_half_width_inflation 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a whole-print capture with three finalized layers and a `SilhouetteSlabSchedule` containing a slab for only the middle layer, **when** rendered, **then** only the middle layer's rectangles contribute pixels (the schedule is the selection filter for whole-print captures; unselected layers draw nothing). | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- schedule_filter_gates_whole_print_layers 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** one finalized layer whose entities carry roles `SparseInfill`, `SupportMaterial`, and `SupportInterface` overlapping in X, **when** rendered by role, **then** the overlap paints the `SupportInterface` class color (paint order: non-support roles first in ascending role-name order, then `SupportMaterial`, then `SupportBaseInterface`, then `SupportInterface` last), with each non-overlapped run in its own `role_color`. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- finalization_role_paint_order_deterministic 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** one finalized layer with entities at `tool_index` 0 and 1 overlapping in X, **when** rendered with `RenderStyle { color_by: ColorBy::Tool, .. }`, **then** intervals union per (layer, tool), tool 1 paints over tool 0 in the overlap (ascending tool index paint order), and each run's color equals `tool_colors.color(tool)`. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- tool_classes_paint_ascending_tool_index 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** a 1.2.0 wedge request with tap `PostPass::LayerFinalization` and two silhouette specs — one default (role) and one `options.color_by: "tool"` — **when** the bundle renders, **then** exactly two silhouette images exist with distinct paths `images/PostPass__LayerFinalization_silhouette_front.png` and `images/PostPass__LayerFinalization_silhouette_front_tool.png`, the tool entry carries `"color_by": "tool"` and `"tool_color_source": "palette"`, and the manifest's `tool_palette` table is present. | `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- silhouette_role_and_tool_specs_render_distinct_images 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** the same capture group, view, scale, viewport, schedule, and `RenderStyle`, **when** `render_silhouette_composite_styled` runs twice, **then** the two PNG byte vectors are identical and the warning lists equal element-for-element; and `render_silhouette_composite` (the packet-247 entry point) remains byte-equivalent to the styled call with `RenderStyle::default()`. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- styled_composite_is_deterministic_and_default_equivalent 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** two 1.2.0 postpass silhouette requests over the same wedge model, one selecting a layer subset and one all layers, **when** both bundles render, **then** both entries record byte-identical `world_bounds_mm` (the whole-print capture and model-extent union make framing selection-independent). | `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- postpass_z_frame_is_model_wide_not_selection_wide 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10. Given** the existing top-down postpass consumers, **when** this packet's changes land, **then** the pre-existing agent-determinism suite (model-source `PostPass::GCodeEmit` top-down bundle) passes unchanged — the per-layer capture rows and their manifests are byte-stable. | `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-11 (docs).** `docs/19_visual_debug.md` documents the postpass silhouette: the `single whole-print capture` shape (D10, why per-layer clones would OOM-scale), the tool-colored silhouette rules (legal captures, palette/filament sources), and the D2 occlusion caveat applied per tool. | `rg -q 'single whole-print capture' docs/19_visual_debug.md && rg -q 'tool-colored silhouette' docs/19_visual_debug.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a 1.2.0 silhouette request naming tap `PostPass::GCodeEmit`, **when** validated, **then** it is still rejected with `ValidationError::SilhouetteUnsupportedForTap` carrying that tap name (only `PostPass::LayerFinalization` joins `SILHOUETTE_TAP_STAGE_IDS`; packet 250 owns GCodeEmit). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- gcode_emit_silhouette_still_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a 1.2.0 model-source silhouette request with `options.color_by: "tool"` and tap `Layer::Slice` (a blackboard capture with no tool assignment), **when** it passes validation (the packet-247 blanket `InvalidColorBy` rejection is removed) and reaches rendering, **then** the command fails closed with `RenderError::ToolColorUnavailable` naming the tap, and no bundle content is written. Both prior validation-time pins of the model-source tool rejection are retired to this per-capture contract: packet 247's `silhouette_tool_coloring_rejected_role_accepted` and packet 248's `model_silhouette_tool_coloring_still_rejected` (old names absent from `crates/`, replacement test present in the new binary). Note: each absence clause is vacuously true until its authoring packet is implemented (247 for the first, 248 for the second — a queue-order artifact); the clauses become meaningful pins once those packets are in the tree. | `rg -q 'silhouette_tool_on_blackboard_tap_fails_tool_color_unavailable' crates/pnp-cli/tests/visual_debug_postpass_silhouette_tdd.rs && cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- silhouette_tool_on_blackboard_tap_fails_tool_color_unavailable 2>&1 | tee target/test-output.log | grep -E "^test result" && ! rg -q 'silhouette_tool_coloring_rejected_role_accepted' crates/ && ! rg -q 'model_silhouette_tool_coloring_still_rejected' crates/`
- **AC-N3. Given** a 1.2.0 silhouette spec with `options.tool_color_source: "filament"` but no `color_by: "tool"`, and separately an unknown `tool_color_source` value, **when** validated, **then** both are rejected with `ValidationError::InvalidColorBy` (the existing 1.1.0 R7 rules, inherited by silhouette unchanged). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- tool_color_source_rules_inherited_on_silhouette 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** a request whose silhouette specs resolve to the same (tap, view, color mode) twice, **when** the bundle renders, **then** duplicate specs collapse into one image (no filename collision, extending packet 247's grouping rule to the color-mode key). | `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd -- duplicate_tool_specs_collapse_to_one_group 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — normative plan; long (~811 lines): ranged reads only (facts 8/12/14/15, D8's LayerFinalization row + slab-source note, D10, D17, §6 R6/R7, §7).
- `docs/spec_packets/247-visual-debug-silhouette-core/packet.spec.md` + `design.md` — the exports this packet builds on (read-only; never edit that directory).
- `docs/19_visual_debug.md` — current user-facing contract; direct read (confirm length post-247; range-read if grown past 300 lines).

## Doc Impact Statement (Required)

- `docs/19_visual_debug.md` — extend the packet-247 silhouette section with: the postpass tap row (`PostPass::LayerFinalization`, schedule-z-diff slabs, why finalized layers cannot use per-region heights), the `single whole-print capture` note (D10), and a "tool-colored silhouette" paragraph (legal captures, `tool_palette`/`tool_color_source` semantics, per-tool occlusion caveat, `_tool` filename suffix) — `rg -q 'single whole-print capture' docs/19_visual_debug.md && rg -q 'tool-colored silhouette' docs/19_visual_debug.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
