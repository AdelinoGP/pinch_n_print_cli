# Implementation Plan: 190-smoothed-overhang-speed

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Do not start until packet 189's AC-3, AC-5 and AC-6 are green.** `EntityMutation::SetPointSpeedFactors`, `LayerCollectionIR.speed_profiles` and the emitter's per-point `f` resolution are hard prerequisites; without them this packet's output is discarded at the entity boundary and every test written here is untestable.

## Steps

### Step 0: Confirm the `[BLOCK]` decision and the 189 dependency, and record the baselines

- Task IDs: `TASK-309`
- Objective: prove the maintainer has recorded the `[BLOCK-1]`/`[BLOCK-1b]`/`[BLOCK-2]`/`[BLOCK-3]` option choice, prove packet 189 landed, and capture the do-not-regress baselines this packet must not move.
- Precondition: packet 189 reported `status: implemented`, **and** `design.md` carries the maintainer's `RESOLVED [BLOCK-1/1b/2/3]: option <A|B|C>` line. **`AC-18` is expected to FAIL until that decision is made, and that is its purpose** — this step is the gate, not a formality, and no implementer can clear it by writing code.
- Postcondition: the `AC-18` gate and the dependency probe both print PASS, and the four baseline counts are recorded in the swarm log (not frozen into any file).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/traits.rs` - locate `pub enum EntityMutation` only
- Files allowed to edit (at most 3):
  - none — read-only discovery step
- Files explicitly out of bounds:
  - everything; this step edits nothing
- Expected sub-agent dispatches:
  - Question: "Run the four baseline commands below and return only their `^test result:` lines."; scope: workspace; return: `FACT` ≤ 6 lines
- Context cost: `S`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'python3 -c "import io,os,re,sys; p=r\".ralph/specs/190-smoothed-overhang-speed/design.md\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); m=re.search(r\"^RESOLVED \\[BLOCK-1/1b/2/3\\]: option ([ABC])\", s, re.M); print(\"PASS: maintainer chose option \"+m.group(1) if m else \"FAIL: the four [BLOCK] items are unresolved - no line-anchored RESOLVED [BLOCK-1/1b/2/3]: option <A|B|C> marker in 190/design.md; this packet cannot be activated\")"'` - FACT (**AC-18, verbatim.** Named by no step's Verification block before this round, which is how a gate whose whole point is to stop activation ended up enforced by nothing an activation gate reads. It is line-anchored and names the option on purpose: a bare `RESOLVED:` substring probe printed PASS on the unresolved tree by matching this packet's own `UNRESOLVED:` prose.)
  - `bash -c 'rg -q "SetPointSpeedFactors" crates/slicer-sdk/src/traits.rs && rg -q "speed_profiles" crates/slicer-ir/src/slice_ir.rs && rg -q "speed_profiles_by_entity" crates/slicer-gcode/src/emit.rs && echo PASS || echo "FAIL: packet 189 has not landed — carrier, mutation or emit-side read is missing"'` - FACT
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd 2>&1 | rg "^test result:"'` - FACT (baseline; measured 6 passed before this packet)
  - `bash -c 'cargo test -p overhang-classifier-default --test slicer_module_binding_tdd 2>&1 | rg "^test result:"'` - FACT (baseline; measured 1 passed)
  - `bash -c 'cargo test -p slicer-runtime --test integration -- overhang_classifier_refactor_regression_tdd:: 2>&1 | rg "^test result:"'` - FACT (baseline; measured 2 passed, 245 filtered out)
  - `bash -c 'cargo test -p slicer-runtime --test integration -- overhang_pipeline_e2e_tdd:: 2>&1 | rg "^test result:"'` - FACT (baseline; measured 2 passed, 245 filtered out)
- Exit condition: **AC-18 prints PASS** and the dependency probe prints PASS. If either prints FAIL, stop — this packet cannot proceed and the queue row stays blocked. An `AC-18` FAIL is a maintainer decision, not an implementer task: do not resolve, weaken or work around the four `[BLOCK]` items to clear it.

### Step 1: Red tests for the new algorithm

- Task IDs: `TASK-309`
- Objective: write the **nine** new failing tests in `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs` — `calculate_speed_matches_canonical_interpolation_and_clamps`, `sixth_speed_section_follows_slowdown_for_curled_perimeters`, `section_speeds_resolve_against_ref_speed_not_original_speed`, `speed_sections_flatten_ties_without_removing_entries`, `per_point_factors_vary_within_one_entity`, **`interpolated_factor_is_not_a_quartile_value`**, `enable_overhang_speed_false_disables_all_mutations_and_absent_defaults_true`, `first_layer_emits_no_speed_mutation`, `non_wall_role_emits_no_mutation_and_no_nan` — and rewrite the existing quartile tests (`quartile_present_receives_speed_factor_below_one`, `quartile_four_is_honored`, plus the two curl tests' mutation matching) onto `EntityMutation::SetPointSpeedFactors`. Keep `quartile_absent_emits_no_mutation` and `all_zero_config_emits_no_mutations` semantically unchanged. **`interpolated_factor_is_not_a_quartile_value` (AC-16) was missing from this list for a round and is the packet's stated primary discriminator** — `AC-5` does not discriminate what it claims to, because with `overhang_quartile.is_some()` still the gate a per-point quartile lookup with zero interpolation yields e.g. `[1.0, 1.0, 0.5, 0.5]` and passes `AC-5` outright. A test the plan never produces is a criterion no step is accountable for; `AC-16`, `AC-17` and `AC-18` all had **zero** hits in this file before this round. **All new tests use `#[module_test]`, not `#[test]`** — measured on the current file: `#[module_test]` 6 occurrences, `#[test]` 0. A `#[test]` here will not be collected by the module harness and will silently not run.
- Precondition: Step 0 PASS.
- Postcondition: `cargo test -p overhang-classifier-default --test basic_tdd` fails to compile (the new symbols `calculate_speed`, `build_speed_sections`, `OVERHANG_OVERLAP_LEVELS` do not exist and the module still emits `SetSpeedFactor`). That is the correct red state; do not stub anything to make it link.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs` - read whole; it is the file being rewritten. (Its length is a ledger fact — re-derive it rather than budgeting against a frozen figure. An earlier revision of this line budgeted "under 350 lines"; the file was already over that before any edit, and this step then adds nine tests to it.)
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` (303 lines) - read whole **once**; this is that read
  - `crates/slicer-sdk/src/traits.rs` - locate `pub enum EntityMutation` and `MergeOp::ModifyEntity`, ±20 lines
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` (Steps 3-4)
  - `crates/slicer-runtime/**` (Step 4)
- Expected sub-agent dispatches:
  - Question: "In `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`, quote the `calculate_speed` lambda verbatim and state exactly what it returns below the first section, above the last, and in between."; scope: that file; return: `SNIPPETS` ≤ 1 of ≤ 25 lines plus ≤ 60 words
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `ExtrusionQualityEstimator::estimate_extrusion_quality` - delegate; never load
- Verification:
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd 2>&1 | rg -q "cannot find|no variant|E0425|E0433|E0599" && echo "RED as expected" || echo "FAIL: tests compiled — the new symbols must not exist yet"'` - FACT
- Exit condition: the binary fails to compile for the stated missing-symbol reason and no other.

### Step 2: Declare the three config keys and regenerate the config-key doc

- Task IDs: `TASK-309`
- Objective: add `enable_overhang_speed` (`bool`, `true`), `slowdown_for_curled_perimeters` (`bool`, `false`) and `bridge_speed` (`float`, `25.0`) to `[config.schema]` in `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`, each with a `display` string and `group = "Speed"` matching the existing rows' shape; then run `cargo xtask gen-config-docs`.
- Precondition: Step 1 complete.
- Postcondition: AC-1 and AC-13 print PASS; `binding_surface_matches_manifest` (`--test slicer_module_binding_tdd`) still passes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` - read whole (under 90 lines)
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` - locate one `type = "bool"` block, ±8 lines, for the exact key-block shape
  - `crates/slicer-ir/src/feedrate.rs` - locate `bridge_speed` in the struct and in `Default`, ±3 lines
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`
  - `docs/15_config_keys_reference.md` (**generator output only** — run `cargo xtask gen-config-docs`; never hand-edit inside the `module-config-keys` markers)
- Files explicitly out of bounds:
  - `modules/core-modules/*/` other than this module
  - `crates/slicer-ir/src/feedrate.rs` — read-only; the fields already exist
  - the interior of `<!-- BEGIN GENERATED: module-config-keys -->` in `docs/15_config_keys_reference.md`
- Expected sub-agent dispatches:
  - Question: "In `crates/slicer-ir/src/feedrate.rs`, what is `FeedrateConfig::default`'s `bridge_speed` value?"; scope: that file; return: `FACT` ≤ 2 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - **generator-owned**; interact only via `cargo xtask gen-config-docs` and `--check`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef::init_fff_params` for the two `coBools` defaults - delegate; never load
- Verification:
  - `bash -c 'python3 -c "import os,sys,tomllib; p=r\"modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; d=tomllib.load(open(p,\"rb\"))[\"config\"][\"schema\"]; ok=(d.get(\"enable_overhang_speed\",{}).get(\"type\")==\"bool\" and d[\"enable_overhang_speed\"][\"default\"] is True and d.get(\"slowdown_for_curled_perimeters\",{}).get(\"type\")==\"bool\" and d[\"slowdown_for_curled_perimeters\"][\"default\"] is False and d.get(\"bridge_speed\",{}).get(\"type\")==\"float\" and abs(d[\"bridge_speed\"][\"default\"]-25.0)<1e-9); print(\"PASS\" if ok else \"FAIL: enable_overhang_speed / slowdown_for_curled_perimeters / bridge_speed missing or wrong type-default\")"'` - FACT (AC-1)
  - `bash -c 'p=docs/15_config_keys_reference.md; if [ ! -f "$p" ]; then echo "FAIL: cannot open $p - run from the workspace root"; elif ! cargo xtask gen-config-docs --check >/dev/null 2>&1; then echo "FAIL: gen-config-docs --check is red - regenerate after the manifest edit"; else python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); missing=[k for k in (\"enable_overhang_speed\",\"slowdown_for_curled_perimeters\") if (b+k+b) not in s]; print(\"PASS\" if not missing else \"FAIL: missing generated rows for \"+str(missing))"; fi'` - FACT (**AC-13, verbatim.** `gen-config-docs --check` is measured **clean** (exit 0) on the unfixed tree, so a red check here means the regeneration was skipped. The `if`/`elif`/`else` shape is the fix for a real misattribution: the earlier `cargo … && python3 … || echo` form, run from a non-workspace cwd, fired its `||` branch and blamed `gen-config-docs` for what was a missing file. It also asserts two keys, not three — `bridge_speed` is already backticked in the doc today and would be non-discriminating; AC-1 covers its manifest declaration.)
  - `bash -c 'cargo test -p overhang-classifier-default --test slicer_module_binding_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo FAIL'` - FACT
- Exit condition: three PASS lines.

### Step 3: Port `speed_sections` and `calculate_speed`

- Task IDs: `TASK-309`
- Objective: add `OVERHANG_OVERLAP_LEVELS`, `build_speed_sections` and `calculate_speed` to `modules/core-modules/overhang-classifier-default/src/lib.rs` as pure functions, with the canonical provenance comments. Do **not** wire them into `run_finalization` yet — this step is proven entirely by the two pure-function tests.
- Precondition: Step 2 complete.
- Postcondition: AC-2, AC-3, AC-4, AC-14 and AC-15 print PASS.
- **Two signature constraints this step must honour, both from canonical and both easy to lose:** (1) `build_speed_sections` takes the role **reference speed** as a parameter and `calculate_speed` takes `original_speed` as a *different* parameter — canonical binds `ext_perimeter_speed` and `original_speed` separately and resolves every percentage section against the former only; the live PnP caller happens to pass the same value to both, which is why only `AC-14`'s deliberately-different-values test can catch a merge. (2) The post-sort tie handling is a **flatten** (overwrite the later entry's speed, keep the entry) and not a de-duplication; canonical has no `std::unique` or `erase`, and `AC-15` pins the six-entry count.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` - already read whole in Step 1; locate `fn overhang_speed` and `fn base_speed` and work adjacent to them
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/overhang-classifier-default/tests/**` (written in Step 1; do not weaken a red test to make it pass)
  - everything outside this module
- Expected sub-agent dispatches:
  - Question: "In `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`'s `GCode::_extrude`, quote both `dynamic_overhang_speeds` vector constructions verbatim and state the single difference between them."; scope: that file; return: `SNIPPETS` ≤ 2 of ≤ 25 lines plus ≤ 40 words
  - Question: "In `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`, quote the lines that build and sort `speed_sections`."; scope: that file; return: `SNIPPETS` ≤ 1 of ≤ 25 lines
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::_extrude` - delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `ExtrusionQualityEstimator::estimate_extrusion_quality` - delegate; never load
- Verification:
  - `bash -c 'rg -q "const OVERHANG_OVERLAP_LEVELS: \[f32; 6\] = \[90\.0, 75\.0, 50\.0, 25\.0, 13\.0, 0\.0\];" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "fn calculate_speed" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "speed_sections" modules/core-modules/overhang-classifier-default/src/lib.rs && echo PASS || echo "FAIL: OVERHANG_OVERLAP_LEVELS / calculate_speed / speed_sections not present"'` - FACT (AC-2)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- calculate_speed_matches_canonical_interpolation_and_clamps --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: calculate_speed_matches_canonical_interpolation_and_clamps did not run or did not pass"'` - FACT (AC-3)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- sixth_speed_section_follows_slowdown_for_curled_perimeters --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: sixth_speed_section_follows_slowdown_for_curled_perimeters did not run or did not pass"'` - FACT (AC-4)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- section_speeds_resolve_against_ref_speed_not_original_speed --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: section_speeds_resolve_against_ref_speed_not_original_speed did not run or did not pass"'` - FACT (AC-14)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- speed_sections_flatten_ties_without_removing_entries --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: speed_sections_flatten_ties_without_removing_entries did not run or did not pass"'` - FACT (AC-15)
- Exit condition: five PASS lines. `round` must be applied inside `calculate_speed`, not by the caller — canonical returns `round(final_speed)` and a test that clamps first and rounds later differs by up to 0.5 mm/s. `round` operates on **mm/s**, not on a multiplier: the division by the role base speed happens once, in Step 4, when the `SetPointSpeedFactors` vector is built.

### Step 4: Rewrite `run_finalization` per point, and update the TRIPWIRE mirror in the same step

- Task IDs: `TASK-309`
- Objective: add `distance_to_prev_boundary` (point-to-segment) and the previous-layer segment list to the produce phase; gate on `enable_overhang_speed` (absent ⇒ true) and skip `layers[0]`; per qualifying entity build a `factors` vector of exactly `path.points.len()` entries from `min(calculate_speed(d_curr), calculate_speed(d_next))` clamped to `original_speed` and then `min`ed with the curl-derived speed; emit one `EntityMutation::SetPointSpeedFactors`; delete `EntityMutation::SetSpeedFactor` and the now-callerless `quartile_for_distance`. **In the same step**, rewrite `mirrored_run_finalization` in `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs` to the identical rule, per that file's own TRIPWIRE.
- Precondition: Step 3 complete; AC-2/3/4 green.
- Postcondition: AC-5, AC-6, AC-7, AC-8, AC-9, AC-10, AC-N1 and AC-N2 all print PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` - already read whole; locate `fn run_finalization`, `nearest_reference_point`, `quartile_for_distance`
  - `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs` (303 lines) - the module doc-comment and `mirrored_run_finalization` only
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/fixtures/overhang_classifier_baseline_speeds.json` — **never load**; if a baseline number is needed, dispatch a `FACT` for that one field. It is a recorded pre-refactor capture and must not be re-recorded to make a test pass.
  - `crates/slicer-core/src/algos/overhang_annotation.rs` — its four concentric `overhang_quartile` bands and `BAND_BOUNDARY_MULTIPLIERS` (the **six overlap levels** the same file documents are what this packet restores, and that restoration is `[BLOCK-1b]` in `design.md` §Open Questions — not a settled exclusion)
  - `crates/slicer-gcode/**`, `crates/slicer-ir/**`, `crates/slicer-sdk/**` — packet 189's surface
- Expected sub-agent dispatches:
  - Question: "In `crates/slicer-runtime/tests/fixtures/overhang_classifier_baseline_speeds.json`, what does the `FACT` field say about the all-zero-config case, and what is the recorded factor for layer 4 / entity 1?"; scope: that file; return: `FACT` ≤ 5 lines
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `ExtrusionQualityEstimator::estimate_extrusion_quality`, specifically that the curled-edge `min` is applied **after** the `original_speed` clamp - delegate; never load
- Verification:
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- per_point_factors_vary_within_one_entity --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: per_point_factors_vary_within_one_entity did not run or did not pass"'` - FACT (AC-5)
  - `bash -c 'rg -q "EntityMutation::SetPointSpeedFactors" modules/core-modules/overhang-classifier-default/src/lib.rs && ! rg -q "EntityMutation::SetSpeedFactor" modules/core-modules/overhang-classifier-default/src/lib.rs && echo PASS || echo "FAIL: module still emits SetSpeedFactor, or does not emit SetPointSpeedFactors"'` - FACT (AC-6)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo FAIL'` - FACT (AC-7, AC-8, AC-10 first conjunct, AC-N1, AC-N2 all live in this binary)
  - `bash -c 'cargo test -p slicer-runtime --test integration -- overhang_classifier_refactor_regression_tdd:: 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && rg -q "SetPointSpeedFactors" crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs && echo PASS || echo "FAIL: the regression pair did not pass, or mirrored_run_finalization was not updated to the per-point rule"'` - FACT (AC-9; the structural conjunct is what detects a stale mirror)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- interpolated_factor_is_not_a_quartile_value --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && python3 -c "import io,os,sys; p=r\"modules/core-modules/overhang-classifier-default/src/lib.rs\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); calls=s.count(\"calculate_speed(\")-s.count(\"fn calculate_speed(\"); print(\"PASS\" if calls>=1 else \"FAIL: calculate_speed has \"+str(calls)+\" call sites - the helper exists but nothing calls it\")" || echo "FAIL: interpolated_factor_is_not_a_quartile_value did not run or did not pass"'` - FACT (**AC-16, verbatim.** This is the step that wires `calculate_speed` into `run_finalization`, so it is the step whose failure mode AC-16's call-site **count** detects: a helper that exists but is never called. AC-16 was named by no step's Verification block before this round, which is why the packet's stated primary discriminator was produced by nothing.)
  - `bash -c 'python3 -c "import io,os,sys; p=r\"modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); need=[\"quartile_present_receives_speed_factor_below_one\",\"quartile_four_is_honored\",\"curled_edge_triggers_slowdown_on_next_layer\",\"curled_edge_out_of_range_emits_no_mutation\"]; NL=chr(10); idx={t:s.find(\"fn \"+t) for t in need}; missing=[t for t in need if idx[t]<0]; end={t:min([x for x in (s.find(NL+chr(35)+chr(91),idx[t]+1),s.find(NL+chr(102)+chr(110)+chr(32),idx[t]+1)) if x>=0] or [len(s)]) for t in need if idx[t]>=0}; stale=[t for t in need if idx[t]>=0 and t!=\"curled_edge_out_of_range_emits_no_mutation\" and \"SetPointSpeedFactors\" not in s[idx[t]:end[t]]]; print(\"PASS\" if not missing and not stale else \"FAIL: deleted=\"+str(missing)+\" not-migrated-to-SetPointSpeedFactors=\"+str(stale))"'` - FACT (**AC-17, verbatim.** This step rewrites the four pre-existing tests, so it owns their migration; AC-17 is what stops one being silently deleted rather than changed. Also previously named by no step.)
- Exit condition: six PASS lines. If a regression test fails, the fix is in the module or the mirror — **never** in the recorded baseline fixture.

### Step 5: Rebuild guests and sweep the regression wall

- Task IDs: `TASK-309`
- Objective: rebuild the guest WASM artifacts invalidated by the guest `src/` edit, then run the full regression sweep.
- Precondition: Step 4 complete.
- Postcondition: AC-11 and AC-12 print PASS, and `cargo clippy --workspace --all-targets -- -D warnings` is clean (the deleted `quartile_for_distance` must not leave a dead-code warning).
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - on failure only, via `Grep` for `FAILED|panicked at|---- .* stdout ----` with `-C 5`; never re-run a test to see more output
- Files allowed to edit (at most 3):
  - none — validation step
- Files explicitly out of bounds:
  - all source files; a red test here is a design signal for Step 4, not something to patch in place
- Expected sub-agent dispatches:
  - Question: "Run `cargo xtask build-guests` then `cargo xtask build-guests --check`, and return whether `--check` reports any `STALE:` line."; scope: workspace; return: `FACT` ≤ 3 lines
  - Question: "Run the four verification commands below and return only their PASS/FAIL lines."; scope: workspace; return: `FACT` ≤ 5 lines
- Context cost: `M` (a 34-artifact guest rebuild plus four cargo runs; none of their output enters the implementer's context)
- Authoritative docs:
  - `CLAUDE.md` §"Guest WASM Staleness" - the mandatory `--check` procedure
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo xtask build-guests --check > target/guard-ac12-guests.txt 2>&1; rc=$?; if [ $rc -ne 0 ]; then echo "FAIL: build-guests --check exited $rc — see target/guard-ac12-guests.txt"; elif rg -q "STALE:" target/guard-ac12-guests.txt; then echo "FAIL: stale guests — rebuild with cargo xtask build-guests"; else echo PASS; fi'` - FACT (AC-12)
  - `bash -c 'cargo test -p overhang-classifier-default --tests 2>&1 | tee target/test-output.log | rg "^test result:" > target/guard-ac11-ohc.txt; rg -q "[1-9][0-9]* failed|^test result: FAILED" target/guard-ac11-ohc.txt && echo "FAIL: see target/test-output.log" || (rg -q "^test result: ok\. [1-9]" target/guard-ac11-ohc.txt && cargo test -p slicer-runtime --test integration -- overhang_pipeline_e2e_tdd:: 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: zero tests ran, or the overhang e2e pair regressed")'` - FACT (AC-11)
  - `bash -c 'cargo check --workspace --all-targets 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
  - `bash -c 'cargo clippy --workspace --all-targets -- -D warnings 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
- Exit condition: four PASS lines. Do not interpret any `slicer-runtime` failure before `--check` is clean.

### Step 6: Docs, the `DEV-009` progress paragraph, the new `DEV-###` row, and `TASK-309`

- Task IDs: `TASK-309`
- Objective: land every entry in `packet.spec.md` §Doc Impact Statement — the `docs/04_host_scheduler.md` and `docs/01_system_architecture.md` sentence rewrites, the `DEV-009` progress paragraph (row stays **`Open`**), one new `DEV-###` row for the previous-layer boundary proxy, and the `TASK-309` registration in `docs/07_implementation_status.md` outside the generated block.
- Precondition: Step 5 complete; all code ACs green.
- Postcondition: every doc verification command in `packet.spec.md` §Doc Impact Statement returns PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/04_host_scheduler.md` - the `SetSpeedFactor` sentence only, located by grep
  - `docs/01_system_architecture.md` - the `overhang-classifier-default` sentence only, located by grep
  - `docs/DEVIATION_LOG.md` - **delegate**; only the `DEV-009` row's tail and the highest `DEV-###`
  - `docs/07_implementation_status.md` - **delegate**; only the highest `TASK-###` row and the generated-block markers
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - doc-comment lines 1-40 only, for the accepted-deviation wording to quote
- Files allowed to edit (this step edits four docs; each has an independently-verified anchor):
  - `docs/04_host_scheduler.md`
  - `docs/01_system_architecture.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - the interior of `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` in `docs/07_implementation_status.md` — regenerated, never hand-edited
  - `docs/15_config_keys_reference.md` — already handled by the generator in Step 2
- Expected sub-agent dispatches:
  - Question: "Re-derive the highest `DEV-###` with `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, re-derive the highest `TASK-###` in `docs/07_implementation_status.md`, and confirm `TASK-309` has zero hits in both files."; scope: those two files; return: `FACT` ≤ 4 lines. **Re-derive at the moment of writing** — a parallel packet may have consumed the next ID since this packet was authored, which is exactly how a duplicate row gets filed.
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - delegated grep only
  - `docs/07_implementation_status.md` - delegated `FACT` only
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'rg -q "SetPointSpeedFactors" docs/04_host_scheduler.md && ! rg -q "config key as .SetSpeedFactor" docs/04_host_scheduler.md && echo PASS || echo FAIL'` - FACT
  - `bash -c 'rg -q "SetPointSpeedFactors" docs/01_system_architecture.md && echo PASS || echo FAIL'` - FACT
  - `bash -c 'rg -q "^\| DEV-009 .*Open" docs/DEVIATION_LOG.md && rg -q "^\| DEV-009 .*TASK-309" docs/DEVIATION_LOG.md && rg -q "^\| DEV-009 .*ext_perimeter_speed" docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` - FACT (the row must stay `Open`; packet 191 closes it. The `ext_perimeter_speed` conjunct forces the paragraph to record the `ref_speed`/`original_speed` collapse; both anchors have zero hits in the file today, verified)
  - `bash -c 'rg -q "unscaled_prev_layer|LinesDistancer" docs/DEVIATION_LOG.md && rg -q "OuterWall centerline" docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` - FACT (the new boundary-proxy row)
  - `bash -c 'python3 -c "import io,os,sys; p=r\"docs/07_implementation_status.md\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); B=\"<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->\"; E=\"<!-- END GENERATED: open-deviations -->\"; i=s.find(B); j=s.find(E); sys.exit(print(\"FAIL: open-deviations generated markers not found in \"+p)) if (i<0 or j<0 or j<i) else None; outside=s[:i]+s[j+len(E):]; print(\"PASS\" if \"TASK-309\" in outside else \"FAIL: TASK-309 is not registered OUTSIDE the open-deviations generated block\")"'` - FACT (**the §Doc Impact Statement `TASK-309` probe, verbatim.** A whole-file `rg -q "TASK-309"` cannot distinguish a row hand-added outside the markers — which this step requires — from one that landed inside the generated block and will be silently destroyed by the next `cargo xtask check-deviations`. Measured: `TASK-156` occurs both inside and outside that block on this tree today.)
- Exit condition: five PASS lines.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Read-only; dependency probe plus four baselines, all delegated |
| Step 1 | M | One test file rewritten; the single whole read of the module source happens here |
| Step 2 | S | Three manifest keys plus a generator run |
| Step 3 | M | Two pure functions plus two OrcaSlicer `SNIPPETS` dispatches |
| Step 4 | M | The `run_finalization` rewrite and the mandatory mirror update, in one step |
| Step 5 | M | 34-artifact guest rebuild plus four delegated cargo runs |
| Step 6 | S | Four docs, each with an independently-verified anchor |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate: `M`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS, including the do-not-regress guards AC-10, AC-11 and AC-12 that were already PASS before the packet started.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions: **NOT none — the supersession set is option-dependent and is the maintainer's `[BLOCK]` decision, which is unresolved.** An earlier revision of this line read "none — this packet supersedes nothing and reopens nothing", which is the honesty defect that matters most in this file, because a closure ceremony reads *this* section and not `design.md`. As designed, this packet reverses two `Accepted` ADRs (**ADR-0032**'s `max(overhang_quartile, curl_quartile)` merge and its "No new config keys" clause; **ADR-0031**'s deletion of the cross-layer wall-distance code and its "applies `EntityMutation::SetSpeedFactor`" decision) and restores both halves of the band deviation recorded in `crates/slicer-core/src/algos/overhang_annotation.rs`. Read `design.md` §Open Questions `[BLOCK-1]`, `[BLOCK-1b]`, `[BLOCK-2]` and `[BLOCK-3]` in full before touching this line; `AC-18` gates the decision and cannot be cleared by writing code. The set to reconcile at closure follows the chosen option:
  - **(A) conform** — nothing to supersede, but the packet has no honest content and the queue row is re-planned rather than closed.
  - **(B) supersede** — one new ADR superseding ADR-0031's consequence and ADR-0032's decision, **plus two `D-<n>-ADR-####-AMENDED` rows** (`…-ADR-0031-AMENDED` and `…-ADR-0032-AMENDED`) in `docs/DEVIATION_LOG.md`. Re-derive the `D-<n>` at the moment of writing with `rg -o '^\| D-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`.
  - **(C) prepass carrier first** — a **narrower** but still non-empty supersession: `AC-6` removes `EntityMutation::SetSpeedFactor` under every option and that is named in ADR-0031's Decision, and ADR-0032's curl-merge and no-new-keys clauses remain live. One amendment row at minimum; see `design.md` §Open Questions and its ADR-0008 note.
  - Independent of the option: the four-band `overhang_quartile` schedule (`BAND_BOUNDARY_MULTIPLIERS`) remains an accepted permanent deviation and the `DEV-009` row must say so — but do **not** let that true statement stand in for the supersession reconciliation above, which is a different question.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Re-run `cargo xtask build-guests --check` immediately before closure; a `STALE:` at closure invalidates every `slicer-runtime` result collected during the packet.
- Record remaining packet-local risk: the previous-layer boundary proxy (`OuterWall` centerline vs canonical's slice boundary) and whichever compensation decision the `[FWD]` in `design.md` resolved to. Both must be visible in the new `DEV-###` row, not only in the packet.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
