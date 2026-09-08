# Requirements: 241-support-agg-rasterizer

## Packet Metadata

- Grouped task IDs: `TASK-419`..`TASK-428`
- Backlog source: `docs/specs/support-families-anchored-entities-plan.md` §12 brief
  "241-support-agg-rasterizer". TASK-419..TASK-428 are reserved for this packet by row #8 of
  that file's trailing `## Packet Queue` table (NOT by the §12 brief, and NOT by the §11
  "Packet queue" table — §11 carries no TASK column at all). Absorbs register row G-07
  (`docs/specs/support-parity-gap-register.md`). The
  stub `docs/spec_packets/stubs/stub-support-agg-rasterizer.md` no longer exists — it was
  deleted when the G-07 premise was corrected (the register records "stub deleted"), so
  there is nothing left to absorb and no `docs/spec_packets/stubs/` directory to touch.
- Packet status: `draft`
- Depends on: `238c-support-renderer-flow-interfaces` — SATISFIED (verified 2026-09-03: its
  `packet.spec.md` frontmatter reads `status: implemented`, as do 236, 238a and 238b). No
  forward dependency blocks activation. Ledger fact — re-derive at activation.
- Aggregate context cost: `M` (per-step roll-up in `implementation-plan.md`; no step rated L)

## Premise corrections (measured 2026-09-03)

1. `fb7b995050` (collision freedom) was ALREADY reproduced by the legacy loop: per-layer
   `clip_polygons(carry, offset_polygons(occupancy, support_object_xy_distance, Miter), Difference)`
   in `SupportPlanner::plan_candidate` (`modules/core-modules/traditional-support-planner/src/lib.rs`).
   Only `a95607d7bf` (column continuity) was missing.
2. AC-6 is therefore a non-regression guard (zero events above a noise floor; area not greater
   than baseline), not an improvement claim.
3. The 30x20 box fixture measures 0/0 column drops; AC-7 and the Step-1 baseline use
   `SupportAdversarial.stl` instead.
4. **Superseded 2026-09-03.** An earlier correction here claimed a faithful port "needed" an
   asymmetric clamp to `pre_grid_carry`. That clamp was **REJECTED by binding human decision
   and removed from the code.** The canonical macro-block halo is canonical behaviour, not a
   porting defect: `seed_fill_block` (`SupportMaterial.cpp`) floods each
   `oversampling × oversampling` block independently, and canonical prints that material where
   no overhang demanded it. Under `agg` the halo therefore (a) prints support PnP's demand model
   never demanded and crosses PnP's foreign-territory bar, and (b) keeps the carry non-empty so
   PnP's structured `SupportPlanDeclineReason::NoRoute` / `code: 1203` decline provably cannot
   fire — an invariant canonical never had (canonical's `diff(carry, trimming)` simply goes
   empty and the caller skips the lower layers). Current position: `agg` is UNCLAMPED and
   OPT-IN; `legacy_semantic` is the DEFAULT and keeps every PnP invariant. See design.md
   §Data and Contract Notes and DEV-166. Every measurement previously quoted in support of the
   clamp was taken under it and is stale (see the appendix banner below).

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

## In Scope

- New guest-side rasterizer module
  `modules/core-modules/traditional-support-planner/src/agg_raster.rs`: byte-grid construction
  (oversampled ≤8×8, macro blocks, boundary ring), polygon→grid rasterization (AGG gray8
  scanline semantics on PnP scaled-integer coordinates), trimming-mask dilation (3×3),
  macro-cell seed fill (4-direction propagation steps), contour extraction (marching-squares-
  equivalent line chaining, `fill_holes`, `offset_in_grid`), island sample-containment filter.
- Manifest knob `[config.schema.support_area_rasterizer]` (`enum`,
  `values = ["agg", "legacy_semantic"]`, `default = "legacy_semantic"` — the default was
  changed from `"agg"` by binding human decision on 2026-09-03) in
  `traditional-support-planner.toml`; module config parsing + rejection of out-of-vocabulary
  values at the module boundary.
- Rewiring of `plan_candidate`'s propagation loop to consume the rasterizer when `agg` is
  explicitly selected, preserving interface anchoring, bottom-contact derivation inputs, and
  demand/body ID threading. Termination bookkeeping (the structured `NoRoute` decline) is NOT
  preserved under `agg` — the block-snapped carry routes around obstacles that would otherwise
  close every route, so the decline does not fire under `agg` when the blocking occupancy is LOCAL (it still fires when occupancy covers the whole grid neighbourhood, since seed fill is then blocked everywhere and the carry genuinely empties). That is an accepted, documented divergence
  (DEV-166); it is retained under the default `legacy_semantic` mode.
- Measurement harness (integration tests): pre-port baseline capture, post-port wall-leakage
  (penetration events + penetrated area vs occupancy grown by `support_object_xy_distance`) and
  column-continuity (abrupt column drops; per-layer macro-block containment) metrics per
  AC-6/AC-7/AC-8. **The total-area drift guard (±25 %) is NOT in scope: it was RETIRED in
  Step 15b** because the accepted canonical `seed_fill_block` halo adds material by design
  (measured +57.09 %), and replaced by the mechanism-derived containment bound — every layer's
  `agg` region must lie inside the `legacy_semantic` region grown by one derived macro-block
  extent. Total area is still RECORDED, but it gates nothing.
  The wall-leakage grow uses a **Round** join (Euclidean xy clearance; `GROW_JOIN` in
  `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`). A Miter join was
  measured and produces ~0.064 mm^2/layer residual at the solid's convex corners (the emitted
  outline is rounder than the planner's Miter trimming mask), so it was not adopted.
  **CLAMP-ERA, not re-measured:** the accompanying "in BOTH modes" equality of that Miter
  residual was measured under the since-removed clamp, when agg and legacy geometry were near
  identical; it must not be read as a current property of the unclamped `agg` mode. The Miter
  join is rejected on the legacy measurement regardless of what agg now does.
  The Round metric's noise floor is `WALL_LEAKAGE_NOISE_FLOOR_UNITS2` = 10_000 units²
  = 1e-4 mm², applied per intersection piece. **Post-clamp-removal measurement (Step 14):
  0 events / 0.0 units² in BOTH modes — no tangency slivers were observed at all**, so the
  floor is inert on this fixture. The clamp-era figure of 88–311 units² per sliver is
  superseded and must not be quoted as current. AC-6 counts events only above that floor.
  Fixture: the tracked 30x20 mm `SupportTest.stl` box measures 0 penetration events / 0
  penetrated area / 0 abrupt column drops in legacy mode, so AC-7's strict gate cannot be
  measured against it. AC-6/AC-7 and the Step-1 baseline are measured against
  `crates/slicer-runtime/tests/fixtures/support-family/SupportAdversarial.stl` (generated
  in-test by the `adversarial_mesh()` helper — three `roof_edge_slot` blocks — via the
  ignored recorder `p241_generate_adversarial_fixture`; `stepped_pocket_mesh` is retained
  only as the rejected 3/3-drops counterexample).
- New test SUBMODULE `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`
  — not a new test binary. It joins the existing aggregated `integration` binary and MUST be
  registered with a `mod support_agg_rasterizer_tdd;` line in
  `crates/slicer-runtime/tests/integration/main.rs` (which currently aggregates 70 modules).
  Without that line the file never compiles and `cargo test --test integration <name>`
  reports "0 tests run" — a false pass. Separately, a new guest test file
  `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs` gets its own
  `[[test]]` stanza in the crate's `Cargo.toml`, matching the existing explicit
  `[[test]] name = "traditional_family_tdd" / path = "tests/traditional_family_tdd.rs"` stanza.
  Note this is a CONVENTION choice, not a compilation requirement: the workspace is edition
  2021 (`Cargo.toml` `[workspace.package] edition = "2021"`) and the crate sets no
  `autotests = false`, so target autodiscovery remains ON and the file would be picked up
  even without the stanza. Declaring it explicitly keeps the crate's two test targets
  symmetric and keeps the `--test agg_rasterizer_tdd` name pinned rather than inferred.
  (Do not carry the older, false rationale that an explicit `[[test]]` stanza disables
  autodiscovery — it does not.)
- Doc impact items listed in `packet.spec.md` §Doc Impact Statement (config-key reference regen;
  TASK registration).

## Out of Scope

- Tree-family rendering/planner surfaces — owned by 238b (done there). Canonical maps tree
  styles to `smsGrid` inside `SupportGridPattern`, but this packet wires the knob ONLY into the
  traditional planner's area propagation; extending it elsewhere is not this slice.
- Renderer flow/density/interface semantics (G-10/G-11/G-12/G-13/G-18, base-interface role,
  regularize consolidation) — owned by 238c; consumed as its output state.
- Raft geometry (`RaftPlan` consumer, raft keys, signed negative layers) — owned by
  `240a-support-raft-substrate` / `240b-support-raft-module` (packet 240 was split; there is no
  bare `240-support-raft` directory).
- Independent support-layer Z (`is_same_z_entity` filter, off-grid entities) — owned by 239.
- The EdgeGrid/SDF branch (`SUPPORT_USE_AGG_RASTERIZER` compiled-out path) — canonical itself
  does not use it; we port the active AGG path only, no dual implementation behind a cfg.
- Orca toolpath identity: behavioral parity measured by wall-leak/column-continuity deltas and
  block counts, not byte-equal G-code (plan §15).
- `docs/DEVIATION_LOG.md` edits and `docs/07` queue-table edits beyond the TASK registration
  gate step.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §12 brief, §3 Rulings 7/8, §6
  invariant 16, §7 E1–E9, §8 human gate, §13 traps T1/T4/T5/T7. Ranged reads (~755 lines).
- `docs/specs/support-parity-gap-register.md` - G-07 row (corrected premise); direct range read.
- `docs/19_visual_debug.md` - bundle contract for human-gate taps; ranged read.
- `docs/15_config_keys_reference.md` - regenerated output target; consult table format around
  the existing `support_*` rows only.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — class `SupportGridPattern` constructor (`smsGrid` branch: oversampling formula `std::clamp(int(scale_(m_support_spacing) / (extrusion_width_scaled + 100)), 1, 8)`, `m_pixel_size = max(extrusion_width_scaled + 21, scale_(m_support_spacing / oversampling))`, bbox grid alignment + one-pixel offset, macro-block arithmetic, `rasterize_polygons` for support and trimming polygons, `seed_fill_block(m_grid2, …, dilate_trimming_region(…))`); static `rasterize_polygons` (gray8 scanline even-odd fill — the semantics replicated on PnP coordinates); static `contours_simplified` (boundary-edge collection, lexicographic chaining, `fill_holes` left/right+top/bottom rule, `assert(abs(2*offset) < pixel_size - 10)` in-cell bound); `extract_support` (trimming difference → islands, `island_samples` containment filter, expanding-vs-shrinking sample set choice by `offset_in_grid` sign); statics `dilate_trimming_region` (3×3 all-set mask) and `seed_fill_block` (top-down/bottom-up/left/right propagation steps gated by the dilated mask).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportGridParams` (`grid_resolution`, `expansion_to_propagate` vs `expansion_to_slice` distinction, `extrusion_width`, `support_closing_radius`, `support_angle`, style) and `SupportMaterialStyle` enum mapping (`smsDefault`→`smsGrid`; tree styles coerced to `smsGrid`).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1`..`AC-8` (knob declaration; canonical grid formulas; extraction + island
  filtering; in-cell restriction; default routing; wall-leakage non-regression guard;
  column-continuity measurement on `SupportAdversarial.stl`; both-modes divergence).
- Negative: `AC-N1` (invalid knob value rejected at module boundary, never defaulted);
  `AC-N2` (explicit `legacy_semantic` keeps every existing planner behavior green) —
  **AC-N2 is RED and stays RED. Measured 2026-09-03:
  `cargo test -p traditional-support-planner --test traditional_family_tdd` reports
  26 passed / 2 failed** (`coarse_same_region_sources_keep_distinct_body_membership`,
  `coarse_source_preference_keeps_mixed_source_memberships`). Both assert two
  `SupportPlanEntry` values sharing one `(global_layer_index, object_id, region_id)` identity —
  forbidden by `docs/02_ir_schemas.md` § "IR 9b — SupportPlanIR" and rejected by
  `SupportPlanIR::duplicate_region_identity` (`crates/slicer-ir/src/slice_ir.rs`) at
  `Blackboard::commit_support_plan` (`crates/slicer-runtime/src/blackboard.rs`) — and they fail
  because `merge_region_identity_entries`
  (`modules/core-modules/traditional-support-planner/src/lib.rs`) correctly collapses the pair.
  By binding human decision the tests are NOT rewritten in this packet; the fix is packet
  `241b-support-plan-ownership-seam`. Filed as DEV-167. Full statement: `packet.spec.md`
  §Negative Test Cases. **Packet 241 closes NARROW and NOT GREEN.**
- Cross-packet impact: confined to `modules/core-modules/traditional-support-planner/**`, the
  two doc files above, and the runtime test surface
  (`crates/slicer-runtime/tests/integration/{main.rs,support_agg_rasterizer_tdd.rs}` plus the
  new `crates/slicer-runtime/tests/fixtures/golden/` directory, which does not yet exist and
  must be created). No production `crates/**` code changes. 238c / 239 / 240a / 240b packets' directories
  are untouched. The knob adds one
  key to the shared config surface — no WIT, IR, or schema-version change.

### Recorded metrics appendix (Step 14) — measured 2026-09-03, clamp REMOVED

> **Provenance.** Every number in this section was measured on 2026-09-03 in a session run
> with the asymmetric printed-area clamp REMOVED from the `RasterizerMode::Agg` arm of
> `SupportPlanner::plan_candidate`
> (`modules/core-modules/traditional-support-planner/src/lib.rs`) and with `legacy_semantic`
> as the DEFAULT (`agg` opt-in). Guest freshness was confirmed first:
> `cargo xtask build-guests --check` exited `0` (`EXIT_FRESH`). The previous appendix
> (Step 7) was measured UNDER THE CLAMP with `agg` as the default; those figures are
> superseded and are summarised as history at the end of this section, never as current
> evidence.

Fixture: `crates/slicer-runtime/tests/fixtures/support-family/SupportAdversarial.stl`
(`adversarial_mesh`: three roof-edge-slot blocks; regenerable via the ignored
`p241_generate_adversarial_fixture` in
`crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`); config
`orca-matched-config.json`, `support_object_xy_distance` 0.35 mm, line width 0.4 mm;
26 body layers in both modes. The `legacy_semantic` reference is the tracked baseline
`crates/slicer-runtime/tests/fixtures/golden/p241_baseline.json`, re-verified reproducible in
this same session by `support_agg_rasterizer_tdd::p241_metric_helpers_agree_on_baseline_fixture`
(PASS).

- **AC-6 wall leakage** — Round join, noise floor `WALL_LEAKAGE_NOISE_FLOOR_UNITS2` = 10_000
  units² = 1e-4 mm². `legacy_semantic`: **0 events / 0.0 units²**. `agg`: **0 events /
  0.0 units²**. Test `agg_wall_leakage_measurement_beats_baseline` — **PASS**.
- **AC-7 column continuity (re-authored, Step 15b)** — test
  `agg_column_continuity_measurement_beats_baseline` — **PASS**. Measured 2026-09-03,
  `cargo xtask build-guests --check` exit `0`.
  - **Drops (gate, unchanged):** `legacy_semantic` **3 abrupt drops**, `agg` **0**. Strictly
    fewer. PASS.
  - **Total emitted support area (RECORDED measurement, no longer a gate):**
    `legacy_semantic` **225789129333 units² (2257.89 mm²)**, `agg` **354695221947 units²
    (3546.95 mm²)**, delta **+57.09 %**. The former ±25 % guard was **retired, not widened**:
    the accepted canonical `seed_fill_block` macro-block halo (DEV-166) adds material by design,
    so an area-inflation guard contradicts the accepted behaviour. See the AC-7 entry in
    `packet.spec.md`.
  - **Macro-block containment bound (new gate).** Derived from the config the run actually used
    — spacing 2.5 mm, `line_width` 0.4 mm → `grid_resolution` **25000 units**, width **4000
    units**, `oversampling` **6**, `pixel_size` **4167 units**, **extent = 25002 units
    (2.5002 mm)**. `MacroBlockExtent::assert_consistent` re-derives every one of those from the
    two mm inputs, so the bound moves with the profile rather than being pinned.
  - **Result: containment HOLDS on all 26 compared layers.** Max per-layer area of `agg` outside
    `legacy` grown by the extent = **0.0 units²**; max *unfiltered* outside area (no sliver
    floor) = **0.0 units² over 0 difference pieces** — `difference_ex(agg, grown_legacy)`
    returns nothing at all, of any size. `CONTAINMENT_SLIVER_FLOOR_UNITS2` (10_000 units² =
    1e-4 mm², the same measured basis as the AC-6 floor) is therefore **inert** on this fixture
    and was not fitted; the result is identical with or without it. `agg_only_layers` = **[]**
    (no layer where `agg` emits support and `legacy` does not).
  - **Measured margin.** Bisected (0.01 mm resolution) smallest grow that actually contains
    `agg` on the worst layer: **22754 units (2.2754 mm)**, on layer 0. Margin under the derived
    extent: **2248 units (0.2248 mm)**, ≈ 9.0 % of one macro block. The grow is non-vacuous —
    the test asserts `required_grow_units > 0`, i.e. `agg` is *not* already inside the ungrown
    legacy region — and is not saturated at the 4× search ceiling.
- **AC-8 both-modes divergence** — `legacy_semantic` 26 body layers, `agg` 26 body layers,
  **26 of 26 layers diverge** (layers 0–25). **Both modes reach the plate**: each mode's lowest
  `SupportBody` layer equals the lowest printable layer. Both modes also drove a full
  `run_slice` to completion emitting a `;TYPE:Support` block on both `SupportTest.stl` and
  `SupportAdversarial.stl`. Test `agg_and_legacy_modes_both_function_and_diverge` — **PASS**.
- **F-I1 control, re-measured and INVERTED** — test renamed
  `support_agg_rasterizer_tdd::agg_printed_area_matches_global_offset_control` →
  **`support_agg_rasterizer_tdd::agg_printed_area_exceeds_global_offset_control`**, because the
  property it asserts changed sign when the clamp was removed.
  - CONTROL is unchanged: `difference(offset(legacy_body_layer, 2001 units Miter),
    offset(occupancy_layer, 0.35 mm Miter))` — the legacy plan grown by `offset_to_slice` and
    clipped to the trimming mask, with no grid anywhere in it.
  - Drops: legacy **3**, control **0**, agg **0**. Areas (mm²): legacy **2257.89**, control
    **2562.83**, agg **3546.95**.
  - Grid contribution `agg − control` = **+984.12 mm² (+38.40 % of the control)**. Global-offset
    contribution `control − legacy` = **+304.94 mm²**. The grid now contributes **~3.2×** what
    the global offset does — under the clamp it contributed 0.027 % as much.
  - Max per-layer symmetric difference control vs agg = **38.1980 mm²**; **26 of 26** compared
    layers differ by more than 1e-3 mm².
  - Interpretation: the column-continuity gain (3 → 0 drops) is still fully reproduced by the
    global offset alone — agg does not beat the control on drops. What changed is that the grid
    is no longer a rounding-scale perturbation of that control: unclamped, its macro-block halo
    dominates the area budget, which is exactly the AC-7 overshoot above. The rewritten test
    asserts these measured facts (agg strictly exceeds the control; grid contribution exceeds
    the offset contribution; every compared layer differs; max per-layer symdiff above a
    1 mm² block-scale floor) — **PASS**.
  - Caveat retained from Step 7: control occupancy comes from the test's `ExactZQueryService`
    while the planner uses `support_analysis` `occupancy_at`; part of the residual may be that
    difference.

**Not re-measured in this session** (Step-7, clamp-era, treat as unverified against the current
code): the pre-floor tangency-sliver counts and per-piece areas; the abrupt-drop layer indices
and sliver dimensions; the rejected-fixture candidate table; and the wedge plate-layer body
areas quoted on `agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`. The corresponding doc
comments in `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` are now
explicitly labelled CLAMP-ERA. (Crate-qualified deliberately: the bare basename is ambiguous
against the module-side
`modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`.)

**Qualitative disclosures that survive the re-measurement** (they concern fixture selection, not
clamp-dependent magnitudes):

- Review F-I2/F-I3: AC-7's "strictly fewer drops" is demonstrated on a fixture SELECTED for
  having a free-edge sliver (roof-edge slots). Fully clearance-bounded pockets
  (`stepped_pocket`) measured 3/3 drops in BOTH modes under the clamp-era run, so the port does
  not remove the general mid-air-drop symptom — only the free-edge case. The metric's
  `landed_on_model` test counts any non-empty intersection with the occupancy below; no
  minimum-overlap threshold is applied.
- Wedge-level functioning proof (traditional family) is the Step-8 test
  `support_agg_rasterizer_tdd::agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`
  (non-empty plan reaching beneath the overhang; not a collision test). **OPEN FAILURE as of
  2026-09-03: this test is RED under `agg`, failing with `SupportPlanIR contains duplicate
  entries for support region (layer=0, object=…, region=0)`. Tracked as Step 17 in
  `implementation-plan.md`, IN PROGRESS, owned by a parallel worker. The packet is NOT ready
  for the Human Validation Gate.** The tree-family
  invariant `support_segments_stay_outside_the_model_and_within_the_build_volume` never touches
  the knob and is not cited as evidence for this packet.
- Human Validation Gate, RESOLVED 2026-09-03: no fixture in this packet shows the grid changing
  a column-continuity outcome that the global offset does not already change, so the default
  stays `legacy_semantic` and `agg` ships opt-in. The packet's value claim rests on canonical
  parity for users who explicitly opt in, not on a measured benefit over legacy+offset. The
  Step-14 numbers above do not disturb this: agg still matches, and does not beat, the control
  on drops.

**Clamp-era history (superseded, Step 7).** The removed appendix recorded, under the asymmetric
clamp with `agg` as the default: AC-6 zero on both modes; AC-7 legacy 3 drops vs agg 0 drops with
an area delta of +13.54 % (then inside the ±25 % guard); AC-8 26/26 layers diverging; and an F-I1
control in which `agg − control` was 0.69 mm² (0.027 %) with a 0.0266 mm² max per-layer symdiff,
i.e. the grid's contribution was negligible. Those figures describe the clamped implementation
only and must not be quoted as current evidence.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -ge 6 && echo PASS` | Full guest rasterizer suite (AC-2..AC-5, AC-N1) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS` | Legacy-mode regression guard (AC-N2) — **CURRENTLY RED: 26 passed / 2 failed, measured 2026-09-03; left red by binding human decision, fix owned by `241b-support-plan-ownership-seam` (DEV-167)** | FACT pass/fail |
| `( cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_wall_leakage_measurement_beats_baseline --exact --nocapture 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_column_continuity_measurement_beats_baseline --exact --nocapture 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_and_legacy_modes_both_function_and_diverge --exact 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && echo PASS` | Measurement-as-gate trio on real fixture slices (AC-6..AC-8); three exact commands chained with `&&` — Cargo accepts one TESTNAME per invocation; names MUST be module-qualified (`support_agg_rasterizer_tdd::<name>`) because a bare name with `--exact` matches 0 tests in the aggregated `integration` binary | FACT pass/fail + recorded metric numbers |
| `cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_printed_area_exceeds_global_offset_control --exact --nocapture 2>&1 \| tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS` | F-I1 control: agg vs a legacy+`offset_to_slice` control with no grid in it (the measurement behind the default-mode decision; the name is module-qualified because a bare name with `--exact` matches 0 tests in the aggregated `integration` binary) | FACT pass/fail + recorded metric numbers |
| `cargo xtask build-guests --check && echo FRESH` | Guest freshness gate (E4/T4) before any attribution | exit code 0 + FRESH |
| `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo xtask check-literals` | Closure gates | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step 1 (baseline) MUST land before any behavior change; Steps 2+ build on a recorded,
  committed baseline artifact. Reversing the order invalidates AC-6/AC-7 comparisons. The
  baseline fixture is `SupportAdversarial.stl` (see In Scope), not the 30x20 box.
- Every step touching `modules/core-modules/traditional-support-planner/**` runs
  `cargo xtask build-guests --check` before attributing any failure (T4/E4); rebuild without
  `--check` if stale.
- Metric numbers quoted anywhere in docs/tests must come from a logged run
  (`target/test-output.log` or the baseline artifact), never estimated (No Unverified Metrics).
- SUPERSEDED 2026-09-03: the original expectation was that the knob's default flips to `agg`
  in the same step that rewires propagation. The default is now `legacy_semantic` (binding human
  decision); routing is proved by EXPLICIT `agg` selection in AC-5's test instead, so no step
  leaves code that nothing can route through.

## Context Discipline Notes

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` is ~3.3k lines — delegate
  everything; ranged reads only via LOCATIONS/SUMMARY returns (T1: verify existence by direct
  listing, globs miss gitignored paths).
- `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` is **2466
  lines** with **28** `#[test]` functions (verified 2026-09-03; AC-N2's `-gt 10` guard is
  therefore satisfiable). Ranged reads only — helpers at the top, then targeted tests. Note the
  28 reconciles with the AC-N2 result recorded above: 26 passed + 2 failed.
- Do NOT load `target/`, golden fixture bodies, or generated WASM bindings.
