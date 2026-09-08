# Implementation Plan: 241-support-agg-rasterizer

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (TASK-419..TASK-428, consecutive, no gaps). Steps 10-16 were added after the reserved range
  was fully consumed by Steps 1-9; each re-uses the TASK ID of the step it continues rather
  than allocating outside the reserved range (allocating outside it requires re-mapping the
  packet frontmatter first).
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
  `crates/slicer-runtime/tests/fixtures/support-family/SupportAdversarial.stl` (generated
  in-test by `adversarial_mesh()` — three `roof_edge_slot` blocks; `stepped_pocket_mesh` is
  retained only as the rejected 3/3-drops counterexample — via the ignored recorder
  `p241_generate_adversarial_fixture`;
  the tracked 30x20 box measures 0/0 drops and cannot gate AC-7): penetration events + total penetrated area (vs occupancy grown by
  `support_object_xy_distance`) and abrupt-column-drop count + total support area, sliced via
  `run_slice` with the matched normal profile.
- Precondition: 238c implemented; `cargo xtask build-guests --check` exit 0; working tree
  clean at the pre-port commit.
- Postcondition: new test SUBMODULE (not a new binary)
  `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs` exists with
  `capture_pre_port_baseline` (writes `target/p241-baseline.json` via a `#[ignore]`d recording
  entry, mirroring the repo's `record_*` discipline), the metric helper functions, and the
  non-ignored guard test `p241_metric_helpers_agree_on_baseline_fixture` this step's
  verification command runs. The directory
  `crates/slicer-runtime/tests/fixtures/golden/` DOES NOT EXIST yet and must be created by
  this step; the baseline JSON is committed under it as `p241_baseline.json`. `main.rs`
  carries the `mod support_agg_rasterizer_tdd;` line — without it the file never compiles and
  the verification command reports "0 tests run" as a false pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` (long; ranged reads
    only) - read by symbol: `support_test_path`, `matched_config_base`, `run_slice_for_family`
    and `run_slice_for_family_with_interface_layers`. Note there is no
    local `run_slice`; the file calls `slicer_runtime::run::run_slice(opts)`.
  - `crates/slicer-runtime/tests/integration/main.rs` (212 lines) - lines 1–40 for the
    mod-registration shape; note both conventions exist there (plain fns wrapped in `#[test]`
    shims in `main.rs`, and `#[test]` fns inline in the submodule). Either is acceptable.
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
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::p241_metric_helpers_agree_on_baseline_fixture --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
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
- Postcondition: a `## Plan Corrections` section exists in this packet's `design.md`
  containing at least one line that begins with the literal token `p241 fidelity probe:`.
  The note is written UNCONDITIONALLY (append-only): if a constant differs from the plan's
  pre-verified evidence, record the difference; if none differs, record an explicit
  `p241 fidelity probe: no-diff` line naming the constants checked. No code change.
  The token is what the verification command greps for — a pre-existing heading must never
  be able to satisfy this step's gate (E1: no vacuous assertions).
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
  - `rg -q '^p241 fidelity probe:' docs/spec_packets/241-support-agg-rasterizer/design.md && echo PROBE-RECORDED`
    (verified 2026-09-03 to return NOTHING against the pre-work tree — the token does not yet
    exist, so this gate is falsifiable. Do NOT relax it to grep a section heading: `## Open
    Questions` already exists in `design.md` and would make the gate pass before the probe runs.)
- Exit condition: fidelity note recorded (either a differing-constant note or an explicit
  `p241 fidelity probe: no-diff` line); dispatch returns ≤ caps.

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
  - `modules/core-modules/traditional-support-planner/src/lib.rs` (long; ranged reads only) -
    read the imports/struct header and `from_config` for module conventions only
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/src/agg_raster.rs` (new)
  - `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs` (new)
  - `modules/core-modules/traditional-support-planner/Cargo.toml` ([[test]] target)
- Files explicitly out of bounds:
  - `modules/core-modules/traditional-support-planner/src/lib.rs` (the knob parse is Step 5;
    the propagation rewiring is Step 6), `crates/**`, other modules
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
  `values = ["agg", "legacy_semantic"]`, `default = "agg"` — **SUPERSEDED by Step 12: the
  default is now `"legacy_semantic"` per the binding human decision of 2026-09-03; do not
  re-apply the `"agg"` default from this line**) in the manifest; parse it in
  `from_config` into a `RasterizerMode` field; reject out-of-vocabulary strings with a fatal
  `ModuleError` naming key + allowed values; regenerate
  `docs/15_config_keys_reference.md`.
- Precondition: Steps 3–4 green.
- Postcondition: AC-1 and AC-N1 hold; unset key resolves to `Agg`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml` -
    `[config.schema.retract_mode]` (the enum declaration pattern: `type = "enum"`,
    `values = [...]`, `default`, `display`, `group`)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
  - `modules/core-modules/traditional-support-planner/src/lib.rs` (parse block ONLY — no
    propagation rewiring this step)
  - `docs/15_config_keys_reference.md` (regen)
- Files explicitly out of bounds:
  - propagation loop body, other modules, `crates/slicer-scheduler/**`. Do NOT extend the
    scheduler — but do not repeat the false rationale either: `ConfigBoundsIndex` is NOT
    numeric-only. `ConfigBoundsIndex::from_modules` harvests `values` from every loaded
    module's `type = "enum"` entries into `enum_values`, and `resolve_global_config` calls
    `bounds.check(..)?`, so declaring the knob in the manifest ALREADY makes a bad value abort
    the slice host-side with `config resolution failed: …`. The module-side rejection this
    step adds is defense-in-depth, matching `SeamPlacer::from_config`
    (`modules/core-modules/seam-placer/src/lib.rs`), which rejects an unknown `seam_mode` with
    `ModuleError::fatal` despite `seam_mode` being a manifest-declared enum. Because the host
    fires first, AC-N1's test MUST drive `from_config` on a directly-constructed `ConfigView`,
    not a full slice.
- Blast-radius discipline: `rg -n 'support_area_rasterizer' modules/ crates/` before editing —
  this MUST return zero hits (the code-side key is genuinely net-new). Do NOT include `docs/`
  in that precheck: as of 2026-09-03 the key is already named in 3 pre-existing doc locations
  (`docs/specs/support-families-anchored-entities-plan.md` ×2 and
  `docs/spec_packets/236-support-stabilization/requirements.md` ×1, plus this packet's own
  files), and a worker seeing those hits could wrongly conclude the key already exists.
  Then `rg -n 'from_config' modules/core-modules/traditional-support-planner/tests/` to catch
  config-shape assertions; fix any fallout in-step.
- Expected sub-agent dispatches:
  - Question: current `[config.schema]` tail of the manifest + any test asserting manifest key
    counts; scope: `modules/core-modules/traditional-support-planner/`; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - §Config Field Types Reference (enum row) ranged read
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q '^\[config\.schema\.support_area_rasterizer\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q '"agg", "legacy_semantic"' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'support_area_rasterizer' docs/15_config_keys_reference.md && echo PASS`
  - `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd invalid_rasterizer_value_is_rejected_not_defaulted -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: declaration + parse + rejection green; doc regen committed; guests fresh.

### Step 6: Propagation rewiring — agg selectable, legacy selectable

*(Historical heading was "agg default, legacy selectable"; the default was flipped to
`legacy_semantic` in Step 12. The rewiring itself is unchanged — only which mode an unset key
selects.)*

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
  - `modules/core-modules/traditional-support-planner/src/lib.rs` (long; ranged reads only) -
    the `SupportPlanner::plan_candidate` propagation loop
    `for layer in (termination_layer..trim_end).rev()` ENDS at
    `propagated_by_layer.insert(layer, carry.clone())`; the `code: 1203` diagnostic is INSIDE
    it. Do not read a range that stops before the loop's closing brace — that hides exactly
    the termination bookkeeping AC-5 must preserve. Also read `from_config` for the mode
    field. The separate emit loop in `plan_candidate` is not rewired here.
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` (2466
    lines, 28 `#[test]` fns) - helpers at the top, then targeted failing tests on demand
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/src/lib.rs`
  - `modules/core-modules/traditional-support-planner/src/agg_raster.rs` (integration glue)
  - `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`
    (routing test)
- Files explicitly out of bounds:
  - `tests/traditional_family_tdd.rs` (any assertion-value fallout from the agg default is
    Step 6b's edit, not this step's — keeping this step inside the 3-file cap), other modules,
    `crates/**`
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
  - `cargo test -p traditional-support-planner --test agg_rasterizer_tdd default_config_routes_propagation_through_legacy_semantic -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: routing + legacy suites green; guests fresh; knob default active end-to-end
  in the guest artifact. Also update the stale `lib.rs` comment line "See the needs-research
  deviation on the grid pattern" (inside the explanatory block above
  `let mut propagated_by_layer`) to cite this packet's rasterizer path — the needs-research
  framing is retired by Ruling 7. Note the referenced item is NOT in `docs/DEVIATION_LOG.md`
  (no such row exists); it is gap-register row G-07, which already routes to this packet.

### Step 6b: Legacy-guard assertion reconciliation

- Task IDs: `TASK-424` (continues Step 6; separate sub-step so each stays inside the
  3-file edit cap)
- Objective: re-run the existing `traditional_family_tdd` suite under the NEW agg default and
  reconcile any geometric assertion whose expected value legitimately tightens, citing the
  measured value in a comment on each change. Never weaken an assertion to make it pass; a
  loosened bound is a defect, a re-baselined exact value with a measured citation is not.
- Precondition: Step 6 green (routing test passes; guests fresh).
- Postcondition: AC-N2 holds under BOTH explicit `legacy_semantic` selection and the agg
  default; every changed assertion carries a `// measured: <value> (p241 Step 6b)` comment.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` (2466
    lines) - only the tests reported failing by the Step-6 run; read from `target/test-output.log`
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`
- Files explicitly out of bounds:
  - `src/lib.rs`, `src/agg_raster.rs` (no behavior change may be made to fix a test here),
    other modules, `crates/**`
- Expected sub-agent dispatches:
  - none; the failing set comes from Step 6's logged run
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §3 Ruling 8 range
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: full legacy suite green under the mode that was default when this step ran
  (`agg`, before Step 12 flipped the default to `legacy_semantic`); every re-baselined
  assertion carries its measured citation; guests fresh. NOTE: this exit was signed off with
  the asymmetric clamp in place; Step 10 later measured four of these tests RED with the clamp
  removed, so this green does not carry over to the current tree.

### Step 6c: Faithful-port measurement (recorded)

- Task IDs: `TASK-424` (continues Step 6)
- What was done: the faithful port was measured end-to-end; canonical macro-block snapping
  added a halo around the carry (one macro block = 25002 units = 2.5002 mm at the matched
  profile — `pixel_size` 4167 x `oversampling` 6),
  which printed phantom support where nothing was demanded and kept the carry non-empty when
  occupancy closed every route, so the `NoRoute` (`code: 1203`) decline was lost and the
  legacy suite went red (commit `d741ae33`). Clamping the printed area to the bare carry was
  also measured: agg and legacy then emitted byte-identical geometry on every layer.

### Step 6d: Asymmetric clamp (recorded)

- Task IDs: `TASK-424` (continues Step 6)
- What was done: `RasterizerMode::Agg` arm of `SupportPlanner::plan_candidate` clamps the
  propagated carry to `pre_grid_carry` and the printed area to
  `offset(pre_grid_carry, offset_to_slice, Miter)` (empty-offset fallback); both suites green
  (commit `2247c842`). Filed as DEV-166 in `docs/DEVIATION_LOG.md`; see design.md §Data and
  Contract Notes "Asymmetric clamp".

#### Disclosures (Step 6 series)

- (a) TDD ordering violated in Step 6: the rewiring was implemented first and the RED baseline
  was reproduced retroactively by stashing `src/` (commit `d741ae33`).
- (b) The `support_area_rasterizer` field on the planner config struct was made `pub` (commit
  `83f9ff36`) so the guest test can construct/inspect it.
- (c) `ISLAND_SAMPLE_INSET_MM = -0.0001` (`modules/core-modules/traditional-support-planner/src/lib.rs`):
  canonical's −20 nm inset rounds to −1 unit at PnP scale; the pre-inset fallback is PnP-only.
- (d) `contours_simplified` (`modules/core-modules/traditional-support-planner/src/agg_raster.rs`)
  returns `Vec<ExPolygon>` with locally computed nesting, not canonical's flat `Polygons`.

### Step 7: Measurement gate tests (wall-leak + column-continuity + divergence)

- Task IDs: `TASK-425`
- Objective: implement the three integration proofs against the Step-1 baseline:
  `agg_wall_leakage_measurement_beats_baseline` (non-regression guard: zero penetration events
  above the documented sliver noise floor — `WALL_LEAKAGE_NOISE_FLOOR_UNITS2` = 1e-4 mm² per
  intersection piece — AND penetrated area not greater than
  baseline; Round-join grow), `agg_column_continuity_measurement_beats_baseline` (as authored in this step: strictly fewer
  abrupt drops AND a ±25% total-area drift guard — that guard was RETIRED in Step 15b and
  replaced by the mechanism-derived macro-block containment bound; see `packet.spec.md` AC-7), `agg_and_legacy_modes_both_function_and_diverge` (different
  outline sets on ≥1 layer, both non-empty and reaching the plate).
- Precondition: Step 6 green; baseline JSON committed.
- Postcondition: AC-6, AC-7, AC-8 hold with recorded numbers. Wedge-level functioning proof
  is the Step-8 test `agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang` (not a
  collision test); the tree-family invariant
  `support_segments_stay_outside_the_model_and_within_the_build_volume` never touches the
  knob and is not an exit condition here.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` (long; ranged reads only) - by
    symbol: `run_slice_for_family_with_interface_layers` for the slice driver and
    `interface_block_count` for the block-count pattern. NOTE: the earlier draft
    cited a line range for block counts that was actually cross-family overlap rejection
    (`try_aggregate_support_plan_irs_with_diagnostics`), not block counting.
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - full (173 lines);
    `prepare_wedge_context_with_overrides` is the override-taking entry
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`
  - `crates/slicer-runtime/tests/fixtures/golden/p241_baseline.json` (only if the metric
    helper signatures changed in Step 6 — re-record with justification comment)
  - `docs/spec_packets/241-support-agg-rasterizer/requirements.md` (recorded-metrics appendix
    — this step's exit condition writes the measured numbers, so the file must be editable
    here, not only in Step 9)
- Files explicitly out of bounds:
  - `modules/**`, other integration files, goldens other than p241_baseline.json,
    `crates/slicer-runtime/tests/integration/main.rs` (its `mod` line landed in Step 1; if a
    new `#[test]` wrapper is genuinely needed, declare the tests inline in the submodule
    instead — both conventions exist in that binary — so this step stays inside the 3-file cap)
- Expected sub-agent dispatches:
  - none; drivers mirror Step-1 patterns
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E1/E2 ranges
- OrcaSlicer refs:
  - upstream `fb7b995050` / `a95607d7bf` symptoms - cited from plan §3; no read needed.
    Legacy already reproduces `fb7b995050`; only `a95607d7bf` is the improvement under test.
- Verification:
  - `( cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_wall_leakage_measurement_beats_baseline --exact --nocapture 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_column_continuity_measurement_beats_baseline --exact --nocapture 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && ( cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_and_legacy_modes_both_function_and_diverge --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 ) && echo PASS`
- Exit condition: three proofs green with numbers recorded in the log + summarized into
  `requirements.md` §Acceptance Summary appendix.

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
- Hosting outcome (recorded): the module-crate host
  (`modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`) was
  BLOCKED by a dev-dependency cycle (`slicer-runtime` -> `slicer-integrated-modules` ->
  `traditional-support-planner`); the wedge harness lives in `slicer-runtime`, so the proof
  is hosted in the integration binary as
  `agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`
  (`crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`). The manifest
  file was not edited.
- Measured timing (recorded): release `pnp_cli`, `resources/regression_wedge.stl`, 5 runs
  each, planner-module elapsed / 200 layers: agg 0.155 ms/layer (median 31 ms),
  legacy_semantic 0.045 ms/layer (median 9 ms). Manifest `estimated-ms-per-layer` hint of 5
  left unchanged: the measured agg figure (0.155 ms/layer) is well under the hint, so no drift
  to correct.
- Measured wedge facts (recorded, printed by the test): 200 printable layers, plate layer 0,
  body layers 0..141 (142 layers, 219 polygons) in both modes, top-interface layers 77..143,
  plate-layer body area 172.004 mm^2 (agg) vs 154.300 mm^2 (legacy_semantic).
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: wedge proof green; timing note recorded; guests fresh.

### Step 9: Closure gates + registration

- Task IDs: `TASK-427`, `TASK-428`
- Objective: run the full gate set; register TASK-419..TASK-428 in `docs/07_implementation_status.md`
  via a bounded dispatch; verify doc-impact greps; prepare the Human Validation Gate artifact
  commands (execution is the human's).
- Precondition: Steps 1–8 green.
- Postcondition (as recorded AT THE TIME, under the clamp with `agg` as the default): gates
  green; TASK rows registered; doc greps pass. The "packet ready for human gate sign-off"
  reading of this postcondition is SUPERSEDED — Step 16 re-ran the gates, and the packet is
  still not gate-ready because Step 17 is open and the dry-run artifacts are clamp-era.
  Status stays `draft` until §8 sign-off.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - tail only via delegated FACT (never full-read)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (TASK rows)
  - `docs/15_config_keys_reference.md` (final regen if Step 5's regen drifted)
  - `docs/spec_packets/241-support-agg-rasterizer/requirements.md` (recorded metrics appendix)
- Files explicitly out of bounds:
  - plan queue table, `docs/DEVIATION_LOG.md`, other packets
- Expected sub-agent dispatches:
  - Question: FACT — is the RESERVED range TASK-419..TASK-428 still unused in
    `docs/07_implementation_status.md` (`rg -o 'TASK-4(1[9]|2[0-8])'` returns nothing), and
    what is the exact insertion point? Do NOT ask for the "next free" ID: that file already
    runs past TASK-530, and this packet's IDs are reserved below the high-water mark by queue
    row #8 of `docs/specs/support-families-anchored-entities-plan.md`. If the reserved range
    is no longer free, stop and re-map in the packet frontmatter before registering. Scope:
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

### Step 10: Root-cause probe — clamp removed, halo proven canonical (recorded)

- Task IDs: `TASK-424` (continues the Step-6 series). No new TASK ID is allocated: the reserved
  range `TASK-419`..`TASK-428` is fully consumed by Steps 1–9, and allocating outside it would
  require re-mapping the packet frontmatter first.
- What was done: the asymmetric clamp was REMOVED from the `RasterizerMode::Agg` arm of
  `SupportPlanner::plan_candidate`
  (`modules/core-modules/traditional-support-planner/src/lib.rs`) and the resulting unclamped
  behaviour was root-caused. Three hypotheses that the macro-block halo was a PnP porting defect
  were tested and REFUTED:
  - H1 "extraction is block-granular" — REFUTED. Extraction is pixel-granular; canonical
    `contours_simplified` (`SupportMaterial.cpp`) never receives `oversampling`.
  - H2 "`dilate_trimming_region` wrongly dilates" — REFUTED. It is a correct erosion:
    measured 144 → 100 set cells on the probe input, where a dilation would have given 196.
  - H3 "`seed_fill_block` is mis-ported" — REFUTED. The port is two-pass, block-local, and
    gated on the dilated mask at BOTH endpoints, matching canonical.
  Conclusion: the halo is canonical, produced by block-local flooding in `seed_fill_block`
  (`SupportMaterial.cpp`); the carry grows by at most one macro-block extent (one macro block =
  25002 units = 2.5002 mm at the matched profile — `pixel_size` 4167 x `oversampling` 6,
  measured 2026-09-03).
  With the clamp removed and `agg` still the default, FOUR tests in the legacy guard suite
  `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` measured
  RED (24 passed / 4 failed), each failing on a PnP invariant that canonical has no equivalent
  of:
  - `candidate_with_no_downward_route_is_declined` — "a candidate with no downward route must
    record a structured decline"; the block-snapped carry routes around the obstacle.
  - `invalid_body_rejected` — "planner diagnostics: []"; the `code: 1203` complete-body
    diagnostic never fires.
  - `base_interface_obstacle` — "obstacle candidate must be structurally declined".
  - `base_column_keeps_clear_of_foreign_territory` — "layer 7 role TopInterface point
    Point2 { x: 27005, y: -4000 } crosses the boundary"; measured x = 27005 against a 20000 bar.
  Mechanism, measured on `candidate_with_no_downward_route_is_declined` at the module-test
  defaults (spacing 2.5 mm, width 0.4 mm, `pixel_size` 4167, `oversampling` 6, one macro block =
  25002 units): at layer 7 the fixture has zero occupancy, so the raw raster's 121 set cells
  become 324 after seed fill — the entire 18x18 interior — and the carry bbox grows from
  (0,0)-(40000,40000) to (-24999,-24999)-(50005,50005), i.e. 4.0 mm to 7.5004 mm. It is then a
  fixed point: layers 6..0 rebuild an identical grid, so the growth is one macro block once, not
  per-layer creep.
- Verification (as recorded by the probe run): the `traditional_family_tdd` suite under the agg
  default, output tee'd to `target/test-output.log` (that log is overwritten by every later run
  — treat the count, not the file, as the durable fact).
- Exit condition: MET — the halo is attributed to canonical behaviour rather than to the port,
  and the clamp is gone from the tree.

### Step 11: Canonical probe — what canonical actually does (recorded)

- Task IDs: `TASK-420` (extends the Step-2 canonical fidelity probe; delegated OrcaSlicer read,
  cited by symbol name only).
- What was done: four canonical findings were established and are recorded here so no future
  reader re-investigates them:
  1. The macro-block halo is DELIBERATE. Canonical stretches supports into the grid so the
     zig-zag support snake can run along grid lines; `seed_fill_block` (`SupportMaterial.cpp`)
     floods each `oversampling × oversampling` block independently to achieve it.
  2. Canonical PRINTS that material where no overhang demanded it. It has no per-region demand
     model and no foreign-territory bar, so the halo costs it nothing.
  3. Canonical has NO decline concept. When trimming closes every route,
     `diff(carry, trimming)` goes empty BEFORE rasterization and the caller simply skips the
     lower layers. PnP's structured `SupportPlanDeclineReason::NoRoute` / diagnostic
     `code: 1203` decline is a PnP-only invariant, and block-snapping cannot preserve it — the
     inflated carry routes around the obstacle.
  4. The port is faithful where it was doubted: extraction is pixel-granular
     (`contours_simplified` takes no `oversampling`), `dilate_trimming_region` is an erosion
     (measured 144 → 100 set cells; a dilation would give 196), and `seed_fill_block` is
     two-pass, block-local, and mask-gated at both endpoints.
- Verification: delegated canonical read (LOCATIONS/SUMMARY contract, `§OrcaSlicer Reference
  Obligations`) plus the H2 cell-count measurement from Step 10. No in-tree test gates this
  step; it is evidence, not behaviour.
- Exit condition: MET — findings recorded in `design.md` §Data and Contract Notes and in
  DEV-166.

### Step 12: Default flipped to `legacy_semantic` (recorded; owned by a concurrent worker)

- Task IDs: `TASK-423` (the knob-declaration step's ID; the manifest default is that step's
  surface).
- What was done: per the binding human decision of 2026-09-03, the `support_area_rasterizer`
  default changes from `"agg"` to `"legacy_semantic"` in
  `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` and in the
  guest-side parse default; `agg` ships opt-in and UNCLAMPED. `docs/15_config_keys_reference.md`
  is regenerated with the new default by the same owner.
- **Ownership: a CONCURRENT worker in this run, NOT this documentation pass.** Verification is
  that worker's exit condition, not evidence produced here. Do not read this entry as proof the
  flip has landed — re-derive the current default from the manifest at the point of use.
- Verification (that worker's): manifest grep for the new default, the guest parse-default test,
  `cargo xtask build-guests --check` exit 0, and the AC-1 command in `packet.spec.md` (which
  greps the schema header and values list only, and is unaffected by the default value).
- Exit condition: owned by that worker.

### Step 13: agg divergence tests (recorded; owned by a concurrent worker)

- Task IDs: `TASK-424` (continues the Step-6 series).
- What was done: tests are added that assert the ACCEPTED divergences under explicit
  `support_area_rasterizer = "agg"` — the block-snapped halo is present rather than clamped
  away, and the structured `NoRoute` / `code: 1203` decline does NOT fire under `agg` when the
  blocking occupancy is LOCAL, while it does under the default `legacy_semantic`. The decline
  STILL fires under `agg` when occupancy covers the whole grid neighbourhood, because seed fill
  is then blocked everywhere and the carry genuinely empties. Both halves are pinned:
  `agg_does_not_decline_no_route_where_legacy_semantic_does` and
  `agg_path_still_declines_no_route_when_occupancy_closes_every_layer`
  (`modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`). This turns the DEV-166 divergence into a pinned,
  falsifiable property instead of prose.
- Ownership: authored by a concurrent worker in this run, not by the documentation pass. The
  two test names above were verified present on disk 2026-09-03 by `grep -n 'fn <name>'`
  against `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`.
- Verification: `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd 2>&1 | tee target/test-output.log`
  — 13 passed (recorded 2026-09-03).
- Exit condition: MET — the divergence is pinned by falsifiable tests in both directions.

### Step 14: Re-measurement after clamp removal — **DONE**

- Task IDs: `TASK-425` (re-uses the Step-7 measurement ID).
- STATUS: **DONE (2026-09-03).** AC-6, AC-7, AC-8 and the F-I1 control were re-measured against
  the UNCLAMPED, opt-in `agg` mode; guest freshness confirmed first (`cargo xtask build-guests
  --check` exit `0`). Every metric now quoted in this packet is post-clamp-removal. The full
  figures live in `docs/spec_packets/241-support-agg-rasterizer/requirements.md` §Recorded
  metrics appendix; the headline results are:
  - AC-6 wall leakage: `legacy_semantic` **0 events / 0.0 units²**, `agg` **0 events /
    0.0 units²** — no slivers observed at all in either mode (the clamp-era 88–311 units²
    sliver figures do not apply to this tree).
  - AC-7 drops: legacy **3** → agg **0**. Total emitted area legacy **225789129333 units²
    (2257.89 mm²)** vs agg **354695221947 units² (3546.95 mm²)** = **+57.09 %**, now RECORDED
    rather than gated. Containment (the replacement gate): **0.0 units² outside on 26/26
    layers, 0 difference pieces**; bisected smallest containing grow **22754 units** against a
    derived extent of **25002 units** (margin 2248 units, ≈ 9.0 % of one macro block).
  - AC-8: **26 of 26** body layers diverge; both modes reach the plate.
  - F-I1 control: `agg − control` = **+984.12 mm² (+38.40 % of control)**;
    `control − legacy` = **+304.94 mm²**; max per-layer symdiff **38.1980 mm²** on 26/26 layers.
    The test was renamed to `agg_printed_area_exceeds_global_offset_control` because the
    property inverted.
- Objective: re-measure AC-6 (wall leakage), AC-7 (column continuity), AC-8 (mode divergence)
  and the F-I1 control against the UNCLAMPED, opt-in `agg` mode, and replace the stale figures
  in `requirements.md` §Recorded metrics appendix.
- Precondition: Steps 12 and 13 landed and green; `cargo xtask build-guests --check` exit 0.
- Postcondition (MET): every number this packet presents as current traces to a logged
  post-clamp-removal run. The appendix banner was REPLACED rather than deleted — it now reads as
  a provenance note for the re-measured figures, and the clamp-era figures are retained below it
  as explicitly labelled history. AC-7's ±25 % total-area guard was RESTATED with the measured
  truth rather than weakened: it was retired (the halo adds material by design) and replaced by
  the strictly stronger macro-block containment bound, which the unclamped run passes.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` (the run's own output)
- Files allowed to edit (at most 3):
  - `docs/spec_packets/241-support-agg-rasterizer/requirements.md` (appendix + banner)
  - `docs/spec_packets/241-support-agg-rasterizer/design.md` (§Risks "No-measured-benefit"
    bullet)
  - `docs/DEVIATION_LOG.md` (DEV-166 stale-metrics sentence only)
- Files explicitly out of bounds:
  - `modules/**` and `crates/**` behaviour changes (this is a measurement step, not a fix step)
- Expected sub-agent dispatches:
  - Question: FACT pass/fail plus the recorded metric numbers for the AC-6/AC-7/AC-8 trio;
    scope: the trio command in `requirements.md` §Verification Commands; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E1/E2 ranges
- OrcaSlicer refs:
  - none
- Verification:
  - the AC-6/AC-7/AC-8 trio command in `requirements.md` §Verification Commands
  - `cargo xtask build-guests --check && echo FRESH`
- Exit condition: MET — no figure is presented as current evidence without a post-clamp-removal
  source. Figures that remain in the packet from the clamped runs are quarantined under explicit
  CLAMP-ERA / superseded labels and are never cited as current.

### Step 15: Documentation honesty pass (recorded)

- Task IDs: `TASK-428` (the packet's registration / doc-readiness ID).
- What was done: the packet documents were brought into line with the two binding human
  decisions of 2026-09-03 (clamp REJECTED; `agg` opt-in and unclamped, `legacy_semantic` the
  DEFAULT). `packet.spec.md`: Goal, AC-1 default, AC-5 (rewritten to assert EXPLICIT `agg`
  routing, with the termination-bookkeeping claim moved out of the criterion into a stated
  divergence), the human-gate mode labels, the recorded default-mode decision plus a new
  opt-in-divergence checklist item, and the DEV-166 doc-impact bullet. `design.md`: the
  "Asymmetric clamp" bullet replaced by the accepted-divergence record, the H1/H2/H3
  refutations and the separate `occupancy_at` / `model_occupancy` region-keying finding added,
  the two locked assumptions corrected, and the no-measured-benefit risk restated.
  `requirements.md`: premise correction 4 superseded, knob default and In-Scope text corrected,
  and the whole Step-7 metrics appendix banner-marked STALE (measured under the clamp) with no
  replacement numbers invented. `docs/DEVIATION_LOG.md`: the DEV-166 row rewritten in place
  (ID, date, and column structure preserved). This plan file: Steps 10–16 appended.
- Scope note: documentation only. No code, manifest, or test file was read or edited by this
  pass; `docs/15_config_keys_reference.md` is owned by a concurrent worker and was not touched.
- Verification: the packet's `status:` frontmatter still reads `draft`; every claim in the
  edited files is either sourced from the probes recorded in Steps 10–11 or explicitly marked
  pending.
- Exit condition: MET — no document in this packet states that `agg` is the default, that the
  clamp is the resolution, or that the `NoRoute` decline survives under `agg`; every clamp-era
  metric is banner-marked stale rather than deleted or replaced.

### Step 16: Closure gates re-run — **DONE**

- Task IDs: `TASK-427` (re-uses the Step-9 closure ID).
- STATUS: **DONE (2026-09-03).** Re-run on the post-clamp-removal, post-default-flip tree.
  Recorded results — every one exited `0`:
  - `cargo check --workspace --all-targets` — exit 0
  - `cargo clippy --workspace --all-targets -- -D warnings` — exit 0
  - `cargo xtask check-literals` — exit 0
  - `cargo xtask build-guests --check` — exit 0 (`EXIT_FRESH`)
  Step 9's gate results predate the clamp removal and the default flip and do NOT carry over;
  they are superseded by the results above.
- Objective: after Steps 12–14 land, re-run the full gate set and re-verify the doc-impact
  greps against the new default.
- Precondition: Steps 12, 13 and 14 complete.
- Postcondition: gates green on the current tree; `docs/15_config_keys_reference.md` shows the
  `legacy_semantic` default. NOTE: green gates do NOT make the Human Validation Gate signable.
  Two prerequisites remain open — the open Step-17 failure below, and the regeneration of the
  clamp-era human-gate dry-run artifacts (`packet.spec.md` §Human Validation Gate).
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log`
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (only if the TASK rows need correcting)
  - `docs/spec_packets/241-support-agg-rasterizer/packet.spec.md` (gate sign-off line)
- Files explicitly out of bounds:
  - plan queue table, other packets' directories
- Expected sub-agent dispatches:
  - Question: FACT pass/fail for the closure gate set; scope: the four gate commands below;
    return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §8 human gate range
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo xtask check-literals`
  - `cargo xtask build-guests --check && echo FRESH`
  - every pipe-suffixed AC command in `packet.spec.md`
- Exit condition: MET for the gate set itself (all four commands exit 0 on the post-flip tree).
  The packet may not flip to `status: implemented` without the Human Validation Gate sign-off
  line, which is blocked on Step 17 and on artifact regeneration.

### Step 17: Duplicate support-region entries under `agg` on the wedge — **UNBLOCKED TEMPORARILY; ROOT CAUSE NOT FIXED**

- Task IDs: `TASK-426` (re-uses the Step-8 real-mesh validation ID).
- STATUS: **UNBLOCKED, NOT RESOLVED (2026-09-03).** The wedge slice no longer aborts, because
  `merge_region_identity_entries`
  (`modules/core-modules/traditional-support-planner/src/lib.rs`) unions entries sharing a
  `(global_layer_index, object_id, region_id, anchor_z)` identity before publish. That merge is a
  **documented temporary unblock**, filed as **DEV-167** in `docs/DEVIATION_LOG.md`, not the fix:
  the producer still mints one `SupportPlanEntry` per candidate per layer. Two
  `traditional_family_tdd` tests fail as a direct consequence, so **AC-N2 is RED** and the packet
  is NOT ready for the Human Validation Gate and MUST NOT be described as such. See Step 20.
- Symptom: `agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`
  (`crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`) FAILS under
  `support_area_rasterizer = "agg"` with
  `SupportPlanIR contains duplicate entries for support region (layer=0, object=…, region=0)`.
- Consequence for this packet: AC-8's "both modes reach the plate" and the Step-8 wedge proof are
  recorded GREEN from the Step-14 measurement run, and the wedge test no longer aborts under the
  temporary merge — but the root cause is unfixed and AC-N2 is red because of it. No claim of
  packet completion, and no `status: implemented` flip, may be made. Ownership of the real fix
  transfers to packet `241b-support-plan-ownership-seam`.
- Verification (to be run by the owning worker, not carried over from Step 14):
  `cargo test -p slicer-runtime --test integration -- support_agg_rasterizer_tdd::agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang --exact --nocapture 2>&1 | tee target/test-output.log`
- Exit condition: **NOT MET.** The wedge test passes under both modes only because of a
  temporary merge; the exit as written ("the test passes under both modes, or the packet records
  a restated criterion backed by a measured run") is satisfied in letter but not in substance,
  and AC-N2 — which the same defect breaks — is red. Recorded as NOT MET, handed to `241b`.

### Step 18: Documentation reconciliation after the clamp rejection (recorded)

- Task IDs: `TASK-428` (the packet's registration / doc-readiness ID; continues Step 15).
- What was done: documentation-only reconciliation of eight defects raised by an adversarial
  review — Step 14/16 status contradictions resolved across this file, `task-map.md` and the
  DEV-166 row; the stale `24996` macro-block extent (and stale `pixel_size 4166`) corrected to
  the measured `25002` / `4167` everywhere in the packet docs; the retired ±25 % total-area guard
  reconciled to "retired, replaced by the containment bound" in `design.md`,
  `requirements.md` and this file; the remaining unqualified "`NoRoute` does NOT fire under
  `agg`" claim qualified; clamp-era figures in `packet.spec.md` and `requirements.md` labelled or
  corrected; the human-gate dry-run artifacts labelled clamp-era with regeneration made an
  explicit gate prerequisite; bare-basename and line-pinned citations made crate-qualified /
  symbol-named; and `agg_printed_area_exceeds_global_offset_control` added to the
  §Verification Commands matrix.
- Scope note: documentation only. No code, manifest, or test file was edited. Test names and file
  paths quoted here were verified present on disk in this session.
- Exit condition: MET — no document in this packet presents a clamp-era figure as current, states
  the ±25 % guard as live, or claims human-gate readiness while Step 17 is open.

### Step 19: Fix the producer defect and restore AC-N2 to green — **NOT DONE (DECLINED)**

- Task IDs: none allocated — no work was performed under this step.
- STATUS: **NOT DONE. Deliberately DECLINED by binding human decision on 2026-09-03. It was not
  skipped, deferred by omission, or forgotten.** This step is recorded here for the first time at
  close-out precisely so the decline is visible rather than inferred from a gap in the numbering.
- What it would have been: fix the traditional planner's per-candidate publishing so that a layer
  yields one `SupportPlanEntry` per `(global_layer_index, object_id, region_id)` triple as
  `docs/02_ir_schemas.md` § "IR 9b — SupportPlanIR" requires, remove the temporary
  `merge_region_identity_entries` unblock, and reconcile
  `coarse_same_region_sources_keep_distinct_body_membership` and
  `coarse_source_preference_keeps_mixed_source_memberships`
  (`modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`) with the
  repaired contract — turning AC-N2 green.
- Why it was declined: the real repair is an ownership seam between candidate identity and region
  identity, which is a design change to the planner's publishing contract rather than a fix
  inside this packet's rasterizer scope. Packet 241 therefore closes **NARROW and NOT GREEN**,
  and this work transfers whole to packet `241b-support-plan-ownership-seam`.
- Explicitly NOT done, by the same decision: no test in `traditional_family_tdd.rs` was rewritten,
  no assertion was softened, and the temporary merge was not removed.
- Exit condition: NOT MET, and intentionally so.

### Step 20: Close-out documentation (recorded)

- Task IDs: `TASK-428` (the packet's registration / doc-readiness ID; continues Step 18).
- What was done: recorded the packet's narrow, not-green close-out across
  `packet.spec.md`, `requirements.md`, `design.md`, `task-map.md`, this file, and
  `docs/DEVIATION_LOG.md`. Specifically:
  - **The wedge failure (Step 17).**
    `agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang`
    (`crates/slicer-runtime/tests/integration/support_agg_rasterizer_tdd.rs`) aborted the `agg`
    slice with a duplicate-support-region rejection at commit.
  - **Verified root cause.** The traditional planner publishes one `SupportPlanEntry` per
    CANDIDATE per layer, so several entries can share one
    `(global_layer_index, object_id, region_id)` identity — a shape
    `docs/02_ir_schemas.md` § "IR 9b — SupportPlanIR" forbids and
    `SupportPlanIR::duplicate_region_identity` (`crates/slicer-ir/src/slice_ir.rs`) rejects at
    `Blackboard::commit_support_plan` (`crates/slicer-runtime/src/blackboard.rs`). It is
    long-standing and NOT introduced by packet 241; it was masked because host
    `union_same_family_entries` (`crates/slicer-wasm-host/src/support_aggregation.rs`) merges on
    `family_id` + layer + `object_id` + `anchor_z` plus (same body OR equal centroid
    `routing_cell`) — a key with no `region_id` in it — so duplicates historically merged by
    centroid coincidence. Unclamped `agg` produced a third column whose centroid sits at
    y = −1.525 mm; `div_euclid` floored it into the neighbouring `routing_cell`, so it never
    merged and commit rejected the plan.
  - **Temporary module-side merge.** `merge_region_identity_entries`
    (`modules/core-modules/traditional-support-planner/src/lib.rs`) unions on
    `(global_layer_index, object_id, region_id, anchor_z)` before publish, preserving geometry
    (role regions concatenated per role kind, no clipping, all body ids retained). It stays as a
    **documented temporary unblock** so `agg` does not abort real-mesh slices. Filed as
    **DEV-167**.
  - **AC-N2 left RED, by decision.** The suite
    `cargo test -p traditional-support-planner --test traditional_family_tdd` measures
    **26 passed / 2 failed**; the failures are
    `coarse_same_region_sources_keep_distinct_body_membership` and
    `coarse_source_preference_keeps_mixed_source_memberships`, which assert the forbidden
    two-entries-per-triple shape and fail because the merge correctly collapses it. Per binding
    human decision the tests are **NOT rewritten in this packet**.
  - **Handoff.** Ownership of the real fix transfers to packet
    `241b-support-plan-ownership-seam` (Step 19, declined above). This packet's directory does
    not author or edit `241b`'s documents.
- Scope note: documentation only. No code, manifest, or test file was edited in this step. Every
  symbol, test name, and file path quoted here was verified present on disk in this session.
- Exit condition: MET — AC-N2 is stated RED in `packet.spec.md`, `requirements.md` and
  `design.md`; the Packet Completion Gate is stated NOT MET; `status:` remains `draft`; DEV-167
  is filed; Steps 19 and 20 are recorded.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | baseline capture, new files only |
| Step 2 | S | delegated probe, note-only edit |
| Step 3 | M | grid math + red tests |
| Step 4 | M | extraction + red tests |
| Step 5 | S | manifest + parse + rejection |
| Step 6 | M | loop rewiring (3-file cap) |
| Step 6b | S | legacy-guard assertion reconciliation |
| Step 7 | M | three measurement proofs |
| Step 8 | S | wedge + timing honesty |
| Step 9 | S | gates + registration (predates the clamp removal; superseded by Step 16) |
| Step 10 | S | root-cause probe (recorded) |
| Step 11 | S | canonical probe (recorded) |
| Step 12 | S | default flip to `legacy_semantic` (concurrent worker) |
| Step 13 | S | agg divergence tests (concurrent worker) |
| Step 14 | M | re-measurement after clamp removal — DONE |
| Step 15 | S | documentation honesty pass (recorded) |
| Step 16 | S | closure gates re-run — DONE (all four gates exit 0) |
| Step 17 | S | wedge duplicate-region failure — UNBLOCKED by a temporary merge (DEV-167); root cause unfixed |
| Step 18 | S | documentation reconciliation after the clamp rejection (recorded) |
| Step 19 | — | producer fix + AC-N2 restoration — NOT DONE, declined by human decision; transferred to `241b` |
| Step 20 | S | close-out documentation (recorded): AC-N2 red, gate not met, DEV-167 filed |

Aggregate: `M`. No step rated L. Split Step 3/4 only if the fidelity probe reveals canonical
complexity beyond the recorded SNIPPETS.

## Packet Completion Gate

- **NOT MET as of 2026-09-03. Packet 241 closes NARROW and NOT GREEN; `status:` stays `draft`.**
  Three independent reasons, each sufficient on its own:
  1. **AC-N2 is RED** — `cargo test -p traditional-support-planner --test traditional_family_tdd`
     measures 26 passed / 2 failed (`coarse_same_region_sources_keep_distinct_body_membership`,
     `coarse_source_preference_keeps_mixed_source_memberships`). By binding human decision the
     tests are not rewritten here; see `packet.spec.md` §Negative Test Cases and Step 20 above.
  2. **Step 17's root cause is unfixed** — it is masked by the temporary
     `merge_region_identity_entries` unblock (DEV-167), and Step 19 (the real fix) is DECLINED
     and transferred to packet `241b-support-plan-ownership-seam`.
  3. **The human-gate dry-run artifacts are clamp-era** and must be regenerated before the
     Human Validation Gate can be signed; it remains UNSIGNED.
- All steps and exits complete — **NOT SATISFIED** (Steps 17 and 19; see above).
- Every pipe-suffixed AC command returns PASS — **NOT SATISFIED** (AC-N2's command fails).
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
