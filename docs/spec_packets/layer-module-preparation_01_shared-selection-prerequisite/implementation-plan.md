# Implementation Plan: shared-selection-prerequisite

## Execution Rules

This is the replacement repair plan for reopened `TASK-572` queue row 01, not instructions to recreate the existing selection/index/Blackboard/input seams. **ACTIVATED 2026-10-08; EXECUTABLE.** All packet files are reconciled, the authoring preflight passed and explicit activation approval was granted. Implementation proceeds under the staged batch discipline below; commit of the authored packet is the docs-only activation commit.

- Work in ordered, test-first vertical cycles. Each editing batch has at most three exact edit paths. An unlisted path or fourth edit stops the batch for contract reconciliation; it is not implicit permission to expand scope.
- Every step maps to `TASK-572`, row 01. The original thirteen ACs, their exact commands, every retained verification-matrix row and all standalone regressions remain mandatory. `packet.spec.md` owns AC command strings; `requirements.md` owns the matrix. Invoke those owners by AC ID or matrix purpose below rather than copying or editing their command strings here.
- Use independent source identities, ordered-chain expectations, widths/roles/config values and nonempty input/output guards. Register tests in the actual binary before counting execution. A helper-only projector, field roster, copied resolver, reconstructed grant set, source grep or synthetic anchored event is not a production oracle (`docs/22_test_quality.md`). Observe invocation 0.8 separately from source-specific 0.4/0.6.
- `docs/21_data_defaults_and_fixtures.md`: production literals stay exhaustive. Watched test literals use meaningful FRU from an existing valid base or a protected-contract `// exhaustive: <reason>` waiver. IR `SupportPlanEntry` has no `Default`; SDK/WIT homonyms are distinct types. Never invent a default or add needless rest syntax while spelling every field.
- All long-file reads are located symbol windows, normally ±40 lines, with explicit bounded ranges selected after grep. Do not load generated trees, lockfiles, large JSON fixtures, vendored code or full logs. Every heavy command and normative fact-check is delegated during implementation, never run by this authoring worker.
- Every dispatch must return its answer **outside thinking**, properly escaped, redundantly naming verdict and scope. Use `FACT` ≤5 lines, `LOCATIONS` ≤20 entries per dispatch, `SNIPPETS` ≤3 bounded excerpts, or `SUMMARY` ≤200 words. Save/log-read findings before overwriting `target/test-output.log`; do not rerun for truncated stdout.
- Re-derive versions, feature gates, actual callers, hard assertions, ADR allocation and historical identities at use. Do not freeze mutable ledger facts into this plan. Inspect `[features]`, `required-features` and file-level cfg before trusting narrow tests; SDK tests require `--features test`, narrow core tests require `--features host-algos` where applicable.
- Guest failures require a just-run ordinary `cargo xtask build-guests --check`: exit 0 fresh, 1 stale/rebuild/recheck, 3 infrastructure BLOCK. Shared-artifact freshness does not prove in-tree mtime freshness; a staged-copy stale panic requires `--force` and recheck. Guest-touching broad runs use `cargo xtask test --summary`, never plain workspace tests. No accelerated-artifact claim.
- `cargo check` and `cargo clippy` gates always use `--all-targets`. Preserve the owners' exact targeted `cargo test --test ... -- --exact` commands; pair them with all-target compile gates rather than changing filters into an all-target/zero-test coverage claim.
- Retain the existing once-only late-bind `build_and_commit_layer_selection_index` (`crates/slicer-runtime/src/layer_executor.rs`), pipeline/prepass callers, immutable Blackboard publication and O(1) `selection_for`. No new runner signature, runtime production schema dependency, second selector, implicit map seeding, `None` dispatch fallback or new preparation API. Source/map-address separation applies to every producer, transport, grant and commit.

### Authoring blocks — must clear before preflight PASS or activation

1. **Finite-window allocation, activated 2026-10-08.** User-approved coordinated staging caps each batch at ≤3 exact files; enumerated I/W/TF/EG/EB ownership and W14 closing gates now incorporate settled typed/generated questions. Intermediate compiler failures are scaffolding, never behavioral red. Cold review accepted this roster, historical-purpose/governance reconciliation and decomposition at activation (2026-10-08); no incomplete roster/old-ABI adapter/weaker gate permission.
2. **Enumerated typed/scope closure proposed; review still required.** Completed local constructor/alias/homonym/scope/rest-pattern/model-receiver resolution is incorporated in design and existing I/S/V/Q/W/11 plus TF00–TF14. Imports/reference/geometry-only context are excluded. Numeric IR, miniature WIT and carried governance dispositions are settled authoring proposals; independent cross-contract review remains required, not compiler-discovered scope expansion. This is not compiler-proven completeness, preflight PASS or activation.
3. **OLD allocation is chosen; capture is future evidence.** NEW `crates/slicer-wasm-host/tests/fixtures/pre-split-layer-infill.component.wasm` is allocated to Step 1 and W11. It does not exist at activation: Step 1 must produce it from the isolated baseline's EXISTING infill guest before WIT migration, verify embedded same-stage export/unversioned core and retain provenance. Baseline availability/compatibility is checked at capture, never assumed. Once captured it is immutable and excluded from rebuilding. Missing/wrong-stage/rebuilt components cannot substitute for AC-R10.
4. **MoonBit allocation grounded; runtime evidence remains future.** Sandbox generation grounds support-query-view/lightning-query-view and unsuffixed changed/new E1/EG01/02/05/06/07/08/09/13/EB01 paths below. Generate/build only in copied scratch; copy named NEEDCOPIED groups after live ownership inspection. W11 owns example typed load/run; W14 closes compilation/guests/resource/example gates. Generator acceptance is not compilation, activation or preflight.
5. **Residual cross-contract review, not unresolved numeric IR policy.** Preserve original thirteen ACs and twenty-one retained matrix commands, R9 runtime executor/four-witness ownership and traceability. SupportGeometryKey keeps layer plus chain; NEW SupportSourceKey is ONLY family_assignments. Both support owners take major bumps from live constants. I1 owns populated actual NEW-owner postcard and independent standalone JSON proofs (OLD geometry-key versus NEW source-key omission). No persisted support-owner input loader found: no new decoder/loader/adapter or old-binary experiment. Numeric IR and synthetic WIT purposes are classified: intentional fixed SupportPlanIR metadata and miniature contrasts remain; only affected canonical expectations migrate. Independent five-contract preflight passed 2026-10-08. The host suite is eight as reconciled in requirements; original commands are unchanged.
6. **Aggregate L execution decomposition is required.** Individual batches are S/M but the aggregate is L. Execute only independently bounded batch contexts with persistent handoffs; do not load or execute the whole packet in one worker. The user's staging approval does not grant an extended context band. Independent preflight assessed this decomposition, including the allocations, at activation (2026-10-08).
7. **S4 future single-path governance gate.** A new decision supersedes the carried ADR-0059 Ruling 2 aggregate/family and Ruling 3 producer-admission clauses and documents typed shared-version consequences. G1/G2/G3 execute BEFORE Step 15. Derive and approve one exact new decision path immediately before G1 writes it; create the decision BEFORE adding references. Derive the deviation ID at G3 write. This bounded future approval gate does not block whole-draft authoring if preflight accepts it; no reserved path/ID, blanket ADR glob or implementation authorization is granted.

### Finite integration windows and compilation policy

**Window I and Window W are sub-rosters of ONE coordinated integration boundary**, not independently compiling schema then ABI phases. Step 0/cold review and explicit activation completed 2026-10-08; execution proceeds under the staged discipline. I owns I1–I6, S1–S6, V1–V5, Q1/Q2 and TF00–TF14; W owns W1–W13, Step 1 OLD capture, E1/EG01/02/05/06/07/08/09/13/EB01 and W11 example witness. Necessary schema-dependent Steps 8–13 constructor/query signatures participate under their existing exact lists; behavior executes only on compiling fresh state. W14 mandatorily closes the entire roster; never require I green before W signature changes. `validate_ir_versions` (`crates/slicer-scheduler/src/validation.rs`) gates GLOBAL SliceIR inclusive min/exclusive max. No global SliceIR bump or manifest-bound edits for subsystem bumps.

**Window W** is W1–W10 plus the explicitly allocated OLD fixture step and E1/W12/W13 plus the explicitly allocated generated-example steps. It cannot begin until the full roster and resource ownership are settled. Breaking canonical WIT, host bindgen, wasm-only macro glue and raw guests cannot compile independently at every three-file boundary. Each batch exit is a migration review, **not green compilation**; no test is run on a known uncompilable intermediate tree and no such error is called behavioral red. Required query-call/constructor signature adaptations in Steps 11A/11B/12/13 belong to this same finite integration window, within those steps' exact edit paths; their behavioral exits are evaluated only after compilation closes. Register Step 8's schema-dependent tests before closing the window, as scaffolding without a red claim. The precise signature-versus-behavior cut in the ownership roster was reviewed and accepted at activation (2026-10-08); the prerequisite wording of those behavioral steps does not authorize leaving old query callers uncompilable. The finite closing gate is Step W14 after every listed and preflight-allocated batch, including example and compatibility witness, completes.

The coordinated boundary owns the complete enumerated blast radius with three-file edit caps and mandatory W14 compile/guest gates. No open-ended “fix compile errors until green” phase or unknown lexical backlog. A genuinely new path stops for bounded reauthoring/preflight. Behavioral red requires compiling fresh state; then falsify the precise join/query/adapter before its behavioral repair. Never erase chains to manufacture green. Review/reconciliation/activation blocks remain.

### Runnable narrow verification helpers

During implementation a command worker may define these in its noninteractive Bash invocation, then use the concrete calls below. They preserve exact target filters, tee combined output, require nonzero executed coverage and read the saved log. They supplement, never replace, owned AC/matrix commands. On expected red, retain the failing assertion and observed values separately; the PASS helper is not a red oracle.

```bash
run_checked() {
  set -o pipefail
  mkdir -p target
  local status=0 inspected=0
  "$@" 2>&1 | tee target/test-output.log >/dev/null || status=$?
  python -c "from pathlib import Path; import re,sys; s=Path('target/test-output.log').read_text(); r=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;',s); assert r and all(int(f)==0 for _,f in r) and sum(int(n) for n,_ in r)>0 and 'test result: FAILED' not in s and int(sys.argv[1])==0; print('FACT PASS: executed='+str(sum(int(n) for n,_ in r))+'; 0 failed; log=target/test-output.log')" "$status" || inspected=$?
  if [ "$status" -ne 0 ]; then return "$status"; fi
  return "$inspected"
}
run_exact() {
  set -o pipefail
  mkdir -p target
  cargo test -p "$1" --test "$2" "$3" -- --exact 2>&1 | tee target/test-output.log >/dev/null || return $?
  python -c "from pathlib import Path; s=Path('target/test-output.log').read_text(); assert 'test result: ok. 1 passed; 0 failed;' in s and 'test result: FAILED' not in s; print('FACT PASS: 1 executed; log=target/test-output.log')"
}
run_filter() {
  set -o pipefail
  mkdir -p target
  cargo test -p "$1" --test "$2" "$3" 2>&1 | tee target/test-output.log >/dev/null || return $?
  python -c "from pathlib import Path; import re; s=Path('target/test-output.log').read_text(); r=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;',s); assert r and all(int(f)==0 for _,f in r) and sum(int(n) for n,_ in r)>0 and 'test result: FAILED' not in s; print('FACT PASS: executed='+str(sum(int(n) for n,_ in r))+'; 0 failed; log=target/test-output.log')"
}
```

All-target gates save output to `target/shared-selection-check.log` / `target/shared-selection-clippy.log` using the owners' exit-preserving wrappers. A successful report-mode quality command still requires fixing/waiving touched-file findings. Read the saved output before drawing conclusions.

## Steps

### Step 0 — Complete authoring ownership and independent preflight

- Task IDs: `TASK-572`, row 01.
- Objective: turn this explicitly blocked repair draft into an executable, bounded contract without changing approved scope.
- Precondition: approved design/draft; self-contained design/plan/task rosters, fixture-purpose dispositions and contested governance clauses available; independent five-contract review required. Scratch receipts are optional prior evidence, not required canonical implementation inputs.
- Postcondition: typed LOCATIONS closure, finite window/fixture/manifest/generated allocations and resolved ownership governance; all packet files agree and independent preflight passed before explicit activation (2026-10-08).
- Files allowed to read, with ranges when over 300 lines: packet contract sections; scratch inventories via ≤200-word summaries and symbol-local LOCATIONS; `docs/21_data_defaults_and_fixtures.md` §1; `docs/22_test_quality.md` §§1–2; `crates/slicer-ir/src/slice_ir.rs` named definitions/constants; WIT inventory's named boundary symbols, ±40-line windows.
- Files allowed to edit (at most 3): none in this discovery/review step. Any coordinator contract-reconciliation edit is a separately authorized ≤3-path authoring slice, not a production action.
- Files explicitly out of bounds: all production/tests/artifacts, backlog/source plan, generated files, OLD fixture and unrelated packets.
- Blast-radius discipline: request LOCATIONS workers scoped separately to each changed type/version; resolve every `VERIFY` and SDK/WIT homonym before declaring closure. The current scratches are evidence inputs, not a satisfying exhaustive ownership citation.
- Expected sub-agent dispatches: question “which exact constructors, assertions, aliases and manifest bounds compile against this changed type?”; scope one inventory type and named crate/module paths; return LOCATIONS. Question “does the reconciled packet pass full authoring preflight?”; scope this packet; return FACT outside thinking, escaped and redundantly naming verdict/blockers.
- Context cost: M; bounded independent authoring handoffs, not one aggregate execution context.
- Authoritative docs: `docs/02_ir_schemas.md` IR Versioning Contract/Config Interner; `docs/03_wit_and_manifest.md` WIT organization; ADR-0022/0044/0045 Decision sections; `docs/21_data_defaults_and_fixtures.md` §1; `docs/22_test_quality.md` §§1–2.
- OrcaSlicer refs: none; no port/algorithm change.
- Verification: independent `/spec-review docs/spec_packets/layer-module-preparation_01_shared-selection-prerequisite --preflight`; each inventory block has a concrete disposition, not an assumed compile result.
- Exit condition: zero unresolved authoring blocks and independent preflight PASS plus explicit activation approval, both completed 2026-10-08.

### Step 1 — Isolated retrospective width-only baseline evidence

- Task IDs: `TASK-572`, row 01; AC-H1 and retained AC-1.
- Objective: measure native wrong-width behavior before chain/output assertions can mask it, without claiming a historical red exists.
- Precondition: Step 0 cleared and activated; current work recorded; isolated baseline/worktree approved; NEW OLD-fixture path selected by design but file not yet required to exist; no history rewrite.
- Postcondition: saved retrospective evidence identifies the baseline selected at use and actual native width/identity/invocation observations, or explicitly reports an unreproduced gap; baseline infill guest captured as immutable same-stage/unversioned-core rejection fixture with provenance receipt before WIT migration.
- Files allowed to read, with ranges when over 300 lines: reflog subjects using `git reflog --all --format='%H %gs'` and a filter for `activate.*register.*family-A delivery`; retained `variant_identity_delivery_tdd::{native_width_witness,assert_width_delivery}` in `crates/slicer-wasm-host/tests/contract/variant_identity_delivery_tdd.rs`, ±40-line windows; baseline manifests/feature gates via bounded summaries.
- Files allowed to edit (at most 3): isolated-baseline copies of `crates/slicer-wasm-host/tests/contract/source_identity_repair_tdd.rs` and `crates/slicer-wasm-host/tests/contract/main.rs`; NEW main-worktree `crates/slicer-wasm-host/tests/fixtures/pre-split-layer-infill.component.wasm` captured from the verified baseline guest. No main-worktree production edits.
- Files explicitly out of bounds: current work, baseline production logic, unrelated assertions, generated artifacts other than approved freshness rebuilding, any history rewrite or stash deletion.
- Blast-radius discipline: no schema change; register `historical_width_witness` as a native-only width observation at baseline-compatible existing seams. Record all delivered source IDs/chains/widths and invocation width before width assertions; do not copy repaired APIs into baseline production.
- Expected sub-agent dispatches: question “does the at-use retained reflog baseline execute the canonical native width fixture and exhibit a measured wrong width?”; scope isolated baseline, registration and saved log; return FACT plus bounded assertion SNIPPETS outside thinking, escaped, redundantly identifying baseline and evidence kind.
- Context cost: M.
- Authoritative docs: `requirements.md` Historical width-red reconstruction; `docs/22_test_quality.md` §§1–2; global stash/preservation rules.
- OrcaSlicer refs: none.
- Verification: worker rederives the actual reflog entry by subject and inspects its tree before choosing it; never pins an authoring SHA. Before any checkout, run `git stash push -u` in the affected worktree, preserving work and retaining the stash identity. Use an isolated worktree; restore original checkout/worktree state and saved work with `git stash apply`, never `pop`/delete/reset. First test on the reconstructed/stashed baseline goes through `cargo xtask test --summary -p slicer-wasm-host --test contract source_identity_repair_tdd::historical_width_witness -- --exact` so literals/freshness preflight runs against that tree. Decode the baseline EXISTING `layer-infill-guest.component.wasm` with `wasm-tools component wit`; verify embedded same-stage export and unversioned `slicer:ir-handles`, then copy that exact artifact to the allocated fixture path before any canonical migration. Record the at-use baseline identity, artifact digest, decoded identity and freshness result in the baseline receipt; never rebuild the captured fixture. Read/retain `target/test-output.log` and receipt under baseline-specific evidence names before repaired tests overwrite them. Repaired witness later runs owned AC-H1.
- Exit condition: exactly one registered witness executed; only a width assertion mismatch with measured delivered values is retrospective red. Missing reflog/artifact/tool, compile failure, zero tests or an identity assertion preempting width observation is BLOCK/unreproduced, never red. OLD fixture capture/provenance must independently succeed before WIT migration; its production is not an activation prerequisite. Current work is restored and stash retained before continuing.

### Step 2 — Register repair witnesses and retain original fixture authority

- Task IDs: `TASK-572`, row 01; AC-R1/R2/R3/R4/R10/R11/H1.
- Objective: register the new host repair module while preserving all existing family-A/B tests and independent expectations.
- Precondition: activated/preflight-cleared contract; historical baseline evidence/gap recorded; fresh ordinary guests before dispatch tests.
- Postcondition: proposed host tests are registered in `contract`; canonical source fixtures expose discriminating identities/geometry before repairs.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/tests/contract/{variant_identity_delivery_tdd,variant_identity_accessors_tdd,shared_selection_dispatch_tdd,authored_coloring_grant_and_strip_tdd}.rs`, named fixtures ±40-line windows; `crates/slicer-wasm-host/tests/common/wasm_cache.rs` loader symbols.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/tests/contract/source_identity_repair_tdd.rs` (new); `crates/slicer-wasm-host/tests/contract/main.rs`; `crates/slicer-wasm-host/tests/contract/infill_holder_resolution_painted_region_tdd.rs` only if the matching source-chain fixture still needs correction.
- Files explicitly out of bounds: production/WIT/IR, original AC removal/rename, SDK guest instrumentation, historical baseline production.
- Blast-radius discipline: no schema addition. FRU/waivers follow docs/21; retain all three painted-holder tests/assertions. Proposed query/chain tests requiring new schema are scaffolded but cannot claim red until compiling after I/W closure.
- Expected sub-agent dispatches: question “which registered old-contract-compatible transport tests fail at the intended assertion with fresh guests?”; scope one exact AC filter per run and saved log; return FACT/SNIPPETS outside thinking, escaped and redundantly naming execution count.
- Context cost: M.
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` §1; `docs/22_test_quality.md` §§1–2; packet repair AC definitions.
- OrcaSlicer refs: none.
- Verification: AC-R1/R2/R4 filters run individually before their repair where compilable; original AC-1/2/3/N1/N2 and painted-holder matrix command retain exact expectations. Inspect red assertion, not only nonzero Cargo exit.
- Exit condition: registration and fixture cardinality are observed; each claimed red is behavioral. Query scaffolding remains separately labeled until its integration window closes.

### Step 3 — Repair source metadata, membership and exact config-address isolation

- Task IDs: `TASK-572`, row 01; AC-1/2/3/N1/N2, AC-B1/B4/BN1, AC-R1/R11.
- Objective: repair key_for/build_layer SOURCE storage and introduce proposed public pure region_map_key_for_sliced_region by factoring existing map_space_region_id/config-address logic, keeping temporary exact address distinct.
- Precondition: transport red registered; no new carrier-chain field is introduced outside Window I ownership.
- Postcondition: source identity/ordered members and all held roles survive once-only selection; missing exact authority rejects at build time with the distinct missing map address; host `resolved_role_tool` derives from resolved `extensions["extruder"]` without becoming a module config read.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/selection.rs::{key_for,build_layer,eligible_carrier_entries,LayerSelectionIndex::selection_for}`; `crates/slicer-scheduler/src/execution_plan.rs::{bind_module_config_view,module_claims_match_active_region}`; `crates/slicer-scheduler/src/validation.rs::resolve_held_claims`; `crates/slicer-core/src/algos/region_mapping.rs::execute_region_mapping_inner`; each ±40-line window.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/selection.rs`; `crates/slicer-wasm-host/tests/contract/variant_identity_delivery_tdd.rs`; `crates/slicer-wasm-host/tests/contract/source_identity_repair_tdd.rs`.
- Files explicitly out of bounds: new index/Blackboard/input creation, stage roster/runner signatures, runtime production dependency changes, general precedence/error policy and geometry algorithms.
- Blast-radius discipline: existing shapes only here. Retain source `RegionKey`/`OriginId`; never decode twice, sort chains or normalize output/grants. `SupportCarrier.variant_chain` introduction and its constructors are Window I, not hidden in this batch.
- Expected sub-agent dispatches: question “does source-storage repair preserve exact model authority, held roles, schema-derived stages and O(1) lookup?”; scope named selection seams and original/repair filters; return FACT outside thinking, escaped with redundant scope/verdict.
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` Config Interner Contract; `docs/04_host_scheduler.md` Complexity Contract; ADR-0056 Decision 1/3; docs/21 and docs/22 test sections.
- OrcaSlicer refs: none.
- Verification: retained AC-N1 builder controls and `run_exact slicer-wasm-host contract source_identity_repair_tdd::held_roles_host_tool_and_carrier_authority` when fixture registration is complete; saved all-target check. Delivery ACs may remain behaviorally red until shared transport is wired; do not claim this projector-only step closes them.
- Exit condition: targeted selection assertions execute; indexed source/member/config semantics are independently verified and compile exits 0; no metadata recomputation or fallback.

### Step 4 — Introduce PRIVATE shared current-geometry assembly before split

- Task IDs: `TASK-572`, row 01; AC-R1/R2/R4 and AC-B1/B4.
- Objective: build one call-local transport from indexed metadata/current products, filtering stored members BEFORE perimeter union and associating prepared data by full source identity.
- Precondition: Step 3 metadata source semantics verified; introducing private internal structs has no preexisting literal sites.
- Postcondition: shared assembly describes current selected views, own/authorized donors and explicit empty views; private module exists without a second selection API or frozen public signature.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/marshal/mod.rs::perimeter_source_regions`; `crates/slicer-wasm-host/src/dispatch.rs::{perimeter_region_index,wall_source_region_id,selected_model_target,push_infill_postprocess_regions}`; `crates/slicer-wasm-host/src/marshal/native.rs::{build_native_layer_request_from_selection,native_layer_perimeter_regions}`; source/prepared IR definitions, ±40-line windows.
- Files allowed to edit (at most 3): NEW PRIVATE `crates/slicer-wasm-host/src/marshal/transport.rs`; `crates/slicer-wasm-host/src/marshal/mod.rs`; `crates/slicer-wasm-host/src/dispatch.rs` assembly insertion before the native/WASM branch, with adapters connected in Step 5.
- Files explicitly out of bounds: public selection/runner signatures, algorithms/union implementation, current snapshot substitution, rediscovery of membership/config/family/claims.
- Blast-radius discipline: private introduction only; no public field/version bump. Exact own perimeter wins; painted fallback is decoded base SOURCE ID with EMPTY chain; modifier fallback is encoded parent SOURCE ID with SAME chain. Missing authorized geometry creates empty selected view, never omission/default/sibling donor. Prepared association uses identity, not filtered position.
- Expected sub-agent dispatches: question “is the private assembly called once before splitting, and does membership filtering precede union?”; scope these three files and bounded transport fixture; return FACT/LOCATIONS outside thinking, escaped and redundantly naming seam.
- Context cost: M.
- Authoritative docs: `design.md` NEW PRIVATE transport assembly/source donor semantics; `docs/04_host_scheduler.md` metadata/current-products contract; docs/21/22.
- OrcaSlicer refs: none.
- Verification: saved all-target check plus internal discriminating fixture; production AC-R1/R2/R4 remain required at Step 5, not discharged by internal assembly assertions.
- Exit condition: private assembly compiles and exact member/donor/empty semantics are falsifiable; no producer config inheritance or second selector introduced.

### Step 5 — Wire BOTH adapters to the same transport and current views

- Task IDs: `TASK-572`, row 01; AC-R1/R2/R4, AC-4/N3, AC-B1/B2/B3/B4/BN1.
- Objective: make native requests and WASM resources consume identical assembly with selected config/claims/source/prepared/current geometry.
- Precondition: private assembly compiles; adapter sites inventoried; no shape changes beyond approved window ownership.
- Postcondition: both production legs deliver all selected source views, including currently absent perimeter identities as empty; snapshots never replace same-layer updated geometry.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/dispatch.rs::{run_stage,dispatch_layer_call,push_slice_regions,push_perimeter_regions}`; `crates/slicer-wasm-host/src/marshal/native.rs::{build_native_layer_request_from_selection,native_perimeter_view}`; `crates/slicer-wasm-host/src/host.rs::{HostSliceRegionView::config,HostPerimeterRegionView::config}`; ±40-line symbol windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/dispatch.rs`; `crates/slicer-wasm-host/src/marshal/native.rs`; `crates/slicer-wasm-host/src/host.rs` existing full-origin map/setter/accessor semantics only.
- Files explicitly out of bounds: index recomputation, public builder arity changes merely for convenience, default config authority, canonical-ID weakening, SDK stage method parameters and support anchor predicates.
- Blast-radius discipline: no schema field/version bump. Invocation config remains explicit only for invocation/non-model context. All scheduled `run_stage` calls require indexed `Some`; AC-N1 intentional rejection remains the only dispatch negative exemption.
- Expected sub-agent dispatches: question “do native and real WASM production calls deliver independently expected current source views and empty cardinality?”; scope exact transport/original filters; return FACT/SNIPPETS outside thinking, escaped, redundantly naming both legs and log.
- Context cost: M.
- Authoritative docs: ADR-0056 Decision 1/3; `docs/02_ir_schemas.md` Config Interner; `docs/03_wit_and_manifest.md` declared reads/canonical IDs; docs/21/22.
- OrcaSlicer refs: none.
- Verification: owned AC-R1/R2/R4 and AC-4/N3; `run_exact slicer-wasm-host contract source_identity_repair_tdd::stored_members_only_before_union`; `run_exact slicer-wasm-host contract source_identity_repair_tdd::exact_own_and_explicit_parent_donors`; `run_exact slicer-wasm-host contract source_identity_repair_tdd::missing_selected_perimeter_is_empty_on_both_legs`; all-target check. Original AC-B2 executes three separate controls and retains its exact command.
- Exit condition: every targeted view/geometry/authority test executes and passes on both legs; no independent per-leg donors/carriers/config selection survives.

### Step 6 — Actual supplied source-origin coloring context reaches conversion

- Task IDs: `TASK-572`, row 01; AC-BN2 and retained coloring matrix.
- Objective: repair grants to source-space full origins and prove the ACTUAL dispatch-computed context flows through native commit and `convert_infill_output`.
- Precondition: both transport legs use source origins; original predicate/range/strip fixtures retained.
- Postcondition: granted variant keeps tool 1; sibling/no-context/out-of-range controls strip to `None` under all slice/map permutations; test never reconstructs grants.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/marshal/out.rs::{AuthoredColoringContext,allows,enforce_authored_coloring,convert_infill_output}`; `crates/slicer-wasm-host/src/marshal/native.rs::commit_native_layer_response_with_authored`; coloring fixture symbols, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/dispatch.rs` actual context connection; `crates/slicer-wasm-host/src/marshal/out.rs`; `crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs` actual supplied-context witness.
- Files explicitly out of bounds: predicate weakening, guest-authored coloring coverage claims, reconstructed equivalent contexts, role-tool-to-module config exposure, native commit signature changes.
- Blast-radius discipline: existing full-origin context type; any newly necessary native wiring edit requires its own ≤3-path step and re-preflight rather than a fourth path. Source tags independent of temporary config addresses.
- Expected sub-agent dispatches: question “does the real conversion receive the dispatch-computed context rather than test-reconstructed grants?”; scope dispatch/conversion/witness; return FACT and bounded connection LOCATIONS outside thinking, escaped with redundant verdict.
- Context cost: M.
- Authoritative docs: `requirements.md` family A actual-context obligation; `docs/22_test_quality.md` production oracle; docs/21.
- OrcaSlicer refs: none.
- Verification: owned AC-BN2 command plus every retained grant/predicate/range/strip test through its matrix command; all-target check. Removing the actual context connection must falsify the witness on a compiling tree.
- Exit condition: independently tagged nonempty accumulators consumed by real conversion with actual supplied context; all negative controls and original tests pass, without claiming new guest coloring coverage.

### Step 7 — Real SDK adapter and invocation config witness

- Task IDs: `TASK-572`, row 01; AC-R5 and strengthened AC-1.
- Objective: make `__slicer_adapt_slice_regions` forward accessor config with `set_config`, and independently encode/decode SDK slice/perimeter and invocation observations.
- Precondition: real SDK guest available; fixture format independently specified before adapting production; ordinary freshness checked/rebuilt after edits.
- Postcondition: invocation 0.8, region 0.4/0.6 and hidden `private_probe` are observed in real guest output; dropping `set_config` falsifies the test.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-macros/src/lib.rs::{__slicer_adapt_slice_regions,__slicer_adapt_perimeter_regions,build_layer_infill_glue}` generated quote-body source windows; SDK config/view accessors and integration setup, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-macros/src/lib.rs`; `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs`; `crates/slicer-runtime/tests/integration/adapt_slice_regions_completeness_tdd.rs`.
- Files explicitly out of bounds: SDK stage method parameter lists, field roster as sole proof, weakening AC-1 width assertions, guessed witness APIs.
- Blast-radius discipline: adapter repair uses existing SDK config setter; new witness encoding is test-only, decoded against literal expectations, not a shared adapter resolver. WIT signature migration belongs to Window W.
- Expected sub-agent dispatches: question “does actual wasm-only SDK adapter output distinguish invocation/source config and reject undeclared private data?”; scope guest build/freshness and exact integration test; return FACT/SNIPPETS outside thinking, escaped and redundantly naming values/log.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` config accessor/declared reads; docs/21/22; `packet.spec.md` AC-R5.
- OrcaSlicer refs: none.
- Verification: owned AC-R5; `run_exact slicer-runtime integration adapt_slice_regions_completeness_tdd::per_region_config_and_invocation_config_are_observed`; fresh actual guest and saved all-target check; retained AC-1 observes invocation separately.
- Exit condition: exactly one integration witness executes with independently decoded values; host-only macro compilation/completeness roster is insufficient.

### Step I1 — BLOCKED owning IR/schema introduction batch

- Task IDs: `TASK-572`, row 01; AC-R3/R8/R9/R10.
- Objective: introduce chain-bearing support/lightning/raft contracts and version changes only after finite-window ownership is complete.
- Precondition: BLOCK 1/2 cleared; exhaustive typed literals/defaults/serde/assertions/manifest roster and independent preflight PASS. The user-approved Window I owns the coupled fallout; this batch retains its three-file cap.
- Postcondition: SupportGeometryKey keeps layer/object/region plus defaulted ordered chain; proposed SupportSourceKey is ONLY family_assignments; geometry maps keep widened SupportGeometryKey. Both support owners take major bumps from live constants; independent key JSON/populated actual-owner postcard tests registered. Other additive carriers retain separately justified classification.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-ir/src/slice_ir.rs::{SupportCandidateSource,SupportGeometryKey,SupportAnalysisIR,SupportPlanEntry,LightningTreeEntry,InfillRaftRegion,SupportEntry,SupportPlanIR::duplicate_region_identity}` and owning CURRENT constants; `crates/slicer-ir/tests/ir_tests.rs::chunk2_ir_schema_versions_are_default_sourced`, ±40-line windows; scratch verified-definition summary.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/slice_ir.rs`; `crates/slicer-ir/tests/ir_tests.rs`; `crates/slicer-ir/tests/support_geometry_ir_shape_tdd.rs`.
- Files explicitly out of bounds: frozen fixture, blanket pin replacement, new decoder/loader/version adapter, fabricated empty chains for known sources, unallocated fallout.
- Blast-radius discipline: key widening breaks owner identity/encoding even with unchanged map spelling/serde default. Bump each affected support owner once from live CURRENT constants to `(old.major + 1, 0, 0)`; geometry maps/layers retained, global SliceIR/manifest bounds unchanged. Existing three paths own design's five exact codec/key tests; no new codec module/Cargo manifest. Owning crate supplies postcard/serde_json dev-dependencies; recheck features/targets/file gates at execution. duplicate_region_identity includes chain; TF00 owns export/error/callers. No positional old-binary compatibility promise.
- Expected sub-agent dispatches: question “which exact owning version/serde/default assertions and typed literals change with each chosen field/key?”; scope one IR type/constant per worker; return LOCATIONS outside thinking, escaped and redundant ownership names.
- Context cost: M; the finite integration roster is distributed across independent bounded worker contexts.
- Authoritative docs: `docs/02_ir_schemas.md` IR Versioning Contract and affected IR sections; docs/21/22.
- OrcaSlicer refs: none.
- Verification: at Window I CLOSE, `run_filter slicer-ir ir_tests chunk2_ir_schema_versions_are_default_sourced`; extend that test to cover the currently missing SupportAnalysisIR current-constant default assertion. Retain existing SupportPlanIR/SupportGeometryIR/InfillIR/SupportIR checks; `run_filter slicer-ir support_geometry_ir_shape_tdd support_geometry_ir_keys_and_sentinel`; owning inline lightning default-version test and settled V1–V4 current-owner versus intentional plan metadata dispositions; all-target/guest gates. Fixed plan metadata equality/clone tests are not persisted old codec compatibility.
- Exit condition: no edits while BLOCKED. After resolution, owning schema/assertion fallout is complete within the explicitly approved finite integration boundary; intermediate compile failure is never behavioral red.

### Step I2 — SDK prepass/query/source projection batch

- Task IDs: `TASK-572`, row 01; AC-R8/R9/R10.
- Objective: propagate exact chain-bearing candidate/geometry/family/plan records and per-full-origin segmentation/config rows through real SDK types and query facade.
- Precondition: exhaustive SDK sites independently inventoried and Window I ownership resolved; I1 chosen shapes agreed with WIT chain records.
- Postcondition: `RegionSegmentationViewEntry` enumerates distinct full origins per object/layer; `RegionSupportConfig` represents one full source origin each; `SupportGeometryViewEntry`, SDK plan/candidate/geometry/family rows retain ordered chain. `SupportAnalysisView::{family_of,region_territory,territory_partition}` and paint support/lightning queries accept exact source identity; SDK stage method parameter lists unchanged. Planners consume occupancy/termination vector fields directly; no invented *_at APIs.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-sdk/src/prepass_types.rs::{SupportPlanEntry,SupportAnalysisView}`; `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView`; `crates/slicer-sdk/src/native.rs` named prepass/layer envelope adapters, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-sdk/src/prepass_types.rs`; `crates/slicer-sdk/src/traits.rs`; `crates/slicer-sdk/src/prepass_builders.rs` `SupportGeometryOutput::{push_support_plan_entry,entries}` reference/chain transport only if required by chosen shape.
- Files explicitly out of bounds: SDK stage parameter lists, pair-only fallback queries, automatic edits to prelude exports/native envelopes absent an actual changed boundary.
- Blast-radius discipline: SDK planner entries differ from IR/generated WIT; PaintRegionLayerView uses new/builders, not Default/literals. Production projections in `crates/slicer-wasm-host/src/marshal/native.rs::support_analysis_view_from_ir` and `crates/slicer-macros/src/lib.rs::build_prepass_support_geometry_glue` are I4/W7/W9; planner constructors/scans 11A/11B. S1–S6/TF own enumerated fixtures/receivers; local classification is not compiler-exhaustive proof.
- Expected sub-agent dispatches: question “does SDK facade preserve full source keys and method arity, with every constructor/query consumer allocated?”; scope SDK types/query sites; return LOCATIONS/FACT outside thinking, escaped and redundantly naming missing sites.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` boundary contracts; `docs/02_ir_schemas.md` source schemas; docs/21/22.
- OrcaSlicer refs: none.
- Verification: `run_checked cargo test -p slicer-sdk --features test --test layer_module_tdd`; S1–S6 actual source/query witnesses and all-target/wasm gates at Window I/W close. SDK-stage compile coverage alone does not prove source queries. Save findings from each complete saved log before the next overwrite.
- Exit condition: allocated SDK batch migration review complete; no independent compile PASS claimed before Window I close.

### Step I3 — Source producers and selection carrier provenance batch

- Task IDs: `TASK-572`, row 01; AC-R3/R8/R9.
- Objective: carry SOURCE chain through contact/candidate/geometry/family/territory, lightning/carrier producers; capture exact PREPASS config address once from each actual SlicedRegion via Step 3 pure mapper separately from source identity.
- Precondition: I1/I2 chosen shapes and exact constructor ownership resolved; no opaque demand/body/entity ID changes.
- Postcondition: contact_work/Contact/candidate routing retain exact captured address and SOURCE chain. All eligible assignment paths (ActiveRegion-driven/no-contact included) publish/query SupportSourceKey from committed origins. Geometry maps retain layered SupportGeometryKey; object-wide geometry unchanged; lightning/carrier preserve source IDs/chains/opaque IDs.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-runtime/src/builtins/support_analysis_producer.rs::{commit_support_analysis_builtin,collect_support_territory}`; `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs::commit_lightning_tree_ir_builtin`; `crates/slicer-wasm-host/src/selection.rs::{build_layer,eligible_carrier_entries,support_carrier_regions}`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`; `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs`; `crates/slicer-wasm-host/src/selection.rs`.
- Files explicitly out of bounds: stored SOURCE normalization, known-source missing/sibling config fallback, geometry algorithms, ActiveRegion/backfill/shell migration, invented resolver/input fields/carrier model grants.
- Blast-radius discipline: SupportCarrier definition/construction/pattern and IR source-key fallout must be explicitly owned across finite Window I; native carrier construction and tests are known additional sites, not hidden fourth edits. Source-chain defaults are not propagation logic.
- Expected sub-agent dispatches: question “do all producer/source references and carrier constructors preserve exact source chain without touching opaque IDs?”; scope named producer/carrier seams; return FACT/LOCATIONS outside thinking, escaped with redundant source/key names.
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` affected support/lightning IR; `design.md` carrier/source-address rules; docs/21/22.
- OrcaSlicer refs: none.
- Verification: named existing producer regressions discovered at use plus AC-R8/R9 after real transport closes; all-target/guest exit at Window I close.
- Exit condition: batch review confirms source chain mint/copy policy; no claim of executable green intermediate tree.

### Step I4 — Host/native/WASM projection and harvest batch

- Task IDs: `TASK-572`, row 01; AC-R3/R8/R10.
- Objective: propagate support full-source records through both projections/harvest and nominal host maps, preserving canonical numeric/string identity.
- Precondition: I chosen schemas and W chosen resource roots agreed; typed IR versus SDK versus generated-WIT constructors classified.
- Postcondition: native/WASM source geometry/family/candidate/plan fields match; canonical IDs reject malformed input rather than infer sibling/request IDs.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/marshal/in_.rs::{project_region_segmentation_view,project_support_analysis_view,project_support_geometry_view,harvest_support_plan_ir_from}`; `crates/slicer-wasm-host/src/marshal/native.rs::{support_analysis_view_from_ir,build_native_prepass_request,commit_native_prepass_response_with_inputs,build_native_layer_request_from_selection}`; host support-output/query impls, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/marshal/in_.rs`; `crates/slicer-wasm-host/src/marshal/native.rs`; `crates/slicer-wasm-host/src/host.rs`.
- Files explicitly out of bounds: guessed IDs/chains, hidden map seeding, layer/anchor predicate changes, invented builder-chain method.
- Blast-radius discipline: exhaustive IR and generated WIT plan constructors independently inventoried; preserve configured origin/claims/resource side effects. Repair `project_region_segmentation_view` to take authoritative source IR in addition to map-space configuration; enumerate full origins per object/layer and emit config rows per full origin, never guess derived source IDs from map bases. Its production caller is `dispatch_prepass_call` in dispatch.rs (W7), which ALREADY receives `slice_ir: Option<Arc<Vec<SliceIR>>>`; native `build_native_prepass_request` (I4/W9) ALREADY receives `PrepassStageInput.slice_ir`. Thread those existing products, not a new input field/SDK stage argument. Macro converts rows in `build_prepass_support_geometry_glue` (W7). Known direct test caller `host_projector_orders_region_segmentation_deterministically` is Q2. Definition, known source-input availability and all known callers are allocated; any additional alias-mediated propagation site must be specifically named before claiming closure. Host changes coordinate W7, not a second nominal core.
- Expected sub-agent dispatches: question “do parallel projections/harvest retain full source chains and canonical ID rejection?”; scope named boundaries; return FACT/SNIPPETS outside thinking, escaped with redundant native/WASM scope.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` canonical IDs/access enforcement; `docs/02_ir_schemas.md` source identity; docs/21/22.
- OrcaSlicer refs: none.
- Verification: retained native prepass/accessor/support identity tests and AC-R8 after window closure; saved all-target/guest gates at boundary.
- Exit condition: exact allocated projections migrated, review complete; no standalone compile/behavior green claim mid-window.

### Step I5 — Source geometry generator and lightning generator batch

- Task IDs: `TASK-572`, row 01; AC-R8/R9.
- Objective: preserve source chains in source-geometry keys and lightning grouping/generation without changing algorithms or units.
- Precondition: source geometry versus physical support geometry policy resolved; map type/version/constructor ownership complete.
- Postcondition: source-key lookups retain layer/source/chain, including intentional intermediate sentinel; lightning grouping/output retain source identity. NEW `source_ordered_chains_remain_distinct_in_generated_lightning` in the already owned algo_lightning_tdd target calls the real generate_lightning_trees with nonempty geometry/config producing nonempty trees, same-number sibling sources and reversed ordered chains. Independently specified SOURCE origins, membership/cardinality and distinguishable payload expectations falsify grouping collapse without changing the geometry algorithm.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-core/src/algos/support_geometry.rs::{execute_support_geometry,add_intermediate_model_layers}`; `crates/slicer-core/src/algos/lightning/mod.rs::generate_lightning_trees`; `crates/slicer-core/tests/algo_lightning_tdd.rs` named generator assertions, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-core/src/algos/support_geometry.rs`; `crates/slicer-core/src/algos/lightning/mod.rs`; `crates/slicer-core/tests/algo_lightning_tdd.rs` source-chain witness/schema fallout.
- Files explicitly out of bounds: geometry algorithm/coordinate changes, speculative ShellClassification timeline migration, producer source-ID normalization.
- Blast-radius discipline: exact exhaustive SupportGeometryKey/LightningTreeEntry constructor sites and grouping keys owned; all additional shape tests allocated by the Window I roster at activation (2026-10-08). No claim a bare core command covers host-algos.
- Expected sub-agent dispatches: question “do generator keys/grouping preserve source identity with unchanged geometry/sentinel semantics?”; scope named generator code and `host-algos` test; return FACT outside thinking, escaped and redundantly naming feature/test/log.
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` source geometry/lightning/version sections; docs/21/22.
- OrcaSlicer refs: none; no algorithm translation.
- Verification: `run_checked cargo test -p slicer-core --features host-algos --test algo_lightning_tdd source_ordered_chains_remain_distinct_in_generated_lightning -- --exact` (require exactly one executed witness); retain the original whole binary via `run_checked cargo test -p slicer-core --features host-algos --test algo_lightning_tdd`. Inspect/save each log before overwrite. AC-R9 runtime production witness remains separate.
- Exit condition: Window I boundary compile/generator tests pass after complete roster; unchanged geometry is not inferred from compile alone.

### Step I6 — Support output/aggregation/territory identity batch

- Task IDs: `TASK-572`, row 01; AC-R8/R9.
- Objective: carry origin chain through support accumulators, plan joins, output and ownership/union/territory references; raft output retains full key.
- Precondition: output IR chains and revised analysis key contracts resolved with complete constructor/assertion ownership.
- Postcondition: `mint_raft_region`, `convert_support_output_with_plan`, aggregation identity joins and territory full-key lookup preserve source chain and opaque IDs. Production `convert_infill_output` and tagged support-output constructors source CURRENT_INFILL_IR_SCHEMA_VERSION/CURRENT_SUPPORT_IR_SCHEMA_VERSION instead of their existing numeric literals; existing constant-sourced empty/untagged branches remain intact.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/marshal/out.rs::{mint_raft_region,convert_support_output_with_plan}`; `crates/slicer-wasm-host/src/support_aggregation.rs::{check_ownership,compare_entries,union_same_family_entries,aggregate_support_plan_irs_with_policy_attributed}`; `crates/slicer-wasm-host/src/support_territory.rs::from_ir`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/marshal/out.rs`; `crates/slicer-wasm-host/src/support_aggregation.rs`; `crates/slicer-wasm-host/src/support_territory.rs`.
- Files explicitly out of bounds: reconstructing source chain from opaque body/demand strings, output builder-chain API additions, unrelated overlap/geometry policy.
- Blast-radius discipline: exhaustive InfillRaftRegion/SupportEntry constructors and accumulator source shape owned; ownership test literal/assertion sites remain explicit Window I roster, not silently omitted.
- Expected sub-agent dispatches: question “do support accumulator/plan/output and raft conversion retain full source keys?”; scope named output and aggregation joins; return FACT/SNIPPETS outside thinking, escaped and redundantly naming keys.
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` InfillIR/SupportIR/SupportPlanIR contracts; docs/21/22.
- OrcaSlicer refs: none.
- Verification: retained support structural/ownership/decline/aggregation matrix witnesses plus AC-R8/R9 at window boundary; all-target/owning version/guest closure after allocated S/V/Q batches and explicitly resolved remaining lexical IR test sites. No subsystem-only manifest-bound migration batch is needed with SliceIR unchanged.
- Exit condition: output/aggregation review complete; every enumerated site migrated and all coupled I/W gates pass at mandatory W14. No independent I compile-before-W requirement.

## Additional finite Window I/W allocations from completed SDK/version inventories

These steps own enumerated sites, with missing surfaces completed by TF00–TF14/generated batches below, not unallocated discovery work. Repeated paths have distinct serial surfaces. Safe test FRU preparation may precede schema introduction; chain/query/version changes close in the coupled I/W boundary. Cold review/reconciliation still block execution; aggregate uses bounded independent handoffs, not one context.

### Step S1 — Typed tree analysis fixtures A

- Task IDs: `TASK-572`, row 01; AC-R8.
- Objective: migrate typed SDK analysis/family/geometry fixtures and retain full-source planner discriminators.
- Precondition: SDK inventory import classification checked; chosen source schema/window governance resolved before field edits.
- Postcondition: tree-family, clearance and diagnostic fixtures keep source chains, family/config/geometry expectations and all prior negatives.
- Files allowed to read, with ranges when over 300 lines: named `run_planner`, analysis/family fixture symbols in edit paths, ±40-line windows; SDK inventory Typed planner test literals section.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`; `modules/core-modules/tree-support-planner/tests/wall_clearance_tdd.rs`; `modules/core-modules/tree-support-planner/tests/diagnostics_tdd.rs`.
- Files explicitly out of bounds: treating tree-family IR SupportPlanEntry as SDK, branch algorithms, deleting negative controls.
- Blast-radius discipline: SDK views/candidates already FRU; exhaustive assignments/geometry receive exact chain or meaningful FRU, never invented IR Default. These are known SDK type sites, not lexical guesses.
- Expected sub-agent dispatches: question “do typed tree fixtures discriminate chain-specific family/geometry and preserve clearance/diagnostics?”; scope three named targets; return FACT outside thinking, escaped with redundant verdict/count/log.
- Context cost: M.
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` §1; `docs/22_test_quality.md` §§1–2; `docs/02_ir_schemas.md` support identity.
- OrcaSlicer refs: none; fixture migration only.
- Verification: `run_filter tree-support-planner tree_family_tdd ''`; `run_filter tree-support-planner wall_clearance_tdd ''`; `run_filter tree-support-planner diagnostics_tdd ''` after boundary closure; all-target check. Metadata confirms these targets and no required-features gates.
- Exit condition: typed fixture migration reviewed within window and each nonzero target passes at close with source/chain assertions intact.

### Step S2 — Typed tree analysis/plan fixtures B

- Task IDs: `TASK-572`, row 01; AC-R8.
- Objective: migrate remaining enumerated tree SDK analysis rows and smooth-node SDK plan literal.
- Precondition: source shapes resolved; SDK versus IR imports in Orca-parity fixture checked.
- Postcondition: buildplate/Orca-parity analysis retains exact source identity; smooth-node entry/reference tests retain membership and geometry.
- Files allowed to read, with ranges when over 300 lines: `tree_analysis`/entry/build-column/read-column symbols in listed files, ±40-line windows; SDK inventory homonym rows.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support-planner/tests/to_buildplate_tdd.rs`; `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`; `modules/core-modules/tree-support-planner/tests/smooth_nodes_tdd.rs`.
- Files explicitly out of bounds: Orca algorithm/source porting, confusing Orca-parity IR entries with SDK, changing original geometry oracle.
- Blast-radius discipline: buildplate/parity assignment rows are exhaustive SDK; smooth_nodes `entry` is explicitly SDK and Default-bearing. Existing IR plan constructors keep valid fixture-base/waiver discipline.
- Expected sub-agent dispatches: question “do these classified fixture rows retain chain and plan-reference semantics?”; scope exact three targets; return FACT outside thinking, escaped and redundantly scoped.
- Context cost: M.
- Authoritative docs: docs/21 §1; docs/22 §§1–2; affected support schemas in docs/02.
- OrcaSlicer refs: none; no canonical behavior change asserted.
- Verification: `run_filter tree-support-planner to_buildplate_tdd ''`; `run_filter tree-support-planner orca_parity_tdd ''`; `run_filter tree-support-planner smooth_nodes_tdd ''` with nonzero saved-log checks and boundary all-target compile.
- Exit condition: every allocated typed fixture target executes/passes after close; parity assertions never weakened for migration.

### Step S3 — Typed traditional analysis and mixed-plan fixtures

- Task IDs: `TASK-572`, row 01; AC-R8.
- Objective: migrate all enumerated traditional SDK family/geometry/config fixtures while preserving raster and mixed IR/SDK plan witnesses.
- Precondition: SDK inventory confirms analysis imports and `SdkSupportPlanEntry` alias; source schema chosen.
- Postcondition: analysis rows and expected source membership retain chain through family assignment, geometry and emitted plans.
- Files allowed to read, with ranges when over 300 lines: `analysis_fixture`, `analysis_fixture_single_blocker`, `analysis_fixture_two_candidates`, named family/source fixtures in edit paths, ±40-line windows.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`; `modules/core-modules/traditional-support-planner/tests/agg_rasterizer_tdd.rs`.
- Files explicitly out of bounds: rasterizer algorithms, baseline weakening, converting unqualified IR plan literals to SDK Defaults.
- Blast-radius discipline: views/candidates already FRU; exhaustive geometry/assignment rows propagate chains. SDK alias plan entries already FRU; explicit chain assertions still required. Coordinate distinct fixture surface with 11A serially.
- Expected sub-agent dispatches: question “do traditional typed fixtures preserve full-source family/geometry and original raster controls?”; scope both targets; return FACT outside thinking, escaped with redundant count/log.
- Context cost: M.
- Authoritative docs: docs/21 §1; docs/22 §§1–2; docs/02 support source schemas.
- OrcaSlicer refs: none.
- Verification: `run_filter traditional-support-planner traditional_family_tdd ''`; `run_filter traditional-support-planner agg_rasterizer_tdd ''`; metadata confirms targets/no required-features gate; all-target compile and R8 actual driver remain mandatory.
- Exit condition: both nonzero targets pass after window close with original geometry/negatives and new exact-source discriminators.

### Step S4 — Host SDK accessor callers, not roster-only coverage

- Task IDs: `TASK-572`, row 01; AC-B2/R8/R9.
- Objective: migrate every enumerated host-test SDK support/lightning accessor invocation to exact source/chain keys.
- Precondition: accessor signatures and specialist facade chosen; original support anchor/global-layer controls retained.
- Postcondition: off-grid/support delivery and lightning per-region queries distinguish chains and preserve all existing assertions.
- Files allowed to read, with ranges when over 300 lines: `native_support`, carrier/render controls, support off-grid query and lightning per-region recording symbols in edit paths, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/tests/contract/shared_selection_dispatch_tdd.rs`; `crates/slicer-wasm-host/tests/contract/support_plan_off_grid_view_tdd.rs`; `crates/slicer-wasm-host/tests/contract/lightning_dispatch_per_region_keying_tdd.rs`.
- Files explicitly out of bounds: redefining support anchor rules, chainless sibling fallback, removing original B2 command/control separation.
- Blast-radius discipline: SDK accessor invocations are classified semantic callers, not import/literal-only edits; IR plan/lightning fixture chain fields use chosen shape and valid FRU/base.
- Expected sub-agent dispatches: question “do actual SDK query caller tests reject sibling/missing-chain reads and preserve original anchor controls?”; scope exact three contract filters; return FACT outside thinking, escaped with redundant identities/count/log.
- Context cost: M.
- Authoritative docs: docs/03 support/lightning facade; docs/21/22; packet AC-B2/R8/R9.
- OrcaSlicer refs: none.
- Verification: unchanged owned AC-B2; `run_filter slicer-wasm-host contract support_plan_off_grid_view_tdd::`; `run_filter slicer-wasm-host contract lightning_dispatch_per_region_keying_tdd::`; retained shared-dispatch matrix, freshness exit 0 and boundary all-target compile.
- Exit condition: all enumerated accessor calls migrate and their nonzero tests pass without collapsing source chain or shifting rendering predicates.

### Step S5 — Runtime SDK lightning accessor callers

- Task IDs: `TASK-572`, row 01; AC-R9.
- Objective: migrate all enumerated runtime lightning SDK query callers and preserve generator/per-region/roundtrip assertions.
- Precondition: LightningTreeEntry and exact SDK query shape chosen; core generation remains source-space.
- Postcondition: same-number ordered-chain variants have independent segments and missing-chain queries cannot borrow results.
- Files allowed to read, with ranges when over 300 lines: `lightning_tree_segments_roundtrip_isolated_by_region`, `fixture_entry`, `lightning_producer_per_region_keying` and their query invocations in edit paths, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/contract/lightning_tree_per_region_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs`.
- Files explicitly out of bounds: producer ID normalization, chain sorting, treating entry-reference-only tokens as mandatory unrelated changes.
- Blast-radius discipline: runtime SDK accessor calls are typed; IR LightningTreeEntry literal fields are distinct. Constructor FRU does not substitute for exact-chain query assertions.
- Expected sub-agent dispatches: question “do the runtime query/producer tests keep both full-source segment sets and reject sibling borrowing?”; scope exact contract/executor filters; return FACT outside thinking, escaped and redundantly named.
- Context cost: M.
- Authoritative docs: docs/02 lightning identity; docs/03 exact query facade; docs/21/22.
- OrcaSlicer refs: none.
- Verification: `run_filter slicer-runtime contract lightning_tree_per_region_roundtrip_tdd::`; `run_filter slicer-runtime contract lightning_tree_view_roundtrip_tdd::`; `run_filter slicer-runtime executor lightning_prepass_tdd::`; freshness/boundary compile and runtime R9 separately.
- Exit condition: every enumerated invocation uses exact full key and all nonzero filters pass after close.

### Step S6 — SDK diagnostic plan guest fixture

- Task IDs: `TASK-572`, row 01; AC-R8/R10.
- Objective: retain diagnostic guest's SDK plan output across full-chain ABI migration.
- Precondition: SDK SupportPlanEntry Default-bearing classification verified; macro prepass conversion allocated W7.
- Postcondition: `run_support_geometry` emits its intended source-chain plan and existing diagnostics without Default silently dropping source provenance.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/src/lib.rs::run_support_geometry` bounded fixture; corresponding `prepass_diagnostic_roundtrip_tdd` test windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/src/lib.rs`.
- Files explicitly out of bounds: SDK stage parameter changes, test-only resolver, diagnostic skip.
- Blast-radius discipline: existing SDK plan literal uses FRU; add actual chain provenance where the fixture requires it, retaining Default for unrelated fields. Generated WIT/IR constructors remain W7/I4.
- Expected sub-agent dispatches: question “does actual rebuilt diagnostic guest preserve plan source chain and diagnostic observations?”; scope guest/freshness/roundtrip filter; return FACT outside thinking, escaped with redundant guest/log.
- Context cost: S.
- Authoritative docs: docs/03 prepass output/diagnostic contract; docs/21/22.
- OrcaSlicer refs: none.
- Verification: rebuild/freshness plus `run_filter slicer-runtime integration prepass_diagnostic_roundtrip_tdd::`; boundary all-target/typed load checks.
- Exit condition: real guest roundtrip executes/passes; FRU adaptation alone never claims chain transport coverage.

### Step V1 — Executor fixture version assignments

- Task IDs: `TASK-572`, row 01; affected IR version assertions and AC-R6/R7.
- Objective: classify and migrate executor InfillIR/SupportIR version assignments without normalizing deliberate compatibility fixtures by fiat.
- Precondition: at-use constants and intended fixture purposes reviewed; serialized type/key policy decided where needed.
- Postcondition: these InfillIR/SupportIR fixtures are classified as current-behavior outputs, not version-rejection controls; each changed owner sources its live constant while every lock/commit/path assertion is retained.
- Files allowed to read, with ranges when over 300 lines: `infill_ir_two_regions`, `support_ir_simple`, `infill_ir`, `infill` in edit paths, ±40-line windows; version inventory fixture-assignment section.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/executor/layer_executor_tdd.rs`; `crates/slicer-runtime/tests/executor/infill_linker_pipeline_smoke_tdd.rs`; `crates/slicer-runtime/tests/executor/order_lock_tdd.rs`.
- Files explicitly out of bounds: automatic numeric replacement, lock weakening, inventing new destination versions.
- Blast-radius discipline: design's V1 ledger settles the current-behavior purpose of infill_ir_two_regions/support_ir_simple/infill_ir/infill. Use owning constants at those constructors, not blanket numeric replacement; preserve all independent lock/commit/path checks and docs/21 fixture bases. No OLD codec claim or frozen destination number.
- Expected sub-agent dispatches: question “which version assignments are current output and which protect historical compatibility?”; scope named fixture symbols; return FACT/LOCATIONS outside thinking, escaped with redundant disposition.
- Context cost: M.
- Authoritative docs: docs/02 IR Versioning Contract; docs/21/22; version inventory typed assignments.
- OrcaSlicer refs: none.
- Verification: `run_filter slicer-runtime executor layer_executor_tdd::`; `run_filter slicer-runtime executor infill_linker_pipeline_smoke_tdd::`; `run_filter slicer-runtime executor order_lock_tdd::`; saved-log nonzero checks at boundary, retained late matrix still required.
- Exit condition: all classified current-behavior constructors use owning constants and original semantic checks pass after boundary closure; no test execution or codec compatibility is inferred from authoring classification.

### Step V2 — Authored/parity/ordering fixture versions

- Task IDs: `TASK-572`, row 01; original authored-tool/ordering/parity witnesses.
- Objective: classify helper-sourced/default-valued InfillIR/SupportIR fixture versions and preserve chain-aware comparator semantics.
- Precondition: version inventory identifies file-local helpers; helper uses separated by actual type, not globally replaced.
- Postcondition: these current-behavior affected IR outputs source appropriate live constants; path_ordering_tdd's module-SemVer helper stays unchanged, with only its InfillIR callers changed directly.
- Files allowed to read, with ranges when over 300 lines: `infill_ir`, `support_ir`, file-local `semver`/ordering fixture constructors in edit paths, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/contract/authored_tool_index_tdd.rs`; `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`; `crates/slicer-runtime/tests/unit/path_ordering_tdd.rs`.
- Files explicitly out of bounds: changing unrelated IR helper versions, comparator weakening, dropping chain comparisons or ordering assertions.
- Blast-radius discipline: SemVer Default is not current schema authority. Design's V2 ledger settles these as current-behavior fixtures. Assign owning constants at affected InfillIR/SupportIR constructors, never change path_ordering_tdd's module-version helper or unrelated type callers; FRU/waivers retain assertions.
- Expected sub-agent dispatches: question “do fixture version and comparator changes preserve authored-tool, ordering and independent parity protection?”; scope three filters; return FACT outside thinking, escaped with redundant semantic names.
- Context cost: M.
- Authoritative docs: docs/02 version contract; docs/21/22; version inventory.
- OrcaSlicer refs: none.
- Verification: `run_filter slicer-runtime contract authored_tool_index_tdd::`; `run_filter slicer-runtime contract parity_invariants_selftest_tdd::`; `run_filter slicer-runtime unit path_ordering_tdd::`; original standalone wall-sequence regression retained later.
- Exit condition: settled current-behavior fixture versions and every nonzero semantic target pass after close; module-SemVer and unrelated IR values remain unchanged.

### Step V3 — Current-owner fixtures and intentional plan metadata preservation

- Task IDs: `TASK-572`, row 01; affected old-value/version-field compatibility witnesses.
- Objective: migrate current-owner geometry/preview/infill versions while preserving the classified intentional SupportPlanIR metadata test inputs/assertions.
- Precondition: serialized geometry key policy decided; purpose of each old-version assertion classified, not inferred from matching numbers.
- Postcondition: current-source geometry/preview/InfillIR pins use owning live constants. The two intentional SupportPlanIR unit merge tests retain matching semver(2, 0, 0) inputs and preservation assertions with a clarifying in-memory metadata comment, not an old codec claim.
- Files allowed to read, with ranges when over 300 lines: `support_geometry_slot_roundtrip`, `support_plan_family_commits_merge_entries_and_metadata`; preview merge/interface/sentinel fixtures; integrated linker `infill`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/unit/blackboard_support_geometry_slot_tdd.rs`; `crates/pnp-cli/tests/support_preview_tdd.rs`; `crates/slicer-runtime/tests/contract/integrated_parity_infill_linker_tdd.rs`.
- Files explicitly out of bounds: guessed/frozen SupportGeometryIR destination, replacing intentional plan metadata values/assertions, adjacent unchanged SliceIR/PerimeterIR numeric versions and global manifest bound changes.
- Blast-radius discipline: SupportGeometryIR takes a major from its live owning constant; current geometry-slot/preview constructors use owning constants. support_plan_family_commits_merge_entries_and_metadata and support_plan_merge_rejects_duplicate_region_identity in the UNIT path retain matching 2.0 inputs/assertions; Blackboard::commit_support_plan requires version equality and starts from a clone of the first owner, deliberately preserving metadata. Only necessary chain/key/duplicate-error edits and a purpose comment apply there. integrated_parity_infill_linker_tdd::infill is a current-behavior global-version mixup: use CURRENT_INFILL_IR_SCHEMA_VERSION only for InfillIR, not adjacent SliceIR/PerimeterIR. I1 owns defaults; TF02 owns full-key sorting; preview DTO schema unchanged.
- Expected sub-agent dispatches: question “do current-owner constants and retained matching plan metadata inputs/assertions implement the settled dispositions without touching adjacent unchanged owners?”; scope named symbols; return FACT/SNIPPETS outside thinking, escaped with redundant disposition.
- Context cost: M.
- Authoritative docs: docs/02 IR versioning; docs/21/22; design's settled numeric IR ledger.
- OrcaSlicer refs: none.
- Verification: `run_filter slicer-runtime unit blackboard_support_geometry_slot_tdd::`; `run_filter pnp-cli support_preview_tdd ''`; `run_filter slicer-runtime contract integrated_parity_infill_linker_tdd::`; window owning schema assertions. Explicitly retain/assert version-equality rejection and first-owner clone metadata preservation, not persisted old-owner decoding.
- Exit condition: every intentional metadata assertion remains protected, current-behavior constructors use their constants and all nonzero targets pass after closure; no OLD codec compatibility claim.

### Step V4 — Lightning/support version fixtures and actual typed diagnostic fallout

- Task IDs: `TASK-572`, row 01; AC-R8/R9/R10 and retained dispatch protocol.
- Objective: classify remaining typed lightning/support fixture assignments and preserve exact actual-instantiation diagnostics under the new nominal resource contract.
- Precondition: current-versus-historical purposes reviewed; canonical guest rebuilt/fresh before asserting engine wording.
- Postcondition: integrated lightning/live support fixtures are current-behavior constructors and source their respective live InfillIR/LightningTreeIR/SupportIR constants; wrong-stage dispatch still fails with independently observed expected diagnostics.
- Files allowed to read, with ranges when over 300 lines: `integrated_parity_lightning_infill`, `support_commit`, `stage_miss_is_fatal_at_instantiation` in edit paths, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/contract/integrated_parity_lightning_infill_tdd.rs`; `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`; `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs` verified wording/resource identity fallout only.
- Files explicitly out of bounds: assuming old no-export wording after nominal split, treating wrong-stage diagnostic as OLD same-stage fixture coverage, weakening original support paths.
- Blast-radius discipline: affected Infill/Support/Lightning constructors use their owning constants, with no future numeric pin or parity weakening. Dispatch's affected canonical export expectation migrates; engine wording changes only from actual typed failure. Frozen OLD path/source is W11, not editable here. Classified synthetic miniature inputs/assertions remain protected through bounded test-expectation isolation in V5 and coordinated verifier adaptation in W8.
- Expected sub-agent dispatches: question “do remaining typed fixture versions and real dispatch diagnostics protect the intended semantics?”; scope three target filters and saved failures; return FACT/SNIPPETS outside thinking, escaped with redundant verdict/log.
- Context cost: M.
- Authoritative docs: docs/02 IR versions; docs/03 typed instantiation; docs/21/22; ADR-0044/0045.
- OrcaSlicer refs: none.
- Verification: `run_filter slicer-runtime contract integrated_parity_lightning_infill_tdd::`; `run_filter slicer-runtime executor live_layer_support_tdd::`; `run_filter slicer-runtime contract dispatch_protocol_tdd::`; fresh guests and real OLD R10 separately.
- Exit condition: all retained assertions pass after current-owner migration; no guessed diagnostic or false OLD rejection/codec claim.

### Step V5 — Retain independent miniature WIT/version controls

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: retain classified synthetic freshness/fingerprint/comparison controls independently of production package migration.
- Precondition: design's miniature-versus-canonical disposition accepted; W8 owns actual new package recognition, V4/W10 own affected canonical expectations.
- Postcondition: fixture-derived worlds retain their signatures/versions and protected output/infra/stage/fingerprint invariants through bounded test-expectation isolation in `xtask/src/build_guests.rs`; the live `stale_reason_with_mode` → `resolve_stage_from_world` expectation seam no longer makes positive-freshness controls drift solely because the live stage version changes. Coordinated W8 owns `xtask/src/wit_verify.rs` miniature comparison expectation isolation and live stage-resolution expectation adaptation; no synthetic version is mistaken for a future canonical destination, and production nominal-version checks remain unweakened.
- Files allowed to read, with ranges when over 300 lines: `xtask/src/build_guests.rs::{stale_reason_with_mode,fresh_ctx}` and located miniature WIT test constructors/owning test symbols, ±40-line windows; `xtask/src/wit_verify.rs::{stage_pkg,stage_expectation,resolve_stage_from_world,make_expect,canonical_pair,export_name_compared_exactly_including_version,stage_resolves_from_embedded_package_name}` as coordinated W8 read context, ±40-line windows.
- Files allowed to edit (at most 3): `xtask/src/build_guests.rs` bounded test-expectation isolation only; `xtask/src/wit_verify.rs` adaptation is owned exclusively by W8, not edited in V5.
- Files explicitly out of bounds: blanket version replacement, new guest fixture imports guessed from filename, rebuilding frozen OLD component.
- Blast-radius discipline: `fresh_ctx`/`MEMO_WIT` and located guest-checker worlds derive canonical models from their own fixtures, but `stale_reason_with_mode` resolves stage expectations through live declarations. V5 isolates that test-expectation seam while preserving synthetic inputs/assertions, including numeric versions/signatures in fingerprint, stage-mismatch, output/infra/lock-divergence and accelerated memoization controls. W8 exclusively owns verifier miniature comparison expectation isolation (`make_expect`/`canonical_pair` and the exact export-version negative oracle) and adaptation of the live package assertion in `stage_resolves_from_embedded_package_name`, preserving the assertion's live-resolution contract and synthetic inputs. Derive any live expectation from declarations at implementation time, never a frozen future numeric destination; do not weaken production nominal-version checks or blindly replace literals.
- Expected sub-agent dispatches: question “do all retained synthetic guest-checker controls execute with unchanged protected assertions after canonical migration?”; scope build_guests tests and saved log; return FACT outside thinking, escaped and redundant purpose/execution outcome.
- Context cost: M.
- Authoritative docs: docs/03 artifact freshness/typed compatibility; ADR-0044/0045; docs/22.
- OrcaSlicer refs: none.
- Verification: `run_checked cargo test -p xtask --bin xtask build_guests::tests::`; save complete-log nonzero/zero-failure findings before overwrite; confirm positive-freshness controls and negative stage/fingerprint/drift controls retain their protected assertions after isolation. Coordinated verifier adaptation is verified under W8's existing filter; all-target compile and guest freshness at W14.
- Exit condition: classified synthetic controls execute/pass with unchanged independent inputs/assertions; real migrated loads and frozen OLD rejection remain separately required.

### Step Q1 — Exact-Z full-chain cache/request identity, unchanged object geometry

- Task IDs: `TASK-572`, row 01; AC-R8 and query/ownership controls.
- Objective: extend `QueryKey`/`ExactZQueryService::query` to ordered source-chain identity and migrate actual aggregation/WIT service callers.
- Precondition: chosen full-source key/hash policy and W6 chain ABI agreed; test-first cache/input controls authored before service repair.
- Postcondition: same full key shares immutable result; same object/region/Z with a different or reordered chain has distinct cache identity. Returned occupancy/blockers/termination/baseline remain object-wide and geometrically unchanged.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/exact_z_query.rs::{QueryKey,ExactZQueryService::query}` bounded service; `support_aggregation.rs::validate_entry`; `dispatch.rs` exact_z_support_query::Host::query, ±40-line windows. Existing direct contract test is Q2; construction-only references require no query-arity edit.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/exact_z_query.rs` service plus inline cache witnesses; `crates/slicer-wasm-host/src/support_aggregation.rs::validate_entry`; `crates/slicer-wasm-host/src/dispatch.rs` WIT Host query implementation.
- Files explicitly out of bounds: source-clipping object-wide collision mesh, changing physical-Z normalization, pair-only ownership/territory/config, fabricated inline existing-test claims.
- Blast-radius discipline: production direct service callers are actual `validate_entry` and WIT Host query; test direct service calls are `tests/contract/exact_z_support_query.rs` (Q2). Add NEW inline `exact_z_query::tests::query_cache_identity_includes_ordered_source_chain` in the owned service file. Call the real service with the same nonempty physical object geometry/Z and distinct empty, sibling and reversed ordered chains; inspect actual cached entries and same-key Arc reuse, independently expected full keys/cardinality, and identical object-wide query geometry/results. Do not fabricate a cache or partition geometry by source. Existing contract positive/cache assertions are preserved, not claimed already inline. `TerritoryClipper::{from_ir,has_territory}` full-source ownership stays I6 independently of object-wide geometry.
- Expected sub-agent dispatches: question “do full-chain cache and actual WIT/aggregation inputs discriminate identities while object-wide geometry remains identical?”; scope Q1/Q2/W6/W7; return FACT outside thinking, escaped with redundant key/geometry scope.
- Context cost: M.
- Authoritative docs: design Exact-Z policy; docs/02 source identity; docs/03 prepass query; docs/21/22.
- OrcaSlicer refs: none; no geometry algorithm change.
- Verification: `run_checked cargo test -p slicer-wasm-host --lib exact_z_query::tests::query_cache_identity_includes_ordered_source_chain -- --exact` (require exactly one); then `run_checked cargo test -p slicer-wasm-host --lib exact_z_query::` with nonzero execution. Inspect/save each log before overwrite. Q2 retained actual direct contract and R8 WIT calls; all-target/guest boundary gates.
- Exit condition: full-source/cache controls execute, same-key Arc reuse and distinct-chain identity are falsifiable, object-wide nonempty geometry equal; no cache-side chain erasure or accidental source clipping.

### Step Q2 — Actual Exact-Z and authoritative source-projection callers

- Task IDs: `TASK-572`, row 01; AC-R8/R10.
- Objective: migrate existing direct query and projection fixture calls while supplying authoritative source IR instead of map-derived guessed source IDs.
- Precondition: Q1/W6 service and I4 source-projection input contracts chosen; explicit source fixtures independently seeded.
- Postcondition: exact-Z contract retains physical-Z/nonempty occupancy/cache checks with chain controls; projector deterministic-order witness receives actual source origins/config rows and preserves ordered chains.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs::exact_z_support_query`; `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs::{host_projector_orders_region_segmentation_deterministically,planner_emits_one_entry_per_region_in_region_map}`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs`; `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs`.
- Files explicitly out of bounds: weakening exact-Z geometry assertions, projection fixture resolver mirrors, reconstructing derived IDs from map base IDs.
- Blast-radius discipline: both known test projection invocations are in the named executor witness; production `dispatch_prepass_call` input is W7, native rows I4/W9 and macro rows W7. Known caller set is allocated. Any extra alias-mediated input caller requires named LOCATIONS disposition before closure, not an unbounded Step 0 placeholder.
- Expected sub-agent dispatches: question “do exact direct query/projection fixtures observe full-source identity with unchanged physical geometry/order guarantees?”; scope two exact filters; return FACT/SNIPPETS outside thinking, escaped with redundant witness/count/log.
- Context cost: M.
- Authoritative docs: design authoritative-source/Exact-Z clauses; docs/02 interner/source; docs/21/22.
- OrcaSlicer refs: none.
- Verification: `run_exact slicer-wasm-host contract exact_z_support_query::exact_z_support_query`; `run_filter slicer-runtime executor prepass_support_geometry_layer_plan_tdd::`; real WIT R8 and boundary compile/freshness separately.
- Exit condition: all known arity/input calls compile and their nonzero tests pass, with explicit source authority and chain-order preservation.

### Step 8 — Register actual runtime full-chain repair drivers

- Task IDs: `TASK-572`, row 01; AC-R6/R7/R8/R9.
- Objective: register executor fixtures driving ordinary/capture commit paths and real support/infill/lightning guests, not host-only/helper-only tests.
- Precondition: AC-R9 owner corrected to runtime executor and aggregate command-count reconciliation preserves retained coverage. Registration may occur as schema-dependent scaffolding inside I/W; actual execution/red claims require closed compile/freshness boundaries.
- Postcondition: four proposed runtime repair tests execute in the correct binary; positive inputs, independent source/chain payloads and ordinary/capture controls discriminate pair-only joins.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-runtime/tests/executor/shared_selection_runtime_tdd.rs` production-driver setup; `crates/slicer-runtime/tests/common/mod.rs` production fixture seams; exact existing support/lightning executor fixtures, ±40-line windows.
- Files allowed to edit (at most 3): NEW `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs`; `crates/slicer-runtime/tests/executor/main.rs`; `crates/slicer-runtime/tests/executor/shared_selection_runtime_tdd.rs` retained-driver extensions only if needed.
- Files explicitly out of bounds: wrong-binary AC-R9 host substitute, test-local selector/resolver, synthetic anchored invocation proof, fabricated capture taps.
- Blast-radius discipline: use approved schema defaults/valid fixture bases under docs/21; no new production type/version here. Source queries include chain-order permutation controls and missing-chain rejection.
- Expected sub-agent dispatches: question “does each new executor witness run actual ordinary/capture production paths and fail the intended full-source assertion before its join repair?”; scope exact runtime filters and log; return FACT/SNIPPETS outside thinking, escaped, redundant driver/leg/count names.
- Context cost: M.
- Authoritative docs: packet AC-R6/R7/R8/R9; `docs/04_host_scheduler.md` commit/capture semantics; docs/21/22.
- OrcaSlicer refs: none.
- Verification: `run_exact slicer-runtime executor source_identity_repair_tdd::runtime_commit_joins_use_full_source_identity`; `run_exact slicer-runtime executor source_identity_repair_tdd::guest_and_partition_joins_preserve_source_identity`; `run_exact slicer-runtime executor source_identity_repair_tdd::support_chain_reaches_planners_renderers_and_output`; `run_exact slicer-runtime executor source_identity_repair_tdd::lightning_and_raft_full_source_roundtrip`. Capture genuine behavioral red per repair before implementation; zero tests/compile errors are blockers.
- Exit condition: each registered driver executes and discriminates its intended regression; no repaired-state PASS is claimed for a red fixture.

### Step 9 — Runtime lock and commit joins use full source identity

- Task IDs: `TASK-572`, row 01; AC-R6 and AC-R9 raft commit.
- Objective: repair lock validation, postprocess reattachment/apply and raft full-key merge in actual executor commit paths.
- Precondition: runtime witnesses registered and compiling/fresh; independent sibling payload/lock controls captured.
- Postcondition: `validate_infill_order_locks`, `LayerStageCommit::PerimetersPostProcess`, `LayerStageCommit::SlicePostProcess` and `merge_infill_ir` join source ID plus ordered chain; config-address restoration cannot rewrite output identity.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-runtime/src/layer_executor.rs` named lock/commit/apply/merge symbols and ordinary/capture drivers, ±40-line windows; output origin definitions.
- Files allowed to edit (at most 3): `crates/slicer-runtime/src/layer_executor.rs`; `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs`.
- Files explicitly out of bounds: scheduling/ordinary error policy, order-lock weakening, config precedence rewrites, synthetic invocation evidence.
- Blast-radius discipline: schema already closed; any source-field fallout outside allocated paths triggers stop/reconciliation. Preserve full chain equality/order and original output fields.
- Expected sub-agent dispatches: question “do ordinary/capture production commits reject sibling replacement and retain both independent full-source payloads?”; scope exact R6/R9 filters, saved log; return FACT/SNIPPETS outside thinking, escaped and redundant join names.
- Context cost: M.
- Authoritative docs: `docs/04_host_scheduler.md` commit/lock contracts; `docs/02_ir_schemas.md` source output identity; docs/21/22.
- OrcaSlicer refs: none.
- Verification: owned AC-R6 and corrected AC-R9 runtime filter; all-target check; retained runtime executor matrix commands.
- Exit condition: production witnesses pass on native/WASM and supported ordinary/capture paths, preserving source IDs with independent payloads; lock rejection control runs.

### Step 10 — Conversion, infill-linker and same-layer partition joins

- Task IDs: `TASK-572`, row 01; AC-R7 and retained AC-B4.
- Objective: repair `merge_slice_postprocess_into`, production `orchestrate_infill`, full-source perimeter indexing and own/modifier-parent partition synchronization.
- Precondition: schema/WIT closure and R7 behavioral red; real infill-linker guest rebuild after edits.
- Postcondition: chain variants and chain-order permutations cannot borrow sibling masks/output; same-layer updated geometry reaches Infill with the unchanged metadata index.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/marshal/out.rs::merge_slice_postprocess_into`; `modules/core-modules/infill-linker/src/orchestrate.rs::orchestrate_infill`; `crates/slicer-runtime/src/region_partition.rs::sync_perimeter_infill_areas_into_slice`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/marshal/out.rs`; `modules/core-modules/infill-linker/src/orchestrate.rs`; `crates/slicer-runtime/src/region_partition.rs`.
- Files explicitly out of bounds: infill geometry algorithms, intentional virtual-variant prepass-mask changes, speculative prepaint ShellClassification migration, config-address identity rewrite.
- Blast-radius discipline: existing closed schemas; full-source indexes and modifier parent keep ordered chain, painted wall donor rule remains distinct. ShellClassification prepaint/base-only scheduling precondition is documented, not migrated.
- Expected sub-agent dispatches: question “do real conversion/linker/current partition drivers retain both variants and the intentional prepass-mask behavior?”; scope exact R7 and B4 plus rebuild/freshness; return FACT outside thinking, escaped with redundant guest/log names.
- Context cost: M.
- Authoritative docs: `design.md` Confirmed downstream source joins; `docs/04_host_scheduler.md` current-products contract; docs/21/22.
- OrcaSlicer refs: none.
- Verification: AC-R7 and unchanged AC-B4; saved all-target check; real guest rebuild/freshness; retained infill-linker/same-layer regression filters from matrix.
- Exit condition: independent mask/output controls and sequential post-perimeter current-geometry witness pass; no helper-only discharge.

### Step 11A — Traditional planner source keys/grouping

- Task IDs: `TASK-572`, row 01; AC-R8.
- Objective: propagate candidate/source/family/geometry/config exact keys through traditional planner grouping and plan emission.
- Precondition: full-chain prepass ABI/SDK projections closed; Exact-Z source-chain/index decision resolved; R8 native/WASM driver red captured.
- Postcondition: `plan_candidate`, `merge_region_identity_entries_with_plane_aliases`, declined constructors and `candidate_family` keep source chain; opaque IDs and geometry unchanged.
- Files allowed to read, with ranges when over 300 lines: `modules/core-modules/traditional-support-planner/src/lib.rs::{plan_candidate,candidate_family,clip_to_territory,canonical_support_family,grid_index_at_plane,merge_region_identity_entries_with_plane_aliases}` and direct model_occupancy/termination_surfaces scans; named family tests, ±40-line windows.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support-planner/src/lib.rs`; `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`; `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs` traditional controls.
- Files explicitly out of bounds: algorithm/physical plane policy changes, pair-only family/source query fallback, opaque ID regeneration.
- Blast-radius discipline: SDK constructors are distinct from IR entries; migrate full-source candidate_family, direct occupancy/termination filters, region_territory/territory_partition calls, grid-index and identity/reference maps. Coarse entry-index/body-reference and previous-by-body semantics stay intact; canonical_support_family remains resolved config interpretation, not inheritance. Schema literal migration allocated S3; test FRU/waivers retain assertions.
- Expected sub-agent dispatches: question “does traditional planner retain exact full-source family/config/geometry through emitted and declined plan entries?”; scope driver and named planner filters; return FACT outside thinking, escaped with redundant source/leg names.
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` analysis/plan identity; `docs/03_wit_and_manifest.md` prepass access; docs/21/22.
- OrcaSlicer refs: none; no algorithm port.
- Verification: AC-R8 traditional leg; existing registered traditional-family filters with saved log/nonzero check; guest rebuild/freshness and all-target compile.
- Exit condition: distinct ordered-chain sources retain distinct family/geometry/config and plan provenance in actual native/WASM calls.

### Step 11B — Tree planner source keys/grouping

- Task IDs: `TASK-572`, row 01; AC-R8.
- Objective: repair tree planner source/group/family joins and plan constructors without changing branch geometry.
- Precondition: full-chain prepass ABI/SDK and Q1/Q2 full-chain Exact-Z/object-wide-geometry policy implemented within finite window; tree controls registered and compiling/fresh for behavioral exit.
- Postcondition: `plan_for_object`, `merge_synthesized_region_row`, `select_coarse_source_entries`, `candidate_family` retain exact source chain and body/demand membership.
- Files allowed to read, with ranges when over 300 lines: `modules/core-modules/tree-support-planner/src/lib.rs::{TreeVolumes::new,plan_for_object,candidate_family,merge_synthesized_region_row,select_coarse_source_entries,group_branches_into_columns}` and direct family-assignment/model-occupancy/territory scans; family tests, ±40-line windows.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support-planner/src/lib.rs`; `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`; `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs` tree controls.
- Files explicitly out of bounds: branch algorithm/coarse grid policy, chainless sibling queries, changing opaque IDs or original negative assertions.
- Blast-radius discipline: classify SDK versus IR entry aliases in tests; use their actual valid bases and approved closed schema shapes. Allocate TreeVolumes::new's family_of call, candidate_family, missing-region/ownership assignment scans, direct occupancy filtering and region_territory calls to this production path. Preserve entry-reference/index grouping; documented superseded smooth_branches is not claimed live production evidence. Inline tree_analysis SDK assignment and coarse_source_selection_prefers_body_per_membership fixture remain owned here; S1/S2 own external fixtures.
- Expected sub-agent dispatches: question “does tree planner grouping/query/emission preserve full source chain and distinct body/demand membership?”; scope actual driver/tree filters; return FACT outside thinking, escaped with redundant full-source/verdict names.
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` support source/plan contracts; `docs/03_wit_and_manifest.md` prepass reads; docs/21/22.
- OrcaSlicer refs: none.
- Verification: AC-R8 tree planner leg; retained tree-family/decline/group controls via saved nonzero filters; guest rebuild/freshness and all-target compile.
- Exit condition: chain variants remain distinct through actual planner output; original geometry/opaque IDs/negatives retained.

### Step 12 — Both support renderers and full-origin carrier transport

- Task IDs: `TASK-572`, row 01; AC-R3/R8 and retained AC-B2.
- Objective: consume exact chain-aware queries in traditional/tree rendering and preserve nonempty-chain absent-source carrier to committed SupportEntry.
- Precondition: chain-bearing plan/selection/shared transport and specialist resources closed; R3/R8 behavioral controls registered.
- Postcondition: same-full-origin entries keep separate contexts/demands/bodies but one wire handle; different chains keep separate handles; renderers/output preserve exact plan source chain.
- Files allowed to read, with ranges when over 300 lines: `modules/core-modules/traditional-support/src/lib.rs::run_support`; `modules/core-modules/tree-support/src/lib.rs::run_support`; private transport carrier assembly in `crates/slicer-wasm-host/src/marshal/transport.rs`; existing AC-B2 three-control fixture, ±40-line windows.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support/src/lib.rs`; `modules/core-modules/tree-support/src/lib.rs`; `crates/slicer-wasm-host/src/marshal/transport.rs` full-origin presence/dedup only.
- Files explicitly out of bounds: carrier model RegionKey/coloring grant, forced empty chain, anchor/global-layer predicate changes, support geometry algorithms.
- Blast-radius discipline: closed variant/output shapes; preserve exclusion by wrong family/global layer/decline/empty role geometry/full identity present even with empty polygons. Carrier config uses filtered invocation authority only.
- Expected sub-agent dispatches: question “do both renderers and transport retain nonempty carrier chain and full-origin absence/dedup while preserving all original exclusions?”; scope R3/R8 and B2; return FACT/SNIPPETS outside thinking, escaped with redundant carrier/leg names.
- Context cost: M.
- Authoritative docs: `design.md` carrier/donor rules; `docs/04_host_scheduler.md` delivery; docs/21/22.
- OrcaSlicer refs: none.
- Verification: owned AC-R3/R8 and unchanged AC-B2 command; `run_exact slicer-wasm-host contract source_identity_repair_tdd::deduplicated_carrier_preserves_plan_provenance`; both real guests rebuilt/fresh. B2 selection global-7/anchor-3 fixture claims delivery/config only; independent global-7/anchor-7 support and raft controls emit nonempty paths/points on both legs.
- Exit condition: two same-origin contexts/one wire handle and additional nonempty-chain origin/two total handles observed; both planners/renderers/query/output chain path proven by R8, not inferred from R3 alone.

### Step 13 — Lightning source query/generator and raft end-to-end witness

- Task IDs: `TASK-572`, row 01; AC-R9 and retained lightning/raft regressions.
- Objective: finish full-source lightning query consumption and raft output commit through generator/SDK/WIT/native/WASM production paths.
- Precondition: I5 generator, W resources/facade, I6 conversion and Step 9 full-key commit closed; runtime R9 registered in correct executor binary.
- Postcondition: independently distinct lightning segments and raft regions preserve source ID/chain; missing-chain sibling query cannot succeed.
- Files allowed to read, with ranges when over 300 lines: `modules/core-modules/lightning-infill/src/lib.rs::run_infill`; `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs::commit_lightning_tree_ir_builtin`; SDK/query/generator symbols allocated in I/W, ±40-line windows.
- Files allowed to edit (at most 3): `modules/core-modules/lightning-infill/src/lib.rs`; `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`; `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs` R9 witness.
- Files explicitly out of bounds: producer map-space normalization, row 08 preparation lightning migration, guessed chain adapter, raft-layer inference from index.
- Blast-radius discipline: closed LightningTreeEntry/InfillRaftRegion schemas; exact source query calls and output full-key joins, no new output builder method.
- Expected sub-agent dispatches: question “does actual generator/query/commit roundtrip retain both source chains and reject chainless sibling reads?”; scope corrected R9 and retained lightning/raft filters; return FACT outside thinking, escaped with redundant runtime/leg/log names.
- Context cost: M.
- Authoritative docs: `docs/02_ir_schemas.md` LightningTreeIR/InfillIR; `docs/03_wit_and_manifest.md` specialist query contract; docs/21/22.
- OrcaSlicer refs: none.
- Verification: `run_exact slicer-runtime executor source_identity_repair_tdd::lightning_and_raft_full_source_roundtrip`; original lightning per-region/guest calls and standalone raft accessor matrix; real guest freshness and all-target check.
- Exit condition: R9 executes once in runtime executor and both native/WASM commit source full keys; all retained lightning/raft assertions remain.

## Window W — coordinated versioned WIT migration batches

Execute W1–W14 within the approved finite integration boundary before schema-dependent behavioral verification in Steps 8–13; register fixtures and migrate required consumer signatures within the finite roster as explicitly described above. Every batch below repeats the template fields independently. Derive exact nonzero-major shared-package and changed stage versions at use from the approved package policy/canonical disk state; no numeric destination is reserved here. Changed support/infill/prepass-support stage signatures take MAJOR versions. Core has no specialist dependency; support owns raft-plan; lightning is independent. Only Layer::Support takes support queries and Layer::Infill takes support/raft plus lightning params. SDK stage method parameter lists remain unchanged; macro composes facade resources. No dual-version workaround.

### Step W1 — Core and two specialist packages

- Task IDs: `TASK-572`, row 01; AC-R10/R8/R9.
- Objective: version core, move support/raft and lightning records/resources into independent specialists with exact ordered-chain queries.
- Precondition: ownership/window BLOCKS resolved and OLD component preserved before canonical changes.
- Postcondition: specialists import core IDs/paint-value/geometry; core retains views/builders/paint/is-raft/region raft-fill without specialist imports.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-schema/wit/deps/ir-types.wit::ir-handles`, located view/query/builder record windows ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-schema/wit/deps/ir-types.wit`; NEW `crates/slicer-schema/wit/deps/support-queries/support-queries.wit`; NEW `crates/slicer-schema/wit/deps/lightning-queries/lightning-queries.wit`.
- Files explicitly out of bounds: new output builder-chain method, core-to-specialist dependency, OLD artifact rebuilding.
- Blast-radius discipline: Window W owns breaking resource/core version fallout across W2–W14 and the fully enumerated remaining allocations under the approved coordinated staging policy; W1 alone is not a compile or compatibility gate.
- Expected sub-agent dispatches: question “are record/resource ownership, source chains and dependency direction correct?”; scope three WIT packages; return FACT outside thinking, escaped and redundant package names.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` WIT organization; ADR-0044/0045; `design.md` split contract.
- OrcaSlicer refs: none.
- Verification: bounded canonical package/ownership review; real compile/typed gate at W14, no intermediate green claim.
- Exit condition: W1 migration review complete; coupled compile state explicitly remains inside Window W.

### Step W2 — Common and consuming stage signatures

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: migrate common transitive core identity and add specialist params only to consuming stage exports.
- Precondition: W1 chosen package/records fixed; changed stage versions rederived at use.
- Postcondition: common extrusion-line uses versioned core; infill/support signatures match allocated specialist ownership and MAJOR stage versions.
- Files allowed to read, with ranges when over 300 lines: exact WIT files below, complete short interfaces or located ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-schema/wit/deps/common.wit`; `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit`; `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`.
- Files explicitly out of bounds: specialist params in nonconsumers, SDK stage parameter lists, rewriting historical probes.
- Blast-radius discipline: common changes affect finalization/postpass aliases transitively; W7 owns all host bindgen roots, not just layer consumers.
- Expected sub-agent dispatches: question “are consuming params/major versions and common transitive identity coherent?”; scope three interfaces; return FACT outside thinking, escaped and redundant version/consumer names.
- Context cost: S.
- Authoritative docs: `docs/03_wit_and_manifest.md` stage/WIT organization; ADR-0045 Decision.
- OrcaSlicer refs: none.
- Verification: signature/import review; actual typed calls at W14; intermediate compile errors are migration state, never red.
- Exit condition: W2 exact import/signature migration reviewed; no new SDK method arity.

### Step W3 — Slice and perimeter core imports

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: migrate direct non-specialist core imports coherently.
- Precondition: versioned core syntax agreed with actual parser/bindgen dialects.
- Postcondition: three worlds retain core-only identity and original stage behavior.
- Files allowed to read, with ranges when over 300 lines: exact stage interfaces below, short-file reads or located ±40-line import windows.
- Files allowed to edit (at most 3): `crates/slicer-schema/wit/deps/layer-slice-postprocess/layer-slice-postprocess.wit`; `crates/slicer-schema/wit/deps/layer-perimeters/layer-perimeters.wit`; `crates/slicer-schema/wit/deps/layer-perimeters-postprocess/layer-perimeters-postprocess.wit`.
- Files explicitly out of bounds: specialist params/imports, unrelated world signatures, algorithm behavior.
- Blast-radius discipline: nominal core import coupling allocated to W7/W8; no standalone compile compliance claim.
- Expected sub-agent dispatches: question “do these worlds share exact core identity without specialists?”; scope three WIT files; return FACT outside thinking, escaped and redundant world names.
- Context cost: S.
- Authoritative docs: `docs/03_wit_and_manifest.md` canonical imports; ADR-0045.
- OrcaSlicer refs: none.
- Verification: bounded import review; all-target/wasm/typed gate at W14.
- Exit condition: exact core use/world imports reviewed, no specialist leak.

### Step W4 — Postprocess/path core imports

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: migrate direct core imports in remaining layer postprocess/path worlds.
- Precondition: W1 versioned core agreed; nonconsumer helper scoping planned.
- Postcondition: postprocess/path interfaces retain stage behavior with shared core identity only.
- Files allowed to read, with ranges when over 300 lines: listed interfaces, short reads or located ±40-line import windows.
- Files allowed to edit (at most 3): `crates/slicer-schema/wit/deps/layer-infill-postprocess/layer-infill-postprocess.wit`; `crates/slicer-schema/wit/deps/layer-support-postprocess/layer-support-postprocess.wit`; `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit`.
- Files explicitly out of bounds: specialist params in these stages, SDK method signatures, retained assertion weakening.
- Blast-radius discipline: generated aliases/helpers must be stage-scoped in W7, not repaired by adding specialist imports here.
- Expected sub-agent dispatches: question “do these nonconsumer worlds retain core-only imports and correct identity?”; scope three files; return FACT outside thinking, escaped and redundantly named.
- Context cost: S.
- Authoritative docs: `docs/03_wit_and_manifest.md` WIT organization; ADR-0045.
- OrcaSlicer refs: none.
- Verification: import/arity review; closing compilation/real load at W14.
- Exit condition: exact core migration reviewed; no mid-window green claim.

### Step W5 — Anchored and prepass core imports

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: migrate core references without introducing specialist params into nonconsumers.
- Precondition: chosen core identity and common transitive closure agreed.
- Postcondition: anchored/mesh/layer-planning worlds retain intended stage-specific interfaces and shared nominal core.
- Files allowed to read, with ranges when over 300 lines: listed WIT files, short reads or located ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit`; `crates/slicer-schema/wit/deps/prepass-mesh-analysis/prepass-mesh-analysis.wit`; `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`.
- Files explicitly out of bounds: preparation lifecycle declarations, anchored synthetic closure config, producer inheritance.
- Blast-radius discipline: keep source record fallout coordinated with approved prepass roster; no blanket stage-version substitution.
- Expected sub-agent dispatches: question “are the exact three core import migrations coherent without unrelated signature changes?”; scope listed worlds; return FACT outside thinking, escaped with redundant names.
- Context cost: S.
- Authoritative docs: `docs/03_wit_and_manifest.md` per-stage prepass/anchored contracts; ADR-0045.
- OrcaSlicer refs: none.
- Verification: bounded canonical review; typed/load gates at W14.
- Exit condition: core-only import closure migrated, retained stage behavior unchanged.

### Step W6 — Prepass support chain ABI and schema stage metadata

- Task IDs: `TASK-572`, row 01; AC-R8/R10.
- Objective: carry full chains through support analysis/geometry/config/family/segmentation/plan ABI and resolved Exact-Z source query, with appropriate major stage metadata.
- Precondition: actual Exact-Z index/query callers traced, SDK/IR identities and chain record policy resolved.
- Postcondition: chain-bearing prepass records use core paint-value/IDs; stage metadata matches canonical signatures and at-use version decisions.
- Files allowed to read, with ranges when over 300 lines: support-geometry-types nested records/query interface and `crates/slicer-schema/src/lib.rs::{STAGES,qualified_export_for_stage_id}`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-schema/wit/deps/prepass-seam-planning/prepass-seam-planning.wit`; `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`; `crates/slicer-schema/src/lib.rs`.
- Files explicitly out of bounds: duplicate paint enum, guessed Exact-Z semantics, rewriting root anchor merely for cosmetic version cleanup.
- Blast-radius discipline: STAGES and hard export/version assertions are allocated W10/remaining inventory; breaking prepass record shapes take MAJOR. `root.wit` has no import/type migration; `prepass-types.wit` changes only if actual record inventory proves necessary and a bounded extra batch is approved.
- Expected sub-agent dispatches: question “do nested support records and Exact-Z lookup preserve source identity with coherent stage metadata?”; scope listed record/query/schema symbols; return FACT/LOCATIONS outside thinking, escaped and redundant exact-query decision.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` prepass contracts; `docs/02_ir_schemas.md` version table; ADR-0045.
- OrcaSlicer refs: none.
- Verification: boundary review, schema metadata assertions at W14; no compile-only proof of Exact-Z correctness.
- Exit condition: explicit source-chain query/index decision implemented within approved paths; untraced downstream site is BLOCK, not omitted work.

### Step W7 — Host nominal maps, dispatch calls and stage-scoped macro facade

- Task IDs: `TASK-572`, row 01; AC-R5/R8/R9/R10.
- Objective: coordinate actual core/specialist resource identity, host allocations/traits, linker call params and wasm-only macro facade composition.
- Precondition: canonical WIT W1–W6 complete; actual importing-world specialist type roots selected, no assumption perimeters imports specialists.
- Postcondition: all host worlds share one nominal core; support/lightning have owned resources/lifetime/read tracking; only consuming glue accesses specialists behind unchanged SDK paint facade.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-wasm-host/src/host.rs` every named bindgen module/with-map/HostPaintRegionLayerView and support-output/Exact-Z impl, symbol-local ±40 lines; `crates/slicer-wasm-host/src/dispatch.rs::{dispatch_layer_call,build_paint_layer_data_with_plan}`; macro symbols below, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/src/host.rs`; `crates/slicer-wasm-host/src/dispatch.rs`; `crates/slicer-macros/src/lib.rs`.
- Files explicitly out of bounds: SDK stage parameter lists, dual-version generated namespace, speculative `emit_layer_interface_guest_impl` (not an existing symbol), guessing missing chain/ID.
- Blast-radius discipline: migrate every bindgen qualified `with` key and type alias, including common-transitive finalization/postpass. Correct key dialect is version before `.resource`. Preserve table/drop and runtime_reads side effects. `dispatch_prepass_call` supplies its existing authoritative `slice_ir` to repaired `project_region_segmentation_view`; Q1 owns exact-Z Host query input chain. `emit_world_preamble`, `layer_per_stage_aliases`, `layer_light_helpers`, `build_layer_infill_glue`, `build_layer_support_glue`, `build_prepass_support_geometry_glue`, generated `__slicer_adapt_paint_layer`, `__slicer_support_plan_from_view`, `__slicer_lightning_tree_from_view` are actual seams. Scope aliases/helpers by stage; preserve reconstructed support SliceIR chain, per-origin segmentation/config rows and canonical ID rejection.
- Expected sub-agent dispatches: question “are every host alias/resource and wasm-only consuming/nonconsuming macro signature migrated with same nominal core?”; scope separately bounded bindgen and macro symbol LOCATIONS workers; return LOCATIONS/FACT outside thinking, escaped with redundant nominal identity names.
- Context cost: M; split symbol work into bounded workers under the same exact edit ownership, not an L full-file read.
- Authoritative docs: `docs/03_wit_and_manifest.md` canonical generation/access isolation; ADR-0044/0045; docs/21/22.
- OrcaSlicer refs: none.
- Verification: batch review only; W14 builds ALL guests and real typed resource calls, because host check misses wasm-only generated signatures.
- Exit condition: finite named host/macro migration reviewed; no core import of specialists, resource leak or helper leak into nonconsumers.

### Step W8 — Embed watchers, verifier and raw infill guest

- Task IDs: `TASK-572`, row 01; AC-R9/R10.
- Objective: register specialist embedded closure/watchers, verifier shared classification/nominal version checks and actual raw guest run arity/query calls.
- Precondition: canonical signatures/resource roots settled; raw versus macro guests explicitly classified.
- Postcondition: closure verification recognizes specialists as shared packages; raw guest uses support/raft/lightning resource params and preserves existing witnesses.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-macros/build.rs::main`; `xtask/src/wit_verify.rs::{SHARED_PACKAGES,macro_embedded_wit_files,resolve_stage_from_world,compare_worlds,find_canon_pkg}` and named unit tests; raw guest `Guest::run`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-macros/build.rs`; `xtask/src/wit_verify.rs`; `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs`.
- Files explicitly out of bounds: weakening shared-subset checks, treating version-stripped package matching as typed rejection evidence, globally rewriting deliberate OLD miniature WIT controls.
- Blast-radius discipline: verifier/core nominal-version tests and embed-file closure assertions owned; W8 exclusively owns `xtask/src/wit_verify.rs` miniature comparison expectation isolation (`make_expect`/`canonical_pair`, exact export-version negative oracle) and live stage-resolution assertion adaptation, coordinated with V5 while preserving synthetic inputs/assertions and production nominal-version checks without frozen future numeric destinations. `xtask/src/build_guests.rs` test-expectation isolation is exclusively V5, not a concurrent W8 edit or blind string replacement.
- Expected sub-agent dispatches: question “does verifier classify specialists and detect nominal version drift while raw guest preserves lightning/raft observations?”; scope named verifier tests/raw guest; return FACT outside thinking, escaped with redundant witness names.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` artifact contracts; ADR-0044/0045; docs/22.
- OrcaSlicer refs: none.
- Verification: metadata in this authoring session grounds package `xtask`, binary target `xtask`, no required feature gate. At W14: `run_checked cargo test -p xtask --bin xtask wit_verify::tests::`; save complete-log nonzero/zero-failure findings before overwrite; retain embed/watchers/shared-subset/export-version tests and add exact nominal-core/specialist cases. Real raw guest tests retain matrix commands; typed compatibility is separate from verifier classification.
- Exit condition: watcher/verifier/raw migration reviewed and grounded binary filters execute at W14; no missing-target placeholder remains.

### Step W9 — Native facade and chain-bearing prepass projections

- Task IDs: `TASK-572`, row 01; AC-R8/R9/R10.
- Objective: reconcile native paint facade and both projection/harvest boundaries with migrated specialist/chain contracts.
- Precondition: I2/I4 source types and W7 resources agreed; no new SDK stage arity.
- Postcondition: support/lightning queries use exact object/source/chain; support anchor and lightning global-layer filters remain distinct; raft query stays print-wide.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView`; `crates/slicer-wasm-host/src/marshal/native.rs::{finish_native_layer_request,support_analysis_view_from_ir,build_native_prepass_request}`; `crates/slicer-wasm-host/src/marshal/in_.rs::{project_region_segmentation_view,project_support_analysis_view,project_support_geometry_view,harvest_support_plan_ir_from}`, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-sdk/src/traits.rs`; `crates/slicer-wasm-host/src/marshal/native.rs`; `crates/slicer-wasm-host/src/marshal/in_.rs`.
- Files explicitly out of bounds: support global/anchor conflation, stage method parameter-list changes, pair-only query fallback, duplicate production assembly.
- Blast-radius discipline: coordinate same symbols with I2/I4 instead of parallel conflicting edits; every IR/generated constructor remains exhaustive in production. Full-source query parameters are not new stage method params.
- Expected sub-agent dispatches: question “does native facade match specialist WIT query semantics and preserve original layer/anchor/raft rules?”; scope these boundaries; return FACT outside thinking, escaped and redundantly identifying filtering semantics.
- Context cost: M.
- Authoritative docs: `docs/03_wit_and_manifest.md` query/access contracts; `docs/02_ir_schemas.md` plan/lightning source records; docs/21/22.
- OrcaSlicer refs: none.
- Verification: original support/lightning/raft matrix plus R8/R9 at W14; host-only cfg native checks cannot prove guest glue.
- Exit condition: exact facade/projection migration reviewed; all real calls still required at W14.

### Step W10 — Hard stage/version and scaffold assertions

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: migrate load-bearing hard assertions to the approved breaking contract without dropping protections.
- Precondition: stage versions rederived and canonical contract final; old-location/source-grep assertion cannot serve as sole specialist behavior proof.
- Postcondition: tests protect actual specialist query location/arity and qualified stage versions; generated module scaffold export assertions match schema authority.
- Files allowed to read, with ranges when over 300 lines: exact test symbols below, ±40-line windows.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`; `crates/slicer-macros/tests/binding_surface_tdd.rs`; `crates/pnp-cli/tests/module_new_tdd.rs`.
- Files explicitly out of bounds: deletion of old-value protection, arbitrary version pins, modification of intentional old artifact compatibility controls.
- Blast-radius discipline: `paint_region_layer_view_has_lightning_tree_segments_method`, `layer_infill_package_version_is_load_bearing`; macro `layer_module_wit_exports_only_detected_stage`, `binding_schema_json_captures_full_export_surface`, `typed_schema_const_mirrors_string_accessors_for_layer_infill`, `typed_schema_exports_are_deterministic_across_invocations`; CLI `stage_layer_infill_scaffolds_its_per_stage_package` are known assertion seams. Any further schema/import version assert joins the cleared finite roster before edits.
- Expected sub-agent dispatches: question “do all hard assertions still falsify stage/nominal drift rather than merely disappear?”; scope named tests; return FACT outside thinking, escaped with redundant assertion/target names.
- Context cost: M.
- Authoritative docs: ADR-0044/0045; `docs/03_wit_and_manifest.md` stage identity; docs/22.
- OrcaSlicer refs: none.
- Verification: at W14 `run_filter slicer-runtime contract wit_drift_detection_tdd::`; `run_filter slicer-macros binding_surface_tdd layer_module_wit_exports_only_detected_stage`; remaining named macro/CLI filters using grounded crate/target metadata and saved nonzero log inspection.
- Exit condition: protection retained and every actual target grounded; no “test removed so compile passes” acceptance.

### Step W11 — Captured immutable OLD component and actual rejection witness

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: preserve genuine OLD same-stage preinvoke rejection/migrated resource calls and prove actual rebuilt Dragon Curve production typed instantiate/infill run; OLD rejection does not isolate stage version versus core import identity.
- Precondition: Step 1 produced `crates/slicer-wasm-host/tests/fixtures/pre-split-layer-infill.component.wasm` from the isolated existing baseline guest, with embedded same-stage/unversioned-core verification and provenance receipt; finite window approved. Allocation is settled; capture/compatibility is future checked evidence, not assumed.
- Postcondition: positive actual support/infill typed loads/query calls share core resource identity; old same-stage component fails clearly before run/callback/output commit.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs::stage_miss_is_fatal_at_instantiation`; `crates/slicer-wasm-host/src/dispatch.rs::dispatch_layer_call` typed instantiate/error construction, ±40-line windows; OLD artifact metadata via bounded FACT only after allocation.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/tests/contract/source_identity_repair_tdd.rs`; `crates/slicer-wasm-host/tests/contract/main.rs`; NEW `crates/slicer-wasm-host/tests/fixtures/pre-split-layer-infill.component.wasm` only by Step 1 verified capture, immutable thereafter. Existing dispatch-protocol diagnostics, if verified to need repair, are owned separate V4 rather than a fourth path here.
- Files explicitly out of bounds: rebuilding OLD into compatibility, asserting export-miss alone proves old-core rejection, dual-version adapter, guessed engine wording.
- Blast-radius discipline: typed counter/witness fixture owns actual invocation/no-commit controls; no new production resource field. The exact proposed artifact allocation is grounded in design and need not preexist activation; capture failure blocks migration/acceptance, not permission to synthesize a wrong-stage fixture.
- Expected sub-agent dispatches: question “do migrated actual resource calls succeed and frozen same-stage old component reject before invocation?”; scope AC-R10 plus allocated artifact; return FACT/SNIPPETS outside thinking, escaped and redundantly naming old/new/count/log.
- Context cost: M.
- Authoritative docs: ADR-0044/0045; `docs/03_wit_and_manifest.md` actual typed compatibility; docs/22.
- OrcaSlicer refs: none.
- Verification: owned AC-R10, `run_exact slicer-wasm-host contract source_identity_repair_tdd::versioned_query_resources_load_and_old_components_reject`; retain production dispatch-protocol filter with saved nonzero execution. No hypothetical future specialist-version isolation experiment.
- Exit condition: captured fixture identity/provenance validated and real nominal/resource positive plus preinvoke negative witnesses execute; capture failure/missing file is BLOCK, not expected rejection success or package-layout inference.

### Step W12 — Live Dragon Curve canonical closure and snapshot

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: migrate labeled live example's vendored canonical infill/core closure and snapshot package roster.
- Precondition: example rebuild/tool-generated allocation and allowed exception to generated-files boundary resolved before execution; no actual community module is added.
- Postcondition: snapshot closure includes both specialists; live infill/core copies match migrated canonical bytes.
- Files allowed to read, with ranges when over 300 lines: `crates/slicer-schema/tests/dragon_curve_wit_snapshot_tdd.rs::{PACKAGES,dragon_curve_vendored_wit_matches_canonical,dragon_curve_world_matches_canonical}`, located windows; live WIT files below.
- Files allowed to edit (at most 3): `crates/slicer-schema/tests/dragon_curve_wit_snapshot_tdd.rs`; `modules/community-modules/dragon-curve/wit/layer-infill.wit`; `modules/community-modules/dragon-curve/wit/deps/ir-types/ir-types.wit`.
- Files explicitly out of bounds: historical foreign-language WIT/Go probe, unallocated generated bindings, claiming snapshot equality rebuilds MoonBit component.
- Blast-radius discipline: PACKAGES closure grows for specialist imports, not old four-dependency copy only; E1/generated/artifact batches separately owned.
- Expected sub-agent dispatches: question “does live example snapshot include full migrated dependency closure?”; scope canonical/example WIT and snapshot; return FACT outside thinking, escaped with redundant closure scope.
- Context cost: S.
- Authoritative docs: `docs/03_wit_and_manifest.md` canonical WIT; `docs/14_submodule_programming_languages.md` labeled example semantics; docs/22.
- OrcaSlicer refs: none.
- Verification: at W14 `run_filter slicer-schema dragon_curve_wit_snapshot_tdd dragon_curve_`; real MoonBit artifact rebuild/load remains additional mandatory evidence.
- Exit condition: closure snapshot reviewed; no artifact freshness/typed-load claim from byte equality.

### Step W13 — Live example common and specialist copies

- Task IDs: `TASK-572`, row 01; AC-R10.
- Objective: complete live example's common transitive core and specialist WIT closure.
- Precondition: generator dependency-directory spelling confirmed against actual resolver; canonical W1/W2 finalized.
- Postcondition: example common/specialist copies exactly match canonical packages and share core nominal identity.
- Files allowed to read, with ranges when over 300 lines: three example files below and canonical matching interfaces, short files or located ±40-line windows.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/wit/deps/common/common.wit`; NEW `modules/community-modules/dragon-curve/wit/deps/support-queries/support-queries.wit`; NEW `modules/community-modules/dragon-curve/wit/deps/lightning-queries/lightning-queries.wit` only after resolver spelling confirmation.
- Files explicitly out of bounds: new generated MoonBit package names guessed from WIT spelling, historical probe files, pretending default build-guests discovers example.
- Blast-radius discipline: exact proposed WIT paths resolved in sandbox; canonical compilation remains future evidence. E1 owns authored inputs, EG01/02/05/06/07/08/09 measured changed/new bindings, EG13 sequential template overlay, EB01 separate artifact; no hidden tooling fourth path.
- Expected sub-agent dispatches: question “do canonical/example dependencies resolve identically with correct generator directory names?”; scope WIT copies/tool resolver metadata; return FACT outside thinking, escaped and redundantly naming resolution result.
- Context cost: S.
- Authoritative docs: `docs/03_wit_and_manifest.md` nested-package layout; `docs/14_submodule_programming_languages.md`; docs/22.
- OrcaSlicer refs: none.
- Verification: snapshot at W14 plus separate actual MoonBit regeneration/build; no direct generated-tree load.
- Exit condition: exact copy paths/resolution validated; tooling unavailable or new filenames unresolved is BLOCK, not artifact success.

### Step E1 — Authored example generator/glue inputs

- Task IDs: TASK-572 / row 01; AC-R10.
- Objective: migrate authored inputs separately from generated output/artifact writes.
- Precondition: cold preflight accepted, explicit activation, W1/W2/W12/W13 agreed; remeasure roster if canonical inputs/toolchain differ.
- Postcondition: explicit specialist inputs and owning glue agree with canonical signatures; SDK stage parameters unchanged.
- Files allowed to read, with ranges when over 300 lines: Makefile/templates below; canonical infill/specialist signatures; generated run/import windows ≤600 lines only.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/Makefile`; `modules/community-modules/dragon-curve/src/glue/main.mbt.in`; `modules/community-modules/dragon-curve/src/glue/moon.pkg.json.in`.
- Files explicitly out of bounds: live generated/artifact paths, algorithm sources, historical probes, SDK stage signatures, broad deletes.
- Blast-radius discipline: generation accepts explicit scratch out-dir and config/types/ir-types/common/support-queries/lightning-queries/layer-infill WIT inputs, `--derive-show --derive-eq --derive-error -w slicer:layer-infill/infill-module`. Generate/build only in explicitly copied scratch project, not live make/bindings/clean. Glue gains owning specialist parameters/imports; final template manifest retains specialist imports. Copy only later NEEDCOPIED groups; unexpected deltas stop for reauthoring.
- Expected sub-agent dispatches: independent bounded input worker; escaped FACT/SNIPPETS outside thinking, redundant exact-path receipt.
- Context cost: M.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: staged input/resource/run/import review; actual compilation/component/resource/example-load gates at W14, not E1 PASS.
- Exit condition: input migration reviewed with no unallocated writes; coupled W14 mandatory.

### Generated-example batch controls

Every EG batch below uses the same approved fresh scratch generation receipt and sequential live ownership inspection; no generator/broad cleanup runs in live tree. Exact unsuffixed NEEDCOPIED paths are repeated in each step. Unchanged G03/G04/G10/G11/G12 and unchanged siblings of G05/G06/G07 are not copied; preserve moon.mod.json/world metadata/live user work. Each batch retains all template fields; its verification is migration review only until W14 all-target compile, cargo build --tests, owning tests, ordinary guest build/freshness, real nominal resource calls/OLD rejection and actual MoonBit component build/load. No intermediate compiler failure is behavioral red.

### Step EG01 — Changed export ABI group
- Task IDs: TASK-572 / 01; AC-R10.
- Objective: copy changed export ABI outputs.
- Precondition: E1/W12/W13 staged, activation/preflight granted, generation receipt/live ownership checked.
- Postcondition: only listed changed outputs match staging.
- Files allowed to read, with ranges when over 300 lines: listed live/staged metadata and export symbol windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/gen/ffi.mbt`; `modules/community-modules/dragon-curve/gen/interface/slicer/layer-infill/infill/ffi.mbt`; `modules/community-modules/dragon-curve/gen/interface/slicer/layer-infill/infill/moon.pkg.json`.
- Files explicitly out of bounds: other outputs/artifacts/broad cleanup.
- Blast-radius discipline: changed outputs only; final manifest overlaid sequentially EG13.
- Expected sub-agent dispatches: independent copier/reviewer; escaped FACT outside thinking, redundant copied/skipped paths.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md.
- OrcaSlicer refs: none.
- Verification: exact copy receipt and shared mandatory W14 controls.
- Exit condition: migration review complete, no independent green.

### Step EG02 — Changed export/core-host group
- Task IDs: TASK-572 / 01; AC-R10.
- Objective: copy changed export/host-service outputs.
- Precondition: EG01 reviewed; approved generation receipt/activation/live ownership.
- Postcondition: three listed outputs match staging.
- Files allowed to read, with ranges when over 300 lines: listed metadata/ABI windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/gen/interface/slicer/layer-infill/infill/top.mbt`; `modules/community-modules/dragon-curve/gen/moon.pkg.json`; `modules/community-modules/dragon-curve/interface/slicer/common/host-services/ffi.mbt`.
- Files explicitly out of bounds: moon.mod.json, unchanged host-service siblings, artifacts.
- Blast-radius discipline: unsuffixed changed paths only, no roster-as-overwrite permit.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant exact receipt.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md.
- OrcaSlicer refs: none.
- Verification: staged copy review/shared W14 controls.
- Exit condition: reviewed migration only; W14 closes.

### Step EG05 — Changed profiling member
- Task IDs: TASK-572 / 01; AC-R10.
- Objective: copy changed profiling ABI only.
- Precondition: EG02 reviewed; staged receipt/activation/live ownership.
- Postcondition: one output matches staging, siblings preserved.
- Files allowed to read, with ranges when over 300 lines: listed metadata/ABI windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/interface/slicer/common/profiling/ffi.mbt`.
- Files explicitly out of bounds: profiling package/top/README and other outputs/artifacts.
- Blast-radius discipline: changed member only, unchanged emission not edit authority.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant copied path/preserved siblings.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md.
- OrcaSlicer refs: none.
- Verification: exact receipt/shared W14 controls.
- Exit condition: review only; no intermediate green/red.

### Step EG06 — Changed config member
- Task IDs: TASK-572 / 01; AC-R10.
- Objective: copy changed config ABI only.
- Precondition: EG05 reviewed; staged receipt/activation/live ownership.
- Postcondition: one output matches staging, siblings untouched.
- Files allowed to read, with ranges when over 300 lines: listed metadata/ABI windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/interface/slicer/config/config-types/ffi.mbt`.
- Files explicitly out of bounds: config-types package/top and other outputs/artifacts.
- Blast-radius discipline: changed bytes only; preserve unrelated live modifications.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant copy receipt.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md.
- OrcaSlicer refs: none.
- Verification: exact receipt/shared W14 controls.
- Exit condition: review only; W14 mandatory.

### Step EG07 — Changed core members
- Task IDs: TASK-572 / 01; AC-R10.
- Objective: copy changed core bindings without specialist dependency.
- Precondition: EG06 reviewed; staged receipt/activation/live ownership.
- Postcondition: two outputs match staging; core imports no specialist.
- Files allowed to read, with ranges when over 300 lines: listed metadata/core-resource windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/interface/slicer/ir-handles/ir-handles/ffi.mbt`; `modules/community-modules/dragon-curve/interface/slicer/ir-handles/ir-handles/top.mbt`.
- Files explicitly out of bounds: unchanged ir-handles package manifest, other outputs/artifacts.
- Blast-radius discipline: exact two changed members only.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant nominal-resource/path receipt.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; ADR-0044/0045; docs/14_submodule_programming_languages.md.
- OrcaSlicer refs: none.
- Verification: core ownership/copy review/shared W14 controls.
- Exit condition: review only; no independent compile claim.

### Step EG08 — New lightning specialist group
- Task IDs: TASK-572 / 01; AC-R9/R10.
- Objective: copy measured lightning-query-view outputs.
- Precondition: EG07 reviewed; proposed WIT names agreed; staged receipt/activation/live ownership.
- Postcondition: three new outputs match staging/nominal identities.
- Files allowed to read, with ranges when over 300 lines: canonical query signature/listed metadata/query windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/interface/slicer/lightning-queries/lightning-queries/ffi.mbt`; `modules/community-modules/dragon-curve/interface/slicer/lightning-queries/lightning-queries/moon.pkg.json`; `modules/community-modules/dragon-curve/interface/slicer/lightning-queries/lightning-queries/top.mbt`.
- Files explicitly out of bounds: guessed version-suffixed directories, other packages/artifacts.
- Blast-radius discipline: installed-generator emitted bytes, not hand-generated/directory overwrite.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant new paths/resource identity.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md; ADR-0044/0045.
- OrcaSlicer refs: none.
- Verification: exact copy/resolution/shared W14 controls including full-chain query.
- Exit condition: review only; generator success is not typed-load green.

### Step EG09 — New support specialist group
- Task IDs: TASK-572 / 01; AC-R8/R10.
- Objective: copy measured support-query-view outputs/support-owned raft query.
- Precondition: EG08 reviewed; proposed WIT names agreed; staged receipt/activation/live ownership.
- Postcondition: three new outputs match exact core/source/query identities.
- Files allowed to read, with ranges when over 300 lines: canonical query signature/listed metadata/query windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/interface/slicer/support-queries/support-queries/ffi.mbt`; `modules/community-modules/dragon-curve/interface/slicer/support-queries/support-queries/moon.pkg.json`; `modules/community-modules/dragon-curve/interface/slicer/support-queries/support-queries/top.mbt`.
- Files explicitly out of bounds: guessed version-suffixed directories, other packages/artifacts.
- Blast-radius discipline: actual emitted bytes/exact paths; no wholesale directory replacement.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant support/raft/path receipt.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md; ADR-0044/0045.
- OrcaSlicer refs: none.
- Verification: exact copy/resolution/shared W14 controls including support/raft calls.
- Exit condition: review only; no independent green.

### Step EG13 — Authored glue overlay
- Task IDs: TASK-572 / 01; AC-R10.
- Objective: overlay authored specialist glue, not fictional generator output.
- Precondition: E1/all generated groups reviewed; staged run/imports agree; activation.
- Postcondition: main/final export manifest match approved templates in scratch/live.
- Files allowed to read, with ranges when over 300 lines: E1 templates/listed run/export manifest windows ≤600 lines.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/gen/interface/slicer/layer-infill/infill/main.mbt`; `modules/community-modules/dragon-curve/gen/interface/slicer/layer-infill/infill/moon.pkg.json`.
- Files explicitly out of bounds: gen/moon.pkg.json (EG02), other outputs/artifacts/algorithm/SDK stage signatures.
- Blast-radius discipline: sequential manifest writer after EG01; main overlays exported interface, not gen root; final manifest retains specialist imports; overlay scratch too.
- Expected sub-agent dispatches: independent worker; escaped FACT/SNIPPETS outside thinking, redundant exact source/target/arity receipt.
- Context cost: S.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/14_submodule_programming_languages.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: final overlay/import/run review/shared W14 controls.
- Exit condition: review only; no standalone compilation claim.

### Step EB01 — Staged build and component copy
- Task IDs: TASK-572 / 01; AC-R10.
- Objective: build actual labeled MoonBit component without live broad generation/deletes.
- Precondition: WIT/generated/overlay groups reviewed; live-to-staged input equality established preserving unchanged project config; toolchain/activation available.
- Postcondition: binary comes from successful actual staged compile/component chain, input/provenance receipt retained.
- Files allowed to read, with ranges when over 300 lines: actual Makefile build commands/staged metadata/embedded WIT/bounded build-log FACT.
- Files allowed to edit (at most 3): `modules/community-modules/dragon-curve/dragon-curve.wasm` by explicit successful scratch artifact copy only.
- Files explicitly out of bounds: live source/generated files/embedded.wasm/_build, OLD fixture, live broad make targets.
- Blast-radius discipline: in copied scratch project run `moon build --target wasm --release`, `wasm-tools component embed --encoding utf16 -w infill-module wit _build/wasm/release/build/gen/gen.wasm -o embedded.wasm`, `wasm-tools component new -o dragon-curve.wasm embedded.wasm`, decode WIT; transients scratch-only. Bypass broad default make prerequisites; copy only allocated binary after successful checks.
- Expected sub-agent dispatches: independent bounded build worker; escaped FACT outside thinking, redundant command exits/staged-live paths/input receipt.
- Context cost: M.
- Authoritative docs: docs/14_submodule_programming_languages.md; docs/03_wit_and_manifest.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: actual command exits/embedded nominal WIT, W11 production example instantiate/run, mandatory shared W14 gates; decode alone not load proof.
- Exit condition: build/copy reviewed; typed green only at W14.

### Existing-step amendments within unchanged exact edit lists

These amend responsibilities, not original verification commands. All other template fields/AC/matrix/standalone gates remain. Repeated paths have serial distinct surfaces.

| Step | Added responsibility |
| --- | --- |
| 3 | Pure public region_map_key_for_sliced_region factors map_space_region_id/key_for only; no map/config resolver or Layer per-call work/new export module. Inline independently expected address/source-distinction test below, no fourth file |
| I1 | Design's five proposed exact key/JSON/populated actual-owner postcard tests, OLD geometry-key omission versus NEW source-key optional omission, complete consumption/all fields; no loader/old-binary experiment; duplicate identity includes chain |
| I2/I4/W6/W7/W9/Q2 | NEW RegionSourceOrigin vector replaces region_ids retaining object_id/layer_index; WIT retains object-id/layer-index. Existing PrepassStageInput slice/map inputs only; project actual SliceIR sources/config-row chains using Step 3 mapper for temporary address. Macro converts vectors. project_geometry keeps layered SupportGeometryKey map. Q2 real source fixtures/independent row ordering, not reverse-map/source-grep |
| I3 | Capture exact config RegionKey separately from SOURCE through contact_work/Contact/candidates; entries.get then config_for; remove known-origin smallest-chain calls/import, not legacy resolver. All eligible ActiveRegion-driven/no-contact assignments join actual source origins. Geometry keys stay SupportGeometryKey; family keys SupportSourceKey. Producer test below |
| 5 | dispatch.rs::perimeter_region_index full SOURCE/chain keyed; already-correct indexed full_perim_index/selected_model_target/cached tool branches unchanged; explicit own/donor relationships |
| 8/9 | Retain four runtime executor R6/R7/R8/R9 drivers. Step 9 alone repairs lock/post/slice/raft-to-slice joins and ordered support+raft PrintEntity.region_key/inline tool-chain observations; physical side table unchanged |
| 10 | infill-linker matching/RegionRecord/WallGroupKey/wall_group_key/wall_groups/compatible_regions preserve chain before config/tool/shared-wall grouping; partition donor joins retain chain/modifier-parent/virtual masks. Correct emission remains context |
| I6 | Tagged SupportRegion/mint_support_region/outer final exhaustive entry/plan join carry chain; untagged fallback not invented provenance; aggregate ownership/union/sort full origin with physical anchor/collision unchanged; has_territory object/layer guard retained |
| 11B | All former region_ids reads migrate: chain-bearing membership/synthesized/coarse/candidate-family/fallback origin scans; inline tree fixtures coherent, actual planner/output R8 mandatory |
| S1 | Tree IR declined_entry/invalid_body_rejected family/plan fields and SDK origin-vector/assignment/geometry fixtures; foreign_territory outer SDK scope. 11B behavioral controls serial later |
| S2 | IR make_support_entry/make_entry_with_negative_index; make_entry_with_index delegates; smooth_nodes SDK Default false waiver corrected with meaningful FRU/semantic pin; all origin vectors |
| S3 | Traditional disabled_and_declined/invalid_body_rejected IR no-Default/family maps; SDK alias top-level FRU; origin vectors; 11A behavioral controls serial later |
| S4/S5 | S4 IR support/off-grid/carrier equality/lightning fields; positive carrier rest pattern not constructor. S5 two IR lightning roundtrip values/fixture_entry body/receivers; native fixture TF04 only |
| S6 | SDK diagnostic Default-FRU retained; coherent chain witness, not invented compile failure |
| V1/V2 | V1 support_ir_simple exhaustive IR entry plus purpose classification; V2 parity literal/version scaffold; TF01 comparator/negative behavior serial distinct surface |
| V3/V4 | V3 preview key/plan/Blackboard duplicate patterns; V4 integrated lightning/live support_commit/plan_for_obj0/two real-region plans/repeated-run full-chain observation; independent tool/output controls retained |
| I4 | base_interface_role_round_trips_both_legs is exhaustive WIT, harvested plan IR; neither borrows SDK Default |
| W11 | Register proposed source_identity_repair_tdd::dragon_curve_migrated_component_typed_load_and_run in existing host test/main paths; read EB01 receipt/live artifact, actual production infill call with independent committed full-source/config/nonempty output/origin. Missing artifact/tool fails, no skip. Evaluation after EB01/compile/freshness; earlier registration scaffold. Original resource/OLD rejection test still required; OLD rejection does not isolate stage/core cause. Host suite eight already reconciled in requirements |

Additional verification (original commands untouched; run only on compiling/fresh coupled tree, save/read nonzero logs before overwrite):

- Step 3: `run_checked cargo test -p slicer-wasm-host --lib selection::tests::region_map_key_for_sliced_region_preserves_source_chain_and_normalizes_once -- --exact`; require exactly one and save findings before overwrite. Independently explicit painted/minted SOURCE/address constants, modifier namespace, empty chain, ordered reversals; assert unchanged source/all address fields, not mapper-twice/copied resolver.
- I3: `run_checked cargo test -p slicer-runtime --lib support_analysis_preserves_source_identity_and_exact_normalized_config_address`; require nonzero and save findings before overwrite. Inline actual commit_support_analysis_builtin with sibling SlicedRegions/independent family/contact config, normalized address versus retained minted/modifier SOURCE, no-contact assignment/territory and distinguishable payloads; not fabricated analysis-only oracle.
- I1: `run_exact slicer-ir support_geometry_ir_shape_tdd support_geometry_key_ordered_chain_selects_distinct_entries`; `run_exact slicer-ir support_geometry_ir_shape_tdd support_source_family_assignment_separates_ordered_variants`; `run_exact slicer-ir support_geometry_ir_shape_tdd support_key_json_legacy_missing_chain_normalizes_to_explicit_empty`; `run_exact slicer-ir support_geometry_ir_shape_tdd support_owner_postcard_populated_maps_preserve_full_identity`; `run_exact slicer-ir support_geometry_ir_shape_tdd support_composite_map_json_rejects_populated_keys`. Retain original sentinel/current-constant/additive omission/default/version checks.
- W11: `run_exact slicer-wasm-host contract source_identity_repair_tdd::dragon_curve_migrated_component_typed_load_and_run`; retain versioned_query_resources_load_and_old_components_reject and AC-R10 actual specialist calls. Example output alone does not prove every specialist was queried.

### Typed-fallout batch controls

TF00–TF14 below allocate every missing enumerated constructor/lookup surface; no unrelated lexical backlog. Every precondition includes accepted cold preflight, explicit activation, agreed schemas and approved coupled I/W boundary. Read named symbols through located ±40-line windows (≤600); feature gates and saved nonzero logs checked at execution. Migration reviews, all listed filters and actual codec/version/resource/example/OLD gates close at W14. Signature-only scaffolding may precede compile close, never behavioral red. Context S/M means independent bounded worker handoffs, not aggregate-context waiver.

### Step TF00 — Manual export and duplicate-plan propagation
- Task IDs: TASK-572 / 01; AC-R8/duplicate contract.
- Objective: export source key and propagate full duplicate identity.
- Precondition: I1 duplicate return agreed; actual manual export/two callers inspected; shared controls.
- Postcondition: root key usable; error carries exact chain, not family as duplicate identity.
- Files allowed to read, with ranges when over 300 lines: I1 duplicate_region_identity and listed symbol windows ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/lib.rs`; `crates/slicer-ir/src/stage_io.rs`; `crates/slicer-runtime/src/blackboard.rs`.
- Files explicitly out of bounds: speculative core reexport/index/new loader/error policy.
- Blast-radius discipline: manual slice_ir reexport; DuplicateSupportPlanEntry/fmt include Vec<(String, PaintValue)>; both commit_support_plan destructure/error constructors; V3/TF05 pattern fallout.
- Expected sub-agent dispatches: independent worker; escaped FACT/LOCATIONS outside thinking, redundant symbols/paths.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: V3 slot/TF05 prepass filters reject same-full-origin duplicates, accept differing-chain nonduplicates, assert diagnostic chain; W14.
- Exit condition: review complete, actual gates required at W14.

### Step TF01 — Comparator negatives and wedge fixture
- Task IDs: TASK-572 / 01; AC-R8/R9/parity.
- Objective: falsify chain-only divergence using real comparators.
- Precondition: V2 literal scaffold reviewed; agreed fields/shared controls.
- Postcondition: support/plan/infill/raft wrong/reversed-chain negatives caught, geometry preserved.
- Files allowed to read, with ranges when over 300 lines: compare_support_ir/compare_support_plan_ir/compare_support_plan_entry/compare_infill_ir/selftests ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/common/parity_invariants.rs`; `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`; `crates/slicer-runtime/tests/common/support_wedge.rs`.
- Files explicitly out of bounds: geometry tolerances/baseline weakening/V2 version-surface replay.
- Blast-radius discipline: chain comparator keys/row multiplicity; explicit raft source/geometry comparison; independent chain-only negatives call real comparator; single_region_support_plan IR no Default.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant caught divergence/count.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-runtime contract parity_invariants_selftest_tdd::; retained integrated parity; W14.
- Exit condition: review and nonzero actual comparator negatives pass at close.

### Step TF02 — Geometry-only visualization ordering
- Task IDs: TASK-572 / 01; retained visual/source order.
- Objective: deterministic full-key ordering without DTO migration.
- Precondition: agreed keys/plan fields/shared controls.
- Postcondition: siblings/reversals order deterministically; geometry/schema unchanged.
- Files allowed to read, with ranges when over 300 lines: listed sort/projection symbols/V3/TF03 expectations ±40 lines.
- Files allowed to edit (at most 3): `crates/pnp-cli/src/support_preview.rs`; `crates/slicer-runtime/src/visual_debug_render.rs`.
- Files explicitly out of bounds: preview fields/schema/slab/role/geometry algorithms.
- Blast-radius discipline: build_preview_doc/role_regions_by_layer/support_geometry_shapes/silhouette_bands compare full identity without reordering chain; sentinel/layer/negative-role retained. V3/TF03 actual projections assert independent polygon order across insertion permutations/chain-only differences.
- Expected sub-agent dispatches: independent worker; escaped FACT/SNIPPETS outside thinking, redundant sort/unchanged-schema receipt.
- Context cost: M.
- Authoritative docs: docs/19_visual_debug.md; docs/02_ir_schemas.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter pnp-cli support_preview_tdd ''; TF03 targets/unchanged visual matrix; W14.
- Exit condition: real ordering witness passes at close, DTO unchanged.

### Step TF03 — Visual fixture bodies/order witness
- Task IDs: TASK-572 / 01; retained visual contracts.
- Objective: migrate exhaustive IR visual fixtures/prove renderer order.
- Precondition: TF02 semantics/agreed no-Default classification/shared controls.
- Postcondition: coherent chains; degenerate/sentinel/warning regressions retained.
- Files allowed to read, with ranges when over 300 lines: seeded_support_geometry_and_plan/mixed_unit_shared_viewport/degenerate tests/plan_entry/raft warnings ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`; `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs`; `crates/slicer-runtime/tests/visual_debug_silhouette_tdd.rs`.
- Files explicitly out of bounds: capture API/schema/seam JSON/fixture skips.
- Blast-radius discipline: actual IR key/plan bodies, not signatures/imports; recognizable nonempty sibling polygons/plan chains and independent production-rendered order, not derived equality alone.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant targets/log/order.
- Context cost: M.
- Authoritative docs: docs/19_visual_debug.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-runtime visual_debug_blackboard_tap_tdd ''; run_filter slicer-runtime visual_debug_render_tap_tdd ''; run_filter slicer-runtime visual_debug_silhouette_tdd ''; original standalone visual commands/W14.
- Exit condition: nonzero fixtures/independent order pass at close.

### Step TF04 — Native/plan/lightning delivery fixtures
- Task IDs: TASK-572 / 01; AC-R8/R9/native parity.
- Objective: allocate remaining delivery fixture bodies.
- Precondition: SDK/IR imports resolved/shared controls.
- Postcondition: coherent chain/native actual commit observations retained.
- Files allowed to read, with ranges when over 300 lines: support_plan/lightning_tree_ir/native_prepass_commit_preserves_layer_support_and_mesh_outputs ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs`; `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs`; `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs`.
- Files explicitly out of bounds: algorithms/expected geometry/false no-Default SDK waiver.
- Blast-radius discipline: support_plan IR no Default; lightning IR; native commit SDK Default false waiver corrected with meaningful FRU/semantic pin, not S5 duplicate ownership.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant body types/oracles.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-runtime contract integrated_parity_support_surface_ironing_tdd::; run_filter slicer-runtime executor lightning_pipeline_linked_tdd::; run_filter slicer-runtime contract native_dispatch_parity_seam_tdd::; W14.
- Exit condition: reviewed fixtures/nonzero real delivery parity pass at close.

### Step TF05 — Prepass maps/plans/duplicate patterns
- Task IDs: TASK-572 / 01; AC-R8/duplicates.
- Objective: migrate explicit family initializers despite outer FRU.
- Precondition: TF00 duplicate shape agreed/shared controls.
- Postcondition: suppression/determinism/commit/duplicates discriminate exact identity.
- Files allowed to read, with ranges when over 300 lines: support_plan_entry/blackboard_with_layer_plan/run/duplicate controls ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/executor/prepass_executor_tdd.rs`; `crates/slicer-runtime/tests/executor/prepass_support_geometry_tdd.rs`; `crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs`.
- Files explicitly out of bounds: schedule rewrite/default empty authority/new input fields.
- Blast-radius discipline: IR plans no Default; explicit pair family maps migrate to SupportSourceKey despite outer Default-FRU; duplicate patterns/assertions and determinism include chain.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant keys/targets.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-runtime executor prepass_executor_tdd::; run_filter slicer-runtime executor prepass_support_geometry_tdd::; run_filter slicer-runtime executor support_config_surface_tdd::; W14.
- Exit condition: reviewed/nonzero exact duplicate/determinism controls pass at close.

### Step TF06 — Diagnostic/ordered identity/family closure
- Task IDs: TASK-572 / 01; AC-R8/ordered identity.
- Objective: migrate actual constructors without config/metric JSON edits.
- Precondition: Step 9 production behavior separately owned/shared controls.
- Postcondition: diagnostic/family source fields and actual PrintEntity chain observed.
- Files allowed to read, with ranges when over 300 lines: blackboard_with_layer_plan/structured_support_identity/owning_analysis/invalid_geometry_fails ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/integration/structured_support_identity.rs`; `crates/slicer-runtime/tests/integration/support_family_closure.rs`.
- Files explicitly out of bounds: old loader/decoder/golden config metrics/geometry weakening.
- Blast-radius discipline: diagnostic IR family initializer migrates; five invalid_geometry_fails IR plans keep protected negatives; independently expected nonempty ordered chain, not equality self-oracle.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant output-only/actual chain receipt.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-runtime integration prepass_diagnostic_roundtrip_tdd::; run_filter slicer-runtime integration structured_support_identity::; run_filter slicer-runtime integration support_family_closure::; W14.
- Exit condition: nonzero actual diagnostic/negative/output controls pass at close, no binary-compatibility claim.

### Step TF07 — Routing/eligibility/modifier territory
- Task IDs: TASK-572 / 01; AC-R8/eligibility.
- Objective: migrate family tuple receivers/coherent layered territory.
- Precondition: I3 semantics/agreed keys/shared controls.
- Postcondition: assignment versus suppression separate, exact source territory observed.
- Files allowed to read, with ranges when over 300 lines: family_assignments_for/entry/support_entry/cross_family_body_overlap/mismatched_family_fatal/needs_support_false_region_yields_no_auto_candidates/modifier_support_territory_clips_families_to_their_own_side ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/support_family_routing.rs`; `crates/slicer-runtime/tests/integration/support_eligibility_signal_tdd.rs`; `crates/slicer-runtime/tests/e2e/modifier_support_territory_e2e_tdd.rs`.
- Files explicitly out of bounds: physical obstacle/geometry/config-address conflation.
- Blast-radius discipline: support_entry exhaustive IR despite nested point FRU; family gets/index/filter/destructures structural source keys; territory retains layered SupportGeometryKey/chain.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant sibling/eligibility expectations.
- Context cost: M.
- Authoritative docs: ADR-0059 amended identity; docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-runtime integration support_family_routing::; run_filter slicer-runtime integration support_eligibility_signal_tdd::; run_filter slicer-runtime e2e modifier_support_territory_e2e_tdd::; W14.
- Exit condition: nonzero exact routing/eligibility/territory controls pass at close.

### Step TF08 — Modifier family/WIT builder/lightning guest
- Task IDs: TASK-572 / 01; AC-R8/R9/R10.
- Objective: allocate remaining map/WIT/IR guest bodies.
- Precondition: generated WIT shape agreed/actual artifacts required/shared controls.
- Postcondition: exact family keys/valid widened WIT builder/real chain-aware lightning calls.
- Files allowed to read, with ranges when over 300 lines: two modifier-family tests/valid_support_entry/invalid IDs/lightning guest ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/e2e/modifier_support_type_family_e2e_tdd.rs`; `crates/slicer-wasm-host/tests/contract/prepass_output_builder_validation_tdd.rs`; `crates/slicer-wasm-host/tests/contract/lightning_infill_guest_calls_lightning_tree_segments_tdd.rs`.
- Files explicitly out of bounds: input fields/invalid-ID weakening/artifact skips.
- Blast-radius discipline: family tuple access migrates, geometry retains layer; valid_support_entry exhaustive WIT no assumed Default; guest literal IR carries chain before query/output.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant WIT/IR/nonzero calls.
- Context cost: M.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-runtime e2e modifier_support_type_family_e2e_tdd::; run_filter slicer-wasm-host contract prepass_output_builder_validation_tdd::; run_filter slicer-wasm-host contract lightning_infill_guest_calls_lightning_tree_segments_tdd::; W14.
- Exit condition: real nonzero builder/modifier/guest controls pass after freshness close.

### Step TF09 — Decline aliases/dispatch/structural plans
- Task IDs: TASK-572 / 01; AC-R8/decline.
- Objective: migrate three actual DeclineEntry values/remaining bodies.
- Precondition: IR/WIT no-Default versus SDK classification/shared controls.
- Postcondition: producing-family attribution/exact dispatch/structural semantics retained.
- Files allowed to read, with ranges when over 300 lines: support_decline_contract/outer and nested decline_is_attributed_to_producing_family_not_last_writer/support_layer_dispatch_joins_plan_identity/structural_entry ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/tests/contract/support_decline_contract.rs`; `crates/slicer-wasm-host/tests/contract/support_identity_layer_dispatch_tdd.rs`; `crates/slicer-wasm-host/tests/contract/support_plan_structural_contract.rs`.
- Files explicitly out of bounds: last-writer fallback/opaque ID changes/WIT-as-SDK default.
- Blast-radius discipline: three IR alias literals (outer contract/nested declining_plan/outer attribution), not alias import only; coherent chain/no-Default waiver/base. Explicit family initializer/two dispatch IR plans/structural WIT body migrate; independent producing-family negatives.
- Expected sub-agent dispatches: independent worker; escaped FACT/LOCATIONS outside thinking, redundant three-site/type receipt.
- Context cost: M.
- Authoritative docs: docs/03_wit_and_manifest.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-wasm-host contract support_decline_contract::; run_filter slicer-wasm-host contract support_identity_layer_dispatch_tdd::; run_filter slicer-wasm-host contract support_plan_structural_contract::; W14.
- Exit condition: actual nonzero decline/structural/dispatch negatives pass at close.

### Step TF10 — Validation/ownership/cross-family maps
- Task IDs: TASK-572 / 01; AC-R8/ownership.
- Objective: migrate explicit keys preserving physical guards.
- Precondition: I6 identity/physical-overlap policy/shared controls.
- Postcondition: sibling-source ownership differs, foreign physical obstacles remain.
- Files allowed to read, with ranges when over 300 lines: entry/family_assignments_for/analysis/entry_in/duplicate-union-cross-family cases ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`; `crates/slicer-wasm-host/tests/unit/support_cross_family_scope_tdd.rs`; `crates/slicer-wasm-host/tests/unit/support_plan_ownership_tdd.rs`.
- Files explicitly out of bounds: policy relaxation/source-based obstacle suppression/invented IR Default.
- Blast-radius discipline: explicit family initializers migrate despite enclosing FRU; geometry retains layer; entry_in genuine FRU from valid entry(body_id,body); empty/values context unchanged.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant full-source/physical negatives.
- Context cost: M.
- Authoritative docs: ADR-0059; docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-wasm-host contract support_plan_validation::; run_filter slicer-wasm-host unit support_cross_family_scope_tdd::; run_filter slicer-wasm-host unit support_plan_ownership_tdd::; W14.
- Exit condition: nonzero original guards/exact ownership pass at close.

### Step TF11 — Traditional renderer fixtures
- Task IDs: TASK-572 / 01; AC-R8.
- Objective: migrate actual no-Default IR render inputs.
- Precondition: facade IR return classification/shared controls.
- Postcondition: source chain matches query; annotation/role/fill controls intact.
- Files allowed to read, with ranges when over 300 lines: paint_view_with_annotations/l_shape_paint_view/paint_with_plan/fixture_with_config ±40 lines.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs`; `modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs`; `modules/core-modules/traditional-support/tests/traditional_family_tdd.rs`.
- Files explicitly out of bounds: algorithms/fill oracle/11A planner surface.
- Blast-radius discipline: IR no Default, valid waiver/base/nonempty geometry; stage/anchor unchanged.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant targets/chain.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter traditional-support enforcer_blocker_tdd ''; run_filter traditional-support support_fill_geometry_tdd ''; run_filter traditional-support traditional_family_tdd ''; R8/W14.
- Exit condition: nonzero real fixture controls pass at close, geometry intact.

### Step TF12 — Traditional anchor/partitioned lightning fixtures
- Task IDs: TASK-572 / 01; AC-R8/R9.
- Objective: allocate remaining renderer/IR lightning inputs.
- Precondition: S3 planner/Step 13 lightning ownership distinct/shared controls.
- Postcondition: plan-at-anchor/interface/partitioned lightning retain ancestry.
- Files allowed to read, with ranges when over 300 lines: paint_with_plan_at/paint_with_plan_at_anchor_z/paint_with_interface_plan/lightning_paint/run_and_link ±40 lines.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs`; `crates/slicer-runtime/tests/integration/infill_partitioned_input_tdd.rs`.
- Files explicitly out of bounds: geometry/anchor-global policy/duplicate Step 13 test edits.
- Blast-radius discipline: exhaustive IR plan no-Default waiver/base and LightningTreeEntry coherent chain; ordinary model paths context.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant fixture/anchor receipt.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter traditional-support traditional_support_tdd ''; run_filter slicer-runtime integration infill_partitioned_input_tdd::; W14.
- Exit condition: nonzero anchor/partition delivery controls pass at close.

### Step TF13 — Tree renderer/off-grid fixtures
- Task IDs: TASK-572 / 01; AC-R8.
- Objective: migrate exact IR tree render inputs/declared planes.
- Precondition: planner S1/S2/11B and renderer Step 12 distinct/shared controls.
- Postcondition: chain reaches query, original role/skeleton/anchor checks retained.
- Files allowed to read, with ranges when over 300 lines: paint_view_with_annotations/l_shape_paint_view/fixture/offgrid_plan_entry_renders_at_declared_anchor_z/paint_with_plan_at_z/paint_with_plan_at/paint_with_interface_plan/extra_wall_count_printed_from_skeleton ±40 lines.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs`; `modules/core-modules/tree-support/tests/tree_family_tdd.rs`; `modules/core-modules/tree-support/tests/tree_support_tdd.rs`.
- Files explicitly out of bounds: branch/skeleton algorithms/off-grid policy/SDK Default for IR.
- Blast-radius discipline: qualified exhaustive IR plans/coherent chain/protected waivers; actual verified offgrid symbol; nonempty geometry retained.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant function/type/targets.
- Context cost: M.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter tree-support enforcer_blocker_tdd ''; run_filter tree-support tree_family_tdd ''; run_filter tree-support tree_support_tdd ''; R8/W14.
- Exit condition: nonzero role/plane/skeleton controls pass at close.

### Step TF14 — Unit carrier plan fixture
- Task IDs: TASK-572 / 01; AC-R3/R8/unit eligibility.
- Objective: allocate remaining actual unit carrier entry body.
- Precondition: carrier definition/equality controls agreed/shared controls.
- Postcondition: full-source fixture preserves absence/global-layer/anchor ancestry.
- Files allowed to read, with ranges when over 300 lines: entry/carrier_minted_for_planned_region_absent_from_slice/neighbors ±40 lines.
- Files allowed to edit (at most 3): `crates/slicer-wasm-host/tests/unit/support_carrier_regions_tdd.rs`.
- Files explicitly out of bounds: S4 dispatch surface/production transport/model grants.
- Blast-radius discipline: return signature context, body exhaustive IR no Default; chain with protected waiver/base; exact absence/eligibility retained; I3/Step 12 own behavior.
- Expected sub-agent dispatches: independent worker; escaped FACT outside thinking, redundant path/type/filter.
- Context cost: S.
- Authoritative docs: docs/02_ir_schemas.md; docs/21_data_defaults_and_fixtures.md; docs/22_test_quality.md.
- OrcaSlicer refs: none.
- Verification: run_filter slicer-wasm-host unit support_carrier_regions_tdd::; AC-R3/R8/W14.
- Exit condition: nonzero actual eligibility/full-source controls pass at close; no intermediate green.

### Step W14 — Close finite WIT/IR integration boundary

- Task IDs: `TASK-572`, row 01; AC-R10 and all schema/query witness dependencies.
- Objective: prove the entire coupled migration compiles all targets and real wasm-only consumers, owns all schema fallout and passes actual typed resource compatibility.
- Precondition: complete finite I/W roster including TF00–TF14, E1/EG01/02/05/06/07/08/09/13/EB01, Step 8 scaffolding and required Steps 11A/11B/12/13 signatures reviewed; I1 codec/key tests and W11 example witness registered; historical-version/governance/cold-review blocks resolved at activation (2026-10-08). No missing typed/query/output site.
- Postcondition: all-target workspace compile, owning old-value/serde tests, all rebuilt ordinary guests, example rebuild/snapshot, verifier, typed positive and old preinvoke rejection pass.
- Files allowed to read, with ranges when over 300 lines: allocated inventories and named migration symbols, ±40-line windows; bounded saved check/test/build logs; artifact WIT summary FACT.
- Files allowed to edit (at most 3): none; any failure requiring code changes returns to a named ≤3-path step and rechecks ownership, never an unbounded repair sweep.
- Files explicitly out of bounds: OLD fixture rebuild, guessed bindings, weakening version/assertion/manifest bounds, deleting failures.
- Blast-radius discipline: prove every changed constant's owning assertion binary and constructor closure within the integration boundary; this is not a deferred acceptance-only gate and not retrospective compile-red evidence.
- Expected sub-agent dispatches: question “does the completed finite migration pass all-target/wasm generation, owning schema fallout and actual typed compatibility?”; scope exact boundary commands per worker; return FACT outside thinking, escaped and redundantly naming exit/count/log.
- Context cost: M, with independent bounded command/evidence handoffs.
- Authoritative docs: `docs/02_ir_schemas.md` versions; `docs/03_wit_and_manifest.md` canonical nominal identity; ADR-0044/0045; docs/21/22.
- OrcaSlicer refs: none.
- Verification: owned all-target check and `cargo build --tests` after WIT changes; owning IR/schema assertions from cleared roster; every relevant macro/verifier/CLI snapshot filter; `cargo xtask build-guests`, then `cargo xtask build-guests --check` exit 0; forced staged refresh if required; actual rebuilt non-specialist and specialist guests; actual example MoonBit rebuild/typed load; AC-R10 real same-nominal-core resource calls and frozen old rejection. Read every saved log with nonzero test checks before overwrite.
- Exit condition: all boundary gates pass with no blind cfg-skipped guest coverage, all version fallout tested here, not only at closure. Otherwise BLOCK and reauthor bounded failing batch.

W14 additionally runs every TF filter and added mapper/producer/I1 codec-key/W11 example command above after all-target compile and ordinary guest freshness, retaining all original commands and controls. It closes BOTH I/W sub-rosters with actual example build/load and nominal resource/OLD preinvoke proofs; no acceptance-only postponement, isolation claim/experiment, fixture weakening or unallocated fourth-path sweep.

### Step 14 — Retained fixture/caller audit and late regression matrix

- Task IDs: `TASK-572`, row 01; all original thirteen ACs and every retained matrix row.
- Objective: preserve prior-work invariants and regress every actual caller after repair; do not replay obsolete field/index creation instructions or claim old path counts are current coverage.
- Precondition: production repairs and finite integration gates complete; original tests/commands/positive-negative controls retained.
- Postcondition: direct dispatcher callers hold production-built indexed `Some`; executor fixtures provision through public `build_and_commit_layer_selection_index`; exact authority is explicitly seeded, never synthesized by helper.
- Files allowed to read, with ranges when over 300 lines: `requirements.md` legacy caller crosswalk and verification matrix; `design.md` fixture authority/navigation rows; `crates/slicer-runtime/tests/common/mod.rs::{layer_input,run_layer_and_commit,run_layer_and_commit_with_bundle}`; `crates/slicer-runtime/tests/common/integrated_parity_harness.rs::run_integrated_parity`; actual `LayerStageInput`/runner sites via LOCATIONS ±40-line windows.
- Files allowed to edit (at most 3): none in this audit/regression step. A genuinely remaining fixture correction is allocated by coordinator into exact ≤3-path repair steps with full fields, retaining assertions, before its edit; no wildcard “all legacy callers” permission.
- Files explicitly out of bounds: stale initial-creation work, skipped assertions, implicit map authority, synthetic grants/selection, changing standalone filters, unrelated parity baselines.
- Blast-radius discipline: legacy 2C–2X labels are retained crosswalk only. Re-derive literal/helper/executor sites and manifest-derived paint metadata; audits must distinguish scheduled dispatch from non-dispatching builder/accessor `None`, module-less stages and capture supported taps. New field fallout is owned I/W, not postponed here.
- Expected sub-agent dispatches: question “do all actual callers use the same production seam and do all retained commands execute required tests after repairs?”; scope one bounded caller family or matrix command; return LOCATIONS/FACT outside thinking, escaped and redundantly naming test/count/log.
- Context cost: M, decomposed caller-family/command handoffs.
- Authoritative docs: `requirements.md` fixture crosswalk/matrix/cross-step obligations; `docs/04_host_scheduler.md` late bind/complexity; docs/21/22.
- OrcaSlicer refs: none.
- Verification: dispatch every original AC command by ID: AC-1/2/3/4/N1/N2/N3/B1/B2/B3/B4/BN1/BN2; every added AC-R1–R11/H1 with corrected R9 ownership; every retained matrix command, including slice resource, painted-holder three tests, direct builder/eligibility/anchored/raft/coloring, runtime executor/integration/contract/unit coverage. Inspect and execute requirements.md §Verification matrix's "Inherited whole-unit execution wrapper" for the whole runtime unit binary: its gated cargo xtask test --summary command and saved-log assertions discharge the unchanged inherited row; never execute plain Cargo whole-unit or run it twice. Other broad guest-touching bins also use cargo xtask test --summary after freshness with saved-log/nonzero required execution. Retain exact standalone commands for `arachne_parity_round2::arachne_wall_sequence_survives_wasm_boundary`, `arachne_wall_sequence_e2e_tdd::live_arachne_layer_one_sandwich_reaches_path_optimizer_unchanged` and `visual_debug_postpass_tap_tdd::postpass_whole_print_tap_contracts`; each executes exactly one. Imported wall-sequence unit tests do not replace unit-bin coverage. Re-run this full retained narrow matrix after final repair wiring and at closure, not only before wiring.
- Exit condition: every original/added criterion and retained matrix/standalone command passes with executed counts, fresh actual guests and explicit current caller audit; no original assertion or command is removed/weakened.

### Step G1 — New decision plus explicit origin/support identity amendments

- Task IDs: `TASK-572`, row 01; AC-R8/R10/R11 and S4 governance gate.
- Objective: record the new full-source aggregate/ownership and versioned shared/specialist contract decision; amend actual load-bearing old clauses with links and consequences.
- Precondition: activated/preflight-accepted bounded allocation; immediately before write, S4 gate rederives and approves ONE exact available new decision path, not a reserved filename or blanket ADR glob. Actual ADR-0059 Ruling 2 aggregate/family and Ruling 3 admission clauses carried in design; ADR-0022 unversioned builder location inspected. Future path approval blocks only G1 execution until satisfied, not whole-draft authoring if preflight accepts this allocation.
- Postcondition: actual new decision is created BEFORE adding references to its path. It adopts the carried full-chain replacements for aggregate, family assignment and producer admission, distinguishes object-wide Exact-Z geometry from source-specific ownership and records typed shared-version clean break. ADR-0059 clauses explicitly superseded; ADR-0022 builder location factually amended with decision link, full-origin principle retained.
- Files allowed to read, with ranges when over 300 lines: `docs/adr/0059-support-families-and-anchored-entities.md` Decision/Rulings 2–3/duplicate-identity clauses; `docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md` Decision; affected aggregation/ownership/resource symbol windows ±40 lines; ADR allocation directory listing only, not full ledger read.
- Files allowed to edit (at most 3): ONE new decision file whose exact workspace path is resolved and approved immediately before write by S4 gate (execution-gated dynamic allocation, not `docs/adr/**` permission); `docs/adr/0059-support-families-and-anchored-entities.md`; `docs/adr/0022-explicit-per-region-origin-for-perimeter-output-builders.md`.
- Files explicitly out of bounds: any unapproved new ADR path, silent accepted-history rewrite, normative source-clipping geometry mandate, preparation lifecycle scope.
- Blast-radius discipline: doc-only three-path batch after gated allocation; version/key runtime edits remain their owning batches. Quote ADR-0059 Ruling 2 “keys merges on declared identity `(family_id, global_layer_index, object_id, region_id, anchor_z)`,” family_assignments[(object_id, region_id)], and Ruling 3 “admits exactly one entry per `(global_layer_index, object_id, region_id)`.” Adopt design's full-chain replacements with every old dimension retained. Preserve default-deny/support-family manifest claims/Warn 1206 and per-body cross-section MAX_BODY_EXTENT_UNITS = 1 << 22/fixtures; never source-clip physical geometry. Complete G1–G3 before Step 15.
- Expected sub-agent dispatches: question “does the at-use allocated decision explicitly amend the actual pair-key and unversioned-location clauses while retaining origin/object-wide geometry principles?”; scope exactly allocated new file plus two named ADRs; return FACT/SUMMARY outside thinking, escaped with redundant gate/allocation outcome.
- Context cost: M.
- Authoritative docs: ADR-0059 Rulings 2–3; ADR-0022 Decision; ADR-0044/0045 actual typed/per-stage contracts; docs/02/03.
- OrcaSlicer refs: none.
- Verification: independent semantic review checks all three carried clauses/replacement keys and retained family/anchor/layer/extent guards; decision links resolve to the actual approved decision created first. Confirm no fixed version/ledger reservation and approved consequences match production evidence. Dynamic single path must be named/approved immediately before write.
- Exit condition: S4 gate clears exact single-file allocation, decision and both factual amendments accepted; unresolved allocation blocks execution, never filled by invented filename or broad edit permit.

### Step G2 — Lightning and WIT governance factual clarifications

- Task IDs: `TASK-572`, row 01; AC-R9/R10.
- Objective: clarify historical implementation reports and scope of version language without inventing a normative prohibition on source chains/shared versions.
- Precondition: G1 accepted allocated decision path available for links; relevant ADR clauses inspected as decision versus historical report.
- Postcondition: ADR-0029 per-region implementation report/query-location facts link to new full-source ordinary contract without moving generator into per-layer stage; ADR-0044 unversioned manifest wit-world annotation language is distinct from typed versioned shared-package identity; ADR-0045 per-stage isolation remains in force.
- Files allowed to read, with ranges when over 300 lines: ADR-0029 Decision and implementation Amendment; ADR-0044 Decision/What actually enforces compatibility; ADR-0045 Decision/per-stage package clauses, bounded symbol/heading windows.
- Files allowed to edit (at most 3): `docs/adr/0029-lightning-prepass-tree-generator.md`; `docs/adr/0044-wit-world-version-is-not-an-identity-token.md`; `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md`.
- Files explicitly out of bounds: claiming ADR-0029 forbids chains, claiming ADR-0044 forbids versioned shared resources, undoing per-stage separation, changing lightning algorithm/producer placement.
- Blast-radius discipline: factual clarification/amendment links only; do not turn historical numeric/version observations into new destination reservations. Preserve accepted decision/history and remove touched legacy line-only citations in favor of named symbols.
- Expected sub-agent dispatches: question “do clarifications correctly separate implementation reports, manifest annotations and typed package versions while retaining per-stage isolation?”; scope three ADR clauses and G1 link; return FACT outside thinking, escaped with redundant decision/report distinction.
- Context cost: M.
- Authoritative docs: ADR-0029/0044/0045 named clauses; G1 accepted decision; docs/03 canonical typed contract.
- OrcaSlicer refs: none; no upstream behavior claim altered.
- Verification: independent semantic review verifies decision links and scoped language, with actual typed-load evidence referenced; package layout alone does not prove future specialist-version isolation.
- Exit condition: all three factual clarifications accepted without overclaiming normative restrictions or experimental isolation.

### Step G3 — At-use allocated deviation and evidence reconciliation

- Task IDs: `TASK-572`, row 01; governance/closure evidence.
- Objective: allocate a real deviation at point of use for approved changed ADR-0059 identity contract and reconcile its final disposition with accepted decision/evidence.
- Precondition: G1 decision filename accepted; exact deviation identifier rederived immediately before write; no frozen DEV number or preallocated ledger slot.
- Postcondition: ledger row records original pair-key clause, approved full-source decision link, affected production/AC witnesses and honest active/resolved disposition derived from current evidence.
- Files allowed to read, with ranges when over 300 lines: `docs/DEVIATION_LOG.md` highest-ID query and relevant row only via worker; accepted decision/ADR-0059 amended clause and R8/R10/R11 evidence summaries, bounded windows.
- Files allowed to edit (at most 3): `docs/DEVIATION_LOG.md` only the at-use allocated relevant row.
- Files explicitly out of bounds: unrelated deviations, frozen numeric IDs, TASK-572 other rows/packets, declaring closure before accepted evidence.
- Blast-radius discipline: ledger-only batch separate from Step 15 and G1/G2; derive identifier at write using anchored DEV-row scan, reconcile concurrent allocation if needed and never overwrite another row.
- Expected sub-agent dispatches: question “what is the actual next deviation identifier at write and what evidence supports this row's current disposition?”; scope relevant ledger rows plus accepted decision/AC evidence; return LOCATIONS/FACT outside thinking, escaped and redundantly naming at-use allocation/disposition.
- Context cost: S.
- Authoritative docs: ADR-0059 amended by G1; `docs/DEVIATION_LOG.md` ledger conventions; global ledger rederivation rules.
- OrcaSlicer refs: none.
- Verification: independent review confirms identifier uniqueness at use, exact decision link, protected original clause and nonfabricated evidence/status; later final-review/bookkeeping gate reconciles disposition, not presumed resolved at authoring.
- Exit condition: at-use allocation and disposition accepted with no duplicate ID or premature closure; no new decision/deviation file is created by this authoring slice.

### Step 15 — Normative schema/WIT/scheduler contracts and ADR governance

- Task IDs: `TASK-572`, row 01; AC-R11/R10 and original doc-impact checks.
- Objective: repair normative docs to match the actual expanded source-chain/WIT contract, explicitly superseding only the indexed Packet 51 sourcing rule.
- Precondition: implementation connection evidence available; G1 new decision/ADR-0059 identity amendment and G2 factual clarification accepted, with G3 at-use allocated deviation/evidence reconciliation. S4 governance allocation gate resolved without freezing ledger IDs.
- Postcondition: affected IR versions/keys/chains and specialist stage/resource responsibilities documented; scheduler describes all held roles, host-only extruder/tool authority, complete carrier exclusions, ordered source outputs versus temporary config addresses and current geometry.
- Files allowed to read, with ranges when over 300 lines: `docs/02_ir_schemas.md` affected IR/version/interner sections; `docs/03_wit_and_manifest.md` WIT organization/query/version/access sections; `docs/04_host_scheduler.md` Complexity/Packet 51/Ordinary full-identity delivery sections, bounded delegated summaries; ADR Decision windows.
- Files allowed to edit (at most 3): `docs/02_ir_schemas.md`; `docs/03_wit_and_manifest.md`; `docs/04_host_scheduler.md`.
- Files explicitly out of bounds: general Error Handling Policy, unrelated docs/history/ADR decisions, future preparation/row 08 contracts.
- Blast-radius discipline: doc-only step. Cite workspace-qualified symbols, not sole line pins; source storage/output applies at every stage including support/lightning/raft. Preserve Packet 51 section for nonindexed stages; explicit supersession states indexed build-time exact-key authority and invocation/non-region frozen config. Carrier exclusions include wrong family/global layer/decline/empty role geometry/full-origin presence even empty; no model/coloring authority for carriers.
- Expected sub-agent dispatches: question “do the three normative contracts match actual source/map boundary, roles/tool authority, global-versus-subsystem version gate and every exclusion/resource rule?”; scope bounded normative sections plus named production seams; return FACT outside thinking, escaped and redundantly naming semantic findings. Governance is explicitly allocated G1/G2/G3, not left as an optional future amendment question.
- Context cost: M.
- Authoritative docs: ADR-0022 explicit output origin; ADR-0044 actual artifact compatibility; ADR-0045 per-stage versioned contracts; ADR-0056 native/WASM dispatch; docs/21/22.
- OrcaSlicer refs: none.
- Verification: run every owned command and paired semantic review in `requirements.md` §"Section-specific documentation verification": each of the six InfillIR/SupportIR/SupportGeometryIR/SupportAnalysisIR/SupportPlanIR/LightningTreeIR sections, IR Versioning Contract, WIT File Organization plus NEW `### Shared core and specialist query ownership`, both per-stage layer/prepass signature/version sections, and scheduler Ordinary full-identity delivery authority section. Do not duplicate or substitute broad heading-only commands here; owner rows require their exact semantics as well as presence. State that manifest min/max bounds check global host SliceIR, not individual affected IR versions, so unchanged SliceIR requires no bound edits and does not imply per-subIR compatibility enforcement. G1/G2/G3 have already completed in separate exact-path batches, never fourth/fifth edits in Step 15.
- Exit condition: heading checks and independent semantic/governance review pass with accepted decision/amendment links and reconciled deviation; no historical ADR or policy contradiction left implicit.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 0 | M | Blocking typed ownership/authoring/preflight; separate bounded dispatches |
| 1 | M | Isolated retrospective baseline; no presumed red |
| 2 | M | Registered host witnesses/retained fixture |
| 3 | M | Existing source-selection semantics |
| 4 | M | Private assembly introduction |
| 5 | M | Both production adapters |
| 6 | M | Actual coloring-context connection |
| 7 | M | Real SDK config/invocation witness |
| I1 | M | Structural layered/source keys, major support owners, populated actual-owner codec and independent key JSON proofs |
| I2 | M | SDK origin vectors/accessors/move-only builders; stage parameter lists unchanged |
| I3 | M | Producer and carrier metadata |
| I4 | M | Parallel projection/harvest |
| I5 | M | Source geometry/lightning generator |
| I6 | M | Output/aggregation/territory |
| S1 | M | Typed tree fixtures A |
| S2 | M | Typed tree fixtures B |
| S3 | M | Typed traditional/raster fixtures |
| S4 | M | Host SDK support/lightning accessor calls |
| S5 | M | Runtime SDK lightning accessor calls |
| S6 | S | SDK diagnostic plan guest |
| V1 | M | Executor fixture version purposes |
| V2 | M | Authored/parity/ordering version helpers |
| V3 | M | Current geometry/infill constants; intentional fixed plan metadata preservation |
| V4 | M | Lightning/support versions and real dispatch diagnostics |
| V5 | M | xtask canonical/OLD miniature stage pins |
| Q1 | M | Exact-Z service/cache and actual production callers |
| Q2 | M | Actual query/source-projection test callers |
| 8 | M | Registered runtime production drivers |
| 9 | M | Runtime lock/commit joins |
| 10 | M | Conversion/linker/partition |
| 11A | M | Traditional planner source/query/grouping |
| 11B | M | Tree planner source/query/grouping |
| 12 | M | Both renderers/shared carrier transport |
| 13 | M | Ordinary lightning/raft runtime roundtrip |
| W1 | M | Coupled core/specialist package introduction |
| W2 | S | Common/consumer signatures |
| W3 | S | Slice/perimeter imports |
| W4 | S | Postprocess/path imports |
| W5 | S | Anchored/prepass imports |
| W6 | M | Chain prepass ABI/Exact-Z/stage metadata |
| W7 | M | Scoped host/macro/dispatch workers; no full macro reads |
| W8 | M | Verifier/watchers/raw guest |
| W9 | M | Native facade/projections |
| W10 | M | Hard version/scaffold assertions |
| W11 | M | Chosen OLD path; Step 1 verified capture required before migration |
| W12 | S | Example snapshot/core copy |
| W13 | S | Example common/specialist copy |
| E1 | M | Three authored Makefile/glue inputs; explicit specialists/scratch staging |
| EG01 | S | Three changed export ABI/manifest paths |
| EG02 | S | Three changed export/gen manifest/host-service paths |
| EG05 | S | Changed profiling ffi only |
| EG06 | S | Changed config ffi only |
| EG07 | S | Two changed core bindings; package manifest retained |
| EG08 | S | Three new unsuffixed lightning specialist outputs |
| EG09 | S | Three new unsuffixed support specialist outputs |
| EG13 | S | Two-path sequential authored glue/manifest overlay |
| EB01 | M | Actual scratch MoonBit/UTF-16 component build, one binary copy |
| TF00 | M | Manual root export/duplicate error/two Blackboard callers |
| TF01 | M | Real comparator chain-only negatives/wedge fixture; V2 distinct |
| TF02 | M | Full-key preview/render ordering, geometry-only schema |
| TF03 | M | Three visual fixture paths/actual renderer-order control |
| TF04 | M | Native SDK commit and remaining IR plan/lightning fixtures |
| TF05 | M | Prepass family initializers/no-Default plans/duplicate patterns |
| TF06 | M | Diagnostic/ordered identity/family closure fixtures |
| TF07 | M | Routing/eligibility/layered modifier territory |
| TF08 | M | Modifier family/WIT builder/actual lightning guest fixtures |
| TF09 | M | Three DeclineEntry values/IR dispatch/WIT structural bodies |
| TF10 | M | Validation/ownership/cross-family full-source keys/physical guards |
| TF11 | M | Traditional renderer IR fixtures/role-fill controls |
| TF12 | M | Traditional anchor/partitioned lightning fixtures |
| TF13 | M | Tree renderer IR/off-grid/skeleton fixtures |
| TF14 | S | Single unit carrier IR body/exact eligibility |
| W14 | M | Finite integration compile/guest/typed closing gate |
| 14 | M | Late retained caller/regression evidence handoffs |
| G1 | M | S4-gated at-use decision file plus ADR-0059/0022 amendments |
| G2 | M | ADR-0029/0044/0045 factual clarification |
| G3 | S | At-use allocated deviation row |
| 15 | M | Normative semantic/ADR review after G1–G3 |

Aggregate is **L**, not a one-context M plan. Bounded S/M roster uses independent contexts/persistent handoffs, no extended-context waiver. Typed/generated allocations and numeric IR purposes are grounded proposals; the miniature WIT purpose disposition is carried by V5/W8 above (no separate purpose tail); cross-contract reconciliation and preflight acceptance of the roster/decomposition completed at activation (2026-10-08). The future exact single-path S4 approval gates G1 write; G1–G3 precede Step 15. Re-derive governance IDs/paths at use, do not freeze them or silently add costs. R9 ownership and host count are already reconciled; context use unmeasured.

## Packet Completion Gate

- Every approved step/finite window and its exit is complete; all authoring BLOCKS closed. No deferred constructor, generated example, SDK observation, query or old-component compatibility obligation.
- All original thirteen ACs, expanded ACs and every retained matrix/standalone command PASS with saved evidence and nonzero required execution. Historical width witness truthfully distinguishes measured red from an unreproduced retrospective gap; no fabricated red or fixture/compile failure credited as behavior evidence. Resolve that gap with coordinator/reviewer, never silently waive it.
- Narrow gates all green first; then required `cargo build --workspace`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report` with touched findings fixed/waived, ordinary guest freshness exit 0, real typed positive/old rejection and example rebuild. Commands use requirements' saved-output wrappers and FACT exits.
- Confirm source-ID plus full ordered-chain through ordinary selection/transport/SDK/grants/runtime/linker/support/lightning/raft, explicit current-geometry delivery and unchanged general error/geometry/anchor policy. No user-scope reduction or loosening acceptance for convenience.
- Independent **FULL final** `/spec-review` (not delta-only or preflight-only) after the workspace ceremony below must accept all evidence and leave no closure-blocking findings. Any corrective edit reruns its narrow/freshness/all-target obligations and affected closure evidence before review acceptance.
- Only after accepted final review: coordinator allocates later bookkeeping into ≤3 exact-path steps, rederives `TASK-572` backlog/queue facts at use through a worker, updates row 01 only and reconciles reopened status/task-map transitions. `docs/07_implementation_status.md` is read/updated through worker dispatch, never full backlog load. Other TASK-572 rows, including row 08, remain open. This implementation-plan authoring slice edits neither ledger nor source plan.
- `packet.spec.md` becomes ready for `status: implemented` only after evidence/review/bookkeeping reconciliation. User-requested commit is then permitted with `git commit -m ...` after inspecting actual diff/status; no amend, rebase/history rewrite, discard of unrelated work or stash deletion. Do not commit unreviewed corrections or generated artifacts without their approved scope.

## Acceptance Ceremony

1. Re-dispatch **every** original/expanded pipe-suffixed AC and retained packet-level/matrix/standalone command. Original fixture migration/provisioning coverage is late regression evidence, not a stale creation checklist. Preserve the owners' strings and correct R9 by coordinated ownership reconciliation, not a hidden host-only substitute or removed aggregate check.
2. Confirm every narrower gate above is green; save compact FACT/count/log results before the next run overwrites the log. Do not infer coverage from an empty feature-gated binary.
3. Dispatch the user-required **`cargo xtask test --summary --workspace`** to an independent command subagent, never plain `cargo test --workspace`. This runs literals/guest preflight and tees `target/test-output.log`. Worker reads that saved log and returns compact FACT pass/fail, required executed counts and log path outside thinking, properly escaped and redundantly named. Zero executed owning binaries fail; legitimate empty unrelated targets require explicit explanation. Do not absorb the full output or poll.
4. Run independent FULL final review of the entire repaired packet/diff/evidence; record packet-local risks, real retrospective evidence gaps and known side effects. No SHIP/closure claim before that review accepts.
5. Reconcile TASK-572 row 01/status/bookkeeping through bounded authorized worker steps, keeping later rows open; satisfy required precommit gates, inspect unrelated work and create only the user-requested non-rewriting commit.
6. Confirm actual context remained within skill standard limits (reading 120k/handoff 150k), or a permitted logged swarm escalation (extended reading 240k/hard stop 300k); otherwise record the authoring lesson. Do not claim measured context use without evidence.

No acceptance command, historical red, compile success, fixture completeness, preflight PASS, activation or implementation closure is claimed by this draft.
