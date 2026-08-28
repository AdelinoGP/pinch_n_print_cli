# Requirements: 247-visual-debug-silhouette-core

## Packet Metadata

- Grouped task IDs: `TASK-442`, `TASK-443`, `TASK-444`, `TASK-445` (new rows; crosswalk in `task-map.md`)
- Backlog source: `docs/07_implementation_status.md` (no existing open TASK covers this; lineage TASK-267..270 closed with packets 157–161)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`pnp_cli visual-debug` renders only top-down XY views. Tree-support defects — branch tapering, interface-band placement/count, raft/base structure — are vertical: invisible from above and currently diagnosable only from IR JSON or G-code text. The approved plan (`docs/specs/visual-debug-silhouette-side-views-plan.md`, reviewed 2026-08-27) defines a `silhouette` visualization kind (schema 1.2.0) that composites selected layers into one X–Z or Y–Z image per (tap, view) via mathematically exact interval projection. This packet is queue row #1 (plan §4.7 steps 1+2): the tracer over the simplest slab source plus the motivating support-plan use case, carrying every foundation later rows build on — the schema gate, the mixing ban, the composite render path, model-wide Z framing, and the 1.2.0 manifest shape.

## In Scope

- Schema 1.2.0 acceptance in `validate_request` (`crates/pnp-cli/src/visual_debug.rs`): `schema_supported` accepts `"1.2.0"`; 1.2.0 requests get the 1.1.0 strict typed options parse; nothing 1.1.0 rejects becomes legal except the `silhouette` kind and `options.view`.
- New `silhouette` visualization kind with `options.view: "front"` (X–Z, default) | `"side"` (Y–Z); unknown values fail closed (plan D5, R5).
- Full 247 fail-closed validation matrix (plan §6 subset): R1 named requires-1.2.0 rejection; R3 mixing ban (silhouette never mixes with `filled_areas`/`filament_lines`/`diagnostic_overlay` in one request, and all silhouette specs in one request must resolve to one view — preserves the pinned bundle-wide `world_bounds_mm` byte-identity, plan fact 6/D6); R4 `frame: "plate"` rejection; R2 `SilhouetteUnsupportedForTap` for every tap outside this packet's supported set; interim rejections for the gcode source (packet 248), `color_by: "tool"` (packet 249), and `composited_overlays` (packet 251).
- Supported silhouette taps in 247: the four `CapturedIr::Slice`-payload taps — `Layer::Slice`, `PrePass::PaintSegmentation`, `Layer::PaintRegionAnnotation`, `Layer::SlicePostProcess` (identical render machinery, one `CapturedIr` match arm) — and `PrePass::SupportGeometry` (`CapturedIr::SupportGeometry`).
- Composite render entry point `render_silhouette_composite` in `crates/slicer-runtime/src/visual_debug_render.rs`: per-(layer, class) interval unions (exact endpoint comparison, sorted sweep; contour-only projection — holes cannot disconnect a connected contour's projection), per-region slabs `[z − effective_layer_height, z]` for Slice-family taps (plan D1), schedule slabs for the support tap, rectangles drawn through the existing `Projector` (single-owner rule) and `Canvas::fill_polygon`, fixed class paint order (plan D2), deterministic emission order, occlusion warning when a later class actually overlaps an earlier one.
- Support tap semantics (plan D9): `SupportPlanIR` roles only, per-role colors (interface bands distinct from body); negative `global_layer_index` (raft) entries skipped with warning W1 naming count + dropped index range; non-empty coarse `SupportGeometryIR.entries` skipped with warning W2 naming count; deviation-log row for the raft follow-up.
- Model-wide Z framing (plan D3): silhouette viewport horizontal axis from `MeshIR::build_volume`'s X (front) or Y (side) extent, vertical axis from its Z extent, both unioned with captured geometry, `VIEWPORT_MARGIN_MM` applied; selection never changes the frame.
- 1.2.0 manifest shape (plan D7): `ImageEntry.layer_index` becomes `Option<i64>` + `skip_serializing_if`; `layer_z` becomes the tri-state `Option<Option<f64>>` + `skip_serializing_if` (correcting the plan's "gains only `skip_serializing_if`" — the gcode source really emits `"layer_z": null` for a layer with no `;Z:` marker, so a plain skip attribute would break 1.0/1.1 byte-compatibility; `Some(None)` preserves the `null`); new `view` and `layers_rendered` (list of inclusive `{start, end}` ranges — maximal ascending runs, lossless) fields, both skip-when-absent; no `typed_capture` payload on silhouette entries (`None`, serialized `null` like gcode entries today); 1.0/1.1 serialization output pinned byte-compatible by tests.
- Per-bundle composite assembly in `run_model_source`: silhouette requests group captures by (tap, view), render once per group, one manifest entry per group ordered by `STAGE_ORDER` position then tap name; filenames `{sanitized_tap}_silhouette_{view}.png`, unique per bundle.
- Docs: `docs/19_visual_debug.md` silhouette section (D4 scale guidance, D2 occlusion caveat placed where a PNG-reading agent will see it, warnings inventory); `docs/02_ir_schemas.md` IR 9a key-semantics correction (producer-verified: `execute_support_geometry` in `crates/slicer-core/src/algos/support_geometry.rs` keys by model-layer/global index on the emit schedule); the raft deviation row.
- New test binaries `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` and `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs` (top-level files — each is its own test binary; no aggregator registration exists or is needed for these crates' test layout).

## Out of Scope

- Gcode-source silhouette, `;Z:` slabs, flow-derived widths, W3, R8/R10 — packet 248.
- `PostPass::LayerFinalization`, the D10 single whole-print capture shape, `color_by: "tool"` on silhouette, D17 — packet 249.
- `PostPass::GCodeEmit`, E-inversion, Z-containment bucketing, W4 — packet 250.
- Seam overlays on silhouette (isolated and `composited_overlays` forms), `z` on seam `overlay_events`, R9 — packet 251.
- `PrePass::RegionMapping` and `PrePass::OverhangAnnotation` silhouettes (rejected here with `SilhouetteUnsupportedForTap`; no queue row owns them — see design.md Open Questions `[FWD]`).
- Arena taps (`Layer::Perimeters` … `Layer::PathOptimization`) — whole-print per-layer execution cost, plan fact 14; `PrePass::MeshAnalysis`, `PrePass::SeamPlanning` — no Z attribution, plan fact 5.
- Raft slab derivation (W1 tracks the follow-up); coarse `SupportGeometryIR.entries` rendering (W2); sub-pixel band inflation (D4); depth cues of any kind; `frame: "plate"` for silhouettes; any change to the top-down renderers, `LEGEND_VERSION`, or the existing overlay system.
- Bumping `LEGEND_VERSION` (`crates/slicer-runtime/src/visual_debug_style.rs`, `"1.1.0"`): silhouettes add fill classes, not glyphs; silhouette entries record the existing legend via `legend_version_for`.

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — ~800 lines; ranged reads only (grounding facts §3, decisions D1–D9, shapes §5, matrix §6, determinism §7).
- `docs/19_visual_debug.md` — 232 lines; direct read (edited in this packet).
- `docs/02_ir_schemas.md` — >1600 lines; delegate; only "IR 9a — SupportGeometryIR" applies.
- `docs/specs/_OLD/visual-pipeline-debug.md` — archived; delegate; only "Bundle Contract" and the Projector single-owner rule apply.
- `docs/08_coordinate_system.md` — direct read; X/Y scaled integers via `Point2::to_mm`, Z mm floats end-to-end.
- `docs/21_data_defaults_and_fixtures.md` — struct-literal churn gate for test fixtures; delegate the watchlist question if a new literal trips `cargo xtask check-literals`.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-11`. Measurable refinements: AC-1's `layers_rendered` must round-trip losslessly (deserializing the ranges and re-expanding yields exactly the resolved index set); AC-6's determinism covers the warnings list order, not just PNG bytes.
- Negative: `AC-N1` through `AC-N10`.
- Cross-packet impact: packets 248–251 consume this packet's exports (`SilhouetteView`, `render_silhouette_composite`, the manifest `view`/`layers_rendered` fields, the validation seams for gcode/tool/overlay interim rejections). The interim rejections AC-N6/AC-N7/AC-N8 are deliberately packet-scoped and will be lifted by rows #2/#5/#3 respectively — their tests are named so those packets can retarget rather than delete blindly.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | R1–R5 + interim rejections (AC-N1..N10) | FACT pass/fail; failures via Grep on the log |
| `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | composite renderer: slab math, interval union, paint order, W1/W2, determinism (AC-2..AC-6) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_silhouette_bundle_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | end-to-end bundle: entry shape, Z framing, filenames (AC-1, AC-7, AC-9) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_request_bundle_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | 1.0/1.1 manifest byte-compat serialization (AC-8) + no regression in existing bundle contract | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_intermediate_renderer_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | existing top-down/manifest determinism suite unregressed by the `ImageEntry` change | FACT pass/fail |
| `cargo test -p slicer-runtime --test visual_debug_render_tap_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | existing renderer suite unregressed by new palette constants/helpers | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate on new test fixtures | exit code |
| `cargo check --workspace --all-targets` | whole-tree compile incl. test targets (blast radius) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- Step order is load-bearing: Step 2 (manifest shape) must land before Step 5 (bundle assembly) because Step 5 constructs silhouette `ImageEntry` rows using the `Option` fields; Steps 3–4 (renderer) are independent of Steps 1–2 and may proceed in parallel workers, but Step 5 needs all of 1–4.
- The end-to-end bundle tests (Steps 5) drive the real pipeline against `resources/regression_wedge.stl` + `modules/core-modules` and therefore require fresh guest WASMs: run `cargo xtask build-guests --check` before attributing any Step-5 test failure (exit 0 = fresh; exit 1 = rebuild without `--check`; exit 3 = infra error, not clean).
- The wedge fixture has no support demand, so it may commit no `SupportPlanIR` entries (and `render_silhouette_composite` fails closed on an empty group): the end-to-end bundle ACs (AC-1/AC-7/AC-9) therefore use only `CapturedIr::Slice` taps, and all support-tap behavior (roles, paint order, W1/W2) is pinned at the renderer level with direct fixtures (AC-4/AC-5). A support-enabled end-to-end run is welcome extra coverage but is not an AC and must not be a green-gate dependency.

## Context Discipline Notes

- `crates/pnp-cli/src/visual_debug.rs` (~2200 lines) and `crates/slicer-runtime/src/visual_debug_render.rs` (~2300 lines) are both long: ranged reads only, targeted at the symbols each step names.
- Do not load `docs/02_ir_schemas.md` or the archived spec in full — delegate the single-section facts.
- The plan document is the normative authority but ~800 lines — read only the sections a step cites.
