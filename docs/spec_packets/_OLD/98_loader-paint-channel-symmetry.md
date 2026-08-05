---
status: implemented
packet: 98
task_ids: [TASK-248]
---

# 98_loader-paint-channel-symmetry

## Goal

Hoist the existing sub-facet-stroke hex decoder block at `crates/slicer-model-io/src/loader.rs:1237-1295` (currently bound to the `paint_color` Material channel and the `paint_supports` channels) into a private helper `decode_strokes_for_channel(hex: &str, semantic: PaintSemantic, tri_verts: &[Vec3], byte_offset: usize) -> Vec<PaintStroke>` that takes the channel's `PaintSemantic` as a parameter, then call it for all four 3MF paint channels (`paint_color` → `PaintSemantic::Material(ToolIndex)`, `paint_supports` → `PaintSemantic::SupportEnforcer` / `SupportBlocker` per the existing hex sub-decoding, `paint_seam` → `PaintSemantic::SeamEnforcer` / `SeamBlocker`, `paint_fuzzy_skin` → `PaintSemantic::FuzzySkin(Flag)`), so that 3MFs encoding paint at sub-facet granularity via TriangleSelector subdivision in OrcaSlicer get decoded symmetrically across all four channels — not just the two already covered. Sub-facet strokes parsed from any channel are subsequently normalized into `paint_data.layers[*].facet_values` by `host:mesh_segmentation` (P94's wiring); this packet ensures the strokes actually arrive at the kernel for every channel. Add per-channel stroke tests to `crates/slicer-model-io/tests/model_loader_tdd.rs`, exercising the existing cube fixtures (`cube_fuzzyPainted.3mf` carries `paint_fuzzy_skin` strokes; a synthetic single-channel fixture may be authored for `paint_seam` if no current fixture exercises it).

## Problem Statement

3MF files emitted by OrcaSlicer encode paint at sub-facet granularity using TriangleSelector subdivision, with the per-facet partition expressed as a hex string. Each of the four documented 3MF paint channels — `paint_color`, `paint_supports`, `paint_seam`, `paint_fuzzy_skin` — uses the SAME hex grammar, but `crates/slicer-model-io/src/loader.rs:1119-1295` only decodes the hex sub-facet partition for `paint_color` (Material semantic) and `paint_supports` (SupportEnforcer / SupportBlocker semantic). The `paint_seam` and `paint_fuzzy_skin` channels parse whole-triangle facet assignments but silently drop their sub-facet hex partitions.

Consequences observable on the cherry-picked fixtures:

- `resources/cube_fuzzyPainted.3mf` carries `paint_fuzzy_skin` strokes that were ENCODED by OrcaSlicer at sub-facet granularity. Pre-P98, those strokes are dropped, so any cube test asserting on per-face fuzzy_skin coverage fails or relies on coincidental whole-triangle paint.
- Any future `paint_seam` per-edge enforcer painting (a documented OrcaSlicer feature) loads as empty.

The fix is mechanical: hoist the existing hex-decoding block at `loader.rs:1237-1295` into a private helper that takes the channel's `PaintSemantic` as a parameter, then call it for each of the four channels. The 3MF format is unchanged; the loader behavior on channels currently working stays byte-identical; the two previously-empty channels now produce strokes.

## Architecture Constraints

- The hex grammar is identical across the four paint channels per OrcaSlicer's TriangleSelector.cpp; SUMMARY confirms.
- The four-channel-to-semantic mapping table lives at the call site (loader.rs), not in the helper. The helper is channel-agnostic.
- Behavior preservation invariant: cube_4color SHA stays identical to post-P97 baseline (AC-9). Wedge byte-identical (AC-8). Cube_fuzzyPainted SHA may differ (AC-10 — fuzzy_skin strokes now respected).
- Negative-hex grace invariant: malformed hex returns a structured error rather than panicking (AC-N1).

## Data and Contract Notes

- IR contracts: none touched.
- WIT boundary: none.
- Determinism: hex decoder is pure functions; same input → same output.

## Locked Assumptions and Invariants

- **Identical hex grammar across channels**: OrcaSlicer parity (confirmed via SUMMARY).
- **Cube_4color and wedge byte-identical**: regression contract (AC-8, AC-9).
- **Cube_fuzzyPainted SHA may differ**: expected behavior change (AC-10).

## Risks and Tradeoffs

- **Risk: a stroke on `paint_seam` decodes to an unexpected `PaintSemantic` because the 2-bit encoding for SeamEnforcer/Blocker isn't documented in our codebase yet.** Mitigation: SUMMARY against TriangleSelector.cpp surfaces the encoding; if unclear, the closure log documents the chosen encoding as a tentative parity assumption and flags for verification with the OrcaSlicer team.
- **Risk: the helper's `Result` return type breaks an existing caller** that expected `Vec<PaintStroke>`. Mitigation: workspace check + per-test gates.
