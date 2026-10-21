# Implementation Plan: lightning-migration

## Execution Rules

Every step maps to `TASK-577`, uses an S/M context slice, and is numbered in **execution order**: the oracle exists before the first production edit; the new path is proven before anything old is removed; deletions are leaf-first so every step's own check compiles.

- **Planning logic is frozen.** The kernel may move, change container types, gain an input adapter and change its plan type's home. It may not change a planning decision. If equivalence fails for a reason that needs a planning change, stop and re-approve; never retune the baseline.
- **Deletion gate.** Steps 10a–13c are forbidden until AC-2 and AC-3 pass.
- **Edit cap.** Three hand-edited files per step. Where a public field's removal is atomic across its constructors and readers, the step that removes it owns the whole blast radius (four or five files) and says so — the alternative is a shipped shim, which this packet forbids.
- **Test literals** use FRU/shared fixture bases or a recorded exhaustive waiver; production literals stay exhaustive.
- **Gate sequence across the deletion phase.** Removing a public type or accessor necessarily breaks test targets before the test steps repair them. Steps 11a–13c therefore verify `cargo check --workspace` (library targets of every crate), each test step verifies its own binary, and **`cargo check --workspace --all-targets` is mandatory at Step 16**. A green mid-stream check is never evidence that test targets compile.
- **Log discipline.** Archive the previous logs, then successfully remove `target/test-output.log` before every xtask invocation under `set -euo pipefail`.
- **Guest rebuild** is required after any change to a guest fingerprint input (module sources/manifest, WIT, `slicer-core`), before re-running the affected test.

## Step 1 — Record the behavioural baseline

- Task IDs: TASK-577. Objective: capture the live host producer's output as a committed, deterministic oracle before anything moves.
- Precondition: the tree builds; the five Classic case inputs exist under `crates/slicer-wasm-host/tests/fixtures/lightning_gate/`.
- Postcondition: `crates/slicer-runtime/tests/fixtures/lightning_baseline/{baseline_classic.json, manifest.json}` committed: per case the full-identity plan entries with directed edge multisets and grounding/root witnesses, the per-caller segment multisets, and `sha256` of plan and inputs. Two recorder runs byte-identical.
- Allowed reads: `crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs::run_slice_against_wedge_returns_nonempty_gcode` and its `SliceRunOptions` construction; the manifest-staging helper in `crates/slicer-runtime/tests/e2e/preparation_runtime_lifecycle_tdd.rs`; `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` symbol window.
- Allowed edits: NET-NEW `crates/slicer-runtime/tests/e2e/lightning_baseline.rs`; `crates/slicer-runtime/tests/e2e/main.rs`; the recorder-generated `crates/slicer-runtime/tests/fixtures/lightning_baseline/` directory (committed verbatim).
- Out of bounds: production source, the `lightning_gate` fixtures, other packet directories.
- Dispatch: confirm the recorder drives the real configured prepass and reads the committed plan; return SUMMARY ≤ 200 words. Inspect the fixture through the decoder.
- Context: M. Authorities: docs04, docs21, docs22.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_baseline::record_lightning_baseline -- --ignored --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: recorder executes, both files written, a second run byte-identical. Falsifying exit: a case yields zero entries, a digest is absent, or two runs differ.

## Step 2 — Register the acceptance harness (red until implemented)

- Task IDs: TASK-577. Objective: land the nine NET-NEW tests with real drivers and independent comparators.
- Precondition: Step 1 baseline exists and AC-1 passes.
- Postcondition: `lightning_migration_tdd` registered; AC-1 green; the other eight compile and fail for their stated reason, recorded as RED evidence.
- Allowed reads: Step 1 helpers; `crates/slicer-runtime/tests/e2e/main.rs`; `crates/slicer-runtime/src/preparation_lifecycle.rs::SliceRuntimeExtensions` window.
- Allowed edits: NET-NEW `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`; `crates/slicer-runtime/tests/e2e/main.rs`; `crates/slicer-runtime/Cargo.toml` (dev-dependencies only).
- Out of bounds: production code, module code, the baseline fixture.
- Dispatch: does each test instantiate the real module and compare to the baseline rather than to itself? Return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs22, docs21.
- Narrow verification: the AC-1 command from `packet.spec.md`.
- Exit: AC-1 passes and the other eight execute. Falsifying exit: a test compiles to an empty body or compares adapters to each other.

## Step 3 — Kernel scaffold, plan types and errors

- Task IDs: TASK-577. Objective: create the kernel module inside the consuming artifact and give the plan types their private home.
- Precondition: none beyond the current tree.
- Postcondition: `src/kernel/{mod.rs, plan.rs, error.rs}` exist; `KernelPlan`/`PlanEntry` are `pub(crate)` counterparts of `LightningTreeIR`/`LightningTreeEntry` **without** a schema-version constant; `KernelError` preserves the current variants.
- Allowed reads: `crates/slicer-ir/src/slice_ir.rs` plan-type definitions; `crates/slicer-core/src/algos/lightning/error.rs`; `modules/core-modules/lightning-infill/src/lib.rs` module root.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/kernel/mod.rs`; NET-NEW `src/kernel/plan.rs`; NET-NEW `src/kernel/error.rs`.
- Out of bounds: the host algorithm (it runs until Step 10c), `slicer-ir`, the consumer path.
- Dispatch: confirm the plan types carry every field the baseline records and no version constant; return FACT.
- Context: S. Authorities: docs02, docs08.
- Narrow verification: `cargo check -p lightning-infill`.
- Exit: the module compiles with the new module declared but unused. Falsifying exit: a recorded field has no home in `PlanEntry`.

## Step 4 — Move the planning algorithm bodies

- Task IDs: TASK-577. Objective: relocate the algorithm into the kernel with no change to planning decisions.
- Precondition: Step 3 plan/error types exist.
- Postcondition: `src/kernel/{tree_node,layer,distance_field}.rs` contain the moved bodies with kernel-relative paths. The host copies remain and still compile, so the Step 1 recorder keeps producing the same baseline.
- Allowed reads: the three host files; `crates/slicer-core/src/algos/lightning/mod.rs` module list.
- Allowed edits: NET-NEW `src/kernel/tree_node.rs`; NET-NEW `src/kernel/layer.rs`; NET-NEW `src/kernel/distance_field.rs`.
- Out of bounds: the host copies, the generator (Step 5).
- Dispatch: diff the moved bodies against the originals and confirm only `use`/type paths changed; return FACT with any non-path difference quoted.
- Context: M. Authorities: docs02.
- Narrow verification: `cargo check -p lightning-infill`.
- Exit: the kernel modules compile standalone. Falsifying exit: a moved body differs beyond path/type adaptation.

## Step 5 — Generator move and typed input adaptation

- Task IDs: TASK-577. Objective: complete the kernel behind one entry point fed by preparation data.
- Precondition: Step 4 moved bodies exist.
- Postcondition: `src/kernel/generator.rs` holds `Generator::generate_trees`/`committed_segments_for_layer`; `src/kernel/input.rs` defines `KernelInput`/`KernelLayer`/`KernelRegion`/`KernelConfig`; `src/kernel/mod.rs` exposes `plan_layer_trees(&KernelInput) -> Result<KernelPlan, KernelError>` in place of `generate_lightning_trees(&[SliceIR], &ResolvedConfig)`. The `sparse_fill_holder` gate is dropped (eligibility is selection, D2).
- Allowed reads: host `generator.rs`; `crates/slicer-sdk/src/preparation_input.rs::PreparationRegion` fields; `crates/slicer-sdk/src/preparation.rs::ConfigView` accessors.
- Allowed edits: NET-NEW `src/kernel/generator.rs`; NET-NEW `src/kernel/input.rs`; `src/kernel/mod.rs`.
- Out of bounds: the host copies, the consumer, the manifest.
- Dispatch: list every value the kernel reads and the typed source supplying it; any value with no source is a stop, not a default; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs02, docs05, docs08.
- Narrow verification: `cargo check -p lightning-infill`.
- Exit: the kernel compiles with no `SliceIR`/`ResolvedConfig` reference. Falsifying exit: a scalar is defaulted because its typed source was not found.

## Step 6 — Move the algorithm tests

- Task IDs: TASK-577. Objective: keep the algorithm's unit coverage with its new owner.
- Precondition: Step 5 kernel entry exists.
- Postcondition: `modules/core-modules/lightning-infill/tests/kernel_tdd.rs` carries the moved assertions adapted to `plan_layer_trees`; `crates/slicer-core/tests/algo_lightning_tdd.rs` is deleted; its `[[test]]` entry with `required-features = ["host-algos"]` is gone.
- Allowed reads: `crates/slicer-core/tests/algo_lightning_tdd.rs`; the kernel entry; `crates/slicer-core/Cargo.toml` `[[test]]` block.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/tests/kernel_tdd.rs`; `crates/slicer-core/tests/algo_lightning_tdd.rs` (delete); `crates/slicer-core/Cargo.toml`.
- Out of bounds: the host algorithm sources, the runtime.
- Dispatch: confirm no assertion is dropped; return moved-vs-original assertion counts.
- Context: S. Authorities: docs22.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p lightning-infill --test kernel_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the moved tests pass under the kernel owner. Falsifying exit: assertion count drops or a test becomes `#[ignore]`d.

## Step 7a — Module-private piece codec

- Task IDs: TASK-577. Objective: implement D3's encoder/decoder and error contract, module-local.
- Precondition: Step 3 plan types exist; D3 is authoritative.
- Postcondition: `private_plan` encodes one piece per layer and decodes it with bounds checks; round-trip preserves full identity; malformed → `57410`, unsupported version → `57411`; the inherited `57401` missing-piece path is untouched; no host dependency, no float on the wire.
- Allowed reads: D3 in `design.md`; `src/kernel/plan.rs`; `crates/slicer-sdk/src/preparation.rs::PreparedPiece` window.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/private_plan.rs`; `modules/core-modules/lightning-infill/src/lib.rs` (`mod` declaration only); `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`.
- Out of bounds: generic SDK/schema/WIT, the kernel internals, the projection.
- Dispatch: bounded-read decode preserves identity and rejects each malformed case with the exact code; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs08 units, docs21, docs22.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p lightning-infill --test lightning_infill_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: round-trip and both error codes assert with literal expected bodies. Falsifying exit: a truncated buffer decodes, or a version mismatch returns `57410`.

## Step 7b — Implement preparation and declare the capability

- Task IDs: TASK-577. Objective: plan from typed committed inputs and publish pieces.
- Precondition: kernel entry exists (Step 5); Step 7a codec exists; baseline available.
- Postcondition: `impl LayerPreparation for LightningInfill` reads regions/configs/targets, builds `KernelInput`, calls `plan_layer_trees`, encodes one piece per layer through `private_plan`; manifest declares `[preparation]`; the module compiles into the guest. AC-2 passes on native and fresh WASM.
- Allowed reads: `crates/slicer-sdk/src/preparation.rs` (`PrintPreparationInput`, `PreparedPlanOutput`); `crates/slicer-wasm-host/src/preparation_input.rs::validate_reads`; an existing preparation-capable test guest manifest.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/preparation.rs`; `modules/core-modules/lightning-infill/lightning-infill.toml`; `modules/core-modules/lightning-infill/src/lib.rs` (`mod` declarations only).
- Out of bounds: the consumer path (Step 8), the kernel internals.
- Dispatch: confirm each declared `input_reads` matches the accessor it feeds; on failure return the `MissingRequired { read }` name.
- Context: M. Authorities: ADR-0066 as amended, docs04/05, docs08.
- Narrow verification: AC-2 command from `packet.spec.md`, preceded by `cargo xtask build-guests --check` (exit 0 required; exit 1 → rebuild → recheck).
- Exit: AC-2 green with nonempty populations on both adapters. Falsifying exit: divergence whose recorded input digest differs first.

## Step 8 — Fresh consumers and typed projection

- Task IDs: TASK-577. Objective: switch the ordinary call to private pieces and publish `planning_geometry` from the same plan.
- Precondition: Step 7b publishes pieces; AC-2 green.
- Postcondition: `run_infill` uses `PreparedPlanView::current()` + `require("lightning-layer-{index}")` and the module decoder, emitting unchanged role/width identity; `projection.rs` builds the `lightning-plan` scene; AC-3 and AC-4 green.
- Allowed reads: `crates/slicer-sdk/src/preparation_visual.rs`; `crates/slicer-runtime/src/preparation_visual_capture.rs::{PreparationViewRequest, PreparationVisualCapture}` and `crates/slicer-runtime/src/run.rs::run_slice_preparation_visual_capture`; current `run_infill` window.
- Allowed edits: `modules/core-modules/lightning-infill/src/lib.rs`; NET-NEW `modules/core-modules/lightning-infill/src/projection.rs`.
- Out of bounds: generic projection schema, CLI renderer, runtime capture structs, the accessor retirement.
- Dispatch: does the scene derive from the same in-memory plan, and does capture-off preserve semantic output? Return FACT.
- Context: M. Authorities: row 06 contract, docs19, docs08, docs22.
- Narrow verification: the AC-3 and AC-4 commands from `packet.spec.md`.
- Exit: both green; `retained_bytes == 0` after disposal; empty scene succeeds, missing scene names its row-06 error. Falsifying exit: capture-off output identical but the scene built by a second planning pass.

## Step 9 — Retarget module-local tests

- Task IDs: TASK-577. Objective: move the module's own tests onto real prepared pieces without shrinking their population.
- Precondition: Step 8 green.
- Postcondition: `lightning_infill_tdd` asserts through prepared pieces; `slicer_module_binding_tdd` asserts the paired export without a tree fixture.
- Allowed reads: both test files; Step 7/8 helpers.
- Allowed edits: `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`; `modules/core-modules/lightning-infill/tests/slicer_module_binding_tdd.rs`.
- Out of bounds: production module code, other crates' tests.
- Dispatch: for each changed test, name the surviving contract and its asserted population; return SUMMARY ≤ 200 words.
- Context: S. Authorities: docs22, docs21.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p lightning-infill 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: all module tests pass with no success-only replacement. Falsifying exit: an assertion count drops without a stated contract change.

## Step 10a — Remove the prepass wiring

- Task IDs: TASK-577. Objective: unhook the stage from the configured prepass while its producer still exists. This is the leaf end of the chain: nothing calls into the prepass block.
- Precondition: **AC-2 and AC-3 green.**
- Postcondition: no `run_builtin_stage` block for lightning, no `PrepassExecutionError::LightningTree`, no `"PrePass::LightningTreeGen" => &[…]` arm in `required_slots`, and no `BlackboardPrepassSlot::LightningTreeIR => blackboard.lightning_tree_ir().is_some()` arm in `ensure_stage_prerequisites` — that last arm is the final reader of the getter Step 12a deletes. The executor test and its registration are retired; the producer module still compiles, now unreferenced.
- Allowed reads: `prepass.rs` window; `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs`; `tests/executor/main.rs`.
- Allowed edits: `crates/slicer-runtime/src/prepass.rs`; `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs`; `crates/slicer-runtime/tests/executor/main.rs`.
- Out of bounds: the producer file (Step 10b), blackboard, scheduler.
- Dispatch: confirm no other configured-prepass path references the stage; return FACT.
- Context: S. Authorities: docs04, docs22.
- Narrow verification: `cargo check -p slicer-runtime --tests` (every runtime test target compiles after the executor test is retired).
- Exit: check passes; the producer is dead code pending deletion. Falsifying exit: the stage is still scheduled anywhere.

## Step 10b — Delete the host producer

- Task IDs: TASK-577. Objective: remove the now-unreferenced producer artifact and its exports.
- Precondition: Step 10a applied.
- Postcondition: `lightning_tree_producer.rs` deleted; `builtins/mod.rs` no longer exports it; the `crates/slicer-runtime/src/lib.rs` re-export and its "not registered" commentary removed.
- Allowed reads: the three files' symbol windows.
- Allowed edits: `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` (delete); `crates/slicer-runtime/src/builtins/mod.rs`; `crates/slicer-runtime/src/lib.rs`.
- Out of bounds: the host algorithm copy (Step 10c), `slicer-ir`.
- Dispatch: confirm the closure for `commit_lightning_tree_ir_builtin` is empty; return LOCATIONS ≤ 10.
- Context: S. Authorities: ADR-0066 as amended, docs01/04.
- Narrow verification: `cargo check --workspace --all-targets`.
- Exit: no producer symbol remains. Falsifying exit: a deprecated alias appears instead of deletion.

## Step 10c — Delete the host copy of the algorithm

- Task IDs: TASK-577. Objective: make the module the only owner of lightning planning.
- Precondition: Step 10b applied (no host caller remains); Steps 3–6 delivered the kernel.
- Postcondition: `crates/slicer-core/src/algos/lightning/` is deleted; `algos/mod.rs` no longer declares it; `host-algos` keeps only its rayon/voronoi algorithms.
- Allowed reads: `crates/slicer-core/src/algos/mod.rs`; `rg` closure for `algos::lightning`.
- Allowed edits: the directory delete; `crates/slicer-core/src/algos/mod.rs`.
- Out of bounds: the module kernel, `slicer-sdk`.
- Dispatch: confirm the closure returns no remaining caller; return LOCATIONS ≤ 10.
- Context: S. Authorities: docs01.
- Narrow verification: `cargo check --workspace --all-targets`.
- Exit: the workspace compiles with no host lightning algorithm. Falsifying exit: a `pub use` shim re-exports the deleted path.

## Step 11a — Remove the SDK accessor

- Task IDs: TASK-577. Objective: delete the guest-facing query method whose only production caller was switched in Step 8.
- Precondition: Step 8 applied; the module no longer calls it.
- Postcondition: `PaintRegionLayerView::lightning_tree_segments_for` is gone; `with_lightning_tree_ir`/the `lightning_tree_ir` field remain for now because `marshal/native.rs` still uses them (Step 11b removes both together).
- Allowed reads: `crates/slicer-sdk/src/traits.rs` accessor window; `rg` closure for `lightning_tree_segments_for`.
- Allowed edits: `crates/slicer-sdk/src/traits.rs`.
- Out of bounds: the field and its builders (atomic with Step 11b).
- Dispatch: confirm zero remaining callers before editing; return FACT.
- Context: S. Authorities: docs05.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes with the accessor deleted. Falsifying exit: an empty-vector forwarding stub is left behind.

## Step 11b — Retire the macro glue, watch entry and linker key

- Task IDs: TASK-577. Objective: remove generated-code references before the SDK field and the contract package disappear.
- Precondition: Step 11a applied.
- Postcondition: the macro no longer emits the lightning-taking form of `__slicer_adapt_paint_layer` (`crates/slicer-macros/src/lib.rs::build_layer_infill_glue`, the only caller of `PaintRegionLayerView::with_lightning_tree_ir` outside `marshal/native.rs`) nor `__slicer_lightning_tree_from_view`; the generated `run` signature no longer takes `lightning_queries: LightningQueryView` and its `layer_per_stage_aliases` list no longer imports that resource, so removing the WIT import in Step 13a leaves no dangling parameter; `LIGHTNING_QUERIES_WIT` and its `nest_dep` branch are gone; `build.rs` no longer watches the package; `preparation_bindings.rs` has no linker key for it.
- Allowed reads: the three files' windows; `rg` closure for the glue symbols.
- Allowed edits: `crates/slicer-macros/src/lib.rs`; `crates/slicer-macros/build.rs`; `crates/slicer-wasm-host/src/preparation_bindings.rs`.
- Out of bounds: the WIT package file (Step 13a), `slicer-sdk`.
- Dispatch: confirm no generated path still references the removed symbols; return LOCATIONS ≤ 20.
- Context: S. Authorities: docs03, docs05.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes with no glue emitter and no `with_lightning_tree_ir` caller outside `marshal/native.rs`. Falsifying exit: the glue is kept behind a `cfg` to keep a binding alive.

## Step 11c — Remove the stage-input field and its whole blast radius

- Task IDs: TASK-577. Objective: remove `LayerStageInput::lightning_tree_ir`, the host query data it fed, the SDK field, and every constructor/reader, in one transition.
- Precondition: Step 11b applied (the macro no longer calls it); Steps 10a–10c removed the producer side.
- Postcondition: the field, its two `layer_executor.rs` constructors, its two `marshal/native.rs` uses, the `dispatch.rs` parameters/builders including the legacy test helper, the `host.rs` `LightningQueryViewData`/handler, and the `PaintRegionLayerView::with_lightning_tree_ir`/field pair are all gone. No empty-list handler survives.
- Allowed reads: the six files' windows; `rg` closure for `lightning_tree_ir` and `with_lightning_tree_ir`.
- Allowed edits: `crates/slicer-wasm-host/src/binding.rs`; `crates/slicer-runtime/src/layer_executor.rs`; `crates/slicer-wasm-host/src/marshal/native.rs`; `crates/slicer-wasm-host/src/dispatch.rs`; `crates/slicer-wasm-host/src/host.rs`; `crates/slicer-sdk/src/traits.rs`. **Six-file step by design**: the field cannot be removed without every initializer and reader, and the packet forbids a transitional shim. Test-literal sites are Steps 14a–14d.
- Out of bounds: the WIT files (Step 13a), `slicer-ir`, test sources (Steps 14a–14d).
- Dispatch: enumerate every site before editing and confirm each is in the edit list; any site outside it stops the step.
- Context: M. Authorities: docs03, docs04, docs05.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes with no field, no handler and no shim. Falsifying exit: a `None`-returning accessor persists.

## Step 11d — Retarget the test guest and its two contract tests

- Task IDs: TASK-577. Objective: keep the export/import linkage proof without the retired query.
- Precondition: Step 11c applied.
- Postcondition: `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs` imports no `LightningQueryView`; `wit_boundary_tdd` and `preparation_pilot_host` assert the prepared path or drop only accessor-specific assertions.
- Allowed reads: the three files.
- Allowed edits: the test guest `src/lib.rs`; `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs`; `crates/slicer-wasm-host/tests/contract/preparation_pilot_host.rs`.
- Out of bounds: production wasm-host code, the real `lightning-infill` module.
- Dispatch: confirm each surviving test names the contract it protects; return SUMMARY ≤ 200 words.
- Context: S. Authorities: docs03, docs22.
- Narrow verification: `cargo check --workspace` (the wasm-host contract binary stays red until Step 14c, by design).
- Exit: the guest imports no deleted package and the workspace libraries compile. Falsifying exit: the guest still imports a deleted package while the check passes for another reason.

## Step 12a — Retire the Blackboard storage and the plan IR

- Task IDs: TASK-577. Objective: remove the public plan IR and its runtime storage, now that nothing reads them.
- Precondition: Steps 11a–11d applied.
- Postcondition: `blackboard.rs` has no lightning field/commit/getter/byte estimate; `slice_ir.rs` has no `LightningTreeIR`/`LightningTreeEntry`/version const; `slicer-ir` re-exports gone. `stage_io.rs` still carries the slot/error names, which hold no type reference and are removed in Step 12b.
- Allowed reads: the three files' windows plus an `rg` closure for the four symbols.
- Allowed edits: `crates/slicer-runtime/src/blackboard.rs`; `crates/slicer-ir/src/lib.rs`; `crates/slicer-ir/src/slice_ir.rs`.
- Out of bounds: `stage_io.rs` (Step 12b), the module kernel.
- Dispatch: re-derive the remaining-reference closure before editing; return LOCATIONS ≤ 20.
- Context: M. Authorities: docs02, docs21.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes. Falsifying exit: a `#[deprecated]` struct or compat alias survives.

## Step 12b — Retire the stage identity from slots and the scheduler

- Task IDs: TASK-577. Objective: remove the stage name everywhere it is enumerated, in one transition.
- Precondition: Step 12a applied.
- Postcondition: `stage_io.rs` has no `BlackboardPrepassSlot::LightningTreeIR`, display arm or `BlackboardError::LightningTreeGeneration`; `STAGE_ORDER` has no `PrePass::LightningTreeGen`; both scheduler tests that assert its membership (`stage_order_tdd`, `stage_list_consistency_tdd`'s `HOST_ONLY_STAGES`) are updated in the same step.
- Allowed reads: the four files' windows.
- Allowed edits: `crates/slicer-ir/src/stage_io.rs`; `crates/slicer-scheduler/src/execution_plan.rs`; `crates/slicer-scheduler/tests/stage_order_tdd.rs`; `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`. **Four-file step by design**: the roster test asserts the stage's presence, so it cannot be deferred.
- Out of bounds: blackboard, wasm-host.
- Dispatch: confirm no other stage list derives the entry; return LOCATIONS ≤ 10.
- Context: M. Authorities: docs04.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: scheduler tests pass and the stage is absent from product code. Falsifying exit: a hand-maintained roster still lists it.

## Step 13a — Delete the WIT package and its imports

- Task IDs: TASK-577. Objective: remove the contract surface the accessor lived in.
- Precondition: Steps 11b–11d applied (no `with:` key, no handler, no guest import).
- Postcondition: `crates/slicer-schema/wit/deps/lightning-queries/` is gone; `layer-infill.wit` and the dragon-curve mirror no longer import it; the world resolves.
- Allowed reads: both `layer-infill.wit` files; the package file; `crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit` (its `prepared-infill-module` includes `layer-infill`).
- Allowed edits: delete `crates/slicer-schema/wit/deps/lightning-queries/lightning-queries.wit`; `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit`; `modules/community-modules/dragon-curve/wit/layer-infill.wit`.
- Out of bounds: `xtask` expectations (Step 13c), the module's manifest.
- Dispatch: run `cargo xtask build-guests --check`; exit 0 fresh, 1 stale → rebuild → recheck, 3 infrastructure (never clean). Return the exit code.
- Context: S. Authorities: docs03, AGENTS.md guest freshness.
- Narrow verification: `cargo xtask build-guests --check`, then `cargo check --workspace` (test targets are knowingly red until Step 14c).
- Exit: the world compiles and guests are fresh. Falsifying exit: a hollow interface or package is retained to keep a binding alive.

## Step 13b — Retire the example module's vendored copy and regenerate its bindings

- Task IDs: TASK-577. Objective: keep the committed dragon-curve example consistent.
- Precondition: Step 13a applied.
- Postcondition: the vendored package copy is gone; `Makefile` no longer lists it as wit-bindgen input; the glue templates no longer reference it; generated `interface/` output is regenerated by the module's documented `make bindings`, never hand-edited.
- Allowed reads: `modules/community-modules/dragon-curve/{Makefile, src/glue/main.mbt.in, src/glue/moon.pkg.json.in}`.
- Allowed edits: delete the vendored `wit/deps/lightning-queries/`; `Makefile`; `src/glue/main.mbt.in`; `src/glue/moon.pkg.json.in`. **Four-file step by design**: the generator reads all four as one input set.
- Out of bounds: the historical probe under `docs/feasibility-probes/`; generated output as a hand edit.
- Dispatch: confirm the regenerated tree contains no `lightning-queries` reference.
- Context: S. Authorities: docs03, docs14.
- Narrow verification: `rg lightning-queries modules/community-modules/dragon-curve` must return **no match**, and `cargo check --workspace` passes.
- Exit: no reference remains in the example. Falsifying exit: generated files are hand-edited instead of regenerated.

## Step 13c — Retire the verifier expectations that hardcode the package

- Task IDs: TASK-577. Objective: make the WIT verifier and its snapshot tests agree with the delivered tree.
- Precondition: Steps 13a/13b applied.
- Postcondition: `SHARED_PACKAGES` no longer lists the package; the expected include-str set, watch-list expectations, difference array and inline package snippets no longer name it; the dragon-curve snapshot pair and the runtime drift test no longer assert the method.
- Allowed reads: `xtask/src/wit_verify.rs` windows around each listed expectation; the two test files.
- Allowed edits: `xtask/src/wit_verify.rs`; `crates/slicer-schema/tests/dragon_curve_wit_snapshot_tdd.rs`; `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`.
- Out of bounds: the real WIT files (already done), other crates.
- Dispatch: list every assertion that referenced the package and confirm each is retargeted or removed with its contract retired; return LOCATIONS ≤ 20.
- Context: M. Authorities: docs03, docs22.
- Narrow verification: `cargo test -p xtask` with tee, then `rg '^test result:'`; plus `cargo test -p slicer-schema` for the snapshot pair.
- Exit: verifier tests pass against the real tree. Falsifying exit: an assertion is deleted while the invariant it protected still exists.

## Step 14a — Retarget the runtime contract binary

- Task IDs: TASK-577. Objective: repair one test **binary** at a time. `cargo test --test contract` compiles every module in that binary, so a binary is the smallest unit that can go green.
- Precondition: Steps 12–13 applied.
- Postcondition: the runtime contract binary compiles and passes: round-trip tests retired as scaffolding, `integrated_parity_lightning_infill_tdd` asserting against the prepared path, `wit_drift_detection_tdd` no longer asserting the removed method, `tests/contract/main.rs` listing exactly the files on disk, and `tests/common/mod.rs` free of the tree literal its `mod common` declaration pulls in.
- Allowed reads: the six files below.
- Allowed edits: `crates/slicer-runtime/tests/contract/integrated_parity_lightning_infill_tdd.rs`; `crates/slicer-runtime/tests/contract/lightning_tree_per_region_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`; `crates/slicer-runtime/tests/contract/main.rs`; `crates/slicer-runtime/tests/common/mod.rs`. **Six-file step by design**: the binary compiles as a unit, and `main.rs` pulls `common` in with it, so deferring any of these leaves the step's own command red.
- Out of bounds: production runtime, the other test binaries.
- Dispatch: confirm no parity assertion is weakened by the move and no identity check degrades to a count; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs22, docs02, ADR-0073.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the binary passes with a nonzero count. Falsifying exit: a parity test injects a pre-committed piece payload, or a test survives only as `#[ignore]`.

## Step 14b — Retarget the runtime executor and integration binaries

- Task IDs: TASK-577. Objective: repair the other two runtime test binaries and the shared helper they use.
- Precondition: Step 14a applied.
- Postcondition: `--test executor` and `--test integration` compile and pass: `lightning_pipeline_linked_tdd` retired as scaffolding, `source_identity_repair_tdd` asserting identity through the prepared path, `infill_partitioned_input_tdd` constructing no tree, and both `main.rs` lists matching disk.
- Allowed reads: the five files below.
- Allowed edits: `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs`; `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs`; `crates/slicer-runtime/tests/executor/main.rs`; `crates/slicer-runtime/tests/integration/infill_partitioned_input_tdd.rs`; `crates/slicer-runtime/tests/integration/main.rs`. **Five-file step by design**, same reason as 14a.
- Out of bounds: production code, wasm-host tests.
- Dispatch: confirm source-identity assertions survive as identity, not length, checks; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs22, docs02/04.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; cargo test -p slicer-runtime --test integration 2>&1 | tee -a target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: both binaries pass with nonzero counts. Falsifying exit: a source-identity assertion degrades to a length check.

## Step 14c — Retarget the wasm-host contract binary

- Task IDs: TASK-577. Objective: repair the wasm-host contract binary, which compiles as one unit.
- Precondition: Steps 11c/11d applied.
- Postcondition: `--test contract` compiles and passes with all thirteen modules and `main.rs` retargeted: dispatch keying and guest-accessor tests assert through the prepared path; identity, selection, parity and view-seam tests keep their populations; no module constructs a tree literal.
- Allowed reads: the fourteen files below.
- Allowed edits: `crates/slicer-wasm-host/tests/contract/{authored_coloring_grant_and_strip_tdd, infill_holder_resolution_painted_region_tdd, infill_postprocess_view_identity_tdd, lightning_dispatch_per_region_keying_tdd, lightning_infill_guest_calls_lightning_tree_segments_tdd, region_eligibility_tdd, sdk_view_transport_parity_tdd, shared_selection_dispatch_tdd, source_identity_repair_tdd, support_identity_layer_dispatch_tdd, variant_identity_accessors_tdd, variant_identity_delivery_tdd, view_seam_identity_tdd}.rs` and `crates/slicer-wasm-host/tests/contract/main.rs`. **Fourteen-file step by design**: a single unrepaired module keeps the whole binary from compiling.
- Out of bounds: production wasm-host code, the standalone `raft_plan_read_accessor_tdd` target (Step 14d).
- Dispatch: for each module, name the retained contract and asserted population; return SUMMARY ≤ 300 words.
- Context: M. Authorities: docs22, docs03, ADR-0073.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-wasm-host --test contract 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the binary passes. Falsifying exit: a module is emptied while its contract still exists elsewhere.

## Step 14d — Retarget the standalone raft accessor target

- Task IDs: TASK-577. Objective: repair the one auto-discovered wasm-host test target that is not part of `--test contract`.
- Precondition: Step 14c applied.
- Postcondition: `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs` constructs no tree input and keeps its accessor assertions against the surviving surface.
- Allowed reads: that file.
- Allowed edits: `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs`.
- Out of bounds: production code, the contract binary.
- Dispatch: name the contract the file protects after the change; return FACT.
- Context: S. Authorities: docs22.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the target passes. Falsifying exit: the file is deleted while its accessor contract still exists.

## Step 14e — Prove the test tree is clean

- Task IDs: TASK-577. Objective: close the test phase with a whole-tree check rather than a per-file claim.
- Precondition: Steps 14a–14d applied.
- Postcondition: `cargo check --workspace --all-targets` passes and the `rg` closure for the four legacy symbols over `crates/**/tests/**` and `modules/**/tests/**` returns zero hits.
- Allowed reads: the closure command; nothing else.
- Allowed edits: **none**. Any remaining hit is a defect in Steps 14a–14d, returned to the owning step.
- Out of bounds: every source file.
- Dispatch: return the closure result and the check exit as FACT.
- Context: S. Authorities: docs22, docs21.
- Narrow verification: `cargo check --workspace --all-targets` and `rg -l 'LightningTreeIR|LightningTreeEntry|PrePass::LightningTreeGen|lightning_tree_segments_for|lightning_tree_ir|with_lightning_tree_ir|LightningQueryViewData|lightning-queries' --glob 'crates/**/tests/**' --glob 'modules/**/tests/**'` returning no paths.
- Exit: both green. Falsifying exit: an `#[ignore]`d test keeps a reference compiling.

## Step 15a — Replace live legacy guidance (architecture, IR, WIT)

- Task IDs: TASK-577. Objective: make the docs describe the migrated contract.
- Precondition: Steps 10–14 applied and green.
- Postcondition: `docs/01_system_architecture.md` carries `### Lightning private preparation migration`; `docs/02_ir_schemas.md` carries `### Lightning legacy IR retirement`; `docs/03_wit_and_manifest.md` carries `### Lightning accessor retirement`, including the note that `docs/feasibility-probes/` is a stale historical mirror outside the live contract.
- Allowed reads: the current lightning sections of those docs.
- Allowed edits: those three files.
- Out of bounds: historical packet prose, ADR records (amended during authoring), `docs/feasibility-probes/**`.
- Dispatch: confirm each heading exists exactly once and no stale legacy symbol remains in the replaced section; return FACT.
- Context: S. Authorities: docs00 map, ADR-0066/0029 as amended.
- Narrow verification: the documentation-placement bullet marked "Step 15a" in `packet.spec.md` `## Verification`.
- Exit: the grep passes. Falsifying exit: the heading is added while the old text asserting the legacy path stays.

## Step 15b — Replace live legacy guidance (scheduler, SDK, visual debug)

- Task IDs: TASK-577. Objective: same for the remaining three docs.
- Precondition: Step 15a applied.
- Postcondition: `docs/04_host_scheduler.md` carries `### Lightning selection-driven preparation`; `docs/05_module_sdk.md` carries `### Lightning prepared module consumption`; `docs/19_visual_debug.md` carries `### Lightning private preparation migration` describing the `planning_geometry` view and `lightning-plan` scene.
- Allowed reads: the current lightning sections of those docs.
- Allowed edits: those three files.
- Out of bounds: as Step 15a.
- Dispatch: same placement check; return FACT.
- Context: S. Authorities: docs04/05/19.
- Narrow verification: the documentation-placement bullet marked "Step 15b" in `packet.spec.md` `## Verification`.
- Exit: greps pass. Falsifying exit: the visual-debug section documents a renderer that does not exist.

## Step 16 — Closure and ledger reconciliation

- Task IDs: TASK-577. Objective: verify the whole packet and record the re-scope truthfully.
- Precondition: every prior step green; guests fresh.
- Postcondition: nine AC commands plus the aggregate `9 passed` pass; `cargo check --workspace --all-targets` and `clippy --all-targets` clean (the packet-scope re-run of the check Step 14e first ran for the test tree); `check-literals` and `check-test-quality --report` findings resolved or waived with a named surface; the source plan's row-08 note and the `docs/07` `TASK-577` line state the equivalence premise and the ADR amendments. Both ledger edits are re-derived at edit time.
- Allowed reads: `packet.spec.md` verification section; the two ledger windows.
- Allowed edits: `docs/specs/layer-module-preparation-plan.md`; `docs/07_implementation_status.md`; `packet.spec.md` (status only, on explicit finalization approval).
- Out of bounds: other packets, the ADR text (amended during authoring), gate outcomes for rows 07/09.
- Dispatch: run each AC command through an execution worker; return FACT pass/fail with bounded failure text.
- Context: M. Authorities: `packet.spec.md`, docs21/22.
- Narrow verification: the `## Verification` list in `packet.spec.md`.
- Exit: everything green and both ledger lines state the equivalence premise without claiming parity. Falsifying exit: a ledger line implies Gate A/B now pass, or the packet claims canonical parity.

## Budget and Completion

| Steps | Context | Scope |
| --- | --- | --- |
| 1, 2 | M | Baseline + harness |
| 3, 4, 5 | S/M | Kernel scaffold, move, input adaptation |
| 6, 7a | S/M | Kernel tests, piece codec |
| 7b, 8 | M | Preparation, consumers/projection |
| 9 | S | Module tests |
| 10a–10c | S | Prepass wiring, producer, host algorithm copy |
| 11a–11d | M | SDK accessor, stage-input field blast radius, macros, test guest |
| 12a, 12b | M | Plan IR, stage identity |
| 13a–13c | S/M | WIT package, example module, verifier expectations |
| 14a–14e | M | Test fallout |
| 15a, 15b | S | Docs |
| 16 | M | Closure + ledger |

No step is rated L. Four steps deliberately exceed the three-file convention because their change is atomic: 11c (a struct field and every initializer/reader), 12b (a stage and the roster test asserting it), 13b (the example module's generator input set), and 13c's verifier sweep. Each says so in place; a transitional shim is never the alternative. The step count follows the retirement's real breadth — a public IR, a scheduler stage, a WIT package with hardcoded verifier expectations, and roughly thirty test files. Packet completion requires every AC, the negative controls, guest freshness and an independent closure review.
