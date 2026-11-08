# t114 post-resolution — adversarial audit of the e2e bucket's claims

The first t114 resolution claimed **"no test is safely deletable"** after a
source-level pruning pass. That claim was **too strong**: it checked whether a
test's *behaviour* was duplicated, not whether the test actually tested the
claim in its name, and not whether the same expensive observation was being
re-taken by several tests. Five parallel sub-agent audits (three reviewers, two
explorers) read all 28 e2e modules against `docs/22_test_quality.md` and the
narrower buckets. This file records what they found, what was adopted, what was
measured, and what is left.

## Verdict

- **Some tests were not testing what they claimed.** Two `scenario_2_*` tests
  asserted on locally-built vectors while the production path was never called;
  one test's name promised a G-code difference its body never compared; one
  fixture test asserted only that a file exists; one "real pipeline produces
  G-code" test passed on an empty file; one disk fixture test had **never run**
  (stale candidate roster, silent `SKIP`).
- **The wall is duplicate *work*, not duplicate *claims*.** The audits found
  **~11 minutes** of repeated observations (estimate: 423 s lightning + 120 s prep + ~95 s small-module merges + 12 s precision) (9 in `lightning_migration_tdd`, 11
  in `preparation_runtime_lifecycle_tdd`, 8 in the small slice modules). The
  remaining ~70 minutes is distinct claims, each paying the measured
  **10.0 s** fixed module load.
- **Deleting tests cannot fix the bucket; consolidating runs and caching
  compiled components can.** The cache remains [ticket 118](../../issues/118-compiled-component-cache.md);
  the lightning corpus consolidation is [ticket 119](../../issues/119-lightning-corpus-consolidation.md).

## Adopted in this take (all measured, all assertions preserved)

| # | Change | Claim effect | Wall effect |
| --- | --- | --- | --- |
| 1 | Deleted `cube_painted_e2e_tdd::painted_3mf_fixture_is_committed` | Removed a decorative `path.exists()` test; its sibling re-asserts the same path before use | 0 s (no slice) |
| 2 | Deleted `scenario_traces_tdd::scenario_2_higher_paint_order_wins_for_custom_overlap` and `::scenario_2_equal_paint_order_conflicting_values_are_fatal` | Both asserted on test-local vectors; `execute_region_mapping_with_cap` was never called | 0 s (no slice) |
| 3 | Moved the one real assertion from #2 to `slicer-ir/tests/ir_tests.rs::paint_semantic_derived_ordering_ranks_material_first` | Keeps the `PaintSemantic` D6 order claim in the crate that owns the type; production ordering/duplicate-key coverage stays in `slicer-core/tests/algo_region_mapping_tdd.rs` and `slicer-runtime/tests/integration/region_mapping_tdd.rs` | 0 s |
| 4 | Renamed `cube_painted_overrides_e2e_tdd::paint_config_override_visibly_differs_gcode` → `::paint_config_override_parses_cleanly_on_the_live_path` | The body only asserted both slices succeed, and the module doc says the G-code diff is intentionally unasserted; the name now matches the claim | 0 s |
| 5 | Repaired `threemf_fixture_e2e_tdd::support_blocker_emits_paint_regions_from_disk` | Added `bridge_support_enforcers.3mf` (3 `support_blocker` parts) to the candidate roster; the test had **always SKIPped** and now executes the disk-path segmentation it claims | 0 s (model load only) |
| 6 | Tightened `slice_end_to_end_tdd::slice_e2e_real_pipeline_produces_gcode` | Empty G-code now fails instead of skipping the first-line check | 0 s |
| 7 | Merged `modifier_infill_boundary_anchoring` into `modifier_infill_two_densities` | Same fixture, same CLI arguments, byte-identical slice; all boundary-anchoring assertions now run against the shared output | **38.76 s → 26.78 s (−11.98 s, −30.9 %)** |
| 8 | Merged `slicing_precision_integration_tdd::legacy_slice_structural_invariants` into `legacy_precision_run_is_deterministic` | INV-3 now runs on the first determinism output; one slice removed, every assertion preserved | 86.66 s → 86.71 s — **below noise; not claimed** |

## Audit findings not adopted (with reasons)

| Finding | Why not now |
| --- | --- |
| `lightning_migration_tdd`: **41 active invocations → 32** by sharing the corpus capture per `(case, arm)`; 9 strictly identical visual reruns, ~423 s estimated. Every assertion survives. | A 4,800-line restructuring into a shared-corpus orchestrator; needs its own session and a full-module before/after. Graduated as [ticket 119](../../issues/119-lightning-corpus-consolidation.md) with the per-test table. |
| `preparation_runtime_lifecycle_tdd`: **111 pipeline starts → 100**; `truthful_lifecycle_metadata` and `required_module_error_preserves_fields` dissolve after moving assertions; one capture is an exact duplicate. | The audit is precise, but the merges cross ownership/trace boundaries; ~120 s saved. Recorded for the next e2e take. |
| `slice_end_to_end_tdd`: **42 → 9** claim-oriented tests (Default/DefaultDet/Support/SupportDet/Combined/Cooling + two contract + one CLI config). | Wall-neutral (the runs are cached). Claim hygiene only; a separate cleanup. |
| `painted_fixture_parity` absorbs `cube_painted` + `cube_painted_overrides` (~55 s); `infill_overlap` merges into the config-block test (~24 s); `wedge_linked_infill_report` merges into the default wedge (~16 s). | Cross-file merges with distinct CLI seams; each needs its own assertion-transfer audit. |
| `preparation_visual_capture_tdd::preparation_only_uses_production_prefix` merges into `capture_is_semantically_neutral`. | **Falsified on inspection:** the neutrality test's captures are full slices; the preparation-only "no layer execution" boundary would be lost. Not merged. |
| `acceptance_gate_gaps_tdd::required_event_set_is_present_for_a_minimal_run` records its own events and asserts their presence. | Real event coverage exists in the lifecycle suite; the test is a unit-level `SliceEventCollector` contract misplaced in e2e. 0 wall; move in the cleanup take. |
| `typed_modifier_kind_tdd::all_modifier_kinds_keep_their_geometry_routes` greps `format!("{:#?}")` for variant names. | Cosmetic Debug-format coupling; the typed `assert_typed_modifier_resolution` covers the real resolution claim. 1.1 s. |
| `run_slice_api_tdd::run_slice_freezes_integrated_prepared_module_without_caller_injection` asserts non-empty output, not the frozen artifact identity. | Needs an artifact-identity comparison; not a deletion. |
| `run_slice_api_tdd::run_slice_against_wedge_returns_nonempty_gcode` greps `src/run.rs` for an absent symbol with no `// test-quality:` waiver. | Legitimate migration-absence guard; add the waiver in the cleanup take. |

## The arithmetic the user asked for

The e2e bucket performs **~237 pipeline starts** (66 full + 45 preparation in
`preparation_runtime_lifecycle`, 41 in `lightning_migration`, plus ~85 in the
rest) for 178 tests. At the measured **10.0 s** fixed module load, that is
**~39.5 minutes of pure module loading** before any slicing; the pipeline work
(~1.8 s per start) adds ~7 minutes, and the tapped captures add the rest. The
audit's ~11 minutes of duplicate work is **real but second-order** next to the
237 loads: even perfect deduplication leaves ~77 minutes. That is why the
compiled-component cache, not test deletion, is the carrying shape.

## Verification

The eight adopted changes were verified narrowly:

- `cargo test -p slicer-ir --test ir_tests paint_semantic_derived_ordering` — 1 passed.
- `cargo test -p slicer-runtime --test e2e -- --test-threads 4 scenario_traces_tdd cube_painted threemf_fixture_e2e_tdd::support_blocker slice_e2e_real_pipeline_produces_gcode` — 16 passed.
- `modifier_infill_tdd` — 1 passed, 26.78 s (was 38.76 s).
- `slicing_precision_integration_tdd` — 3 passed, 86.71 s (was 86.66 s; not claimed).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo xtask check-literals` — 0 violations; `check-test-quality --report` — 0 findings.
