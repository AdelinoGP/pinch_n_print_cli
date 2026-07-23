---
status: implemented
packet: 177-arachne-baselines-to-structural-invariants
task_ids: []
backlog_source: docs/DEVIATION_LOG.md D-112-SELFCAPTURED-BASELINES
context_cost_estimate: M
---

# Packet Contract: 177-arachne-baselines-to-structural-invariants

## Goal

Replace the perimeter suite's self-captured JSON oracles with source-geometry structural
invariants. The correctness gate is a measured Arachne-versus-classic coverage
floor over reproducible Arachne perimeter inputs, with the D5 bow defect kept as
a synthetic discriminator so a 0.668 coverage ratio cannot pass.

## Scope Boundaries

Keep the canonical even `max_bead_count` correction, but do not claim that an
odd value triggers a giant-centre-bead branch: `LimitedBeadingStrategy.cpp::compute`
does not contain that branch. Delete the eight
`crates/slicer-core/tests/fixtures/arachne/*.json` snapshots and every
`expected_perimeter_ir.json` under
`crates/slicer-runtime/tests/fixtures/perimeter_parity/`; no snapshot or
provenance value remains in the active perimeter test path. Rebuild the fixture
consumers around source geometry and structural assertions. Add a standalone runtime test binary
for paired classic/Arachne coverage measurements, sharing its capture harness
with the existing perimeter integration tests. Rehome the nine red core tests
without changing their bodies and correct the stale runtime header.

No production Arachne geometry behavior changes beyond the `max_bead_count`
default correction. No generated artifact is re-recorded.

## Prerequisites and Activation Gate

- ADR-0042 is accepted: `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md`.
- The module manifests are the per-module files such as
  `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`; dispatch
  of `wall_generator` is in `crates/slicer-scheduler/src/execution_plan.rs`.
- The packet remains `draft` until the measurement table is populated from the
  paired source-geometry harness and the preflight checks pass.
- If the repeatability delta exceeds `0.02`, or the resulting threshold admits
  `0.668`, stop and leave the packet `draft`; do not tune the threshold.

## Acceptance Criteria

Every `cargo test` command below uses the feature required by its crate. The
commands write combined test output to `target/test-output.log` and require an
explicit test-count result; an exit code alone is not evidence.

- **AC-1. Given** the production defaults `ArachneParams::default()` and `BeadingFactoryParams::default()`, **when** the correction lands, **then** both literal values are `10` and both are even. | `mkdir -p target && cargo test -p slicer-core --features host-algos --test arachne_invariants -- production_defaults_max_bead_count_is_even --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' && echo PASS || { echo FAIL; false; }`
- **AC-2. Given** the inventory command `rg -n 'max_bead_count:\s*9' crates/slicer-core/`, **when** the correction lands, **then** it returns no matches and the affected test binaries retain their test counts. | `if rg -n 'max_bead_count:\s*9' crates/slicer-core/ >/dev/null 2>&1; then echo ODD-REMAINS; false; else echo CLEAN; fi`
- **AC-3. Given** the five Arachne perimeter source fixtures, **when** the paired harness runs classic and Arachne at identical aligned Z planes, **then** `design.md` records five numeric ratios, the observed minimum, the repeatability-derived margin, and the pinned coverage threshold (pinned because the whole-model X-extent metric alone is too coarse to encode the D5 discriminator at 0.99 — see design.md "Measured Coverage Baseline" for the pinning justification); the D5 row is explicitly sanity-only and excluded from the minimum. | `fail=0; for s in tapered_wedge narrow_strip_widening max_bead_count_cap complex_multi_feature cube_4color_arachne; do rg -q "^\| \`$s\` \| [0-9]+\.[0-9]+ \| [0-9]+\.[0-9]+ \| [0-9]+\.[0-9]+ \| [0-9]+\.[0-9]+ \| [0-9]+\.[0-9]+ \|" .ralph/specs/177-arachne-baselines-to-structural-invariants/design.md || fail=1; done; rg -q '\*\*Chosen threshold \(pinned\):\*\*' .ralph/specs/177-arachne-baselines-to-structural-invariants/design.md || fail=1; rg -q 'repeatability' .ralph/specs/177-arachne-baselines-to-structural-invariants/design.md || fail=1; test $fail -eq 0 && echo FILLED || { echo MEASUREMENT-EMPTY; false; }`
- **AC-4. Given** the committed threshold, **when** the runtime structural tests evaluate synthetic ratios, **then** they reject `0.668`, admit `0.990`, and use the named threshold constant from the measurement table. | `fail=0; mkdir -p target; cargo test -p slicer-runtime --test arachne_structural_invariants -- coverage_threshold_rejects_d5_broken_ratio --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' || fail=1; cargo test -p slicer-runtime --test arachne_structural_invariants -- coverage_threshold_accepts_d5_fixed_ratio --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' || fail=1; test $fail -eq 0 && echo PASS || { echo FAIL; false; }`
- **AC-5. Given** the five source fixtures, **when** the paired coverage invariant runs, **then** every ratio is at or above the threshold and a failure names the fixture, aligned Z, classic X extent, Arachne X extent, and percentage ratio. | `mkdir -p target && cargo test -p slicer-runtime --test arachne_structural_invariants -- arachne_coverage_floor_over_source_corpus --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' && echo PASS || { echo FAIL; false; }`
- **AC-6. Given** the tapered-wedge STL, **when** its structural parity test runs, **then** `tapered_wedge_parity_is_structural` compares paired classic/Arachne output rather than an absolute-coordinate snapshot. | `mkdir -p target && cargo test -p slicer-runtime --test arachne_structural_invariants -- tapered_wedge_parity_is_structural --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' && echo PASS || { echo FAIL; false; }`
- **AC-7. Given** the in-memory source cases, **when** the core fixture consumers run, **then** named structural assertions exist and pass independently: `centrality_flags_are_structurally_consistent`, `transitions_present_where_bead_count_changes`, and `bead_count_sequence_is_monotonic_within_transition_bounds`. | `fail=0; mkdir -p target; cargo test -p slicer-core --features host-algos --test centrality -- centrality_flags_are_structurally_consistent --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' || fail=1; cargo test -p slicer-core --features host-algos --test propagation -- transitions_present_where_bead_count_changes --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' || fail=1; cargo test -p slicer-core --features host-algos --test bead_count -- bead_count_sequence_is_monotonic_within_transition_bounds --exact 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' || fail=1; test $fail -eq 0 && echo ALL-PRESENT || { echo MISSING-FAIL; false; }`
- **AC-8. Given** the eight core snapshots and every perimeter expected IR snapshot, **when** conversion lands, **then** none of those JSON files exists and no active test source loads them. | `test -z "$(rg --files -g '*.json' crates/slicer-core/tests/fixtures/arachne 2>/dev/null)" && test -z "$(rg --files -g 'expected_perimeter_ir.json' crates/slicer-runtime/tests/fixtures/perimeter_parity 2>/dev/null)" && test -z "$(rg -l 'fixtures/arachne/.*\.json|expected_perimeter_ir\.json' crates/slicer-core/tests crates/slicer-runtime/tests 2>/dev/null)" && echo SNAPSHOTS-REMOVED || { echo SNAPSHOTS-REMAIN; false; }`
- **AC-9. Given** the nine `arachne_parity_red_*.rs` tests, **when** rehoming completes, **then** no old path remains, every name in the pre-move set is still present in the post-move set (no test was lost in the renames), and any names present post-move but absent pre-move are exactly the AC-1 (`production_defaults_max_bead_count_is_even`) and AC-N2 (`bead_width_invariant_rejects_oversized_bead`) fixup additions captured after Step 6. | `if ls crates/slicer-core/tests/arachne_parity_red_*.rs >/dev/null 2>&1; then echo NOT-MOVED-FAIL; false; elif test -s /tmp/pnp-177-pre-test-names.txt && cargo test -p slicer-core --features host-algos -- --list 2>/dev/null | rg ': test$' | sort > /tmp/pnp-177-post-test-names.txt && (comm -23 /tmp/pnp-177-pre-test-names.txt /tmp/pnp-177-post-test-names.txt | rg -q . && echo TESTS-LOST-FAIL; false) || (comm -13 /tmp/pnp-177-pre-test-names.txt /tmp/pnp-177-post-test-names.txt | sort > /tmp/pnp-177-ac9-added.txt && (diff -q <(printf '%s\n' 'bead_width_invariant_rejects_oversized_bead: test' 'production_defaults_max_bead_count_is_even: test') /tmp/pnp-177-ac9-added.txt && echo NAMES-PRESERVED) || { echo UNEXPECTED-ADDITIONS-FAIL; false; }); then true; fi`
- **AC-10. Given** the runtime Arachne parity header, **when** hygiene lands, **then** it no longer says `fails on purpose` and identifies D-104f as the sole intentionally open red case. | `! rg -q 'fails on purpose' crates/slicer-runtime/tests/arachne_parity.rs && rg -q 'D-104f' crates/slicer-runtime/tests/arachne_parity.rs && echo HEADER-OK || { echo HEADER-STALE; false; }`

## Negative Test Cases

- **AC-N1. Given** synthetic coverage `0.668`, **when** the coverage predicate evaluates it, **then** the test passes only by observing a rejection whose diagnostic names the ratio and threshold. | `mkdir -p target && cargo test -p slicer-runtime --test arachne_structural_invariants -- coverage_invariant_rejects_synthetic_d5_regression --exact --nocapture 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' && echo PASS || { echo FAIL; false; }`
- **AC-N2. Given** a synthetic spacing-domain bead greater than `2 * optimal_spacing_mm`, **when** the bead-width predicate evaluates it, **then** the test passes only by observing a rejection whose diagnostic names the offending spacing and cap. | `mkdir -p target && cargo test -p slicer-core --features host-algos --test arachne_invariants -- bead_width_invariant_rejects_oversized_bead --exact --nocapture 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed' && echo PASS || { echo FAIL; false; }`
- **AC-N3. Given** the no-recapture rule, **when** the tree is inspected, **then** no Arachne perimeter recorder or expected-IR loader remains in the perimeter test path. The forbidden symbols are the recorder functions deleted in Step 5 (`record_tapered_wedge`, `record_perimeter`, `record_wedge`, `record_narrow_strip`, `record_max_bead`, `record_complex_multi`, `record_cube_4color`, `record_holed_square`, `record_bridge`, `record_overhang`, `record_multi_tool`, `record_spiral_vase`), the loader (`load_expected_perimeters`), and the fixture filename (`expected_perimeter_ir`). The `PipelineInstrumentation::record_edges` method on the unrelated instrumentation trait is out of scope. | `test -f crates/slicer-runtime/tests/arachne_structural_invariants.rs && ! rg -q 'record_tapered_wedge|record_perimeter|record_wedge|record_narrow_strip|record_max_bead|record_complex_multi|record_cube_4color|record_holed_square|record_bridge|record_overhang|record_multi_tool|record_spiral_vase|load_expected_perimeters|expected_perimeter_ir' crates/slicer-runtime/tests/integration crates/slicer-runtime/tests/arachne_structural_invariants.rs && echo NO-RECAPTURE || { echo RECAPTURE-PATH; false; }`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test arachne_invariants`
- `cargo test -p slicer-runtime --test arachne_structural_invariants`
- `cargo test -p slicer-runtime --test integration perimeter_parity`

## Authoritative Docs and Domain Terms

- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` — structural classes, coverage floor, and spacing-domain bead cap.
- `docs/specs/arachne-parity-recovery.md` — corrected Track B history and no-recapture rule.
- `docs/DEVIATION_LOG.md` — `D-112-SELFCAPTURED-BASELINES` and D-104f rows only.
- `docs/08_coordinate_system.md` — mm/unit boundaries.
- `CONTEXT.md` — `Coverage subject`, `Repeatability margin`, `Self-captured baseline`, and `Structural invariant`.

## Doc Impact Statement

- `docs/DEVIATION_LOG.md`, `D-112-SELFCAPTURED-BASELINES` row — close only after every AC is green; verification: `rg -q 'D-112-SELFCAPTURED-BASELINES.*Closed' docs/DEVIATION_LOG.md`.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md`, Consequences — record the measured threshold, repeatability margin, five coverage subjects, and deletion of self-captured JSON oracles; verification: `rg -q 'measured coverage threshold' docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md`.
- `docs/specs/arachne-parity-recovery.md`, Track B — correct the stale RED/rebaseline description and record the structural replacement; verification: `rg -q 'tapered-wedge.*structural|structural.*tapered-wedge' docs/specs/arachne-parity-recovery.md`.
- `CONTEXT.md`, glossary terms — record the resolved `Coverage subject` and `Repeatability margin` vocabulary; verification: `rg -q '^### Coverage subject$' CONTEXT.md && rg -q '^### Repeatability margin$' CONTEXT.md`.

<!-- snippet: context-discipline -->
Workers must use the exact file surfaces in `design.md` and `implementation-plan.md`, never load generated code, lockfiles, target artifacts, or the deleted JSON snapshots, and must return bounded evidence. Cargo output is always tee'd to `target/test-output.log`; broad workspace tests are reserved for the acceptance ceremony.
