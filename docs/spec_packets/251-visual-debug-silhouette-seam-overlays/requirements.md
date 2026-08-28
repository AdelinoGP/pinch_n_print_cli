# Requirements: 251-visual-debug-silhouette-seam-overlays

## Packet Metadata

- Grouped task IDs: `TASK-455`, `TASK-456`, `TASK-457`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Seam placement is a Z-sensitive decision (seam towers, layer-to-layer seam drift) that a side view makes inspectable at a glance, but seams have no silhouette story after packets 247–250: `SeamPlanIR` is rejected as a silhouette *tap* (no polygon geometry — plan fact 5), and `overlays: ["seams"]` is illegal on any non-`diagnostic_overlay` kind today (`validate_request`'s overlays rule, `crates/pnp-cli/src/visual_debug.rs`). Plan D18 resolves this with two forms: the isolated 1.1.0-style overlay image on a faint silhouette base, and a new `composited_overlays` option drawing glyphs on the colored base — both sourced from the blackboard-committed `SeamPlanIR` (`chosen_candidate.point` is a mm `Point3WithWidth` carrying `z` — fact 11), with the seam event's manifest mirror gaining `z` additively.

## In Scope

- `OverlayEvent::Seam` (`crates/slicer-runtime/src/visual_debug_style.rs`) gains `z: Option<f32>` with `#[serde(skip_serializing_if = "Option::is_none")]`; every existing construction site (`collect_overlay_events`'s `Perimeter`/`SeamPlan` arms in `crates/slicer-runtime/src/visual_debug_render.rs`) passes `z: None` so 1.0/1.1 serialization stays byte-identical; only the silhouette seam path emits `Some(z)`. Pattern sites (`draw_overlay_events` in `visual_debug_render.rs`, the gcode glyph loop in `crates/pnp-cli/src/visual_debug_gcode.rs`) take `..` rest patterns.
- A silhouette seam-event helper: filter `SeamPlanIR.entries` by `region_key.global_layer_index` ∈ the rendered layer set, in entries source order (the existing `collect_overlay_events` SeamPlan-arm convention); per-view horizontal coordinate = `point.x` (`front`) / `point.y` (`side`); missing seam plan (`blackboard.seam_plan()` `None`) fails closed with an error naming the seam plan.
- Renderer: an isolated seam-overlay entry (247's composite rectangles recolored `FAINT_BASE` + `GlyphKind::Circle` glyphs in `overlay_palette::SEAM` at `Projector::project(h, z)`, `GLYPH_HALF_PX × resolution_scale`) and a composited variant (glyph pass over the colored base before PNG encode), both returning the rendered events; 247/249's existing entry points delegate unchanged (no signature churn, byte-equivalence preserved).
- `VisualizationOptions` gains `#[serde(default)] pub composited_overlays: Option<Vec<String>>`; `ImageEntry` gains `composited_overlays: Option<Vec<String>>` (`skip_serializing_if`), emitted on composited silhouette entries only.
- R9 validation, all named `ValidationError::InvalidOverlays` unless stated: silhouette `overlays`/`composited_overlays` accept only `"seams"`; `composited_overlays` is silhouette-only, model-source-only (gcode → `OverlayUnsupportedOnGcode`, matching R10), 1.2.0-only (message names `"1.2.0"`, incl. the 1.0.0 stray-key path), non-empty; specs collapsing to one (tap, view, color mode) group must agree on both overlay options.
- Assembly: isolated images at `{sanitized_tap}_silhouette_{view}_overlay_seams.png`, emitted once per (tap, view) — the faint base ignores color mode, so role and tool groups share one isolated image; composited glyphs land on the group's existing base image (no extra file); entries carry `overlay`/`overlay_events` (isolated) or `composited_overlays`/`overlay_events` (composited).
- Pin retirements/retargets (cross-packet fallout rule): 247's `composited_overlays_not_accepted_by_247` retired (AC-N7's named-matrix replacement); 248's `gcode_silhouette_overlay_rejections_unchanged` retargeted (AC-N3 — its arm (b)'s blanket `InvalidOverlays` becomes the seams-aware split).
- Docs: `docs/19_visual_debug.md` seam-overlay subsection (AC-8).

## Out of Scope

- Any non-seam glyph kind on silhouettes (travel, retractions, z_hops, tool_changes — plan §8; AC-N1 pins the rejection).
- Seams on the gcode source (fact 11: final G-code carries no seam marker) and on top-down views (behavior frozen; `LayerCollection`-family seams stay `OverlayUnsupportedForTap`).
- `SeamPlanIR` as a silhouette *tap* (stays `SilhouetteUnsupportedForTap` — R2).
- Populating `z` on top-down seam events (would change 1.1.0 bytes — locked to `None` there).
- Legend bump — the glyph, color, and meaning are legend 1.1.0's exactly; `LEGEND_VERSION` stays `"1.1.0"` (247's fills-not-glyphs precedent).
- Per-overlay mode objects or a `composite_seams` boolean (plan D18's rejected request shapes).
- `scored_candidates` rendering — only `chosen_candidate` draws.

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — ~811 lines; ranged reads only (facts 5/11, D18, §5/§6/§7/§8).
- Packet 247 `packet.spec.md` + `design.md` (exports, AC-N7 pin); packet 248 `packet.spec.md` AC-N3 only.
- `docs/19_visual_debug.md` — isolated-overlay section (1.1.0 semantics to mirror); range-read post-247.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`. AC-5 is the serialization-compat pin for the additive `z`; AC-6 protects 247/249 byte-equivalence; AC-7 (tool-colored base) assumes packet 249 in-tree — guaranteed by queue order, stated as a step precondition.
- Negative: `AC-N1` through `AC-N8` (every R9 arm, the §8 travel-glyph exclusion, both cross-packet retirements, the missing-seam-plan fail-closed path).
- Cross-packet impact: retires 247's `composited_overlays_not_accepted_by_247`; retargets 248's `gcode_silhouette_overlay_rejections_unchanged`; must not perturb 249's styled-composite byte-equivalence pins or 250's GCodeEmit entries (glyphs never draw on GCodeEmit/LayerFinalization bases unless their group requests them — the mechanism is tap-agnostic and needs no per-tap code).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | seam filtering/coords, determinism, base-entry byte-equivalence (AC-2, AC-6) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --test visual_debug_seam_overlay_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | bundle forms, z mirror, legacy serialization, missing-plan fail-closed (AC-1/3/4/5/7, N8) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | full R9 matrix + retirements (AC-N1..N7) | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_overlays_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | top-down overlay behavior frozen after the `Seam` field addition | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate on new fixtures | FACT exit code |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` | closure gates | FACT pass/fail |

## Step Completion Expectations

- The `OverlayEvent::Seam` field change (Step 1) must land with its pattern-site fixes in the same step — the or-patterns in `draw_overlay_events` and the gcode glyph loop stop compiling otherwise.
- Steps retiring 247/248 pins state "packet 247/248 implemented" as preconditions; until then the absence greps are vacuously true (queue-order artifact, annotated in the ACs).
- AC-7's fixture needs a tool-carrying tap (`PostPass::LayerFinalization`) — its step declares "packet 249 implemented" as a precondition even though the packet-level dependency is #1 only.

## Context Discipline Notes

- `crates/pnp-cli/src/visual_debug.rs` and `crates/slicer-runtime/src/visual_debug_render.rs` are ~2.2k lines each (pre-247, larger after) — symbol-anchored ranged reads only (`VisualizationOptions`, the overlays block in `validate_request`, the silhouette branch, `collect_overlay_events`, `render_stage_capture_styled`'s `OverlayIsolated` arm).
- The wedge e2e tests need fresh guest WASMs: `cargo xtask build-guests --check` before blaming failures; the wedge pipeline commits a `SeamPlanIR` via `modules/core-modules/seam-planner-default`.
