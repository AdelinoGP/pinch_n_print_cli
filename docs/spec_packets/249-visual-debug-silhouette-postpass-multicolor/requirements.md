# Requirements: 249-visual-debug-silhouette-postpass-multicolor

## Packet Metadata

- Grouped task IDs: `TASK-449`, `TASK-450`, `TASK-451` (new rows; re-derived 2026-08-27 as the next block after packet 248's TASK-446..448, over `docs/07_implementation_status.md`, `docs/specs/*.md`, all local `docs/spec_packets/*/task-map.md`, and `origin/master`-only packets 243–246; the completion-gate worker adds these rows to docs/07)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Tree-support and finalization defects (entity merge order, travel-reconciled output, per-tool structure) are only visible after `PostPass::LayerFinalization`, but packet 247 rejects postpass silhouettes for a structural reason it hands forward: `run_postpass_taps` (`crates/pnp-cli/src/visual_debug.rs`) builds **one `StageCapture` per (tap, applicable layer), each cloning the entire whole-print IR** (`capture.finalized_layers.clone()` per row — plan fact 8, verified). An all-layers silhouette through that shape clones the whole print once per layer — an OOM-shaped cost on exactly the large models a side view serves. The plan's D10 mandates a single whole-print capture for silhouette consumption with the per-layer rows byte-untouched for top-down consumers. Multicolor (D17) lands here too: silhouettes gain `color_by: "tool"` on tool-carrying captures with the per-capture `ToolColorUnavailable` contract replacing 247's blanket validation rejection — 247's `[FWD to packet 249]` names both obligations.

## In Scope

- Capture shape (D10): `pub enum PostpassCaptureShape { PerLayer, WholePrint }` and `pub fn postpass_stage_captures(...)` extracted from `run_postpass_taps`'s row-building loop in `crates/pnp-cli/src/visual_debug.rs`; `WholePrint` emits one `StageCapture` per tap carrying the whole-print IR once (`layer_index: 0` / `layer_z: 0.0`, doc-commented as unread for whole-print captures); `PerLayer` reproduces today's rows byte-for-byte. `run_postpass_taps` gains the shape parameter; the sole call site passes `WholePrint` iff the request is a silhouette bundle. The `WholePrint` arm handles `PostPass::GCodeEmit` generically (single capture) so packet 250 reuses it, even though validation keeps GCodeEmit silhouettes rejected here.
- Renderer (`crates/slicer-runtime/src/visual_debug_render.rs`): `pub fn render_silhouette_composite_styled(captures, view, resolution_scale, viewport, schedule, style: &RenderStyle) -> Result<(RenderedImage, Vec<String>), RenderError>`; the packet-247 `render_silhouette_composite` delegates with `RenderStyle::default()` (byte-equivalent, pinned). New extraction arm for `CapturedIr::LayerFinalization` (whole-print): a layer draws iff its `global_layer_index` has a slab in `schedule`; per entity, per consecutive `Point3WithWidth` pair, horizontal interval `[min(h0 − w0/2, h1 − w1/2), max(h0 + w0/2, h1 + w1/2)]` (h = x or y per view; paths with <2 points skipped; `travel_moves` never drawn); classes per `ExtrusionRole` (colors via the in-module `role_color`; paint order: non-support roles ascending by role name — `Custom` uses its inner string — then `SupportMaterial`, `SupportBaseInterface`, `SupportInterface` last) or per `PrintEntity.tool_index` under `ColorBy::Tool` (ascending paint order, colors from `RenderStyle.tool_colors`). Tool mode on any silhouette capture without tool assignment (every packet-247 blackboard arm) fails closed with the existing `RenderError::ToolColorUnavailable { tap, layer_index }`.
- Slab source: the postpass group's `SilhouetteSlabSchedule` is built by pnp-cli from the whole-print capture's own finalized layers — sorted by `global_layer_index`, `z_bottom` = previous finalized layer's `z` (0.0 for the first) — then filtered to the resolved layer selection. Rationale (plan D8 slab-source note, repeated per the queue directive): `LayerCollectionIR` carries no per-region `effective_layer_height`, so the schedule z-diff is the only height a finalized layer can honestly attest; D1's rejection of schedule diffs targets taps whose IR *does* carry per-region heights.
- Validation + assembly (`crates/pnp-cli/src/visual_debug.rs`): `SILHOUETTE_TAP_STAGE_IDS` += `"PostPass::LayerFinalization"` (only); the blanket silhouette `color_by: "tool"` → `InvalidColorBy` rejection removed for the model source (per-capture `ToolColorUnavailable` takes over; the R7 `tool_color_source` rules inherited unchanged); silhouette groups keyed (tap, view, color mode) with duplicate specs collapsing per key; tool-group filenames insert `_tool` before `.png` per 247's scheme; `layers_rendered` for postpass groups = resolved selection ∩ finalized indices as maximal inclusive ranges; `tool_palette` emitted via the existing `tool_palette_entries` (and `filament_tool_colors` when `tool_color_source: "filament"` on the model source — existing resolution reused, no new code path).
- 247/248 test surgery: both prior validation-time pins of the model-source tool rejection — 247's `silhouette_tool_coloring_rejected_role_accepted` and 248's `model_silhouette_tool_coloring_still_rejected` (if 248 has landed) — retired to the new per-capture contract (AC-N2); no other 247 or 248 pin changes.
- `docs/19_visual_debug.md` postpass + multicolor subsection (see Doc Impact in `packet.spec.md`).

## Out of Scope

- `PostPass::GCodeEmit` silhouettes, E-inversion over typed `Move.e`, Z-containment bucketing, W4 (packet 250; AC-N1 pins the boundary).
- The standalone gcode source in any form — this packet neither reads nor conditions on packet 248's work; the gcode-source tool-coloring case is owned by 248 (deviation from the queue note's D17 recommendation, recorded in 248's `requirements.md`: assigning it here would create a hidden 249→248 renderer dependency contradicting this packet's "depends on #1 only" row).
- Seam overlays / `composited_overlays` (packet 251).
- Any change to the existing per-layer postpass rows, the top-down renderers, `shapes_for_styled`, or `Arc`-ing `CapturedIr` payloads (plan D10 rejected alternative).
- `PrePass::RegionMapping` / `PrePass::OverhangAnnotation` silhouettes (unowned per 247's `[FWD]`; not absorbed here).

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — ~811 lines; ranged reads only: §3 facts 8/12/14/15, §4.3 D8 (LayerFinalization row + slab-source note) and D10, §4.5 D17, §6 R6/R7, §7.
- `docs/spec_packets/247-visual-debug-silhouette-core/` — read `packet.spec.md` + `design.md` only (exports, `[FWD]`s); never edit.
- `docs/19_visual_debug.md` — direct read.
- `docs/08_coordinate_system.md` — summary sections (Point3WithWidth/z are mm floats; no unit conversion on this path).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-11`.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: discharges 247's `[FWD to packet 249]` (blanket tool rejection → per-capture `ToolColorUnavailable`; `SilhouetteUnsupportedForTap` lifted for LayerFinalization only); packet 250 consumes `PostpassCaptureShape::WholePrint`, `render_silhouette_composite_styled`, and the (tap, view, color mode) grouping; packet 248's model-source-only tool rejection (if 248 landed first) is removed by the same validator edit, and its pinning test `model_silhouette_tool_coloring_still_rejected` is retired alongside 247's (AC-N2) — the two packets' validator changes compose in either order because 248 touches only the gcode arm's acceptance and 249 only the model-source rejection.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --test visual_debug_postpass_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Capture shape + bundle ACs | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Renderer ACs + 247 regression (same binary) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Validation staging incl. retargeted 247 pin | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_agent_determinism_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Existing top-down postpass path byte-stable (AC-10) | FACT pass/fail |
| `cargo test -p slicer-runtime --test visual_debug_postpass_tap_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Postpass capture machinery unregressed | FACT pass/fail |
| `cargo xtask check-literals` | Watched-type literal gate (LayerCollectionIR/PrintEntity fixtures) | FACT exit code |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` | Closure gates | FACT pass/fail |

## Step Completion Expectations

- Step 1 (capture shape) must land before Step 4 (assembly) — the silhouette branch calls `run_postpass_taps` with the new parameter. Steps 2–3 (renderer) and Step 1 are order-independent.
- The wedge end-to-end steps need fresh guest WASMs: run `cargo xtask build-guests --check` before attributing any Step 4/5 failure to the packet's code.
- The retargeting of `silhouette_tool_coloring_rejected_role_accepted` (Step 4) and the validator lift must land in the same step — a window where validation accepts tool coloring but the old test still pins rejection is a red gate, not a defect.

## Context Discipline Notes

- `crates/pnp-cli/src/visual_debug.rs` (~2200 lines) and `crates/slicer-runtime/src/visual_debug_render.rs` (~2300 lines, larger post-247): ranged reads anchored on symbols named in `design.md` only.
- The plan file is ~811 lines: never read in full.
- Renderer fixtures construct `LayerCollectionIR`/`PrintEntity` directly: `LayerCollectionIR` has a manual `impl Default` (use `..Default::default()`); `PrintEntity` deliberately has no `Default` — follow the existing suites' `// exhaustive:` waiver pattern (docs/21).
