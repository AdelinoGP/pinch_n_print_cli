---
status: implemented
packet: 251-visual-debug-silhouette-seam-overlays
task_ids:
  - TASK-455
  - TASK-456
  - TASK-457
---

# 251-visual-debug-silhouette-seam-overlays

## Goal

Bring seam glyphs to silhouettes (plan D18): the existing isolated form `overlays: ["seams"]` becomes legal on model-source silhouette specs with its exact 1.1.0 meaning (a `FAINT_BASE`-gray silhouette base plus the legend-1.1.0 red filled-circle seam glyphs at projected `(x_or_y, z)`, every rendered seam mirrored into the entry's `overlay_events`), the seam event shape gains an additive optional `z` field that is absent from all 1.0/1.1 serialization, a new 1.2.0-only option `composited_overlays: ["seams"]` draws the same glyphs onto the colored silhouette base image, and the full R9 validation matrix fails closed with named errors — retiring packet 247's `deny_unknown_fields` interim pin and retargeting packet 248's gcode-overlay pin.

## Problem Statement

Seam placement is a Z-sensitive decision (seam towers, layer-to-layer seam drift) that a side view makes inspectable at a glance, but seams have no silhouette story after packets 247–250: `SeamPlanIR` is rejected as a silhouette *tap* (no polygon geometry — plan fact 5), and `overlays: ["seams"]` is illegal on any non-`diagnostic_overlay` kind today (`validate_request`'s overlays rule, `crates/pnp-cli/src/visual_debug.rs`). Plan D18 resolves this with two forms: the isolated 1.1.0-style overlay image on a faint silhouette base, and a new `composited_overlays` option drawing glyphs on the colored base — both sourced from the blackboard-committed `SeamPlanIR` (`chosen_candidate.point` is a mm `Point3WithWidth` carrying `z` — fact 11), with the seam event's manifest mirror gaining `z` additively.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This path never converts units: `SeamPlanEntry.chosen_candidate.point` is a mm `Point3WithWidth` (fact 11); glyph centers go through `Projector::project(x_or_y_mm, z_mm)` — the single-owner rule, no new transform.
- Serialization lock: `OverlayEvent` is `Serialize`-only with `#[serde(tag = "event", rename_all = "snake_case")]`; the new `Seam.z` must be `Option<f32>` + `skip_serializing_if = "Option::is_none"`, and every non-silhouette construction site passes `None` — 1.0/1.1 manifests (including `overlay_events` on gcode and top-down model bundles) stay byte-identical, pinned by AC-5 on serialization output.
- Legend lock: the seam glyph is `GlyphKind::Circle` in `overlay_palette::SEAM` (`[220, 0, 0]`), `GLYPH_HALF_PX` (6) × `resolution_scale` — identical to `event_glyph`'s 1.1.0 mapping; `LEGEND_VERSION` stays `"1.1.0"` (fills got a schema bump in 247; glyphs are unchanged in meaning).
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): `SeamPlanIR`/`SeamPlanEntry`/`SeamPosition` fixture literals use `..Default::default()` where `Default` exists (`SeamPosition`, `Point3WithWidth` derive it; `SeamPlanEntry` — verify; fall back to the `// exhaustive:` waiver); `VisualizationOptions` gains a 6th field and is already watched — test literals keep `..Default::default()`.

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
