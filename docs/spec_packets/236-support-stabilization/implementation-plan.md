# Implementation Plan: 236-support-stabilization

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs (TASK-344..TASK-352).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step N".
- Before attributing ANY guest/parity test failure to code: `cargo xtask build-guests --check` must have returned exit 0 this session (T4). Every cargo test invocation tees combined output to `target/test-output.log`; results are read from the file, never re-run (E5/T3).

## Steps

### Step 1: AC-8 — per-region family assignment minting (TASK-344)

- Task IDs: `TASK-344`
- Objective: change `commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`, mint loop ~253–279) so `family_assignments` are minted per RegionMap region per `(object, layer)`, giving candidate-less regions a structured assignment via the same `config_for_region_smallest_chain` → `support_family` → `"traditional"` lookup. Do NOT re-attempt the mesh-path-gate hypothesis (plan T11).
- Precondition: working tree clean on `parity/support-planners-clean`; `cargo xtask build-guests --check` exit 0.
- Postcondition: `planner_emits_one_entry_per_region_in_region_map` passes with its assertions unmodified (2 entries/layer, region_ids {7,42}, byte-identical skeletons); existing producer unit tests (`support_analysis_producer.rs` in-file tests at ~980/~1046) stay green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - lines 37–120, 230–300 (mint region), in-file tests ~900–1060
  - `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` - lines 480–595
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  - `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` (only if a decline-record observation is added; the target test's assertions stay untouched)
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support-planner/src/lib.rs` (full load), any planner/renderer source, other packets' dirs
- Blast-radius discipline: no struct field added; additive map rows only. Verify consumers tolerate assignment-without-candidate: run `structured_support_identity` integration suite (command below).
- Expected sub-agent dispatches:
  - Question: confirm RegionMap iteration API used by the producer today (entry key type + ordering); scope: `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` lines 56–120; return: SNIPPETS ≤30 lines; purpose: deterministic region walk.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` §3 Ruling 1, §12 brief item 1 - delegated SUMMARY
- OrcaSlicer refs: none (host-side routing fix).
- Verification:
   - `cargo test -p slicer-runtime --test executor -- prepass_support_geometry_layer_plan_tdd::planner_emits_one_entry_per_region_in_region_map --exact | tee target/test-output.log` - FACT pass/fail (AC-1)
  - `cargo test -p slicer-runtime --test integration -- structured_support_identity --exact | tee -a target/test-output.log` - FACT pass/fail (consumer tolerance)
- Exit condition (falsifying): if the count assertion still fails with per-region minting in place AND guests verified fresh, STOP — the Ruling-1 root cause is wrong; report to orchestrator instead of weakening the assertion.

### Step 2: G-21 red fixtures — negative + positive validator tests (TASK-345)

- Task IDs: `TASK-345`
- Objective: author three failing-first/guard scheduler tests in the `scheduler_unit` binary: `family_scoped_support_claims_do_not_conflict_globally` (full-directory topology: four support claims multi-held + family planners writing `SupportPlanIR`/`SupportIR` → zero advisories after Step 3), `genuine_claim_conflict_still_rejected_after_family_exemption` (two non-support same-claim holders → `ClaimConflict` still reported, AC-N1 claim half), and `genuine_write_conflict_still_rejected_after_aggregation_recognition` (two modules writing the same IR field with no ADR-recognized aggregation/orderability relation between them → `WriteConflict` still reported after Step 3, AC-N1 write half).
- Precondition: Step 1 merged locally (test ordering independent but keeps the branch coherent).
- Postcondition: all three new tests exist and fail/pass for the right reason (advisories currently emitted / conflict machinery present).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/validation.rs` - lines 480–580 (exemption precedent), 684–722 (write conflicts), 100–200 (error types)
  - `crates/slicer-scheduler/tests/unit/stage_canon_seam_support_tdd.rs` - lines 21–105 (`dag_validation_request_base` fixture builder to mirror)
- Files allowed to edit (at most 3):
  - the scheduler validation test file hosting the new tests (one file; home: `crates/slicer-scheduler/tests/unit/` — `stage_canon_seam_support_tdd.rs` already builds `DagValidationRequest`s there via `dag_validation_request_base`)
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/validation.rs` THIS step (Step 3 owns it), production manifests, `docs/adr/0059-*.md` (read-only; its write-topology clause is quoted, not amended)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` row G-21 - direct range read
- OrcaSlicer refs: none.
- Verification (one TESTNAME per invocation — Cargo accepts a single positional filter before `--exact`):
  - `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::family_scoped_support_claims_do_not_conflict_globally --exact 2>&1 | tee target/test-output.log` - FACT: FAIL expected (red)
  - `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_claim_conflict_still_rejected_after_family_exemption --exact 2>&1 | tee -a target/test-output.log` - FACT pass (negative case already green against current validator — it guards the exemption landing)
  - `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_write_conflict_still_rejected_after_aggregation_recognition --exact 2>&1 | tee -a target/test-output.log` - FACT pass (same guard posture, write-conflict side)
- Exit condition (falsifying): any of the three tests cannot be authored against real request-builder types → placement or API assumption wrong; re-dispatch before writing.

### Step 3: G-21 — family-scoped claim exemption + aggregation orderability in validate_startup_dag (TASK-346)

- Task IDs: `TASK-346`
- Objective: two changes to `validate_startup_dag`. (a) CLAIMS: extend the global `GlobalClaimConflicts` pass with a family-scoped exemption set {`support-generator`, `support-planner`, `support-family:traditional`, `support-family:tree`}, mirroring the fill-role block at ~553; per-region pass unchanged. (b) WRITES: in `validate_write_conflicts`, recognize ADR-0059's stated write topology as ORDERABLE — when both modules of a conflicting pair hold family-scoped support claims and the shared field is `SupportPlanIR` / `SupportIR`, the host-aggregator aggregation edge IS the ordering (ADR-0059: "the host aggregator is the sole writer of the aggregated `SupportPlanIR`, and family planners emit only family-scoped entries for host aggregation"), so treat it like `left_transforms_right || right_transforms_left`. No pair is blanket-exempted; genuine conflicts stay reported. This preserves ADR-0059 verbatim — no amendment deviation, no superseding ADR.
- Precondition: Step 2's tests exist and are red/green as specified.
- Postcondition: AC-2 and both AC-N1 halves pass; `docs/04_host_scheduler.md` "Validation Passes" gains the orderability-recognition sentence (same commit).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/validation.rs` - lines 480–580, 684–742
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/validation.rs`
  - the Step 2 test file (turn the positive test green expectations)
  - `docs/04_host_scheduler.md` (one sentence in "Validation Passes")
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/execution_plan.rs`, manifests, other validators
- Expected sub-agent dispatches:
  - Question: SUMMARY of `docs/04_host_scheduler.md` "Validation Passes" wording around the fill-role exemption sentence; return: SUMMARY ≤200 words; purpose: matching doc style.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` G-21 - direct range read
  - `docs/04_host_scheduler.md` - delegated SUMMARY only
- OrcaSlicer refs: none.
- Verification (one TESTNAME per invocation):
  - `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::family_scoped_support_claims_do_not_conflict_globally --exact 2>&1 | tee target/test-output.log` - FACT pass/fail (AC-2)
  - `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_claim_conflict_still_rejected_after_family_exemption --exact 2>&1 | tee -a target/test-output.log` - FACT pass/fail (AC-N1 claim half)
  - `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_write_conflict_still_rejected_after_aggregation_recognition --exact 2>&1 | tee -a target/test-output.log` - FACT pass/fail (AC-N1 write half)
  - `rg -q 'family-scoped' docs/04_host_scheduler.md && echo PASS || echo FAIL` - FACT PASS/FAIL
- Exit condition (falsifying): exemption requires touching per-region pass logic → scope creep; stop and report.

### Step 4: G-23 — tripwire real collision/avoidance inputs + classified rebless (TASK-347)

- Task IDs: `TASK-347`
- Objective: replace `SupportGeometryView { entries: vec![] }` (`orca_parity_tdd.rs` ~421) with occupancy derived from `overhang_plate_fixture` polygons so collision and avoidance ladders execute; add the marker comment `G-23 fixture precondition` immediately before the non-empty-entries assertion; classify drift vs the current goldens in writing (E3); record justification in `docs/DEVIATION_LOG.md` (re-derive next DEV id by grep at write time); rebless via `SUPPORT_PLANNER_REGEN_GOLDEN=1` (the live gate — `SUPPORT_WEDGE_REGEN_GOLDEN` has zero live-code hits; see design.md Plan Corrections).
- Precondition: fresh guests (`cargo xtask build-guests --check` exit 0); old-golden baseline captured (run tripwire once pre-change and record counts).
- Postcondition: AC-3 and AC-4 pass WITHOUT the regen env var; drift classification text exists in the deviation log; golden files updated at workspace-root `resources/golden/`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` - lines 380–560, helper fns 860–1050
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`
  - `resources/golden/benchy_tree_support_regression_branch_count.txt` (env-gate write only)
  - `resources/golden/benchy_tree_support_regression_endpoints.txt` (env-gate write only)
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support-planner/src/lib.rs` (any edit = scope drift; STOP and report), `docs/DEVIATION_LOG.md` handled via its own single-row append within this step's third edit slot if needed (see note)
- Note: if the deviation-log row cannot share an edit slot (4 files needed), split into Steps 4a (code+marker) and 4b (log+rebless) rather than widening reads.
- Expected sub-agent dispatches:
  - Question: re-derive highest DEV-### open row in `docs/DEVIATION_LOG.md`; scope: that file; return: FACT 1 line; purpose: free deviation id.
- Context cost: `M`
- Authoritative docs:
  - plan §7 E3 (tolerances frozen), §13 T6/T7 - delegated SUMMARY
- OrcaSlicer refs: none (self-captured golden discipline only).
- Verification:
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT exit=0 required first
  - `SUPPORT_PLANNER_REGEN_GOLDEN=1 cargo test -p tree-support-planner -- benchy_tree_support_regression_tripwire --exact 2>&1 | tee target/test-output.log` - FACT regenerated (once, after classification recorded)
  - `cargo test -p tree-support-planner -- benchy_tree_support_regression_tripwire --exact 2>&1 | tee target/test-output.log` - FACT pass/fail (AC-3)
  - `rg -q 'G-23 fixture precondition' modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs && echo PASS || echo FAIL` - FACT PASS/FAIL (AC-4)
- Exit condition (falsifying): strengthening inputs requires editing planner source, or drift exceeds tolerances in a way classification cannot justify → STOP; report; do not widen tolerances.

### Step 5: G-22 — declare threshold-angle bounds + regenerate config docs (TASK-348)

- Task IDs: `TASK-348`
- Objective: add `[config.schema.support_threshold_angle]` and `[config.schema.support_overhang_angle]` (float, min 0, max 90, default 30.0, group "Support", alias cross-referenced) to `traditional-support-planner.toml`; author negative test `out_of_range_support_threshold_angle_is_rejected` (AC-N2, expects `ConfigResolutionError::OutOfRange` via the TASK-182 bounds mechanism; test home: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, which already asserts `ConfigResolutionError::OutOfRange`) red-first; then run `cargo xtask gen-config-docs` in the SAME commit (T8).
- Precondition: Step 2 dispatch identified where scheduler config-resolution tests live (reuse for AC-N2 placement).
- Postcondition: AC-5 greps pass; AC-N2 passes; `gen-config-docs --check` exit 0; `docs/15_config_keys_reference.md` generated tables show both keys.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` - lines 27–90 (schema block shape)
  - `crates/slicer-ir/src/resolved_config.rs` - lines 955–975 (macro line + semantics doc)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (AC-N2 test)
  - `docs/15_config_keys_reference.md` (via `cargo xtask gen-config-docs` only — never hand-edited)
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (macro line already correct; no host clamp needed), other module manifests
- Expected sub-agent dispatches:
  - None required (all shapes verified at authoring time).
- Context cost: `S`
- Authoritative docs:
  - `docs/config/host-keys.toml` line 57 - direct read (documentation-only range record being superseded by enforcement)
  - `docs/specs/support-parity-gap-register.md` G-22 - direct range read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` `support_threshold_angle` - delegate LOCATIONS only if the declared range is disputed (in-tree macro doc already records coInt min 0 max 90 default 30).
- Verification:
   - `cargo test -p slicer-scheduler --test scheduler_integration -- config_bounds_enforcement_tdd::out_of_range_support_threshold_angle_is_rejected --exact 2>&1 | tee target/test-output.log` - FACT pass/fail (AC-N2)
  - `rg -q '^\[config\.schema\.support_threshold_angle\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q '^\[config\.schema\.support_overhang_angle\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && cargo xtask gen-config-docs --check` - FACT exit 0 (AC-5)
- Exit condition (falsifying): declaring the alias key changes resolution behavior for existing configs (double declaration collides in `ConfigBoundsIndex`) → investigate strictest-wins merge before committing; if the mechanism rejects alias+canonical co-declaration, keep canonical declared and document the alias as a comment-only row, reporting the deviation.

### Step 6: G-24 — guest freshness assertion in the parity harness (TASK-349)

- Task IDs: `TASK-349`
- Objective: add `assert_guest_freshness(wasm_path)` to `run_integrated_parity` (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`) mirroring `is_stale`'s newest-source-vs-artifact comparison (pattern documented by `pnp_cli_locator::staleness_reason`), panicking with the stale-reason before bindings are built; all four support integrated-parity contract tests gain it transitively (AC-6/AC-7).
- Precondition: fresh artifacts on disk (exit-0 check) so the assert passes immediately.
- Postcondition: harness greps show the call; `integrated_parity_support_planner_native_matches_wasm` green; a stale-artifact dry run (rename artifact temporarily) fails with a freshness message, not a geometry mismatch.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/integrated_parity_harness.rs` - full (small)
  - `crates/pnp-cli-locator/src/lib.rs` - ±40 lines around `staleness_reason`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/integrated_parity_harness.rs`
  - one integrated-parity contract test file IF it bypasses `run_integrated_parity` and needs a direct call
- Files explicitly out of bounds:
  - `xtask/src/build_guests.rs` (read-only mirror source), other parity suites' comparators
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `AGENTS.md` Guest WASM Staleness section - direct read (exit-code contract)
  - plan §7 E4 - delegated SUMMARY
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'assert_guest_freshness' crates/slicer-runtime/tests/common/integrated_parity_harness.rs && echo PASS || echo FAIL` - FACT PASS/FAIL (AC-6)
  - `cargo test -p slicer-runtime --test contract -- integrated_parity_support_planner_tdd::integrated_parity_support_planner_native_matches_wasm --exact 2>&1 | tee target/test-output.log` - FACT pass/fail (AC-7)
- Exit condition (falsifying): the assert cannot determine staleness without duplicating the whole fingerprint walk → simplify to mtime-based conservative check inside the test harness and say so in its doc comment; a full fingerprint reimplementation is out of budget.

### Step 7: view-seam identity test + paint BASE fallback fix (TASK-350)

- Task IDs: `TASK-350`
- Objective: (a) add `native_and_wasm_layer_views_are_field_identical` in the new file `crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs` (registered in `tests/contract/main.rs`): one representative layer view built via `build_native_layer_request` (`crates/slicer-wasm-host/src/marshal/native.rs:113`) vs the wasm `dispatch_layer_call` projection, compared field-by-field (T9 guard); (b) fix the `matching_base.is_empty()` fallback in `execute_paint_segmentation` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs:857`) to derive BASE polygons from the object's OWN contours, pinned by `paint_base_fallback_uses_own_object_contours` in the new standalone binary `paint_segmentation_base_fallback_tdd`.
- Precondition: Steps 1–6 committed (independent, but keeps gate runs meaningful); fresh guests.
- Postcondition: AC-8 and AC-9 pass; no existing executor/paint tests regress.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/native.rs` - lines 100–215
  - `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs` - ±30 lines around its `build_native_layer_request` call site - purpose: existing in-test usage of the native builder to mirror
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - ±40 around 857
- Files allowed to edit (at most 3 per sub-step; Step 7 runs as two sub-commits):
  - Sub-step 7a (wasm-host seam test): new test file `crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs` + its one-line `mod view_seam_identity_tdd;` registration in `crates/slicer-wasm-host/tests/contract/main.rs`. Binary: `--test contract`.
  - Sub-step 7b (paint fallback): `crates/slicer-core/src/algos/paint_segmentation/mod.rs` fix + NEW standalone test file `crates/slicer-core/tests/paint_segmentation_base_fallback_tdd.rs` opening with `#![cfg(feature = "host-algos")]` (precedent: `paint_segmentation_multi_object_isolation_tdd.rs`). slicer-core has NO test aggregator — files under `tests/` are auto-discovered as standalone binaries, so no Cargo.toml or main.rs registration is needed; binary: `--test paint_segmentation_base_fallback_tdd`, test name `paint_base_fallback_uses_own_object_contours`.
- Files explicitly out of bounds:
  - guest shims, WIT files, `marshal/in_.rs`, `crates/slicer-core/Cargo.toml`
- Expected sub-agent dispatches:
  - Question: how does the wasm leg construct its layer view at the `dispatch_layer_call` boundary (function + input types)? scope: `crates/slicer-wasm-host/src/`; return: LOCATIONS ≤20; purpose: field-comparison surface.
- Context cost: `M`
- Authoritative docs:
  - `AGENTS.md` coordinate hazard + plan E8 - direct read
  - plan §12 brief items "Native/wasm view seam" and "Paint fallback" - delegated SUMMARY
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-wasm-host --test contract -- view_seam_identity_tdd::native_and_wasm_layer_views_are_field_identical --exact 2>&1 | tee target/test-output.log` - FACT pass/fail (AC-8)
  - `cargo test -p slicer-core --features host-algos --test paint_segmentation_base_fallback_tdd -- paint_base_fallback_uses_own_object_contours --exact 2>&1 | tee target/test-output.log` - FACT pass/fail (AC-9; E6 flag mandatory)
- Exit condition (falsifying): the two legs are NOT field-identical for the representative input → that is a live T9 bug; capture the differing fields as FACT and report to orchestrator before fixing beyond this packet's scope.

### Step 8: deletions, remediation rows, ADR acceptance, re-measurement (TASK-351)

- Task IDs: `TASK-351`
- Objective: delete `docs/spec_packets/{215-raft-geometry,216-support-interface-layers,217-support-type-variants,218-support-gcode-e2e}/`; rewrite rows 3–6 of `docs/specs/support-generation-remediation-plan.md` (lines 61–64) to point each slug at its absorbing packet (215→240, 216→220-shipped+G-18-residue→238c, 217→220/224-nothing-remains, 218→242) in the same commit; flip ADR-0059 `Status: proposed` → `accepted` and append `## Amendments` recording Ruling 1; measure post-fix tree/traditional XY path-length + deposited material vs `tmp/SupportTest_{Tree,Normal}_Orca.gcode` (regenerate references if missing, T1) and append the dated `packet-236 re-measurement` note to `docs/specs/support-parity-gap-register.md`.
- Precondition: Steps 1–7 complete (measurements must reflect final behavior).
- Postcondition: AC-10, AC-11, AC-12 all PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/support-generation-remediation-plan.md` - lines 55–70
  - `docs/adr/0059-support-families-and-anchored-entities.md` - full (short)
  - predecessor packet dirs - via SUMMARY dispatch ONLY (never load their five files directly)
- Files allowed to edit (at most 3 per sub-commit; use two commits):
  - Commit A: the four deleted dirs (git rm) + `docs/specs/support-generation-remediation-plan.md` + `docs/adr/0059-*.md`
  - Commit B: `docs/specs/support-parity-gap-register.md` (measurement note)
- Files explicitly out of bounds:
  - `docs/spec_packets/224-*` (frozen by user ruling), any other packet dir, `docs/DEVIATION_LOG.md` except nothing this step
- Expected sub-agent dispatches:
  - Question: parse `tmp/p236-tree.gcode`, `tmp/p236-normal.gcode`, and the two Orca reference gcode files; return XY path length + deposited filament totals per file; scope: `tmp/*.gcode`; return: FACT 8 numbers; purpose: AC-12 measurement. (Slice commands themselves run foreground per AGENTS.md build/run commands.)
- Context cost: `M`
- Authoritative docs:
  - plan §10 supersession mapping - delegated SUMMARY
  - `docs/specs/support-parity-gap-register.md` header rules for measurement notes - direct range read
- OrcaSlicer refs: reference G-code files only (artifacts, not source reads).
- Verification:
  - `test ! -d docs/spec_packets/215-raft-geometry && test ! -d docs/spec_packets/216-support-interface-layers && test ! -d docs/spec_packets/217-support-type-variants && test ! -d docs/spec_packets/218-support-gcode-e2e && ! rg -q '\| generated \| docs/spec_packets/21[5678]-' docs/specs/support-generation-remediation-plan.md && echo PASS || echo FAIL` - FACT PASS/FAIL (AC-10)
  - `rg -q '^Status: accepted' docs/adr/0059-support-families-and-anchored-entities.md && rg -q '^## Amendments' docs/adr/0059-support-families-and-anchored-entities.md && echo PASS || echo FAIL` - FACT PASS/FAIL (AC-11)
  - `rg -q 'packet-236 re-measurement' docs/specs/support-parity-gap-register.md && echo PASS || echo FAIL` - FACT PASS/FAIL (AC-12)
- Exit condition (falsifying): Orca references missing from `tmp/` and the human cannot regenerate them before this step → park AC-12 as the only open item, record it verbatim in the Human Validation Gate evidence file, and proceed to Step 9 (AC-12 closes at gate time).

### Step 9: green gate, human-gate artifacts, docs/07 registration (TASK-352)

- Task IDs: `TASK-352`
- Objective: run the packet green gate in order (`cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask build-guests --check`; `cargo xtask test --summary --workspace -- --no-fail-fast`; `cargo xtask check-literals --report` count-unchanged check; `cargo xtask check-deviations`); produce Human Validation Gate artifacts (tree + traditional slices of `SupportTest.stl` with the matched profiles, visual-debug bundles, evidence file `evidence/human-gate.md` with checklist verdicts); register TASK-344..352 rows in `docs/07_implementation_status.md` via worker dispatch (packet-232 Step 7 precedent) in the section's local format `- [ ] TASK-NNN — <desc>. Spec: docs/spec_packets/236-support-stabilization/.`
- Precondition: Steps 1–8 exits all green (or AC-12 parked per its falsifying condition).
- Postcondition: gates PASS; evidence file exists with named checklist verdicts pending sign-off; `docs/07` rows present; packet ready for `status: implemented` ONLY after human sign-off.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - summary digest only (via `--summary-from` if re-digesting)
- Files allowed to edit (at most 3):
  - `docs/spec_packets/236-support-stabilization/evidence/human-gate.md`
  - `docs/07_implementation_status.md` (dispatch-performed row registration)
- Files explicitly out of bounds:
  - any source file (fix-forward loop returns to its owning step), `docs/specs/*` beyond already-edited rows
- Expected sub-agent dispatches:
  - Question: register nine TASK rows (text supplied verbatim from `task-map.md`) under the packets backlog section of `docs/07_implementation_status.md`; scope: that file; return: FACT pass/fail + appended lines; purpose: ledger registration without full-file load.
  - The workspace suite itself runs via `cargo xtask test --summary --workspace -- --no-fail-fast` (E5/T3; output digested, full log read from `target/test-output.log`).
- Context cost: `M`
- Authoritative docs:
  - `AGENTS.md` Test Discipline + plan §8 Human Validation Gate - direct read
- OrcaSlicer refs: none beyond Step 8 measurements.
- Verification:
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask test --summary --workspace -- --no-fail-fast` - FACT PASS/FAIL from digest (packet-level gate; never piped into an AC)
  - `rg -q 'TASK-344' docs/07_implementation_status.md && rg -q 'TASK-352' docs/07_implementation_status.md && echo PASS || echo FAIL` - FACT PASS/FAIL
- Exit condition (falsifying): any workspace-suite failure whose attribution has not been preceded by an exit-0 freshness check in the same session → run the check first (T4); a failure in a surface outside this packet's steps → record as pre-existing (T10 posture: name it, do not fix it here, report to orchestrator).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | producer mint-loop rewrite + executor test |
| Step 2 | S | two red/green fixtures, one file |
| Step 3 | M | validator exemptions + doc sentence |
| Step 4 | M | tripwire inputs, classification, rebless |
| Step 5 | S | manifest keys + regen + negative test |
| Step 6 | S | harness freshness assert |
| Step 7 | M | seam identity test + paint fallback |
| Step 8 | M | deletions/ADR/measurement (two commits) |
| Step 9 | M | gates + evidence + ledger dispatch |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate: M; no L steps.

## Packet Completion Gate

- All steps and exits complete (AC-12 may be parked only per Step 8's falsifying condition, closing at gate time).
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (Step 9).
- Reconcile superseded status transitions: drafts 215–218 deleted (not status-flipped); ADR-0059 accepted; gap-register G-21/G-22/G-24 destinations consumed by this packet, G-23 note recorded.
- `packet.spec.md` is ready for `status: implemented` only after the Human Validation Gate sign-off line carries date + verdict.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC (AC-1..AC-12, AC-N1, AC-N2) and the three packet-level gates.
- Record remaining packet-local risk (largest: G-21 orderability-recognition precision; mitigation is ADR-0059-clause-scoped recognition limited to family-scoped support claim holders + the two AC-N1 negative guards).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use `--all-targets` where the gate demands it; targeted test commands name explicit binaries and `--exact` tests verified to exist or authored by the cited step (invariant 16).
