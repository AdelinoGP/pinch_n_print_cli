# Design: 251-visual-debug-silhouette-seam-overlays

## Controlling Code Paths

- Primary code path: `validate_request` → `run_model_source`'s silhouette branch (`crates/pnp-cli/src/visual_debug.rs`, authored by packet 247) → seam events from `ctx.blackboard.seam_plan()` (the `layer_plan()`-opt-in precedent in the same file) → **new** seam-overlay render entry beside `render_silhouette_composite`/`_styled` (`crates/slicer-runtime/src/visual_debug_render.rs`) → `Projector` + `Canvas` + `draw_glyph`.
- Neighboring tests/fixtures: `crates/pnp-cli/tests/visual_debug_overlays_tdd.rs` (isolated-overlay bundle assertions, `overlay_events` mirroring patterns), packet 247's `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` and `crates/pnp-cli/tests/visual_debug_silhouette_bundle_tdd.rs` (wedge harness, decoded pixels), `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs` (`decode_rgb` + glyph-pixel assertion patterns).
- OrcaSlicer comparison: none — PnP-native tool; no parity obligations.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This path never converts units: `SeamPlanEntry.chosen_candidate.point` is a mm `Point3WithWidth` (fact 11); glyph centers go through `Projector::project(x_or_y_mm, z_mm)` — the single-owner rule, no new transform.
- Serialization lock: `OverlayEvent` is `Serialize`-only with `#[serde(tag = "event", rename_all = "snake_case")]`; the new `Seam.z` must be `Option<f32>` + `skip_serializing_if = "Option::is_none"`, and every non-silhouette construction site passes `None` — 1.0/1.1 manifests (including `overlay_events` on gcode and top-down model bundles) stay byte-identical, pinned by AC-5 on serialization output.
- Legend lock: the seam glyph is `GlyphKind::Circle` in `overlay_palette::SEAM` (`[220, 0, 0]`), `GLYPH_HALF_PX` (6) × `resolution_scale` — identical to `event_glyph`'s 1.1.0 mapping; `LEGEND_VERSION` stays `"1.1.0"` (fills got a schema bump in 247; glyphs are unchanged in meaning).
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): `SeamPlanIR`/`SeamPlanEntry`/`SeamPosition` fixture literals use `..Default::default()` where `Default` exists (`SeamPosition`, `Point3WithWidth` derive it; `SeamPlanEntry` — verify; fall back to the `// exhaustive:` waiver); `VisualizationOptions` gains a 6th field and is already watched — test literals keep `..Default::default()`.

## Code Change Surface

- Selected approach: seams are a **bundle-level, tap-agnostic** decoration on silhouette groups. The seam data comes once from the blackboard (never from captures — no silhouette tap carries `SeamPlanIR`), gets filtered per group's rendered layers, and is drawn by two small renderer variants that wrap the packet 247/249 composite internals. Existing entry-point signatures are frozen; the new behavior is additive entry points plus assembly wiring.

- Exact surface, per file:
  - `crates/slicer-runtime/src/visual_debug_style.rs`
    - `OverlayEvent::Seam` gains `/// Z, mm — silhouette seam events only; absent (None) on all 1.0/1.1 paths.` `#[serde(skip_serializing_if = "Option::is_none")] z: Option<f32>`. Blast radius (verified 2026-08-27, pre-247 tree — re-derive at implementation): construction `visual_debug_render.rs` `collect_overlay_events` `Perimeter` arm (`{ x, y }` → `{ x, y, z: None }`) and `SeamPlan` arm (same — top-down stays `None` deliberately even though `p.z` exists: byte-compat outranks completeness there); patterns `visual_debug_render.rs` `draw_overlay_events` or-pattern and `crates/pnp-cli/src/visual_debug_gcode.rs` glyph-loop or-pattern (add `..`); `visual_debug_style.rs` `kind()`/`event_glyph()` already use `{ .. }`. No test constructs `OverlayEvent::Seam` as a literal (verified by workspace grep; re-verify).
  - `crates/slicer-runtime/src/visual_debug_render.rs`
    - `pub fn silhouette_seam_events(seam_plan: &slicer_ir::SeamPlanIR, view: SilhouetteView, rendered_layers: &BTreeSet<u32>) -> Vec<OverlayEvent>` — entries in source order, filtered by `region_key.global_layer_index ∈ rendered_layers`; each yields `OverlayEvent::Seam { x: p.x, y: p.y, z: Some(p.z) }` (both world coordinates kept so the event is self-describing; the glyph projects the per-view horizontal — `x` for `Front`, `y` for `Side` — against `z`).
    - `pub fn render_silhouette_seam_overlay(captures: &[StageCapture], view: SilhouetteView, resolution_scale: u32, viewport: ViewportBoundsMm, schedule: &SilhouetteSlabSchedule, seam_plan: &slicer_ir::SeamPlanIR, rendered_layers: &BTreeSet<u32>) -> Result<(RenderedImage, Vec<OverlayEvent>, Vec<String>), RenderError>` — the isolated form: build the group's role-mode rectangles via the 247 internals, recolor `FAINT_BASE` (`recolor_shapes` precedent), draw, then glyphs at `Projector::project(h, z)` with `GLYPH_HALF_PX × resolution_scale`; returns the drawn events and the group's warnings.
    - `pub fn render_silhouette_composite_seamed(...same params as render_silhouette_composite_styled... , seams: Option<(&slicer_ir::SeamPlanIR, &BTreeSet<u32>)>) -> Result<(RenderedImage, Vec<OverlayEvent>, Vec<String>), RenderError>` — the composited form: the styled composite plus a glyph pass before `encode_png`; `render_silhouette_composite_styled` delegates with `seams: None` (byte-equivalence pinned by 249's AC-8 and this packet's AC-6). Exact internal factoring is the implementer's (the canvas must be glyph-able pre-encode); the frozen contract is the delegation equivalence. If packet 250 landed `render_gcode_emit_silhouette` as a separate entry, give it the same optional glyph pass only if a GCodeEmit group requests composited seams — same mechanism, no per-tap logic.
  - `crates/slicer-runtime/src/lib.rs` — re-export the three new symbols beside the 247–250 re-exports.
  - `crates/pnp-cli/src/visual_debug.rs`
    - `VisualizationOptions` gains `#[serde(default)] pub composited_overlays: Option<Vec<String>>` (doc comment: silhouette-only, 1.2.0-only, `"seams"` the only member).
    - `validate_request`: the existing `overlays` block splits by kind — `diagnostic_overlay` keeps today's rules verbatim; `silhouette` accepts exactly `["seams"]` on a model source (gcode → `OverlayUnsupportedOnGcode { name: "seams" }`; non-seams member → `InvalidOverlays` naming seams-only); any other kind keeps the existing `InvalidOverlays`. New `composited_overlays` checks: silhouette-only, model-source-only (gcode → `OverlayUnsupportedOnGcode`), members exactly `["seams"]`, non-empty, declared schema must be 1.2.0 (both the 1.1.0 typed path and the 1.0.0 stray-key loop reject with a message naming `"1.2.0"` — the `view` precedent from 247's AC-N4); group-conflict rule: silhouette specs resolving to one (tap, view, color mode) must serialize identical `overlays`+`composited_overlays` values, else `InvalidOverlays`.
    - Silhouette assembly branch: compute `rendered_layers` (the group's selection ∩ schedule indices — the same set behind `layers_rendered`); when any silhouette spec has overlay forms, read `ctx.blackboard.seam_plan()` once (`None` → fail closed via `VisualDebugError::CaptureFailed` whose message contains `seam plan`, mirroring `TapSourceUnavailable`'s stance — pinned through the AC-N8 helper test); per (tap, view): isolated form → one extra image `{sanitized_tap}_silhouette_{view}_overlay_seams.png`, emitted once regardless of color modes (faint base ignores color), entry `overlay: Some("seams")`, `overlay_events: Some(events)`; composited form → the group's base render goes through the seamed entry, entry gains `composited_overlays: Some(vec!["seams"])` + `overlay_events`; a spec with both forms yields both.
    - `ImageEntry` gains `#[serde(skip_serializing_if = "Option::is_none")] pub composited_overlays: Option<Vec<String>>`. Blast-radius: every `ImageEntry` literal site in `visual_debug.rs` (4 pre-247; more after 247–250 — dispatch a LOCATIONS sweep at implementation, never trust this count) adds `composited_overlays: None`; no test constructs `ImageEntry` as a Rust literal (247's verified claim — re-verify).
  - Tests: new `crates/pnp-cli/tests/visual_debug_seam_overlay_tdd.rs`; edits to `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` and `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` (R9 matrix, retirements).
  - Docs: `docs/19_visual_debug.md`.

- Rejected alternatives and reasons:
  - Populating `z` on the top-down `SeamPlan`/`Perimeter` event arms — changes every 1.1.0 `overlay_events` byte stream; the frozen-legacy-bytes invariant (247's AC-8 lineage) outranks field completeness. Locked to `None` there.
  - A plain (non-`Option`) `z: f32` field — same byte break, structurally.
  - Sourcing seams from a `CapturedIr::SeamPlan` capture by adding `PrePass::SeamPlanning` to the silhouette whitelist — R2 rejects the tap (a seam is a point, not slab geometry); the overlay is decoration on other taps' silhouettes, which is exactly D18's shape.
  - Per-overlay mode objects `[{kind, mode}]` or a `composite_seams` bool — plan D18's explicitly rejected request shapes.
  - Distinct filename for the composited base (e.g. `_composited_seams`) — the plan's composited form *is* the base image; a second base per group would break the request-to-entry mapping. The group-conflict validation (AC-N6) closes the collision hole instead.
  - Isolated image per (tap, view, color mode) — two byte-identical faint-base files per tap under role+tool bundles; collapsing to (tap, view) preserves filename uniqueness and determinism.
  - Glyph outline/halo for tool-palette visibility — restyles a legend-1.1.0 glyph per palette; documented caveat instead (the isolated form is the legibility escape hatch 1.1.0 built).

## Files in Scope (read + edit)

- `crates/pnp-cli/src/visual_debug.rs` — options, R9 validation, assembly, manifest field; the packet's largest edit.
- `crates/slicer-runtime/src/visual_debug_render.rs` — seam events, isolated + composited render entries.
- `crates/slicer-runtime/src/visual_debug_style.rs` + `crates/slicer-runtime/src/lib.rs` + `crates/pnp-cli/src/visual_debug_gcode.rs` — the `Seam.z` field, its pattern fixes, re-exports.
- Tests: new `crates/pnp-cli/tests/visual_debug_seam_overlay_tdd.rs`; `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`; `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`.
- Docs: `docs/19_visual_debug.md`.

Justification for exceeding three primaries: the `Seam.z` addition mechanically touches three files (enum + two pattern sites) in one step; the rest is the established two-crate silhouette split; each step stays ≤3 edits.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — `SeamPlanIR`/`SeamPlanEntry`/`SeamPosition`/`Point3WithWidth`/`RegionKey` shapes only — never edited.
- `crates/slicer-runtime/src/blackboard.rs` — the `seam_plan()` accessor (correction during implementation: not `layer_executor.rs` as originally written) and the `TapSourceUnavailable` precedent — never edited.
- `crates/pnp-cli/tests/visual_debug_overlays_tdd.rs` — event-assertion patterns; edit only if a pin conflicts (its assertions are key-presence-based, tolerant of the additive field — verified; the suite is a required regression run regardless).
- Packet 247/248/249/250 spec/design files — export contracts; never edit those directories.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not applicable (no parity), never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `modules/core-modules/**` sources (incl. `seam-planner-default`) — guest artifacts matter only as prebuilt test dependencies (`cargo xtask build-guests --check` before blaming e2e failures).
- Packet directories 247–250 (beyond the read-only spec/design files) — never create or modify files there.
- `docs/07_implementation_status.md` — worker-dispatch updates only at the completion gate.

## Expected Sub-Agent Dispatches

- Question: every `OverlayEvent::Seam` construction/pattern site in the current tree (post-247/248/249/250); scope: `crates/`; return: `LOCATIONS ≤10`; purpose: Step 1 blast radius (re-derive; the pre-247 inventory above is a snapshot).
- Question: every `ImageEntry { ... }` literal site; scope: `crates/pnp-cli/src/visual_debug.rs` + `crates/pnp-cli/tests/`; return: `LOCATIONS ≤15`; purpose: Step 4 blast radius.
- Question: landed shape of 247's silhouette branch (grouping key, `layers_rendered` computation) and — if present — 250's seamed/gcode-emit entry; scope: `crates/pnp-cli/src/visual_debug.rs`, `crates/slicer-runtime/src/visual_debug_render.rs`; return: `SNIPPETS ≤3×30 lines`; purpose: Steps 3–4.
- Question: exact assertions of 248's `gcode_silhouette_overlay_rejections_unchanged` and 247's `composited_overlays_not_accepted_by_247`; scope: `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`; return: `FACT`; purpose: Step 2 retirements.
- Question: run the step's test binary tee'd to `target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure; purpose: every step.
- Question: `cargo xtask build-guests --check` exit code (0/1/3) before wedge e2e steps; `cargo xtask check-literals` after fixtures; return: `FACT`; purpose: Steps 4–5.

## Data and Contract Notes

- IR/manifest contracts: no IR/WIT change — `SeamPlanIR` is consumed as-is. Manifest deltas are 1.2.0-only: the additive `z` on silhouette seam events, `composited_overlays` on composited entries, `overlay`/`overlay_events` reuse on isolated silhouette entries. 1.0/1.1 output byte-frozen (AC-5 pins the seam event; unchanged code paths pin the rest).
- WIT boundary: none.
- Determinism/scheduler constraints: events in `SeamPlanIR.entries` source order (the committed plan is deterministic; matches the existing SeamPlan-arm convention), filtered — never sorted differently from the top-down mirror; glyphs drawn in event order after all rectangles; isolated images emitted in group order per 247; warnings order inherited from the base render. No `HashMap` iteration.
- Filename contract: `{sanitized_tap}_silhouette_{view}_overlay_seams.png` (247's `_overlay_{kind}` insertion rule); composited form adds no filename. Every silhouette variant in one bundle remains collision-free (247's invariant + AC-N6's conflict rejection).

## Locked Assumptions and Invariants

- Seams are model-source, `SeamPlanIR`-sourced, `chosen_candidate`-only; the silhouette never re-derives or guesses a seam, and a requested-but-uncommitted seam plan fails closed (AC-N8).
- `OverlayEvent::Seam.z` is `Some` exactly on silhouette-sourced events and `None` everywhere else; 1.0/1.1 serialization is byte-identical before/after (AC-5).
- `overlays: ["seams"]` on a silhouette keeps the exact 1.1.0 isolated meaning: faint base, glyphs, mirrored events — no colored base in the same image.
- `"seams"` is the only legal member of either option on a silhouette; travel/retraction/z-hop/tool-change glyphs stay excluded (plan §8).
- `render_silhouette_composite`/`_styled` (and 250's GCodeEmit entry, if landed) remain byte-equivalent when no seams are requested.
- One silhouette plane per bundle; one isolated seam image per (tap, view); the composited form never adds a file. Glyph/color/legend are 1.1.0's; `LEGEND_VERSION` unbumped.

## Risks and Tradeoffs

- Glyph legibility over dense tool palettes: the fixed red circle can sit on similar hues. Accepted with a docs caveat (AC-8) — the isolated form is the designed escape hatch; restyling the glyph would fork the legend.
- The group-conflict rejection (AC-N6) is stricter than silent last-writer-wins but is the only behavior that keeps one base filename per group honest; a user wanting both a plain and a composited base uses two bundles (the D6 two-bundle workflow precedent).
- Sub-pixel slabs (D4) can leave a seam glyph floating over background in its own layer band — the glyph is still honest (the seam's Z is real); noted in the docs subsection rather than suppressed.
- `..` rest patterns on the `Seam` match arms trade exhaustiveness-checking for additive-field tolerance at exactly two glyph-drawing sites — the same tradeoff every existing `{ .. }` arm in `visual_debug_style.rs` already made.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 — assembly wiring + bundle tests over the wedge pipeline)
- Highest-risk dispatch and required return format: Step 4/5 wedge e2e runs (guest-WASM-dependent; seam plan must be committed by `seam-planner-default`); `FACT pass/fail` preceded by the `build-guests --check` exit code (0/1/3).

## Open Questions

- `[FWD to implementer]` If `SeamPlanEntry` lacks `Default`, use the `// exhaustive:` waiver pattern for fixtures (as the support suites do) rather than adding a derive to `slicer-ir`.
- `[FWD to implementer]` AC-7 needs a tool-carrying base; if the wedge fixture's `PostPass::LayerFinalization` single-tool output makes the tool-palette assertion trivial, a two-tool `support_filament` config (the `visual_debug_typed_tap_capture_tdd` pattern) supplies a second tool without new fixtures.
- `[FWD to the batch orchestrator]` This is the queue's last row: 247's unowned `PrePass::RegionMapping`/`PrePass::OverhangAnnotation` question leaves the batch still unowned — record it in the final batch report for a follow-up packet decision.
- No `[BLOCK]` items.
