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
depends_on: 238c-support-renderer-flow-interfaces
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: M
---

# Packet Contract: 241-support-agg-rasterizer

## Closure Status (2026-09-03) — CLOSED BY HUMAN OVERRIDE, NOT GREEN

**Packet 241 is CLOSED (`status: implemented`) by explicit human decision, shipping with two
known test failures.** This is an OVERRIDE of the Packet Completion Gate, not a gate that
passed. The human directing this session was told AC-N2 is red and the Human Validation Gate
is unsigned, and elected to close anyway ("It will ship out with the 2 reds, I know that").
Recorded here so no later reader mistakes closure for green.

- The **Packet Completion Gate is NOT MET** — see `implementation-plan.md` §Packet Completion
  Gate, which records the same verdict. Closure did not satisfy it; closure overrode it.
- **AC-N2 is RED and stays RED.** Two tests in
  `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` fail; the
  suite measures 26 passed / 2 failed. Full statement of the failure and its verified cause is
  in §Negative Test Cases at AC-N2. By binding human decision the tests are NOT rewritten here.
- The **Human Validation Gate is UNSIGNED** (§Human Validation Gate). Its prerequisites are not
  met: the dry-run artifacts on disk are clamp-era and were never regenerated, and AC-N2 is red.
- What DID land and is measured green: the `agg` grid rasterizer port, its `support_area_rasterizer`
  knob, the guest rasterizer suite, the AC-6/AC-7/AC-8 measurement trio, and the closure gate set
  (`implementation-plan.md` Steps 14 and 16).
- The unresolved producer defect behind AC-N2 — the traditional planner publishing one
  `SupportPlanEntry` per candidate per layer, so several entries can share one
  `(global_layer_index, object_id, region_id)` identity — is filed as **DEV-167** in
  `docs/DEVIATION_LOG.md`, together with the interim module-side merge that unblocks `agg` on
  real meshes. **Ownership of the real fix transfers to packet
  `241b-support-plan-ownership-seam`.**

## Goal

Port the canonical `SupportGridPattern` AGG rasterization path (`SupportMaterial.cpp`) into the
traditional planner's area propagation as `support_area_rasterizer = agg` (canonical, OPT-IN),
keeping the current propagate-without-growth semantic as `legacy_semantic` — which is the
DEFAULT (binding human decision, 2026-09-03; recorded in §Human Validation Gate) — with
before/after wall-leakage (collision freedom) and column-continuity (coverage) measurements as
the acceptance gate (plan §3 Rulings 7/8, §7 E1/E2).

## Scope Boundaries

The traditional planner's per-layer area propagation only:
`modules/core-modules/traditional-support-planner/` — one new grid-rasterizer module in the
guest, one manifest knob, and the propagation loop that consumes it. Renderer flow/density is
238c (done there); tree-side rasterization does not exist canonically for tree styles (canonical
maps every tree style to `smsGrid`, but this packet wires the knob only where PnP's traditional
planner propagates area); raft is 240a-support-raft-substrate / 240b-support-raft-module;
independent support-layer Z is 239-support-independent-layer-z.

## Prerequisites and Blockers

- Depends on: `238c-support-renderer-flow-interfaces` — SATISFIED. Verified 2026-09-03:
  `docs/spec_packets/238c-support-renderer-flow-interfaces/packet.spec.md` frontmatter reads
  `status: implemented`, as do `236-support-stabilization`, `238a-support-pattern-config-keys`,
  and `238b-tree-planner-canonical-fidelity`. The chain 236 → 238a → 238b → 238c is fully
  landed, so no forward dependency blocks activation. This is a ledger fact — re-derive it at
  activation (`head -3` each dep's `packet.spec.md`) rather than trusting this line.
- Unblocks: `242-support-family-orca-closure`.
- Activation blockers: none beyond the dependency above; `[BLOCK]`-tagged questions live in
  `design.md` §Open Questions.

## Acceptance Criteria

- **AC-1 (knob declared).** Given
  `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`,
  **when** its `[config.schema]` is inspected, **then** a `support_area_rasterizer` table
  exists with `type = "enum"`, `values = ["agg", "legacy_semantic"]`,
  `default = "legacy_semantic"` (changed from `"agg"` by binding human decision 2026-09-03:
  `agg` ships opt-in) — following the manifest enum pattern of `retract_mode`
  (`modules/core-modules/path-optimization-default/path-optimization-default.toml`) — and
  `docs/15_config_keys_reference.md` names the key with its two values (T8: declaration +
  doc regen in one commit). The pipe-suffixed command below greps the schema-table header, the
  values list, and the config-reference row; it does NOT assert the `default` value, so the
  default flip does not affect it and it is left unchanged. | `rg -q '^\[config\.schema\.support_area_rasterizer\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q '"agg", "legacy_semantic"' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'support_area_rasterizer' docs/15_config_keys_reference.md && echo PASS`
- **AC-2 (grid construction is canonical).** Given the new guest-side rasterizer module, **when**
  support polygons are projected onto the byte grid at default config, **then** the construction
  matches canonical `SupportGridPattern`'s `#ifdef SUPPORT_USE_AGG_RASTERIZER` branch exactly at
  PnP scale: pixel size `max(extrusion_width_scaled + 21, spacing_scaled / oversampling)` with
  `oversampling = clamp(spacing_scaled / (extrusion_width_scaled + 100), 1, 8)`, macro blocks of
  `oversampling × oversampling` cells, a one-pixel empty boundary ring, and seed-fill over each
  macro block up to the 3×3-dilated trimming mask (`seed_fill_block` + `dilate_trimming_region`
  semantics, four-direction propagation steps). | `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd grid_construction_matches_canonical_formulas -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-3 (contour extraction + island filtering).** Given a filled byte grid, **when**
  contours are extracted, **then** the extraction chains cell-boundary edges into closed loops
  (marching-squares equivalent of canonical `contours_simplified`), honors `fill_holes`
  left/right + top/bottom neighbor filling, applies `offset_in_grid` expansion/shrink via the
  loop offset, splits islands by the trimming polygons via
  `host::clip_polygons(.., ClipOperation::Difference)` (canonical uses `difference_ex`; in
  this tree that symbol is NOT re-exported by `slicer-sdk` and `slicer-core` is not a
  dependency of this module, so it is unavailable here — note it is NOT a host/guest boundary:
  `slicer_core::polygon_ops` is ungated and does compile to wasm32, as `arachne-perimeters`
  demonstrates. We route through the SDK host op rather than adding a `slicer-core`
  dependency, keeping this module's dependency surface unchanged), and keeps only islands
  containing an input-island sample point (canonical `extract_support`'s sample-containment
  filter — the column-continuity fix from upstream `a95607d7bf`). | `cargo test -p traditional-support-planner --test agg_rasterizer_tdd contour_extraction_filters_islands_by_samples -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-4 (in-cell expansion restriction).** Given an extracted layer polygon with positive
  `expansion_to_slice`, **when** the printed area is derived, **then** expansion happens inside
  each oversampled macro cell during extraction (per-cell `offset_in_grid`), never as a global
  polygon offset — the wall-leakage fix from upstream `fb7b995050`. No `host::offset_polygons`
  call may appear inside `agg_raster.rs` at all: island-sample generation is the one step that
  needs an offset, and it runs at the `lib.rs` call site and is passed into `extract_support`
  as its `samples` argument, so the rasterizer module is offset-free by construction. | `( ! rg -q 'offset_polygons' modules/core-modules/traditional-support-planner/src/agg_raster.rs ) && mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd expansion_is_restricted_inside_the_macro_cell -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-5 (explicit `agg` selection routes propagation through the rasterizer).** Given config
  that EXPLICITLY selects `support_area_rasterizer = "agg"` (this is no longer the default —
  the default is `"legacy_semantic"`), **when** the traditional planner propagates a
  contact region downward through ≥ 2 layers against model occupancy, **then** the emitted
  per-layer body comes from the rasterizer path (contact polygons stretched into the grid,
  trimmed by occupancy, re-extracted), and interface anchoring and demand/body ID threading are
  unchanged from today. Termination-layer bookkeeping is deliberately NOT asserted here — see
  the divergence note below. The historical test name `default_config_routes_propagation_through_rasterizer`
  was renamed to `default_config_routes_propagation_through_legacy_semantic` by the default
  flip (Step 12); it now asserts BOTH that an absent key routes to `legacy_semantic` and, via
  its `explicit_agg_run` arm, that an explicit `"agg"` selection routes through the rasterizer,
  which is what this criterion gates. | `cargo test -p traditional-support-planner --test agg_rasterizer_tdd default_config_routes_propagation_through_legacy_semantic -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`

  **Known divergence under `agg` (deliberately NOT an acceptance criterion; filed as DEV-166 in
  `docs/DEVIATION_LOG.md`).** Termination-layer bookkeeping is NOT preserved when `agg` is
  selected. Canonical `seed_fill_block` (`SupportMaterial.cpp`) floods each
  `oversampling × oversampling` macro block independently, growing the carry by up to one
  macro-block extent, so the inflated carry routes around an obstacle that would otherwise close
  every route: PnP's structured `SupportPlanDeclineReason::NoRoute` / diagnostic `code: 1203`
  decline does not fire under `agg` when the blocking occupancy is LOCAL (it still fires when occupancy covers the whole grid neighbourhood, since seed fill is then blocked everywhere and the carry genuinely empties), and the block-snapped halo crosses PnP's
  per-region foreign-territory bar. Canonical has no decline concept at all — when trimming
  closes every route, `diff(carry, trimming)` goes empty before rasterization and the caller
  simply skips the lower layers. Both invariants are retained under the DEFAULT
  `legacy_semantic` mode, which AC-N2 gates.

- **AC-6 (wall-leakage non-regression guard — collision freedom).** Given
  `SupportAdversarial.stl` sliced via `run_slice` before and after this packet (self-captured
  baseline vs post-port, same config), **when** support body outlines are tested against
  per-layer model occupancy grown by `support_object_xy_distance` (Round join — Euclidean xy
  clearance), **then** the post-port run measures **zero penetration events** above the
  documented noise floor for clipper tangency slivers — `WALL_LEAKAGE_NOISE_FLOOR_UNITS2` =
  10_000 units² = 1e-4 mm², applied **per intersection piece** — and a
  penetrated-area sum **not greater than** the pre-port baseline recorded in
  Step 1 (E1: measured numbers recorded in the test output and `requirements.md`; existence
  checks do not satisfy this AC). This is a guard, not an improvement claim: the legacy loop's
  per-layer Miter-grown occupancy difference in `SupportPlanner::plan_candidate` already
  reproduces `fb7b995050`. Wedge-level functioning proof for the traditional family is the
  Step-8 test `support_agg_rasterizer_tdd::agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`
  (a non-empty-plan/reach proof, NOT a collision test) — **that test is currently RED under
  `agg`; see `implementation-plan.md` Step 17.**

  **Measured result (Step 14, 2026-09-03, clamp REMOVED):** `legacy_semantic` **0 events /
  0.0 units²**; `agg` **0 events / 0.0 units²**. No penetration slivers were observed at all,
  of any size, in either mode, so the noise floor is inert on this fixture and was not fitted.
  An earlier draft of this criterion quoted "observed slivers 88–311 units², ≥ 32× below the
  floor; identical in both modes" — those figures are **CLAMP-ERA and do not describe the
  current tree**; they are superseded by the 0/0 result above. | `cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_wall_leakage_measurement_beats_baseline --exact --nocapture 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-7 (column-continuity measurement — coverage; re-authored Step 15b).** Given the same
  `SupportAdversarial.stl` runs (the tracked 30x20 box measures 0/0 drops and cannot gate this
  AC), **when** per-column coverage across consecutive layers is compared (column = connected
  body component tracked down-layer), **then** both of the following hold:
  1. the post-port run has **strictly fewer abrupt column drops** than the pre-port baseline
     recorded in Step 1 (columns "missing abruptly when going down" is the upstream `a95607d7bf`
     symptom) — unchanged;
  2. on **every** layer carrying an `agg` support body, that layer's `agg` body region lies
     **inside the `legacy_semantic` body region for the same layer grown by one macro-block
     extent**, where the extent is *derived*, never hardcoded, from the config the run actually
     used: `oversampling = clamp(grid_resolution / (line_width_units + 1), 1, 8)`,
     `pixel_size = max(line_width_units + 1, mm_to_units(spacing / oversampling))`,
     `extent = oversampling * pixel_size` — the same arithmetic as `GridParams::from_polygons`
     (`modules/core-modules/traditional-support-planner/src/agg_raster.rs`), mirrored in
     `MacroBlockExtent` in the test and re-derived by `MacroBlockExtent::assert_consistent` so
     the bound moves if the profile moves. The grow uses a **Miter** join because canonical
     `seed_fill_block` (`SupportMaterial.cpp`) is block-local and separable per axis
     (Chebyshev), not radial.

  The metric's `landed_on_model` test counts ANY non-empty intersection with the occupancy below
  (a 0.1–0.2 mm overlap counts); there is no minimum-overlap threshold. Measured figures — the
  drops, the recorded total-area delta, the derived extent, the per-layer outside-the-bound area
  and the smallest grow that actually contains `agg` — are printed by the test and recorded in
  `requirements.md`.

  **Retired:** AC-7 previously also required total emitted support area to change by less than
  ±25% versus baseline "so continuity is not bought by inflation". That guard was **removed, not
  widened.** Its premise is contradicted by the accepted canonical behaviour: with the
  asymmetric printed-area clamp removed by human decision, canonical `seed_fill_block` floods
  each `oversampling * oversampling` macro block independently, so the support carry grows by at
  most one macro-block extent and **that halo adds material by design** (DEV-166). Measured
  2026-09-03: legacy 225789129333 units² (2257.89 mm²) vs agg 354695221947 units²
  (3546.95 mm²) = **+57.09 %**, which is the halo, not a defect. The containment bound above
  replaces it and is strictly stronger than any area ratio: a ratio cannot distinguish a
  block-scale halo from support appearing somewhere else entirely, while containment forbids the
  latter outright. The total-area figures remain a RECORDED measurement, not a gate. | `cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_column_continuity_measurement_beats_baseline --exact --nocapture 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-8 (both modes diverge measurably).** Given one fixture slice per mode
  (`support_area_rasterizer = agg` vs `"legacy_semantic"`) through `run_slice`, **when** both
  plans are compared, **then** they produce different body outline sets on at least one layer
  (proof the knob actually switches code paths), and BOTH runs complete with non-empty support
  plans reaching the plate beneath the fixture overhang. | `cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_and_legacy_modes_both_function_and_diverge --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`

Every AC names exact fields, paths, counts, or output fragments and ends with its own runnable
command. Commands tee to `target/test-output.log` with a non-zero matched-count guard
(invariant 16).

## Negative Test Cases

- **AC-N1 (invalid knob value rejected).** Given a config supplying
  `support_area_rasterizer = "marching_squares"` (not in the declared enum set), **when** the
  traditional planner module parses its config view, **then** it fails with a fatal
  `ModuleError` naming the key and the allowed values — the defense-in-depth pattern already used
  by `SeamPlacer::from_config` (`modules/core-modules/seam-placer/src/lib.rs`), which rejects an
  unknown `seam_mode` with `ModuleError::fatal` even though `seam_mode` is a manifest-declared
  enum. No silent fallback to either mode. NOTE: the host rejects out-of-vocabulary enum values
  first — `ConfigBoundsIndex::from_modules` harvests `values` from every loaded module's
  `[config.schema]` and `resolve_global_config` calls `bounds.check(..)?`, aborting the slice
  with `config resolution failed: …`. The host check is therefore NOT numeric-only. The guest
  check still carries weight (it fires for a `ConfigView` built directly, when no loaded module
  declares the key, and when a colliding declaration wins `or_insert_with`), so this AC's test
  drives `from_config` on a constructed `ConfigView` rather than a full slice. | `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd invalid_rasterizer_value_is_rejected_not_defaulted -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N2 (legacy mode still functions) — RED. THIS CRITERION FAILS ON THE CURRENT TREE AND
  IS NOT MET BY THIS PACKET.** Given `support_area_rasterizer = "legacy_semantic"`
  explicitly selected, **when** the planner runs the full propagation suite inputs (blocked
  route → structured decline; plate termination; top-z lowering), **then** all existing
  `traditional_family_tdd` assertions hold unchanged — proving Ruling 8's "prior behavior stays
  selectable" and guarding against silent degradation of the legacy path. |
  `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS`

  **Measured status, 2026-09-03: RED.** The suite
  `cargo test -p traditional-support-planner --test traditional_family_tdd` measures
  **26 passed / 2 failed** (28 `#[test]` functions in the file, consistent with the count
  recorded in `requirements.md` §Context Discipline Notes). The two failures are:

  - `coarse_same_region_sources_keep_distinct_body_membership`
  - `coarse_source_preference_keeps_mixed_source_memberships`

  both in `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`.

  **Cause — verified, not suspected.** Both tests assert that TWO `SupportPlanEntry` values share
  one `(global_layer_index, object_id, region_id)` identity. That shape is forbidden by
  `docs/02_ir_schemas.md` § "IR 9b — SupportPlanIR", which states that each `SupportPlanEntry`
  is produced once per `(global_layer_index, object_id, region_id)` triple, and it is rejected
  by `SupportPlanIR::duplicate_region_identity` (`crates/slicer-ir/src/slice_ir.rs`) at
  `Blackboard::commit_support_plan` (`crates/slicer-runtime/src/blackboard.rs`). The two tests
  fail because `merge_region_identity_entries`
  (`modules/core-modules/traditional-support-planner/src/lib.rs`) CORRECTLY collapses the pair
  each of them constructs. They encode the long-standing producer defect, not the contract.

  **By binding human decision (2026-09-03) these tests are NOT rewritten in this packet, and no
  assertion in them is softened.** Restoring their intent — keeping distinct body membership
  and mixed-source membership expressible without minting duplicate region identities —
  requires an ownership seam between candidate identity and region identity, and is the charter
  of packet `241b-support-plan-ownership-seam`. The producer defect and the interim module-side
  merge that unblocks `agg` on real meshes are filed as **DEV-167** in
  `docs/DEVIATION_LOG.md`.

  **AC-N2 is RED and stays red. Packet 241 closes NARROW and NOT GREEN.** This criterion may not
  be counted toward the Packet Completion Gate, and no `status: implemented` flip may rest on it.

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- Primary targeted proof: `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -ge 6 && echo PASS`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - governing plan; §12 brief
  "241-support-agg-rasterizer", §3 Rulings 7/8, §6 invariant 16, §7 E1–E9, §8 human gate,
  §13 traps T1/T4/T5/T7. Bounded ranged reads.
- `docs/specs/support-parity-gap-register.md` - row G-07 (premise corrected per Ruling 7;
  destination rerouted to this packet); direct range read.
- `docs/19_visual_debug.md` - visual-debug bundle contract for the human-gate taps; ranged
  read around `## Request Shape` and `### Tap Classes And Execution Closure`. The "Stage Tap
  Inventory" heading itself is NOT in this file — it lives in
  `docs/specs/_OLD/visual-pipeline-debug.md`, which `19_visual_debug.md` only references.
- `docs/specs/support-families-anchored-entities-plan.md` §17-agent debugging companion
  (`docs/17_agent_debugging.md`) - timing/DAG diagnosis boundaries; consult only if a gate
  command misbehaves.
- `docs/15_config_keys_reference.md` - regenerated, not read as authority.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` - add `support_area_rasterizer` row (enum, default
  `"agg"`, values `"agg"|"legacy_semantic"`, owner `traditional-support-planner`) after the
  manifest lands - `rg -q 'support_area_rasterizer' docs/15_config_keys_reference.md`
- `docs/07_implementation_status.md` - TASK-419..TASK-428 registered at packet-owned closure
  (Step 9) - `rg -q 'TASK-419' docs/07_implementation_status.md`. These IDs are RESERVED for
  this packet by queue row #8 of `docs/specs/support-families-anchored-entities-plan.md` and
  are below the live high-water mark in `docs/07_implementation_status.md`. Step 9 must verify
  the reserved range is still unused (`rg -o 'TASK-4(1[9]|2[0-8])' docs/07_implementation_status.md`
  returns nothing), NOT allocate the "next free" ID — that query returns a much higher number.
- `docs/DEVIATION_LOG.md` - row **DEV-166** filed 2026-09-03 and REWRITTEN 2026-09-03 after
  the clamp was rejected. It no longer documents a clamp; it records the ACCEPTED divergence:
  `agg` reproduces canonical macro-block snapping (`seed_fill_block` / `extract_support`,
  `SupportMaterial.cpp`), which prints support where PnP's demand model demands none, crosses
  PnP's foreign-territory bar, and prevents the structured `NoRoute` / `code: 1203` decline
  from firing. Resolution recorded in the row: `agg` is opt-in and NOT the default;
  `legacy_semantic` is the default and retains every PnP invariant (see design.md
  §Data and Contract Notes). The G-07 premise correction still lives in the gap register
  row itself. -
  `rg -q '^\| DEV-166 ' docs/DEVIATION_LOG.md`
- Queue table of `docs/specs/support-families-anchored-entities-plan.md` - orchestrator-owned;
  this packet does not touch it.

## Human Validation Gate

Blocking per plan §8. Artifacts to produce (all under `tmp/p241-*`, gitignored — verify by
direct listing, trap T1):

1. `tmp/p241-agg-fixture.gcode` — tracked fixture
   `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`, matched profile
   `tmp/support-family-config-normal-matched.json`, `agg` mode — which since the
   2026-09-03 default decision must be selected EXPLICITLY in the profile JSON
   (`"support_area_rasterizer": "agg"`); it is no longer what an unset key yields:
   `cargo run --bin pnp_cli --release -- slice --module-dir modules/core-modules --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-normal-matched.json --output tmp/p241-agg-fixture.gcode`
   (the `slice` subcommand's model flag is `--model` per `Cmd::Slice` in
   `crates/pnp-cli/src/main.rs`; the `--input` spelling in AGENTS.md is a doc-level alias
   example — use the flag the CLI actually parses). `--module-dir modules/core-modules` is
   REQUIRED on every `pnp_cli slice` / `visual-debug` command in this section: measured
   2026-09-03, without it the slice is degenerate on this machine (9 layers, no support);
   with it the agg fixture emitted 273 `;LAYER_CHANGE` and 123 `;TYPE:Support` blocks — those
   two counts were measured with the asymmetric clamp in place and with `agg` as the default,
   so they are STALE for the current code and must be re-measured from regenerated artifacts
   before the gate is signed (see the dry-run status note below).
2. `tmp/p241-legacy-fixture.gcode` — identical command (including
   `--module-dir modules/core-modules`) except the profile JSON carries
   `"support_area_rasterizer": "legacy_semantic"` → `tmp/p241-legacy-fixture.gcode`. This is
   now the DEFAULT mode; keep the key explicit anyway so the artifact stays unambiguous. BOTH
   modes inspected (plan §12 brief).
3. **Non-coplanar real-mesh case (T7 — mandatory):** slice `resources/regression_wedge.stl`
   through the full pipeline in `agg` mode — explicitly selected, not default — with
   `--module-dir modules/core-modules` → `tmp/p241-agg-wedge.gcode`.
4. Visual-debug bundle for THIS packet's boundary — wall-leakage tap (support body vs model
   occupancy at mid-height layers) and column-continuity tap (consecutive body layers), written
   under `tmp/p241-vd/` with its `manifest.json` (`pnp_cli visual-debug` invoked with
   `--module-dir modules/core-modules`).

**Dry-run status: the artifacts on disk are CLAMP-ERA and MUST be regenerated before the gate
is signed.** Artifacts 1–4 exist (`tmp/p241-agg-fixture.gcode`, `tmp/p241-legacy-fixture.gcode`,
`tmp/p241-agg-wedge.gcode`, `tmp/p241-vd/manifest.json`; verified by direct listing
2026-09-03 with `ls -l --time-style=full-iso`), but their mtimes are 16:32:57–16:33:13 local,
whereas the commit that removed the asymmetric clamp and flipped the default
(`reject DEV-166 clamp; agg opt-in, legacy_semantic default`) is dated 17:27:02 local. Every
one of them therefore describes the CLAMPED implementation with `agg` as the default — the
opposite of what ships — and none of them is evidence for the current tree.

They were deliberately NOT regenerated by the documentation pass that recorded this: the wedge
artifact's slice is blocked by the open Step-17 failure
(`agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`, duplicate support-region entries
under `agg`), under repair in parallel.

**Gate prerequisites (BLOCKING — the checklist below may not be signed until both are met):**

1. Step 17 is resolved (`implementation-plan.md`). As of 2026-09-03 it is resolved only
   TEMPORARILY: `merge_region_identity_entries`
   (`modules/core-modules/traditional-support-planner/src/lib.rs`) stops the `agg` wedge slice
   aborting, but it is a documented temporary unblock (DEV-167), the underlying producer defect
   is unfixed, and AC-N2 is RED because of it. A temporary unblock is NOT a resolution for
   sign-off purposes.
2. Artifacts 1–4 are REGENERATED on the post-clamp-removal tree, with `agg` selected
   EXPLICITLY in the profile JSON, and every checklist figure re-measured from the regenerated
   files. No number derived from the clamp-era artifacts may be entered on the checklist.

The gate itself is left UNSIGNED.

Checklist to sign (each item names source, layer, tap, verdict; per E2 written inspection,
never a test claim):

- [ ] Termination: columns reach the plate/model beneath their overhangs in BOTH modes; no
      column terminates short or passes through the model.
- [ ] Coverage: demanded overhang regions carry support on the fixture; no column vanishes
      abruptly going down in agg mode (the G-07 symptom).
- [ ] Collision freedom: no support intersects model walls in EITHER mode — inspect the
      wall-leakage tap at thin-wall layers (the legacy path already reproduces `fb7b995050`;
      this is a non-regression check, not a before/after leak fix).
- [ ] Interfaces: roofs/floors sit correctly in both modes (no regression vs 238c state).
- [ ] Block counts vs Orca references (REQUIRED): `;TYPE:Support material` block counts and
      total support-extrusion length for `tmp/p241-agg-fixture.gcode` vs
      `tmp/SupportTest_Normal_Orca.gcode` recorded in writing; delta stated, not guessed.
- [ ] Rasterizer-specific observations: wall-leak/column-continuity verdict per mode, with
      layer indices of any visible difference between `tmp/p241-agg-fixture.gcode` and
      `tmp/p241-legacy-fixture.gcode`.
- [x] Default-mode decision: **DECIDED 2026-09-03 (human, binding) — the default is
      `legacy_semantic`.** Rationale: the F-I1 control measured no benefit of the grid pipeline
      over a legacy+`offset_to_slice` control, and no discriminating fixture was found in this
      packet (that measurement was taken under the since-removed clamp — see the staleness
      banner on requirements.md §Recorded metrics appendix). With the clamp removed, `agg`
      does differ from `legacy_semantic` substantially, but the difference IS the canonical
      block-snapped halo, which PnP's demand model classifies as phantom support. `agg`
      therefore ships opt-in and `legacy_semantic` remains the default.
- [ ] Opt-in divergence acknowledged (NEW): confirm in writing that shipping `agg` is
      acceptable given that it is opt-in, UNCLAMPED (the asymmetric clamp is REJECTED by human
      decision and removed from the code), and knowingly diverges from two PnP invariants —
      the structured `NoRoute` / `code: 1203` decline, which does not fire under `agg` when the blocking occupancy is LOCAL (it still fires when occupancy covers the whole grid neighbourhood, since seed fill is then blocked everywhere and the carry genuinely empties),
      and the per-region foreign-territory bar, which the block-snapped halo crosses (DEV-166).

Sign-off: `_date_ _verdict_` (packet may not flip to `status: implemented` without it).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — class `SupportGridPattern`: constructor `smsGrid` branch (oversampling clamp formula, `m_pixel_size`, macro-block sizing, one-pixel boundary ring, `rasterize_polygons` calls, `seed_fill_block` over `dilate_trimming_region`), static `rasterize_polygons` (AGG gray8 scanline fill semantics being replicated), static `contours_simplified` (cell-edge collection, line chaining, `fill_holes` neighbor rule, `offset_in_grid` loop offset), `extract_support` (island split vs trimming polygons, `island_samples` containment filter, expansion-vs-shrink sample handling), static `seed_fill_block` / `dilate_trimming_region` (macro-cell 4-direction propagation, 3×3 dilation mask); instantiation site in the support-layers builder path (~`generate_support_layers` region) showing which callers pass `expansion_to_propagate` vs `expansion_to_slice`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportGridParams` field meanings (`grid_resolution`, `extrusion_width`, `support_closing_radius`, `support_angle`, style) consumed by the constructor.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
