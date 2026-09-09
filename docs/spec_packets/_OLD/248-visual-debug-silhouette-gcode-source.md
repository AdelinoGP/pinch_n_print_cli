---
status: implemented
packet: 248-visual-debug-silhouette-gcode-source
task_ids:
  - TASK-446
  - TASK-447
  - TASK-448
---

# 248-visual-debug-silhouette-gcode-source

## Goal

Extend the `silhouette` visualization kind (packet 247, schema 1.2.0) to the standalone `.gcode` source: per-layer Z slabs derived from `;Z:` markers (plan D12), per-move flow-derived extrusion widths via the rectangular inversion `w = Δe × A_filament / (L × h)` with `gcode_line_width_mm` as the explicit fallback (D13/D14/D16), an unclassified-`;TYPE:` interval class painted first (D15), the W3 slab warning, the R8 fail-closed width error, and the gcode-source half of D17 (palette-only `color_by: "tool"`), replacing packet 247's interim `SilhouetteUnsupportedOnGcodeSource` rejection.

## Problem Statement

Packet 247 ships the silhouette kind for model-source blackboard taps and rejects the standalone `.gcode` source with the interim `SilhouetteUnsupportedOnGcodeSource` variant, with an explicit `[FWD to packet 248]` handing this packet its removal. A reported defect usually arrives as a `.gcode` file, not a model+config pair, so the side view is incomplete without this source. The gcode path has everything the plan needs already parsed — `;Z:` markers, `;TYPE:` roles, per-move E deltas under `M82`/`M83`, tracked tools — but the parser discards the per-move Δe after computing `is_extrusion`, knows nothing of `filament_diameter`/`M200`/`G92 E`, and has no slab or interval machinery. This packet closes plan §4.7 step 3 (D12–D16, W3, R8, R10) as one coherent slice.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This module-specific corollary (pinned by `visual_debug_gcode.rs`'s own module doc): the gcode path works entirely in plain `f64` millimeters and never constructs IR types — no `mm_to_units` round-trip may appear in this packet.
- Projector single-owner rule (archived spec, binding): the silhouette rectangles project both corners through `Projector::project(x_or_y_mm, z_mm)`; no new world→pixel transform. The `Projector`'s y-flip renders larger Z toward the top.
- Fail-closed doctrine: material is never silently dropped or guessed — every skip is a named warning (W3, unclassified summary), every unrecoverable datum a named error (R8). This mirrors the parser's existing "never approximate what we don't fully understand" stance.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): `Segment` grows to 6 named fields and becomes watched; new test literals need `..` FRU (it derives no `Default` — add `#[derive(Default)]`? No: `PointMm` has no `Default`; use an exhaustive waiver or a small fixture helper fn in tests).

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
