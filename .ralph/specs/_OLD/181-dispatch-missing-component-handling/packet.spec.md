---
status: implemented
packet: 181-dispatch-missing-component-handling
task_ids:
  - TASK-297
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 181-dispatch-missing-component-handling

## Goal

Make an absent compiled WASM component always fatal instead of silently successful: escalate all three `None`-returning branches of `compile_module_component` to fatal load errors, change all five `MissingComponent → Ok(success)` arms in `crates/slicer-wasm-host/src/dispatch.rs` to return the phase-appropriate fatal module error, and migrate every test that encoded the old graceful-skip behavior. Closes deviation missing-component dispatch defect and retires the placeholder-skip capability.

## Scope Boundaries

Covers the component-load path (`compile_module_component`), the five `MissingComponent` consumer arms in `dispatch.rs`, and the one behavioral blast radius those changes create — tests where an absent component reaches `WasmRuntimeDispatcher`. **The six `.unwrap_or_else(|| (WasmInstancePool::placeholder(), None))` fallbacks in `crates/slicer-runtime/src/` are deliberately left intact**: ~13 test files across the `unit` and `executor` buckets intentionally leave `wasm_handles` empty and supply the pool/component through a bespoke runner, a pattern ADR-0007 explicitly sanctions. Removing those fallbacks would break ~53 call sites for no missing-component dispatch defect benefit, since a production handle miss now hits the fatal dispatch arm anyway. Does not change WIT, IR, or the per-stage interface work owned by the ADR-0045 packet chain (162-165). No placeholder marker is introduced — the skip path is removed, not conditioned.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: draft packets `163_per-stage-wit-packages-pilot` (AC-12) and `164_per-stage-wit-packages-bulk`, which name missing-component dispatch defect as filed-but-out-of-scope. Verify each carve-out at reconciliation time rather than trusting this summary; as authored, the three carve-outs are: 164's `implementation-plan.md` dispatch-restructuring step ("leave the `MissingComponent` conversion arms (missing-component dispatch defect) byte-untouched"), 164's `design.md` §ADR-0015 note and its out-of-bounds list ("out of scope and must not be widened or narrowed here"; "do not touch"), and 163's `requirements.md` §Out of Scope, which excludes repairing `postpass_gcode_boundary_tdd.rs` "or the four `DispatchPhase::MissingComponent` conversion sites it rides on". This packet makes all three stale and they must be reconciled when 163/164 are refined. (163 also states the arm count as **four**; the tree has five.)
- Activation blockers: none. Packet `140_lightning-module-rewrite` is currently `active`; this packet stays `draft` until that clears.

## Acceptance Criteria

Every command uses a **substring** filter, never `--exact`: the `contract`/`integration`/`executor`/`unit`/`e2e` binaries are aggregated mod-lists, so libtest names are module-qualified and `--exact` on a bare fn name matches zero tests while exiting 0. Each command guards with `rg -v '\b0 passed'` — note the `\b`, without which the guard also swallows `10 passed`/`20 passed`.

- **AC-1. Given** a discovered module whose `.wasm` path exists but whose bytes fail `engine.compile_component`, **when** the loader runs, **then** it returns `Err(Box<LiveModuleLoadError>)` naming that module id, and no `LiveModuleBinding` for it is emitted with an absent component. (The error is boxed: `load_live_modules_for_plan(...) -> Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>>` — this is already the signature, so **no signature change is required**. All 12 loader call sites in this harness file use the non-`_with_config` variant; follow that convention. Other workspace callers — `tests/common/wasm_cache.rs`, `tests/e2e/slice_end_to_end_tdd.rs`, `tests/common/perimeter_harness.rs`, and the two production sites in `crates/slicer-runtime/src/run.rs` — need no compile fix; their exposure is behavioral only and they load real modules.) | `bash -c 'cargo test -p slicer-runtime --test integration -- live_module_loading_rejects_uncompilable_component 2>&1 | rg "^test result" | rg -v "\b0 passed" || echo "FAIL: 0 tests ran"'`
- **AC-2. Given** a discovered module whose companion `.wasm` is an 8-byte placeholder stub (the condition `is_placeholder_wasm` detects — `fs::metadata(wasm_path).len() <= 8`; there is no manifest key for this), **when** the loader runs, **then** it also returns `Err(Box<LiveModuleLoadError>)` naming that module id, rather than emitting a `Warning` and continuing with an absent component. | `bash -c 'cargo test -p slicer-runtime --test integration -- live_module_loading_rejects_placeholder_stub 2>&1 | rg "^test result" | rg -v "\b0 passed" || echo "FAIL: 0 tests ran"'`
- **AC-3. Given** `crates/slicer-runtime/tests/contract/postpass_gcode_boundary_tdd.rs`, **when** it runs, **then** its `make_module_with_config` helper no longer discards the compiled guest via a `_component`-prefixed parameter, the real component is threaded into the dispatched `CompiledModuleLive`, and the test asserts `gcode_ir.commands` was mutated by the guest. | `bash -c '! rg -q "\b_component\s*:" crates/slicer-runtime/tests/contract/postpass_gcode_boundary_tdd.rs && cargo test -p slicer-runtime --test contract -- postpass_gcode_boundary 2>&1 | rg "^test result" | rg -v "\b0 passed" || echo "FAIL: guest still discarded or 0 tests ran"'`

## Negative Test Cases

- **AC-N1. Given** a `CompiledModuleLive` presenting no compiled component, **when** each of the five dispatch entry points runs (`PrepassStageRunner::run_stage`, `LayerStageRunner::run_stage`, `FinalizationStageRunner::run_stage`, `PostpassStageRunner::run_gcode_postprocess`, `PostpassStageRunner::run_text_postprocess`), **then** each returns its phase-appropriate fatal module error naming the module id — `PrepassRunnerError`, `LayerStageError::FatalModule`, `FinalizationError`, and `PostpassError::FatalModule` respectively — and none returns `Ok(PrepassStageOutput::None)`, `Ok(None)`, `Ok(FinalizationOutput::Success)`, `Ok(PostpassOutput::GCodeSuccess)`, or `Ok(PostpassOutput::TextSuccess { .. })`. | `bash -c 'cargo test -p slicer-runtime --test contract -- missing_component_is_fatal_for_all_five_stages 2>&1 | rg "^test result" | rg -v "\b0 passed" || echo "FAIL: 0 tests ran"'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -c 'for b in unit contract executor integration e2e; do echo "== $b"; cargo test -p slicer-runtime --test $b 2>&1 | rg "^test result"; done; for t in visual_debug_postpass_tap_tdd arachne_wall_sequence_e2e_tdd; do echo "== $t"; cargo test -p slicer-runtime --test $t 2>&1 | rg "^test result"; done'` — the five buckets named by `CLAUDE.md` **plus** the two top-level un-bucketed binaries that construct a `CompiledModuleLive` or touch `wasm_handles`. No bucket filter reaches them and `cargo check` compiles them cleanly, so only an explicit run surfaces a regression.
- `bash -c 'rg -o "WasmInstancePool::placeholder\(\), *None" crates/slicer-runtime/src | wc -l'` — must print **6**. The fallbacks are intentionally preserved; reducing them is out of scope. Use `rg -o … | wc -l`, not `rg -c`: against a *directory* `-c` prints one `path:count` line per file (4 lines) and never the total.

## Authoritative Docs

- `docs/adr/0015-prepass-export-normalization.md` — direct read; the "MUST NOT catch and discard module fatals" clause this packet restores.
- `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` — ranged read of §Decision numbered item 1 (the sentence is at ~lines 58-61, **not** the section's lead paragraph, which is about `LayerStageCommitData`). It states `None` is the empty/`MissingComponent` case for `run_stage`. This packet **contradicts** that clause and therefore amends it — see Doc Impact.
- `docs/adr/0007-compiled-module-static-live-split.md` — ranged read of §"What future architecture reviews must not re-litigate". It introduced the `wasm_handles` pairing, `CompiledModuleLive`'s `Option<Arc<WasmComponent>>` field, and `WasmInstancePool::placeholder()`, and forbids consolidating that symbol away. This packet **conforms: it does not remove `WasmInstancePool::placeholder()`, and it does not touch any of the six production fallback sites in `crates/slicer-runtime/src/`.** (Steps 7-9 do swap placeholder pools for real ones at a few *test* call sites — which is what the ADR's own next sentence requires: "Tests that DO need real dispatch must use a real pool — not silently fall back through the placeholder.")
- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` — delegated SUMMARY; verified to contain zero occurrences of `wasm_component`, `CompiledModuleLive`, or `MissingComponent`, so it constrains nothing here.
- `docs/04_host_scheduler.md` — ranged read of `### Error Handling Policy` (~line 1226 of 1485) only; edited by this packet.
- `docs/DEVIATION_LOG.md` — the missing-component dispatch defect row only (large file; ranged read).

## Doc Impact Statement (Required)

- `docs/04_host_scheduler.md` §"Error Handling Policy" — record that an absent compiled component is **always** a fatal error, at load and at dispatch, and that the previous graceful-stage-skip behavior is retired. Verification grep: `rg -q 'absent compiled component' docs/04_host_scheduler.md`
- `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` — **append an `## Amendment — <date> (packet 181)` section that quotes the retired clause verbatim and states the replacement** (`None` is the empty-commit case only; a missing component is `Err(LayerStageError::FatalModule)`). Follow the `synthetic-diagram ADR amendment` precedent exactly: ADR-0037 carries an `## Amendment — 2026-07-15 (packet 161)` section that quotes the clause it retires, and the deviation row points at it. **Do not delete or silently rewrite §Decision item 1 in place** — the amendment is a decision record, and an in-place edit destroys the evidence a future reader needs.

  Consequence for verification: the clause text (`is the empty/`) **must still match after this packet**, because the Amendment section quotes it. A `! rg -q 'is the empty/'` negation gate would therefore be wrong — it would only pass if the implementer did the in-place deletion this packet forbids. Verification greps: `rg -q '^## Amendment' docs/adr/0020-layer-stage-commit-as-per-stage-enum.md && rg -q 'packet 181' docs/adr/0020-layer-stage-commit-as-per-stage-enum.md && rg -q 'is the empty/' docs/adr/0020-layer-stage-commit-as-per-stage-enum.md && echo PASS || echo FAIL`. (The `is the empty/` anchor is deliberately backtick-free: the clause reads ``` `None` is the empty/`MissingComponent` case ```, and a pattern carrying those backticks inside a double-quoted `bash -c` string would be parsed as command substitution.) All three currently: heading absent, `packet 181` absent, clause present — so the compound check fails today and is falsifiable.
- `docs/DEVIATION_LOG.md` — **file one tracking row for the deferred placeholder cleanup.** `is_placeholder_wasm` and `LoadedModule::placeholder_wasm` (`crates/slicer-scheduler/src/manifest.rs`), plus that file's affordance doc comment ("runtime dispatch will skip them with a diagnostic"), are inert and false after this packet but are not removed here, and **no successor packet exists in `.ralph/specs/`** to own them. Severity Low, status Open, owner `slicer-scheduler maintainers`. **Re-derive the id — do not quote one from this packet:** `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, then take the next free number, and write the resulting id into the verification grep at implementation time.
- `docs/DEVIATION_LOG.md` — the historical missing-component row and amendment label are retired by the post-purge ledger. The surviving evidence is the packet-181 amendment in ADR-0020 and the five-stage fatal dispatch test. Verification: `! rg -q '^\| missing-component dispatch defect |^\| missing-component ADR amendment ' docs/DEVIATION_LOG.md && rg -q 'packet 181' docs/adr/0020-layer-stage-commit-as-per-stage-enum.md && rg -q 'missing_component_is_fatal_for_all_five_stages' crates/slicer-runtime/tests/contract/dispatch_missing_component_tdd.rs`
- `docs/07_implementation_status.md` — hand-add the `TASK-297` backlog row (outside the generated block), then run `cargo xtask check-deviations` to regenerate the open-deviations block. Verification grep: `rg -q 'TASK-297' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
