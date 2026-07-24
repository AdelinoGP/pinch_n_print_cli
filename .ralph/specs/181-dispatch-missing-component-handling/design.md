# Design: 181-dispatch-missing-component-handling

## Controlling Code Paths

- Primary code path: `compile_module_component` (`crates/slicer-wasm-host/src/execution_plan_live.rs`, returns `Option<Arc<WasmComponent>>`) produces the component; it threads through `LiveModuleBinding::wasm_component` and `run.rs`'s `wasm_handles` side-table into `CompiledModuleLive::new` (`crates/slicer-wasm-host/src/binding.rs`, 5 params) at six sites in the four executor modules **`crates/slicer-runtime/src/{prepass,layer_executor,postpass,layer_finalization}.rs`** — these are in `slicer-runtime`, **not** `slicer-wasm-host`, which contains no `CompiledModuleLive::new` call outside `binding.rs`'s own definition. The five private `dispatch_*` helpers convert an absent component into `DispatchPhase::MissingComponent`, and the five runner-trait arms in `crates/slicer-wasm-host/src/dispatch.rs` (long; ranged reads only) launder that into `Ok(success)`.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/contract/main.rs` (an explicit `mod` list — count it at edit time, do not trust a pinned number) and `tests/integration/main.rs`; shared fixtures `tests/common/dispatch_fixture.rs` (`.no_wasm()` builder, `make_no_wasm_bundle`) and `tests/common/mod.rs` (`run_layer_and_commit`); guest cache `tests/common/wasm_cache.rs` (`compiled_guest`); loader tests `tests/integration/live_module_loading_tdd.rs` (long; ranged reads only). `crates/slicer-wasm-host/tests/contract/` also exists (18 files; 2 construct `CompiledModuleLive`, both passing `Some(...)`, so both are unaffected).
- OrcaSlicer comparison: none. Host-runtime error handling with no canonical analogue; this packet consults no `OrcaSlicerDocumented/` source.

## Architecture Constraints

- ADR-0015 §Decision: "The macro/host glue MUST NOT catch and discard module fatals; they propagate up to the slice command." This packet restores conformance and must not introduce a new swallow point while removing the old one.
- **ADR-0020 is contradicted and must be amended, not silently rewritten.** `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` §Decision numbered item 1 states: "`run_stage` returns `Option<LayerStageCommit>`; `None` is the empty/`MissingComponent` case." After this packet `None` is the empty-commit case **only** — a missing component is `Err(LayerStageError::FatalModule)`. That is a reversal of the clause, not a narrowing, so per the preflight S8 rule it requires its own decision record: file `D-181-ADR-0020-AMENDED` quoting the contested clause, matching the existing `D-161-ADR-0037-AMENDED` / `D-283-ADR-0046-AMENDED` convention.
- ADR-0045 imposes no constraint — zero occurrences of `wasm_component`, `CompiledModuleLive`, or `MissingComponent`.
- **ADR-0007 conformance (`docs/adr/0007-compiled-module-static-live-split.md`).** This is the ADR that introduced the `wasm_handles` pairing threaded through every executor function, `CompiledModuleLive`'s `Option<Arc<WasmComponent>>` field, and `WasmInstancePool::placeholder()` — so it governs the surface an earlier draft of this packet proposed to change. Its §"What future architecture reviews must not re-litigate" states: **"Do not consolidate `WasmInstancePool::placeholder()` away. It exists as the explicit fallback for in-process test pipelines that don't need real dispatch."** This packet **conforms**: it does not remove `WasmInstancePool::placeholder()`, and it does not touch any of the six **production** fallback sites in `crates/slicer-runtime/src/`. An earlier draft removed them, which would have forced ~53 call-site migrations across the very "in-process test pipelines" the clause protects; measurement of that blast radius is what caused the change to be dropped. Note the scope precisely: Steps 7-9 *do* swap placeholder pools for real ones at a handful of **test** call sites (`tests/common/mod.rs`, `tests/executor/live_seam_path_tdd.rs`, `tests/contract/postpass_gcode_{empty_list,boundary}_tdd.rs`, `tests/unit/dag_validation_tdd.rs`). That is not a violation — it is what the clause's own next sentence mandates: "Tests that DO need real dispatch must use a real pool — not silently fall back through the placeholder." No amending deviation is required, and the completion gate asserts the production fallback count is still 6 so the conformance cannot silently erode.
- **`placeholder_wasm` is inferred, not declared.** `is_placeholder_wasm` (`crates/slicer-scheduler/src/manifest.rs`) is `fs::metadata(wasm_path).len() <= 8`; there is no TOML key anywhere in `modules/` or `crates/`. Any prose in this packet, in `docs/04`, or in the amended ADR must describe the real mechanism — an ≤8-byte stub file — and never say a module "declares" it.
- **No placeholder marker is introduced.** Because the skip is removed rather than conditioned, nothing needs to carry a placeholder fact from the loader to the six construction sites. This deliberately avoids widening the `wasm_handles` side-table, whose value type `(Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)` appears in ~21 signatures across the four executor modules plus `pipeline.rs`, and in three declarations in `run.rs` including a `pub` field on `PrepassContext`.
- **One behavioral blast radius, deliberately.** Changing the dispatch arms affects tests where *an absent component reaches `WasmRuntimeDispatcher`* — a set measured complete in Step 1. An earlier draft also removed the six `wasm_handles` fallbacks, which would have created a **second, much larger** radius: callers passing an empty or partial `wasm_handles` with a module-bearing stage list. Measurement found **18 such files**, 13 of them deliberately using the pattern below. That change is now out of scope, so the second radius does not exist.
- **The empty-`wasm_handles` + bespoke-runner pattern is intentional and must not be broken.** Across the `unit` and `executor` buckets (`path_ordering_tdd`, `tool_ordering_tdd`, `layer_executor_tdd`, `postpass_executor_tdd`, `prepass_executor_tdd`, `layer_finalization_tdd`, and others), tests inject a real pool/component inside a test-local runner impl and never populate `wasm_handles`, relying on the fallback. ADR-0007 sanctions exactly this. Preserving the fallbacks costs nothing: once the five arms are fatal, a genuine production handle miss is caught at dispatch.
- **The five test buckets are not the whole test surface.** `crates/slicer-runtime` has top-level un-bucketed test binaries; two are relevant — `tests/visual_debug_postpass_tap_tdd.rs` (empty `wasm_handles`) and `tests/arachne_wall_sequence_e2e_tdd.rs` (constructs a `CompiledModuleLive`, passing `Some(...)`). No `--test {unit,contract,executor,integration,e2e}` filter reaches either and `cargo check --all-targets` compiles them happily, so only explicit runs surface a regression. The completion gate names both.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Trigger note: this packet edits only host crates, but AC-3 newly makes `postpass_gcode_boundary_tdd` depend on the real `postpass-guest` executing, so the `--check` gate is mandatory before attributing an AC-3 failure to the code change.

## Code Change Surface

- Selected approach: make an absent compiled component fatal at the two points that matter — at load, and at dispatch — and migrate the tests that encoded the old skip. No marker, no propagation, no side-table change, and no change to the executor handle-lookup fallbacks.
- Exact functions, traits, manifests, tests, and fixtures:
  - `compile_module_component` (`crates/slicer-wasm-host/src/execution_plan_live.rs`) — all three `Warning` + `None` branches become fatal load errors.
  - The five `MissingComponent` consumer arms and their five `dispatch_*` producers (`crates/slicer-wasm-host/src/dispatch.rs`).
  - New `crates/slicer-runtime/tests/contract/dispatch_missing_component_tdd.rs` **plus its `mod` registration** in `tests/contract/main.rs`.
  - The migration set named in `requirements.md` §In Scope — the single radius of tests where an absent component reaches `WasmRuntimeDispatcher`.
- Rejected alternatives and reasons:
  - *Preserve the skip behind an explicit placeholder marker.* Requires carrying the fact from the loader to six sites in another crate — either widening `wasm_handles` (~21 signatures) or adding a field to `CompiledModuleStatic`. All that machinery would protect a path with **no current users** (no module in the tree is an ≤8-byte stub; the smallest real core-module `.wasm` is 68,495 bytes) and whose trigger is a file-size heuristic. Retiring it is cheaper and safer.
  - *Loader-fatal only.* Leaves five unconditional laundering arms that any future absent-component producer silently re-enters.
  - *Dispatch-narrowing only.* Leaves the loader warning-and-continuing, so the wrong-slice-looks-clean window stays open upstream.
  - *Delete the vacuous test.* It names a real boundary; the repair is to make it execute the guest.

## Files in Scope (read + edit)

Steps split these so no single step edits more than 3.

- `crates/slicer-wasm-host/src/execution_plan_live.rs` — `compile_module_component` fatal escalation.
- `crates/slicer-wasm-host/src/dispatch.rs` — the five arms.
- `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — the two new RED loader tests and the three pre-existing loader tests that encode the old behavior.
- `crates/slicer-runtime/tests/contract/dispatch_missing_component_tdd.rs` (new) + `tests/contract/main.rs` (registration).
- `crates/slicer-runtime/tests/common/dispatch_fixture.rs`, `tests/common/mod.rs` — the shared no-component fixtures.
- `crates/slicer-runtime/tests/contract/{dispatch_protocol_tdd,infill_postprocess_contract_tdd,postpass_gcode_empty_list_tdd,postpass_gcode_boundary_tdd}.rs`, `tests/unit/dag_validation_tdd.rs`, `tests/executor/live_seam_path_tdd.rs` — the migration set.

Explicitly **not** in scope: `crates/slicer-runtime/src/{prepass,layer_executor,postpass,layer_finalization}.rs`. They hold the six fallbacks, which this packet preserves.

## Read-Only Context

- `crates/slicer-wasm-host/src/dispatch.rs` (long; never in full) — ±40-line windows located by `rg 'DispatchPhase::MissingComponent'` only.
- `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` (642 lines) — ranged reads only.
- `crates/slicer-scheduler/src/manifest.rs` — `is_placeholder_wasm` and `placeholder_wasm()` only.
- `crates/slicer-wasm-host/src/pool.rs` — `WasmInstancePool::placeholder` only.
- `docs/adr/0015-...md`, `docs/adr/0020-...md` (§Decision item 1), `docs/04_host_scheduler.md` (`### Error Handling Policy`), `docs/DEVIATION_LOG.md` (DEV-087 row only).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — not consulted; never load.
- `crates/slicer-schema/wit/**` and any `.wit` file — this packet changes no interface.
- `crates/slicer-runtime/src/run.rs`, `pipeline.rs` — read-only; no side-table widening is needed under the selected approach.
- `.ralph/specs/162_*`, `163_*`, `164_*`, `165_*` — other packets' directories; SUMMARY dispatch only, never edit.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.

## Expected Sub-Agent Dispatches

- Question: list every test-side `CompiledModuleLive::new` call site passing `None`, and every use of `.no_wasm()` / `run_layer_and_commit`; scope: `crates/**/tests/**`; return: `LOCATIONS` (<=20 entries); purpose: Step 1 — the candidate set, narrowed to the real migration set by the classification dispatch below.
- Question: for each site above, does it dispatch through `WasmRuntimeDispatcher`, or through a test-local runner impl?; scope: the files named by the previous dispatch; return: `SUMMARY` (<=200 words); purpose: Step 1 — this classification is what separates the real migration set from the ~13 files that deliberately leave `wasm_handles` empty and are out of scope.
- Question: quote the three `None`-returning branches of `compile_module_component` with their diagnostic construction; scope: `crates/slicer-wasm-host/src/execution_plan_live.rs`; return: `SNIPPETS` (<=3 x 30 lines); purpose: Step 3.

## Data and Contract Notes

- IR/manifest contracts: unchanged. The `placeholder_wasm` manifest field and `is_placeholder_wasm` remain, but become inert for dispatch; removing them is later cleanup.
- WIT boundary: unchanged. Success variants span two crates — `PrepassStageOutput::None` is `slicer-core` (`src/stage_io.rs`); `FinalizationOutput::Success`, `PostpassOutput::GCodeSuccess`, `PostpassOutput::TextSuccess { text: String }` are `slicer-ir` (`src/stage_io.rs`).
- Determinism/scheduler constraints: a slice that previously produced silently-degraded output for an unreadable, uncompilable, or stub module now fails loudly. That is the intended behavior change and the reason Doc Impact touches `docs/04_host_scheduler.md` and ADR-0020.

## Locked Assumptions and Invariants

- Locks the invariant: **an absent compiled component is always an error.** There is no legal absent-component path after this packet. Non-reversible by config; it is the substance of DEV-087's closure and of the ADR-0020 amendment.
- Retires the placeholder-skip affordance documented in `manifest.rs`. Scaffolding a module now requires a `.wasm` that actually compiles.

## Risks and Tradeoffs

- **A previously-silent broken deployment now fails.** Any environment shipping a module whose `.wasm` is unreadable, uncompilable, or an ≤8-byte stub starts failing where it previously produced quiet, wrong output. The fatal error must name the module id and the underlying cause so the failure is self-diagnosing.
- **A documented capability is removed.** `manifest.rs` advertises that placeholder-binary modules stay discoverable and plan-constructible while dispatch skips them. No module uses it today, but a contributor scaffolding a new module will now hit a hard load failure instead. This is a deliberate, user-confirmed trade: a silent stage-skip keyed off a file-size heuristic is a worse failure mode than a loud error. The `docs/04` edit must state the new expectation so the removal is discoverable.
- The migration set is small but its boundary is subtle: the discriminator is whether an absent component reaches `WasmRuntimeDispatcher`, not merely whether `wasm_handles` is empty. Mis-drawing that line in either direction either leaves a red suite the packet's AC filters would not reveal, or drags in ~13 out-of-scope files. Step 1's second dispatch exists solely to settle it.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 — authoring the five-stage fatal contract test — and Step 5, the five arms; Step 3 is the third `M`)
- Highest-risk dispatch and required return format: Step 1's classification dispatch — `SUMMARY` capped at 200 words — because the discriminator ("does an absent component reach `WasmRuntimeDispatcher`, or a test-local runner?") is what separates the real migration set from the ~13 files that deliberately leave `wasm_handles` empty and are out of scope.

## Open Questions

None.
