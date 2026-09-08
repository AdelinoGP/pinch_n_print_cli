# Requirements: 248-visual-debug-silhouette-gcode-source

## Packet Metadata

- Grouped task IDs: `TASK-446`, `TASK-447`, `TASK-448` (new rows; re-derived 2026-08-27 as max+1..max+3 over `docs/07_implementation_status.md`, `docs/specs/*.md`, all local `docs/spec_packets/*/task-map.md` — highest TASK-445, packet 247 — and `origin/master`-only packets 243–246, highest TASK-356; the completion-gate worker adds these rows to docs/07)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 247 ships the silhouette kind for model-source blackboard taps and rejects the standalone `.gcode` source with the interim `SilhouetteUnsupportedOnGcodeSource` variant, with an explicit `[FWD to packet 248]` handing this packet its removal. A reported defect usually arrives as a `.gcode` file, not a model+config pair, so the side view is incomplete without this source. The gcode path has everything the plan needs already parsed — `;Z:` markers, `;TYPE:` roles, per-move E deltas under `M82`/`M83`, tracked tools — but the parser discards the per-move Δe after computing `is_extrusion`, knows nothing of `filament_diameter`/`M200`/`G92 E`, and has no slab or interval machinery. This packet closes plan §4.7 step 3 (D12–D16, W3, R8, R10) as one coherent slice.

## In Scope

- `Segment` gains the per-move E delta (`e_delta_mm`); `ParsedGcode` gains the parsed `; filament_diameter = …` values and an `M200`-seen flag; the parser handles `G92 E<val>` resets (grounding-surfaced correctness fix: today a mid-file `G92 E0` makes the next absolute-mode move's Δe hugely negative, silently misclassifying real extrusion as travel on the existing renders too).
- Slab derivation per D12: `[previous ;Z: marker, z]` per parsed layer, first `[0, z]`; W3 warning naming layers with duplicate/non-monotonic/absent markers, those layers skipped (never a guessed slab).
- Flow width per D13/D16: `w = Δe × A_filament / (L × h)` (rectangular inversion of our emitter's model), `A_filament = π × (d/2)²` from the file's own config comment (comma-separated multi-tool values indexed by the segment's tool, clamped to the last entry), `h` = the layer's slab height, `Δe` from the parser's E-mode handling.
- Fallback per D14: request `gcode_line_width_mm` used for underivable moves only; no fallback + an underivable rendered move → fail closed (R8) via new `GcodeRenderError::SilhouetteWidthUnderivable` mapped to new `VisualDebugError::SilhouetteWidthUnderivable`, naming the missing datum (no `filament_diameter` comment; `M200` volumetric). Underivability is evaluated lazily, per rendered extruding segment, in parse order.
- Composite render `render_gcode_silhouette` in `visual_debug_gcode.rs`: per-(layer, class) interval unions (segment horizontal extent inflated by its own `w/2` per endpoint), unclassified class first (D15, `GCODE_UNCLASSIFIED_COLOR`), remaining role classes in ascending lexicographic role order, tool classes ascending tool index under `color_by: "tool"` (palette-only); rectangles drawn back-to-front through the shared `Projector`; whole-file, selection-independent framing (horizontal from parsed bounds per view, vertical `[min(0, markers), max ;Z:]`, existing margin).
- Shared interval union: promote packet 247's private union helper to `pub fn union_silhouette_intervals` in `crates/slicer-runtime/src/visual_debug_render.rs`, re-exported from `slicer-runtime` (if 247 landed it under another public name, reuse that name — the AC greps accept name-resolution-equivalent forms); the gcode path calls it rather than owning a drifting copy (this module's history: it once owned a second Projector copy that drifted).
- Validation staging: gcode-source silhouette accepted under 1.2.0 (`SilhouetteUnsupportedOnGcodeSource` variant, Display arm, and the 247 interim test removed/replaced); non-empty `taps` + gcode silhouette rejected via `SilhouetteUnsupportedForTap`; the packet-247 blanket `InvalidColorBy` for silhouette+tool narrowed to the model source only.
- Bundle assembly in `run_visual_debug`'s gcode arm: silhouette branch before the per-layer loop; filenames `gcode_silhouette_{view}.png` / `gcode_silhouette_{view}_tool.png`; entry `tap: ""` (the existing empty-taps gcode convention), `layers_rendered` = drawn layers (selected minus W3-skipped) as maximal inclusive ranges, no `layer_index`/`layer_z` keys, `gcode_parser_version` set, warnings = parse warnings + W3 + unclassified summary.
- D17 deviation (recorded): this packet owns the gcode-source palette-only tool-coloring case rather than packet 249 (the queue note recommended 249). Rationale: (a) `Segment.tool` is already parsed and the gcode top-down path already supports palette-only tool coloring, so the tool class is the same class-key switch this packet's interval extraction needs anyway; (b) assigning it to 249 would make 249 depend on this packet's renderer, contradicting the approved queue's "249 depends on #1 only"; (c) validator staging stays consistent and hidden-dependency-free: 248 narrows the blanket rejection to model-source silhouettes, 249 removes it for tool-carrying model captures.
- `docs/19_visual_debug.md` gcode-silhouette subsection (see Doc Impact in `packet.spec.md`).

## Out of Scope

- Model-source silhouettes, `PostPass::LayerFinalization`/`PostPass::GCodeEmit`, and typed `Move.e` position-differencing inversion (packets 249/250; the plan's §9 emitter round-trip test targets D11's typed inversion and belongs to 250 — this packet's round-trip fixtures author E values from the closed-form formula instead).
- Seam overlays on silhouette and `composited_overlays` (packet 251); R10's existing rejections merely stay pinned here.
- `filled_areas`/`filament_lines` behavior on any source — `filled_areas` keeps `gcode_line_width_mm` mandatory and never derives width from E (D13's scoping decision); its validation and tests stay byte-untouched.
- Stadium cross-section modeling, generator sniffing, arcs (`G2`/`G3`), `M200` support (it remains a poison marker, not a modeled mode), and any change to the existing top-down gcode renders.
- The `;Z:`-less layer problem beyond W3 skip-with-warning (no Z inference of any kind).

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — ~811 lines; ranged reads only: §3 facts 9/12/13, §4.4, §4.5 D17 (gcode clause), §5–§8.
- `docs/spec_packets/247-visual-debug-silhouette-core/` — read `packet.spec.md` + `design.md` only (exports ledger, `[FWD]` obligations); never edit.
- `docs/19_visual_debug.md` — direct read (232 lines pre-247).
- `docs/08_coordinate_system.md` — summary sections only (this path is mm-only; it never constructs IR types).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-11`.
- Negative: `AC-N1` through `AC-N7`.
- Cross-packet impact: removes packet 247's `SilhouetteUnsupportedOnGcodeSource` + interim test per 247's `[FWD]`; narrows 247's silhouette tool-coloring rejection to the model source (247's model-based pinning test `silhouette_tool_coloring_rejected_role_accepted` stays green; 249 retargets it); packet 250 will reuse `union_silhouette_intervals` and this packet's docs caveats; packet 251 inherits AC-N3's pinned overlay rejections.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | All parser/width/slab/render/bundle ACs | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --test visual_debug_validation_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Validation staging incl. 247 regressions | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_gcode_renderer_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | Existing gcode renders unbroken by parser changes (G92/Segment field) | FACT pass/fail |
| `cargo test -p slicer-runtime --test visual_debug_silhouette_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result"` | 247's composite suite green after the union-helper promotion | FACT pass/fail |
| `cargo xtask check-literals` | Watched-type literal gate (Segment grows to 6 fields) | FACT exit code |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` | Closure gates | FACT pass/fail |

## Step Completion Expectations

- Steps 1–2 (parser + derivation helpers) are independent of packet 247; Steps 3–5 require packet 247 implemented (they consume `SilhouetteView`, the union helper, and the 1.2.0 validation/manifest surface). Do not reorder Step 4's validation changes after Step 5's bundle assembly — the assembly tests assume the acceptance staging is in place.
- The G92 fix (Step 1) changes `parse_gcode` behavior consumed by the existing top-down renders; the existing `visual_debug_gcode_renderer_tdd` suite must be green at the end of Step 1, not deferred to closure.

## Context Discipline Notes

- `crates/pnp-cli/src/visual_debug.rs` is ~2200 lines and `crates/pnp-cli/src/visual_debug_gcode.rs` ~1650: ranged reads only, anchored on the symbols named in `design.md`.
- The plan file is ~811 lines: never read in full; the sections listed above suffice.
- Fixture G-code is authored inline in tests (small strings); never load real `.gcode` artifacts from disk.
