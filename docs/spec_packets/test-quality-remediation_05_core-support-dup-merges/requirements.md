# Requirements: core-support-dup-merges

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE (support overhang)`; `core/DUP-CORE (support geometry)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md` §5.1 DUP-CORE support slices
- Packet status: `draft`
- Dependency: `core-region-mapping-dup-merges` (queue-generation order only; no exported symbols)
- Aggregate context cost: `S`

## Problem Statement

The support-overhang target repeats its coplanar-step input as a weaker non-empty check after a stronger test already pins the only emitted layer and contact area. Separately, `support_geometry.rs` repeats two integration tests inside a production file's `#[cfg(test)]` module. Keeping both copies creates maintenance drift without protecting another input, but deleting by name alone could silently lose the RC-1 coplanarity rationale, exact layer/area assertions, per-object schedule sets, emitted-entry witness, or the integration-only empty-plan case.

## In Scope

- Apply this exact overhang survivor map in `crates/slicer-core/tests/support_overhang_detection_tdd.rs`:
  - Pre-edit inventory, in file order (`19`): `sharp_tails_add_first_layer_contacts_when_enabled`, `sharp_tails_disabled_by_default_emits_none`, `enforce_support_layers_forces_full_contacts_in_leading_layers`, `enforce_support_layers_beyond_model_changes_nothing`, `bridge_areas_are_removed_from_contacts_under_bridge_no_support`, `bridge_removal_disabled_keeps_bridge_contacts`, `cantilever_pass_records_wide_overhang_annotations`, `overhang_is_detected_once_at_the_step_layer`, `coplanar_step_does_not_hide_the_contact`, `straight_column_produces_no_contacts`, `a_region_sitting_on_a_different_region_below_is_not_a_contact`, `shallower_threshold_yields_no_more_contact_area_than_a_plain_difference`, `zero_angle_uses_the_overlap_offset_not_a_plain_difference`, `threshold_angle_bump_catches_an_exactly_at_threshold_overhang`, `threshold_angle_is_clamped_to_eighty_nine_degrees`, `tiny_spots_are_filtered_out`, `xy_expansion_grows_the_finished_contact`, `blockers_are_subtracted_from_the_contact`, `degenerate_inputs_do_not_panic`.
  - Absorb `coplanar_step_does_not_hide_the_contact` into survivor `overhang_is_detected_once_at_the_step_layer`; preserve its `Regression pin for RC-1` coplanarity rationale in the survivor's comments.
  - Post-merge inventory, in file order (`18`): the same list with only `coplanar_step_does_not_hide_the_contact` removed.
- Preserve the exact overhang assertion relationship:
  - Both tests construct `let layers = pillar_then_cap();` and call `sweep(&layers, &params(45.0, 0.2))`.
  - The absorbed test asserts only `assert!(!sweep(&layers, &params(45.0, 0.2)).is_empty(), "a coplanar step must still register a support contact")`.
  - The survivor strictly subsumes that assertion by asserting the complete contact-layer vector equals `vec![3_usize]`, which proves non-emptiness, one contact layer, and its exact index; it additionally asserts `expected = 2.0 * 8.0 * 4.0` and `(got - expected).abs() < 0.1` for the expanded-back contact area.
- Consolidate the support-geometry twins into `crates/slicer-core/tests/algo_support_geometry_tdd.rs`:
  - Integration target pre- and post-edit inventory, in file order (`3 → 3`): `emits_for_2_layer_fixture`, `build_emit_schedule_two_objects_per_object_semantics`, `empty_plan_produces_empty_support`.
  - Inline `#[cfg(test)]` inventory in `crates/slicer-core/src/algos/support_geometry.rs`, in file order (`2 → 0`): `support_geometry_emits_for_2_layer_fixture`, `build_emit_schedule_two_objects_per_object_semantics`.
  - `support_geometry_emits_for_2_layer_fixture` and integration survivor `emits_for_2_layer_fixture` use identical two-layer fixtures and assertions: `result.is_ok()` followed by non-empty `SupportGeometryIR.entries`. Delete the inline twin and its duplicate helper fixture.
  - The inline and integration `build_emit_schedule_two_objects_per_object_semantics` tests use identical six-layer, two-object fixtures and exact assertions: `obj-A` equals `{1,3,5}` and `obj-B` equals `{0,1,2,3,4,5}`. Keep the integration survivor and retain the inline test's more diagnostic `got {a_sched:?}` / `got {b_sched:?}` messages.
  - Preserve `empty_plan_produces_empty_support` unchanged because no inline twin covers its default `LayerPlanIR` input or its successful-empty-`entries` assertion.
- Keep the production symbols under test unchanged: `slicer_core::algos::overhang_annotation::detect_support_contacts` (called by `sweep`), `slicer_core::algos::support_geometry::build_emit_schedule`, and `slicer_core::algos::support_geometry::execute_support_geometry`.
- Reconcile the source counts and feature-correct discovery counts. The durable census manifest lists both integration targets with `required: ["host-algos"]`; it records targets/features, not per-function counts.
- After both merges and their gates pass, update only the §7 `core` ledger row in `docs/specs/test-quality-remediation-plan.md`: preserve `partial` state, record this packet's absorbed/surviving tests and actual validations, and name `core-wall-sequence-dup-merges` as remaining work without editing the Packet Queue.

## Out of Scope

- Any production statement before `#[cfg(test)] mod tests` in `crates/slicer-core/src/algos/support_geometry.rs`, and all production code in `crates/slicer-core/src/algos/overhang_annotation.rs`.
- Every other `slicer-core` test file, especially sibling-packet homes `crates/slicer-core/tests/flow_tdd.rs`, `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`, and `crates/slicer-core/tests/algo_region_mapping_tdd.rs`.
- New tests, renamed survivors, changed tolerances, changed geometry, changed schedule behavior, new helpers, public APIs, IR/WIT/schema/config/manifest changes, and guest WASM artifacts.
- Existing docs except the implementation-time `docs/specs/test-quality-remediation-plan.md` §7 `core` ledger-row edit above; `docs/07_implementation_status.md`, sibling packet directories, the Packet Queue, every other ledger row, and all other documentation remain out of scope. The independent orchestrator owns post-preflight queue status.
- The crate-wide zero-findings wave exit; this packet proves zero findings only in its three touched Rust files.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` §§1–4, §5.1, §§6–7, and Packet Queue - survivor-union, census, feature, wave, and queue requirements.
- `docs/22_test_quality.md` §§1–5 - falsifiable-oracle and report-mode gate standard.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - named-regression-input and accounted-retirement standard.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - report-mode status and crate-wave zero-findings rule.
- `docs/02_ir_schemas.md` §IR 9a `SupportGeometryIR` - schedule identity and accumulated per-object height semantics.
- `docs/08_coordinate_system.md` §The Rule, §Conversion & Determinism, and §Conversion When Porting OrcaSlicer Code - millimeter/internal-unit and area-conversion authority.
- `docs/specs/test-quality-remediation-census.json` - both target entries require `host-algos`; no per-test-function counts are stored there.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — confirm canonical `detect_overhangs` retains the lower-layer growth, difference, and expand-back behavior already pinned by the surviving exact contact-area witness; no canonical code is ported or changed.

## Acceptance Summary

- Positive: `AC-1` reconciles and runs the complete 18-test overhang survivor inventory; `AC-2` pins the survivor's exact layer, area, fixture, and RC-1 rationale; `AC-3` reconciles integration `3 → 3` and inline `2 → 0`; `AC-4` pins emitted-entry, empty-plan, and exact schedule assertions in their named bodies; `AC-5` proves feature-correct target registration; `AC-6` proves touched-scope report-mode findings are zero; `AC-7` verifies the partial six-column §7 `core` ledger row.
- Negative: `AC-N1` fails on an unaccounted retirement, renamed/missing survivor, lost assertion fragment, or residual absorbed inline/overhang definition.
- Cross-packet impact: no symbol or file export. `core-wall-sequence-dup-merges` depends only on this packet's independently generated queue status.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 18; cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. 18 passed; 0 failed" target/test-output.log'` | Feature-correct overhang discovery and execution reconciliation. | FACT pass/fail; ≤20 failure lines. |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test algo_support_geometry_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -c ": test$" target/test-output.log || true)" -eq 3; cargo test -p slicer-core --features host-algos --test algo_support_geometry_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok\. 3 passed; 0 failed" target/test-output.log'` | Feature-correct support-geometry integration discovery and execution reconciliation. | FACT pass/fail; ≤20 failure lines. |
| `bash -lc 'set -euo pipefail; test "$(grep -Ec "^[[:space:]]*#\[test\]" crates/slicer-core/src/algos/support_geometry.rs || true)" -eq 0; ! rg -q "#\[cfg\(test\)\]|fn support_geometry_emits_for_2_layer_fixture|fn build_emit_schedule_two_objects_per_object_semantics" crates/slicer-core/src/algos/support_geometry.rs'` | Inline `2 → 0` disposition and no residual test module. | FACT pass/fail. |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/support_overhang_detection_tdd.rs crates/slicer-core/tests/algo_support_geometry_tdd.rs crates/slicer-core/src/algos/support_geometry.rs 2>&1 | tee target/test-output.log >/dev/null; rg -q -F "check-test-quality: 0 finding(s) in 0 file(s) [report mode]" target/test-output.log'` | Touched-scope false-green gate. | FACT pass/fail. |
| `cargo check --workspace --all-targets` | Compile every target after deleting the inline module. | FACT pass/fail. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Detect unused imports/helpers and lint regressions. | FACT pass/fail. |
| `cargo xtask check-literals` | Preserve watched fixture-literal discipline. | FACT pass/fail. |
| `bash -lc 'set -euo pipefail; python -c "from pathlib import Path; t=Path(\"docs/specs/test-quality-remediation-plan.md\").read_text(encoding=\"utf-8\"); s=t.split(\"## 7. Ledger\",1)[1].split(\"\\n## \",1)[0]; rows=[x for x in s.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; headers=[x for x in rows if x.split(\"|\")[1].strip()==\"Wave\"]; assert len(headers)==1 and [x.strip() for x in headers[0].split(\"|\")[1:-1]]==[\"Wave\",\"State\",\"Retired/changed symbols\",\"Surviving/new coverage\",\"Validation\",\"Remaining gap\"]; core=[x for x in rows if x.split(\"|\")[1].strip()==\"core\"]; assert len(core)==1; c=[x.strip() for x in core[0].split(\"|\")[1:-1]]; assert len(c)==6; wave,state,changed,coverage,validation,gap=c; assert wave==\"core\" and state==\"partial\"; assert all(x in changed for x in (\"core-support-dup-merges\",\"coplanar_step_does_not_hide_the_contact\",\"overhang_is_detected_once_at_the_step_layer\",\"support_geometry_emits_for_2_layer_fixture\",\"build_emit_schedule_two_objects_per_object_semantics\")); assert all(x in coverage for x in (\"overhang_is_detected_once_at_the_step_layer\",\"emits_for_2_layer_fixture\",\"build_emit_schedule_two_objects_per_object_semantics\",\"empty_plan_produces_empty_support\")); assert all(x in validation for x in (\"cargo test\",\"cargo check --workspace --all-targets\",\"cargo clippy --workspace --all-targets -- -D warnings\",\"cargo xtask check-literals\",\"check-test-quality\")); assert \"core-wall-sequence-dup-merges\" in gap; q=t.split(\"## Packet Queue\",1)[1]; qrows=[x for x in q.splitlines() if x.startswith(\"|\") and not x.startswith(\"|---\")]; assert [x.strip() for x in qrows[0].split(\"|\")[1:-1]]==[\"#\",\"packet slug\",\"goal (one sentence)\",\"task ids\",\"depends on\",\"status\",\"packet dir\"]; assert any(\"core-support-dup-merges\" in x for x in qrows); print(\"PASS: anchored core ledger row has six partial-state evidence cells; Packet Queue table intact\")"'` | Verify the sole §7 `core` row remains partial and records this slice without touching the queue. | FACT pass/fail. |

## Step Completion Expectations

- Step 2 starts only after Step 1 has reconciled `19 → 18` and proved the exact layer/area survivor.
- Neither step may compensate for a failed count or assertion check by changing production behavior, adding a replacement test, or editing an unrelated test.
- Full-crate `check-test-quality` zero findings remain a later core-wave closure expectation; packet completion requires only the touched-scope zero recorded above.
- The §7 ledger edit occurs only after both merge steps pass and may touch only the `core` row; Packet Queue state remains orchestrator-owned.

## Context Discipline Notes

- Read only the named windows of the 521-line overhang test and 352-line support-geometry source; inventories and survivor mappings above avoid exploratory whole-crate reads.
- OrcaSlicer confirmation is delegated and bounded; no canonical source enters the implementer's context.
- Cargo runs are delegated and always tee test output to `target/test-output.log`; inspect that log rather than rerunning.
