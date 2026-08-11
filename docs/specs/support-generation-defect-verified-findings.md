# Support Generation Defect — Verified Findings and Packet Brief

Status: verified (2026-08-11, visual-debug session)
Audience: a zero-context session that must (a) re-verify every claim below,
(b) author the fix packet under `docs/spec_packets/`, and (c) close it.

## 1. Problem statement

PNP's support generation is broken: with `enable_support: true`, the slicer
emits "support" (scan-line or grid-MST lines) **inside the model's own
cross-section on every layer**, instead of supporting pieces under overhangs
that rise from the build plate. This was demonstrated against
`tmp/SupportTest.stl` (a pillar at X −10..0, Y 0..20, Z 0..30 that widens
into a ledge at X 0..20, Y 0..20 above Z≈25) and compared against
`tmp/SupportTest_Normal_Orca.gcode` / `tmp/SupportTest_Tree_Orca.gcode`
(OrcaSlicer ground truth, sliced from `tmp/SupportTest.3mf`).

Orca reference behavior (verified from the G-code): support first appears at
Z≈4.71 (the ledge overhang onset is at Z≈25 in model coords; the Orca file
was sliced with a different orientation/placement — the support lattice
appears at X≈103–126.5, Y≈88.2–111.8, **outside** the pillar footprint),
rises from the build plate, and never fills the model interior.

PNP behavior (verified): support lines at X −10..0 (the pillar interior) on
layers 10/24/30 (Z 2.2/5.0/6.0) — all below the overhang — and nothing under
the ledge.

## 2. Architecture context (read before touching code)

PNP splits support into two tiers (docs/01_system_architecture.md
§PrePass::SupportGeometry and §Layer::Support):

1. **Planner tier (canonical primary path).** `support-planner`
   (`modules/core-modules/support-planner/`) runs at `PrePass::SupportGeometry`
   as a WASM guest. It detects overhang facets (`detect_overhang_facets`,
   z-normal threshold 45°), creates contact points at their centroids, and
   propagates them top-down through a per-layer Prim MST
   (`plan_for_object`), emitting `SupportPlanIR` (per-(layer, object, region)
   `branch_segments`). `tree-support` (`modules/core-modules/tree-support/`)
   consumes `SupportPlanIR` at `Layer::Support` via
   `PaintRegionLayerView::support_plan_segments_for` and emits the planned
   segments directly.
2. **Filler tier (fallback).** `traditional-support`
   (`modules/core-modules/traditional-support/`) is a per-layer rectilinear
   scan-line filler that never reads `SupportPlanIR` (its manifest declares
   reads `SliceIR`/`SurfaceClassificationIR`/`PaintRegionIR` only). It fills
   every `region.polygons()` expolygon gated only on paint policy +
   `SliceRegionView::needs_support()`.

Claim resolution: both support modules hold the `support-generator` claim;
`support_type` config selects the winner (`crates/slicer-scheduler/src/
execution_plan.rs`, `support_generator_preferred_module_id` — values
starting with `tree`/`hybrid` select `com.core.tree-support`, everything
else `com.core.traditional-support`). Absent key → traditional-support.

## 3. Verified root causes (three, compounding)

### RC-1 — Planner: lone propagated nodes emit nothing (primary defect)

`plan_for_object`'s emission loop
(`modules/core-modules/support-planner/src/lib.rs`, the `for layer_rev in
(0..top).rev()` loop) emits `branch_segments` only for:

- MST edges between two surviving nodes (the `for (a_idx, b_idx, _) in
  &mst_edges` loop), and
- fresh contact tips: `if node.dist_to_top != 0 || origin_contacts_emitted[i]
  { continue; }` — the "lone fresh contact" arm.

A node that has **no surviving MST edge and `dist_to_top > 0`** (a lone
propagated node) emits nothing. This is exactly what happens after branch
merging: nodes converge at `max_move_xy = tan(branch_angle) * effective_layer_height`
per layer (≈0.2 mm/layer at 45°/0.2 mm) and merge when closer than
`DEFAULT_MERGE_DISTANCE_MM = 0.8` (the `drop[*a.max(b)] = true` merge pass).
Once a column's nodes merge into one survivor, the survivor has no MST edge
and `dist_to_top > 0` → **the column vanishes mid-air instead of continuing
to the build plate**. Orca's `TreeSupport` emits a vertical column for lone
nodes; this port has an emission gap.

**Visual evidence (verified):** with `support_type: "tree(auto)"`, the
`PrePass::SupportGeometry` tap shows a branch segment at layer 124
(Z=25.0, the contact layer just below the ledge) — a diagonal from
(6.67, 13.33) to (13.33, 6.67) inside the ledge region — but **no branch
geometry at layers 100/50/0** (Z 20.2/10.2/0.2). The plan covers only ~20
layers below the contact, never reaching the plate. `Layer::Support` at 124
emits the planned diagonal (plan consumption works), but at 100/50/0 it
falls back to the grid-MST filler.

### RC-2 — Filler tier: fills the whole region polygon, never clipped to overhang

Both fallback fillers iterate `region.polygons()` and scan-fill every
expolygon:

- `modules/core-modules/traditional-support/src/lib.rs` — `run_support`
  (line ~140: `let polygons = region.polygons();`) → `fill_expolygon`
  (line 183).
- `modules/core-modules/tree-support/src/lib.rs` — `run_support` (line ~145)
  → `fill_expolygon_tree` (line 210).

Eligibility is gated only on paint policy + `needs_support()`; the fill is
never clipped to `region.overhang_areas()`. So once eligible, the entire
model cross-section gets "support" lines on every layer.

**Visual evidence (verified):** default config (traditional-support wins),
`Layer::Support` tap: 9 horizontal scan lines at X −10..0, Y 2..18 (2 mm
spacing = `line_width/density` = 0.4/0.2) on layers 10/24/30 — the pillar
interior, on every layer, including layers far below the overhang.

### RC-3 — `needs_support` is hardcoded `true` at the WIT boundary

`crates/slicer-wasm-host/src/marshal/in_.rs`, `sliced_region_to_data`:
`needs_support: true` (line 410) for **every** slice region on **every**
layer. The `SurfaceClassificationIR`-derived eligibility that
`SliceRegionView::needs_support`'s contract describes
(`crates/slicer-sdk/src/views.rs` field doc; `crates/slicer-sdk/src/traits.rs`
`run_support` doc: "Default (no paint) → generate support iff
`SliceRegionView::needs_support()` is true (the SurfaceClassificationIR-derived
eligibility flag)") is never computed. `SlicedRegion` (the IR struct,
`crates/slicer-ir/src/slice_ir.rs`) has **no** `needs_support` field to
marshal — the marshaller had nothing to read even if it wanted to.

The per-layer overhang data the marshaller **does** already clip into
`SliceRegionData.overhang_areas` / `overhang_quartile_polygons`
(in_.rs lines 353–393, from `SurfaceClassificationIR.overhang_quartile_polygons`,
produced by `annotate_overhangs` in `crates/slicer-core/src/algos/
overhang_annotation.rs` — PNP's analog of Orca's `detect_overhangs` diff)
is ignored for eligibility.

### RC-4 — Zero-width contact tips

`tapered_radius` (`modules/core-modules/support-planner/src/lib.rs:1298`)
returns 0 at `dist_to_top == 0` (tip). Fresh contact tips are emitted as
zero-width segments (the lone-contact arm, `width = tapered_radius(...) * 2.0`
= 0). Consequence: the visual-debug `filled_areas` render of
`PrePass::SupportGeometry` at layer 124 fails with
`no usable Point3WithWidth.width for filled_areas; refusing to infer a bead
width` — and a printed zero-width tip would be a degenerate extrusion.

## 4. Canonical Orca reference (for the packet's parity framing)

`OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp`:

- `detect_overhangs` (the function at ~line 1370): support areas per layer =
  `diff(layerm_polygons, expand(lower_layer_polygons, lower_layer_offset))`
  — the parts of each layer that overhang the layer below, with
  `lower_layer_offset` derived from the angle threshold
  (`lower_layer.height / tan(threshold_rad)`). Support is **never** the full
  layer cross-section.
- `bottom_contact_layers_and_layer_support_areas` (~line 2592) +
  `generate_base_layers` (~line 2953): the overhang contact areas are
  propagated **down to the build plate**, producing support on every layer
  from plate to overhang.

`OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`:
`TreeSupport::drop_nodes` — the reference for the planner's propagation;
lone nodes continue as vertical columns (the PNP port's emission gap).

## 5. Fix direction (for the packet author — verify each before implementing)

1. **RC-1 (primary):** in `plan_for_object`'s emission loop, emit a
   vertical column segment for lone surviving nodes with `dist_to_top > 0`
   (no MST edge, not merged away). Compare against Orca `drop_nodes`
   behavior. This makes the planner tier produce plate-to-overhang columns.
2. **RC-2:** clip the fallback fill to `region.overhang_areas()` (the
   already-marshalled per-layer overhang bands) in both
   `traditional-support` and `tree-support` fallback paths. Note: this alone
   cannot produce plate-to-overhang columns — `traditional-support` has no
   downward propagation by design (documented in its module doc-comment) —
   so the planner tier (RC-1) is the canonical primary fix; RC-2 makes the
   fallback at least not fill the model interior.
3. **RC-3:** derive `needs_support` per region from the clipped overhang
   bands at the marshalling boundary (or add the field to `SlicedRegion` and
   populate it from `SurfaceClassificationIR`), so non-overhang layers are
   not support-eligible at all.
4. **RC-4:** floor the tip width at a minimum bead width (e.g. the module's
   `line_width_mm` or a small fraction of it) so contact tips are printable
   and renderable.

## 6. Reproduction (exact commands, verified working)

Prerequisites: `cargo xtask build-guests` (guest WASMs are stale after any
`crates/slicer-schema/wit/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`,
`crates/slicer-core/**`, or module `src/**` edit — a stale guest fails typed
instantiation or silently runs old code; run `cargo xtask build-guests --check`
first). Then `cargo build -p pnp-cli --bin pnp_cli`.

Request files (already on disk under `tmp/`):

- `tmp/support-config.json` — `{"layer_height": 0.2, "enable_support": true,
  "support_density": 20.0, "support_angle": 0.0, "support_speed": 60.0,
  "line_width": 0.4}` (no `support_type` → traditional-support wins).
- `tmp/support-config-tree.json` — same + `"support_type": "tree(auto)"`
  (tree-support wins).
- `tmp/visual-debug-support.json` — model mode, `tmp/SupportTest.stl`,
  config `tmp/support-config.json`, module_dirs `["modules/core-modules"]`,
  layers [10, 24, 30], taps `["Layer::Support"]`, visualizations
  `["filament_lines", "filled_areas"]`.
- `tmp/visual-debug-tree.json` — same but config
  `tmp/support-config-tree.json`, layers [10, 125, 130].
- `tmp/visual-debug-tree2.json` — tree config, taps
  `["PrePass::SupportGeometry", "Layer::Support"]`, layers [10, 125].
- `tmp/visual-debug-tree5.json` — tree config, taps
  `["PrePass::SupportGeometry"]`, layers [124], visualizations
  `["filament_lines"]` only (the `filled_areas` variant fails with the
  RC-4 zero-width error — that failure is itself evidence).
- `tmp/visual-debug-gcode.json` / `tmp/visual-debug-gcode2.json` — gcode
  mode on `tmp/SupportTest_Normal_Orca.gcode`, `gcode_line_width_mm: 0.4`,
  layers [30] / [31, 32, 33, 34], taps `["final_gcode"]`.

Run:

```bash
cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug \
  --request tmp/visual-debug-support.json --output target/vd-support --overwrite
cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug \
  --request tmp/visual-debug-tree2.json --output target/vd-tree2 --overwrite
cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug \
  --request tmp/visual-debug-tree5.json --output target/vd-tree5 --overwrite
cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug \
  --request tmp/visual-debug-gcode2.json --output target/vd-gcode-n2 --overwrite
cargo run -q -p pnp-cli --bin pnp_cli -- support-preview \
  --input tmp/SupportTest.stl --output target/support-preview.json \
  --config tmp/support-config-tree.json --module-dir modules/core-modules
```

Then read `target/<bundle>/manifest.json` first (per
`.claude/skills/visual-debug/SKILL.md`), then the PNGs. Note: the
`PrePass::SupportGeometry` tap's `typed_capture` is `null` in the manifest
because `serde_json::to_value` cannot serialize
`HashMap<SupportGeometryKey, Vec<ExPolygon>>` (struct keys) — the PNGs are
the evidence for that tap; `Layer::Support`'s `typed_capture` IS populated
and carries the exact path coordinates.

## 7. Verification checklist for the zero-context session

1. `cargo xtask build-guests --check` clean (rebuild if stale).
2. Reproduce RC-2: `target/vd-support/manifest.json` — `Layer::Support`
   typed captures at layers 10/24/30 contain 9 support paths each at
   X −10..0, Y 2..18 (2 mm spacing), i.e. support inside the pillar on
   non-overhang layers.
3. Reproduce RC-1: `target/vd-tree2` — `PrePass::SupportGeometry` PNGs show
   the dark-blue outline (RGB 0,90,140) but no teal branch segments
   (RGB 0,200,160) at layers 10/125; `Layer::Support` at 125 emits 149
   grid-MST paths (the fallback), proving `SupportPlanIR` was empty there.
   `target/vd-tree5` — layer 124 shows a cyan diagonal branch (the contact
   segment) and `filled_areas` fails with the zero-width error (RC-4).
4. Reproduce RC-3: `crates/slicer-wasm-host/src/marshal/in_.rs:410` is
   `needs_support: true` unconditionally; `SlicedRegion` has no
   `needs_support` field.
5. Orca ground truth: `target/vd-gcode-n2` — support lattice appears at
   layer 32 (Z=4.714) outside the pillar, absent on pillar-only layers.
6. After the fix: re-run 2–4 and confirm (a) planner branches reach the
   build plate, (b) fallback fill is clipped to overhang areas, (c) no
   support on non-overhang layers, (d) tips have non-zero width.

## 8. Out of scope (do not let the packet grow into these)

- Raft generation (`SupportPlanIR.raft_plan` geometry — packet 124 owns it).
- Support interface layers / `support_interface_bottom_layers` (packets
  210a/210b/211 own the planner's interface work).
- Orca numerical parity of branch radii/positions — the planner is a
  documented algorithmic-shape port, not a parity port
  (`docs/specs/_OLD/support-modules-orca-port.md`).
- The `support_type` claim-resolution mechanism itself (works as designed).
- G-code emission of support (roles `SupportMaterial`/`SupportInterface`
  already exist in `ExtrusionRole`).

## 9. Files touched by the fix (expected, re-verify at implementation time)

- `modules/core-modules/support-planner/src/lib.rs` — lone-node emission
  (RC-1), tip-width floor (RC-4).
- `modules/core-modules/traditional-support/src/lib.rs` — overhang clip
  (RC-2).
- `modules/core-modules/tree-support/src/lib.rs` — overhang clip in the
  fallback path (RC-2).
- `crates/slicer-wasm-host/src/marshal/in_.rs` — `needs_support` derivation
  (RC-3); possibly `crates/slicer-ir/src/slice_ir.rs` (`SlicedRegion` field)
  and the WIT `slice-region-view` if the field is added end-to-end.
- Tests: `modules/core-modules/support-planner/tests/`,
  `modules/core-modules/traditional-support/tests/`,
  `modules/core-modules/tree-support/tests/`,
  `crates/slicer-wasm-host/tests/contract/` (slice-region-view contract
  tests pin `needs_support` behavior).

Guest-WASM staleness rule (CLAUDE.md): after editing any of the above,
`cargo xtask build-guests` before re-running visual-debug or tests.
