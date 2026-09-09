---
status: implemented
packet: 241-support-agg-rasterizer
task_ids:
  - TASK-419
  - TASK-420
  - TASK-421
  - TASK-422
  - TASK-423
  - TASK-424
  - TASK-425
  - TASK-426
  - TASK-427
  - TASK-428
---

# 241-support-agg-rasterizer

## Goal

Port the canonical `SupportGridPattern` AGG rasterization path (`SupportMaterial.cpp`) into the
traditional planner's area propagation as `support_area_rasterizer = agg` (canonical, OPT-IN),
keeping the current propagate-without-growth semantic as `legacy_semantic` — which is the
DEFAULT (binding human decision, 2026-09-03; recorded in §Human Validation Gate) — with
before/after wall-leakage (collision freedom) and column-continuity (coverage) measurements as
the acceptance gate (plan §3 Rulings 7/8, §7 E1/E2).

## Problem Statement

The gap register's G-07 row was filed with a "needs-research first" premise: that the canonical
`SupportGridPattern` AGG rasterizer changes support outline shape but not termination, coverage,
or collision freedom. **Ruling 7 of the governing plan refuted that premise** with upstream
history: `fb7b995050` reworked grid projection onto the AGG rasterizer precisely to stop supports
leaking through or around object walls (a collision-freedom defect), via ≤8×8 oversampling plus
expansion restricted inside the cell; `a95607d7bf` fixed support columns missing abruptly when
going down (a coverage/termination defect) caused by grid-extraction contour filtering. The
research question is settled — this packet is a PORT.

PnP's traditional planner (`modules/core-modules/traditional-support-planner/src/lib.rs`,
long — ranged reads only; port of the `SupportMaterial.cpp` orchestration) implements
only the *semantic* half:
propagate-without-growth carry, trimmed per layer at `support_object_xy_distance`. It has no
byte-grid projection, no oversampling, no in-cell expansion restriction, no seed fill, and no
contour extraction — so it reproduces `fb7b995050` (the per-layer Miter-grown occupancy
difference in `SupportPlanner::plan_candidate` already delivers collision freedom; measured
2026-09-03: zero penetration events in legacy mode above the clipper-sliver noise floor) but
not `a95607d7bf`. This packet ports the rasterizer as a
Ruling-8 knob: `support_area_rasterizer = agg` (canonical) selectable, `legacy_semantic` the
DEFAULT (binding human decision, 2026-09-03; `agg` ships opt-in); both paths tested; the parity
evidence in this packet runs `agg` by EXPLICIT selection, not as the default.

## Architecture Constraints

- Invariant 16 (plan §6): every acceptance command names explicit `--exact` test names or
  asserts matched-count non-zero in the same run — all AC commands tee to
  `target/test-output.log` and guard a non-zero ok-count.
- E1 (no vacuous assertions): AC-6/AC-7 compare MEASURED metric values against the Step-1
  baseline record (fixture `SupportAdversarial.stl`; AC-6 is a non-regression guard, AC-7 the
  strict improvement); a test that only checks artifact existence or computes-and-ignores a boolean
  is a defect.
- E4/T4 (guest freshness): planner changes are guest changes. Run
  `cargo xtask build-guests --check` before attributing any failure; rebuild without `--check`
  if stale; never grep for `STALE:` to decide.
- E6/T5 (feature-gated blindness): any slicer-core test command in this packet carries
  `--features host-algos`; reconcile binary counts if a narrow run disagrees with workspace.
- E8/E9: snake_case key (`support_area_rasterizer`); all grid math on PnP scaled-integer
  coordinates (`i64`, 1 unit = 100 nm) with mm only at config parse and offset boundaries.
- T7 (planner green ≠ real-mesh correctness): human gate includes a non-coplanar mesh slice;
  crate-suite green alone does not close the packet.
- T8 (silent config defaults): the knob is read from the module's filtered `ConfigView`;
  undeclared keys silently vanish (G-16 mechanism), so the manifest declaration (AC-1) and the
  parser must land in the SAME commit, plus the config-reference regen.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

Additional mandatory constraint (scale translation): canonical formulas are written in Orca
scaled coordinates (1 unit = 1 nm). Every constant translates by ÷100 at the PnP boundary:
`extrusion_width_scaled + 21` becomes `(width_units + 21)` where `width_units =
mm_to_units(extrusion_width_mm)` etc., and the clamp divisor `extrusion_width_scaled + 100`
becomes `width_units + 100`. The in-cell bound `abs(2 * offset_in_grid) < pixel_size - 10`
keeps its literal form in PnP units after consistent translation; assert it as a debug
invariant in the extraction entry.

## Data and Contract Notes

- IR/manifest contracts: body geometry rides the existing
  `SupportPlanEntry.roles → SupportPlanRoleRegion { role: SupportPlanRole::SupportBody,
  regions: Vec<ExPolygon> }` transport — no new IR type. The knob is manifest-declared
  snake_case; undeclared keys silently default (E9/T8) so declaration + parse + doc-regen are
  one commit.
- **Duplicate-type disambiguation (pin these; both names resolve two ways in this tree):**
  - `SupportPlanEntry` exists twice — the IR type (`crates/slicer-ir/src/slice_ir.rs`) and an
    SDK mirror (`crates/slicer-sdk/src/prepass_types.rs`, which derives `Default` where the IR
    one does not). `slicer_sdk::prelude` re-exports the **SDK** one, so guest code that imports
    the prelude gets the SDK type. This packet's planner code uses the **SDK** type, exactly as
    `lib.rs` does today — do not "correct" it to `slicer_ir::SupportPlanEntry`.
    `SupportPlanRoleRegion` / `SupportPlanRole` / `SupportPlanDeclineReason` are IR types in
    both paths, so they are unambiguous.
  - `ClipOperation` exists twice — `slicer_sdk::host::ClipOperation` (re-exported by
    `slicer_sdk::prelude`) and `slicer_core::polygon_ops::ClipOperation`, bridged internally by
    `to_core_clip_op`. This packet uses the **SDK** one, reached through the prelude, matching
    the existing `host::clip_polygons(..)` call sites in `lib.rs`.
  - `Point2` likewise has an unrelated same-named type in
    `crates/slicer-core/src/arachne/sparse_point_grid.rs`; the one this packet means is the IR
    `Point2` re-exported by `slicer_sdk::prelude`.
- Grid ↔ polygon contract: `GridParams.origin` is the rotated-bbox min in PnP units;
  `pixel_size` ≥ extrusion width so extracted contours stay printable; the one-pixel boundary
  ring is guaranteed unset by construction (canonical "Grid has to have the boundary pixels
  unset").
- Determinism/scheduler constraints: rasterization is a pure function of (polygons, params);
  iteration order over cells is row-major fixed; no float accumulation across layers beyond
  what the legacy path already carries. Layer-parallel safety unchanged (manifest hint stays
  `layer-parallel-safe = false`).
- **Macro-block halo: ACCEPTED divergence; the asymmetric clamp is REJECTED (binding human
  decision, 2026-09-03; recorded as DEV-166 in `docs/DEVIATION_LOG.md`).** A faithful port makes
  the extraction strictly larger than the polygons it was built from: canonical
  `seed_fill_block` (`SupportMaterial.cpp`) floods each `oversampling × oversampling` macro
  block INDEPENDENTLY, so the carry grows by at most one macro-block extent (measured
  2026-09-03 at the matched profile: one macro block = `pixel_size` 4167 x `oversampling` 6 =
  25002 units = 2.5002 mm). **This is
  canonical behaviour, not a porting bug.** Canonical does it deliberately — supports are
  stretched into the grid so the zig-zag support snake can run along grid lines — and canonical
  consequently prints that material where no overhang demanded it.
  Consequences in PnP, both accepted for the opt-in mode only: (a) support is printed where
  PnP's demand model demands none (the emit loop derives bodies from `propagated_by_layer`)
  and the halo crosses PnP's per-region foreign-territory bar; (b) the inflated carry routes
  around an obstacle that would otherwise close every route, so PnP's structured
  `SupportPlanDeclineReason::NoRoute` / diagnostic `code: 1203` decline does not fire under `agg` when the blocking occupancy is LOCAL (it still fires when occupancy covers the whole grid neighbourhood, since seed fill is then blocked everywhere and the carry genuinely empties). Canonical has NO decline concept: when trimming closes every route,
  `diff(carry, trimming)` goes empty before rasterization and the caller simply skips the lower
  layers. Block-snapping cannot preserve a PnP-only invariant that canonical never had.
  An asymmetric clamp of the propagated carry and the printed area to `pre_grid_carry` was
  implemented (Step 6d) and has since been **REJECTED by human decision and REMOVED** from the
  `RasterizerMode::Agg` arm of `SupportPlanner::plan_candidate`
  (`modules/core-modules/traditional-support-planner/src/lib.rs`). Reason: the clamp reduced the
  agg arm to legacy + a global `offset_to_slice`, deleting the very behaviour the port exists to
  reproduce (F-I1 control, requirements.md appendix — measured UNDER the clamp and therefore
  stale for the current code).
  **Current position: `agg` ships UNCLAMPED and OPT-IN; `legacy_semantic` is the DEFAULT and
  retains every PnP invariant** (structured decline, territory bar, demand-only emission).
- **Root-cause probe findings (recorded; implementation-plan.md Steps 10–11). Do not
  re-investigate.** Three hypotheses that the halo was a PnP porting defect were tested and
  REFUTED:
  - H1 — "extraction is block-granular": REFUTED. Extraction is pixel-granular; canonical
    `contours_simplified` (`SupportMaterial.cpp`) never receives `oversampling` at all.
  - H2 — "`dilate_trimming_region` wrongly dilates": REFUTED. It is a correct erosion —
    measured 144 → 100 set cells on the probe input, where a dilation would have given 196.
  - H3 — "`seed_fill_block` is mis-ported": REFUTED. The port is two-pass, block-local, and
    gated on the dilated mask at BOTH endpoints, matching canonical.
  The halo is therefore produced by canonical block-local flooding, not by this port.
- **Separate confirmed bug found in passing — NOT agg-specific, NOT a cause of any packet-241
  failure; recorded for a FOLLOW-UP packet and deliberately not filed as this packet's work.**
  `occupancy_at` (`modules/core-modules/traditional-support-planner/src/lib.rs`) filters on
  `object_id` AND `region_id`, while the `support_analysis` producer keys `model_occupancy` per
  (layer, object, region). Sibling regions' slices are therefore omitted from the trimming
  mask. This mask also feeds the LEGACY `Difference` path, so the defect is independent of the
  rasterizer and predates it. No packet-241 test failure was traced to it.
- Grid facts for the matched profile (measured 2026-09-03): `support_base_pattern_spacing`
  2.5 mm and `line_width` 0.4 mm give `grid_resolution` 25000 units, width 4000 units,
  `oversampling` 6, `pixel_size` **4167 units**, and therefore a macro-block extent of
  `oversampling * pixel_size` = **25002 units (2.5002 mm)**; `offset_to_slice` 2001 units,
  `offset_to_propagate` -1 unit
  (`OFFSET_TO_PROPAGATE` in `modules/core-modules/traditional-support-planner/src/lib.rs`).
  The extent is pinned by live assertions in the test suite (`MacroBlockExtent` /
  `MacroBlockExtent::assert_consistent` in
  `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`), which re-derive it
  from the two mm inputs rather than hardcoding it. An earlier draft of this packet quoted
  `pixel_size` 4166 / extent 24996 — those figures were wrong and are corrected here.
- Rotation: canonical rotates polygons by `-params.support_angle` when non-zero; PnP has no
  support-angle knob yet (not in this packet's scope), so the rotation branch is coded but
  exercised only at angle 0 until an angle key exists — recorded here, not as a [BLOCK].

## Locked Assumptions and Invariants

- Knob vocabulary LOCKED: `"agg"` | `"legacy_semantic"` (Ruling 8; plan §12 wording
  "the legacy semantic"). The DEFAULT is `"legacy_semantic"` — changed from `"agg"` by binding
  human decision on 2026-09-03 (recorded in `packet.spec.md` §Human Validation Gate); the
  vocabulary itself is unchanged. Renaming requires a new packet decision, not a follow-up
  edit.
- Canonical formula fidelity LOCKED (AC-2): oversampling clamp 1..=8, pixel-size max-form,
  macro-block arithmetic, boundary ring — translated ÷100 to PnP units, asserted by test.
- The legacy path LOCKED byte-equivalent behavior (AC-N2). It is now also the DEFAULT path.
  **AC-N2 is nevertheless RED on the current tree and stays red** — the suite measures
  26 passed / 2 failed, for a producer-side reason unrelated to the rasterizer: the planner
  publishes one `SupportPlanEntry` per candidate per layer, so several entries can share one
  `(global_layer_index, object_id, region_id)` identity, which
  `docs/02_ir_schemas.md` § "IR 9b — SupportPlanIR" forbids. See `packet.spec.md`
  §Negative Test Cases, `implementation-plan.md` Step 20, and DEV-167. The fix is owned by
  packet `241b-support-plan-ownership-seam`; packet 241 closes NARROW and NOT GREEN.
  Parity evidence for this packet runs `agg`, which is EXPLICITLY selected and is no longer the
  default — this supersedes the original "parity evidence runs the DEFAULT" reading of Ruling 8,
  which the 2026-09-03 default decision overrides.
- Measurement baselines LOCKED to the Step-1 committed artifact (recorded on
  `SupportAdversarial.stl`, legacy mode); post-port comparisons quote those numbers, never
  re-derived ones.

## Risks and Tradeoffs

- Port-fidelity risk: subtle divergence in chaining or seed-fill order changes outlines
  without breaking invariants; mitigated by AC-2/AC-3 formula-level tests and the AC-6 guard /
  AC-7 measured delta against Orca-referenced symptoms.
- Performance risk: oversampled grids cost memory/time per candidate; bounded by the ≤8×8
  clamp and per-candidate bbox sizing; the manifest `estimated-ms-per-layer = 5` hint may need
  updating after measurement — update it in the same step if measured drift exceeds the hint's
  honesty (do not guess).
- Coverage inflation risk: continuity fixes could inflate total area. AC-7's ±25 % total-area
  guard USED to catch this and has been **RETIRED** (Step 15b) — not widened. Its premise is
  contradicted by the accepted canonical behaviour: the `seed_fill_block` macro-block halo
  (DEV-166) adds material by design, measured at **+57.09 %** on `SupportAdversarial.stl`.
  The replacement gate is per-layer containment: every layer's `agg` body region must lie
  inside the `legacy_semantic` region for the same layer grown by ONE derived macro-block
  extent. That is strictly stronger than an area ratio — a ratio cannot tell a block-scale
  halo from support appearing somewhere else entirely, while containment forbids the latter
  outright. Measured 2026-09-03: **0.0 units² outside on 26/26 layers, 0 difference pieces**,
  with a bisected smallest containing grow of **22754 units** against the derived extent of
  **25002 units** (margin 2248 units, ≈ 9.0 % of one macro block).
- Legacy-path regression risk while editing the shared loop; mitigated by keeping the legacy
  branch textually separate and AC-N2 running the full existing suite.
- Human-gate subjectivity on "wall leak": mitigated by AC-6's numeric penetration metric doing
  the gating and the visual tap serving confirmation only (E2).
- No-measured-benefit-over-control risk (position updated 2026-09-03, after the clamp was
  rejected). The F-I1 control test
  `support_agg_rasterizer_tdd::agg_printed_area_exceeds_global_offset_control`
  (`crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`, renamed from
  `agg_printed_area_matches_global_offset_control` when its assertion inverted) RESOLVED this
  risk in Step 14. Under the clamp it measured agg equal to a legacy+`offset_to_slice` control
  within 0.0266 mm²/layer, with identical drop counts — i.e. the grid contributed nothing.
  Re-measured 2026-09-03 with the clamp REMOVED, on `SupportAdversarial.stl`: agg − control =
  +984.12 mm² (+38.40 % of the control) while control − legacy = +304.94 mm², so the grid now
  contributes about 3.2× what the global offset does; max per-layer symmetric difference rose
  from 0.0266 mm² to 38.1980 mm², on 26/26 layers.
  Current position: the grid pipeline IS discriminable from the offset, and the earlier
  "no measured benefit over control" finding was an artifact of the clamp, not a property of
  the port. What the measurement does NOT establish is that the difference is desirable: the
  extra area IS the canonical block-snapped halo, which PnP's demand model classifies as
  phantom support: it drove the total-area delta to +57.09 %, breaching AC-7's original ±25 %
  guard. That guard was retired by human decision (Step 15b) and replaced with a
  mechanism-derived bound — agg's per-layer region must lie inside the legacy region grown by
  one derived macro-block extent (25002 units = 2.5002 mm at the matched profile). AC-7 is
  GREEN under that bound: measured 2026-09-03, the agg region falls entirely inside the grown
  legacy region on 26/26 layers (0.0 units² outside, 0 difference pieces), and bisection puts
  the smallest containing grow at 22754 units, a 9.0 % margin under the bound. The containment
  result independently corroborates the canonical one-macro-block analysis.
  The packet's value claim therefore rests on canonical parity for users who explicitly opt in.
