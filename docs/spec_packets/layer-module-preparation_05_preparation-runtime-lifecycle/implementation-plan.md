# Implementation Plan: preparation-runtime-lifecycle

## Execution Rules

**Draft only; generation does not authorize execution.** TASK-574 is shared by queue 03–05; this row cannot close it independently of the producer gates. No activation, acceptance commands, queue/backlog edits or commits occur during generation.

One step at a time, TDD then the specified bounded production change. Read bounds below mean locate the named symbol/heading with `rg`, then at most ±40 lines per question; long files are never loaded wholesale. Every step is TASK-574. Step authority and edit caps are independent; no step may silently expand its surface.

Narrow command convention: `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e preparation_runtime_lifecycle_tdd::<named_test> -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`. Substitute only the explicitly named test below. Full runnable AC commands belong to packet.spec. Before each guest-driving narrow run, delegate `cargo xtask build-guests --check` and inspect exit 0/1/3: stale -> rebuild then check; infrastructure error -> stop, never claim clean. Tests always tee combined output and require positive executed count. Inspect failure logs, do not rerun to recover truncation. Gates/check/clippy use `--all-targets`; targeted `--test`/`--lib` runs deliberately select the proven binary rather than widening to all tests.

## Steps

### Step 0: Author the actual prepared AnchoredEvents reader fixture

- Task IDs: TASK-574. Objective: a module-owned prepared read in the existing ordinary `Layer::AnchoredEvents` dispatch, not synthetic host-commit evidence.
- Precondition: #01–#04 implemented/export gates and executable #02 pilot; new anchored fixture directory absent. Postcondition: real dual native/component prepared module owns its codec and produces a nonempty anchored proposal from required `probe`.
- Allowed reads: SDK `src/traits.rs::LayerModule::run_anchored_events`, `src/layer_collection_builder.rs::set_anchored_event_collection` windows; IR `AnchoredEntity`/`OrderedEventCollection` definitions; #02 preparation macro and #04 typed permissions; existing guest Cargo/build discovery windows.
- Allowed edits (3): `crates/slicer-wasm-host/test-guests/preparation-lifecycle-anchored-guest/Cargo.toml`, `src/lib.rs`, `module.toml` only.
- Out of bounds: runtime/dependency edits, producer fixtures, WIT/SDK/core/IR types, converting synthetic host commits into module dispatch. Blast radius: new fixture only, no existing fields/versions; production literals exhaustive, test literals FRU/waiver; fixture keys snake_case and producer permission vocabulary unchanged.
- Dispatch: Is the ordinary method an actual module-owned required plan read whose changed token changes emitted geometry? Scope new fixture and exact SDK method signatures; FACT <=5 lines. Context M.
- Authority: docs/05 Module State Lifecycle; docs/04 actual Layer dispatch versus anchored host closure; ADR-0066/Q17 private opaque plan; docs/08 coordinates, docs/21 literals. Orca refs: none.
- Narrow verification: future delegated `cargo check --manifest-path crates/slicer-wasm-host/test-guests/preparation-lifecycle-anchored-guest/Cargo.toml --all-targets`, guest build/freshness exits; real-driver AC-10 follows after Step 5 wiring. Exit: macro generates paired native/WASM AnchoredEvents entries with `require("probe")` and real collection proposal; input-only observations/host synthetic entities cannot satisfy it.

### Step 1: Author the dual-target prepared fixture

- Task IDs: TASK-574. Objective: a real native/component PathOptimization fixture consuming final typed inputs, with independent literal `probe` codec and fault modes.
- Precondition: implemented #01–#04/export agreement, executable #02 pilot and Step 0 anchored fixture; new PathOptimization fixture absent. Postcondition: dual native/WASM fixture discovered; ordinary method still reconstructs from_config; runtime production types have a direct legal schema dependency, independent of dev dependencies.
- Allowed reads: #02/#03/#04 public exports; produced fixture Cargo manifests/entrypoint windows for actual names/families; Step 0 anchored manifest; SDK PathOptimization guest windows; `xtask/src/build_guests.rs::discover_guests`; runtime `[dependencies]`/`[dev-dependencies]`, schema Cargo dependency sections and scheduler/wasm-host/SDK schema edges only.
- Allowed edits (3): new guest `Cargo.toml`, new guest `src/lib.rs`, `crates/slicer-runtime/Cargo.toml`. Paths are fully rooted in design's code surface.
- Out of bounds: WIT, SDK/macro/core implementation, producer fixture edits, root Cargo feature changes/other packets; no handwritten generated bindings/lockfiles. Runtime Cargo explicitly moves `slicer-schema = { path = "../slicer-schema" }` from dev-only to `[dependencies]`, removing the dev duplicate; adds both owned lifecycle fixtures and required #04 libraries only as dev-dependencies. Same three-file edit cap.
- Blast radius: production dependency move covers production `SliceRuntimeExtensions`/all native/schema type imports, and test dependency graph; existing schema dev consumers retain access through the production entry. No feature/version/WIT/type widening; new fixture literals exhaustive and test watched literals FRU/waiver. Graph authority is runtime → schema plus existing scheduler/wasm-host/SDK → schema; schema has no dependencies/back edge.
- Dispatch: Does Cargo declare schema as production rather than dev-only, with no cycle, and do paired fixture entries compile? Scope enumerated manifests/new guest/producer exports; FACT <=5 lines. Context M.
- Authority: docs/03 Concurrency & Instance Isolation; docs/05 Module State Lifecycle; docs/21 §1, docs/22 §2. Orca refs: none.
- Narrow verification: future delegated production manifest/graph check from requirements plus `cargo check -p slicer-runtime --all-targets`; guest freshness/build/check FACT exits. Exit: exact schema entry is production with no cycle/dev-only reliance, both owned fixtures compile, native/component prepared exports and ordinary signatures match; merely compiling a plain module fails.

### Step 2: Wire real run_slice regression fixtures

- Task IDs: TASK-574. Objective: registered e2e driver for all ACs including AC-10 and metadata-only AC-8, staged external artifacts and independent observations; no Ready injection. Author literal selection/name/length/accounting expectations, actual anchored reader changed-token controls, and a separate synthetic-host-closure lifetime control here.
- Precondition: Step 1 real fixture artifacts available. Postcondition: registered tests drive the existing wedge slice plus native/external fixture registrations; activation test is intentionally red until lifecycle wiring.
- Allowed reads: `tests/e2e/run_slice_api_tdd.rs::run_slice_against_wedge_returns_nonempty_gcode`; e2e aggregator; `tests/integration/live_module_loading_tdd.rs` native-registration/override constructors; #02 schema-corpus input/provenance sections; fixture src method window.
- Allowed edits (3): new guest `module.toml`, new `tests/e2e/preparation_runtime_lifecycle_tdd.rs`, `tests/e2e/main.rs` under crates listed in design.
- Out of bounds: production lifecycle implementation, producer test backings, archived artifacts, all other tests/packets. Blast radius: new test registration/fixture manifest only; declared fixture keys snake_case and #04 input_reads exact; no shared config/schema changes.
- Dispatch: Does `mod preparation_runtime_lifecycle_tdd;` register e2e and existing features compile the fixture? Scope runtime Cargo/aggregator/new test; FACT <=5 lines. Context M.
- Authority: docs/22 §2 false-green rules; docs/01 Priority tiers; docs/04 Anchored invocation closure. Orca refs: none.
- Narrow verification: `activation_order_and_final_inputs` using command convention; expected TDD red must be missing lifecycle API/activation assertion, not missing guest, zero tests or unrelated load error. Exit: named test is listed/attempted in the real e2e target and independent expected markers are pinned; no claim of acceptance pass yet.

### Step 3: Add sidecar types and production-used forwarding seam

- Task IDs: TASK-574. Objective: exact public exports, observer and extension registration without widening existing public structs.
- Precondition: Steps 0–2 fixtures/red evidence, production schema dependency, locked packet.spec shapes. Postcondition: run_slice/collector/extensions share startup; actual real layer dispatcher is wrapped by NET-NEW PreparedLayerStageRunner in the existing runner slot; synthetic entities still feed only builtin host work.
- Allowed reads: run.rs `run_slice`, `run_slice_with_collector`, integrated registrations and pipeline construction; lib reexports; #03 frozen adapter exports; `IntegratedModuleRegistration` definition window.
- Allowed edits (3): new `src/preparation_lifecycle.rs`, `src/lib.rs`, `src/run.rs` in slicer-runtime.
- Out of bounds: SliceRunOptions/SliceOutcome fields, pipeline public fields, host loader envelopes, SDK/IR/WIT/schema constants. Blast radius: new types/functions have no pre-existing literals; unchanged public fields require no churn. Inventory any unexpected shape change before editing; stop/split instead of widening.
- Dispatch: Do run_slice and collector invoke the same forwarding prefix and do selected external modules suppress native pairs? Scope run/lifecycle plus host loader provenance branch; LOCATIONS <=20. Context M.
- Authority: docs/01 Priority tiers/Data Ownership; docs/05 Module State Lifecycle. Orca refs: none.
- Narrow verification: delegated all-targets check and `external_override_dual_leg` (still red only for missing preparation); exit: no production-dead facade, no global registry/Ready injection and no altered config precedence.

### Step 4: Freeze selected identities and shared eligibility

- Task IDs: TASK-574. Objective: selected binding companion, deterministic print owner set and final typed input projection.
- Precondition: Step 3 single startup seam and producer contracts executable. Postcondition: loaded provenance/version/artifact drives freeze; owner union uses shared targets across promoted layers/selected ordinary stages including Layer::AnchoredEvents, never synthetic entity commits.
- Allowed reads: run.rs selected live bindings; pipeline.rs `promote_global_layers`/Blackboard product windows; #01 projection and #04 from_committed exports; actual host `WasmRuntimeDispatcher::run_stage` selection projection window.
- Allowed edits (3): runtime `src/run.rs`, `src/preparation_lifecycle.rs`, `src/pipeline.rs`.
- Out of bounds: #01/#04 resolvers, loader structs, fill-holder catalogs, unfinished LayerArena and guessed future stage outputs. Blast radius: private companion only, no struct-field/constant/config/WIT additions.
- Dispatch: Does every target kind/paint/member/role/nonregion authority survive capture without duplicated selection? Scope companion and public projection callsites; SUMMARY <=200 words. Context M.
- Authority: docs/04 Stage Prerequisites/Anchored invocation closure; #01 AC-BN1 distinction; docs/01 Data Ownership. Orca refs: none.
- Narrow verification: `unused_nondefault_and_plain_empty` and `shared_selection_all_target_kinds`; exit: owner union independent of HashMap order, unused and prepared empty-target owners excluded, plain empty-target call preserved. Red consumer absence may remain until Step 5, but no wrong selection/identity is accepted.

### Step 5: Route fresh ordinary and anchored prepared consumers

- Task IDs: TASK-574. Objective: host prepared adapter method and runtime lifecycle-aware forwarding runner, preserving ordinary delivery.
- Precondition: frozen binding/owner set and shared selection captured. Postcondition: prepared native/WASM consumers execute through the selected binding's fresh adapter; plain modules delegate unchanged.
- Allowed reads: host dispatch.rs `LayerStageRunner for WasmRuntimeDispatcher`, `dispatch_layer_call` AnchoredEvents arm and native request windows; #03 fresh adapter contracts; runtime `execute_single_layer_inner` ordinary runner loop and `execute_anchored_event_collections_with_mode_and_feedrate` synthetic host body windows; #01 implemented factoring.
- Allowed edits (3): `crates/slicer-wasm-host/src/dispatch.rs`, runtime `src/layer_executor.rs`, `src/preparation_lifecycle.rs`.
- Out of bounds: public runner-trait fields/signatures, shared marshalling/selection semantics, ordinary error-policy fixes, SDK/WIT and producer packet edits. Blast radius: additive method/internal forwarding path only; no existing structs or schema constants widened.
- Dispatch: Does production-created PreparedLayerStageRunner forward real ordinary Layer::AnchoredEvents to run_prepared_stage while synthetic host commits remain dispatcher/read-free? Scope run.rs wrapper construction (read-only), lifecycle trait implementation, host/executor methods; LOCATIONS <=20. Context M.
- Authority: docs/04 Runner-Trait Input Borrow Structs/Anchored invocation closure; docs/03 Concurrency & Instance Isolation; docs/05 Module State Lifecycle. Orca refs: none.
- Narrow verification: `non_infill_fresh_concurrent_reads`, `external_override_dual_leg`, `prepared_anchored_module_reads_and_host_closure_lifetime`; freshness required. Exit: production routing and accessor delegation compile, actual selected Layer::AnchoredEvents reaches the new runner, synthetic host work remains dispatcher/read-free. Until Step 6 activation and Step 7 cleanup, these full-driver tests may remain red only for those missing lifecycle behaviors; do not fabricate Ready inputs or claim final consumer/lifetime acceptance. Rerun them after Steps 6–7 to establish real module reads/changed output and outer host-closure retention.

### Step 6: Activate late PrePass and all-Ready dependency guard

- Task IDs: TASK-574. Objective: production preparation ordering and truthful readiness; normalize every pipeline forwarder.
- Precondition: real selected adapters and final typed inputs available. Postcondition: all public pipeline variants and full slice use the same late closure; each eligible owner prepared once; no LayerArena/anchored consumer before all required Ready.
- Allowed reads: pipeline.rs all `run_pipeline*` entrypoint and core prefix windows; run.rs pipeline calls; companion owner/transaction boundaries; #03 finish/current and #04 constructor signatures.
- Allowed edits (3): runtime `src/pipeline.rs`, `src/preparation_lifecycle.rs`, `src/run.rs`.
- Out of bounds: STAGE_ORDER/enum expansion, prepass algorithms, between-stage barriers, general public struct changes. Blast radius: add PipelineError::Preparation and its only surveyed exhaustive fmt::Display in this same pipeline file; no existing test asserts a schema constant. Cancelled remains the current variant path.
- Dispatch: Does final paint/shell/support commit precede prepare and every public forwarding path reach the same core? Scope pipeline/run closure; LOCATIONS <=20. Context M.
- Authority: docs/04 PrePass Execution/Stage Prerequisites/Error Handling Policy and documented ordinary limitation; docs/01 Memory Model. Orca refs: none.
- Narrow verification: `activation_order_and_final_inputs`; embedded `cargo test -p slicer-runtime --lib preparation_lifecycle::tests::missing_ready_blocks_consumer -- --exact` with pipefail/tee/positive-count convention. Exit: both tests green through actual production closure; missing-ready callback has zero consumers and no fabricated Ready.

### Step 7: Preserve failures and cooperative cleanup outcome

- Task IDs: TASK-574. Objective: typed original errors, aborted transactions, cancellation checkpoints and join-before-dispose.
- Precondition: Step 6 activation/guard proof. Postcondition: required failures stop dependent work; normal/failure/cancel join actual prepare/read guards and finish/unwind applicable synthetic host closure before disposal, preserving errors. Busy remains a producer return value, not a waiting operation or a synthetic-host read guard.
- Allowed reads: lifecycle scope/guard/Ready code; pipeline error/phase joins; existing runtime cancellation checkpoint and Rayon return windows; #03 disposal Busy/error precedence.
- Allowed edits (3): runtime `src/preparation_lifecycle.rs`, `src/pipeline.rs`, e2e `preparation_runtime_lifecycle_tdd.rs`.
- Out of bounds: preemptive WASM interrupt, general nonfatal ordinary policy, CLI cancellation protocol, allocator accounting and producer store fields. Blast radius: no new public fields/WIT/schema; match fallout confined to already-owned PipelineError Display.
- Dispatch: Are active calls joined before dispose for every outcome and are fatal:false fields unchanged? Scope typed failure/join paths and new negative tests; SUMMARY <=200 words. Context M.
- Authority: docs/04 Cooperative Cancellation/Error Handling Policy/ordinary host limitation; docs/01 Memory Model. Orca refs: none.
- Narrow verification: `required_module_error_preserves_fields`, `trap_poison_and_failure_cleanup`, `cancellation_waits_for_inflight_cleanup`, and Step 5's three deferred full-driver tests. Exit: independent consumer counts zero for preparation failures; zero retention only after quiescence; cancellation not success; native/WASM actual anchored reads and changed-token outputs proven separately from synthetic-host lifetime retention.

### Step 8: Implement metadata-only preparation capture and production accounting

- Task IDs: TASK-574. Objective: #06's exact typed seam sharing validated production startup and prepare, stopping before consumers.
- Precondition: full production activation/failure cleanup proof. Postcondition: capture returns piece names/lengths, typed targets/lifecycle and actual accounting only; no opaque bytes/handles or host format decoding; disposes store before any arena/module/synthetic host closure.
- Allowed reads: run startup/validation/capture windows; pipeline late-prepass completion; lifecycle metadata/typed target paging; #04 DTOs; #03 PreparedPlanMetadata accessors; ADR-0066 diagnostics/Q17 windows. No piece reads for capture implementation.
- Allowed edits (3): runtime `src/run.rs`, `src/pipeline.rs`, `src/preparation_lifecycle.rs`.
- Out of bounds: prepare_prepass_context substitution, raw opaque capture/observer/CLI/DAG payloads, host plan decoder, #06 opt-in typed diagnostic snapshots and visual schemas, shared versions/config. Blast radius: exact new metadata types only; no existing results/WIT fields.
- Dispatch: Do capture/full prefix and Ready guards match, without open/read/copy of pieces, and do totals equal producer production counters rather than stored lengths? Scope metadata capture/independent fixture; FACT <=5 lines. Context M.
- Authority: docs/04 Stage Prerequisites/LayerCollectionIR lifecycle; docs/01 Data Ownership/Memory Model; docs/16 Global allocator contract. Orca refs: none.
- Narrow verification: `preparation_only_capture_shared_prefix`. Exit: independent `probe` name/length 4 and literal selected-target expectations; changed variant/provenance removes old target; changed put length 2 changes metadata/retained-payload gauge but not zero transfer. Zero arena/module/host-closure counts and post-cleanup retained zero despite nonempty prior storage; metadata usable after disposal. Ordinary outputs separately prove plan semantics.

### Step 9: Complete stage/target/decoder coverage from shared pilot inputs

- Task IDs: TASK-574. Objective: independent full-driver target-kind corpus and ready-empty/decoder cases, including non-infill adapters.
- Precondition: production lifecycle and capture green; #02 corpus input artifacts verified. Postcondition: model-paint/modifier/perimeter members/multiroles/support/raft/actual anchored deliveries cover schema-derived prepared/plain stages without a copied roster/oracle.
- Allowed reads: e2e test named fixture windows; #02 schema compile-driver corpus inputs and artifact provenance; `slicer_schema::STAGES`/stage_by_id; #01 target shapes/#04 typed DTO methods; fixture module-owned codec.
- Allowed edits (3): e2e `preparation_runtime_lifecycle_tdd.rs`, guest `src/lib.rs`, guest `module.toml`.
- Out of bounds: producer corpus implementation, target-key DTO shapes, core algorithms, generated bindgen/source and production config schema. Blast radius: fixture-only schema keys/input declarations stay contained; no existing literals or constants altered.
- Dispatch: Does every claimed driver assertion have a nonempty independent control and mutate the actual observed consumer outcome? Scope fixture/test only; SUMMARY <=200 words. Context M.
- Authority: docs/22 §2.1/2.3/2.4, docs/21 §1; #01 selection contract and #04 lossless paint keys. Orca refs: none.
- Narrow verification: `shared_selection_all_target_kinds`, `unused_nondefault_and_plain_empty`, `ready_empty_optional_required_decoder`, `non_infill_fresh_concurrent_reads`, `prepared_anchored_module_reads_and_host_closure_lifetime`. Exit: no vacuity/artifact skip/same resolver oracle; real selected anchored module owns decoding, synthetic entities never masquerade as its reads/calls.

### Step 10: Prove overlapping prints and in-flight cancellation

- Task IDs: TASK-574. Objective: explicit blocking/barrier fixtures proving print isolation, fresh reads and safe disposal without timing guesses.
- Precondition: single-print normal/failure/cancel semantics green. Postcondition: overlapping prints and active call cancellation have independent token/guard/lifetime observations.
- Allowed reads: lifecycle guard/join/cancel windows; guest fault modes; e2e explicit synchronization fixture windows; existing cancel_flag tests as compatibility context.
- Allowed edits (3): e2e `preparation_runtime_lifecycle_tdd.rs`, guest `src/lib.rs`, runtime `src/preparation_lifecycle.rs`.
- Out of bounds: changing serial pool limits, enabling shared memory, sleeps as timing assertions, report peak metrics, global test registry mutations. Blast radius: no existing public fields/versions/config changes; fixture-only modes remain declared snake_case.
- Dispatch: Do tests overlap active calls and prove the runtime caller joins before disposal, rather than running slices sequentially or making producer disposal wait? Scope test synchronization/record assertions; FACT <=5 lines. Context M.
- Authority: docs/03 Concurrency & Instance Isolation; docs/04 Cooperative Cancellation; docs/22 non-vacuity. Orca refs: none.
- Narrow verification: `concurrent_print_isolation_and_cleanup`, `cancellation_waits_for_inflight_cleanup`. Exit: barriers prove overlap and guarded Busy/dispose ordering; reads never cross tokens; no presumed latency/peak reduction.

### Step 11: Prove truthful observations and arena run-to-completion

- Task IDs: TASK-574. Objective: actual owner/module/version/phase records and arena lifetime traces on success/failure.
- Precondition: lifecycle and inflight isolation complete. Postcondition: static declarations/edges and actual prepare states remain distinct; instrument duration optionality and no between-stage arena retention falsified.
- Allowed reads: lifecycle event/counter windows; executor actual arena create/commit/drop and anchored call windows; existing instrumentation opt-in wiring; e2e trace assertions.
- Allowed edits (3): runtime `src/preparation_lifecycle.rs`, `src/layer_executor.rs`, e2e `preparation_runtime_lifecycle_tdd.rs`.
- Out of bounds: ProgressEvent/JSONL fields or version bumps, report structs/global allocator, persistent arenas or between-stage barriers. Blast radius: separate sidecar records only; zero existing field/version assertion changes.
- Dispatch: Are records based on actual executed calls, with durations only instrumented and name/byte counters not peaks? Scope lifecycle/executor/new tests; SUMMARY <=200 words. Context M.
- Authority: docs/17 DAG introspection/Live slice instrumentation; docs/09 Instrumented Stream; docs/16 Global allocator contract; docs/04 LayerCollectionIR lifecycle. Orca refs: none.
- Narrow verification: `truthful_lifecycle_metadata`, `arena_run_to_completion_on_failure`. Exit: independent call/arena trace order, no raw payload in records, no fake Start/Ready, no noninstrumented duration and no retained unfinished arena.

### Step 12: Document normative runtime and SDK lifetime

- Task IDs: TASK-574. Objective: exact scheduler/capture/ownership/SDK sections, preserving ordinary limitations and legacy lightning.
- Precondition: behavior ACs green and observer/capture exports implemented. Postcondition: exact doc-impact sections describe observed runtime, not merely future declarations.
- Allowed reads: docs/04 named normative sections and new behavior assertions; docs/01 Memory Model/Priority tiers; docs/05 Module State Lifecycle; packet.spec exact heading/export windows.
- Allowed edits (3): `docs/04_host_scheduler.md`, `docs/01_system_architecture.md`, `docs/05_module_sdk.md`.
- Out of bounds: other packets, queue/backlog, ADR amendments, legacy lightning behavior and visual schemas. Blast radius: documentation only; no WIT/config/struct/version edits.
- Dispatch: Do new sections match selection/readiness/cancel/cleanup and exact exported shapes while ordinary error limitation/ADR-0029 stay intact? Scope only named new sections; SUMMARY <=200 words. Context S.
- Authority: existing docs/04 cancellation/error/prerequisites; docs/05 reconstruction; docs/adr/0029 Decision. Orca refs: none.
- Narrow verification: rerun AC-1/2/3/5/8/10's complete commands including exact doc greps. Exit: all exact headings and behavioral tests pass; docs distinguish selected Layer::AnchoredEvents reads from synthetic host commits and state metadata-only capture/production accounting; generic prose alone cannot discharge behavior.

### Step 13: Document observations and perform narrow acceptance

- Task IDs: TASK-574. Objective: debugging guidance and independent packet acceptance, not workspace-wide testing by default.
- Precondition: every narrower step/criterion passed with fresh guests and positive test counts. Postcondition: exact observation section distinguishes static capability metadata from executed lifecycle; complete authoritative matrix reports FACT results.
- Allowed reads: docs/17 Live slice instrumentation/DAG introspection; all five packet docs; bounded test result/failure log windows only.
- Allowed edits (1): `docs/17_agent_debugging.md`. During execution, packet status/backlog closure is a separate explicitly authorized parent ceremony after complete review, not extra edits inside this step.
- Out of bounds: code changes to get a pass, queue/backlog/other packets, full workspace test invocation, activation/commit during generation. Blast radius: docs only; no contract or version changes.
- Dispatch: Run all AC commands/matrix individually, return FACT per command with executed count; independent full packet review only after implementation. Scope exact commands and doc-impact sections; no omitted tail. Context S.
- Authority: docs/17 static/live distinction; docs/21 literals/docs/22 quality; packet.spec and requirements command matrix. Orca refs: none.
- Narrow verification: AC-9 complete command plus all matrix commands. Exit: every required command passes, freshness exit zero, report-mode findings in touched code fixed/justified, no unverified AC or false-green zero-test result.

## Per-Step Budget Roll-Up

Steps 0–11: M, one named fixture/closure question and <=3 edits each; production dependency move is inside Step 1's already-inventoried Cargo file. Steps 12–13: S docs/acceptance dispatch. Aggregate M with sequential bounded worker contexts; no L step. Stop/split if bounds cannot be honored.

## Packet Completion Gate / Acceptance Ceremony

Future execution only: all atomic exits; every complete AC command; authoritative matrix; independent full spec review. No default full-suite gate is requested or authorized. Parent coordinates TASK-574 closure with rows 03/04 and status changes only after their actual gates; this draft never edits their files. Reconfirm unresolved forward dependencies, no unintended ordinary behavior change, exact #06 exports and unchanged legacy lightning before requesting implemented status. Generation self-review/preflight is not this ceremony.
