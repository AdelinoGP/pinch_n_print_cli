# Design: 241-support-agg-rasterizer

## Controlling Code Paths

- Primary code path: `modules/core-modules/traditional-support-planner/src/lib.rs`
  (`SupportPlanner::plan_candidate` → the per-layer propagation loop building
  `propagated_by_layer`) plus the new sibling module
  `modules/core-modules/traditional-support-planner/src/agg_raster.rs` (the port).
- Neighboring tests/fixtures:
  `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs` (new),
  `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` (legacy
  guard, existing), `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`
  (new, measurement harness), `crates/slicer-runtime/tests/integration/support_family_closure.rs`
  (existing fixture driver patterns reused by reference, not edited except `main.rs` registration),
  `crates/slicer-runtime/tests/common/support_wedge.rs` (wedge context with overrides — reused,
  not edited).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat
  delegation rules.

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

## Code Change Surface

- Selected approach: self-contained guest-side rasterizer module + a mode switch at the top of
  the propagation loop. The legacy loop stays verbatim under `legacy_semantic`. No WIT change:
  everything runs inside the guest using pure Rust grid code plus the ONE host polygon op the
  rasterizer needs, `slicer_sdk::host::clip_polygons` (already linked by `lib.rs`).
  `host::offset_polygons` is NOT called from `agg_raster.rs`: the two offsets this packet
  needs — the occupancy clearance mask and island-sample generation — both run at the
  `lib.rs` call site and are passed into the rasterizer as ready-made polygons. This is what
  makes AC-4's `! rg -q 'offset_polygons' src/agg_raster.rs` satisfiable by construction.
  `difference_ex` is not available in this module: it lives in
  `crates/slicer-core/src/polygon_ops.rs`, is NOT re-exported by `slicer-sdk` (the SDK
  re-exports only `slicer_core::perimeter_utils::WallSequence`), and `slicer-core` is not a
  dependency of `traditional-support-planner`. This is a dependency-surface fact, NOT a
  host/guest boundary: `pub mod polygon_ops` is ungated and compiles to wasm32 today —
  `arachne-perimeters` depends on `slicer-core` unconditionally and calls `difference_ex` in
  its guest code. We deliberately do NOT add that dependency here (it would widen this
  module's dependency surface for one operation the SDK already exposes), so every set
  difference in this packet is `host::clip_polygons(a, b, ClipOperation::Difference)`.
- Exact functions, traits, manifests, tests, and fixtures:
  - NEW `agg_raster::SupportGrid { params: GridParams, support: Vec<u8>, trimming: Vec<u8> }`
    — the `SupportGridPattern` equivalent; owns the two byte grids for one candidate and is
    the receiver for `extract_support`. Built once per candidate, extracted per layer.
  - NEW `agg_raster::GridParams { pixel_size: i64, origin: Point2, grid_size: (usize, usize),
    macro: usize }` — derived from support polygons bbox + spacing/line-width exactly per the
    canonical constructor formulas (AC-2).
  - NEW `agg_raster::rasterize_polygons(&[ExPolygon], &GridParams) -> Vec<u8>` — even-odd
    scanline fill over scaled-integer vertices; replicates canonical AGG gray8 semantics
    (supersampled coverage collapsed to set/unset, matching the binary grid canonical stores).
  - NEW `agg_raster::dilate_trimming_region(&[u8], &GridParams) -> Vec<u8>` — 3×3 all-set mask.
  - NEW `agg_raster::seed_fill_block(&mut Vec<u8>, trimming, &GridParams)` — per-macro-block
    top-down + bottom-up propagation with left/right sweeps gated by the dilated mask.
  - NEW `agg_raster::contours_simplified(&[u8], offset_in_grid: i64, fill_holes: bool,
    &GridParams) -> Vec<ExPolygon>` — boundary-edge collection + lexicographic chaining +
    `fill_holes` neighbor rule + per-loop offset by `offset_in_grid` (in-cell restriction, AC-4).
  - NEW `agg_raster::SupportGrid::extract_support(&self, offset_in_grid: i64,
    fill_holes: bool, samples: &[Point2]) -> Vec<ExPolygon>` — difference vs the trimming
    polygons via `host::clip_polygons(.., ClipOperation::Difference)` (NOT `difference_ex`,
    which this module does not depend on — see the dependency-surface note above), then the
    island sample-containment filter (ray-crossing
    point-in-island, canonical `extract_support`). The expanding-vs-shrinking sample choice
    is made by the CALLER in `lib.rs` (it needs `host::offset_polygons`) and handed in as
    `samples`, keeping `agg_raster.rs` offset-free per AC-4.
  - MODIFIED `SupportPlanner` (`from_config`): new field `support_area_rasterizer:
    RasterizerMode` parsed from `"agg" | "legacy_semantic"`; unknown strings → fatal
    `ModuleError` naming key + allowed values (AC-N1). Default `Agg`.
  - MODIFIED `plan_candidate` propagation loop: when `Agg`, build one `agg_raster::SupportGrid`
    per candidate from the contact geometry (support polygons = current carry,
    trimming polygons = occupancy grown by `support_object_xy_distance` via the EXISTING
    `host::offset_polygons` clearance code — that offset builds the *trimming mask*, which is
    canonical; what disappears from the agg path is any post-extraction global polygon
    expansion), then per layer extract with `expansion_to_slice` for print area and
    `expansion_to_propagate` (= unexpanded carry semantic) for the next layer's input.
    Termination bookkeeping (empty-carry → diagnostic 1203 + structured
    `SupportPlanDeclineReason::NoRoute`) is preserved identically. When `LegacySemantic`, the
    loop body is byte-identical to today's.
  - MODIFIED `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`:
    add `[config.schema.support_area_rasterizer]` enum table (AC-1).
  - NEW tests: `tests/agg_rasterizer_tdd.rs` (grid math, extraction, rejection, routing);
    `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` (baseline capture +
    three measurement/divergence tests); `main.rs` `mod` registration line.
  - Cargo.toml: register `agg_rasterizer_tdd` `[[test]]` target.
- Rejected alternatives and reasons:
  - *Port into `slicer-core`* (like 238c's regularize move): rejected — the rasterizer is
    traditional-planner-private today; no second consumer exists, and hosting it in the guest
    keeps the host dependency surface unchanged. Revisit if the renderer later needs grids.
  - *Port the EdgeGrid/SDF branch too*: rejected — canonical compiles it out
    (`SUPPORT_USE_AGG_RASTERIZER`); carrying both forks doubles test surface for dead code.
  - *Global polygon-offset approximation of expansion*: rejected — it is precisely the defect
    `fb7b995050` fixed; AC-4 forbids it on the agg path. (Note: PnP's legacy loop already
    reproduces `fb7b995050`'s outcome via the per-layer Miter-grown occupancy difference in
    `SupportPlanner::plan_candidate`; AC-4 keeps the agg module offset-free by construction.)
  - *Knob defaulting silently on unknown values* (mirroring lenient string keys elsewhere):
    rejected — Ruling 8 knobs replace legitimate behavior, so an out-of-vocabulary value must be
    loud (AC-N1). The precedent is `SeamPlacer::from_config`
    (`modules/core-modules/seam-placer/src/lib.rs`), which returns `ModuleError::fatal` on an
    unknown `seam_mode` even though that key is a manifest-declared enum. (NOT
    `canonical_support_family` — that helper in `crates/slicer-ir/src/slice_ir.rs` does the
    opposite, silently falling back to `SUPPORT_FAMILY_TRADITIONAL`.) The host also rejects
    bad enum values first via `ConfigBoundsIndex::check` from `resolve_global_config`; the
    module check is defense-in-depth, not the sole enforcement point.

## Files in Scope (read + edit)

Target at most 3 primary files per step; the packet total spans:

- `modules/core-modules/traditional-support-planner/src/agg_raster.rs` (new) - role: the port;
  expected change: ~450 lines of grid construction/fill/extraction + unit-testable API
- `modules/core-modules/traditional-support-planner/src/lib.rs` - role: consumer wiring;
  expected change: mode field + parse (~15 lines), propagation-loop branch (~60 lines)
- `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` -
  expected change: +1 `[config.schema]` table (7 lines)
- `modules/core-modules/traditional-support-planner/Cargo.toml` - expected change: +1
  `[[test]]` target
- `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs` (new) -
  role: guest-side proofs (AC-2..AC-5, AC-N1)
- `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` (new) - role:
  baseline capture + measurement harness (AC-6..AC-8)
- `crates/slicer-runtime/tests/integration/main.rs` - expected change: +1 `mod` line
- `docs/15_config_keys_reference.md` - regen gate output only

## Read-Only Context

- `modules/core-modules/traditional-support-planner/src/lib.rs` - long; ranged reads only.
  Read by symbol, not by remembered range: `from_config` (the `PrepassModule` trait method)
  for the config-parse conventions, and the `SupportPlanner::plan_candidate` propagation loop
  `for layer in (termination_layer..trim_end).rev()` which ends at
  `propagated_by_layer.insert(layer, carry.clone())` and CONTAINS the `code: 1203`
  diagnostic — any read range that stops before the loop's closing brace hides the
  termination bookkeeping AC-5 must preserve. The separate emit loop in `plan_candidate`
  (the one that derives bodies from `propagated_by_layer`) is a distinct block. Delegate the rest.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - long; ranged reads
  only. Read only the driver symbols: `support_test_path`, `matched_config_base` (the
  neighbouring `matched_config_for` is distinct), `run_slice_for_family` and
  `run_slice_for_family_with_interface_layers`, and the block-count helper
  `interface_block_count`. There is no local `run_slice` — the file calls
  `slicer_runtime::run::run_slice(opts)`. Do not edit.
- `crates/slicer-runtime/tests/common/support_wedge.rs` - whole file (173 lines) as the T7
  wedge-driver pattern; `prepare_wedge_context_with_overrides(support_enabled, overrides)`
  is the entry that takes config overrides.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - NEVER load; delegate
  LOCATIONS/SUMMARY around class `SupportGridPattern` and its statics; cite by symbol
  name, never by line number (the checkout revision differs per developer).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Packets 236–238c / 239 / 240a / 240b directories and their owned modules (`tree-support`,
  `traditional-support`, `tree-support-planner`, raft surfaces) - other packets' scope
- `crates/slicer-schema/wit/**` - no WIT change is authorized by this packet
- `docs/specs/support-families-anchored-entities-plan.md` queue table - orchestrator-owned
- `docs/DEVIATION_LOG.md` - explicitly out of scope (see Doc Impact Statement)

## Expected Sub-Agent Dispatches

- Question: exact current text of `plan_candidate`'s propagation loop + `from_config` parse
  block; scope, by symbol (the file is long — ranged reads only; do NOT rely on line pins,
  they have rotted once already in this packet): `SupportPlanner::from_config`'s config parse
  block and `SupportPlanner::plan_candidate`'s per-layer propagation loop (the loop building
  `propagated_by_layer`), both in
  `modules/core-modules/traditional-support-planner/src/lib.rs`; return: `SNIPPETS`;
  purpose: Step-3 red-baseline + wiring patch target.
- Question: confirm canonical constant forms in the `smsGrid` constructor branch and the exact
  propagation-step structure of `seed_fill_block`; scope:
  `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` class `SupportGridPattern`;
  return: `SNIPPETS` (≤30 lines ×3); purpose: port fidelity check before coding (Step 2).
- Question: LOCATIONS of every test asserting on `propagated_by_layer`-derived body geometry or
  diagnostic code 1203; scope: `modules/core-modules/traditional-support-planner/`; return:
  `LOCATIONS` ≤20; purpose: blast radius of the loop rewiring (Step 3).
- Question: FACT on whether any golden/baseline artifact hard-asserts traditional body outline
  counts; scope: `crates/slicer-runtime/tests/fixtures/golden/`, module goldens; return:
  `FACT`; purpose: struct-literal/assertion fallout pre-enumeration (Step 3).

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` — Steps 3 and 4, the port itself (grid construction, then seed fill +
  extraction). Step 2 is a read-only canonical fidelity probe rated `S`. The port is already
  split across two steps precisely so neither reaches `L`; the Step-2 SNIPPETS dispatch
  lands the canonical constants before either one starts coding.
- Highest-risk dispatch and required return format: canonical constant verification —
  `SNIPPETS` ≤30 lines ×3, else redispatch narrower.

## Open Questions

- [FWD] Should `estimated-ms-per-layer` be revised after measuring the agg path on the wedge?
  Implementer-resolvable at Step 8 with a measured number; keep the old value if drift is
  within noise.
- [FWD] A future `support_angle` key would exercise the rotation branch; confirm out-of-scope
  with 238a's key list before filing anything — do NOT add the key in this packet.
- [BLOCK] None at authoring time; activation blocked only by 238c reaching `implemented`.

## Plan Corrections

Recorded by Step 2 (canonical fidelity probe, `TASK-420`), 2026-09-03. All canonical facts below
were re-derived from the live checkout of
`OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` via a delegated sub-agent.
Citations are by symbol name only (never line number), per the OrcaSlicer citation rule.

p241 fidelity probe: DIFF - the AGG fill rule is **nonzero**, not even-odd. This file's
Code Change Surface describes `rasterize_polygons` as an "even-odd scanline fill". Canonical
`rasterize_polygons` never calls `filling_rule`, so AGG's default `fill_non_zero` applies
(grepping the file for `filling_rule|fill_even_odd` returns nothing). The port must use the
nonzero winding rule. For the simple, non-self-intersecting support contours this packet
rasterizes the two rules agree, but the difference is real for self-overlapping input and the
port is written to the canonical rule.

p241 fidelity probe: DIFF - there is no coverage threshold. Canonical rasterizes to an
antialiased gray8 buffer and every consumer (`dilate_trimming_region`, `seed_fill_block`,
`contours_simplified`) tests `!= 0`, i.e. *any* nonzero coverage counts as set. `seed_fill_block`
additionally writes the literal value `1` for newly filled cells, so the grid is a mix of `1`
and AA coverage values, all treated as "set". A binary set/unset grid is therefore behaviorally
equivalent to canonical provided the rasterizer marks any partially-covered cell as set - that
is the rule the port implements, and it is NOT the same as a 50%-coverage threshold.

p241 fidelity probe: DIFF - `dilate_trimming_region` is an **erosion**, and its mask is
8-neighborhood ALL-set, not a generic "3x3 mask". The canonical function name is misleading; its
own comment describes unmasking the boundary pixels. A cell is set in the output only when all
nine cells of its 3x3 neighborhood are set in the input. The outermost row and column are
skipped and remain unset. `requirements.md`'s "trimming-mask dilation (3x3)" phrasing should be
read as this erosion.

p241 fidelity probe: DIFF - grid sizing arithmetic. The packet paraphrases "macro blocks of
`oversampling x oversampling` cells, a one-pixel empty boundary ring". Canonical is more
specific and the port follows canonical exactly:
`grid_size_raw = ceil((bbox.max - bbox.min) / pixel_size)` componentwise, taken AFTER
`bbox.offset(pixel_size)` (a full-pixel margin on all four sides, not a one-cell ring);
`grid_blocks = (grid_size_raw + oversampling - 1 - 2) / oversampling` componentwise;
`grid_size = grid_blocks * oversampling + (2, 2)`. Note also that the pre-AGG
`bbox.align_to_grid` uses the UNOVERSAMPLED `grid_resolution = scale_(support_spacing)`, and
that `bbox.offset(20)` precedes it. AC-2's named formulas (oversampling clamp, pixel-size
max-form) are confirmed unchanged:
`oversampling = clamp(scale_(support_spacing) / (extrusion_width_scaled + 100), 1, 8)` and
`pixel_size = max(extrusion_width_scaled + 21, scale_(support_spacing / oversampling))`,
both translated to PnP units by the divide-by-100 rule.

p241 fidelity probe: DIFF - `seed_fill_block` is exactly **two passes**, not iterate-to-fixpoint,
and fill never crosses a macro-block boundary (each block is independent). Pass one runs
top-to-bottom with, per row, a left-to-right then a right-to-left sweep; pass two runs
bottom-to-top with the same two horizontal sweeps. Every propagation step is gated on the
dilated trimming mask being unset at BOTH the source and the destination cell. The block origin
skips the one-pixel boundary. The packet's "four-direction propagation steps" phrasing is
consistent with this but underspecifies the pass count and the both-endpoints mask gate, which
the port implements literally.

p241 fidelity probe: DIFF - `contours_simplified`'s `fill_holes` is a **single non-iterative
pass** reading the unmodified grid and writing into a copy: a cell is filled when its left AND
right neighbors are set, OR its top AND bottom neighbors are set. The `offset_in_grid` is applied
per corner point after rescaling (`p *= pixel_size; p += bbox.min`), with the sign chosen by
corner orientation from the vector between the previous and next retained points; collinear
points are dropped and only corners are emitted. The in-cell bound
`abs(2 * offset) < pixel_size - 10` is confirmed present as an assert and is asserted by the port.

p241 fidelity probe: no-diff - `extract_support`'s structure is as the packet describes.
Confirmed: difference of the simplified contours against the trimming polygons yields islands;
the sample set is chosen by the SIGN of `offset_in_grid` (`> 0` uses the union of the support
polygons, i.e. expanding; otherwise the intersection of the support polygons with the islands,
i.e. shrinking); islands are kept when any sample falls inside, tested by crossing-parity
against the contour and all holes, with a lexicographic binary-search prefilter over the sorted
samples. `island_samples` shrinks each expolygon by a small fixed inset and takes up to four
points per resulting polygon at a fixed stride, then sorts.

p241 fidelity probe: no-diff - `expansion_to_slice` vs `expansion_to_propagate`. Confirmed
canonical defaults `expansion_to_slice = scaled_spacing / 2 + 5` (positive, grows the printed
region) and `expansion_to_propagate = -3` (a tiny shrink, so downward-projected contours do not
creep outward layer over layer). Callers pass `expansion_to_slice` for polygons that become
printed geometry and `expansion_to_propagate` for polygons projected down to the next layer.
`fill_holes = true` everywhere except the dense-interface-trimmed calls.

p241 fidelity probe: STRUCTURAL CORRECTION - the grid is built **per layer**, not per candidate.
This file's Code Change Surface says the `Agg` branch should "build one
`agg_raster::SupportGrid` per candidate". That is not implementable and is not canonical: the
trimming mask is the per-layer model occupancy grown by `support_object_xy_distance` (see
`occupancy_at` and the `host::offset_polygons` clearance call inside `plan_candidate`'s
propagation loop), so it differs at every layer. A single per-candidate grid cannot carry it.
Canonical likewise constructs a `SupportGridPattern` per layer. The port therefore builds one
`SupportGrid` per (candidate, layer) from that layer's carry and that layer's trimming mask, and
extracts twice from it - once with `expansion_to_slice` for the printed area and once with
`expansion_to_propagate` for the next layer's carry. AC-5's wording ("contact polygons stretched
into the grid, trimmed by occupancy, re-extracted") is satisfied by this shape.

p241 fidelity probe: STRUCTURAL CORRECTION - `run_slice` cannot reach plan geometry, so AC-6,
AC-7 and AC-8 are driven through the prepass context instead. `slicer_runtime::run::run_slice`
returns a `SliceOutcome` carrying only `gcode_text`, `layer_count`, `wallclock_ms` and
`profile` - no blackboard and no IR - so the "sliced via `run_slice`" phrasing in those ACs
cannot yield support body outlines or per-layer model occupancy. The measurement harness uses
`slicer_runtime::run::prepare_prepass_context`, reaching body regions via
`blackboard.support_plan()` then `entries`, `roles`, `SupportPlanRole::SupportBody`, `regions`,
and per-layer occupancy via `slicer_wasm_host::exact_z_query::ExactZQueryService::query`. This
is the mechanism the existing invariant `support_never_intersects_model_at_exact_z` already
uses. The AC verification commands name test names rather than drivers, so they remain
satisfiable unchanged.

p241 fidelity probe: no-diff - `SupportGridParams` location. It is a file-local struct in
`SupportMaterial.cpp` immediately above `class SupportGridPattern`, NOT a member of
`SupportMaterial.hpp` as `packet.spec.md` and `requirements.md` state under OrcaSlicer Reference
Obligations. The field semantics cited there are otherwise accurate.
