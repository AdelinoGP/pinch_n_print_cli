# Implementation Plan: 181-dispatch-missing-component-handling

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Test-filter rule:** `unit`/`contract`/`executor`/`integration`/`e2e` are aggregated mod-lists, so libtest names are module-qualified. Use **substring** filters, never `--exact` on a bare fn name — `--exact` matches zero tests and exits 0, a silent false pass.
- **RED vs GREEN guard rule.** GREEN gates pipe through `rg "^test result" | rg -v '\b0 passed'`. The `\b` is required: without it the guard also swallows `10 passed`/`20 passed`. That guard is **wrong for a RED step** — a genuine failure prints `FAILED. 0 passed; 1 failed`, which it filters out, producing the same output as a zero-match filter. RED steps print the result line unfiltered and require an explicit failure count.
- **Migrate, never weaken.** Several steps turn pre-existing tests red *by design*. Update them to assert the new behavior; never relax an assertion to `is_ok()` to obtain a pass.
- **Never write that a module "declares" `placeholder_wasm`.** The loader branch is `if module.placeholder_wasm()` — a `LoadedModule` accessor over a field set at manifest-ingest by `is_placeholder_wasm`, which is `fs::metadata(wasm_path).len() <= 8`. There is no TOML key.
- **Do not touch the six `wasm_handles` fallbacks.** See `design.md` §Architecture Constraints — removing them is out of scope and would break an ADR-0007-sanctioned test pattern.

## Steps

### Step 1: Inventory the tests that dispatch without a component

- Task IDs: `TASK-297`
- Objective: Enumerate every test where an absent component reaches `WasmRuntimeDispatcher` — the only behavioral blast radius this packet creates. Confirm the counterpart class (callers passing an empty `wasm_handles`) is **not** affected, because the six fallbacks are deliberately left in place.
- Precondition: none.
- Postcondition: a written inventory. No code edited.
- Files allowed to read: delegated only — see dispatches. Do not open `dispatch.rs`.
- Files allowed to edit (at most 3): none (read-only discovery step).
- Files explicitly out of bounds: all production source.
- Blast-radius discipline: this IS the blast-radius step. Its output gates Steps 6-9. Scope it precisely: a test is in class only if an absent component reaches one of the five `WasmRuntimeDispatcher` runner methods. Tests that leave `wasm_handles` empty but supply a pool/component through a bespoke runner are **out** of class — that pattern is used by ~13 files across the `unit` and `executor` buckets and is untouched by this packet.
- Expected sub-agent dispatches:
  - Question: list every test-side `CompiledModuleLive::new` passing `None`, plus every use of `.no_wasm()` and `run_layer_and_commit`; scope: `crates/**/tests/**`; return: `LOCATIONS` (<=20 entries)
  - Question: for each site returned above, does it dispatch through `WasmRuntimeDispatcher`, or through a test-local runner impl?; scope: the files named by the first dispatch; return: `SUMMARY` (<=200 words)
- Context cost: `S`
- Authoritative docs: none required.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -ln "no_wasm\(\)|run_layer_and_commit" crates/slicer-runtime/tests'` — FACT: the shared-fixture consumers.
  - `bash -c 'rg -o "WasmInstancePool::placeholder\(\), *None" crates/slicer-runtime/src | wc -l'` — FACT: must print 6; this step must **not** reduce it. Use `rg -o … | wc -l`, not `rg -c`: against a directory `-c` prints one `path:count` line per file and never the total.
- Exit condition: the inventory names at minimum the `.no_wasm()` consumers (`contract/dispatch_protocol_tdd.rs`, `contract/infill_postprocess_contract_tdd.rs`), `run_layer_and_commit`'s one external caller (`executor/live_seam_path_tdd.rs`), and the inline-`None` sites in `unit/dag_validation_tdd.rs`, `executor/live_seam_path_tdd.rs`, `contract/postpass_gcode_empty_list_tdd.rs`, `contract/postpass_gcode_boundary_tdd.rs`. All other test-side `CompiledModuleLive::new` sites pass `Some(...)` and are out of class. One site the `CompiledModuleLive::new` query will **not** surface and which is nonetheless out of class: `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` builds a `LiveModuleBinding { wasm_component: None }` by hand, but only for plan construction — it never dispatches and never reaches `compile_module_component`. Recorded here so a future reader does not re-litigate it.

### Step 2: RED — loader must reject both an uncompilable component and an ≤8-byte stub

- Task IDs: `TASK-297`
- Objective: Author `live_module_loading_rejects_uncompilable_component` (AC-1) and `live_module_loading_rejects_placeholder_stub` (AC-2) in the existing loader test file. Each builds a temp module dir — one with `.wasm` bytes that are not a valid component, one with an ≤8-byte stub — and asserts the loader returns `Err(Box<LiveModuleLoadError>)` naming the module id. Both must FAIL today. Follow the file's convention: all 12 of its loader call sites use `load_live_modules_for_plan` (the non-`_with_config` variant).
- Precondition: `compile_module_component` still emits `Warning` + `None` on all three branches.
- Postcondition: both tests exist and fail for the right reason.
- Files allowed to read: `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` (long; treat any line count as a stale hint) — ranged reads around the existing module-discovery setup only; `crates/slicer-scheduler/src/manifest.rs` — `is_placeholder_wasm` only, for the exact ≤8-byte threshold.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`
- Files explicitly out of bounds: `crates/slicer-wasm-host/src/execution_plan_live.rs`; `crates/slicer-wasm-host/src/dispatch.rs`.
- Blast-radius discipline: not applicable — adds no struct field or constant.
- Expected sub-agent dispatches:
  - Question: run both new tests and report each `test result` line plus the failing assertion; scope: `cargo test -p slicer-runtime --test integration -- live_module_loading_rejects`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs: none required.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test integration -- live_module_loading_rejects --nocapture 2>&1 | rg "^test result|panicked at"'` — FACT: the result line must contain **`2 failed`**. `0 passed; 0 failed` means the filter matched nothing and is a failure of this step. Do not apply the `\b0 passed` guard here.
- Exit condition: both tests fail because the load succeeded with an absent component — not a fixture or compile error.

### Step 3: Escalate all three loader branches to fatal, and migrate the three loader tests it invalidates

- Task IDs: `TASK-297`
- Objective: In `compile_module_component`, change **all three** `None`-returning branches — `module.placeholder_wasm()`, `std::fs::read` failure, `engine.compile_component` failure — from `DiagnosticLevel::Warning` + `None` to a fatal load error naming the module id and underlying cause. Migrate the three pre-existing tests that encode the old contract.
- Precondition: Step 2's tests exist and are RED.
- Postcondition: the loader never yields an absent component; AC-1 and AC-2 pass; the `integration` binary is green.
- Files allowed to read: `crates/slicer-wasm-host/src/execution_plan_live.rs` — `compile_module_component` and `load_live_modules_for_plan_with_config` only; `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` (long; treat any line count as a stale hint) — ranged reads around the three named tests; `docs/adr/0015-prepass-export-normalization.md`.
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/execution_plan_live.rs`
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`
- Files explicitly out of bounds: `crates/slicer-wasm-host/src/dispatch.rs`; Step 2's two new tests.
- Blast-radius discipline: the behavioral fallout is named, not discovered — migrate `non_component_bytes_are_skipped_with_compile_failure_diagnostic` and `mixed_valid_and_invalid_binaries_load_deterministically_side_by_side` (both `.unwrap()` the load and assert `wasm_component.is_none()` + a `Warning`), and `placeholder_wasm_is_skipped_with_structured_warning_diagnostic`. Migrate the assertions; do not delete them. Both loader entry points already return `Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>>`, so **no signature change is required** and the other workspace call sites (`tests/common/wasm_cache.rs`, `tests/e2e/slice_end_to_end_tdd.rs`, `tests/common/perimeter_harness.rs`, and the two production sites in `crates/slicer-runtime/src/run.rs`) need no compile fix — their exposure is behavioral only, and they load real modules. If `LiveModuleLoadError` gains a variant, add every exhaustive `match` on it to this step's edit list first; the sole exhaustive match is the in-file `Display` impl.
- Expected sub-agent dispatches:
  - Question: quote the three `None`-returning branches with their diagnostic construction; scope: `crates/slicer-wasm-host/src/execution_plan_live.rs`; return: `SNIPPETS` (<=3 x 30 lines)
- Context cost: `M`
- Authoritative docs: `docs/adr/0015-prepass-export-normalization.md` — direct read.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test integration -- live_module_loading_rejects 2>&1 | rg "^test result" | rg -v "\b0 passed" || echo "FAIL: 0 tests ran"'` — FACT pass/fail (AC-1, AC-2).
  - `bash -c 'for b in integration e2e; do echo "== $b"; cargo test -p slicer-runtime --test $b 2>&1 | rg "^test result"; done'` — FACT pass/fail; `e2e` included because it loads real modules through the changed path.
- Exit condition: AC-1 and AC-2 pass; `integration` and `e2e` are green with all three migrated tests asserting the new behavior.

### Step 4: RED — author the five-stage fatal contract test

- Task IDs: `TASK-297`
- Objective: Author `missing_component_is_fatal_for_all_five_stages` (AC-N1) in a new contract test file and **register it** with `mod dispatch_missing_component_tdd;` in `crates/slicer-runtime/tests/contract/main.rs` (an explicit mod-list; an unregistered file silently reports "0 tests run"). It must be RED: the arms still launder.
- Precondition: Step 3 complete.
- Postcondition: the test exists, is registered, and fails for the right reason.
- Files allowed to read: `crates/slicer-runtime/tests/common/dispatch_fixture.rs` — for constructing all five stage inputs; `crates/slicer-runtime/tests/contract/main.rs` — the mod-list; `crates/slicer-wasm-host/src/dispatch.rs` (long; never in full) — only the five runner-trait signatures, via `rg 'fn run_stage|fn run_gcode_postprocess|fn run_text_postprocess'`.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/dispatch_missing_component_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs` (registration)
- Files explicitly out of bounds: `crates/slicer-wasm-host/src/dispatch.rs` (no production edit here); every other test file.
- Blast-radius discipline: not applicable — adds no struct field or constant.
- Expected sub-agent dispatches:
  - Question: report the five runner-trait method signatures and the input types each needs; scope: `crates/slicer-wasm-host/src/dispatch.rs`; return: `LOCATIONS` (<=20 entries)
- Context cost: `M`
- Authoritative docs: none required.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -q "mod dispatch_missing_component_tdd;" crates/slicer-runtime/tests/contract/main.rs && echo PASS || echo "FAIL: unregistered"'` — FACT PASS/FAIL.
  - `bash -c 'cargo test -p slicer-runtime --test contract -- missing_component_is_fatal --nocapture 2>&1 | rg "^test result|panicked at"'` — FACT: must contain **`1 failed`**. `0 passed; 0 failed` means the filter matched nothing (likely a missing `mod`) and is a failure of this step.
- Exit condition: the test is registered, compiles, and shows `1 failed` because the arms still launder.

### Step 5: GREEN — make all five dispatch arms fatal

- Task IDs: `TASK-297`
- Objective: Change all five `Err(e) if e.phase == DispatchPhase::MissingComponent` arms in `crates/slicer-wasm-host/src/dispatch.rs` to return the phase-appropriate fatal module error (`PrepassRunnerError`, `LayerStageError::FatalModule`, `FinalizationError`, `PostpassError::FatalModule`) naming the module id.
- Precondition: Step 4 complete; AC-N1 is RED.
- Postcondition: AC-N1 passes. Tests in `contract`, `unit`, and `executor` may now be red; Steps 6-9 migrate them.
- Files allowed to read: `crates/slicer-wasm-host/src/dispatch.rs` (long; never in full) — ±40-line windows via `rg 'DispatchPhase::MissingComponent'`.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/dispatch.rs`
- Files explicitly out of bounds: `crates/slicer-runtime/tests/contract/dispatch_missing_component_tdd.rs` (frozen after Step 4); every other test file.
- Blast-radius discipline: no signature change. The behavioral fallout is owned by Steps 6-9; this step is expected to leave suites red, and those steps must not be skipped on the strength of a green AC filter. The five arms change together — a partial fix leaves one stage laundering, which AC-N1 asserts against.
- Expected sub-agent dispatches:
  - Question: report each of the five `MissingComponent` consumer arms with its enclosing fn and returned variant; scope: `crates/slicer-wasm-host/src/dispatch.rs`; return: `LOCATIONS` (<=20 entries)
- Context cost: `M`
- Authoritative docs: `docs/adr/0015-prepass-export-normalization.md` — the norm the fatal arms restore.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test contract -- missing_component_is_fatal 2>&1 | rg "^test result" | rg -v "\b0 passed" || echo "FAIL: 0 tests ran"'` — FACT pass/fail (AC-N1).
  - `bash -c 'rg -c "DispatchPhase::MissingComponent" crates/slicer-wasm-host/src/dispatch.rs'` — FACT: still resolves all five arms plus five producers (no arm silently deleted).
  - `bash -c 'for b in unit contract executor integration e2e; do echo "== $b"; cargo test -p slicer-runtime --test $b 2>&1 | rg "^test result"; done'` — FACT: records the red set for Steps 6-9.
- Exit condition: AC-N1 passes, all five arms are fatal, and the red set is recorded.

### Step 6: Migrate the `.no_wasm()` fixture and its consumers

- Task IDs: `TASK-297`
- Objective: `.no_wasm()` / `make_no_wasm_bundle` (`tests/common/dispatch_fixture.rs`) exists to dispatch without a component, which is now always fatal. Either give the fixture a real compiled component or repurpose it to assert the fatal, and update its two consumers.
- Precondition: Step 5 complete; these tests are red.
- Postcondition: the `contract` tests using `.no_wasm()` are green with no assertion weakened.
- Files allowed to read: those three files — the failing assertions only, via `rg`; `tests/common/wasm_cache.rs` — `compiled_guest` only.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/dispatch_fixture.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs`
  - `crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs`
- Files explicitly out of bounds: production source; `tests/common/mod.rs` (Step 7).
- Blast-radius discipline: `dispatch_fixture.rs` is shared; confirm from Step 1's inventory that `.no_wasm()` has exactly these two consumers before changing its semantics.
- Expected sub-agent dispatches:
  - Question: show `.no_wasm()` / `make_no_wasm_bundle` and both consumers' assertions; scope: those three files; return: `SNIPPETS` (<=3 x 30 lines)
- Context cost: `S`
- Authoritative docs: none required.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test contract 2>&1 | rg "^test result"'` — FACT pass/fail.
- Exit condition: the named tests assert a specific outcome — never a relaxed `is_ok()`.

### Step 7: Migrate the `run_layer_and_commit` helper and its caller

- Task IDs: `TASK-297`
- Objective: `run_layer_and_commit` (`tests/common/mod.rs`) constructs with `None` and `?`-propagates through `LayerStageRunner::run_stage`. Give it a real component or make its contract explicit, and update its one external caller, `tests/executor/live_seam_path_tdd.rs` — which additionally builds its own inline `CompiledModuleLive::new(..., None, ...)` and `.expect("PathOptimization dispatch must succeed")`.
- Precondition: Step 6 complete.
- Postcondition: the `executor` bucket is green.
- Files allowed to read: those two files — the construction sites and failing assertions only, via `rg`.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/mod.rs`
  - `crates/slicer-runtime/tests/executor/live_seam_path_tdd.rs`
- Files explicitly out of bounds: production source; `tests/common/dispatch_fixture.rs` (frozen after Step 6).
- Blast-radius discipline: `run_layer_and_commit` is shared but has exactly one external caller per Step 1's inventory; re-confirm before changing its signature. Do **not** touch the six `wasm_handles` fallbacks — the many `executor`-bucket tests that leave `wasm_handles` empty rely on them and are out of this packet's scope.
- Expected sub-agent dispatches:
  - Question: show `run_layer_and_commit` and the inline construction plus `.expect` in `live_seam_path_tdd.rs`; scope: those two files; return: `SNIPPETS` (<=3 x 30 lines)
- Context cost: `S`
- Authoritative docs: none required.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test executor 2>&1 | rg "^test result"'` — FACT pass/fail.
- Exit condition: the `executor` bucket is green with assertions migrated, not weakened.

### Step 8: Migrate the remaining inline-`None` sites

- Task IDs: `TASK-297`
- Objective: Migrate the two remaining inline `CompiledModuleLive::new(..., None, ...)` sites — `tests/unit/dag_validation_tdd.rs` (which discards its dispatch result with `let _ =`, so it may not be red; verify before editing) and `tests/contract/postpass_gcode_empty_list_tdd.rs` (which asserts `Ok(PostpassOutput::GCodeSuccess)`).
- Precondition: Step 7 complete.
- Postcondition: the `unit` bucket is green, and every `contract` test **except** `postpass_gcode_boundary_tdd` passes. That one is deliberately still red here — it is Step 9's subject, so the `contract` bucket only goes fully green after Step 9.
- Files allowed to read: those two files — the constructions and assertions only, via `rg 'CompiledModuleLive::new'`.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs`
  - `crates/slicer-runtime/tests/contract/postpass_gcode_empty_list_tdd.rs`
- Files explicitly out of bounds: production source; files frozen by Steps 6 and 7.
- Blast-radius discipline: `dag_validation_tdd.rs` may already pass because it discards the result — do not edit a passing test.
- Expected sub-agent dispatches:
  - Question: after Step 7, which tests in the `unit` and `contract` buckets fail, and on which assertion?; scope: `cargo test -p slicer-runtime --test unit`, `--test contract`; return: `SUMMARY` (<=200 words)
- Context cost: `S`
- Authoritative docs: none required.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test unit 2>&1 | rg "^test result"'` — FACT pass/fail; must be green.
  - `bash -c 'cargo test -p slicer-runtime --test contract 2>&1 | rg "^test result|^failures:" -A3'` — FACT: the only remaining failure may be `postpass_gcode_boundary_tdd`, which Step 9 owns. Any other contract failure is a defect in this step.
- Exit condition: the `unit` bucket is green; the only red contract test is `postpass_gcode_boundary_tdd`; any test left untouched was verified already-passing.

### Step 9: De-vacuum the postpass boundary contract test

- Task IDs: `TASK-297`
- Objective: Rewrite `postpass_gcode_boundary_carries_all_payload_variants_into_guest` and its `make_module_with_config` helper so the compiled `postpass-guest` is threaded into the dispatched `CompiledModuleLive` instead of being discarded via the `_component: Arc<WasmComponent>` parameter, and assert `gcode_ir.commands` was actually mutated by the guest.
- Precondition: Step 8 complete — the suite is green.
- Postcondition: the test executes real WASM and would fail if the guest were removed; AC-3 passes.
- Files allowed to read: `crates/slicer-runtime/tests/contract/postpass_gcode_boundary_tdd.rs` — full; `tests/common/wasm_cache.rs` — `compiled_guest` only.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/contract/postpass_gcode_boundary_tdd.rs`
- Files explicitly out of bounds: all production source.
- Blast-radius discipline: not applicable — test-only change.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check` and report only STALE/clean; scope: repo root; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs: none required.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c '! rg -q "\b_component\s*:" crates/slicer-runtime/tests/contract/postpass_gcode_boundary_tdd.rs && cargo test -p slicer-runtime --test contract -- postpass_gcode_boundary 2>&1 | rg "^test result" | rg -v "\b0 passed" || echo "FAIL: guest still discarded or 0 tests ran"'` — FACT pass/fail (AC-3).
- Exit condition: AC-3 passes, no `_component`-prefixed parameter remains, and the assertion observes a real mutation.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Read-only; two delegated inventories, scoped to dispatch-reaching sites only. |
| Step 2 | S | One test file; two RED loader tests with temp module dirs. |
| Step 3 | M | Loader branch changes plus migration of three pre-existing loader tests. |
| Step 4 | M | RED contract test spanning five stage inputs + aggregator registration. |
| Step 5 | M | The five arms in `dispatch.rs` (long; ranged windows only). |
| Step 6 | S | Shared `.no_wasm()` fixture + its two consumers. |
| Step 7 | S | Shared `run_layer_and_commit` + its one caller. |
| Step 8 | S | Two remaining inline-`None` sites. |
| Step 9 | S | One test file; guest-freshness check first. |

Aggregate: `M`. Three `M` steps (3, 4, 5), the rest `S`. No `L` step.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS with a non-zero test count.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- `cargo xtask build-guests --check` reports clean (AC-3 depends on real guest execution).
- **All five buckets green, plus the two un-bucketed binaries that construct a `CompiledModuleLive` or touch `wasm_handles`:** `bash -c 'for b in unit contract executor integration e2e; do echo "== $b"; cargo test -p slicer-runtime --test $b 2>&1 | rg "^test result"; done; for t in visual_debug_postpass_tap_tdd arachne_wall_sequence_e2e_tdd; do echo "== $t"; cargo test -p slicer-runtime --test $t 2>&1 | rg "^test result"; done'`. The un-bucketed binaries are reached by no bucket filter and `cargo check` compiles them cleanly, so only an explicit run surfaces a regression.
- `bash -c 'rg -o "WasmInstancePool::placeholder\(\), *None" crates/slicer-runtime/src | wc -l'` prints **6** — the fallbacks must be intact; reducing them is out of scope and would break the ADR-0007-sanctioned empty-`wasm_handles` test pattern. (`rg -c` against a directory prints per-file lines, not a total.)
- Edit `docs/04_host_scheduler.md` §"Error Handling Policy": an absent compiled component is always fatal, at load and at dispatch; the placeholder-skip affordance is retired. Verify `rg -q 'absent compiled component' docs/04_host_scheduler.md`.
- Amend `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` §Decision item 1 to drop `MissingComponent` from the meaning of `None`, **and file `D-181-ADR-0020-AMENDED`** in `docs/DEVIATION_LOG.md` quoting the contested clause (convention: `D-161-ADR-0037-AMENDED`, `D-283-ADR-0046-AMENDED`). Verify `rg -q 'D-181-ADR-0020-AMENDED' docs/DEVIATION_LOG.md`.
- Flip the DEV-087 row to `Closed — <date> (packet 181)`, correct its stale "four ... arms" count to five, and record that the row's untraced reachability question is answered — reachable via the `fs::read` and `compile_component` branches — selecting the row's own option (B).
- Hand-add the `TASK-297` row to `docs/07_implementation_status.md` (outside the generated block), then run `cargo xtask check-deviations` to regenerate the open-deviations block, which must never be hand-edited.
- Reconcile draft packets `163`/`164`, whose plans say to leave the `MissingComponent` arms "byte-untouched" — stale once this lands. Record the note; do not edit their files. **Also rebut their stated justification**, which is the one live objection to this packet: they assert the arms are "load-bearing for host-builtin stages that legitimately have no component". That is false — host builtins run through `run_builtin_stage` under `host:*` stage ids and never construct a `CompiledModuleLive`, and `build_live_execution_plan` inserts only discovered `LiveModuleBinding`s. Record the rebuttal with those two symbol references so the objection is not silently re-litigated.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: an absent compiled component is now always fatal, so any environment shipping a module whose `.wasm` is unreadable, uncompilable, or an ≤8-byte stub fails slicing loudly instead of emitting a silently degraded slice — and the placeholder scaffolding affordance documented in `manifest.rs` is retired. Confirm the error names the module id and the underlying cause.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

The workspace-wide `cargo check` and `cargo clippy` gate commands must use `--all-targets`. This does **not** apply to the narrow `cargo test -p <crate> --test <binary>` verification commands above — `--all-targets` is not a valid combination with `--test`.
