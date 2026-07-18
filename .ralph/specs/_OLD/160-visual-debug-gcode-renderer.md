---
status: implemented
packet: 160-visual-debug-gcode-renderer
task_ids:
  - TASK-270
---

# 160-visual-debug-gcode-renderer

## Goal

Implement the standalone final-G-code visual-debug path that parses the documented Pinch 'n Print G0/G1 subset and writes deterministic final-view PNG artifacts plus manifest warnings.

## Problem Statement

Packet 157 (implemented, commit `3e33ca01`) and packet 158 (implemented) establish the opt-in visual-debug command, bundle contract, and typed-tap capture, but the standalone final-G-code source still cannot produce the final artifact a printer would receive: `crates/pnp-cli/src/visual_debug.rs:519-561`'s `VisualDebugSource::Gcode` arm is a verified placeholder that never opens the request's G-code file, never parses it, uses only the first requested layer for every synthesized image, and never writes PNG bytes (no PNG-encoding dependency exists in the crate). TASK-270 closes that rendering slice without depending on the intermediate renderer. It must make unsupported input visible as warnings, preserve unknown-role extrusion, iterate every requested/resolved layer, and fail rather than create misleading evidence.

## Architecture Constraints

- Final rendering consumes serialized text after `PostPass::TextPostProcess`, not `GCodeIR`, so the artifact represents the printer-facing final G-code.
- This is an opt-in artifact path only. No `slice` flag, typed tap, scheduler edge, module invocation, ordinary-slice allocation, or intermediate-renderer dependency may be introduced.
- Unsupported commands are warnings with source line numbers, never guessed geometry; a bundle succeeds only when its requested PNGs and manifest are complete.
- `Viewport { width, height }` (`visual_debug.rs:264-267`) is pixel dimensions derived from `resolution_scale` only, already computed identically for both source arms before dispatch (`visual_debug.rs:499-503`); this packet must not change its shape. The "one model-wide XY viewport" requirement (AC-4) is an internal mm-space bounding-box computation (parsed geometry + documented margin) used only to project consistently into that already-shared pixel canvas.
- No PNG-encoding dependency exists in `crates/pnp-cli/Cargo.toml` today, and no code path in the crate writes PNG bytes yet — this packet is the first to add both. PNG files must be fully written before the existing atomic `manifest.json` temp-then-rename commit (`visual_debug.rs:604-620`), so a failed run never leaves a successful-looking `manifest.json` next to partial/missing images.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: no IR or schema changes; `Manifest`/`ImageEntry` already carry every field this packet populates (`gcode_parser_version`, `png_path`, `viewport`, `legend_version`, `warnings`) — this packet only fills them with real values in place of the placeholder's synthesized ones.
- WIT boundary: none.
- Determinism/scheduler constraints: source-order parsing, stable warning/image ordering, fixed palette/legend, one internally-computed model-wide XY bounding box reused for every emitted PNG, and no scheduler interaction.

## Locked Assumptions and Invariants

- Standalone G-code is the only source mode for this renderer and `final_gcode` is the only tap it owns; `TapSelector` is untagged so no new enum variant is needed for the tap name.
- `filled_areas` requires explicit `gcode_line_width_mm`; E values never determine physical bead width. This requirement is already validated by packet 157 (`visual_debug.rs:197-215`); AC-N1's new test confirms existing behavior rather than exercising new validation logic.
- Role-less extrusion is retained as `unclassified`; unsupported constructs are not approximated.
- Successful output contains the complete requested manifest and PNG set, never a partial success; PNG files are written before the existing atomic `manifest.json` rename.
- The placeholder's `req.layers.first()`-only behavior must not survive: every requested/resolved layer needs its own rendered entry per requested tap/visualization combination.

## Risks and Tradeoffs

- The documented subset intentionally omits full printer macro and preview semantics; warnings make that limitation observable instead of silently fabricating geometry.
- A new pure-Rust PNG dependency is required (verified: none exists in `crates/pnp-cli/Cargo.toml` today); implementation must record its enabled features and license review without expanding this packet into unrelated rendering work.
- The placeholder's per-layer behavior (`req.layers.first()` only) must be restructured into a layer loop; this is a real, verified gap, not a hypothetical one — underestimating it risks an AC-1 regression where only one layer's images are ever produced.
