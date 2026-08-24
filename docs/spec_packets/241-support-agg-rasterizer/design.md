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
  baseline record; a test that only checks artifact existence or computes-and-ignores a boolean
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
  everything runs inside the guest using `slicer_sdk::host` polygon ops it already links
  (`clip_polygons`, `offset_polygons` for sample generation only) plus pure Rust grid code.
- Exact functions, traits, manifests, tests, and fixtures:
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
  - NEW `agg_raster::extract_support(&self, offset_in_grid, fill_holes, samples) ->
    Vec<ExPolygon>` — difference vs trimming polygons (`difference_ex`), island
    sample-containment filter (ray-crossing point-in-island, canonical `extract_support`),
    expanding-vs-shrinking sample choice by offset sign.
  - MODIFIED `SupportPlanner` (`from_config`): new field `support_area_rasterizer:
    RasterizerMode` parsed from `"agg" | "legacy_semantic"`; unknown strings → fatal
    `ModuleError` naming key + allowed values (AC-N1). Default `Agg`.
  - MODIFIED `plan_candidate` propagation loop: when `Agg`, build one `SupportGridPattern`
    equivalent per candidate from the contact geometry (support polygons = current carry,
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
    `fb7b995050` fixed; AC-4 forbids it on the agg path.
  - *Knob defaulting silently on unknown values* (mirroring lenient string keys elsewhere):
    rejected — Ruling 8 knobs replace legitimate behavior, so an out-of-vocabulary value must be
    loud (AC-N1), matching enum-bounds precedent rather than string-key leniency.

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

- `modules/core-modules/traditional-support-planner/src/lib.rs` - full file is ~687 lines;
  read lines 30–150 (config parse) and 300–470 (propagation loop + bottom contact) directly;
  delegate the rest if needed.
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - ranges around
  `run_slice_for_family_with_interface_layers` (~159–190) and `matched_config_base` (~67–110)
  only, as driver patterns to mirror; do not edit.
- `crates/slicer-runtime/tests/common/support_wedge.rs` - whole file (~160 lines) as the T7
  wedge-driver pattern.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - NEVER load; delegate
  LOCATIONS/SUMMARY around `SupportGridPattern` (class starts ~line 637 of the local checkout;
  cite by symbol, never line).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Packets 236–238c / 239 / 240 directories and their owned modules (`tree-support`,
  `traditional-support`, `tree-support-planner`, raft surfaces) - other packets' scope
- `crates/slicer-schema/wit/**` - no WIT change is authorized by this packet
- `docs/specs/support-families-anchored-entities-plan.md` queue table - orchestrator-owned
- `docs/DEVIATION_LOG.md` - explicitly out of scope (see Doc Impact Statement)

## Expected Sub-Agent Dispatches

- Question: exact current text of `plan_candidate`'s propagation loop + `from_config` parse
  block; scope: `modules/core-modules/traditional-support-planner/src/lib.rs` lines 75–130 and
  330–470; return: `SNIPPETS`; purpose: Step-3 red-baseline + wiring patch target.
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
- Grid ↔ polygon contract: `GridParams.origin` is the rotated-bbox min in PnP units;
  `pixel_size` ≥ extrusion width so extracted contours stay printable; the one-pixel boundary
  ring is guaranteed unset by construction (canonical "Grid has to have the boundary pixels
  unset").
- Determinism/scheduler constraints: rasterization is a pure function of (polygons, params);
  iteration order over cells is row-major fixed; no float accumulation across layers beyond
  what the legacy path already carries. Layer-parallel safety unchanged (manifest hint stays
  `layer-parallel-safe = false`).
- Rotation: canonical rotates polygons by `-params.support_angle` when non-zero; PnP has no
  support-angle knob yet (not in this packet's scope), so the rotation branch is coded but
  exercised only at angle 0 until an angle key exists — recorded here, not as a [BLOCK].

## Locked Assumptions and Invariants

- Knob vocabulary LOCKED: `"agg"` (default) | `"legacy_semantic"` (Ruling 8; plan §12 wording
  "the legacy semantic"). Renaming requires a new packet decision, not a follow-up edit.
- Canonical formula fidelity LOCKED (AC-2): oversampling clamp 1..=8, pixel-size max-form,
  macro-block arithmetic, boundary ring — translated ÷100 to PnP units, asserted by test.
- The legacy path LOCKED byte-equivalent behavior under explicit selection (AC-N2); parity
  evidence runs the DEFAULT (Ruling 8).
- Measurement baselines LOCKED to the Step-1 committed artifact; post-port comparisons quote
  those numbers, never re-derived ones.

## Risks and Tradeoffs

- Port-fidelity risk: subtle divergence in chaining or seed-fill order changes outlines
  without breaking invariants; mitigated by AC-2/AC-3 formula-level tests and the AC-6/AC-7
  measured deltas against Orca-referenced symptoms.
- Performance risk: oversampled grids cost memory/time per candidate; bounded by the ≤8×8
  clamp and per-candidate bbox sizing; the manifest `estimated-ms-per-layer = 5` hint may need
  updating after measurement — update it in the same step if measured drift exceeds the hint's
  honesty (do not guess).
- Coverage inflation risk: continuity fixes could inflate total area; AC-7's ±25% guard catches
  buying coverage with material.
- Legacy-path regression risk while editing the shared loop; mitigated by keeping the legacy
  branch textually separate and AC-N2 running the full existing suite.
- Human-gate subjectivity on "wall leak": mitigated by AC-6's numeric penetration metric doing
  the gating and the visual tap serving confirmation only (E2).

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 port itself; mitigated by the SNIPPETS fidelity dispatch before
  coding and by splitting grid-construction from extraction into two sub-commits within the
  step)
- Highest-risk dispatch and required return format: canonical constant verification —
  `SNIPPETS` ≤30 lines ×3, else redispatch narrower.

## Open Questions

- [FWD] Should `estimated-ms-per-layer` be revised after measuring the agg path on the wedge?
  Implementer-resolvable at Step 8 with a measured number; keep the old value if drift is
  within noise.
- [FWD] A future `support_angle` key would exercise the rotation branch; confirm out-of-scope
  with 238a's key list before filing anything — do NOT add the key in this packet.
- [BLOCK] None at authoring time; activation blocked only by 238c reaching `implemented`.
