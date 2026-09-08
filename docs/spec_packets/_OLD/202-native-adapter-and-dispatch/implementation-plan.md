# Implementation Plan: 202-native-adapter-and-dispatch

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- After every step that edits `crates/slicer-macros/**` or `crates/slicer-sdk/**`, run `cargo xtask build-guests --check` and rebuild on `STALE:` before any guest-loading test.

## Steps

### Step 1: SDK native seam types (`slicer_sdk::native`)

- Task IDs: `ADR-0056` (Decision item 3)
- Objective: create `crates/slicer-sdk/src/native.rs` with `NativeStageEntry` and the four per-family request/response envelopes, fields enumerated from the wasm32 glue survey.
- Precondition: packet 201 implemented (`ModuleProvenance` etc. in tree).
- Postcondition: `cargo check -p slicer-sdk --all-targets` green; envelopes cover every SDK value each glue family passes/drains.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/builders.rs` - accessor regions only (locate `pub fn` list by grep)
  - `crates/slicer-sdk/src/views.rs` - struct headers only
  - `crates/slicer-sdk/src/lib.rs` - module list
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/native.rs` (new)
  - `crates/slicer-sdk/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/host.rs`, `crates/slicer-macros/**`
- Expected sub-agent dispatches:
  - Question: per glue family, which SDK values are handed to the trait method and drained from the builder; scope: `crates/slicer-macros/src/lib.rs`; return: SNIPPETS ≤30 lines per family
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - Decision item 3
- OrcaSlicer refs: none
- Verification:
  - `cargo check -p slicer-sdk --all-targets 2>&1 | tee target/test-output.log` - FACT pass/fail
  - `cargo xtask build-guests --check` - must be run; rebuild on STALE (slicer-sdk is a universal guest dep)
- Exit condition: types compile natively AND the wasm32 guest build stays green (`build-guests` rebuild succeeds); a family whose glue passes a value the envelope cannot carry falsifies the envelope — extend it before exiting.

### Step 2: Macro native adapter — Layer family + AC-1 red/green

- Task IDs: `ADR-0056` (Decision item 3)
- Objective: emit `__slicer_native_entry()` for layer-family stages in `generate_slicer_module_impl`; author `native_adapter_tdd.rs` asserting `SdkLayerInfillModule::__slicer_native_entry()` is `NativeStageEntry::Layer(..)` (AC-1); add the `sdk-layer-infill-guest` dev-dep.
- Precondition: Step 1 complete.
- Postcondition: AC-1 green; all 21 workspace module crates still compile natively; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-macros/src/lib.rs` - `generate_slicer_module_impl` ±80 lines; one layer glue emitter ±80 lines
  - `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs` - short; read whole (this crate is the `#[slicer_module]`-bearing module source, not a wrapper)
- Files allowed to edit (at most 3):
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-runtime/tests/contract/native_adapter_tdd.rs` (new) + `crates/slicer-runtime/tests/contract/main.rs` (register `mod native_adapter_tdd;`) — counts as 2 edits
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/**` (later steps)
- Blast-radius discipline: `crates/slicer-runtime/Cargo.toml` dev-dep addition happens here if the test needs it now (swap one of the ≤3 edits: author the test file and Cargo.toml this step, register the mod next step if the cap binds — never exceed 3).
- Expected sub-agent dispatches:
  - Question: current `tests/contract/main.rs` mod list; scope: that file; return: FACT ≤5 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/05_module_sdk.md` - delegate SUMMARY of the `#[slicer_module]` section
- OrcaSlicer refs: none
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log` - macro emission compiles in every module crate
  - `cargo xtask build-guests --check` - rebuild on STALE
  - `cargo test -p slicer-runtime --test contract native_entry_layer_family 2>&1 | tee target/test-output.log` - AC-1
- Exit condition: AC-1 PASS with guests fresh; a wasm32 guest build failure after the macro edit falsifies the cfg-gating — fix before exiting.

### Step 3: Macro native adapter — Prepass, Postpass, Finalization families

- Task IDs: `ADR-0056` (Decision item 3)
- Objective: extend the emitter to the remaining three families (same table-driven pattern from `slicer_schema::STAGES`).
- Precondition: Step 2 green.
- Postcondition: every `#[slicer_module]` type in the workspace exposes a family-correct `__slicer_native_entry()`; guests fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-macros/src/lib.rs` - the three non-layer glue emitters, ±80 lines each (locate by symbol)
- Files allowed to edit (at most 3):
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-runtime/tests/contract/native_adapter_tdd.rs` (family assertions for a prepass/postpass/finalization witness — reuse existing sdk test-guest crates as dev-deps if needed, else assert via a minimal in-test `#[slicer_module]` impl)
  - `crates/slicer-runtime/Cargo.toml` (dev-deps if needed)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/**`
- Expected sub-agent dispatches:
  - Question: which sdk-* test-guest crates cover prepass/postpass/finalization (`sdk-prepass-guest`, `sdk-postpass-text-guest`, `sdk-finalization-guest`) and their module type names; scope: `crates/slicer-wasm-host/test-guests/*/src/lib.rs`; return: LOCATIONS ≤10
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - Decision item 3
- OrcaSlicer refs: none
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`
  - `cargo xtask build-guests --check` - rebuild on STALE
  - `cargo test -p slicer-runtime --test contract native_adapter 2>&1 | tee target/test-output.log`
- Exit condition: family assertions PASS for all four families; guests fresh.

### Step 4: Native transport in the marshalling boundary (`marshal/native.rs`)

- Task IDs: `ADR-0056` (Decision item 3)
- Objective: implement request builders (IR/`*StageInput` → SDK views) and response committers (SDK output → `*OutputCollected` → existing `out.rs` + `OriginBucket`) for all four families.
- Precondition: Step 1 complete (envelopes exist).
- Postcondition: `cargo check -p slicer-wasm-host --all-targets` green; layer request field list mirrors, one-for-one, the field list `adapt_slice_regions_completeness_tdd.rs` pins for the wasm leg.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/mod.rs` - whole (~35 lines)
  - `crates/slicer-wasm-host/src/marshal/accumulators.rs` - struct definitions only
  - `crates/slicer-wasm-host/src/marshal/out.rs` - converter signatures only (grep `pub fn`)
  - `crates/slicer-wasm-host/src/binding.rs` - `*StageInput` structs (lines 60–140)
  - `crates/slicer-runtime/tests/integration/adapt_slice_regions_completeness_tdd.rs` - field checklist region
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/marshal/native.rs` (new)
  - `crates/slicer-wasm-host/src/marshal/mod.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/marshal/{in_.rs,out.rs,origin.rs,accumulators.rs,leaf.rs}` - shared code is consumed, never edited (one-answer rule)
- Expected sub-agent dispatches:
  - Question: how does `dispatch.rs` feed `*OutputCollected` into `out.rs` converters and `OriginBucket` for the layer family (call shape only); scope: `crates/slicer-wasm-host/src/dispatch.rs`; return: SNIPPETS ≤30 lines
- Context cost: `M`
- Authoritative docs:
  - `CONTEXT.md` term "Marshalling boundary" - already quoted in design.md; do not re-read CONTEXT.md
- OrcaSlicer refs: none
- Verification:
  - `cargo check -p slicer-wasm-host --all-targets 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: compiles; the per-field mirror checklist (written as a code comment table in `native.rs`) covers every field in the completeness test's list — any missing field falsifies the step.

### Step 5: Dispatch routing branch in the four runner impls

- Task IDs: `ADR-0056` (Decision item 3)
- Objective: add `native_entry: Option<NativeStageEntry>` to `CompiledModuleLive` (default `None` in `new`, add `with_native_entry`); add the native branch at the entry of the four `impl *StageRunner for WasmRuntimeDispatcher` blocks calling `marshal/native.rs`.
- Precondition: Step 4 complete.
- Postcondition: wasm-path behavior bit-identical when `native_entry` is `None` (full contract regression green).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/dispatch.rs` - the four impl entry regions only (locate by `impl PrepassStageRunner for WasmRuntimeDispatcher` and siblings)
  - `crates/slicer-wasm-host/src/binding.rs` - lines 20–140
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/binding.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/host.rs` (bindgen worlds untouched)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `CompiledModuleLive.native_entry`: all constructions go through `CompiledModuleLive::new` (measured at authoring: 6 production call sites in `crates/slicer-runtime/src/{layer_executor,postpass,prepass,layer_finalization}.rs` plus test sites in 13 files, zero struct literals outside `binding.rs`); defaulting inside `new` means no caller edits this step. Dispatch a LOCATIONS re-check before editing; if any literal construction exists, add that file to the edit list or split the step.
- Expected sub-agent dispatches:
  - Question: re-verify `CompiledModuleLive` construction sites (`::new(` and `CompiledModuleLive {`); scope: `crates/`; return: LOCATIONS ≤20
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` - Decision section (no wasm-internal types cross the seam)
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log` - full contract binary green (wasm-path regression incl. `macro_all_worlds_roundtrip_tdd`)
  - `cargo test -p slicer-wasm-host 2>&1 | tee target/test-output.log` - dispatcher-protocol regression
- Exit condition: both regressions green with `native_entry: None` everywhere; any behavioral diff on the wasm path falsifies the branch placement.

### Step 6: Parity seam contract tests (AC-2, AC-3)

- Task IDs: `ADR-0056` (Decision items 3–4)
- Objective: author `native_dispatch_parity_seam_tdd.rs` — identical `LayerStageInput` through wasm and native paths of one `WasmRuntimeDispatcher` using `sdk-layer-infill-guest` (native type + `.component.wasm` twin); structural-equality assertions per AC-2; component-free native dispatch per AC-3.
- Precondition: Steps 2, 5 complete; guests fresh (`cargo xtask build-guests --check` clean).
- Postcondition: AC-2 and AC-3 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/macro_all_worlds_roundtrip_tdd.rs` - setup/driver region only
  - `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs` - whole
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs`
  - `crates/slicer-runtime/Cargo.toml` (dev-dep if not added in Step 2)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/**` (no production edits to make a test pass — parity failures are diagnosed, not papered over)
- Expected sub-agent dispatches:
  - Question: run the two AC commands; scope: repo root; return: FACT pass/fail + failing-assert SNIPPET ≤20 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - Decision item 4 (no byte-equality)
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test contract native_dispatch_parity_seam 2>&1 | tee target/test-output.log` - AC-2
  - `cargo test -p slicer-runtime --test contract native_dispatch_without_component 2>&1 | tee target/test-output.log` - AC-3
- Exit condition: AC-2 and AC-3 PASS on a fixture with ≥1 non-empty region (an empty-commit "pass" falsifies the fixture, not the code); structural mismatch between paths is a defect in `marshal/native.rs` or the adapter — fix there, never relax the assertions.

### Step 7: Plumbing — binding attachment, run.rs, registry table (AC-4, AC-N1, AC-N2)

- Task IDs: `ADR-0056` (Decision items 2–3)
- Objective: add `native_entry` to `LiveModuleBinding`; extend `load_live_modules_for_plan_with_integrated` with `native_entries` + the attachment rule; thread `binding.native_entry` through the four executor projection sites via `with_native_entry`; add `slicer_integrated_modules::native_entries()` and pass it in `run.rs`; author the three integration tests.
- Precondition: Steps 5–6 complete.
- Postcondition: AC-4, AC-N1, AC-N2 green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` - lines 30–345
  - `crates/slicer-runtime/src/run.rs` - loader call regions only
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` - setup region
- Files allowed to edit (at most 3 per sub-slice; execute as three sub-slices):
  - 7a: `crates/slicer-wasm-host/src/execution_plan_live.rs`, `crates/slicer-integrated-modules/src/lib.rs`, `crates/slicer-runtime/src/run.rs`
  - 7b: `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-runtime/src/postpass.rs` (projection lines only)
  - 7c: `crates/slicer-runtime/src/layer_finalization.rs`, `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` (the `LiveModuleBinding` literal site)
- Files explicitly out of bounds:
  - `crates/pnp-cli/**` (203), `SliceRunOptions` fields
- Blast-radius discipline: `LiveModuleBinding.native_entry` — struct-literal sites measured at authoring: `crates/slicer-wasm-host/src/execution_plan_live.rs` and `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`; both belong to this step's edit budget (execution_plan_live.rs in 7a, config_view_binding_tdd.rs in 7c). Re-verify with a LOCATIONS dispatch before editing.
- Expected sub-agent dispatches:
  - Question: re-verify `LiveModuleBinding {` literal sites; scope: `crates/`; return: LOCATIONS ≤10
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - Decision item 2 (override falls out of first-root-wins)
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test integration integrated_binding_attaches_native_entry 2>&1 | tee target/test-output.log` - AC-4
  - `cargo test -p slicer-runtime --test integration external_override_forces_wasm_dispatch 2>&1 | tee target/test-output.log` - AC-N1
  - `cargo test -p slicer-runtime --test integration integrated_without_native_entry_fails_loud 2>&1 | tee target/test-output.log` - AC-N2
- Exit condition: all three PASS; an external-provenance binding ever receiving `Some` native entry falsifies the attachment rule (this is the packet's core negative guarantee).

### Step 8: Docs + closure gates

- Task IDs: `ADR-0056` (Decision items 3–5)
- Objective: land the two doc edits and run every gate.
- Precondition: Steps 1–7 complete.
- Postcondition: AC-N3 greps pass; gates green; guests fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/04_host_scheduler.md` - §Phase 4 heading region only (locate by grep)
  - `docs/05_module_sdk.md` - `#[slicer_module]` section only (locate by grep)
- Files allowed to edit (at most 3):
  - `docs/04_host_scheduler.md`
  - `docs/05_module_sdk.md`
- Files explicitly out of bounds:
  - `CONTEXT.md`, `docs/adr/*`, `docs/07_implementation_status.md`
- Expected sub-agent dispatches:
  - Question: run all pipe-suffixed AC commands + the three gates + `cargo xtask build-guests --check`; scope: repo root; return: FACT pass/fail each
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - Decision item 5 wording for the single-threaded note
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'native_entry' docs/04_host_scheduler.md && rg -q '__slicer_native_entry' docs/05_module_sdk.md` - AC-N3
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT clean
- Exit condition: every pipe-suffixed AC command PASS; guests fresh; clippy clean.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | envelope enumeration via dispatches |
| Step 2 | M | macro emission (layer) + AC-1 |
| Step 3 | M | remaining families |
| Step 4 | M | marshal/native.rs |
| Step 5 | M | dispatch branch + binding field |
| Step 6 | M | parity seam tests |
| Step 7 | M | plumbing + negative ACs (two sub-slices) |
| Step 8 | S | docs + gates |

Split before activation if aggregate cost exceeds M or any step is L. (Aggregate is M: steps are narrow-file, dispatch-heavy, and each stays within its ≤3-edit cap; if any single step trends L in practice, split it at the family boundary.)

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is NOT updated by this packet (no TASK rows exist; plan-level [FWD] pending — see `requirements.md` §Packet Metadata).
- Reconcile reopened/superseded status transitions: none (no packet superseded).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command, plus `cargo xtask build-guests --check`.
- Record remaining packet-local risk (input-leg field-mapping drift pending 204's parity gate; empty native instrumentation captures).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
