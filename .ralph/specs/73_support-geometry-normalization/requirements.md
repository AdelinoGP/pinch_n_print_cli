# Requirements: 73_support-geometry-normalization

## Packet Metadata

- Grouped task IDs:
  - `TASK-166` — resolve per-object/per-region config and thread it through so downstream stages consume config values. This packet continues that intent at the **WIT-export layer** for `run-support-geometry` (TASK-166's own work was at the `RegionMapIR`/`RegionPlan` layer).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`run-support-geometry` is the one prepass export that never received config and whose errors vanish — a latent bug, not a design choice. The macro down-converter (`crates/slicer-macros/src/lib.rs`, support arm ≈1825–1907) injects an **empty** `ConfigView` because the WIT export has no `config-view` parameter (it states this verbatim at ≈1831–1833), so the `support-planner` reads `support_enabled`, `support_raft_layers`, `tree_support_branch_diameter`, and every other key as a default — the tree-support planner has never honored user support config, and `support_enabled = false` cannot disable it. The same export returns a bare `support-geometry-output` record instead of `result<_, module-error>`, so the module's `Err(ModuleError::fatal(…))` (e.g. on an empty layer-plan, `support-planner/src/lib.rs:198`) is explicitly discarded (`let _ = out;`, ≈1900–1903). Every sibling prepass stage gets both `config-view` and a `result` channel; this one is an incomplete normalization. The SDK trait and module body are already in the correct shape, so the fix is confined to the WIT export and its host/macro glue — but it deliberately changes slicer behavior, which is why fixture re-baselining and a regression test are part of the slice.

## In Scope

- Normalize `run-support-geometry` in `crates/slicer-schema/wit/world-prepass.wit`: add `config: config-view`; convert `support-geometry-output` from a returned record to a `resource { push-support-plan-entry: func(entry: support-plan-entry) -> result<_, string> }`; return `result<_, module-error>` (the shared `slicer:common` one from packet 72).
- Rewrite the macro support arm: pass the real WIT `config-view`, drain the SDK `SupportGeometryOutput` builder into the WIT resource, propagate via `__slicer_error_out`; delete the empty-`ConfigView` injection, the `let _ = out;` swallow, and the `SupportGeometryOutput { … }` return shim (incl. the `fn run_support_geometry(...) -> SupportGeometryOutput` glue signature at ≈1962–1968).
- Rework the host dispatch arm (`dispatch.rs` ≈975–1018): push a `support-geometry-output` resource + the `config_handle`; consume `result<_, ModuleError>`; reshape `harvest_support_plan_ir` (≈1848) to read the drained resource; remove `push_support_geometry_result` (`wit_host.rs` ≈1944).
- Add `crates/slicer-runtime/tests/support_geometry_config_normalization_tdd.rs` with the three named tests behind AC-2, AC-N1, AC-N2.
- Re-baseline `prepass_support_geometry_tdd`, `blackboard_support_geometry_slot_tdd`, and benchy fixtures **only if** their output changes (default-config slices should be unchanged; any change is config taking effect and must be inspected, not blindly accepted).
- Doc edits per `packet.spec.md` §Doc Impact Statement.

## Out of Scope

- Any change to the `support-planner` algorithm, the SDK `PrepassModule` trait, or `SupportGeometryOutput`/`SupportPlanEntry` shapes.
- The contract relocation / dedup / shared `module-error` mechanics (packet 72).
- Other prepass exports (mesh-analysis, layer-planning, seam-planning) — already normalized.
- TASK-166's `RegionMapIR`/`RegionPlan` config resolution — already done; this packet only fixes the WIT-export boundary.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — prepass world contract. > 300 lines: delegate a SUMMARY of the `run-support-geometry` export.
- `docs/02_ir_schemas.md` — `SupportPlanIR` / `support-plan-entry` exact field names. Delegate; load only that section.
- `docs/04_host_scheduler.md` — prepass dispatch and the module-error → `DispatchError` mapping. Delegate; needed for AC-N2 only.

## Acceptance Summary

- Positive cases: `AC-1` (WIT shape), `AC-2` (raft config honored end-to-end), `AC-3` (macro swallow/injection removed), `AC-4` (host record-stash removed + config handle passed), `AC-5` (SDK/module unchanged), `AC-6` (enabled path still emits), `AC-7` (guest freshness) — all in `packet.spec.md`. Refinement: AC-2 asserts negative `global_layer_index` values `-1` and `-2` specifically; AC-4 asserts the dispatch arm calls `call_run_support_geometry` with a config handle.
- Negative cases: `AC-N1` (`support_enabled = false` → zero `SupportPlanIR` entries), `AC-N2` (empty layer-plan fatal → `DispatchError`).
- Cross-packet impact: depends on packet 72; nothing downstream.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -c 'rg -U -q "run-support-geometry: func\([^)]*config: config-view[^)]*\) -> result<_, module-error>" crates/slicer-schema/wit/world-prepass.wit && rg -q "resource support-geometry-output" crates/slicer-schema/wit/world-prepass.wit; echo EXIT=$?'` | AC-1 WIT shape | FACT `EXIT=0` |
| `cargo test -p slicer-runtime --test support_geometry_config_normalization_tdd` | AC-2 + AC-N1 + AC-N2 | FACT pass/fail; SNIPPETS ≤20 lines on fail |
| `bash -c '! rg -q "run-support-geometry has no config-view parameter" crates/slicer-macros/src/lib.rs && ! rg -q "Ignore error from run_support_geometry" crates/slicer-macros/src/lib.rs; echo EXIT=$?'` | AC-3 macro cleaned | FACT `EXIT=0` |
| `bash -c '! rg -q "push_support_geometry_result" crates/slicer-runtime/src/wit_host.rs crates/slicer-runtime/src/dispatch.rs; echo EXIT=$?'` | AC-4 host record-stash gone | FACT `EXIT=0` |
| `bash -c 'rg -q "output: &mut SupportGeometryOutput" crates/slicer-sdk/src/traits.rs && rg -q "fn run_support_geometry" modules/core-modules/support-planner/src/lib.rs; echo EXIT=$?'` | AC-5 SDK/module unchanged | FACT `EXIT=0` |
| `cargo test -p slicer-runtime --test prepass_support_geometry_tdd` | AC-6 enabled path | FACT pass/fail |
| `cargo test -p slicer-runtime --test blackboard_support_geometry_slot_tdd` | regression (commit slot) | FACT pass/fail |
| `cargo test -p slicer-runtime --test benchy_end_to_end_tdd` | regression (e2e gcode) | FACT pass/fail |
| `cargo test -p support-planner` | regression (module unit tests unchanged) | FACT pass/fail |
| `cargo xtask build-guests --check` | AC-7 guest freshness | FACT clean / `STALE:` list |
| `cargo check --workspace` | gate | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | gate | FACT pass/fail |

No command above is `cargo test --workspace`.

## Step Completion Expectations

- Ordering: the WIT edit (Step 1) precedes the macro (Step 2) and host (Step 3) glue, which must agree on the new signature; the regression test (Step 4) follows all three and requires a guest rebuild.
- Cross-step invariant: after Step 3, a guest rebuild is mandatory before any `support_geometry_config_normalization_tdd` / `prepass_support_geometry_tdd` run — these exercise the real support-planner guest, and a stale guest will fail against the new host signature for reasons unrelated to the test logic.
- Fixture re-baselining is decided by **inspecting** any diff in default-config output, never by regenerating golden files blindly (default config should be unchanged; a change means config now flows and must be understood).

## Context Discipline Notes

- `crates/slicer-macros/src/lib.rs` (~3000 lines) — range-read only the support arm (≈1825–1916) and the glue impl signature (≈1955–1970). Do not load whole.
- `crates/slicer-runtime/src/{wit_host.rs,dispatch.rs}` (~6000 / ~2000 lines) — range-read only the support-geometry dispatch arm (≈975–1018), `harvest_support_plan_ir` (≈1848), and `push_support_geometry_result` (≈1944). Do not load whole.
- `modules/core-modules/support-planner/src/lib.rs` (~1270 lines) — read only `run_support_geometry` (≈184–254) to confirm it is unchanged; the algorithm body is out of bounds for editing.
- Heaviest dispatch hint: integration-test runs return FACT pass/fail or SNIPPETS ≤20 lines on failure — never the full log.
