# Implementation Plan: 191-overhang-add-intersections

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Do not start until packet 190 is closed.** `build_speed_sections`, `calculate_speed` and `distance_to_prev_boundary` are hard prerequisites; without them there is no `min_distance` to gate the segmentation branch and no way for an inserted vertex to carry a different speed than its neighbours.
- Step 2 edits the five-file mutation chain plus the WIT. That chain cannot be split without leaving the workspace non-compiling mid-step. The authority for exceeding the usual three-file limit is `references/templates/design.md`'s "Target at most 3 primary files; **justify extras and consider splitting**" — the justification is given in `design.md` §Files in Scope. It is **not** `SKILL.md` §Packet Safety's blast-radius clause, which an earlier draft cited: that clause is scoped to "adding a new struct field or bumping a public schema/version constant", says nothing about an edit cap, and this packet does neither (Step 2's own Blast-radius bullet concedes the trigger is unmet).

## Steps

### Step 0: Confirm the 190 dependency and record the baselines

- Task IDs: `TASK-310`
- Objective: prove packets 189 and 190 landed, and capture the do-not-regress baselines. Every count below will have moved since this packet was authored — **re-derive, never pin.**
- Precondition: packet 190 reported `status: implemented`.
- Postcondition: the dependency probe prints PASS and the five baselines are recorded in the swarm log, not frozen into any file.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` - read whole **once**; this is that read
- Files allowed to edit (at most 3):
  - none — read-only discovery step
- Files explicitly out of bounds:
  - everything; this step edits nothing
- Expected sub-agent dispatches:
  - Question: "Run the five baseline commands below and return only their `^test result:` lines."; scope: workspace; return: `FACT` ≤ 7 lines
- Context cost: `S`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'rg -q "fn calculate_speed" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "OVERHANG_OVERLAP_LEVELS" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "fn build_speed_sections" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "fn distance_to_prev_boundary" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "EntityMutation::SetPointSpeedFactors" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "speed_profiles_by_entity" crates/slicer-gcode/src/emit.rs && echo PASS || echo "FAIL: packet 189 or 190 has not landed, or 190 named build_speed_sections / distance_to_prev_boundary differently"'` - FACT
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd 2>&1 | rg "^test result:"'` - FACT (baseline)
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | rg "^test result:"'` - FACT (baseline)
  - `bash -c 'cargo test -p slicer-runtime --test integration -- overhang_classifier_refactor_regression_tdd:: 2>&1 | rg "^test result:"'` - FACT (baseline)
  - `bash -c 'cargo test -p slicer-runtime --test executor 2>&1 | rg "^test result:"'` - FACT (baseline; the finalization mutation round-trip bucket)
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result:"'` - FACT (baseline)
- Exit condition: the dependency probe prints PASS. If it prints FAIL, stop — the queue row stays blocked. **`build_speed_sections` and `distance_to_prev_boundary` are deliberately in the probe, but only half of the stated rationale holds and the false half is corrected here.** `distance_to_prev_boundary` really is named only in 190's prose and pinned by no AC of 190's, so a 190 implementer could go fully AC-green under a different name and this step would still have printed PASS. **`build_speed_sections` is different: 190's `AC-14` names it by symbol** ("Given `build_speed_sections` called with a role reference speed of `60` …"), so that name *is* pinned upstream. An earlier revision of this line asserted "packet 190 names them only in prose and pins neither in an AC" for both. Keep both conjuncts — a symbol pinned in 190's AC is still worth asserting here, since 190 is `draft` and its ACs can move — but do not repeat the false justification.

### Step 1: Red tests for the insertion algorithm and the geometry channel

- Task IDs: `TASK-310`
- Objective: write the **nine** failing tests — `boundary_crossing_inserts_one_vertex_at_boundary_offset`, `min_spacing_filter_is_two_sided_quarter_flow_width`, `segmentation_gate_and_t_parameters_match_canonical`, `min_distance_is_smallest_slower_section_or_minus_one`, `crossing_segment_gains_vertices_on_the_original_polyline`, `no_insertion_when_gates_unmet_leaves_point_count_unchanged`, `none_distance_takes_the_no_insertion_path` (all in `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs`), plus `set_path_points_then_point_speed_factors_applies_in_order` and `set_path_points_rejects_empty_or_unclosed_loop` (`crates/slicer-sdk/tests/finalization_builder_tdd.rs`). **`none_distance_takes_the_no_insertion_path` (AC-N3) is written here** — Step 3 verifies it but this list omitted it. **Its fixture is constrained and the constraint is what makes it discriminate:** the previous layer must contribute **no `OuterWall` segments** (so `distance_to_prev_boundary` returns `None`), and the entity must carry **at least one segment with `line_len > 4.0` mm** while `min_distance <= 0` (no `speed_sections` entry slower than `original_speed`). On a layer with no boundary the crossing branch cannot fire at all, so the segmentation gate is the only discriminator; with a short segment or `min_distance > 0` a `None → 0.0` coercion produces no insertion anyway and the test passes against the exact defect it exists to catch — the same defect class `AC-5`'s `B != 0.5` condition was fixed for.
- Precondition: Step 0 PASS.
- Postcondition: both binaries fail to **compile** (`EntityMutation::SetPathPoints`, `insert_extended_points`, `min_distance_from_sections` do not exist). That is the correct red state; do not stub anything to make them link.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs` - read whole; it is the file being extended
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` - locate `modify_entity_set_point_speed_factors_applies` and `modify_entity_unknown_id_errors`, ±40 lines each
  - `crates/slicer-ir/src/slice_ir.rs` - locate `impl ExtrusionPath3D { fn is_closed }` and `ExtrusionRole::is_loop`, ±20 lines each
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs`
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs`
- Files explicitly out of bounds:
  - every `src/` file and the WIT (Steps 2-3)
  - `crates/slicer-runtime/**` (Step 4)
- Expected sub-agent dispatches:
  - Question: "In `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`'s `estimate_points_properties`, quote the segmentation block verbatim from the outer proximity test through `t1`, including the `a0` and `a1` definitions."; scope: that file; return: `SNIPPETS` ≤ 1 of ≤ 25 lines. **The test for AC-5 must be written from this quote, not from memory** — `a1` carries a leading `1.0f -` that `a0` does not.
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `estimate_points_properties` - delegate; never load
- Verification:
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd 2>&1 | rg -q "cannot find|no variant|E0425|E0433|E0599" && echo "RED as expected" || echo "FAIL: tests compiled — the new symbols must not exist yet"'` - FACT
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | rg -q "cannot find|no variant|E0433|E0599" && echo "RED as expected" || echo "FAIL: tests compiled — SetPathPoints must not exist yet"'` - FACT
- Exit condition: both binaries fail to compile for the stated missing-symbol reason and no other. AC-5’s test must contain at least one case where **`(next.distance + 3 * boundary_offset) / line_len != 0.5`**. **Do not use the earlier `a0 != 1.0 - a1` condition** — `packet.spec.md` AC-5 now records that it merely reduces to `curr.distance != next.distance` and is neither necessary nor sufficient, so grading Step-1 exit against it would hold the implementer to a condition this packet’s own AC declares refuted.

### Step 2: Add the `SetPathPoints` geometry channel and its `apply_to` guards

- Task IDs: `TASK-310`
- Objective: add `set-path-points(list<point3-with-width>)` to `variant entity-mutation` **and** `point3-with-width` to `world-finalization`'s `use slicer:types/geometry.{…}` list; add `EntityMutation::SetPathPoints(Vec<Point3WithWidth>)` to `crates/slicer-sdk/src/traits.rs`; implement the `apply_to` branch (replace `e.path.points` wholesale; reject an empty vector; reject a list that breaks the closing repeat when `ExtrusionRole::is_loop()` is true for the entity's role, quoting the emitter's own wording about a wall mutator dropping the closing repeat); mirror the case in `WitEntityMutation` and its `fm::EntityMutation` match arm, in `dispatch.rs`, and in `crates/slicer-macros/src/lib.rs`. Then rebuild guests.
- Precondition: Step 1 complete.
- Postcondition: AC-1, AC-2, AC-N1 and AC-11 print PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` (under 120 lines) - read whole
  - `crates/slicer-schema/wit/deps/types.wit` - locate `record point3-with-width`, ±3 lines
  - `crates/slicer-sdk/src/traits.rs` - locate `pub enum EntityMutation` and the `MergeOp::ModifyEntity` arm of `apply_to`, ±40 lines each
  - `crates/slicer-wasm-host/src/host.rs` - locate `pub enum WitEntityMutation` and the `fm::EntityMutation::SetSpeedFactor` arm, ±20 lines each
  - `crates/slicer-wasm-host/src/dispatch.rs` - locate `host::WitEntityMutation::SetSpeedFactor`, ±20 lines
  - `crates/slicer-macros/src/lib.rs` - locate `::slicer_sdk::traits::EntityMutation::SetSpeedFactor`, ±20 lines
  - `crates/slicer-gcode/src/emit.rs` - locate the defensive wall-closure check only, to quote its wording
  - `CLAUDE.md` §"WIT/Type Changes Checklist" and §"Guest WASM Staleness" - read both sections
- Files allowed to edit (the chain cannot be split without leaving the workspace non-compiling mid-step):
  - `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/**` (Steps 3-4)
  - `crates/slicer-ir/**` — no IR type changes; `Point3WithWidth` and `is_loop` already exist
  - any other `wit` file under `crates/slicer-schema/wit/`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field and no schema constant is added, so there is no struct-literal blast radius. The additive `variant` case is a **compile-error** blast radius on the host side (`host.rs`, `dispatch.rs` must handle it) and a **silent-drop** blast radius on the guest-macro side (`crates/slicer-macros/src/lib.rs` matches exhaustively but generates code, so a missing arm is a compile error inside a generated guest, surfacing only at `bash -c 'rg -q "SetPathPoints\(Vec<Point3WithWidth>\)" crates/slicer-sdk/src/traits.rs && rg -q "set-path-points\(list<point3-with-width>\)" crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit && rg -q "use slicer:types/geometry\.\{(\s*point3-with-width|[^}]*,\s*point3-with-width)" crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit && rg -q "SetPathPoints" crates/slicer-wasm-host/src/host.rs && rg -q "SetPathPoints" crates/slicer-wasm-host/src/dispatch.rs && rg -q "SetPathPoints" crates/slicer-macros/src/lib.rs && echo PASS || echo "FAIL: SetPathPoints missing from a channel file, or point3-with-width was not added to the world-finalization use list"'`). AC-1 asserts all five files for that reason.
  - Also confirm no other WIT world declares `entity-mutation`: dispatch the `LOCATIONS` question below before editing.
- Expected sub-agent dispatches:
  - Question: "Does any file under `crates/slicer-schema/wit/` or `modules/core-modules/*/wit-guest/` declare an `entity-mutation` variant that must be kept in sync?"; scope: those globs; return: `LOCATIONS` ≤ 10 entries
  - Question: "Run `cargo xtask build-guests` then `cargo xtask build-guests --check`; report whether `--check` prints any `STALE:` line."; scope: workspace; return: `FACT` ≤ 3 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated grep for the `entity-mutation (variant)` bullet only
  - `docs/05_module_sdk.md` - delegated grep for the `modify_entity` variant table only
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'rg -q "SetPathPoints\(Vec<Point3WithWidth>\)" crates/slicer-sdk/src/traits.rs && rg -q "set-path-points\(list<point3-with-width>\)" crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit && rg -q "use slicer:types/geometry\.\{(\s*point3-with-width|[^}]*,\s*point3-with-width)" crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit && rg -q "SetPathPoints" crates/slicer-wasm-host/src/host.rs && rg -q "SetPathPoints" crates/slicer-wasm-host/src/dispatch.rs && rg -q "SetPathPoints" crates/slicer-macros/src/lib.rs && echo PASS || echo "FAIL: SetPathPoints missing from a channel file, or point3-with-width was not added to the world-finalization use list"'` - FACT (AC-1)
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd -- set_path_points_then_point_speed_factors_applies_in_order --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: set_path_points_then_point_speed_factors_applies_in_order did not run or did not pass"'` - FACT (AC-2)
  - `bash -c 'cargo test -p slicer-sdk --test finalization_builder_tdd -- set_path_points_rejects_empty_or_unclosed_loop --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: set_path_points_rejects_empty_or_unclosed_loop did not run or did not pass"'` - FACT (AC-N1)
  - `bash -c 'cargo xtask build-guests --check > target/guard-ac11-guests.txt 2>&1; rc=$?; if [ $rc -ne 0 ]; then echo "FAIL: build-guests --check exited $rc — see target/guard-ac11-guests.txt"; elif rg -q "STALE:" target/guard-ac11-guests.txt; then echo "FAIL: stale guests — rebuild with cargo xtask build-guests"; else echo PASS; fi'` - FACT (AC-11)
- Exit condition: four PASS lines. Do not proceed while `--check` reports `STALE:` — every later `slicer-runtime` failure would be unattributable.

### Step 3: Port `estimate_points_properties`' two insertion branches

- Task IDs: `TASK-310`
- Objective: add `segment_intersections`, `min_distance_from_sections` and `insert_extended_points` to `modules/core-modules/overhang-classifier-default/src/lib.rs`. **`insert_extended_points` takes `distances: &[Option<f32>]` and returns `Vec<Option<f32>>`** — packet 190’s `distance_to_prev_boundary` returns `Option<f32>`, and a `None` means "no previous-layer boundary to measure against". **Normative unwrap rule (`design.md` §Locked Assumptions, defended by `AC-N3`):** every gate consuming a `None` takes the **no-insertion** path — a segment either of whose endpoint distances is `None` is skipped by both branches, and a candidate whose re-queried distance is `None` is discarded. Never substitute `0.0`, `f32::MAX` or `-1.0`. Then, as before, with canonical provenance comments naming `estimate_points_properties` and `ExtrusionQualityEstimator::estimate_extrusion_quality` by function name only. Do **not** wire them into `run_finalization` yet — this step is proven by the four pure-function tests.
- Precondition: Step 2 complete.
- Postcondition: AC-3, AC-4, AC-5, AC-6 and AC-N2 print PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` - already read whole in Step 0; locate `build_speed_sections`, `calculate_speed`, `distance_to_prev_boundary` and work adjacent to them
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/overhang-classifier-default/tests/**` (written in Step 1; **never weaken a red test to make it pass** — `CLAUDE.md` Test Discipline forbids it explicitly)
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` — every constant here is canonical; no config key is added
  - everything outside this module
- Expected sub-agent dispatches:
  - Question: "In `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`'s `estimate_points_properties`, quote the threshold-crossing block verbatim — the XOR predicate, the `intersections_with_line` call, the `p.distance` assignment and the two-sided `min_spacing` test."; scope: that file; return: `SNIPPETS` ≤ 1 of ≤ 25 lines
  - Question: "In the same function, what exactly is `min_spacing`, and is the `p1` candidate's spacing test compared against `p0` or against `curr.position`?"; scope: that file; return: `FACT` ≤ 5 lines
  - Question: "In `ExtrusionQualityEstimator::estimate_extrusion_quality`, quote the `smallest_distance_with_lower_speed` computation including the `found` flag and the `-1.f` fallback."; scope: that file; return: `SNIPPETS` ≤ 1 of ≤ 15 lines
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `estimate_points_properties` and `ExtrusionQualityEstimator::estimate_extrusion_quality` - delegate; never load
- Verification:
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- boundary_crossing_inserts_one_vertex_at_boundary_offset --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: boundary_crossing_inserts_one_vertex_at_boundary_offset did not run or did not pass"'` - FACT (AC-3)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- min_spacing_filter_is_two_sided_quarter_flow_width --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && rg -q "min_spacing" modules/core-modules/overhang-classifier-default/src/lib.rs && rg -q "min_spacing[^=;]*=\s*[^;]*0\.25" modules/core-modules/overhang-classifier-default/src/lib.rs && echo PASS || echo "FAIL: the two-sided min_spacing test did not pass, or min_spacing is absent / not defined with the 0.25 factor"'` - FACT (AC-4)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- segmentation_gate_and_t_parameters_match_canonical --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: segmentation_gate_and_t_parameters_match_canonical did not run or did not pass"'` - FACT (AC-5)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- min_distance_is_smallest_slower_section_or_minus_one --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: min_distance_is_smallest_slower_section_or_minus_one did not run or did not pass"'` - FACT (AC-6)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- no_insertion_when_gates_unmet_leaves_point_count_unchanged --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: no_insertion_when_gates_unmet_leaves_point_count_unchanged did not run or did not pass"'` - FACT (AC-N2)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- none_distance_takes_the_no_insertion_path --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: none_distance_takes_the_no_insertion_path did not run or did not pass"'` - FACT (AC-N3; the `None` unwrap rule, which had no defending test before round 3)
- Exit condition: six PASS lines. Record the `p0`/`p1` `min_spacing` decision now; Step 5 files its `DEV-###` row.

### Step 4: Wire the insertion into `run_finalization`, and update the TRIPWIRE mirror in the same step

- Task IDs: `TASK-310`
- Objective: in `run_finalization`, after the per-point distances are built, call `insert_extended_points`; when the returned list is longer, emit `EntityMutation::SetPathPoints(new_points)` **then** `EntityMutation::SetPointSpeedFactors(new_factors)`; when it is unchanged, emit only the profile exactly as packet 190 does. **In the same step**, update `mirrored_run_finalization` in `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs` per that file's TRIPWIRE.
- Precondition: Step 3 complete; AC-3 through AC-6 green.
- Postcondition: AC-7, AC-8 and AC-9 print PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/overhang-classifier-default/src/lib.rs` - locate `fn run_finalization` only
  - `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs` (over 300 lines) - module doc-comment and `mirrored_run_finalization` only
- Files allowed to edit (at most 3):
  - `modules/core-modules/overhang-classifier-default/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/fixtures/overhang_classifier_baseline_speeds.json` — **never load and never re-record.** If a baseline number is needed, dispatch a `FACT` for that one field. `CLAUDE.md` Test Discipline: update fixtures only when the canonical-correct output genuinely changed, and never to make a test pass.
  - `crates/slicer-gcode/**`, `crates/slicer-ir/**`, `crates/slicer-sdk/**` — packets 189/191-Step-2 surfaces
  - `crates/slicer-core/src/algos/overhang_annotation.rs` — its four concentric quartile bands and `BAND_BOUNDARY_MULTIPLIERS` are untouched by 189/190/191 (the six overlap levels it also documents are packet 190’s business, not this packet’s)
- Expected sub-agent dispatches:
  - Question: "In `crates/slicer-runtime/tests/fixtures/overhang_classifier_baseline_speeds.json`, what does the `FACT` field say about the all-zero-config case?"; scope: that file; return: `FACT` ≤ 3 lines
- Context cost: `M`
- Authoritative docs:
  - none for this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` - `ExtrusionQualityEstimator::estimate_extrusion_quality`, for the fact that `calculate_speed` is evaluated over the **returned** `extended_points`, i.e. after insertion - delegate; never load
- Verification:
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- crossing_segment_gains_vertices_on_the_original_polyline --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: crossing_segment_gains_vertices_on_the_original_polyline did not run or did not pass"'` - FACT (AC-7)
  - `bash -c 'cargo test -p slicer-runtime --test integration -- overhang_classifier_refactor_regression_tdd:: 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && rg -q "SetPathPoints" crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs && echo PASS || echo "FAIL: the regression pair did not pass, or mirrored_run_finalization was not updated for the geometry mutation"'` - FACT (AC-8; the structural conjunct detects a stale mirror)
  - `bash -c 'cargo test -p overhang-classifier-default --test basic_tdd -- all_zero_config_emits_no_mutations --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && cargo test -p slicer-runtime --test integration -- overhang_classifier_refactor_regression_tdd::default_config_case_a_matches_baseline_zero_mutations --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the zero-config no-mutation guarantee regressed"'` - FACT (AC-9)
- Exit condition: three PASS lines. If a regression test fails, the fix is in the module or the mirror — never in the recorded baseline fixture.

### Step 5: Rebuild guests, sweep the regression wall, and close `DEV-009`

- Task IDs: `TASK-310`
- Objective: rebuild the guests, run the full sweep, then land every entry in `packet.spec.md` §Doc Impact Statement — the `docs/05_module_sdk.md` and `docs/03_wit_and_manifest.md` rows, the `docs/02_ir_schemas.md` §IR-10 ordering rule, the `TASK-310` registration in `docs/07_implementation_status.md`, the `DEV-009` closure, one residual `DEV-###` row for the `p0`/`p1` `min_spacing` decision recorded in Step 3, and `cargo xtask check-deviations` to regenerate the open-deviations block.
- Precondition: Step 4 complete; all code ACs green.
- Postcondition: AC-10, AC-11 and AC-12 print PASS, and every doc verification command in `packet.spec.md` §Doc Impact Statement returns PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - on failure only, via `Grep` for `FAILED|panicked at|---- .* stdout ----` with `-C 5`; never re-run a test to see more output
  - `docs/05_module_sdk.md` - the `modify_entity` variant table only, located by grep
  - `docs/03_wit_and_manifest.md` - the `entity-mutation (variant)` bullet only, located by grep
  - `docs/02_ir_schemas.md` (long; ranged reads only — a line count is a ledger fact, do not pin one) - §"IR 10 — LayerCollectionIR" through the start of §"IR 11" **only**
  - `docs/DEVIATION_LOG.md` - **delegate**; the `DEV-009` row's tail and the highest `DEV-###`
  - `docs/07_implementation_status.md` - **delegate**; the highest `TASK-###` row and the generated-block markers
- Files allowed to edit (this step edits five docs; each has an independently-verified anchor):
  - `docs/05_module_sdk.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/02_ir_schemas.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - all source files; a red test here is a design signal for Step 3 or 4, not something to patch in place
  - the interior of `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` in `docs/07_implementation_status.md` — regenerated, never hand-edited
- Expected sub-agent dispatches:
  - Question: "Re-derive the highest `DEV-###` with `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, re-derive the highest `TASK-###` in `docs/07_implementation_status.md`, and confirm `TASK-310` has zero hits in both."; scope: those two files; return: `FACT` ≤ 4 lines. **Re-derive at the moment of writing** — packet 190 filed a row and a parallel packet may have filed more since; a frozen number is how a duplicate row gets committed.
  - Question: "Run the six verification commands below and return only their PASS/FAIL lines."; scope: workspace; return: `FACT` ≤ 8 lines
- Context cost: `M` (a 34-artifact guest rebuild, six cargo runs, five doc edits; no cargo output enters the implementer's context)
- Authoritative docs:
  - `docs/02_ir_schemas.md` - ranged read as above
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` - delegated `FACT` only
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `bash -c 'cargo xtask build-guests --check > target/guard-ac11-guests.txt 2>&1; rc=$?; if [ $rc -ne 0 ]; then echo "FAIL: build-guests --check exited $rc — see target/guard-ac11-guests.txt"; elif rg -q "STALE:" target/guard-ac11-guests.txt; then echo "FAIL: stale guests — rebuild with cargo xtask build-guests"; else echo PASS; fi'` - FACT (AC-11)
  - `bash -c 'cargo test -p overhang-classifier-default --tests 2>&1 | tee target/test-output.log | rg "^test result:" > target/guard-ac10-addint.txt; rg -q "[1-9][0-9]* failed|^test result: FAILED" target/guard-ac10-addint.txt && echo "FAIL: see target/test-output.log" || (rg -q "^test result: ok\. [1-9]" target/guard-ac10-addint.txt && cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && cargo test -p slicer-runtime --test integration -- overhang_pipeline_e2e_tdd:: 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: zero tests ran, or finalization_builder_tdd / the overhang e2e pair regressed")'` - FACT (AC-10)
  - `bash -c 'cargo test -p slicer-runtime --test executor 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo FAIL'` - FACT
  - `bash -c 'cargo clippy --workspace --all-targets -- -D warnings 2>&1 | rg -q "^error" && echo FAIL || echo PASS'` - FACT
  - `bash -c 'rg -q "SetPathPoints" docs/05_module_sdk.md && rg -q "set-path-points" docs/03_wit_and_manifest.md && rg -q "min_spacing" docs/DEVIATION_LOG.md && echo PASS || echo "FAIL: docs/05_module_sdk.md, docs/03_wit_and_manifest.md or the p0/p1 min_spacing residual row is missing"'` - FACT
  - `bash -c 'python3 -c "import io,os,sys; p=r\"docs/07_implementation_status.md\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; s=io.open(p,encoding=\"utf-8\").read(); B=\"<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->\"; E=\"<!-- END GENERATED: open-deviations -->\"; i=s.find(B); j=s.find(E); sys.exit(print(\"FAIL: open-deviations generated markers not found in \"+p)) if (i<0 or j<0 or j<i) else None; outside=s[:i]+s[j+len(E):]; print(\"PASS\" if \"TASK-310\" in outside else \"FAIL: TASK-310 is not registered OUTSIDE the open-deviations generated block\")"'` - FACT (**the §Doc Impact Statement `TASK-310` probe, verbatim.** Split out of the doc-grep chain above because a whole-file `rg -q "TASK-310"` cannot distinguish a row hand-added outside the markers — which this step requires — from one that landed inside the generated block and is destroyed by the `cargo xtask check-deviations` this same step runs. Measured: `TASK-156` occurs both inside and outside that block on this tree today. **This step runs `check-deviations` at its exit, so an inside-the-block row would be destroyed within the step itself.**)
  - `bash -c 'python3 -c "import io,re,os,sys; p=r\"docs/DEVIATION_LOG.md\"; sys.exit(print(\"FAIL: cannot open \"+p+\" - run from the workspace root\")) if not os.path.exists(p) else None; L=io.open(p,encoding=\"utf-8\").read().splitlines(); rows=[l for l in L if re.match(r\"\|\s*DEV-009\b\",l)]; print(\"FAIL: no DEV-009 row\") if not rows else print(\"PASS\" if (\"Closed\" in rows[0] and \"TASK-310\" in rows[0] and \"speed sections\" in rows[0]) else \"FAIL: DEV-009 is not Closed, lacks TASK-310, or does not record that the {90,75,50,25,13,0} overlap levels return as emission-time speed sections\")"'` - FACT (AC-12)
- Exit condition: seven PASS lines, and `cargo xtask check-deviations` has been run so the generated open-deviations block reflects `DEV-009`'s closure. Re-run the `TASK-310` probe **after** `check-deviations`, not before — that is the run that would destroy a row mistakenly placed inside the markers.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Read-only; dependency probe plus five baselines, all delegated. The single whole read of the module source happens here |
| Step 1 | M | Two test files, eight tests; the AC-5 test must be written from a verbatim canonical quote |
| Step 2 | M | Five-file mutation chain plus the WIT `use` list, plus a 34-artifact guest rebuild |
| Step 3 | M | The densest algorithm in the three-packet set; three bounded OrcaSlicer dispatches |
| Step 4 | M | Wiring plus the mandatory mirror update, in one step |
| Step 5 | M | Guest rebuild, six delegated cargo runs, five doc edits including the `DEV-009` closure |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate: `M`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS, including the do-not-regress guards AC-9, AC-10 and AC-11.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read; then run `cargo xtask check-deviations` so the generated block reflects `DEV-009`'s closure.
- Reconcile reopened/superseded status transitions: none. `DEV-009` closes here. **Write the closing row in the settled framing, which is what `AC-12` actually checks:** `annotate_overhangs`’ four concentric `overhang_quartile` bands (`BAND_BOUNDARY_MULTIPLIERS` in `crates/slicer-core/src/algos/overhang_annotation.rs`) are unchanged, **while canonical’s `{90, 75, 50, 25, 13, 0}` overlap levels return as emission-time speed sections in packet 190**. AC-12’s third conjunct is the literal phrase `speed sections` (0 hits in the file today); an earlier draft of this gate told the implementer to restate that "the four-band `overhang_quartile` schedule remains an accepted permanent deviation", which AC-12 no longer checks — following it would have produced a sentence that leaves AC-12 red.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Re-run `cargo xtask build-guests --check` immediately before closure; a `STALE:` at closure invalidates every `slicer-runtime` result collected during the packet.
- Record remaining packet-local risk: the `p0`/`p1` `min_spacing` decision (whichever way it went, its `DEV-###` row must exist), and the fact that no whole-output G-code check is available to confirm the end-to-end effect because `DEV-093` records the pipeline is not byte-deterministic run-to-run. State that limit explicitly rather than implying byte-level verification happened.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
