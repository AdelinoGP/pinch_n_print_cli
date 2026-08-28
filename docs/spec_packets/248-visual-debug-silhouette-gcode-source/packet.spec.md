---
status: implemented
packet: 248-visual-debug-silhouette-gcode-source
task_ids:
  - TASK-446
  - TASK-447
  - TASK-448
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
plan_source: docs/specs/visual-debug-silhouette-side-views-plan.md (Packet Queue row #2)
---

# Packet Contract: 248-visual-debug-silhouette-gcode-source

## Goal

Extend the `silhouette` visualization kind (packet 247, schema 1.2.0) to the standalone `.gcode` source: per-layer Z slabs derived from `;Z:` markers (plan D12), per-move flow-derived extrusion widths via the rectangular inversion `w = Δe × A_filament / (L × h)` with `gcode_line_width_mm` as the explicit fallback (D13/D14/D16), an unclassified-`;TYPE:` interval class painted first (D15), the W3 slab warning, the R8 fail-closed width error, and the gcode-source half of D17 (palette-only `color_by: "tool"`), replacing packet 247's interim `SilhouetteUnsupportedOnGcodeSource` rejection.

## Scope Boundaries

Everything lands on the standalone-gcode path (`crates/pnp-cli/src/visual_debug_gcode.rs` plus the gcode arm of `run_visual_debug`/`validate_request` in `crates/pnp-cli/src/visual_debug.rs`), with one small `slicer-runtime` edit promoting packet 247's interval-union helper to a shared `pub` fn. Model-source silhouettes, `PostPass::*` taps, typed `Move.e` inversion, and seam overlays stay untouched — owned by packets 249/250/251. `filled_areas` keeps its no-E-derivation rule and `gcode_line_width_mm` requirement byte-for-byte. Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 247 (`247-visual-debug-silhouette-core`, currently `draft` — FORWARD-DEP; every step consuming its exports states "packet 247 implemented" as a precondition; the swarm executes queue order).
- Unblocks: nothing in the queue (packets 249/251 depend on #1 only; packet 250 depends on #1 #2 #3).
- Activation blockers: packet 247 not yet `implemented`.

## Acceptance Criteria

- **AC-1. Given** a `schema_version: "1.2.0"` gcode-source request with empty `taps`, a two-layer fixture carrying `;LAYER_CHANGE`/`;Z:0.2`/`;Z:0.4` markers, `;TYPE:` roles, and a `; filament_diameter = 1.75` config comment, and one `{"type": "silhouette"}` visualization (no `options.view`), **when** `run_visual_debug` succeeds, **then** the bundle contains exactly one silhouette PNG named `images/gcode_silhouette_front.png`, and its manifest entry has `"source": "gcode"`, `"tap": ""`, `"visualization": "silhouette"`, `"view": "front"`, a `"layers_rendered"` list of inclusive `{"start", "end"}` ranges equal to the resolved layer indices, a `"gcode_parser_version"` string, a `"world_bounds_mm"` object, and **no** `"layer_index"` or `"layer_z"` key. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- gcode_silhouette_bundle_entry_shape 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** a fixture whose absolute-mode E values are authored from the rectangular flow formula (`Δe = L × w × h / A_filament`, `w = 0.5`, `h = 0.2`, `d = 1.75`) and a second fixture identical except `M83` relative mode, **when** widths are derived, **then** `silhouette_segment_width_mm` recovers `0.5` for every extruding move in both fixtures (within f64 round-trip of the authored values, no fallback consulted), and the rendered interval for a horizontal move spans the move's X extent inflated by `w/2` at each end. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- flow_width_roundtrip_absolute_and_relative_modes 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** an adaptive-height fixture with markers `;Z:0.2`, `;Z:0.5`, `;Z:0.65`, **when** rendered as a silhouette, **then** the three layers' rectangles span slabs `[0, 0.2]`, `[0.2, 0.5]`, `[0.5, 0.65]` — verified by decoded-pixel row extents at three distinct slab heights — and the first slab bottom is `0`, never a marker-delta guess. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- adaptive_z_markers_derive_per_layer_slabs 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a fixture with **no** `filament_diameter` comment and a request supplying `gcode_line_width_mm: 0.42`, **when** rendered, **then** the bundle succeeds and every extruding move uses the `0.42` fallback width (pixel extent matches the fallback-inflated interval, distinct from a control render with `gcode_line_width_mm: 0.84`). | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- fallback_width_used_when_underivable 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** a fixture with an extruding move before any `;TYPE:` marker overlapping a later `;TYPE:`-classified move in X, **when** rendered, **then** the unclassified run paints `GCODE_UNCLASSIFIED_COLOR` (`[128, 128, 128]`) where not overlapped and the overlap paints the role class's color (unclassified is FIRST in paint order, occluded by every role class), and the existing bundle-wide unclassified warning is present. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- unclassified_class_paints_first_and_warns 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** a fixture whose layer 1 marker repeats layer 0's Z (`;Z:0.2` twice), whose layer 2 marker decreases (`;Z:0.1`), and whose layer 3 has no `;Z:` marker at all, **when** rendered, **then** the entry's warnings contain W3 text naming each offending layer index and the offending Z values (or the absence of a marker), those three layers contribute no pixels, and `layers_rendered` excludes them. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- w3_nonmonotonic_duplicate_and_markerless_layers_skip_with_warning 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** an absolute-mode fixture containing a mid-file `G92 E0` reset followed by extruding moves with post-reset E values, **when** parsed, **then** the post-reset segments have `is_extrusion: true` with the correct positive `e_delta_mm` (the reset synchronizes the parser's carried E position instead of producing a huge negative delta), and no unsupported-construct warning is emitted for the `G92 E` line. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- g92_e_reset_synchronizes_e_position 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** the same gcode text, layer selection, view, and canvas, **when** the silhouette renders twice, **then** the two PNG byte vectors are identical and the two warning lists are equal element-for-element. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- gcode_silhouette_is_deterministic 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** two silhouette requests over the same fixture, one selecting a layer subset and one selecting all layers, **when** both bundles render, **then** both entries record byte-identical `world_bounds_mm` (whole-file horizontal extent; vertical `[0, max ;Z:]` — framing never depends on selection). | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- gcode_silhouette_framing_is_selection_independent 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10. Given** a 1.2.0 gcode-source silhouette request with `options.color_by: "tool"` over a fixture with a `T1` tool change, **when** the bundle renders, **then** the PNG is named `images/gcode_silhouette_front_tool.png`, tool intervals paint `ToolColors::default()` colors in ascending tool index order (tool 1 occludes tool 0 on overlap), and the entry carries `"color_by": "tool"`, `"tool_color_source": "palette"`, with the manifest's `tool_palette` table present (palette-only — a standalone `.gcode` resolves no config). | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- gcode_silhouette_tool_coloring_palette_only 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-11 (docs).** `docs/19_visual_debug.md` gains a gcode-silhouette subsection stating the flow-derived width rule (why silhouette differs from `filled_areas`), the deposited-width caveat (low `flow_factor` moves render thin; the silhouette is not a width-measurement tool), the D14 fallback rule, and the W3 warning. | `rg -q 'flow-derived' docs/19_visual_debug.md && rg -q 'deposited' docs/19_visual_debug.md && rg -q 'W3' docs/19_visual_debug.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a fixture with **no** `filament_diameter` config comment, at least one extruding move on a selected layer, and a request with **no** `gcode_line_width_mm`, **when** rendered, **then** the command fails closed with `VisualDebugError::SilhouetteWidthUnderivable` whose Display names the missing `filament_diameter` comment and the `gcode_line_width_mm` remedy, and no bundle directory content is written. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- width_underivable_without_diameter_fails_closed 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a fixture containing an `M200` volumetric-extrusion command before extruding moves and a request with no `gcode_line_width_mm`, **when** rendered, **then** the command fails closed with `VisualDebugError::SilhouetteWidthUnderivable` whose Display names `M200` volumetric extrusion; the same fixture with `gcode_line_width_mm: 0.42` succeeds using the fallback for the poisoned moves. | `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd -- m200_volumetric_poisons_flow_derivation 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** (a) a 1.2.0 gcode-source `diagnostic_overlay` request with `options.overlays: ["seams"]` and (b) a 1.2.0 gcode-source silhouette spec carrying an `options.overlays` key, **when** validated, **then** (a) is rejected with `ValidationError::OverlayUnsupportedOnGcode` naming `seams` (R10, unchanged) and (b) is rejected with `ValidationError::InvalidOverlays` (overlays applies to diagnostic_overlay; packet 251 owns silhouette seam overlays). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- gcode_silhouette_overlay_rejections_unchanged 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N4. Given** the tree after this packet, **when** searched, **then** `SilhouetteUnsupportedOnGcodeSource` and packet 247's interim test `silhouette_on_gcode_source_rejected_interim` are absent from `crates/`, and a validation test named `silhouette_on_gcode_source_accepted` exists and passes in their place. | `! rg -q 'SilhouetteUnsupportedOnGcodeSource' crates/ && ! rg -q 'silhouette_on_gcode_source_rejected_interim' crates/ && rg -q 'silhouette_on_gcode_source_accepted' crates/pnp-cli/tests/visual_debug_validation_tdd.rs && cargo test -p pnp-cli --test visual_debug_validation_tdd -- silhouette_on_gcode_source_accepted 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N5. Given** a 1.2.0 gcode-source silhouette request whose `taps` names `PrePass::SupportGeometry`, **when** validated, **then** it is rejected with `ValidationError::SilhouetteUnsupportedForTap` whose reason states the standalone gcode source has no pipeline taps. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- gcode_silhouette_rejects_named_taps 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N6. Given** a 1.2.0 **model**-source silhouette spec with `options.color_by: "tool"`, **when** validated, **then** it is still rejected with `ValidationError::InvalidColorBy` (packet 247's blanket rejection is narrowed to the model source only; packet 249 lifts it for tool-carrying model captures), while `color_by: "role"` on a gcode silhouette is accepted. | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- model_silhouette_tool_coloring_still_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N7. Given** a 1.2.0 gcode-source silhouette request with `frame: "plate"`, **when** validated, **then** it is rejected with `ValidationError::SilhouettePlateFrameUnsupported` (R4 is source-independent). | `cargo test -p pnp-cli --test visual_debug_validation_tdd -- gcode_silhouette_plate_frame_rejected 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p pnp-cli --test visual_debug_gcode_silhouette_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/visual-debug-silhouette-side-views-plan.md` — normative plan; long (~811 lines): ranged reads only (facts 9/12/13, §4.4 D12–D16, D17's gcode clause, §5–§7 for this packet).
- `docs/spec_packets/247-visual-debug-silhouette-core/packet.spec.md` + `design.md` — the exports this packet builds on (read-only; never edit that directory).
- `docs/19_visual_debug.md` — current user-facing contract (232 lines pre-247; direct read).

## Doc Impact Statement (Required)

- `docs/19_visual_debug.md` — extend the packet-247 silhouette section with a "Standalone G-code silhouettes" subsection: `;Z:` slab rule, flow-derived width rule and why it differs from `filled_areas` (which keeps its no-E-derivation rule), rectangular-model caveat on foreign (stadium-model) files, deposited-width caveat, `gcode_line_width_mm` fallback semantics, W3 warning, palette-only tool coloring — `rg -q 'flow-derived' docs/19_visual_debug.md && rg -q 'deposited' docs/19_visual_debug.md && rg -q 'W3' docs/19_visual_debug.md`

## Deviations from Design (recorded at closure)

- **D-A (packet-authoring defect).** `design.md` assigns `GcodeRenderError::SilhouetteWidthUnderivable` to Step 4 but its `map_gcode_error` arm to Step 5, while Step 5'''s precondition is "Step 4 complete". This is circular: `map_gcode_error` in `crates/pnp-cli/src/visual_debug.rs` matches `GcodeRenderError` exhaustively, so the variant cannot compile without its arm. Resolved by a narrow grant letting the Step 4 worker land `VisualDebugError::SilhouetteWidthUnderivable`, its Display arm, and the one `map_gcode_error` arm. Lesson for future packets: when a new error variant crosses an exhaustive match in another file, the variant and its mapping arm are one atomic step.
- **D-B.** `Segment` gained `pub source_line: usize` beyond `design.md`'''s declared surface. Required by the packet'''s own rule that `M200` poisons flow derivation *from its line onward*, evaluated per rendered segment — unanswerable without a per-segment source line. `cargo xtask check-literals` stays at 0 violations.
- **D-C.** `gcode_silhouette_slabs` is `pub` (design wrote a bare `fn`), so `slab_derivation_w3_cases` — an integration test — can assert on the slab map directly, as its exit condition demands.
- **D-D (Step 3 no-op).** Packet 247 had already landed `union_silhouette_intervals` as `pub fn` in `crates/slicer-runtime/src/visual_debug_render.rs` and re-exported it from `crates/slicer-runtime/src/lib.rs`. Per `design.md`'''s "keep the landed name, do not create two unions" clause, Step 3 became verify-only; no `slicer-runtime` edit was made. Verified at closure: exactly one `fn union_silhouette_intervals` definition exists in `crates/`.
- **D-E (closure-review fix).** `gcode_silhouette_slabs` originally carried `prev_z: Option<f64>` and ran the `z <= prev` check only inside `if let Some(prev)`, exempting the first `;Z:` marker from monotonicity. A `;Z:0` or negative first marker therefore produced a degenerate/inverted slab that rendered every move at width 0 with no W3 warning while still appearing in `layers_rendered` — fail-open against the packet'''s own doctrine, uncovered by any AC. Fixed red-first: `prev_z` initializes to `0.0` and one comparison governs all layers. A `!z.is_finite()` guard was added alongside it, because `;Z:` is parsed with `parse::<f64>()` which accepts `NaN`/`inf`, and `NaN` slips through `z <= prev_z`. Pinned by `first_z_marker_must_be_positive`.

## Dogfood Findings (post-closure, real CLI on real files)

The packet closed on tests using inline synthetic fixtures. A follow-up dogfood
session ran the real `pnp_cli visual-debug` binary against real G-code — our own
emitter's `calicat_a.gcode` (13,662 lines, 173 layers) and a foreign OrcaSlicer
file (`multi_color_cube.orca.gcode`) — and surfaced four gaps no acceptance
criterion covered. All four are fixed; each carries a regression test in
`crates/pnp-cli/tests/visual_debug_gcode_silhouette_tdd.rs`.

- **DF-1 — fallback widths were silent.** Rendering the foreign file (no
  `; filament_diameter` comment) with `gcode_line_width_mm` succeeded with an
  EMPTY warning list, so every width came from the fallback while the image
  looked exactly as authoritative as a flow-derived one. `render_gcode_silhouette`
  now tallies fallback vs. flow-derived segments and emits one warning per
  `FallbackCause` (missing diameter comment / `M200` poisoning) naming the count,
  the cause, and the consequence. Pin: `fallback_width_use_is_warned`.
- **DF-2 — the warning channel was 100% noise.** Measured on `calicat_a.gcode`:
  234 warnings, ALL 234 `M73` progress commands, zero W3. A genuine W3
  skip-with-warning — this feature's own fail-closed signal — would have been
  invisible. `parse_gcode` now keeps the first occurrence verbatim (line number
  intact) and collapses later duplicates of the same construct into one summary
  carrying the true total, ordered by first occurrence. Pin:
  `repeated_unsupported_constructs_collapse`.
- **DF-3 — extreme aspect ratios render unreadably.** The 4-layer foreign file
  (239 mm wide x 0.8 mm tall, 300:1) rendered as a ~1-pixel line; the fixed 2 mm
  viewport margin was 2.5x the object's entire height. Deliberately NOT fixed by
  non-uniform axis scaling: distorting the axes would misrepresent deposited
  width, which is the one thing this view exists to show honestly. Framing,
  margin, and `Projector` are untouched (packet 247's pinned bounds stay valid);
  instead a warning fires when the vertical data extent covers <1% of canvas
  height, suggesting a layer subset or the `side` view. Pin:
  `extreme_aspect_ratio_is_warned`.
- **DF-4 — request-schema papercut.** `source` is internally tagged `kind` while
  `visualizations[]` entries use `type`; the bare serde failure
  (`missing field kind`) explained neither. The wire format is unchanged;
  `run_cli` now augments the parse error to name the valid `source.kind` values
  and note the deliberate key difference.

DF-3's threshold and DF-2's collapse are both deterministic and allocation-ordered
(no `HashMap` on either path), preserving the packet's determinism invariant.

## Residual Risk (recorded at closure)

- Multi-tool `filament_diameter` lists clamp to the last entry for out-of-range tool indices: a foreign file with more tools than diameters renders with the last diameter rather than failing. Documented in `docs/19_visual_debug.md`.
- R8 is evaluated lazily per rendered move, so a layer selection that avoids poisoned moves succeeds where a wider selection fails. Deliberate — it keeps partial inspection of a damaged file possible.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
