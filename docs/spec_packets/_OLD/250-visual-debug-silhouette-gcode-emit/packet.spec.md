---
status: implemented
packet: 250-visual-debug-silhouette-gcode-emit
task_ids:
  - TASK-452
  - TASK-453
  - TASK-454
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
plan_source: docs/specs/visual-debug-silhouette-side-views-plan.md (Packet Queue row #4)
---

# Packet Contract: 250-visual-debug-silhouette-gcode-emit

## Goal

Extend the `silhouette` visualization kind (packets 247/249, schema 1.2.0) to `PostPass::GCodeEmit`: recover per-move extrusion widths from the typed `GCodeIR` command stream by consecutive-`Some(e)` position differencing (plan fact 9 corrected + D11 — `Move.e` is the accumulated E position; `Δe < 0` inline purge retracts are non-extruding and skipped; `e: None` travels carry no interval and never reset the carried position), bucket each extruding move by Z-containment into finalized-layer schedule slabs with the W4 nearest-slab out-of-slab warning, render roles or tools through a dedicated GCodeEmit composite entry point on the D10 single whole-print capture, fix `run_postpass_taps` to configure its `DefaultGCodeEmitter` with the model source's resolved config (grounding-surfaced: today it emits with `ResolvedConfig::default()`, so the captured stream's E values ignore the request's `filament_diameter`), and lift packet 249's interim `SilhouetteUnsupportedForTap` rejection for this tap.

## Scope Boundaries

Model source only, tap `PostPass::GCodeEmit` only: the renderer's inversion/bucketing entry point plus the shared flow-width closed form promoted from packet 248 (`crates/slicer-runtime/src/visual_debug_render.rs`), the assembly/validation/schedule plumbing and the emitter-config fidelity fix (`crates/pnp-cli/src/visual_debug.rs`). The standalone gcode source (packet 248), `PostPass::LayerFinalization` (packet 249), seam overlays (packet 251), and every top-down render path stay behavior-untouched — the emitter-config fix changes captured-IR fidelity for postpass taps on both view families and owns that fallout. Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packets 247, 248, and 249 (all currently `draft` — FORWARD-DEP; every step consuming their exports states "packet NNN implemented" as a precondition; the swarm executes queue order).
- Unblocks: nothing in the queue (packet 251 depends on #1 only).
- Activation blockers: packets 247/248/249 not yet `implemented`.

## Acceptance Criteria

- **AC-1. Given** a `schema_version: "1.2.0"` model-source request over `resources/regression_wedge.stl` with tap `PostPass::GCodeEmit`, a layer range, and one `{"type": "silhouette"}` visualization, **when** `run_visual_debug` succeeds, **then** the bundle contains exactly one silhouette PNG named `images/PostPass__GCodeEmit_silhouette_front.png`, its manifest entry has `"view": "front"`, a `"layers_rendered"` list equal to the resolved selection intersected with the finalized-layer indices (maximal inclusive ranges), **no** `"layer_index"`/`"layer_z"` key, and the postpass path produced exactly one whole-print `StageCapture` for the tap (`PostpassCaptureShape::WholePrint`, one `CapturedIr::GCodeEmit` clone total). | `cargo test -p pnp-cli --test visual_debug_gcode_emit_silhouette_tdd -- gcode_emit_silhouette_bundle_entry_shape 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a `LayerCollectionIR` fixture with entity paths of known `Point3WithWidth.width = 0.5` and `flow_factor = 1.0` emitted through `DefaultGCodeEmitter` configured `with_resolved_config` (`filament_diameter = 2.85`), **when** `gcode_emit_silhouette_segments` inverts the resulting `GCodeIR.commands`, **then** every extruding segment's recovered width equals `0.5` within `1e-3` mm (consecutive-`Some(e)` differencing against the emitter's accumulated positions, `w = Δe × π(d/2)² / (L₃D × h)` with `h` from the containing slab), proving the inversion is exact for PnP-generated streams. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit_e_inversion_roundtrips_emitter_width 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** a hand-built `GCodeIR` command stream containing, in order: an extruding `Move { e: Some(1.0) }`, a travel `Move { e: None }`, a second extruding `Move { e: Some(2.0) }`, and an inline purge-retract `Move { e: Some(1.2) }` (negative delta), **when** inverted, **then** exactly two extruding segments are produced — the second's `Δe` is `1.0` (the carried position survives the `e: None` travel; travel contributes no interval), and the negative-delta move contributes no segment (skipped like a degenerate move, never drawn). | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit_travel_carries_position_and_negative_delta_skipped 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a two-slab schedule (`[0, 0.2]`, `[0.2, 0.4]`) and extruding moves at `z = 0.2` and `z = 0.4`, **when** `render_gcode_emit_silhouette` renders, **then** each move's rectangle occupies its containing slab's rows (`z_bottom < z ≤ z_top`; a move at exactly the slab top buckets into that slab, never the one above) — verified by decoded pixels at two distinct slab heights — and no W4 warning is emitted (every Z in-slab is the W4 negative case). | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit_z_containment_buckets_without_w4 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** the same schedule plus one extruding move at `z = 0.7` (above every slab — the nonplanar case), **when** rendered, **then** that move's rectangle draws in the nearest slab (`[0.2, 0.4]`), its pixels are present (material is never silently dropped), and the warnings contain exactly one W4 entry naming the affected Z value `0.7` and stating nearest-slab placement. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit_out_of_slab_draws_nearest_with_w4 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** a command stream with a `GCodeCommand::ToolChange { from: 0, to: 1 }` between extruding runs overlapping in X, **when** rendered with `RenderStyle { color_by: ColorBy::Tool, .. }`, **then** intervals union per (slab, tool) with tool 0 initial before any `ToolChange`, tool 1 paints over tool 0 in the overlap (ascending tool index paint order), and each run's color equals `tool_colors.color(tool)`. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit_tool_classes_track_toolchange 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** a 1.2.0 wedge request with tap `PostPass::GCodeEmit` and two silhouette specs — one default (role) and one `options.color_by: "tool"` — **when** the bundle renders, **then** exactly two silhouette images exist with distinct paths `images/PostPass__GCodeEmit_silhouette_front.png` and `images/PostPass__GCodeEmit_silhouette_front_tool.png`, and the tool entry carries `"color_by": "tool"` with the manifest `tool_palette` present. | `cargo test -p pnp-cli --test visual_debug_gcode_emit_silhouette_tdd -- gcode_emit_role_and_tool_specs_render_distinct_images 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** the same `GCodeIR`, view, scale, viewport, schedule, style, and filament diameter, **when** `render_gcode_emit_silhouette` runs twice, **then** the two PNG byte vectors are identical and the warning lists are equal element-for-element. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit_silhouette_is_deterministic 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** two typed-capture requests (no visualizations) over the same wedge model with tap `PostPass::GCodeEmit`, differing only in the config's `filament_diameter` (`1.75` vs `2.85`), **when** both capture, **then** the first extruding `Move.e` values in the two `typed_capture` streams differ by the filament-area ratio `(2.85/1.75)⁻²` within `1e-3` relative — pinning that `run_postpass_taps`'s emitter now carries the request's resolved config instead of `ResolvedConfig::default()`. | `cargo test -p pnp-cli --test visual_debug_gcode_emit_silhouette_tdd -- postpass_capture_emitter_uses_request_config_diameter 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10 (docs).** `docs/19_visual_debug.md` gains a GCodeEmit-silhouette paragraph stating the tap's unique value (the pre-`GCodePostProcess` typed stream), the `Z-containment` bucketing rule with the W4 warning, and the honest caveat that this second E-inversion is `testable mainly against itself` (both quoted phrases are absent from the doc today, so the greps fail until written); the packet-248 deposited-width/rectangular-model caveats are extended to name GCodeEmit, not duplicated. | `rg -q 'Z-containment' docs/19_visual_debug.md && rg -q 'testable mainly against itself' docs/19_visual_debug.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a `GCodeIR` whose only E-carrying moves are negative-delta purge retracts (plus travels), **when** rendered as a silhouette, **then** the render fails closed with `RenderError::MissingGeometryField` (zero extruding rectangles across the whole group — packet 247's zero-rectangle rule, inherited), and no PNG bytes are produced. | `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd -- gcode_emit_all_negative_deltas_fail_closed 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** the tree after this packet, **when** searched, **then** packet 249's interim pin `gcode_emit_silhouette_still_rejected` is absent from `crates/`, a validation test named `gcode_emit_silhouette_accepted` exists and passes in its place, and packet 247's `silhouette_unsupported_taps_rejected_with_reasons` no longer exercises `PostPass::GCodeEmit` as a rejected tap (its remaining arms still pass). Each absence clause is vacuously true until its authoring packet (249, 247) is implemented — a queue-order artifact; the clauses become meaningful pins once those packets are in the tree. | `! rg -q 'gcode_emit_silhouette_still_rejected' crates/ && rg -q 'gcode_emit_silhouette_accepted' crates/pnp-cli/tests/visual_debug_validation_tdd.rs && cargo test -p pnp-cli --test visual_debug_validation_tdd -- gcode_emit_silhouette_accepted 2>&1 | tee target/test-output.log | grep -E "^test result" && cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_unsupported_taps_rejected_with_reasons 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** a 1.2.0 **gcode-source** silhouette request whose `taps` names `PostPass::GCodeEmit`, **when** validated, **then** it is still rejected with `ValidationError::SilhouetteUnsupportedForTap` (the standalone gcode source has no pipeline taps — packet 248's rule, unchanged by this lift). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- gcode_silhouette_rejects_named_taps 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** the existing top-down postpass consumers, **when** this packet's changes (including the emitter-config fix) land, **then** the agent-determinism suite (model-source `PostPass::GCodeEmit` top-down bundle, two-run byte comparison) passes unchanged. | `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p pnp-cli --test visual_debug_gcode_emit_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — normative plan; long (~811 lines): ranged reads only (facts 9/10/12, D8's GCodeEmit row + slab-source note, D10, D11, D16, D17, §6 W4, §7, §9's round-trip test row).
- `docs/spec_packets/247-visual-debug-silhouette-core/packet.spec.md` + `design.md`, `docs/spec_packets/248-visual-debug-silhouette-gcode-source/packet.spec.md` + `design.md`, `docs/spec_packets/249-visual-debug-silhouette-postpass-multicolor/packet.spec.md` + `design.md` — the exports this packet builds on (read-only; never edit those directories).
- `docs/19_visual_debug.md` — current user-facing contract; range-read post-247/248/249 (grown past its pre-247 232 lines).

## Doc Impact Statement (Required)

- `docs/19_visual_debug.md` — extend the silhouette section with a `PostPass::GCodeEmit` paragraph: pre-`GCodePostProcess` unique value, position-differencing E-inversion (negative deltas skipped, travels carry the position), `Z-containment` slab bucketing + W4, the `testable mainly against itself` caveat, and a GCodeEmit mention appended to packet 248's deposited-width/rectangular-model caveats — `rg -q 'Z-containment' docs/19_visual_debug.md && rg -q 'testable mainly against itself' docs/19_visual_debug.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
