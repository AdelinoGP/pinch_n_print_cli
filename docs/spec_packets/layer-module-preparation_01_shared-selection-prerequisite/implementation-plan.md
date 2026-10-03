# Implementation Plan: shared-selection-prerequisite

## Execution rules

Do not execute until independent preflight and explicit activation. Work one atomic step at a time, at most three edited files. New interface test compilation failures are scaffolding, not behavioral red. Preserve independently expected identities/configs/roles and positive nonempty guards. Every test must tee combined output to `target/test-output.log`; inspect that saved file before overwriting it. Create `target/` before the first run.

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
- Blast radius: no shape/version change. Use FRU or protected-contract exhaustive waivers; build exact chains into map and slice. Five test names exactly match family-A commands; no missing artifact skip. Missing-map test expects rejection, superseding old fallback draft.
- Dispatch: question "does the distinct-config test execute once and fail on wrong delivered config with fresh guests?"; scope exact red command and bounded saved assertion; return FACT.
- Context: M. Authorities: docs/02 Config Interner, docs/21 §1, docs/22 §§1–2, delegated. Orca: none.
- Verification: `cargo xtask build-guests --check`; then `set -o pipefail; cargo test -p slicer-wasm-host --test contract variant_identity_delivery_tdd::distinct_variant_configs_reach_both_legs -- --exact 2>&1 | tee target/test-output.log >/dev/null; result=$?; python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'running 1 test' in s and 'distinct_variant_configs_reach_both_legs ... FAILED' in s; print('FACT EXPECTED_RED: inspect saved delivery assertion')" && test "$result" -ne 0`.
- Exit: exactly one named test ran and its delivery assertion falsifies current width/identity handling. Compile failure, zero tests, guest instantiation failure or unrelated panic is not an exit.

### Step 2 — Introduce the public shared projection

- Task IDs: TASK-572, row 01.
- Objective: implement the concrete public seam, authority/error/presence shapes and reusable source rules for family B.
- Precondition: Step 1 behavioral red recorded; shapes in packet.spec verified against current IR.
- Postcondition: public projection compiles, exact model config and explicit non-region invocation context exist; source-rule code is reusable and not copied per consumer.
- Allowed reads: `crates/slicer-wasm-host/src/dispatch.rs::{support_carrier_regions,module_receives_slice_region}`; `marshal/mod.rs::perimeter_source_regions`; `crates/slicer-scheduler/src/{execution_plan,validation}.rs` binding/family matcher/static accessors/held claims; `crates/slicer-schema/src/lib.rs::{STAGES,StageSpec,stage_by_id,TIER_LAYER}`; `crates/slicer-ir/src/slice_ir.rs` full key/interner/support/raft/source types. Each located symbol ±40 lines.
- Allowed edits (three): NET-NEW `crates/slicer-wasm-host/src/selection.rs`; `crates/slicer-wasm-host/src/lib.rs` public module; `crates/slicer-wasm-host/src/dispatch.rs` move carrier implementation behind a delegating wrapper only, not live dispatch integration yet.
- Out of bounds: existing struct fields, runtime consumer, host/native delivery and coloring policy, schema stage roster, geometry merge helper body.
- Blast radius: new projection structs have no existing literals. Reused carrier wrapper keeps its old signature. No WIT/schema/version edits; `SupportPlanEntry` remains unbound to model config. Inline unit test `selection::tests::model_identity_and_non_region_authority` uses independently expected keys, configs, support presence and schema-derived stage set; unit evidence does not discharge production ACs.
- Dispatch: question "does public seam preserve exact authority, explicit absence and supported schema set?"; scope new inline test plus all-target check; return FACT.
- Context: M. Authorities: source plan selection section, docs/02 interner, docs/03 declared reads, ADR-0056 Decision 1/3. Orca: none.
- Verification: `set -o pipefail; cargo test -p slicer-wasm-host --lib selection::tests::model_identity_and_non_region_authority -- --exact 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; assert 'test result: ok. 1 passed; 0 failed;' in Path('target/test-output.log').read_text(); print('FACT PASS: 1 executed')"`; delegated `cargo check --workspace --all-targets` saved to `target/shared-selection-check.log` with FACT exit.
- Exit: named test executes/pass and all-target check exits 0; fabricated non-region key, exact-model fallback, stage roster duplication or lost support-entry provenance falsifies the step.

### Step 3 — Add selection-backed native construction and commit channel

- Task IDs: TASK-572, row 01.
- Objective: make native transport consume the public selection and accept coloring authority without altering SDK envelopes.
- Precondition: Step 2 public seam available; existing direct-builder/commit callers inventoried.
- Postcondition: new native builder/commit entry points compile, preserve old direct transport wrapper signatures, and carry filtered configs, all held roles and no-slice carriers.
- Allowed reads: `crates/slicer-wasm-host/src/marshal/native.rs` existing builders and layer commit branches; new selection shapes; `marshal/out.rs::convert_infill_output`; `crates/slicer-sdk/src/native.rs` existing request/response fields; `tests/contract/{view_seam_identity_tdd,region_eligibility_tdd,anchored_events_both_legs_tdd}.rs` direct calls only. Symbol windows ±40 lines.
- Allowed edits (one): `crates/slicer-wasm-host/src/marshal/native.rs` — the exact NET-NEW functions in design, common transport factoring, identity-safe prepared annotations; production dispatch switches in Step 5.
- Out of bounds: SDK/WIT fields, old direct caller arity, selection policy, runtime gating, test assertions.
- Blast radius: no existing shape fields or version constants; direct builder and commit signatures unchanged, including two anchored commit callers. New functions take the existing envelopes plus selection/context. Empty no-context wrapper preserves deny-by-default coloring, not a second production authority.
- Dispatch: question "do new native channels and old direct wrapper callers compile without envelope drift?"; scope all-target check and listed call sites; return FACT.
- Context: M. Authorities: ADR-0056 Decision 1/3, docs/03 boundary filtering, docs/22 observer limitations. Orca: none.
- Verification: delegated `cargo check --workspace --all-targets > target/shared-selection-check.log 2>&1; result=$?; echo "FACT check exit=$result"; exit "$result"`.
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
- Verification: `set -o pipefail; cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd:: 2>&1 | tee target/test-output.log >/dev/null && python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'variant_grant_does_not_authorize_sibling ... ok' in s and 'test result: ok.' in s and 'test result: ok. 0 passed;' not in s; print('FACT PASS: grant tests executed')"`; delegated all-target check saved as in Step 3.
- Exit: sibling/range/no-context cases strip, authorized exact origin retains tool; all existing test cases execute/pass and all-target check exits 0.

### Step 5 — Wire shared selection into ordinary native/WASM dispatch

- Task IDs: TASK-572, row 01.
- Objective: remove live pair-key/default authority and independent transport reconstruction; turn family-A runner red green.
- Precondition: Steps 2–4 APIs compile; full call-site inventory in design current.
- Postcondition: one projection runs before native/WASM split; both legs receive exact filtered views/claims, non-region contexts and coloring grants; missing exact model authority rejects pre-invocation.
- Allowed reads: `crates/slicer-wasm-host/src/dispatch.rs::{run_stage,dispatch_layer_call,push_slice_regions}`; `host.rs` map fields/setters/readers and slice/perimeter config traits; `marshal/native.rs` new builder/commit; `marshal/{origin,leaf,in_}.rs` canonical identity/converters; Step 1 tests and new selection fields. ±40-line symbol windows.
- Allowed edits (three): `crates/slicer-wasm-host/src/dispatch.rs`; `crates/slicer-wasm-host/src/host.rs`; `crates/slicer-wasm-host/src/marshal/native.rs` final transport plumbing only, not selection policy.
- Out of bounds: test expectations, IR/WIT/version fields, producer config, SDK envelopes, runtime scheduling, global ordinary error policy.
- Blast radius: both host delivery maps/setters/readers, dispatcher map parameters/all nine setter branch pairs and push lookup; source/resource canonical-ID conversion; new native production calls. Exact inventory in design, no added public fields. Keep old direct wrappers but no live production call to them. Resource exact miss must not read default fields; explicit carrier entry remains invocation-authorized.
- Dispatch: question "do all five runner cases and corrected painted cases execute/pass with fresh guests and compile every caller?"; scope requirements' delivery/painted commands, freshness and all-target check; return FACT per command.
- Context: M. Authorities: docs/02 exact interner, docs/03 declared reads, ADR-0056, docs/04 retained global policy. Orca: none.
- Verification: repeat the five-test delivery and three-test painted commands verbatim from requirements; `cargo xtask build-guests --check` before attribution; delegated all-target check saved as Step 3.
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
- Precondition: shared seam wired in dispatcher; runtime still has old gate until Step 9; metadata proves executor target ungated.
- Postcondition: two new executor tests compile, assert positive fixture cardinalities and expose old runtime selection divergence behaviorally where applicable.
- Allowed reads: `crates/slicer-runtime/src/layer_executor.rs::{execute_per_layer,execute_captured_stages_with_support_tools,execute_single_layer_inner,module_invocation_allowed_on_layer}`; executor `anchored_events_roundtrip_tdd.rs`/`support_anchored_reach_tdd.rs` real pool/runner setup; `crates/slicer-scheduler/src/execution_plan.rs` plan construction/accessors; existing runtime Blackboard/LayerArena public APIs needed by those fixtures; schema stage symbols; public projection. Locate each symbol, ±40 lines only.
- Allowed edits (two): NET-NEW `crates/slicer-runtime/tests/executor/shared_selection_runtime_tdd.rs`; `crates/slicer-runtime/tests/executor/main.rs` registration.
- Out of bounds: production code, synthetic anchored config changes, optional fixture skip/features, schema roster and source contracts.
- Blast radius: new fixture literals only. Stage parameter set derives from STAGES; expected inclusion/exclusion checks assert Layer/export fields, not a hand roster. Use real `execute_per_layer`, native counters and real anchored guest nonempty proposal; also drive the capture path's supported taps with positive committed IR and eligibility counters. Cover explicit no-slice/empty-slice/support optional plan cases. Direct runner-only calls cannot discharge runtime criteria.
- Dispatch: question "do two registered tests actually reach runtime module selection and falsify old divergence rather than fail fixture setup?"; scope exact executor command, bounded call-counter/commit failures and freshness; return FACT.
- Context: M. Authorities: docs/04 scheduling, source-plan optional products/targeting, docs/22. Orca: none.
- Verification: `cargo xtask build-guests --check`; run requirements' two-test `shared_selection_runtime_tdd::` command, capturing any expected behavioral red and exact assertion separately. Do not require all cases to fail if old behavior happens to agree.
- Exit: exactly two tests compile/execute with nonempty positive controls; any red is an actual routing/counter mismatch. Missing prerequisite fixture IR, guest failure or synthetic event-only evidence blocks the step.

### Step 9 — Consume the shared projection at runtime eligibility

- Task IDs: TASK-572, row 01.
- Objective: replace ordinary runtime's independent gates with the shared production decision (AC-B4/BN1).
- Precondition: Step 8 runtime fixtures execute and independent expected counters/contexts are fixed.
- Postcondition: both ordinary eligibility call sites call shared selection with actual static paint metadata/config/current IR; compatibility paint helper delegates a shared predicate; all runtime cases pass.
- Allowed reads: `crates/slicer-runtime/src/layer_executor.rs` two ordinary stage loops, prepared-view association and compatibility predicate; `crates/slicer-scheduler/src/execution_plan.rs::CompiledModuleStatic` accessors; public selection input/output; Step 8 tests. Each located branch ±40 lines.
- Allowed edits (one): `crates/slicer-runtime/src/layer_executor.rs` ordinary eligibility plumbing only.
- Out of bounds: scheduler field/index contracts, other runtime files, instrumentation/error policy, synthetic anchored closure, fixture expectations.
- Blast radius: no runner/input/static field additions. Existing public paint helper signature preserved by delegation. Repeated projection at runtime/dispatch is acceptable with identical immutable inputs; no layer-spanning cache or native-provenance condition.
- Dispatch: question "do runtime eligibility and no-work counters match shared seam for both call sites?"; scope executor two-test command and all-target check; return FACT.
- Context: M. Authorities: docs/04 scheduling, source-plan targeting, ADR-0056. Orca: none.
- Verification: repeat requirements' two-test executor command; delegated all-target check saved as Step 3.
- Exit: exactly two tests pass, runtime consumes `invoke` rather than duplicate gate logic and absent optional support stays absent; no synthetic anchored change or test-only projection substitute.

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
- Postcondition: normative scheduler section exists and agrees with implemented public fields/consumers; all narrow acceptance and all-target gates pass.
- Allowed reads: `docs/04_host_scheduler.md` RegionMapping/Error Handling Policy, `docs/02_ir_schemas.md` Config Interner, docs/03 declared reads and ADR-0056 decisions (delegated bounded section facts); this packet contracts; saved gate findings. Production facts only via named-symbol FACT dispatches.
- Allowed edits (one): `docs/04_host_scheduler.md` exact new section from Doc Impact, leave global Error Handling Policy unchanged.
- Out of bounds: code/test edits, packet/source-plan/backlog bookkeeping in this step, unrelated normative docs, preparation semantics.
- Blast radius: no fields/version/assertion changes. Preserve all family-A/B tests and preexisting painted/grant/raft cases. Public seam fields and no-model-key/absent/empty rules must appear in semantic review, not just a heading grep.
- Dispatch: question "does each AC and matrix gate pass and does the section accurately describe both families?"; scope requirements commands, public seam and new section; return FACT per command. No independent preflight verdict inferred from self-review or gate success.
- Context: S. Authorities: all requirements named sections, bounded delegated facts. Orca: none.
- Verification: exact Doc Impact `rg` command; re-dispatch every AC and requirements matrix command including freshness, check/clippy all-targets, literals and touched-file quality review. No workspace suite.
- Exit: both families/each named executed-count assertion/gates/doc semantic review pass; no unapproved changes or unresolved touched-file quality findings. TASK-572's later rows remain open.

## Budget and completion ceremony

Steps 1–10: M each, scoped to one boundary and compact discarded validation output. Step 11: S. Aggregate M; no L step or broad tracing required. Extra consumer/test files are separated into bounded steps, not a hidden multi-crate implementation sweep.

Completion requires every step exit, all authoritative AC commands, public export acceptance, explicit optional-product proof and doc semantic review. Re-dispatch the narrow matrix, record bounded failure evidence and remaining future-work notes, and report context breaches before any status transition. Workspace tests, commits and activation are outside packet authoring. Coordinator-owned status/closure bookkeeping happens only after acceptance and separate authorization; never close the whole TASK-572 from this row alone.
