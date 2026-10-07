# Implementation Plan: shared-selection-prerequisite

## Execution rules

Do not execute until independent preflight and explicit activation. Work one atomic step at a time, at most three edited files. New interface test compilation failures are scaffolding, not behavioral red. Preserve independently expected identities/configs/roles and positive nonempty guards. Every test must tee combined output to `target/test-output.log`; inspect that saved file before overwriting it. Create `target/` before the first run.

ONE explicit intermediate field/helper compile gap starts in 2C and spans 2D–2X; Step 2Y closes it with mandatory all-target verification. Bounded migration reviews are not interim green compile/behavior gates. Step 2O's shared parity metadata channel completes the earlier parity-caller scaffolding before that gate. No workspace suite is run.

All steps map to TASK-572, merged queue row 01. All reads below are symbol-local ±40-line windows in long files, not full-file loads. Heavy commands and normative fact-checks are delegated. Every dispatch returns FACT ≤5 lines (or ≤20 compiler/failure lines if failing), outside thinking, properly escaped, redundantly naming verdict, execution count and log path. No Orca refs apply. No step may touch source plan, backlog, other packets, producer/WIT/guest contracts, preparation APIs or global error-policy text.

## Steps

### Step 1 — Register delivery red and correct painted identity

- Task IDs: TASK-572, row 01.
- Objective: reproduce the ordinary full-identity transport defect on production runner paths (AC-1/2/3/N1/N2).
- Precondition: activation approved; ordinary guest freshness exit 0. Stale/infrastructure artifacts are not bug evidence.
- Postcondition: five family-A tests are registered; one width-delivery test fails behaviorally before repair. Existing painted map/slice identity agrees.
- Allowed reads: `crates/slicer-wasm-host/tests/contract/infill_holder_resolution_painted_region_tdd.rs::{build_module_config,build_slice_ir,build_painted_region_map,run_infill_stage}`; `tests/raft_plan_read_accessor_tdd.rs::run_native_entry`; `tests/common/wasm_cache.rs` loader signatures; `crates/slicer-wasm-host/src/{binding,traits}.rs` runner borrow/entry symbols; `crates/slicer-ir/src/{slice_ir,stage_io}.rs` key/interner/source/commit shapes; `crates/slicer-sdk/src/{native,views}.rs` entry/envelope/accessors. Bounded symbol windows only.
- Allowed edits (three): NET-NEW `crates/slicer-wasm-host/tests/contract/variant_identity_delivery_tdd.rs`; `crates/slicer-wasm-host/tests/contract/main.rs` registration; `crates/slicer-wasm-host/tests/contract/infill_holder_resolution_painted_region_tdd.rs` matching chain and misleading comment correction only.
- Out of bounds: all production code, source/WIT/version changes, accessor/targeting expectations and guest algorithms.
- Blast radius: no shape/version change. Use FRU or protected-contract exhaustive waivers; build exact chains into map and slice. Five test names exactly match family-A commands; no missing artifact skip. AC-N1 owns host builder rejection for both partial/absent maps (inner exact missing key plus wrapper stage/module), and scheduled-module `selection: None` dispatcher fatal rejection on both legs, with same-setup complete-index positive invocation/nonempty-commit controls. Runtime mapping is AC-B4/Step 8, not this wasm-host fixture. Missing-map rejection supersedes the old fallback draft; new interface compile failures remain scaffolding, not behavioral red.
- Dispatch: question "does the distinct-config test execute once and fail on wrong delivered config with fresh guests?"; scope exact red command and bounded saved assertion; return FACT.
- Context: M. Authorities: docs/02 Config Interner, docs/21 §1, docs/22 §§1–2, delegated. Orca: none.
- Verification: `cargo xtask build-guests --check`; then `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract variant_identity_delivery_tdd::distinct_variant_configs_reach_both_legs -- --exact 2>&1 | tee target/test-output.log >/dev/null; result=$?; python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'running 1 test' in s and 'distinct_variant_configs_reach_both_legs ... FAILED' in s; print('FACT EXPECTED_RED: inspect saved delivery assertion')" && test "$result" -ne 0`.
- Exit: exactly one named test ran and its delivery assertion falsifies current width/identity handling. Compile failure, zero tests, guest instantiation failure or unrelated panic is not an exit.

### Step 2 — Introduce the public whole-print selection index

- Task IDs: TASK-572, row 01.
- Objective: implement `build_layer_selection_index(LayerSelectionIndexInput<'_>) -> Result<LayerSelectionIndex, LayerSelectionBuildError>`, `LayerSelectionIndex::selection_for`, module metadata input, unchanged authority/inner-error/presence shapes and reusable source rules for family B. NET-NEW `pub struct LayerSelectionBuildError { pub error: SelectionError, pub stage_id: String, pub module_id: String }` in `crates/slicer-wasm-host/src/selection.rs` carries failing module/stage for either unchanged inner error variant.
- Precondition: Step 1 behavioral red recorded; shapes in packet.spec verified against current IR.
- Postcondition: whole-print construction covers every committed layer/bound module pair, including noninvoking entries; exact model/perimeter config and explicit non-region context are resolved once. `selection_for` is O(1), with no reconstruction. A per-layer routine is private to the builder. The index is selection metadata only; `slice_regions` records build-time identity/eligibility, never delivered payload geometry. Preserve merged membership for full-identity association to current products at transport time.
- Roster channel: `pub use slicer_schema::{StageSpec, STAGES};` in `crates/slicer-wasm-host/src/selection.rs`; runtime supplies `slicer_wasm_host::selection::STAGES`, never names `slicer_schema` directly, and needs no manifest edit.
- Allowed reads: `crates/slicer-wasm-host/src/dispatch.rs::{support_carrier_regions,module_receives_slice_region}`; `marshal/mod.rs::perimeter_source_regions`; `crates/slicer-scheduler/src/{execution_plan,validation}.rs` binding/family matcher/static accessors/held claims; `crates/slicer-schema/src/lib.rs::{STAGES,StageSpec,stage_by_id,TIER_LAYER}`; `crates/slicer-ir/src/slice_ir.rs` full key/interner/support/raft/source types. Each located symbol ±40 lines.
- Allowed edits (three): NET-NEW `crates/slicer-wasm-host/src/selection.rs`; `crates/slicer-wasm-host/src/lib.rs` public module; `crates/slicer-wasm-host/src/dispatch.rs` move carrier implementation behind a delegating wrapper only, not live dispatch integration yet.
- Out of bounds: existing struct fields, runtime consumer, host/native delivery and coloring policy, schema stage roster, geometry merge helper body.
- Blast radius: new index/input/result structs have no existing literals. Reused carrier wrapper keeps its old signature. No WIT/schema/version edits; `SupportPlanEntry` remains unbound to model config. Inline unit test `selection::tests::model_identity_and_non_region_authority` uses independently expected keys, configs, support presence, schema-derived stage set, complete pair coverage and repeat accessor observations; unit evidence does not discharge production ACs. No all-region/claim/carrier scan occurs in the accessor.
- Dispatch: question "does public seam preserve exact authority, explicit absence and supported schema set?"; scope new inline test plus all-target check; return FACT.
- Context: M. Authorities: source plan selection section, docs/02 interner, docs/03 declared reads, ADR-0056 Decision 1/3. Orca: none.
- Verification: `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --lib selection::tests::model_identity_and_non_region_authority -- --exact 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 1 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 1 executed')"`; delegated `mkdir -p target; cargo check --workspace --all-targets` saved to `target/shared-selection-check.log` with FACT exit.
- Exit: named test executes/pass and all-target check exits 0; fabricated non-region key, exact-model fallback, stage roster duplication or lost support-entry provenance falsifies the step.

### Step 2A — Commit the immutable index product on Blackboard

- Task IDs: TASK-572, row 01.
- Objective: provide legal runtime storage without extending scheduler `ExecutionPlan`.
- Precondition: Step 2 whole-print API compiles; `Blackboard::new` constructor and existing write-once product pattern verified.
- Postcondition: ONE `Option<Arc<LayerSelectionIndex>>` product is initialized to `None`; NET-NEW `layer_selection_index` and `commit_layer_selection_index` provide immutable access/write-once commit. A second commit returns a runtime-local error for the existing fatal boundary; do not add IR error/slot variants or falsely label the product with another slot.
- Allowed reads: `Blackboard::{new,commit_region_map,region_map,commit_slice_ir,slice_ir}` and `commit_prepass` (`crates/slicer-runtime/src/blackboard.rs`); `BlackboardPrepassSlot`/`BlackboardError` (`crates/slicer-ir/src/stage_io.rs`) read-only; `LayerSelectionIndex` (`crates/slicer-wasm-host/src/selection.rs`). Symbol-local windows only.
- Allowed edits (one): `crates/slicer-runtime/src/blackboard.rs` product, constructor, accessor/commit and local tests only.
- Out of bounds: scheduler fields, input fields, WIT/IR/SDK, unrelated product lifecycle.
- Blast radius: ONE Blackboard product, no external direct struct-literal migration; do not change existing product semantics.
- Dispatch: question "does write-once storage compile without scheduler-to-wasm-host coupling?"; scope all-target check and constructor/commit review; return FACT.
- Context: S. Authorities: docs/04 immutable product/complexity contract, docs/21. Orca: none.
- Verification: delegated all-target check saved with Step 3's exact check wrapper; symbol-local review of initialization and second-commit rejection.
- Exit: all-target check exits 0; index ownership is immutable and no partial commit or ExecutionPlan field exists.

### Step 2B — Build once at every production late-bind seam

- Task IDs: TASK-572, row 01.
- Objective: finish selection after late PrePass and promotion, before any Layer invocation.
- Precondition: Steps 2/2A APIs compile; both pipeline promotion paths and `prepare_prepass_context` promotion verified.
- Postcondition: NET-NEW `pub fn build_and_commit_layer_selection_index(plan: &ExecutionPlan, blackboard: &mut Blackboard) -> Result<(), LayerStageError>` (`crates/slicer-runtime/src/layer_executor.rs`) gathers schema/static module metadata and committed whole-print products after `promote_global_layers`, builds ONCE, and commits only success. It is public so runtime test fixtures driving executor entry points directly use this SAME production build/commit entry after seeding `slice_ir` and any exact map data, before execution/capture; no test-local gathering or second resolver. Both pipeline paths and the prepared-context path invoke it before layer execution/return. ANY `LayerSelectionBuildError` maps to `LayerStageError::FatalModule { stage_id, module_id, message }` (`crates/slicer-ir/src/stage_io.rs`) using wrapper stage/module and inner error rendering, including exact `RegionKey` for missing model/perimeter config, before ANY Layer invocation; no inner-error-enum expansion or per-layer retry.
- Allowed reads: `promote_global_layers`, `execute_single_layer_inner`, `execute_captured_stages_with_support_tools` (`crates/slicer-runtime/src/layer_executor.rs`); promotion callers in `crates/slicer-runtime/src/pipeline.rs`; `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`); compiled-stage/static-module accessors in `crates/slicer-scheduler/src/execution_plan.rs`; Blackboard products and whole-print input shapes. Bounded symbol windows only.
- Allowed edits (three): `crates/slicer-runtime/src/layer_executor.rs` build/commit helper only; `crates/slicer-runtime/src/pipeline.rs` both post-promotion calls/error propagation; `crates/slicer-runtime/src/run.rs` prepared-context post-promotion call/error propagation.
- Out of bounds: `promote_global_layers` signature, plan-compile scheduling, runner trait, independent per-call resolvers, global error policy, fixture expectations.
- Blast radius: no new scheduler field; existing empty compile-time layer lists are unchanged. Diagnostics obtain stage/module from `LayerSelectionBuildError`, retaining exact key in the inner error without changing `SelectionError::{UnknownLayerStage,MissingModelConfig}`. Missing bound pairs are fatal, never silent defaults.
- Dispatch: question "does every production late-bind caller build/commit once and reject missing authority before Layer execution?"; scope caller/error review and all-target check; return FACT.
- Context: M. Authorities: docs/04 `resolve_active_regions` Complexity Contract, docs/02 Config Interner, ADR-0056. Orca: none.
- Verification: delegated all-target check saved with Step 3's exact check wrapper; production missing-authority fatal mapping is proved by AC-B4's unchanged runtime command in Step 8, while AC-N1 proves only host build/dispatch rejection.
- Exit: all-target compile passes; every named production seam is wired and committed index covers all bound pairs; no build at plan compile or per-layer call.

### Step 2C — Add the input field and consume indexed runtime eligibility

- Task IDs: TASK-572, row 01.
- Objective: replace both ordinary runtime gates with O(1) indexed decisions and pass call-local selection without changing `run_stage`.
- Precondition: Step 2B build/commit seam wired; existing literal inventory in design verified again.
- Postcondition: `LayerStageInput<'a>` gains `selection: Option<&'a LayerSelection>`; both production constructions in `execute_single_layer_inner` and `execute_captured_stages_with_support_tools` borrow `selection_for(layer.index, module_id)` from Blackboard and use `invoke`. Paint/family/carrier gates are removed here; `module_invocation_allowed_on_layer` delegates `selection::paint_allows_layer`. Missing index/pair is fatal.
- Allowed reads: `LayerStageInput` (`crates/slicer-wasm-host/src/binding.rs`), `LayerStageRunner::run_stage` (`crates/slicer-wasm-host/src/traits.rs`), both named executor branches, Blackboard/index accessors, runtime common fixture input constructor. Symbol-local windows only.
- Allowed edits (three): `crates/slicer-wasm-host/src/binding.rs`; `crates/slicer-runtime/src/layer_executor.rs` two input constructions/eligibility/compatibility wrapper only; `crates/slicer-runtime/tests/common/mod.rs` one existing input literal and necessary index fixture setup.
- Out of bounds: runner signature, independent selection/config resolution, scheduler/WIT/SDK changes, instrumentation/synthetic anchored semantics.
- Blast radius: ONE existing wasm-host field addition and both production literals; docs/21 FRU/waiver discipline applies to tests. Every direct `run_stage` caller supplies held indexed `Some`; `None` is confined to non-dispatching direct builder/accessor fixtures and the AC-N1 rejection control. Step 2G completes selection-aware runtime common helpers; Steps 2G–2S migrate every runtime direct-dispatch/helper-mediated fixture site, not only the literal; Steps 2U–2X provision the executor-driven fixture class through the public production build/commit entry.
- Dispatch: question "do both eligibility sites perform keyed lookup and pass the borrowed selection without changing the trait?"; scope these files and literal inventory; return FACT.
- Context: M. Authorities: docs/04 complexity, docs/21, ADR-0056. Orca: none.
- Verification: bounded source/borrow review and inventory FACT; all-target compile gate is deliberately deferred until Steps 2D–2X complete enumerated field/helper and executor-provisioning migration. Interim missing-field/helper-channel compiler errors are tracked scaffolding, not behavioral red or a green gate.
- Exit: all three allowed files reconciled; both live paths consume indexed `invoke`; outstanding wasm-host literals are owned by 2D–2F, runtime dispatcher/helper sites by 2G–2S and executor-driven fixtures by 2U–2X. Final compile acceptance remains mandatory in Step 2Y.

### Step 2D — Migrate raft, eligibility and lightning input literals

- Task IDs: TASK-572, row 01.
- Objective: update the first bounded batch of existing test input constructions.
- Precondition: Step 2C field exists; tests' production/direct transport boundaries identified.
- Postcondition: four existing literals across three files have explicit selection: raft's two and lightning's one reach `run_stage` and pass held indexed `Some`; eligibility's one is non-dispatching builder-only and may use `None`, without changing old builder arity.
- Allowed reads: each listed input/runner fixture, public index input/accessor, `LayerStageInput` definition; bounded symbol windows.
- Allowed edits (three): `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs` (2 literals); `crates/slicer-wasm-host/tests/contract/region_eligibility_tdd.rs` (1); `crates/slicer-wasm-host/tests/contract/lightning_infill_guest_calls_lightning_tree_segments_tdd.rs` (1).
- Out of bounds: production source, guest source, geometry/assertion changes or fixture skips.
- Blast radius: field-only/necessary committed-product index fixture setup; preserve every existing role/config/output assertion and apply docs/21 FRU or justified exhaustive waiver.
- Dispatch: question "are all four literals initialized with valid production selections and unchanged assertions?"; scope three fixture windows/inventory; return FACT.
- Context: S. Authorities: docs/21/22, docs/04 indexed consumption. Orca: none.
- Verification: literal/fixture review; all-target compile remains deferred to Step 2Y and named matrix commands to Step 5/11 after dispatch wiring.
- Exit: these four sites migrated; no missing-field site hidden and no claim of interim all-target pass.

### Step 2E — Migrate postprocess, SDK parity and support input literals

- Task IDs: TASK-572, row 01.
- Objective: update the second bounded batch without weakening full-identity/support coverage.
- Precondition: Steps 2C/2D complete; production versus transport-only sites identified.
- Postcondition: six existing input literals initialize the new field: postprocess-view identity's four are non-dispatching builder-only (`None` legal); SDK parity's one and support identity's one reach `run_stage` and require held indexed `Some`.
- Allowed reads: listed test fixture input/runner symbols, public index/selection APIs, common fixture patterns; bounded windows only.
- Allowed edits (three): `crates/slicer-wasm-host/tests/contract/infill_postprocess_view_identity_tdd.rs` (4 literals); `crates/slicer-wasm-host/tests/contract/sdk_view_transport_parity_tdd.rs` (1); `crates/slicer-wasm-host/tests/contract/support_identity_layer_dispatch_tdd.rs` (1).
- Out of bounds: production/guest source, test expectation changes, skips/default config fallback.
- Blast radius: input-field and required index fixture plumbing only; preserve all existing identity/perimeter/support assertions; docs/21 discipline applies.
- Dispatch: question "do these six sites retain independent identity/support assertions with valid selection lifetimes?"; scope listed files and inventory; return FACT.
- Context: M. Authorities: docs/21/22, docs/02 exact authority. Orca: none.
- Verification: literal/fixture review; combined compile gate remains deferred to Step 2Y, and production fixture behavior to Step 5/11 after dispatcher consumes the field.
- Exit: all six listed sites migrated with no hidden required fourth file or weakened assertion.

### Step 2F — Finish wasm-host input-literal migration

- Task IDs: TASK-572, row 01.
- Objective: close wasm-host literal churn, including Step-1-created delivery fixtures; defer the mandatory compile gate until runtime migration completes.
- Precondition: Steps 2C–2E completed; Step 1 red evidence saved before any fixture rewrite.
- Postcondition: painted holder's one existing literal and new delivery dispatch literals use held indexed `Some`; seam identity's two are non-dispatching builder-only (`None` legal). Test index construction uses exact committed inputs/static metadata, not a second resolver. Runtime helper/caller migration remains explicit scaffolding owned by 2G–2S.
- Allowed reads: input/runner fixtures in the three listed files; index metadata/config/borrow types; saved Step 1 red; current literal inventory. Bounded symbol windows only.
- Allowed edits (three): `crates/slicer-wasm-host/tests/contract/infill_holder_resolution_painted_region_tdd.rs` (1 existing literal); `crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs` (2); `crates/slicer-wasm-host/tests/contract/variant_identity_delivery_tdd.rs` (NET-NEW from Step 1, any new literals/index setup).
- Out of bounds: production changes, saved red evidence, weakening missing-config/unknown-holder/order/width assertions, guest/SDK/IR/WIT changes.
- Blast radius: thirteen pre-existing wasm-host test literals in eight files, one runtime common literal and two production constructions are accounted for across Steps 2C–2F; fourteen/nine was the combined test count. New fixture literals also obey docs/21. No runner trait/signature change; every dispatcher caller requires `Some`, except the deliberate AC-N1 rejection control.
- Dispatch: question "does wasm-host literal inventory reconcile with thirteen/eight and correct dispatcher/builder classification?"; scope `rg -n 'LayerStageInput\s*\{' crates` and bounded fixture review; return FACT with exact sites, classification and deferred-gate ownership.
- Context: M. Authorities: docs/21/22, docs/04 complexity, docs/02 interner. Orca: none.
- Verification: literal inventory and assertion-preservation review; mandatory all-target check with Step 3's exact saved-log wrapper is deferred to Step 2Y after 2G–2X. Run unchanged matrix commands after Step 5 wiring; no interim compile/behavioral-green claim.
- Exit: all enumerated wasm-host sites initialized/classified, every dispatcher fixture has held indexed `Some` except AC-N1, no unlisted edit; Step 1's original behavioral red remains recorded. Final all-target acceptance is mandatory in Step 2Y, not claimed here.

### Step 2G — Add selection-aware runtime helpers and anchored direct callers

- Task IDs: TASK-572, row 01.
- Objective: define the single runtime test-side selection channel and migrate both omitted no-product dispatcher fixtures.
- Precondition: 2C–2F field/wasm-host migration reviewed; public builder/storage shapes exist; design inventory re-derived.
- Postcondition: `layer_input` accepts a borrowed indexed selection; common direct-dispatch helpers accept stage/layer/module identity, claims/config/static paint metadata and committed products, build ONLY through public `build_layer_selection_index`, and hold the index across `run_stage`. `run_layer_and_commit_with_bundle`/`run_layer_and_commit` construct selection internally from bundle static metadata. Both anchored fixtures pass `Some`; absent products remain absent, never fabricated maps/model keys. The executor-side provisioning contract is part of this same channel decision: fixtures driving executor entry points directly use public production `build_and_commit_layer_selection_index` (`crates/slicer-runtime/src/layer_executor.rs`), never test-local metadata gathering that could drift. Their fixture edits belong only to Steps 2U–2X, not this step's three-file batch.
- Allowed reads: `layer_input`, `TestModuleBundle::as_live`, `run_layer_and_commit_with_bundle` (`crates/slicer-runtime/tests/common/mod.rs`); the two listed fixtures' dispatcher/module/layer setup; `LayerSelectionIndexInput`, `LayerSelectionModuleInput`, `build_layer_selection_index`, `selection_for` (`crates/slicer-wasm-host/src/selection.rs`); `CompiledModuleStatic::region_split_semantics` (`crates/slicer-scheduler/src/execution_plan.rs`). Symbol-local ±40-line windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/common/mod.rs`; `crates/slicer-runtime/tests/executor/anchored_events_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/executor/support_anchored_reach_tdd.rs`.
- Out of bounds: production code, second resolver, manual selected entries, implicit config fallback/map seeding, assertions, guest/IR/WIT/SDK/signature changes.
- Blast radius: common helper's existing literal/internal value site and two one-site fixtures; FRU/docs-21 or justified exhaustive waiver. Preserve every assertion; minimal exact-map fixture data is allowed when needed, but these two no-product setups require no model entries. Runtime uses only wasm-host's roster re-export.
- Dispatch: question "do helpers build only through the public seam and both no-product direct callers supply held Some?"; scope named symbols/lifetimes and inventory; return FACT outside thinking, escaped, redundantly naming verdict/site counts/deferred log gate.
- Context: M. Authorities: docs/21/22, docs/02 exact interner, docs/04 complexity. Orca: none.
- Verification: bounded helper/borrow review, reconcile 52 value sites/25 files and zero non-dispatching callers; no cargo gate at this scaffolding step.
- Exit: single public-builder test channel owns selection/lifetimes; both callers migrated with unchanged assertions; remaining dispatch/helper sites explicitly owned by 2H–2S, executor-driven provisioning by 2U–2X, compile gate 2Y.

### Step 2H — Migrate protocol, macro and slice-adaptation direct callers

- Task IDs: TASK-572, row 01.
- Objective: migrate eleven direct sites without altering protocol or completeness evidence.
- Precondition: 2G common helper contract available; current stage/module/product setup located.
- Postcondition: all four protocol, six macro and one adaptation sites pass held indexed `Some`; nonempty selected slice/perimeter identities have minimal exact committed map data aligned with original module-wide config, not implicit helper fallback.
- Allowed reads: `layer_input`/`run_stage` fixture windows in the three listed files; common selection helper/public builder input and exact key/interner shapes, symbol-local ±40 lines only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs`; `crates/slicer-runtime/tests/contract/macro_all_worlds_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/integration/adapt_slice_regions_completeness_tdd.rs`.
- Out of bounds: production/guest code, resolver duplication, protocol/completeness expectations, assertion weakening or skips.
- Blast radius: eleven helper-call sites; preserve every assertion/positive guard; minimal exact map data only, FRU/docs-21 discipline, no new public field/signature.
- Dispatch: question "do all eleven dispatcher sites supply Some without changing protocol/completeness assertions?"; scope listed windows/fixture data; return escaped FACT outside thinking with verdict/count and deferred 2Y gate.
- Context: M. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded site/config/borrow review; compile and runtime-bin evidence deferred to 2Y.
- Exit: eleven sites reconciled, exact products supplied where demanded, assertions unchanged; no fourth edit.

### Step 2I — Migrate live seam/support and native infill claims

- Task IDs: TASK-572, row 01.
- Objective: migrate three direct sites and the two live files' helper-mediated fixture data.
- Precondition: 2G helper channel exists; real committed slice/perimeter identities and original config/claims inspected.
- Postcondition: each direct site supplies held `Some`; live seam/support bundle-helper calls use internal selection with minimal exact maps for their selected model/perimeter identities. Native infill exact holder/config data retains its claim assertions.
- Allowed reads: listed files' `layer_input`, `run_stage`, `run_layer_and_commit`/`run_layer_and_commit_with_bundle`, slice/perimeter/module/config fixture symbols; common helper/public builder, ±40-line symbol windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/executor/live_seam_path_tdd.rs`; `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`; `crates/slicer-runtime/tests/contract/native_infill_claim_resolution_tdd.rs`.
- Out of bounds: production/guest code, seam/support/claim expectation changes, assertion weakening, skips or fallback.
- Blast radius: three direct value sites plus helper-mediated data in these same files; preserve every assertion, geometry and absent-plan control. Minimal exact committed maps aligned with original fixture config are allowed; FRU/docs-21 discipline.
- Dispatch: question "are live direct/helper paths selection-complete while seam/support/holder witnesses stay intact?"; scope listed symbols; return escaped FACT outside thinking with verdict/site count/deferred 2Y gate.
- Context: M. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded Some/lifetime/exact-map/assertion review; compile/runtime-bin gate deferred to 2Y.
- Exit: all three direct sites and both helper-mediated fixture surfaces reconciled without a fourth edit or changed expectation.

### Step 2J — Migrate native seam, raft and fuzzy-skin direct callers

- Task IDs: TASK-572, row 01.
- Objective: migrate eight direct sites, including fuzzy-skin's extra native-only control.
- Precondition: 2G common helpers exist; live/static metadata limitation of parity harness explicitly tracked until 2O.
- Postcondition: four native seam, one raft and three fuzzy sites use indexed `Some`; add minimal exact model/perimeter map data where selected authority is missing. Parity closures use the common helper's metadata channel completed in 2O.
- Allowed reads: listed files' input/runner, module/config and slice/perimeter setup; `run_integrated_parity` (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`) closure/builder windows and common helper, symbol-local ±40 lines only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs`; `crates/slicer-runtime/tests/integration/raft_geometry.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_fuzzy_skin_tdd.rs`.
- Out of bounds: harness edits here, production/guest code, parity/seam/raft expectations, assertions/skips/default authority.
- Blast radius: eight sites; preserve every assertion and geometry; minimal exact map fixture data allowed, raft remains explicit non-region context. FRU/docs-21 discipline; no static/live module-shape changes.
- Dispatch: question "do eight sites use the common selection channel and preserve existing controls?"; scope listed windows and tracked harness dependency; return escaped FACT outside thinking with verdict/count/deferred 2O/2Y gates.
- Context: M. Authorities: docs/21/22, docs/02 exact interner. Orca: none.
- Verification: bounded site/config/borrow review; harness scaffolding remains explicit, final compile/runtime evidence in 2Y.
- Exit: eight sites owned/reconciled, no unlisted edit; harness completion remains 2O's obligation.

### Step 2K — Migrate arachne, classic and gyroid parity callers

- Task IDs: TASK-572, row 01.
- Objective: migrate six parity dispatch sites through the single shared helper.
- Precondition: 2G helper exists; 2O metadata-channel scaffolding recorded, current committed config products inspected.
- Postcondition: both legs in each file hold indexed `Some` from the common helper through the parity closure; add minimal exact maps for arachne/classic, audit/preserve gyroid's existing map and config authority.
- Allowed reads: listed files' parity closure, input/runner and exact product/module-config symbols; common helper and `run_integrated_parity` (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`), ±40-line windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/integrated_parity_arachne_perimeters_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_classic_perimeters_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_gyroid_infill_tdd.rs`.
- Out of bounds: harness/production/guest edits, canonical algorithms, parity assertions, baselines/skips/default fallback.
- Blast radius: six sites; preserve every assertion and exact geometry inputs; minimal exact map fixture data aligned with original config allowed. FRU/docs-21 discipline; no second resolver.
- Dispatch: question "are six parity calls selection-complete without changing comparison assertions?"; scope listed closures/maps; return escaped FACT outside thinking with verdict/count/deferred gates.
- Context: M. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded site/map/assertion review; 2O completes shared metadata, 2Y owns compile/runtime evidence.
- Exit: six sites migrated with original or minimal exact products and unchanged assertions; no fourth edit.

### Step 2L — Migrate linker, lightning and path-optimization parity callers

- Task IDs: TASK-572, row 01.
- Objective: migrate six parity sites, including separately owned path-optimization arenas.
- Precondition: 2G helper available; 2O harness channel tracked; current maps/current arena products inspected.
- Postcondition: both legs use common-helper indexed `Some`; preserve/audit lightning's existing map, add minimal exact linker/path-optimization authority for selected slice/perimeter identities with original module config.
- Allowed reads: listed parity closures, input/runner and map/product setup; common helper and `run_integrated_parity` (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`), symbol-local ±40 lines only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/integrated_parity_infill_linker_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_lightning_infill_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_path_optimization_tdd.rs`.
- Out of bounds: harness/production/guest code, path/linking/comparator changes, assertions/baselines/skips or fallback.
- Blast radius: six sites, both current arena borrows retained; minimal exact map data allowed, preserve every assertion; FRU/docs-21 discipline.
- Dispatch: question "do six calls hold exact indexed authority without geometry/comparator drift?"; scope listed symbols; return escaped FACT outside thinking with verdict/count/deferred gates.
- Context: M. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded lifetime/map/assertion review, metadata completion 2O and compile/runtime evidence 2Y.
- Exit: six sites reconciled with no fourth edit or changed assertion.

### Step 2M — Migrate rectilinear, seam and support-ironing parity callers

- Task IDs: TASK-572, row 01.
- Objective: migrate seven sites, including seam's extra native-only control.
- Precondition: 2G helper exists; 2O metadata channel tracked; existing exact rectilinear map inspected.
- Postcondition: two rectilinear, three seam and two support-ironing sites supply common-helper held `Some`; retain rectilinear map, add minimal exact seam/support-ironing authority as required by selected products.
- Allowed reads: listed parity closure/control and input/runner/map/product symbols; common helper and `run_integrated_parity` (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`), ±40-line symbol windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/integrated_parity_rectilinear_infill_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_seam_placer_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs`.
- Out of bounds: harness/production/guest code, geometry/parity/control assertion changes, skips/baselines or fallback.
- Blast radius: seven sites; preserve every assertion including extra native control; minimal exact map fixture data allowed with original config; FRU/docs-21 discipline.
- Dispatch: question "do all seven calls including native-only control supply indexed Some?"; scope listed windows; return escaped FACT outside thinking with verdict/count/deferred gates.
- Context: M. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded site/map/assertion review; 2O harness completion, 2Y compile/runtime evidence.
- Exit: seven sites reconciled, exact products and original controls retained, no fourth edit.

### Step 2N — Migrate top ironing, traditional support and tree parity callers

- Task IDs: TASK-572, row 01.
- Objective: migrate six parity sites without changing support-family/optional-plan semantics.
- Precondition: 2G helper exists; the 2O harness 5-param closure lands first or in the same batch (compile gate remains 2Y); committed slice/perimeter/support fixture symbols located.
- Postcondition: each leg receives common-helper held `Some`; minimal exact map data covers selected model/perimeter keys with original module-wide config/support family. Optional support absence/presence and current arena geometry remain unchanged.
- Allowed reads: listed parity closures, module/config, input/runner and committed products; common helper and `run_integrated_parity` (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`), symbol-local ±40 lines only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_traditional_support_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_tree_support_tdd.rs`.
- Out of bounds: harness/production/guest code, support predicates, algorithm/comparator changes, assertion weakening/skips/fallback.
- Blast radius: six sites; minimal exact fixture maps allowed, no fabricated non-region model keys; preserve every assertion, FRU/docs-21 discipline.
- Dispatch: question "do six parity calls have exact indexed authority with unchanged support controls?"; scope listed windows; return escaped FACT outside thinking with verdict/count/deferred gates.
- Context: M. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded map/selection/lifetime/assertion review; 2O metadata completion, 2Y compile/runtime evidence.
- Exit: six sites reconciled without changed assertion or fourth edit.

### Step 2O — Finish wave parity and shared static-metadata channel

- Task IDs: TASK-572, row 01.
- Objective: migrate wave's two sites and finish the shared metadata addition used by all fourteen parity files.
- Precondition: 2J–2N caller scaffolding reviewed; public helper exists; harness manifest and both builder/closure bindings inspected.
- Postcondition: wave's existing exact map remains authoritative, both calls pass held `Some`. `run_integrated_parity` retains manifest-derived `region_split_semantics` in both `CompiledModuleBuilder` bindings and exposes static paint metadata to its closure or equivalent common-helper harness addition; both legs obtain entries ONLY via the public builder. Earlier parity caller scaffolding is reconciled, no hidden per-file resolver or omitted native-only control.
- Allowed reads: wave input/runner/map/config symbols; `IntegratedParitySpec`, `run_integrated_parity` (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`); common selection helper and manifest/static metadata accessor symbols, ±40-line windows only. Earlier fourteen parity closures are read-only bounded compatibility checks.
- Allowed edits (two): `crates/slicer-runtime/tests/contract/integrated_parity_wave_overhangs_tdd.rs`; `crates/slicer-runtime/tests/common/integrated_parity_harness.rs`.
- Bounded reconciliation (harness-wide arity change): every other `run_integrated_parity` caller adapts its closure parameter list only to the 5-param shape — 15 files: `infill_linker`, `lightning_infill`, `path_optimization`, `rectilinear_infill`, `seam_placer`, `support_surface_ironing` (in-scope; local selection-module construction retained) and `layer_planner`, `machine_gcode_emit`, `overhang_classifier`, `part_cooling`, `seam_planner`, `skirt_brim`, `support_planner`, `wipe_tower` (contract) plus `perimeter_spatial_capture` (integration). No other line in those 15 files changes. This reconciliation is required because the harness arity change breaks every 3-param caller; it is the bounded packet reconciliation the Exit condition requires, not a hidden fourth edit.
- Out of bounds: earlier parity-file edits, production/guest/manifest edits, module-shape changes, comparator/algorithm/assertion changes, resolver duplication or fallback.
- Blast radius: two wave sites plus shared harness channel; preserve every assertion in all parity files, FRU/docs-21 discipline. Minimal exact map corrections allowed in wave only; no new production static/live fields.
- Dispatch: question "does the harness supply actual static metadata and reconcile all fourteen parity closures?"; scope builders/closure/helper and bounded callers; return escaped FACT outside thinking with verdict/coverage/deferred 2Y gate.
- Context: M. Authorities: docs/21/22, docs/04 metadata-only index, docs/02 interner. Orca: none.
- Verification: bounded harness/borrow/caller review and re-derived value inventory; compile gate remains 2Y after helper-mediated and executor-driven batches.
- Exit: wave's two calls and all parity static-metadata dependencies reconciled; further necessary caller edits trigger bounded packet reconciliation, never a hidden fourth edit.

### Step 2P — Audit helper-mediated dispatch fixture setup and arachne

- Task IDs: TASK-572, row 01.
- Objective: cover the first helper-only batch with explicit exact fixture data, not selection-helper fallback.
- Precondition: 2G helper supplies internal indexed `Some`; fourteen helper-mediated files re-derived (both live files already owned by 2I); static bundle metadata inspected.
- Postcondition: arachne's nonempty model products receive minimal exact committed map entries with original config; `DispatchFixtureBuilder::build` supports explicit model/perimeter exact-map fixture setup without auto-committing a conflicting default map before callers install custom authoritative maps. Empty `dispatch_config_tdd` direct setup needs no map addition; audit its helper channel and generic-fixture callers. Selection construction remains entirely common-helper internal.
- Allowed reads: `run_real_arachne_guest` (`crates/slicer-runtime/tests/arachne_parity_round2.rs`), `DispatchFixtureBuilder::build`/`DispatchFixture::run_layer` (`crates/slicer-runtime/tests/common/dispatch_fixture.rs`), config dispatch setup in listed contract file; common helper, custom-map caller setup in 2R read-only, ±40-line symbol windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/arachne_parity_round2.rs`; `crates/slicer-runtime/tests/common/dispatch_fixture.rs`; `crates/slicer-runtime/tests/contract/dispatch_config_tdd.rs` (audit/no edit when helper/generic fixture covers it).
- Out of bounds: production/guest code, implicit map seeding at selection/dispatch time, duplicate resolver, assertion/config-effect weakening or skips.
- Blast radius: helper-internal selection covers input construction; explicit minimal exact fixture data allowed. Preserve every assertion and custom-map write-once behavior; FRU/docs-21 discipline, no geometry change.
- Dispatch: question "does explicit fixture setup supply missing exact authority without colliding with caller-owned maps?"; scope named setups/custom-map compatibility; return escaped FACT outside thinking with verdict/no-edit classification/deferred gate.
- Context: M. Authorities: docs/21/22, docs/02 exact interner. Orca: none.
- Verification: bounded exact-key/config/write-once/assertion review; compile/runtime evidence deferred to 2Y, including the standalone `arachne_wall_sequence_survives_wasm_boundary` regression in `crates/slicer-runtime/tests/arachne_parity_round2.rs` (not an aggregator member).
- Exit: arachne/generic fixture data reconciled; config empty path correctly classified; no fourth edit or helper fallback.

### Step 2Q — Reconcile infill-output, pathopt and postprocess helper data

- Task IDs: TASK-572, row 01.
- Objective: migrate three helper-mediated files' missing exact model/perimeter authority. For `crates/slicer-runtime/tests/contract/dispatch_pathopt_tdd.rs`, this step owns helper-mediated map data only; Step 2V separately owns the `execute_per_layer` provisioning surface.
- Precondition: 2G internal selection and 2P explicit fixture setup available; dynamic arena mutations and partial maps located.
- Postcondition: infill-output's changed slices, pathopt's seeded perimeters and postprocess variants beyond its partial tool-precedence map have minimal exact committed keys/configs before selection construction. Preserve original per-region overrides/tool precedence and output/failure assertions; no general fallback.
- Allowed reads: listed files' bundle-helper dispatch, product mutations/config and exact-map setup; common helper and `DispatchFixtureBuilder` explicit setup, ±40-line symbol windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs`; `crates/slicer-runtime/tests/contract/dispatch_pathopt_tdd.rs`; `crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs`.
- Out of bounds: production/guest code, output/error/precedence assertions, geometry changes, skips or implicit helper authority.
- Blast radius: input construction already covered by common helper; minimal exact fixture data aligned with original config/overrides allowed; preserve every assertion, FRU/docs-21 discipline; do not mask intended missing-component/invalid-output witnesses with earlier missing authority.
- Dispatch: question "do exact maps let original output/precedence/failure witnesses reach their intended boundary?"; scope listed fixture windows; return escaped FACT outside thinking with verdict/authority coverage/deferred gate.
- Context: M. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded key/config/failure-boundary/assertion review; compile/runtime evidence 2Y.
- Exit: all three data surfaces reconciled, original positive/negative witnesses preserved, no fourth edit.

### Step 2R — Audit complete modifier/config maps and repair negative-spacing data

- Task IDs: TASK-572, row 01.
- Objective: preserve existing authoritative region maps and supply missing arachne negative-spacing fixture authority.
- Precondition: 2G internal selection and 2P custom-map-compatible fixture setup available; current key/product coverage re-derived.
- Postcondition: modifier's `build_region_map` and per-region config's `region_map` already carry complete exact model/perimeter identities and require no edit solely for helper selection; audit config fields/explicit fixture setup for compatibility. Negative-spacing adds minimal exact map data carrying its original invalid spacing/config so its intended actionable module fatal remains the witness, not an earlier missing-map fatal.
- Allowed reads: `build_region_map`/split/perimeter setup (`crates/slicer-runtime/tests/contract/modifier_split_subregion_density_tdd.rs`); `region_map`/`run_echo` (`crates/slicer-runtime/tests/contract/per_region_config_tdd.rs`); negative-spacing slice/module/dispatch/assertion setup in listed executor file; common helper/explicit fixture setup, ±40-line windows only.
- Allowed edits (three maximum): `crates/slicer-runtime/tests/contract/modifier_split_subregion_density_tdd.rs` (audit/no edit if complete); `crates/slicer-runtime/tests/contract/per_region_config_tdd.rs` (audit/no edit if complete); `crates/slicer-runtime/tests/executor/arachne_negative_spacing_fatal.rs`.
- Out of bounds: production/guest code, density/extruder/spacing expectations, assertion weakening, skips or fallback.
- Blast radius: input construction covered by internal helper; existing exact maps retained, minimal data corrections only if re-audit requires them. Preserve every assertion and original invalid input; FRU/docs-21 discipline.
- Dispatch: question "are existing exact maps complete and does negative-spacing still reach its intended fatal?"; scope named map/product/config symbols; return escaped FACT outside thinking with verdict/no-edit classification/deferred gate.
- Context: S. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded exact-map/config/failure-boundary review; compile/runtime evidence 2Y.
- Exit: both existing-map audits recorded, negative fixture repaired without weakening diagnostic assertion, no fourth edit.

### Step 2S — Finish square/linker data and audit lightning's complete map

- Task IDs: TASK-572, row 01.
- Objective: finish remaining helper-mediated fixture authority in one bounded batch.
- Precondition: internal selection channel and explicit fixture setup exist; current model/perimeter identity coverage inspected.
- Postcondition: simple-square and linker-smoke receive minimal exact maps aligned with original module config; lightning's `lightning_region_map` already covers its shared layer-0/object/region/empty-chain slice/perimeter identity and needs no edit solely for helper selection. Audit declared config fields and both successive module calls; retain its intentional holder/config authority.
- Allowed reads: listed files' slice/perimeter/module/config and bundle-helper dispatch symbols; `lightning_region_map` (`crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs`); common helper, ±40-line windows only.
- Allowed edits (three maximum): `crates/slicer-runtime/tests/executor/arachne_perimeters_simple_square.rs`; `crates/slicer-runtime/tests/executor/infill_linker_pipeline_smoke_tdd.rs`; `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs` (audit/no edit if complete).
- Out of bounds: production/guest code, walls/linking/holder assertions, canonical geometry, skips or fallback.
- Blast radius: helper-internal selection covers construction; minimal exact fixture data allowed, retain lightning's complete map. Preserve every assertion and current-product progression; FRU/docs-21 discipline.
- Dispatch: question "do final helper fixtures have exact indexed authority with unchanged wall/linker outputs?"; scope listed symbols; return escaped FACT outside thinking with verdict/no-edit classification/deferred gate.
- Context: S. Authorities: docs/21/22, docs/02 interner. Orca: none.
- Verification: bounded exact-map/config/borrow/assertion review; re-derive all direct/helper/executor inventories before 2Y.
- Exit: helper-mediated allocation complete (fourteen files including both live files in 2I), no hidden fourth edit or missing authority.

### Step 2U — Provision executor pair and unit collection-builder fixtures

- Task IDs: TASK-572, row 01.
- Objective: provision the production Blackboard index before direct executor calls in the executor-bin pair and unit collection-builder fixture.
- Precondition: 2C–2S migration reviews complete; Step 2B's public `build_and_commit_layer_selection_index` and Step 2G's channel decision available; plans, modules and seeded products re-audited.
- Postcondition: every direct executor site in the three files calls `build_and_commit_layer_selection_index` after seeding `slice_ir` and any exact map data, before `execute_per_layer`/capture. Candidate regions for bound module stages receive minimal exact committed `RegionKey` entries aligned with intended fixture config; slices with no regions need no entries. Preserve every assertion and negative witness.
- Allowed reads: `build_and_commit_layer_selection_index`/`execute_per_layer` (`crates/slicer-runtime/src/layer_executor.rs`); `execution_plan_fixture`, `compiled_stage`, `compiled_module`, `ScriptedRunner` and seed/call sites (`crates/slicer-runtime/tests/executor/layer_executor_tdd.rs`); `make_module`, `SeedingRunner`, `layer_world_builder_commit_preserves_entities_tool_changes_and_z_hops` (`crates/slicer-runtime/tests/executor/layer_world_deep_copy_tdd.rs`); `make_module`, `PerimeterSeedingRunner`, `macro_drain_invokes_host_get_ordered_entities_exactly_once` (`crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs`); `seed_slice_ir` (`crates/slicer-runtime/tests/common/seed.rs`). Symbol-local ±40-line windows only, including each fixture's plan/module/seed symbols.
- Allowed edits (three): `crates/slicer-runtime/tests/executor/layer_executor_tdd.rs`; `crates/slicer-runtime/tests/executor/layer_world_deep_copy_tdd.rs`; `crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs`.
- Out of bounds: production/common-helper edits, test-local gathering or second resolver, implicit map seeding, geometry/assertion changes, skips, guest/IR/WIT/SDK/runner/schema/version changes.
- Blast radius: direct executor provisioning only; preserve every assertion and fatal/nonfatal/order/collection witness, FRU/docs-21 discipline or justified exhaustive waiver. Minimal exact entries only where candidates demand authority; no-region slice needs none. Re-derive site counts, do not freeze authoring counts as acceptance.
- Dispatch: question "does every module-bearing executor site use the public production build/commit entry before execution, retaining assertions and exact authority?"; scope named plan/module/seed/call windows; return FACT outside thinking, properly escaped, redundantly naming verdict, reconciled site count and deferred Step 2Y compile/runtime gate.
- Context: M. Authorities: docs/02 exact interner, docs/04 indexed consumption, docs/21/22. Orca: none.
- Verification: bounded provisioning-order/exact-key/config/assertion review and re-derived site inventory; compile/runtime evidence deferred to Step 2Y's executor/unit bins.
- Exit: all three provisioning surfaces reconciled with unchanged assertions and no fourth edit; no interim compile/runtime-green claim.

### Step 2V — Provision unit ordering pair and pathopt executor sites

- Task IDs: TASK-572, row 01.
- Objective: provision the same production index for both unit ordering fixtures and the pathopt contract's direct executor sites.
- Precondition: Step 2U reviewed; public production build/commit entry exists; Step 2Q's helper-mediated map-data ownership reconciled separately from this step's executor-provisioning surface.
- Postcondition: direct executor sites in all three files call `build_and_commit_layer_selection_index` after seeded `slice_ir` and exact map data, before `execute_per_layer`/capture. Add only minimal exact committed `RegionKey` entries required by candidates for bound module stages, matching intended config; no-region slices need no entries. Step 2Q continues to own helper-mediated map data in `crates/slicer-runtime/tests/contract/dispatch_pathopt_tdd.rs`; Step 2V owns its executor provisioning. Preserve every assertion.
- Allowed reads: `build_and_commit_layer_selection_index`/`execute_per_layer` (`crates/slicer-runtime/src/layer_executor.rs`); `plan_with_stages`, `stage`, `compiled_module`, `seed_tool_slice`, `LiveDispatcherWithInfill` and seed/call sites (`crates/slicer-runtime/tests/unit/path_ordering_tdd.rs`); `plan_with_stages`, `stage`, `compiled_module`, `LiveDispatcherWithLayerCollection` and seed/call sites (`crates/slicer-runtime/tests/unit/tool_ordering_tdd.rs`); `bundle_with_component`, `SeedingRunner`, `path_optimization_empty_input_is_no_op` and direct executor plan/module/seed sites (`crates/slicer-runtime/tests/contract/dispatch_pathopt_tdd.rs`); `seed_slice_ir` (`crates/slicer-runtime/tests/common/seed.rs`). Symbol-local ±40-line windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/unit/path_ordering_tdd.rs`; `crates/slicer-runtime/tests/unit/tool_ordering_tdd.rs`; `crates/slicer-runtime/tests/contract/dispatch_pathopt_tdd.rs`.
- Out of bounds: production/common-helper edits, 2Q helper-mediated surface changes, test-local gathering/resolver, implicit map seeding, ordering/tool/path expectations, assertion weakening/skips, guest/IR/WIT/SDK/runner/schema/version changes.
- Blast radius: executor provisioning only, coordinated with 2Q ownership; preserve every assertion and negative/empty/repeat witness, FRU/docs-21 discipline or justified exhaustive waiver. Minimal exact map entries aligned with fixture config only where candidates require them; no-region slice needs none.
- Dispatch: question "do ordering and pathopt executor sites provision through the production entry without changing helper-owned data or assertions?"; scope named plan/module/seed/call symbols; return FACT outside thinking, properly escaped, redundantly naming verdict, reconciled site count and deferred Step 2Y compile/runtime gate.
- Context: M. Authorities: docs/02 exact interner, docs/04 indexed consumption, docs/21/22. Orca: none.
- Verification: bounded provisioning-order/key/config/assertion and 2Q/2V ownership review; compile/runtime evidence deferred to Step 2Y's unit/contract bins.
- Exit: all three executor provisioning surfaces reconciled, assertions unchanged, 2Q co-ownership explicit, no fourth edit or interim green claim.

### Step 2W — Provision standalone arachne wall-sequence fixture

- Task IDs: TASK-572, row 01.
- Objective: provision the production index for the standalone live arachne-to-path-optimizer executor regression.
- Precondition: Step 2V reviewed; public production build/commit entry exists; standalone target and its module-bearing plan/seed inspected.
- Postcondition: `live_arachne_layer_one_sandwich_reaches_path_optimizer_unchanged` calls `build_and_commit_layer_selection_index` after committing `slice_ir` and required exact map data, before `execute_per_layer`/capture. Add minimal exact committed `RegionKey` entries for seeded candidate regions and intended module config, preserving every assertion; no-region slices need no entries.
- Allowed reads: `build_and_commit_layer_selection_index`/`execute_per_layer` (`crates/slicer-runtime/src/layer_executor.rs`); `live_arachne_layer_one_sandwich_reaches_path_optimizer_unchanged`, `LiveArachneThenPathOpt`, `live_module`, `slice`, `plan` and their module/seed/call setup (`crates/slicer-runtime/tests/arachne_wall_sequence_e2e_tdd.rs`). Symbol-local ±40-line windows only.
- Allowed edits (one): `crates/slicer-runtime/tests/arachne_wall_sequence_e2e_tdd.rs`.
- Out of bounds: production/helpers/other fixtures, test-local gathering/resolver, implicit map seeding, geometry/sequence assertions, skips, canonical algorithms, guest/IR/WIT/SDK/runner/schema/version changes.
- Blast radius: one standalone executor provisioning surface; preserve every assertion and live dispatcher delegation, FRU/docs-21 discipline or justified exhaustive waiver. Minimal exact candidate entries aligned with intended config only; no-region slice needs none.
- Dispatch: question "does the standalone live sequence fixture provision through the production entry and retain every sequence assertion?"; scope named plan/module/seed/call symbols; return FACT outside thinking, properly escaped, redundantly naming verdict, reconciled site count and deferred Step 2Y standalone runtime gate.
- Context: S. Authorities: docs/02 exact interner, docs/04 indexed consumption, docs/21/22. Orca: none.
- Verification: bounded provisioning-order/exact-key/config/assertion review; compile/runtime evidence deferred to Step 2Y, which must execute exactly one passing test in this standalone target.
- Exit: the standalone fixture is production-index provisioned with unchanged assertions, no second edit or interim compile/runtime-green claim.

### Step 2X — Provision coloring, postpass-tap and support-disabled executor fixtures

- Task IDs: TASK-572, row 01.
- Objective: complete the executor-driven fixture class with the three remaining module-bearing direct-executor files.
- Precondition: Steps 2U–2W reviewed; public production `build_and_commit_layer_selection_index` available; each fixture's plan/module/handles/skip path inspected.
- Postcondition: `authored_tool_index_tdd`'s `execute_per_layer_with_events_and_support_tools` site, `visual_debug_postpass_tap_tdd`'s `execute_per_layer_with_events` site and `support_disabled_no_output`'s `execute_per_layer_with_anchored_events` site call `build_and_commit_layer_selection_index` after committing `slice_ir`/exact map data and before execution. `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs` is a standalone target: `postpass_whole_print_tap_contracts` is executed by 2Y at exactly one passing test. Preserve the authored-range-check, whole-print-tap and disabled-support no-output assertions.
- Allowed reads: `build_and_commit_layer_selection_index` and the three entry points (`crates/slicer-runtime/src/layer_executor.rs`); `committed_tool_index`, `infill_ir`, the `modules: vec![compiled_module(...)]` plan and `execute_per_layer_with_events_and_support_tools` call (`crates/slicer-runtime/tests/contract/authored_tool_index_tdd.rs`); `compiled_module_fixture`, `stage_fixture`, `execute_per_layer_with_events` call (`crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs`); `NoSupportLayerRunner`, the `Layer::Support` plan and `execute_per_layer_with_anchored_events` call (`crates/slicer-runtime/tests/integration/support_disabled_no_output.rs`). Symbol-local ±40-line windows only.
- Allowed edits (three): `crates/slicer-runtime/tests/contract/authored_tool_index_tdd.rs`; `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs`; `crates/slicer-runtime/tests/integration/support_disabled_no_output.rs`.
- Out of bounds: production/helper edits, test-local gathering or second resolver, implicit map seeding, tool-index/tap/disabled-support assertions, skips, guest/IR/WIT/SDK/runner/schema/version changes.
- Blast radius: three executor provisioning surfaces completing the ten-file executor-driven class with 2U–2X; preserve every assertion including the out-of-range fallback and the disabled-support no-artifact/zero-path witnesses, FRU/docs-21 discipline or justified exhaustive waiver. Minimal exact candidate entries aligned with intended config only; no-region slice needs none.
- Dispatch: question "do the final three module-bearing executor fixtures provision through the public production entry with unchanged assertions and no hidden skip?"; scope named plan/module/handle/call symbols; return FACT outside thinking, properly escaped, redundantly naming verdict, reconciled site count and deferred Step 2Y compile/runtime gates.
- Context: M. Authorities: docs/02 exact interner, docs/04 indexed consumption, docs/21/22. Orca: none.
- Verification: bounded provisioning-order/exact-key/config/skip-path/assertion review; compile/runtime evidence deferred to Step 2Y's contract/integration bins and its standalone `postpass_whole_print_tap_contracts` regression.
- Exit: all three executor provisioning surfaces reconciled with unchanged assertions, no fourth edit, no interim compile/runtime-green claim; the ten-file executor-driven class is complete.

### Step 2Y — Close migration compile gap and record bins plus standalone regressions

- Task IDs: TASK-572, row 01.
- Objective: restore the mandatory all-target compile gate and prove touched runtime bins and all standalone migrated targets execute without failures.
- Precondition: 2C–2X migration reviews complete; static parity channel and executor-driven production provisioning reconcile all callers; inventories re-derived and assertions unchanged. Guest freshness must return exit 0 before check/runtime runs/attribution; stale artifacts rebuild/recheck per requirements, infrastructure exit 3 blocks. Unit and all three standalone targets (`arachne_parity_round2`, `arachne_wall_sequence_e2e_tdd`, `visual_debug_postpass_tap_tdd`) are required, not covered by executor/integration/contract aggregators.
- Postcondition: ONE intermediate compile gap is closed by `cargo check --workspace --all-targets`; executor/integration/contract/unit saved logs each prove nonzero execution, successful test result and zero failures. The targeted `arachne_wall_sequence_survives_wasm_boundary` in `crates/slicer-runtime/tests/arachne_parity_round2.rs`, standalone `crates/slicer-runtime/tests/arachne_wall_sequence_e2e_tdd.rs` and standalone `crates/slicer-runtime/tests/visual_debug_postpass_tap_tdd.rs` each prove exactly one passing test and zero failures. Capture each command's FACT/count/log findings before the next overwrites the log. These are migration regressions, not completion of unwired selection-delivery ACs or independent preflight; no workspace suite.
- Allowed reads: saved `target/shared-selection-check.log`, `target/test-output.log` and freshness FACTs; enumerated migration helper/caller symbols only as bounded failure-location windows (±40 lines). No broad tracing.
- Allowed edits (zero): none; any necessary fix returns to its owning bounded step and all required gates rerun after repair.
- Out of bounds: source/test/doc edits in this gate step, assertion weakening, workspace suite, stale-artifact deflection or treating zero tests as pass.
- Blast radius: no field/data edits; review FRU/docs-21 compliance and preservation of every assertion from prior batches; minimal exact fixture corrections stay in their owners. No new IR/WIT/SDK/runner/schema/version changes.
- Dispatch: question "do all targets compile, all four touched runtime bins execute nonzero with zero failed, and all three standalone regressions each pass exactly once?"; scope one command at a time and saved log; return FACT outside thinking, escaped, redundantly naming verdict/executed count/log path before overwrite (≤20 failure lines if blocked).
- Context: S. Authorities: docs/21/22, requirements freshness/test discipline. Orca: none.
- Verification: delegated `cargo xtask build-guests --check` (exit 0 required); then `mkdir -p target; cargo check --workspace --all-targets > target/shared-selection-check.log 2>&1; result=$?; echo "FACT check exit=$result"; exit "$result"`; then each following command separately, retaining FACT before overwrite. Integration requires `--features perimeter-spatial-test-support` (packet-254 guard test) and a built debug CLI (`cargo build -p pnp-cli`) for `no_linker_module_degraded_raw_output`:
  - `set -o pipefail; mkdir -p target; cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; import re; s=Path('target/test-output.log').read_text(); results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', s); assert results and all(int(f)==0 for _,f in results) and sum(int(n) for n,_ in results)>0 and 'test result: FAILED' not in s; print('FACT PASS: executor executed='+str(sum(int(n) for n,_ in results))+'; 0 failed; log=target/test-output.log')"`
  - `set -o pipefail; mkdir -p target; cargo test -p slicer-runtime --test integration --features perimeter-spatial-test-support 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; import re; s=Path('target/test-output.log').read_text(); results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', s); assert results and all(int(f)==0 for _,f in results) and sum(int(n) for n,_ in results)>0 and 'test result: FAILED' not in s; print('FACT PASS: integration executed='+str(sum(int(n) for n,_ in results))+'; 0 failed; log=target/test-output.log')"`
  - `set -o pipefail; mkdir -p target; cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; import re; s=Path('target/test-output.log').read_text(); results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', s); assert results and all(int(f)==0 for _,f in results) and sum(int(n) for n,_ in results)>0 and 'test result: FAILED' not in s; print('FACT PASS: contract executed='+str(sum(int(n) for n,_ in results))+'; 0 failed; log=target/test-output.log')"`
  - `set -o pipefail; mkdir -p target; cargo test -p slicer-runtime --test unit 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; import re; s=Path('target/test-output.log').read_text(); results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', s); assert results and all(int(f)==0 for _,f in results) and sum(int(n) for n,_ in results)>0 and 'test result: FAILED' not in s; print('FACT PASS: unit executed='+str(sum(int(n) for n,_ in results))+'; 0 failed; log=target/test-output.log')"`
  - `set -o pipefail; mkdir -p target; cargo test -p slicer-runtime --test arachne_parity_round2 arachne_wall_sequence_survives_wasm_boundary -- --exact 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 1 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 1 executed')"`
  - `set -o pipefail; mkdir -p target; cargo test -p slicer-runtime --test arachne_wall_sequence_e2e_tdd live_arachne_layer_one_sandwich_reaches_path_optimizer_unchanged -- --exact 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 1 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 1 executed')"`
  - `set -o pipefail; mkdir -p target; cargo test -p slicer-runtime --test visual_debug_postpass_tap_tdd postpass_whole_print_tap_contracts -- --exact 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 1 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 1 executed')"`
- Exit: 2C–2X gap closed, all-target check exit 0, fresh-guest exit 0, each executor/integration/contract/unit command exit 0/nonzero executed/zero failed, all three standalone migrated regressions (`arachne_parity_round2` wall-sequence filter, `arachne_wall_sequence_e2e_tdd`, `visual_debug_postpass_tap_tdd`) exit 0/exactly one passed/zero failed, with retained per-command FACT. The standalone wall-sequence target imports `crates/slicer-runtime/tests/unit/path_ordering_tdd.rs` and therefore needs the `--exact` filter; its imported tests remain covered by the unit bin. No `cargo test --workspace` is run. Unrelated-looking failures are investigated with freshness evidence, not skipped or reclassified to obtain green; delivery ACs and all these migration regressions still run after Step 5 and at closure.

### Step 3 — Add selection-backed native construction and commit channel

- Task IDs: TASK-572, row 01.
- Objective: make native transport consume the public selection and accept coloring authority without altering SDK envelopes.
- Precondition: Steps 2–2X index/storage/late-bind/input and executor-provisioning migrations complete and Step 2Y gates pass; existing direct-builder/commit callers inventoried.
- Postcondition: new native builder/commit entry points compile, preserve old direct transport wrapper signatures, and carry filtered configs, all held roles and no-slice carriers.
- Allowed reads: `crates/slicer-wasm-host/src/marshal/native.rs` existing builders and layer commit branches; new selection shapes; `marshal/out.rs::convert_infill_output`; `crates/slicer-sdk/src/native.rs` existing request/response fields; `tests/contract/{view_seam_identity_tdd,region_eligibility_tdd,anchored_events_both_legs_tdd}.rs` direct calls only. Symbol windows ±40 lines.
- Allowed edits (one): `crates/slicer-wasm-host/src/marshal/native.rs` — the exact NET-NEW functions in design, common transport factoring, identity-safe prepared annotations; production dispatch switches in Step 5.
- Out of bounds: SDK/WIT fields, old direct caller arity, selection policy, runtime gating, test assertions.
- Blast radius: no existing shape fields or version constants; direct builder and commit signatures unchanged, including two anchored commit callers. New functions take the existing envelopes plus selection/context. Empty no-context wrapper preserves deny-by-default coloring, not a second production authority.
- Dispatch: question "do new native channels and old direct wrapper callers compile without envelope drift?"; scope all-target check and listed call sites; return FACT.
- Context: M. Authorities: ADR-0056 Decision 1/3, docs/03 boundary filtering, docs/22 observer limitations. Orca: none.
- Verification: delegated `mkdir -p target; cargo check --workspace --all-targets > target/shared-selection-check.log 2>&1; result=$?; echo "FACT check exit=$result"; exit "$result"`.
- Exit: all-target compile passes, configs/claims come from passed selection, carriers are not hidden under `input.slice.map`, and no production switch is falsely claimed yet.

### Step 4 — Migrate coloring keys and all existing literal sites

- Task IDs: TASK-572, row 01.
- Objective: repair full-origin grant/enforcement authority coherently before production selection wiring (AC-BN2 foundation).
- Precondition: Step 3 commit accepts optional context; existing grant-type/literal inventory confirmed.
- Postcondition: `HashSet<OriginId>` replaces coarse grants in all existing production/test sites; stripping uses committed region chain.
- Allowed reads: `crates/slicer-wasm-host/src/marshal/out.rs::{AuthoredColoringContext,allows,enforce_authored_coloring}`; `marshal/origin.rs::OriginId`; dispatcher `run_stage` grant construction; existing coloring test fixtures/bodies; scheduler exact key methods. ±40-line windows.
- Allowed edits (three): `crates/slicer-wasm-host/src/marshal/out.rs`; `crates/slicer-wasm-host/src/dispatch.rs` grant type/construction and exact config lookup only; `crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs` both existing grant literals plus isolated sibling-variant enforcement test.
- Out of bounds: grant predicate semantics, tool range semantics, new fields/WIT, native/host/runtime routing, existing test weakening.
- Blast radius: one production `AuthoredColoringContext` literal in runner, two test literals (`granting_ctx`, different-region case); `allows`'s one enforcement call uses full committed chain. No new field. Add named test `variant_grant_does_not_authorize_sibling`; preserve preexisting predicate/strip tests.
- Dispatch: question "does only the exact variant retain tools and do all old range/deny cases remain?"; scope coloring module command and saved log; return FACT with counts.
- Context: M. Authorities: source-plan full identity, docs/22 falsifiability, existing two-sided coloring predicate. Orca: none.
- Verification: `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd:: 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'variant_grant_does_not_authorize_sibling ... ok' in s and 'test result: ok.' in s and 'test result: ok. 0 passed;' not in s; print('FACT PASS: grant tests executed')"`; delegated all-target check saved as in Step 3.
- Exit: sibling/range/no-context cases strip, authorized exact origin retains tool; all existing test cases execute/pass and all-target check exits 0.

### Step 5 — Wire shared selection into ordinary native/WASM dispatch

- Task IDs: TASK-572, row 01.
- Objective: remove live pair-key/default authority and independent transport reconstruction; turn family-A runner red green.
- Precondition: Steps 2–2X migrations and Step 2Y gates pass, and 3–4 APIs compile; full call-site inventory in design current.
- Postcondition: `WasmRuntimeDispatcher::run_stage` (`crates/slicer-wasm-host/src/dispatch.rs`) consumes `input.selection` before native/WASM split; `None` for a scheduled module returns `LayerStageError::FatalModule` naming scheduled stage/module with neither native entry nor WASM guest invoked or output committed. Both legs receive indexed exact filtered configs/claims, non-region contexts and coloring grants with no recomputation, but payload geometry comes from CURRENT call-local arena slice/support roles/perimeters, including earlier same-layer commits. Associate by full `RegionKey`/`OriginId` and stored merged membership; an absent selected identity is currently possibly empty, never stale snapshot bytes. Missing exact model authority already rejects at production late-bind build, before any Layer invocation.
- Allowed reads: `crates/slicer-wasm-host/src/dispatch.rs::{run_stage,dispatch_layer_call,push_slice_regions}`; `host.rs` map fields/setters/readers and slice/perimeter config traits; `marshal/native.rs` new builder/commit; `marshal/{origin,leaf,in_}.rs` canonical identity/converters; Step 1 tests and new selection fields. ±40-line symbol windows.
- Allowed edits (three): `crates/slicer-wasm-host/src/dispatch.rs`; `crates/slicer-wasm-host/src/host.rs`; `crates/slicer-wasm-host/src/marshal/native.rs` final transport plumbing only, not selection policy.
- Out of bounds: test expectations, IR/WIT/version fields, producer config, SDK envelopes, runtime scheduling, global ordinary error policy.
- Blast radius: both host delivery maps/setters/readers, dispatcher map parameters/all nine setter branch pairs and push lookup; source/resource canonical-ID conversion; new native production calls. Input field/storage were added and callers migrated in Steps 2A–2W, accepted by Step 2Y; no further public field additions here. Keep old direct wrappers but no live production call to them. Resource exact miss must not read default fields; explicit carrier entry remains invocation-authorized. Transport maps/grants derive solely from selected targets/perimeter configs/model full origins; never refilter/rederive carriers or use pair-key/default claims/config.
- Dispatch: question "do all five runner cases and corrected painted cases execute/pass with fresh guests and compile every caller?"; scope requirements' delivery/painted commands, freshness and all-target check; return FACT per command.
- Context: M. Authorities: docs/02 exact interner, docs/03 declared reads, ADR-0056, docs/04 retained global policy. Orca: none.
- Verification: repeat the five-test delivery and three-test painted commands verbatim from requirements; `cargo xtask build-guests --check` before attribution; delegated all-target check saved as Step 3. Step 2Y's migration-bin regressions (executor/integration/contract/unit plus all three standalone migrated regressions) are retained and rerun after dispatch wiring, capturing each FACT before log overwrite; the pre-wiring migration pass alone cannot prove callers survive the new dispatcher contract.
- Exit: required named counts pass; no sibling/default model fallback, malformed-ID fallback, divergent leg selection or fourth edit requirement. Missing map is the planned local routing error, not an unrelated infrastructure failure.

### Step 6 — Register accessor/native-perimeter boundary witnesses

- Task IDs: TASK-572, row 01.
- Objective: prove actual host resource lookup/filtering/canonical rejection and native perimeter config (AC-4/N3).
- Precondition: Step 5 full-origin maps compile and delivery suite passes.
- Postcondition: two named tests are registered and exercise real public host traits plus native runner.
- Allowed reads: `crates/slicer-wasm-host/tests/contract/slice_region_view_contract_tdd.rs` resource/config setup; `host.rs` public builder/push/setter/host trait symbols; `marshal/in_.rs` existing converters; Step 1 native recording setup; `crates/slicer-ir/src/slice_ir.rs` perimeter/config fields. ±40-line windows.
- Allowed edits (three): NET-NEW `crates/slicer-wasm-host/tests/contract/variant_identity_accessors_tdd.rs`; `crates/slicer-wasm-host/tests/contract/main.rs` registration; `crates/slicer-wasm-host/tests/contract/slice_region_view_contract_tdd.rs` explicit origin config inputs for both existing fixtures only.
- Out of bounds: production changes, guest sources, envelope changes and any other fixtures or existing assertion weakening.
- Blast radius: no field/version. Use converters instead of invented resource shapes; mutate region-ID string to `"01"` for canonical rejection. Independent transport fields cover WIT accessors; native dispatch proves live filtering. Existing direct resource assertion fallout is fully named in design: install original metadata fields for `object-172`/7 and clone existing complete fields for `object-live`/7, both empty chains; preserve both tests. No claim of real WASM postprocess geometry.
- Dispatch: question "do both named tests execute and enforce exact fields/errors?"; scope two-test accessor command and saved failure snippets; return FACT.
- Context: M. Authorities: docs/03 WIT source/declared reads, docs/02 interner, docs/21/22. Orca: none.
- Verification: repeat requirements' two-test `variant_identity_accessors_tdd::` command and two-test `slice_region_view_contract_tdd::` command, with guest freshness before attribution.
- Exit: both new tests and both existing resource tests pass with correct widths/absent `private_probe`, both malformed-ID errors and unchanged metadata/live-perimeter geometry; no helper-only simulation of native delivery.

### Step 7 — Register shared source/non-region/anchored dispatch witnesses

- Task IDs: TASK-572, row 01.
- Objective: prove the exported projection and actual production consumers cover all selected categories (AC-B1/B2/B3).
- Precondition: Step 5 dispatch wired; Step 6 accessor setup available; guests fresh.
- Postcondition: three new tests assert merged membership, roles, carrier/raft invocation authority and actual anchored module config. Carrier eligibility uses `global_layer_index`; `anchor_layer_index` remains provenance.
- Allowed reads: `crates/slicer-wasm-host/tests/{raft_plan_read_accessor_tdd.rs,contract/support_identity_layer_dispatch_tdd.rs,contract/anchored_events_both_legs_tdd.rs}` named runner setups only; `tests/common/wasm_cache.rs`; public selection module; `crates/slicer-wasm-host/test-guests/anchored-events-roundtrip-guest/src/lib.rs::run_anchored_events`; `modules/core-modules/{classic-perimeters,traditional-support}/src/lib.rs` relevant config/plan read symbols only. ±40-line windows.
- Allowed edits (two): NET-NEW `crates/slicer-wasm-host/tests/contract/shared_selection_dispatch_tdd.rs`; `crates/slicer-wasm-host/tests/contract/main.rs` registration.
- Out of bounds: all guest/prod/IR/WIT edits, invented guest APIs, exact geometry parity or baselines.
- Blast radius: new fixtures only; FRU/justified exhaustive waivers. Explicit input/model/member/role cardinality and positive nonempty paths required. Anchor guest positive switch 1 yields two fixed entities at layer 7; switch 0 suppresses proposal. In AC-B2, private `carrier_selection_delivery_fixture` at layer 7 must select exactly entry 0 with global 7/anchor 3 and reject entry 1 with global 3/anchor 7, alongside wrong-family entry 2 and declined entry 3 on global layer 7. Supply nonempty role geometry and distinct absent-from-slice identities; preserve selected anchor 3 as provenance, observe native delivery/WASM host config, and do not demand nonempty rendering from that mismatched anchor. Separately execute private `support_rendering_positive_control` at layer 7 with one matching-family, nondeclined global-7/anchor-7 carrier and nonempty support geometry; assert actual nonempty native/WASM committed support paths and their points. Separately execute private `raft_positive_control` and assert nonempty native/WASM paths. Keep all three executions inside the existing test; do not mix control entries into the selection fixture or change its exact eligible cardinality. Preserve carrier global-layer selection, paint anchor filtering and renderer anchor filtering; export shapes, test count and scope are unchanged.
- Dispatch: question "do all three production tests exercise both legs and positive/negative controls?"; scope shared-selection dispatch command, artifact check and bounded assertion facts; return FACT.
- Context: M. Authorities: source plan merged-source/non-region semantics, docs/02/03, ADR-0056, docs/22. Orca: none.
- Verification: `cargo xtask build-guests --check`; repeat requirements' three-test `shared_selection_dispatch_tdd::` command.
- Exit: exactly three tests pass; carrier selection/delivery retains exactly one global-7/anchor-3 target and excludes global-3/anchor-7, separate global-7/anchor-7 support and separate raft controls have actual nonempty native/WASM paths, sources/member identities/configs are exact, non-region sources have no keys, and actual module config affects anchored proposal presence. A nonempty-rendering assertion on the mismatched-anchor selection fixture or combining controls into that fixture falsifies completion.

### Step 8 — Register real runtime-driver eligibility and optional-product regressions

- Task IDs: TASK-572, row 01.
- Objective: demonstrate production runtime decisions and explicit absent/empty controls (AC-B4/BN1), not a projector-only test.
- Precondition: shared index/late-bind/storage and both runtime consumption paths wired in Steps 2A–2C; dispatcher consumes input selection; metadata proves executor target ungated.
- Postcondition: two new executor tests compile and assert positive fixture cardinalities, complete indexed pair coverage, ordinary/capture eligibility and explicit absence/empty behavior. AC-B4's existing `shared_selection_runtime_tdd::schema_stages_and_production_eligibility` test exercises committed inputs missing a variant's exact config at the production late-bind builder: `LayerStageError::FatalModule` names stage/module/exact key with zero Layer invocations and no Layer output committed. Its sequential fixture commits `Layer::Perimeters`, then invokes `Layer::Infill` on the same layer and asserts independently expected post-perimeter fill-mask geometry distinct from the build-time snapshot with original indexed configs/claims and no selection rebuild. Missing bound-pair internal-consistency fatal with zero calls remains required; repeat layer consumption cannot rebuild selection.
- Allowed reads: `crates/slicer-runtime/src/layer_executor.rs::{execute_per_layer,execute_captured_stages_with_support_tools,execute_single_layer_inner,module_invocation_allowed_on_layer,build_and_commit_layer_selection_index}` and its `LayerStageCommit::Perimeters` arm; `sync_perimeter_infill_areas_into_slice` (`crates/slicer-runtime/src/region_partition.rs`); executor `anchored_events_roundtrip_tdd.rs`/`support_anchored_reach_tdd.rs` real pool/runner setup; `crates/slicer-scheduler/src/execution_plan.rs` plan construction/accessors; existing runtime Blackboard/LayerArena public APIs needed by those fixtures; schema stage symbols; public projection. Locate each symbol, ±40 lines only.
- Allowed edits (two): NET-NEW `crates/slicer-runtime/tests/executor/shared_selection_runtime_tdd.rs`; `crates/slicer-runtime/tests/executor/main.rs` registration.
- Out of bounds: production code, synthetic anchored config changes, optional fixture skip/features, schema roster and source contracts.
- Blast radius: new fixture literals only. Stage parameter set derives from STAGES; expected inclusion/exclusion checks assert Layer/export fields, not a hand roster. Use real `execute_per_layer`, native counters and real anchored guest nonempty proposal; also drive the capture path's supported taps with positive committed IR and eligibility counters. Cover explicit no-slice/empty-slice/support optional plan cases. Direct runner-only calls cannot discharge runtime criteria.
- Dispatch: question "do two registered tests actually reach indexed runtime selection through late-bind build and both drivers, including zero-call fatal controls?"; scope exact executor command, bounded call-counter/commit failures and freshness; return FACT.
- Context: M. Authorities: docs/04 scheduling, source-plan optional products/targeting, docs/22. Orca: none.
- Verification: `cargo xtask build-guests --check`; run requirements' unchanged two-test `shared_selection_runtime_tdd::` command; delegated all-target check saved as Step 3.
- Exit: exactly two tests execute/pass with nonempty positive controls and both drivers consuming indexed `invoke`; missing authority/pair controls are fatal with zero calls and no silent default. AC-B4 proves runtime mapping with stage/module/exact key, and sequential Infill sees the independently expected post-perimeter fill mask rather than the build-time snapshot, retaining indexed configs/claims without rebuilding selection. Missing fixture IR, guest failure, stale geometry, per-call reconstruction or synthetic-only evidence blocks the step. Former Step 9's runtime wiring is merged into Steps 2B–2C, not deferred here.

### Step 10 — Prove coloring production/commit permutations

- Task IDs: TASK-572, row 01.
- Objective: complete AC-BN2 using the actual projection-derived grants and native production commit, not isolated grant construction.
- Precondition: full-origin grant migration and selection-backed native commit active.
- Postcondition: named new coloring witness covers both variants, range/no-context denial and independent slice/map permutations; preexisting tests preserved.
- Allowed reads: existing coloring fixture module and Step 1 native runner fixture; public selection grant fields; `marshal/native.rs` production commit function; `marshal/out.rs::{convert_infill_output,enforce_authored_coloring}`; `marshal/origin.rs`. ±40-line windows.
- Allowed edits (one): `crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs` — add `variant_grants_cover_native_and_wasm_boundaries` only; no weakening preexisting tests.
- Out of bounds: production code, guest tools/claims, SDK shape, fixture fallback or role suppression.
- Blast radius: no field/version changes. Use nonempty tagged paths and literal expected `Some(1)`/`None`; native emits paths for both variants to prove commit strips the denied sibling rather than never generating it. Independently constructed WASM accumulator inputs exercise the real conversion boundary, not a purported new guest behavior.
- Dispatch: question "does exact origin authority survive all four permutations on native production and WASM enforcement boundary?"; scope entire coloring module command and saved assertions; return FACT with executed count.
- Context: M. Authorities: source-plan full identity and existing two-sided coloring contract, docs/22. Orca: none.
- Verification: repeat requirements' coloring module command; inspect saved results for both newly added tests and every preexisting case.
- Exit: new named witness plus all old cases execute/pass; granted sibling, out-of-range retention, missing positive paths or omitted permutation falsifies completion.

### Step 11 — Document and accept both families

- Task IDs: TASK-572, row 01 only.
- Objective: publish exact production seam/authority semantics and close row-01 acceptance without claiming later preparation work.
- Precondition: all prior step exits pass and every named AC is covered.
- Postcondition: normative scheduler section exists and agrees with implemented public fields/consumers; all narrow acceptance and all-target gates pass. The section carries the explicit Packet 51 ConfigView Sourcing supersession for indexed Layer stages.
- Allowed reads: `docs/04_host_scheduler.md` RegionMapping/Error Handling Policy including the Packet 51 **Layer Stage Dispatch ConfigView Sourcing** section, `docs/02_ir_schemas.md` Config Interner, docs/03 declared reads and ADR-0056 decisions (delegated bounded section facts); this packet contracts; saved gate findings. Production facts only via named-symbol FACT dispatches.
- Allowed edits (one): `docs/04_host_scheduler.md` exact new section from Doc Impact, leave global Error Handling Policy unchanged.
- Out of bounds: code/test edits, packet/source-plan/backlog bookkeeping in this step, unrelated normative docs, preparation semantics.
- Blast radius: no fields/version/assertion changes. Preserve all family-A/B tests and preexisting painted/grant/raft cases. Public seam fields, no-model-key/absent/empty rules and the explicit Packet 51 supersession for bound Layer stages must appear in semantic review, not just a heading grep.
- Dispatch: question "does each AC and matrix gate pass and does the section accurately describe both families, including the explicit Packet 51 supersession?"; scope requirements commands, public seam and new section; return FACT per command. No independent preflight verdict inferred from self-review or gate success.
- Context: S. Authorities: all requirements named sections, bounded delegated facts. Orca: none.
- Verification: exact Doc Impact `rg` command; re-dispatch every AC and requirements matrix command including freshness, check/clippy all-targets, literals and touched-file quality review; retain post-Step-5 Step-2Y executor/integration/contract/unit and all three standalone regression FACTs or rerun those commands with freshness before attribution. No workspace suite.
- Exit: both families/each named executed-count assertion/gates/doc semantic review pass; no unapproved changes or unresolved touched-file quality findings. TASK-572's later rows remain open.

## Budget and completion ceremony

Steps 1–8 and 10: M each; added Steps 2A/2D: S, 2B/2C/2E/2F: M; runtime migration base Steps 2G–2P: M each, overflow 2Q: M, 2R/2S: S, executor-driven fixture batches 2U/2V/2X: M each, 2W: S, final 2Y: S; Step 11: S. Former Step 9 is merged into Steps 2B–2C. Aggregate L; no single step is L, so the no-L-step rule is satisfied; no broad tracing required. Each step edits at most three explicitly named files. Runtime migration covers the direct-caller/common/harness/helper-mediated surfaces in 2G–2S and the separate ten-file executor-driven fixture class in 2U–2X, with the pathopt surface split explicitly between 2Q/2V, never a hidden over-three-file batch. The field/helper migration has ONE explicit intermediate compile gap (2C–2X), closed by mandatory all-target verification in 2Y, not hidden or reported green. Step 2Y also records fresh-guest executor/integration/contract/unit nonzero regressions and all three standalone migrated regressions with exactly-one-passed assertions; no workspace suite. Letter 2T is intentionally unused (the final gate moved to 2Y); no other existing step is renumbered.

Completion requires every step exit, all authoritative AC commands, public export acceptance, explicit optional-product proof and doc semantic review. Re-dispatch the narrow matrix, record bounded failure evidence and remaining future-work notes, and report context breaches before any status transition. Workspace tests, commits and activation are outside packet authoring. Coordinator-owned status/closure bookkeeping happens only after acceptance and separate authorization; never close the whole TASK-572 from this row alone.
