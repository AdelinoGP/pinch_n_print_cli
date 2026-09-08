# Design: 249-visual-debug-silhouette-postpass-multicolor

## Controlling Code Paths

- Primary code path: `validate_request` → `run_model_source` (`crates/pnp-cli/src/visual_debug.rs`) → `run_postpass_taps` (same file; tiers 2–4 via `execute_per_layer_with_events_and_support_tools`, `execute_layer_finalization`, `slicer_runtime::postpass::execute_postpass_with_capture` with the `PostPassCapture` sink) → **new** `postpass_stage_captures` (`WholePrint`) → packet 247's silhouette branch → **new** `render_silhouette_composite_styled` (`crates/slicer-runtime/src/visual_debug_render.rs`).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs` (drives `execute_postpass_with_capture` directly; builds `CapturedIr::LayerFinalization(capture.finalized_layers.clone())` — the shape this packet's fixtures mirror), `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs` (model-source PostPass e2e harness — the AC-2/AC-7 driver pattern exists today), packet 247's `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` and `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` (wedge helpers `wedge_path`/`module_dir`/`write_bounded_config`).
- OrcaSlicer comparison: none — PnP-native tool; no parity obligations.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This path never converts units: `Point3WithWidth.x/y/z/width` and `LayerCollectionIR.z` are millimeter `f32`s end-to-end (`docs/08`); no `Point2`/`mm_to_units` appears in the new code.
- Projector single-owner rule (archived spec, binding): rectangle corners go through `Projector::project(x_or_y_mm, z_mm)`; no new transform.
- D10 shape lock: the whole-print capture is a **shape of `StageCapture` rows**, not a change to `CapturedIr` (the plan rejects `Arc`-ing the payload — it would change the serialized `typed_capture` shape and every match arm). `StageCapture.layer_index`/`layer_z` on a whole-print row are `0`/`0.0` and documented unread; no consumer may branch on them.
- The existing per-layer postpass rows and every top-down render path are byte-frozen (AC-10); `shapes_for_styled` is not touched — silhouette tool coloring lives entirely in the composite path.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): fixtures of `LayerCollectionIR` use `..Default::default()`; `PrintEntity` has no `Default` by design — use the existing suites' `// exhaustive:` waiver; `PostPassCapture` implements `Default` (its `::default()` is the existing `run_postpass_taps` construction pattern).

## Code Change Surface

- Selected approach: the capture-shape seam is the row-building loop already inside `run_postpass_taps` — extract it as a pub, directly unit-testable function with a shape enum, rather than adding a parallel postpass driver (the tiers-2–4 execution is identical for both shapes; only row assembly differs). The renderer grows a styled entry point mirroring the existing `render_stage_capture` → `render_stage_capture_styled` precedent.

- Exact surface, per file:
  - `crates/pnp-cli/src/visual_debug.rs`
    - `pub enum PostpassCaptureShape { PerLayer, WholePrint }` and `pub fn postpass_stage_captures(capture: &slicer_runtime::postpass::PostPassCapture, stage_ids: &[String], applicable: &[u32], applicable_layer_z: &dyn Fn(u32) -> f32, shape: PostpassCaptureShape) -> Vec<slicer_runtime::StageCapture>` — implementer may simplify the `layer_z` lookup to a slice of `(u32, f32)` pairs; the contract is: `PerLayer` reproduces today's rows byte-for-byte (per applicable layer × tap, whole-print payload cloned per row, `layer_z` from the matching finalized layer); `WholePrint` emits one row per tap (`CapturedIr::LayerFinalization(finalized_layers.clone())` / `CapturedIr::GCodeEmit(gcode_ir.clone())` — one clone per tap total), `layer_index: 0`, `layer_z: 0.0`, doc-commented unread.
    - `run_postpass_taps(ctx, request, support_tools, shape: PostpassCaptureShape)` — signature gains the shape; the `applicable.is_empty()` → `NoApplicableLayer` guard and the whole-print closure reporting stay identical for both shapes. Blast radius: exactly one call site (`run_model_source`, the `postpass_output` block) — it passes `WholePrint` iff the request's visualizations are silhouettes (the mixing ban makes this a bundle-wide predicate 247 already computes).
    - `validate_request`: `SILHOUETTE_TAP_STAGE_IDS` += `"PostPass::LayerFinalization"`; remove the blanket silhouette `color_by: "tool"` → `InvalidColorBy` rejection (the R7 `tool_color_source` checks stay); everything else untouched.
    - Silhouette assembly branch (authored by 247): grouping key extended from (tap, view) to (tap, view, color mode); per postpass group — build `SilhouetteSlabSchedule` from the whole-print capture's finalized layers (sorted by `global_layer_index`; `z_bottom` = previous finalized `z`, `0.0` first) filtered to the resolved selection; `layers_rendered` = selection ∩ finalized indices as maximal `LayerRangeEntry` ranges; call `render_silhouette_composite_styled` with `RenderStyle { color_by, tool_colors }` (tool colors via existing `filament_tool_colors`/`ToolColors::default()` resolution, `tool_palette` via existing `tool_palette_entries`); tool-group filenames insert `_tool` before `.png`; entry `color_by`/`tool_color_source` fields set exactly as the existing top-down entries set them.
  - `crates/slicer-runtime/src/visual_debug_render.rs`
    - `pub fn render_silhouette_composite_styled(captures: &[StageCapture], view: SilhouetteView, resolution_scale: u32, viewport: ViewportBoundsMm, schedule: &SilhouetteSlabSchedule, style: &RenderStyle) -> Result<(RenderedImage, Vec<String>), RenderError>`; `render_silhouette_composite` becomes a delegation with `RenderStyle::default()` (AC-8 pins byte-equivalence).
    - Extraction arm `CapturedIr::LayerFinalization(layers)` (whole-print): draw a layer iff `schedule` carries its slab; per `PrintEntity`, per consecutive point pair, interval `[min(h0 − w0/2, h1 − w1/2), max(h0 + w0/2, h1 + w1/2)]`; class = role (role mode) or `tool_index` (tool mode); `travel_moves` contribute nothing; paths with <2 points skipped.
    - Class-order extension: role ranks — non-support roles first ordered ascending by role name (`Custom` by inner string), then `SupportMaterial`, `SupportBaseInterface`, `SupportInterface` last — composed with 247's existing class order (the support-plan role order is unchanged); colors via the in-module `role_color(&ExtrusionRole)`. Tool mode: classes ascending `tool_index`, colors `style.tool_colors.color(t)`.
    - Tool mode over any capture arm without tool assignment (all 247 blackboard arms) → `Err(RenderError::ToolColorUnavailable { tap, layer_index })` (existing variant; `layer_index` = the capture's `layer_index`).
  - `crates/slicer-runtime/src/lib.rs` — re-export `render_silhouette_composite_styled` beside the 247 re-exports.
  - Tests: new `crates/pnp-cli/tests/visual_debug_postpass_silhouette_tdd.rs`; edits to `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` (renderer ACs) and `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` (AC-N1/N3 + the AC-N2 retarget).
  - Docs: `docs/19_visual_debug.md`.

- Rejected alternatives and reasons:
  - `Arc<Vec<LayerCollectionIR>>` inside `CapturedIr` — changes the serialized `typed_capture` shape and every existing match arm (plan D10's rejected alternative).
  - Accepting the per-layer clones for silhouette — memory scales as print size × layer count (plan fact 8); the OOM-shaped failure the packet exists to avoid.
  - Passing the layer selection to the renderer as a new parameter — would change 247's exported `render_silhouette_composite` signature for all callers; the schedule already carries exactly the per-layer slab data, so filtering it encodes selection with zero signature churn for role-mode callers.
  - Slabs from `ctx.blackboard.layer_plan()` for the postpass group — finalization modules may alter layers; the capture's own finalized `z` values are the only heights the rendered IR can attest (D8 slab-source note; D1's per-region-height rule is inapplicable — `LayerCollectionIR` has no `effective_layer_height`).
  - Keeping validation-time tool rejection with a tap whitelist instead of the per-capture render error — 247's `[FWD]` and plan R6 pin the per-capture `ToolColorUnavailable` contract (the pinned top-down precedent), and a validation whitelist would fork two sources of truth for "tool-carrying".
  - Extending `shapes_for_styled` for silhouette tool coloring — the composite path never goes through `shapes_for*`; touching it risks the frozen top-down contract for zero reuse.

## Files in Scope (read + edit)

- `crates/pnp-cli/src/visual_debug.rs` — capture shape, validation lift, assembly extension; the packet's largest edit.
- `crates/slicer-runtime/src/visual_debug_render.rs` — styled entry point + LayerFinalization arm + class-order extension.
- `crates/slicer-runtime/src/lib.rs` — one re-export line.
- Tests: new `crates/pnp-cli/tests/visual_debug_postpass_silhouette_tdd.rs`; `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`; `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`.
- Docs: `docs/19_visual_debug.md`.

Justification for exceeding three primaries: same two-crate split 247 established (CLI assembly vs runtime pixel math); the lib.rs edit is one line; each step stays ≤3 edits.

## Read-Only Context

- `crates/slicer-runtime/src/postpass.rs` — `PostPassCapture` (`finalized_layers: Vec<LayerCollectionIR>`, `gcode_ir: GCodeIR`) and `execute_postpass_with_capture` signature only — never edited.
- `crates/slicer-runtime/src/layer_executor.rs` — `StageCapture`, `CapturedIr::LayerFinalization`/`GCodeEmit`, `POSTPASS_TAP_STAGE_IDS` region only — never edited.
- `crates/slicer-ir/src/slice_ir.rs` — `LayerCollectionIR`/`PrintEntity`/`ExtrusionPath3D`/`Point3WithWidth`/`ExtrusionRole` shapes only — never edited.
- `crates/slicer-runtime/src/visual_debug_style.rs` — `ColorBy`/`ToolColors` only (`RenderStyle` itself lives in `crates/slicer-runtime/src/visual_debug_render.rs`) — never edited.
- `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs` — fixture pattern for `PostPassCapture` construction — edit only if a pin conflicts (none expected; the per-layer shape is unchanged).
- `crates/pnp-cli/tests/visual_debug_intermediate_renderer_tdd.rs` — wedge harness helpers only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not applicable (no parity), never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `crates/pnp-cli/src/visual_debug_gcode.rs` and everything packet 248 touches — this packet must build and pass with or without 248 present.
- `modules/core-modules/**` sources — guest artifacts matter only as prebuilt test dependencies (`cargo xtask build-guests --check` before blaming e2e failures).
- Packet directories 247/248/250/251 (beyond 247's two read-only files) — never create or modify files there.
- `docs/07_implementation_status.md` — worker-dispatch updates only at the completion gate.

## Expected Sub-Agent Dispatches

- Question: exact landed shape of 247's silhouette branch (grouping key construction, where the bundle-wide silhouette predicate lives) in `run_model_source`; scope: `crates/pnp-cli/src/visual_debug.rs`; return: `SNIPPETS ≤2×30 lines`; purpose: Steps 1/4.
- Question: does 247's `silhouette_tool_coloring_rejected_role_accepted` live in `visual_debug_validation_tdd.rs` and what does it assert?; scope: that file; return: `FACT`; purpose: Step 4 retarget.
- Question: run the step's test binary tee'd to `target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure; purpose: every step.
- Question: `cargo xtask build-guests --check` exit code before the wedge e2e steps; scope: repo root; return: `FACT (0/1/3)`; purpose: Steps 4–5.
- Question: `cargo xtask check-literals` exit code after new fixtures; scope: repo root; return: `FACT`; purpose: Steps 2–3.

## Data and Contract Notes

- IR/manifest contracts: no IR/WIT/schema change anywhere — `PostPassCapture`, `StageCapture`, `CapturedIr`, and every manifest field shape are consumed as-is; the only manifest deltas are 247's existing fields on new entries (`view`, `layers_rendered`, `color_by`, `tool_color_source`, `tool_palette`). 1.0/1.1 bundles byte-frozen (unchanged code paths; AC-10 regression pins the postpass one).
- WIT boundary: none.
- Determinism/scheduler constraints: whole-print extraction iterates `layers` sorted by `global_layer_index` (sort defensively — `Vec` order is producer order); rectangle emission ascending layer → class order (role ranks or tool index) → interval start; group order per 247 (STAGE_ORDER position, then tap, then role-before-tool within a (tap, view)); schedule built from sorted finalized indices; all sources are `Vec`s — no `HashMap` iteration.
- Viewport note: `compute_silhouette_viewport_bounds` consumes `geometry_points_mm`, whose `CapturedIr::LayerFinalization` arm already includes every layer's entity points and travel destinations (verified) — so a whole-print capture makes framing selection-independent by construction; travel destinations may widen the horizontal frame slightly beyond extrusion, matching the top-down viewport's existing behavior. Width inflation (≤ half a line width) may exceed the geometry-point union but sits far inside the 2 mm margin — accepted, noted for reviewers.

## Locked Assumptions and Invariants

- One whole-print `StageCapture` per postpass tap on silhouette bundles; per-layer rows byte-identical for every non-silhouette bundle (both pinned by AC-1's two arms).
- Whole-print rows carry `layer_index: 0` / `layer_z: 0.0` as documented-unread placeholders — no consumer branches on them; the schedule is the sole selection/slab authority for whole-print captures.
- Slabs for finalized layers are consecutive-z diffs from the capture's own layers, first from 0 — never the blackboard layer plan, never per-region heights (which the IR does not carry).
- Tool coloring is legal only where the capture carries tool assignment; everywhere else fails closed with the existing `ToolColorUnavailable` — silhouette gets no looser rule than the top-down renderer's pinned contract (plan D17/R6).
- `render_silhouette_composite` ≡ `render_silhouette_composite_styled` with `RenderStyle::default()`, byte-for-byte (AC-8).
- Grouping key is (tap, view, color mode); filenames `{sanitized_tap}_silhouette_{view}[_tool].png`; one silhouette plane per bundle (247 invariant, inherited).

## Risks and Tradeoffs

- `run_postpass_taps` still executes the whole print (tiers 2–4) for any postpass tap — D10 removes the clone multiplication, not the execution cost; that cost is inherent to postpass taps (plan facts 8/15) and documented.
- A silhouette request mixing a blackboard tap and `PostPass::LayerFinalization` under `color_by: "tool"` fails at render time (per-capture contract), not validation — a user discovers the incompatibility later than a validation error would tell them. Deliberate: this is the pinned top-down contract 247's `[FWD]` mandates; the error names the offending tap.
- The (tap, view, color mode) grouping key change touches 247's assembly code — the role-only path must produce byte-identical bundles before/after (AC-10 plus 247's own suite gate this).
- If packet 248 lands first, its model-source-only tool rejection is removed by this packet's validator edit; if 249 lands first, 248's narrowing becomes a no-op edit on an already-lifted check. Both orders compose because the two packets touch disjoint validator clauses; the swarm executes queue order regardless.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 — validation lift + assembly extension + retargeted tests)
- Highest-risk dispatch and required return format: Step 4/5 wedge e2e runs (guest-WASM-dependent); `FACT pass/fail` preceded by the `build-guests --check` exit code.

## Open Questions

- `[FWD to packet 250]` Exports you consume: `PostpassCaptureShape::WholePrint` (its `GCodeEmit` arm already emits the single whole-print capture), `render_silhouette_composite_styled` (add your `CapturedIr::GCodeEmit` extraction arm there), the (tap, view, color mode) grouping, and the schedule-as-selection-filter semantics for whole-print captures. Extend `SILHOUETTE_TAP_STAGE_IDS` with `"PostPass::GCodeEmit"` and retarget AC-N1's `gcode_emit_silhouette_still_rejected` pin when you land.
- `[FWD to packet 251]` Tool-colored silhouette entries exist from this packet on; when compositing seam glyphs onto silhouette bases, define glyph visibility over tool palettes as well as role palettes.
- `[FWD to the batch orchestrator]` 247's unowned-tap question (`PrePass::RegionMapping`/`PrePass::OverhangAnnotation`) remains unowned after this packet; this packet deliberately did not absorb RegionMapping (its dynamic `config_tint` class-ordering story deserves its own decision record).
- No `[BLOCK]` items.
