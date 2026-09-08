# Design: 248-visual-debug-silhouette-gcode-source

## Controlling Code Paths

- Primary code path: `validate_request` → `run_visual_debug` gcode arm (`crates/pnp-cli/src/visual_debug.rs`) → `parse_gcode` → **new** `render_gcode_silhouette` (`crates/pnp-cli/src/visual_debug_gcode.rs`) → shared `Projector` + local raster buffer → `ImageEntry`/manifest emission.
- Neighboring tests/fixtures: `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` (inline-string gcode fixtures, decoded-pixel assertions), `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` (library-call validation harness), `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs` (gcode-source bundle pattern), packet 247's `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs` (union-helper behavior pins).
- OrcaSlicer comparison: none — PnP-native tool; the only Orca-facing fact is D16's documented rectangular-vs-stadium caveat (a docs line, not a parity obligation).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This module-specific corollary (pinned by `visual_debug_gcode.rs`'s own module doc): the gcode path works entirely in plain `f64` millimeters and never constructs IR types — no `mm_to_units` round-trip may appear in this packet.
- Projector single-owner rule (archived spec, binding): the silhouette rectangles project both corners through `Projector::project(x_or_y_mm, z_mm)`; no new world→pixel transform. The `Projector`'s y-flip renders larger Z toward the top.
- Fail-closed doctrine: material is never silently dropped or guessed — every skip is a named warning (W3, unclassified summary), every unrecoverable datum a named error (R8). This mirrors the parser's existing "never approximate what we don't fully understand" stance.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): `Segment` grows to 6 named fields and becomes watched; new test literals need `..` FRU (it derives no `Default` — add `#[derive(Default)]`? No: `PointMm` has no `Default`; use an exhaustive waiver or a small fixture helper fn in tests).

## Code Change Surface

- Selected approach: keep the gcode silhouette entirely inside the self-contained gcode module (a new composite entry point beside `render_gcode_visual_debug_styled`), with `visual_debug.rs` owning validation staging, the silhouette branch in the gcode arm, and manifest emission — mirroring packet 247's split (validation/assembly in pnp-cli, pixel math near the geometry source). One `slicer-runtime` edit shares 247's interval union.

- Exact surface, per file:
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
    - `Segment` gains `pub e_delta_mm: f64` (0.0 for non-E moves; the signed per-move delta for E-carrying moves). Blast radius (verified 2026-08-27): exactly **one** construction site — the `layers[li].segments.push(Segment { … })` in `parse_gcode` — and **zero** test struct-literals (tests read segments via `parse_gcode`).
    - `ParsedGcode` gains `pub filament_diameters_mm: Vec<f64>` (empty when the file carries no `; filament_diameter = …` comment) and `pub volumetric_extrusion_line: Option<usize>` (1-indexed source line of the first `M200`, `None` otherwise). Blast radius: one construction site (the `ParsedGcode { … }` return in `parse_gcode`); no test literals.
    - `parse_gcode`: new `parse_filament_diameter_comment` beside `parse_printable_area_comment` (exact key match `filament_diameter`, comma-separated positive finite f64 list, else `None`); an `"M200"` match arm recording `volumetric_extrusion_line` (no more unsupported-construct warning for it — it is now understood, as a poison marker); a `"G92"` match arm parsing an `E<val>` token and assigning `last_e = val` (a `G92` without `E` keeps the existing unsupported-construct warning; X/Y/Z offsets stay unsupported). The `G0`/`G1` arm stores the computed `e_delta` into the pushed `Segment`.
    - New slab derivation `fn gcode_silhouette_slabs(parsed: &ParsedGcode) -> (BTreeMap<i64, (f64, f64)>, Vec<String>)` — walks `parsed.layers` in order carrying the last accepted `;Z:` marker; layer with `layer_z: Some(z)` and `z > prev` yields slab `(prev, z)` (first accepted marker yields `(0.0, z)`); `z <= prev` (duplicate/non-monotonic) or `layer_z: None` yields no slab plus one W3 warning per offending layer naming `layer_index` and the Z values (or "no ;Z: marker"). A skipped layer does **not** advance the carried marker.
    - New width derivation `pub fn silhouette_segment_width_mm(e_delta_mm: f64, length_mm: f64, slab_height_mm: f64, filament_diameter_mm: f64) -> f64` — the closed-form `Δe × (π·(d/2)²) / (L × h)`; pub for direct unit pinning (AC-2).
    - New `pub fn render_gcode_silhouette(gcode_text: &str, layer_indices: &[i64], view: slicer_runtime::SilhouetteView, canvas_width: u32, canvas_height: u32, fallback_width_mm: Option<f64>, color_by: ColorBy) -> Result<GcodeSilhouetteOutput, GcodeRenderError>` with `pub struct GcodeSilhouetteOutput { pub parser_version: String, pub warnings: Vec<String>, pub png_bytes: Vec<u8>, pub width: u32, pub height: u32, pub world_bounds_mm: ViewportBoundsMm, pub layers_rendered: Vec<i64> }`. Flow: parse; `NoRenderableMoves` check as today; slabs + W3; viewport = horizontal from `parsed.bounds_mm` (X extent for `Front`, Y for `Side`) × vertical `[0.0.min(first slab bottom), max accepted ;Z:]`, through the local `viewport_bounds` margin helper (framing reads whole-file data only — selection-independent); per selected layer with a slab: per extruding segment, width = derived (segment-tool-indexed diameter, clamped to the last entry) or `fallback_width_mm` when underivable (no diameters parsed, or the segment's source position is at/after `volumetric_extrusion_line`); underivable + no fallback → `Err(SilhouetteWidthUnderivable)` naming the datum; interval = `[min(h0, h1) − w/2, max(h0, h1) + w/2]` on the view's horizontal axis; classes keyed unclassified-first then role-string ascending (`ColorBy::Role`) or ascending tool index (`ColorBy::Tool`, `ToolColors::default()` — palette-only); per (layer, class) union via the shared `slicer_runtime::union_silhouette_intervals`; rectangles emitted ascending layer index → class paint order → ascending interval start, both corners through `Projector`, filled into the RGB buffer; `encode_png`.
    - `GcodeRenderError` gains `SilhouetteWidthUnderivable { detail: String }` with Debug/Display naming the missing datum and the `gcode_line_width_mm` remedy.
  - `crates/slicer-runtime/src/visual_debug_render.rs` + `crates/slicer-runtime/src/lib.rs`
    - Promote packet 247's private interval-union helper to `pub fn union_silhouette_intervals(intervals: Vec<(f32, f32)>) -> Vec<(f32, f32)>` (sorted endpoint sweep, touch merges, exact comparison) and re-export it; 247's composite renderer delegates to it. If 247 landed the helper already-public under another name, keep the landed name and re-export — do not create two unions.
  - `crates/pnp-cli/src/visual_debug.rs`
    - `ValidationError`: **remove** `SilhouetteUnsupportedOnGcodeSource` and its Display arm (247's `[FWD]`); `validate_request`'s silhouette checks: gcode source no longer rejected; non-empty `taps` + gcode silhouette → `SilhouetteUnsupportedForTap { tap, reason }` (reason: no pipeline taps on a standalone gcode source); the blanket silhouette `color_by: "tool"` → `InvalidColorBy` check becomes model-source-only (gcode accepts tool, palette-only; `tool_color_source: "filament"` on a gcode silhouette resolves to the palette exactly like the existing gcode top-down rule).
    - `VisualDebugError` gains `SilhouetteWidthUnderivable(String)` with a Display arm; `map_gcode_error` maps the new `GcodeRenderError` variant to it.
    - Gcode arm of `run_visual_debug`: silhouette branch **before** the existing per-layer visualization loop (the mixing ban guarantees disjointness): resolve view from the single silhouette group (one plane per bundle, 247's AC-N9 check is source-independent); call `render_gcode_silhouette` once per color mode requested (role and/or tool specs collapse into one group per mode); emit `ImageEntry` per group — `source: "gcode"`, `tap: ""`, `visualization: "silhouette"`, `view`, `layers_rendered` from `GcodeSilhouetteOutput.layers_rendered` compressed to maximal inclusive `LayerRangeEntry` ranges, `layer_index: None`, `layer_z: None`, `typed_capture: None`, `gcode_parser_version`, `world_bounds_mm`, warnings; filenames `images/gcode_silhouette_{view}.png` and `…_{view}_tool.png`; `tool_palette` emitted via `tool_palette_entries(&ToolColors::default())` when a tool group rendered.
  - Tests: new `crates/pnp-cli/tests/visual_debug_gcode_silhouette_tdd.rs`; edits to `crates/pnp-cli/tests/visual_debug_validation_tdd.rs` (remove/replace the interim test, add AC-N3/N5/N6/N7 pins).
  - Docs: `docs/19_visual_debug.md` (gcode-silhouette subsection).

- Rejected alternatives and reasons:
  - Routing gcode silhouettes through `render_silhouette_composite` by synthesizing `StageCapture`/`CapturedIr` rows — the gcode path deliberately never constructs IR types (module doc pin), and the plan's §9 scope places this path in `visual_debug_gcode.rs`.
  - A second private interval-union copy in the gcode module — this exact module previously owned a drifting Projector copy; the shared helper is ~15 lines of `slicer-runtime` visibility instead.
  - Deriving a slab for markerless/non-monotonic layers from `layer_height` config comments or neighbor interpolation — D12 rejects uniform heights; a guessed slab is the misleading-image failure mode. Skip + W3.
  - Treating `M200` as unsupported-construct noise while still deriving widths — linear-E inversion of volumetric E values silently produces wrong widths; poison-marking is the honest reading.
  - Assigning the gcode tool-coloring case to packet 249 (the queue note's recommendation) — creates a hidden 249→248 renderer dependency; full rationale recorded in `requirements.md` §In Scope.
  - Sentinel tap names (`"gcode"`) on the silhouette entry — `tap: ""` is the existing empty-taps gcode-bundle convention; inventing a pseudo-tap adds a second convention for no consumer gain.

## Files in Scope (read + edit)

- `crates/pnp-cli/src/visual_debug_gcode.rs` — parser fields, slab/width derivation, composite renderer; the packet's largest edit.
- `crates/pnp-cli/src/visual_debug.rs` — validation staging, error variants, gcode-arm silhouette branch, manifest emission.
- `crates/slicer-runtime/src/visual_debug_render.rs` + `crates/slicer-runtime/src/lib.rs` — union-helper promotion + re-export only.
- Tests: new `crates/pnp-cli/tests/visual_debug_gcode_silhouette_tdd.rs`; `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`.
- Docs: `docs/19_visual_debug.md`.

Justification for exceeding three primaries: two production files carry the split the plan's §9 mandates (gcode math vs validation/assembly); the runtime pair is a two-line visibility change; each step stays ≤3 edits.

## Read-Only Context

- `docs/spec_packets/247-visual-debug-silhouette-core/packet.spec.md` + `design.md` — exports (`SilhouetteView`, `LayerRangeEntry`, validation variants, filename scheme) — never edit that directory.
- `crates/slicer-runtime/src/visual_debug_style.rs` — `GCODE_UNCLASSIFIED_COLOR`, `gcode_role_color`, `ToolColors`, `tool_palette_color` region only.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` — fixture/decoding patterns only; edit only if a G92-behavior pin there conflicts (none found in grounding).
- `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs` — gcode bundle harness pattern only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not applicable (no parity), never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `crates/slicer-gcode/**` — the emitter/serializer are not touched; the round-trip fixtures author E values from the closed form (typed-emitter round-trips belong to packet 250).
- Packet directories 247/249/250/251 (beyond 247's two read-only files) — never create or modify files there.
- `docs/07_implementation_status.md` — worker-dispatch updates only at the completion gate.

## Expected Sub-Agent Dispatches

- Question: after packet 247 lands, what is the exact name/visibility of its interval-union helper in `crates/slicer-runtime/src/visual_debug_render.rs`?; scope: that file; return: `FACT`; purpose: Step 3 (reuse-or-promote decision).
- Question: does 247's `silhouette_tool_coloring_rejected_role_accepted` test use a model-source request (expected) or a gcode-source one?; scope: `crates/pnp-cli/tests/visual_debug_validation_tdd.rs`; return: `FACT`; purpose: Step 4 (if gcode-source, retarget it here and note the change for 249).
- Question: run the step's `cargo test -p pnp-cli --test <file>` and report pass/fail with failing test names; scope: single test binary tee'd to `target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure; purpose: every step.
- Question: `cargo xtask check-literals` exit code after new test fixtures; scope: repo root; return: `FACT`; purpose: Steps 1/3/5.

## Data and Contract Notes

- IR/manifest contracts: 1.2.0 gcode silhouette entries follow 247's manifest shape (`view`, `layers_rendered`, absent `layer_index`/`layer_z`); 1.0/1.1 gcode bundles stay byte-frozen (247's AC-8 already pins them — this packet adds no serialization change to legacy paths). `world_bounds_mm` on a gcode silhouette entry carries the X–Z or Y–Z plane; legality rests on 247's mixing ban + one-plane-per-bundle + per-entry `view`.
- WIT boundary: none — no IR, WIT, or guest-facing types; everything is CLI/runtime host-side.
- Determinism/scheduler constraints: rectangle emission ascending layer → class order (unclassified, then role strings ascending, or tools ascending) → interval start; W3 warnings in layer order; `BTreeMap` slab keys; parse order is the only iteration source — no `HashMap` anywhere on this path.

## Locked Assumptions and Invariants

- Slabs come only from accepted `;Z:` markers: `[last accepted marker, z]`, first `[0, z]`; a layer without an honest slab renders nothing and warns (W3). No layer-height config comment, interpolation, or marker-delta guess ever produces a slab.
- Width is deposited width from the rectangular inversion; `filled_areas` never derives width from E (unchanged, pinned by its untouched tests); the fallback is used only for underivable moves, never preferred over a derivable one.
- `Δe <= 0` and zero-XY-displacement moves are never drawn (the parser already excludes them from `segments`/`is_extrusion`); `G92 E` synchronizes the carried E position; `M200` poisons flow derivation from its source line onward.
- Framing is whole-file and selection-independent; one silhouette plane per bundle (247 invariant, inherited).
- The gcode silhouette filename stem is `gcode_silhouette_{view}[_tool].png` and the entry's `tap` is `""` — later packets extend, never repurpose, this scheme.

## Risks and Tradeoffs

- The G92 fix changes `is_extrusion` classification for existing top-down gcode renders on files with mid-print `G92 E0` (previously misdrawn as travel). This is a correctness fix, but any self-captured baseline pinning the old wrong pixels must be re-baselined to canonical-correct output (Test Discipline rule) — Step 1 runs the full `visual_debug_gcode_renderer_tdd` suite to surface any.
- f64→f32 interval endpoints and exact-comparison unions can leave sub-pixel seams between nearly-touching runs; honest (the gap exists in the data), same tradeoff 247 accepted.
- Multi-tool `filament_diameter` lists clamp to the last entry for out-of-range tools — a foreign file with more tools than diameters renders with the last diameter rather than failing; the derived width is still from the file's own data. Documented in docs/19.
- Lazy R8 evaluation means a selection that avoids poisoned moves succeeds; a later selection can fail. Deliberate: fail-closed applies to what is actually rendered, keeping partial inspection possible on damaged files.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5 — bundle assembly + end-to-end manifest tests)
- Highest-risk dispatch and required return format: Step 1's full `visual_debug_gcode_renderer_tdd` regression run after the parser changes; `FACT pass/fail` + failing names.

## Open Questions

- `[FWD to packet 250]` `union_silhouette_intervals` (or 247's landed name) and `silhouette_segment_width_mm`'s closed form are yours to reuse for D11; do not fork a third union or a second rectangular formula. The docs/19 deposited-width and rectangular-model caveats this packet writes cover GCodeEmit too — extend, don't duplicate.
- `[FWD to packet 249]` This packet narrows the silhouette tool-coloring rejection to the model source; your validator change removes that model-source rejection for tool-carrying captures and retargets `silhouette_tool_coloring_rejected_role_accepted` (247's `[FWD]` stands, unchanged in destination).
- `[FWD to packet 251]` AC-N3 pins `overlays` on a gcode silhouette → `InvalidOverlays` and gcode `diagnostic_overlay` seams → `OverlayUnsupportedOnGcode`; when `composited_overlays` lands, keep the gcode source rejected with a named error (plan R9) and retarget these pins.
- No `[BLOCK]` items.
