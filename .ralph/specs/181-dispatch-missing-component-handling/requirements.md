# Requirements: 181-dispatch-missing-component-handling

## Packet Metadata

- Grouped task IDs: `TASK-297`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Deviation **DEV-087** (Open, 2026-07-17). `crates/slicer-wasm-host/src/dispatch.rs` contains five match arms of the shape `Err(e) if e.phase == DispatchPhase::MissingComponent => Ok(<success>)`, one per runner trait method:

| Enclosing fn | Laundered into | Variant's crate |
| --- | --- | --- |
| `WasmRuntimeDispatcher as PrepassStageRunner::run_stage` | `Ok(PrepassStageOutput::None)` | `slicer-core` |
| `WasmRuntimeDispatcher as LayerStageRunner::run_stage` | `Ok(None)` | — |
| `WasmRuntimeDispatcher as FinalizationStageRunner::run_stage` | `Ok(FinalizationOutput::Success)` | `slicer-ir` |
| `WasmRuntimeDispatcher as PostpassStageRunner::run_gcode_postprocess` | `Ok(PostpassOutput::GCodeSuccess)` | `slicer-ir` |
| `WasmRuntimeDispatcher as PostpassStageRunner::run_text_postprocess` | `Ok(PostpassOutput::TextSuccess { text })` | `slicer-ir` |

All five producers are the private `dispatch_*` helpers, and all trigger on one condition: the `Option<&Arc<WasmComponent>>` argument is `None`, reason `"no compiled WASM component available"`.

**The deviation's own open question is now answered, and the answer selects its own option (B).** The row states *"Whether a real module can reach `None` in production has **not** been traced"* and offers "(A) prove `None` is unreachable … then narrow the laundering to an explicit placeholder marker; (B) if reachable, make it fatal." It is reachable. `compile_module_component` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) returns `None` in three cases, **each emitting only a `DiagnosticLevel::Warning`**:

1. `module.placeholder_wasm()` — the `LoadedModule` accessor over a field populated at manifest-ingest time by `is_placeholder_wasm`, which is `fs::metadata(wasm_path).len() <= 8`. The scaffolding case.
2. `std::fs::read(module.wasm_path())` fails — a **real** module with an unreadable `.wasm`.
3. `engine.compile_component(&bytes)` fails — a **real** module whose bytes are not a valid component.

Cases 2 and 3 apply to a genuinely discovered module sitting in the execution plan; its stage is silently laundered into success, producing an empty stage and a wrong slice with no diagnostic. That violates ADR-0015's "The macro/host glue MUST NOT catch and discard module fatals".

**Two ledger corrections this packet carries.** (a) The DEV-087 row says "**four** … arms"; the tree has **five** — it omits the `LayerStageRunner::run_stage` arm. (b) The row's Status still describes the reachability question as untraced; it is traced, and this packet records the answer.

**Why the placeholder path is removed rather than preserved.** `placeholder_wasm` is not a manifest declaration — there is no TOML key anywhere in `modules/` or `crates/`. It is inferred by `is_placeholder_wasm`, which is `fs::metadata(wasm_path).len() <= 8`. `manifest.rs` documents the resulting affordance ("Modules with placeholder binaries are discoverable for manifest validation and plan construction, but runtime dispatch will skip them with a diagnostic"), but no module in the tree is currently a placeholder — the smallest real core-module `.wasm` is 68,495 bytes. A silent stage-skip triggered by a file-size heuristic is a worse failure mode than a loud error, and anyone scaffolding a module can build a stub that compiles. The capability is therefore retired rather than conditioned behind a marker, which also removes the need to propagate any placeholder fact from the loader to the six production construction sites.

A second, independent path to an absent component exists at **six** sites across four executor modules in `crates/slicer-runtime/src/` — `prepass.rs` (1), `layer_executor.rs` (2), `postpass.rs` (2), `layer_finalization.rs` (1) — where a `wasm_handles` lookup miss falls back to `.unwrap_or_else(|| (WasmInstancePool::placeholder(), None))`. (`run.rs` only *builds* the side-table.) **These are left intact** — see §Out of Scope for the measurement that settled it. Once the five dispatch arms are fatal, a genuine production handle miss is caught at dispatch, so the fallback is no longer a silent-success path and removing it would only break the ADR-0007-sanctioned test pattern.

The laundering also sustains a **vacuous contract test**: `postpass_gcode_boundary_carries_all_payload_variants_into_guest` (`crates/slicer-runtime/tests/contract/postpass_gcode_boundary_tdd.rs`) compiles a real guest, then discards it via a `_component: Arc<WasmComponent>` parameter on its `make_module_with_config` helper, dispatches with an absent component, and asserts `Ok(GCodeSuccess)` plus `commands == expected`. Both hold trivially. The test passes with the guest deleted, despite its name. The DEV-087 row notes the `_component` underscore "was a compiler unused-variable warning that was silenced rather than heeded; that warning was the bug report."

## In Scope

- Escalate **all three** `None`-returning branches of `compile_module_component` to fatal load errors naming the module id — including the `is_placeholder_wasm` branch, whose current diagnostic already (incorrectly) claims "dispatch of this module will fail fatally".
- Change all five `MissingComponent` arms in `dispatch.rs` to return the phase-appropriate fatal module error (`PrepassRunnerError`, `LayerStageError::FatalModule`, `FinalizationError`, `PostpassError::FatalModule`).
- Add contract coverage asserting the fatal at all five stages, in a new `crates/slicer-runtime/tests/contract/dispatch_missing_component_tdd.rs` **registered via `mod dispatch_missing_component_tdd;` in `crates/slicer-runtime/tests/contract/main.rs`** — that binary is an explicit mod-list, and an unregistered file silently reports "0 tests run".
- **Migrate the one behavioral blast radius: tests where an absent component reaches `WasmRuntimeDispatcher`.** `dispatch_fixture.rs`'s `.no_wasm()` builder (consumers: `tests/contract/dispatch_protocol_tdd.rs`, `tests/contract/infill_postprocess_contract_tdd.rs`); the `run_layer_and_commit` helper in `tests/common/mod.rs` (one external caller: `tests/executor/live_seam_path_tdd.rs`); and the inline `CompiledModuleLive::new(..., None, ...)` sites in `tests/unit/dag_validation_tdd.rs`, `tests/executor/live_seam_path_tdd.rs`, `tests/contract/postpass_gcode_empty_list_tdd.rs`, and `tests/contract/postpass_gcode_boundary_tdd.rs`. This class is measured complete — every other test-side `CompiledModuleLive::new` passes `Some(...)`.
- De-vacuum `postpass_gcode_boundary_carries_all_payload_variants_into_guest`.
- Update `docs/04_host_scheduler.md`, amend ADR-0020 with a `D-181-ADR-0020-AMENDED` deviation row, flip/correct the DEV-087 row, and register `TASK-297`.

## Out of Scope

- Any WIT or IR change. The five stage success variants keep their shapes; only *when* they are produced changes.
- Introducing a manifest key for placeholder modules. The capability is retired here; re-introducing it as a declared opt-in would be its own packet with a manifest-schema change.
- The per-stage versioned-interface split owned by draft packets `162`-`165`. This packet removes their DEV-087 carve-out without touching their surface.
- DEV-026 and DEV-085, which the same draft packets also name as out-of-scope. Each has its own queue entry.
- Changing `is_placeholder_wasm` itself, or the `placeholder_wasm` manifest field. They become inert for dispatch purposes; removing them is cleanup for a later packet.
- **Removing the six `.unwrap_or_else(|| (WasmInstancePool::placeholder(), None))` fallbacks in `crates/slicer-runtime/src/{prepass,layer_executor,postpass,layer_finalization}.rs`.** An earlier draft of this packet did remove them, on the theory that a `wasm_handles` lookup miss should be an internal error. Measurement refuted it: **18 test files** pass an empty or partial `wasm_handles` alongside a module-bearing stage list, and 13 of them do so *deliberately*, injecting the real pool/component through a bespoke runner — a pattern ADR-0007 sanctions ("`WasmInstancePool::placeholder()` … exists as the explicit fallback for in-process test pipelines that don't need real dispatch"). Removing the fallbacks would have forced ~53 call-site migrations across the `unit` and `executor` buckets for **no DEV-087 benefit**: the deviation is about dispatch laundering, and once the five arms are fatal, a genuine production handle miss is caught there anyway. The fallbacks stay; the completion gate asserts the count is still 6.

## Authoritative Docs

- `docs/adr/0015-prepass-export-normalization.md` — direct read; the "MUST NOT catch and discard module fatals" clause.
- `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` — ranged read of §Decision **numbered item 1** (~lines 58-61; the section's lead paragraph is about `LayerStageCommitData` and is not the relevant text). Contradicted and amended by this packet.
- `docs/adr/0007-compiled-module-static-live-split.md` — ranged read of §"What future architecture reviews must not re-litigate". Introduced the `wasm_handles` pairing and `WasmInstancePool::placeholder()`, and forbids consolidating that symbol away. This packet **conforms: it does not remove `WasmInstancePool::placeholder()`, and it does not touch any of the six production fallback sites in `crates/slicer-runtime/src/`** (see §Out of Scope). Steps 7-9 do swap placeholder pools for real ones at a few *test* call sites, which the ADR's own next sentence requires.
- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` — delegated SUMMARY; zero occurrences of `wasm_component`/`CompiledModuleLive`/`MissingComponent`, so it constrains nothing.
- `docs/04_host_scheduler.md` — ranged read of `### Error Handling Policy` (~line 1226 of 1485); edited.
- `docs/DEVIATION_LOG.md` — the DEV-087 row only; large file, never read in full.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (loader fatal on an uncompilable component), `AC-2` (loader fatal on an 8-byte placeholder stub), `AC-3` (boundary test de-vacuumed and guest-executing).
- Negative: `AC-N1` (absent component is fatal at all five dispatch entry points, never `Ok`).
- Cross-packet impact: unblocks the DEV-087 carve-out in draft packets 163 and 164; no code overlap with them.

## Verification Commands

**Copy commands from `packet.spec.md`, not from this table** — markdown escaping of `|` here is not valid shell, and transcribing an escaped pipe into a ripgrep pattern silently turns an alternation into a literal match.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `--test integration -- live_module_loading_rejects_uncompilable_component` with the `\b0 passed` guard | AC-1 loader escalation | FACT pass/fail |
| `--test integration -- live_module_loading_rejects_placeholder_stub` with the guard | AC-2 placeholder stub is fatal | FACT pass/fail |
| `! rg -q "\b_component\s*:" …postpass_gcode_boundary_tdd.rs` then `--test contract -- postpass_gcode_boundary` with the guard | AC-3 de-vacuumed test | FACT pass/fail |
| `--test contract -- missing_component_is_fatal_for_all_five_stages` with the guard | AC-N1 fatal at all five stages | FACT pass/fail |
| `rg -q "mod dispatch_missing_component_tdd;" crates/slicer-runtime/tests/contract/main.rs` | New test file is registered (guards the "0 tests run" false pass) | FACT PASS/FAIL |
| `for b in unit contract executor integration e2e; do cargo test -p slicer-runtime --test $b; done` | All five buckets green | FACT pass/fail per bucket |
| `cargo test -p slicer-runtime --test visual_debug_postpass_tap_tdd` and `--test arachne_wall_sequence_e2e_tdd` | The two **un-bucketed** top-level binaries that construct a `CompiledModuleLive` or touch `wasm_handles`; no bucket filter reaches them and `cargo check` compiles them cleanly | FACT pass/fail |
| `rg -o "WasmInstancePool::placeholder\(\), *None" crates/slicer-runtime/src \| wc -l` | Must print **6** — the fallbacks are intentionally preserved. `rg -c` against a directory prints per-file lines, not a total | FACT count |
| `cargo xtask build-guests --check` | Guest freshness before attributing any AC-3 failure | FACT clean/STALE |
| `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` | Compilation + lint gates | FACT pass/fail |

## Step Completion Expectations

**Never use `--exact` with a bare fn name** against an aggregated binary; and guard with `rg -v '\b0 passed'` — the `\b` matters, because without it the guard also swallows `10 passed`/`20 passed`. That guard is for **GREEN** gates only: a genuine RED prints `FAILED. 0 passed; 1 failed`, which the guard filters out, making a real failure indistinguishable from a zero-match filter. RED steps print the result line unfiltered and require `1 failed`.

The five dispatch arms must change together: leaving any one laundering unconditionally reintroduces the defect on that stage, and AC-N1 asserts all five in one test. **Migrate, never weaken** — several steps turn pre-existing tests red by design; update them to assert the new behavior rather than relaxing an assertion to `is_ok()`.

## Context Discipline Notes

`crates/slicer-wasm-host/src/dispatch.rs` is long (~2.7k lines) — locate each of the five arms by `rg 'DispatchPhase::MissingComponent'` and read only ±40-line windows; never read it in full. `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` is long — ranged reads only. `docs/04_host_scheduler.md` is long — locate the `### Error Handling Policy` heading by `rg` and read only that section. Treat every line count here as a stale hint, never an identifier; locate by symbol or heading. `docs/DEVIATION_LOG.md` is large; read only the DEV-087 row.
