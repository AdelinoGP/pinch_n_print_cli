# Implementation Plan: 241-support-agg-rasterizer

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (TASK-419..TASK-428, consecutive, no gaps).
- Use TDD (red test first), then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write
  "see Step 1".
- After any step touching `modules/core-modules/traditional-support-planner/**`, run
  `cargo xtask build-guests --check` and require exit 0 before attributing failures; rebuild
  without `--check` if stale (T4/E4). Stale-guest failures are your bug until proven otherwise.
- Every verification command tees to `target/test-output.log` and guards a non-zero matched
  count (invariant 16). Read results from the log; never re-run for more output.
- All `cargo check`/`clippy`/`test` gate commands use `--all-targets` where applicable.
- Metric numbers recorded in tests/docs must come from logged runs — never estimated
  (No Unverified Metrics; plan §7 E1).

## Steps

### Step 1: Pre-port baseline capture (measurement-as-gate foundation)

- Task IDs: `TASK-419`
- Objective: capture and commit the PRE-port wall-leakage and column-continuity baseline for
  the tracked fixture: penetration events + total penetrated area (vs occupancy grown by
  `support_object_xy_distance`) and abrupt-column-drop count + total support area, sliced via
  `run_slice` with the matched normal profile.
- Precondition: 238c implemented; `cargo xtask build-guests --check` exit 0; working tree
  clean at the pre-port commit.
- Postcondition: new test file
  `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` exists with
  `capture_pre_port_baseline` (writes `target/p241-baseline.json` via a `#[ignore]`d recording
  entry, mirroring the repo's `record_*` discipline) and metric helper functions; the baseline
  JSON is committed under `crates/slicer-runtime/tests/fixtures/golden/p241_baseline.json`;
  `main.rs` carries the `mod support_agg_rasterizer_tdd;` line.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - lines 1–200 (driver
    patterns: `support_test_path`, `matched_config_base`, `run_slice_for_family_with_interface_layers`)
  - `crates/slicer-runtime/tests/integration/main.rs` - lines 1–40 (mod-registration shape)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `crates/slicer-runtime/tests/fixtures/golden/p241_baseline.json` (generated)
- Files explicitly out of bounds:
  - `modules/**` (no behavior change yet), `OrcaSlicerDocumented/**`, `target/`, goldens other
    than the new baseline file
- Expected sub-agent dispatches:
  - Question: confirm `matched_config_base` key set and whether `support_object_xy_distance`
    is overridden anywhere in the closure tests; scope:
    `crates/slicer-runtime/tests/integration/support_family_closure.rs`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E1/E2 ranges
- OrcaSlicer refs:
  - none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- p241_metric_helpers_agree_on_baseline_fixture --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `test -s crates/slicer-runtime/tests/fixtures/golden/p241_baseline.json && echo BASELINE-CAPTURED`
- Exit condition: baseline JSON committed; helper test green; metric definitions (penetration
  event, penetrated area, column drop) documented in the test file header comment.

### Step 2: Canonical fidelity probe (read-only, pre-coding)

- Task IDs: `TASK-420`
- Objective: verify the port's constant set against the live checkout before coding:
  oversampling formula, pixel-size max-form, macro-block arithmetic, boundary-ring offsets,
  `seed_fill_block` propagation-step order, `dilate_trimming_region` mask, `contours_simplified`
  chaining + `fill_holes` rule, `extract_support` sample filter.
- Precondition: Step 1 committed.
- Postcondition: dispatch return recorded in this packet's `design.md` §Plan Corrections
  (append-only note) if ANY constant differs from the plan's pre-verified evidence; no code
  change.
- Files allowed to read, with ranges when over 300 lines:
  - none directly; all reads delegated
- Files allowed to edit (at most 3):
  - `docs/spec_packets/241-support-agg-rasterizer/design.md` (Plan Corrections note only)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**` (delegate), all source trees
- Expected sub-agent dispatches:
  - Question: SNIPPETS of the `smsGrid` constructor branch (oversampling + pixel-size +
    macro-block lines), `seed_fill_block` + `dilate_trimming_region` bodies, and
    `contours_simplified` edge-collection/chaining; scope:
    `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` class `SupportGridPattern`;
    return: `SNIPPETS` (≤30 lines ×3)
  - Question: FACT on `SupportGridParams` field semantics (`expansion_to_propagate` vs
    `expansion_to_slice`); scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp`;
    return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §3 Ruling 7 range
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - delegate; never load
- Verification:
  - `rg -q 'Plan Corrections|## Open Questions' docs/spec_packets/241-support-agg-rasterizer/design.md && echo PROBE-RECORDED`
- Exit condition: fidelity note recorded (or explicit no-diff note); dispatch returns ≤ caps.

### Step 3: Grid construction + rasterization (red-first)

- Task IDs: `TASK-421`
- Objective: create `agg_raster.rs` with `GridParams` derivation + `rasterize_polygons` +
  `dilate_trimming_region`, red tests first in `tests/agg_rasterizer_tdd.rs`.
- Precondition: Step 2 fidelity note recorded; guests fresh.
- Postcondition: `GridParams::from_polygons` reproduces canonical formulas on PnP units for a
  square island + thin-wall trimming fixture; `rasterize_polygons` fills an even-odd correct
  byte grid (boundary ring unset); `dilate_trimming_region` erodes the mask to the 3×3 all-set
  interior.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support-planner/src/lib.rs` - lines 30–150 (module
    imports/config conventions)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/src/agg_raster.rs` (new)
  - `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs` (new)
  - `modules/core-modules/traditional-support-planner/Cargo.toml` ([[test]] target)
- Files explicitly out of bounds:
  - `modules/core-modules/traditional-support-planner/src/lib.rs` (wiring is Step 5),
    `crates/**`, other modules
- Blast-radius discipline: new module, no existing struct literals change; `Cargo.toml` gains
  one `[[test]]` stanza — no test asserts the target list today (verify with
  `rg -n 'agg_rasterizer_tdd' modules/core-modules/traditional-support-planner/` before/after).
- Expected sub-agent dispatches:
  - none required; formulas come from Step 2's SNIPPETS
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - via coord-system constraint; ranged consult only
- OrcaSlicer refs:
  - `SupportMaterial.cpp` `SupportGridPattern` constructor + statics - delegate (Step 2 return)
- Verification:
  - `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd grid_construction_matches_canonical_formulas -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: construction tests green; guests fresh; no `lib.rs` change yet.

### Step 4: Seed fill + contour extraction + island filter (red-first)

- Task IDs: `TASK-422`
- Objective: add `seed_fill_block`, `contours_simplified` (with `fill_holes` +
  `offset_in_grid`), and `extract_support` (trimming difference, sample containment,
  expanding-vs-shrinking sample choice); red tests first.
- Precondition: Step 3 green.
- Postcondition: seed fill closes gaps inside macro cells up to the dilated mask; extraction
  chains closed loops; `fill_holes` bridges 1-cell holes; positive/negative `offset_in_grid`
  stays inside the cell (debug assert); islands without samples are dropped — including the
  regression case where naive extraction drops a column that narrows to a sub-cell sliver
  (the `a95607d7bf` symptom).
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/src/agg_raster.rs`
  - `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`
  - `modules/core-modules/traditional-support-planner/Cargo.toml` (only if test deps needed)
- Files explicitly out of bounds:
  - `src/lib.rs`, `crates/**`, other modules
- Expected sub-agent dispatches:
  - Question: confirm ray-crossing containment + `island_samples` choice semantics; scope:
    `SupportMaterial.cpp` `extract_support`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - none beyond Step 2 record
- OrcaSlicer refs:
  - `SupportMaterial.cpp` `extract_support` / `contours_simplified` / `seed_fill_block` -
    delegate; never load
- Verification:
  - `cargo test -p traditional-support-planner --test agg_rasterizer_tdd contour_extraction_filters_islands_by_samples -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `cargo test -p traditional-support-planner --test agg_rasterizer_tdd expansion_is_restricted_inside_the_macro_cell -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: extraction suite green; guests fresh.

### Step 5: Knob declaration + config parse + rejection (red-first)

- Task IDs: `TASK-423`
- Objective: declare `[config.schema.support_area_rasterizer]` (enum,
  `values = ["agg", "legacy_semantic"]`, `default = "agg"`) in the manifest; parse it in
  `from_config` into a `RasterizerMode` field; reject out-of-vocabulary strings with a fatal
  `ModuleError` naming key + allowed values; regenerate
  `docs/15_config_keys_reference.md`.
- Precondition: Steps 3–4 green.
- Postcondition: AC-1 and AC-N1 hold; unset key resolves to `Agg`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml` - lines
    50–62 (enum declaration pattern)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
  - `modules/core-modules/traditional-support-planner/src/lib.rs` (parse block ONLY — no
    propagation rewiring this step)
  - `docs/15_config_keys_reference.md` (regen)
- Files explicitly out of bounds:
  - propagation loop body, other modules, `crates/slicer-scheduler/**` (host bounds index is
    numeric-only; the module-boundary rejection is the enforcement point — do not extend the
    scheduler)
- Blast-radius discipline: `rg -n 'support_area_rasterizer' modules/ crates/ docs/` before
  editing to confirm zero prior references; `rg -n 'from_config' modules/core-modules/traditional-support-planner/tests/` to catch config-shape assertions; fix any fallout in-step.
- Expected sub-agent dispatches:
  - Question: current `[config.schema]` tail of the manifest + any test asserting manifest key
    counts; scope: `modules/core-modules/traditional-support-planner/`; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - §Config Field Types Reference (enum row) ranged read
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q '^\[config\.schema\.support_area_rasterizer\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q '"agg", "legacy_semantic"' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'support_area_rasterizer' docs/15_config_keys_reference.md && echo PASS || echo FAIL`
  - `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd invalid_rasterizer_value_is_rejected_not_defaulted -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: declaration + parse + rejection green; doc regen committed; guests fresh.

### Step 6: Propagation rewiring — agg default, legacy selectable

- Task IDs: `TASK-424`
- Objective: branch `plan_candidate`'s propagation loop on the mode: `Agg` builds the grid from
  the contact carry (support polygons) and occupancy-grown trimming mask, extracts per-layer
  print area (`expansion_to_slice`) and propagation carry (`expansion_to_propagate`), preserves
  the empty-carry diagnostic 1203 + structured `NoRoute` decline; `LegacySemantic` keeps the
  current loop body verbatim.
- Precondition: Step 5 green.
- Postcondition: AC-5 and AC-N2 hold; default config routes through the rasterizer; the full
  existing `traditional_family_tdd` suite passes unchanged under explicit legacy selection AND
  under default (where its geometric assertions are re-verified against agg output — any
  assertion that legitimately tightens is updated in this step with the measured new value, not
  weakened).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support-planner/src/lib.rs` - lines 300–470 (loop) +
    30–150 (parse)
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` - lines
    1–245 (helpers) + targeted failing tests on demand
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/src/lib.rs`
  - `modules/core-modules/traditional-support-planner/src/agg_raster.rs` (integration glue)
  - `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`
    (routing test)
- Files explicitly out of bounds:
  - `tests/traditional_family_tdd.rs` EXCEPT assertion-value updates forced by agg-default
    output (each update must cite the measured value in a comment); other modules; `crates/**`
- Blast-radius discipline: dispatch a `LOCATIONS` worker for every test asserting body outline
  geometry or diagnostic 1203 BEFORE editing (design.md §Expected Sub-Agent Dispatches);
  pre-bake the fallout list into this step's edits; no follow-up `cargo check` discovery.
- Expected sub-agent dispatches:
  - Question: LOCATIONS of tests asserting body geometry/decline reasons in the planner crate;
    scope: `modules/core-modules/traditional-support-planner/`; return: `LOCATIONS` ≤20
  - Question: FACT whether any golden hard-asserts traditional outline counts; scope:
    `crates/slicer-runtime/tests/fixtures/golden/` + module goldens; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §3 Ruling 8 range
- OrcaSlicer refs:
  - `SupportMaterial.cpp` instantiation site (~`generate_support_layers` region) - delegate
    SUMMARY: which callers pass `expansion_to_propagate` vs `expansion_to_slice`
- Verification:
  - `cargo test -p traditional-support-planner --test agg_rasterizer_tdd default_config_routes_propagation_through_rasterizer -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: routing + legacy suites green; guests fresh; knob default active end-to-end
  in the guest artifact. Also update the stale `lib.rs` comment block ("See the needs-research
  deviation on the grid pattern", propagation-loop comment ~line 350) to cite this packet's
  rasterizer path — the needs-research framing is retired by Ruling 7.

### Step 7: Measurement gate tests (wall-leak + column-continuity + divergence)

- Task IDs: `TASK-425`
- Objective: implement the three integration proofs against the Step-1 baseline:
  `agg_wall_leakage_measurement_beats_baseline` (zero penetration events AND strictly smaller
  penetrated area), `agg_column_continuity_measurement_beats_baseline` (strictly fewer abrupt
  drops AND ±25% total-area guard), `agg_and_legacy_modes_both_function_and_diverge` (different
  outline sets on ≥1 layer, both non-empty and reaching the plate).
- Precondition: Step 6 green; baseline JSON committed.
- Postcondition: AC-6, AC-7, AC-8 hold with recorded numbers; the wedge invariant
  `support_segments_stay_outside_the_model_and_within_the_build_volume` stays green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - lines 159–200
    (run_slice driver), 845–885 (block-count pattern)
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - full (~160 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`
  - `crates/slicer-runtime/tests/fixtures/golden/p241_baseline.json` (only if the metric
    helper signatures changed in Step 6 — re-record with justification comment)
  - `crates/slicer-runtime/tests/integration/main.rs` (only if a new #[test] wrapper is needed)
- Files explicitly out of bounds:
  - `modules/**`, other integration files, goldens other than p241_baseline.json
- Expected sub-agent dispatches:
  - none; drivers mirror Step-1 patterns
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E1/E2 ranges
- OrcaSlicer refs:
  - upstream `fb7b995050` / `a95607d7bf` symptoms - cited from plan §3; no read needed
- Verification:
  - `( cargo test -p slicer-runtime --test integration -- agg_wall_leakage_measurement_beats_baseline --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- agg_column_continuity_measurement_beats_baseline --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- agg_and_legacy_modes_both_function_and_diverge --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && echo PASS`
  - `cargo test -p slicer-runtime --test integration -- support_segments_stay_outside_the_model_and_within_the_build_volume --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- Exit condition: three proofs green with numbers recorded in the log + summarized into
  `requirements.md` §Acceptance Summary appendix comment; wedge invariant green.

### Step 8: Real-mesh validation + performance honesty (T7)

- Task IDs: `TASK-426`
- Objective: run the agg path on `resources/regression_wedge.stl` through the full pipeline
  (wedge harness with overrides selecting `support_area_rasterizer` implicitly default) and
  assert a non-empty plan with body regions reaching beneath the overhang; measure per-layer
  planner time delta vs the legacy path on the same fixture and update the manifest
  `estimated-ms-per-layer` hint ONLY with the measured number.
- Precondition: Step 7 green.
- Postcondition: wedge plan non-empty; measured timing recorded in the log; manifest hint
  updated with the measured value or left with a no-drift note.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`
    (wedge-backed test via the sdk test feature, mirroring existing crate tests)
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
    (hint only, measured)
  - `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`
    (wedge integration proof if better hosted there)
- Files explicitly out of bounds:
  - `src/lib.rs` behavior (no logic change), other modules
- Expected sub-agent dispatches:
  - Question: does any existing crate test already slice the wedge via the sdk test feature;
    scope: `modules/core-modules/*/tests/`; return: `LOCATIONS` ≤20
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §13 T7 range
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: wedge proof green; timing note recorded; guests fresh.

### Step 9: Closure gates + registration

- Task IDs: `TASK-427`, `TASK-428`
- Objective: run the full gate set; register TASK-419..TASK-428 in `docs/07_implementation_status.md`
  via a bounded dispatch; verify doc-impact greps; prepare the Human Validation Gate artifact
  commands (execution is the human's).
- Precondition: Steps 1–8 green.
- Postcondition: gates green; TASK rows registered; doc greps pass; packet ready for human gate
  sign-off (status stays `draft` until §8 sign-off).
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - tail only via delegated FACT (never full-read)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (TASK rows)
  - `docs/15_config_keys_reference.md` (final regen if Step 5's regen drifted)
  - `docs/spec_packets/241-support-agg-rasterizer/requirements.md` (recorded metrics appendix)
- Files explicitly out of bounds:
  - plan queue table, `docs/DEVIATION_LOG.md`, other packets
- Expected sub-agent dispatches:
  - Question: FACT next free TASK id range + exact insertion point; scope:
    `docs/07_implementation_status.md`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §8 human gate range
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo xtask check-literals`
  - `rg -q 'TASK-419' docs/07_implementation_status.md && rg -q 'support_area_rasterizer' docs/15_config_keys_reference.md && echo REG-OK`
- Exit condition: all gates green; registration greps pass; human-gate artifact commands listed
  in `packet.spec.md` §Human Validation Gate execute cleanly (dry-run the two fixture slices).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | baseline capture, new files only |
| Step 2 | S | delegated probe, note-only edit |
| Step 3 | M | grid math + red tests |
| Step 4 | M | extraction + red tests |
| Step 5 | S | manifest + parse + rejection |
| Step 6 | M | loop rewiring + legacy guard |
| Step 7 | M | three measurement proofs |
| Step 8 | S | wedge + timing honesty |
| Step 9 | S | gates + registration |

Aggregate: `M`. No step rated L. Split Step 3/4 only if the fidelity probe reveals canonical
complexity beyond the recorded SNIPPETS.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions: none for this packet (stub consumed at
  authoring; register row rerouted at authoring).
- `packet.spec.md` is ready for `status: implemented` ONLY after the Human Validation Gate
  sign-off line is recorded.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (rotation branch unexercised until a support-angle key
  exists; recorded in `design.md` §Data and Contract Notes).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm
  ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification
commands must use `--all-targets` so the test, bench, and example targets compile.
