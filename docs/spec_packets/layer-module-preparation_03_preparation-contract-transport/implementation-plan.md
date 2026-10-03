# Implementation Plan: preparation-contract-transport

## Execution Rules

Draft only: no step executes before independent preflight, explicit activation and row 02's executable pilot gate. Every step maps to partial `TASK-574`. Work test-first against actual production boundaries, then implement and run the narrow falsifying command. Each edit list is complete and capped at three files; new files and registrations count. No hidden bulk edits or field additions. If production composition requires another file, add an explicit preparatory step/re-preflight rather than exceeding the cap.

All dispatch results must be outside thinking, properly escaped and redundant about executed/not executed, with FACT pass/fail or bounded SNIPPETS. Every test command below means combined `tee target/test-output.log`, pipefail and executed count as fully spelled out in requirements/ACs. Heavy command/doc verification is delegated. Narrow test targets intentionally remain narrow; build/check/clippy acceptance uses `--all-targets`.

## Steps

### Step 0: Declare the controlled fixture feature
- Task: TASK-574. Objective: establish the explicit host-only feature before later feature-aware compilation.
- Precondition: activation/producer executable gate satisfied; metadata confirms the feature is absent. Postcondition: preparation-test-fixtures exists without changing default production behavior or adding a missing fixture dependency.
- Allowed reads: host Cargo metadata/manifest and requirements feature contract.
- Allowed edits (1): `crates/slicer-wasm-host/Cargo.toml`.
- Out of bounds: source code, guest dependencies before their crates exist, default-feature changes and all other packets/plan/backlog.
- Blast radius: no public struct/WIT/version or literal changes.
- Dispatch: question “Is the empty host-only feature declared with defaults unchanged?” Scope host metadata; FACT <=5 lines. Cost S.
- Authority: requirements feature/target grounding and docs/22 feature discipline. Orca refs: none.
- Verification: summarized Cargo metadata and host all-target check in default/fixture-feature modes, no behavioral test claim.
- Exit: feature is missing, enabled by default, or changes production semantics fails.

### Step 1: Shared declaration/error contract and sidecar accessor
- Task: TASK-574. Objective: add exact declaration/transport shapes and expose qualified capability metadata without field churn.
- Precondition: executable producer gate passes; producer sidecar shape is verified against disk. Postcondition: seven transport variants and declaration getters compile; old schemas/constants unchanged.
- Allowed reads: row-02 public exports; schema StageSpec/SlicerModuleSchema/PreparationExportSchema and package test symbol windows; docs/03 typed identity sections.
- Allowed edits (2): `crates/slicer-schema/src/preparation.rs` (new), `crates/slicer-schema/src/lib.rs`.
- Out of bounds: existing schema fields/versions/STAGES and all other packet/backlog/plan files.
- Blast radius: accessor/new private-field types only; zero existing literals/version assertions changed, as inventoried in design. Inline new tests use constructors/waivers.
- Dispatch: exact question “Do the seven Rust errors map one-to-one to producer WIT and does the accessor leave existing fields unchanged?” Scope schema/new module/producer WIT; FACT <=5 lines. Cost S.
- Authority: docs/03 WIT organization/typed compatibility, ADR-0045/0066. Orca refs: none.
- Verification: schema package lookup test and all-target schema check; counted tests per matrix.
- Exit: wrong enum mapping, changed scheduled export/version assertion, or accessor returning a world name fails the step.

### Step 2: Optional preparation parser and companion storage
- Task: TASK-574. Objective: own strict parsing/storage separately from ordinary IR permissions.
- Precondition: shared declaration exists. Postcondition: missing table is None; valid declaration stored in companion; exact malformed-field variants fail.
- Allowed reads: ingest_manifest_text/LoadedModuleBuilder/LoadedModule::ir_reads windows in scheduler manifest; docs/03 manifest/access sections; docs/04 “Manifest ↔ Runtime Naming Map (Normative)” bounded section; new schema contract.
- Allowed edits (2): `crates/slicer-scheduler/src/preparation.rs` (new, inline tests), `crates/slicer-scheduler/src/lib.rs`.
- Out of bounds: LoadedModule fields, ordinary ingestion/selection orchestration, runtime/config declarations and backlog.
- Blast radius: new companion/private fields only; no existing struct literal changes.
- Dispatch: question “Does parser require interface/input_reads, default views, reject unknown keys/duplicates and retain exact names without granting ordinary reads?” Scope new module; FACT <=5 lines. Cost S.
- Authority: docs/03 Module Manifest Schema and Host-Boundary Access Enforcement; approved declaration section. Orca refs: none.
- Verification: scheduler library `preparation::tests::` prefix with nonzero-count guard; author exact `preparation::tests::naming_exception_is_scoped` for AC-9. It checks new input_reads/input_reads() and UnknownKey for input-reads, then calls existing pub(crate) ingest_manifest_text within this crate to verify legacy [ir-access].reads/LoadedModule::ir_reads() and rejection of renamed [ir_access], without altering the legacy parser.
- Exit: any invalid table succeeds, unknown preparation metadata is tolerated, ordinary ir_reads grants preparation access, or new spelling silently changes/aliases legacy ingestion.

### Step 3: SDK serialized facades and macro adaptation
- Task: TASK-574. Objective: normalize host callbacks/TLS/require while preserving exact opted-in authoring seam.
- Precondition: producer SDK/macro compiled examples passed. Postcondition: host facades use shared errors, require emits 57401, TLS restores on return/unwind, paired generated entry remains exact.
- Allowed reads: producer SDK preparation/native and macro generator symbol windows; docs/05 entry/state lifecycle; shared error contract.
- Allowed edits (3): `crates/slicer-sdk/src/preparation.rs`, `crates/slicer-sdk/tests/layer_module_tdd.rs`, `crates/slicer-macros/src/lib.rs`.
- Out of bounds: SDK ordinary traits/native request/response fields, macro ordinary worlds and existing schema literal fields.
- Blast radius: method/trait additions and private wrapper normalization, no existing public field/version additions. New facade construction sites are only these files in this step.
- Dispatch: question “Does actual generated glue use the production callback seam and required static method without changing ordinary from_config or signature?” Scope named symbols; SNIPPETS <=3 x 30 lines. Cost M.
- Authority: docs/05 entry/lifecycle, ADR-0056 and row-02 forward exports. Orca refs: none.
- Verification: SDK `--features test --test layer_module_tdd preparation_transport_`; macro all-target check; tests counted.
- Exit: default-success prepare, algorithm-specific native shortcut, changed plain ABI or leaked TLS binding fails.

### Step 4: Owner store, checked accounting and publication transaction
- Task: TASK-574. Objective: implement production immutable storage, poison, ready marker, hook and disposal.
- Precondition: shared errors/facade traits compile. Postcondition: exact owner/store APIs in design exist, checked math precedes mutation/allocation, incomplete output cannot publish.
- Allowed reads: ADR-0066 consequences; design store contract; existing host table/context symbol windows only.
- Allowed edits (3): `crates/slicer-wasm-host/src/preparation_store.rs` (new, inline tests), `crates/slicer-wasm-host/src/preparation.rs` (new public failure/binding type definitions and store re-exports), `crates/slicer-wasm-host/src/lib.rs`.
- Out of bounds: HostExecutionContext/LiveModuleBinding field additions, runtime release/scheduling and IR/codec code.
- Blast radius: all new types private-field; existing fields/constants/literals unchanged. Test-only counter setup is inside production module's cfg(test), not a replacement store.
- Dispatch: question “Can failed puts/validator/drop ever make current ready, and do overflow/disposal drop retained name/payload bytes without stale strong handles?” Scope new store; FACT <=5 lines. Cost M.
- Authority: approved private-plan section, docs/01 ownership, ADR-0066. Orca refs: none.
- Verification: exact library `preparation_store::tests::checked_accounting_never_wraps` counted; host all-target check in default and fixture-feature modes.
- Exit: partial mutation on overflow, checking offset plus an unclipped maximum request, caught writer fault followed publication, raw payload Debug, or disposal while an active guard exists fails. Reject offset beyond length first; compute checked length-minus-offset/min before bounded conversion/end calculations.

### Step 5: Frozen native pair and production invocation scopes
- Task: TASK-574. Objective: bind required preparation and ordinary entry from one native artifact, with real store callbacks/fresh facade.
- Precondition: store/facades and producer NativePreparedStageEntry exist. Postcondition: immutable native binding validates metadata, preserves ModuleError detail and supplies only paired ordinary stage entry.
- Allowed reads: native entry/error definitions and schema qualified export windows; new store/parser/facades; ADR-0056 Decision.
- Allowed edits (2): `crates/slicer-wasm-host/src/preparation.rs` (native constructor/invocation and explicit child-module path registration), `crates/slicer-wasm-host/src/preparation_native.rs` (new).
- Out of bounds: integrated registry, existing LiveModuleBinding, selection/dispatch orchestration, new native envelopes.
- Blast radius: additive private binding enum/types; no existing public field or literal changes.
- Dispatch: question “Is the native stage pointer taken only from the paired artifact and are fatal=false errors preserved but unpublished?” Scope new native adapter; FACT <=5 lines. Cost S.
- Authority: ADR-0056/0066, docs/05 native state lifecycle. Orca refs: none.
- Verification: host all-target check, SDK facade tests; future native behavioral driver authored in Step 9.
- Exit: independent preparation provider lookup, swallowed module error, retained module value or native nonserialized plan bypass fails.

### Step 6: Canonical composed WASM bindings and resource type preparation
- Task: TASK-574. Objective: provide typed prepared Layer bindings and concrete backing/wrapper type definitions using existing ordinary import authority.
- Precondition: producer canonical WIT/composed worlds/verifier executable gate passed; native facade exists. Postcondition: every schema-backed Layer composed binding and its concrete backing types compile and keep ordinary exports unchanged; executable WASM freezing/resource methods are completed in Step 7, not claimed here.
- Allowed reads: host.rs canonical perimeters/stage bindgen blocks, StageSpec/STAGES symbols, producer canonical WIT; docs/03 nested layout/type identity.
- Allowed edits (3): `crates/slicer-wasm-host/src/preparation_bindings.rs` (new), `crates/slicer-wasm-host/src/preparation_host.rs` (new concrete backing/wrapper types, no fake-success Host methods), `crates/slicer-wasm-host/src/preparation.rs`.
- Out of bounds: ordinary WIT files/worlds, xtask verifier changes owned by row 02, HostExecutionContext field widening and scheduler plan.
- Blast radius: new generated binding source modules only; no WIT version/signature or old assertion change. Generated output is never edited.
- Dispatch: question “Do composed bindings remap ordinary and preparation resources to exactly one defining identity without placeholders that return success?” Scope new bindings/backing types/canonical alias windows; SNIPPETS <=3 x 30 lines. Cost M.
- Authority: ADR-0045/0066 and docs/03 type identity/layout. Orca refs: none.
- Verification: host all-target check in fixture/default modes; ordinary schema package and macro binding-surface regression tests.
- Exit: missing stage mapping, duplicated resource identity, capability counted as stage or preparation companion Component fails.

### Step 7: Production imported resources and projected linker context
- Task: TASK-574. Objective: normalize pilot backings to real production Host implementations and fresh call-local read tables.
- Precondition: store and composed bindings compile. Postcondition: prepared wrapper shares ordinary.table; ordinary linker imports project to inner context; preparation-only imports project to wrapper without double registration.
- Allowed reads: HostExecutionContext/table and existing add_to_linker windows; bounded producer harness summary; new bindings/store; docs/03 access/isolation.
- Allowed edits (3): `crates/slicer-wasm-host/src/preparation_host.rs` (real imported Host methods), `crates/slicer-wasm-host/src/preparation_wasm.rs` (new frozen component/typed invocation implementation), `crates/slicer-wasm-host/src/preparation.rs`.
- Out of bounds: ordinary HostExecutionContext fields/trait signatures, test-only Host equivalents, final row-04 typed input definitions.
- Blast radius: new backing/wrapper types private-field; existing resources remain unchanged.
- Dispatch: question “Does each resource use implicit owner and production store checks, and does read-fixture deny production mode and undeclared reads at Host boundary?” Scope production resource methods; FACT <=5 lines. Cost M.
- Authority: docs/03 enforcement/concurrency, approved imported-resource/private-plan sections. Orca refs: none.
- Verification: fixture/default host all-target checks; guest freshness exit-code gate after SDK/macro changes.
- Exit: table-index retention, standalone exported guest resources, ordinary ir_access grant, double-registered imports or read-fixture production bypass fails.

### Step 8: Small dual-target transport fixture
- Task: TASK-574. Objective: author same-source module-owned numeric-text codec and explicit negative modes.
- Precondition: real SDK/macro/backing seams available; no final input projection is needed for controlled fixture. Postcondition: actual PathOptimization module emits independent comment literals and named pieces through real APIs.
- Allowed reads: SDK run_path_optimization/ModuleError/GcodeOutputBuilder symbols; producer dual-target fixture pattern/discover_guests summary; docs/05 guest invariants.
- Allowed edits (3): new `crates/slicer-wasm-host/test-guests/preparation-transport-path/Cargo.toml`, `src/lib.rs`, `module.toml`.
- Out of bounds: producer fixtures/packet files, host codec implementations, geometry algorithms and production module config declarations.
- Blast radius: new fixture-local literals only; native request literals belong to driver, not this step. No coordinate conversion.
- Dispatch: question “Is this discovered dual-target same-source macro fixture using the exact PathOptimization trait and fixture-only modes specified in design?” Scope these three files/discovery authority; FACT <=5 lines. Cost S.
- Authority: docs/05 guest invariants/entry point and producer forward contracts. Orca refs: none.
- Verification: guest build/freshness with exit code inspected and fixture Cargo check using existing guest build path; no broad test run.
- Exit: private plan bytes interpreted in host, omitted empty workspace sentinel, absent ordinary export or trait-default preparation fails.

### Step 8a: Prepare fixture dependency and the empty test module
- Task: TASK-574. Objective: make fixture integration and later registration compile without hiding a fourth edit in Step 9.
- Precondition: Step 8's native/guest crate exists and compiles. Postcondition: fixture dev-dependency resolves and the new test module exists, with imports/scaffolding only and no claimed behavioral pass.
- Allowed reads: fixture Cargo/source, host Cargo, existing contract module conventions; docs/22 scaffolding rule.
- Allowed edits (2): `crates/slicer-wasm-host/Cargo.toml` (fixture dev-dependency), `crates/slicer-wasm-host/tests/contract/preparation_transport_tdd.rs` (new scaffold).
- Out of bounds: driver registration before its file exists, transport mock implementation, dummy/decorative tests and other packet/plan files.
- Blast radius: new dependency and new test file only; no existing public field/version mutation.
- Dispatch: question “Does the dev-dependency resolve to the dual-target fixture, and is the scaffold free of dummy success tests?” Scope these two files; FACT <=5 lines. Cost S.
- Authority: docs/22 §2.7/§4, producer dual-target precedent. Orca refs: none.
- Verification: host all-target check with the fixture feature; no test execution or behavioral claim at this preparatory step.
- Exit: dependency points to a missing crate, feature hides compilation, or scaffolding asserts its own success fails.

### Step 9: Real contract driver and counted registration
- Task: TASK-574. Objective: construct actual native/WASM/store/resource calls and register all future exact tests without test-equivalent transport.
- Precondition: production adapters and fixture build fresh. Postcondition: driver uses frozen APIs/production Host resources; contract aggregator compiles the two new modules under explicit fixture feature.
- Allowed reads: production_guest_smoke driver, contract aggregator, host builder/output/resource insertion symbol windows; producer old-artifact provenance summary; Cargo metadata.
- Allowed edits (2): `crates/slicer-wasm-host/tests/contract/preparation_transport_driver.rs` (new), `crates/slicer-wasm-host/tests/contract/main.rs`.
- Out of bounds: new test-only staging stores/Host impls/native facades, runtime run_slice/scheduling, other aggregators.
- Blast radius: new driver uses existing native request fields unchanged; constructors/fixtures or exhaustive waivers pin required fields. No literal churn elsewhere.
- Dispatch: question “Do new modules register in contract and do all calls hit production typed resources/paired native adapter rather than pilot-local storage?” Scope driver/aggregator; FACT <=5 lines. Cost M.
- Authority: docs/22 test derivation, docs/21 literal discipline; actual production smoke driver. Orca refs: none.
- Verification: host all-target check with preparation-test-fixtures; driver first fresh native/WASM test authored in Step 11 must execute, never silently skip missing artifacts.
- Exit: unresolved feature/registration, fake successful transport, or missing-fixture return silently fails.

### Step 10: Declaration/export/ordinary compatibility negatives and docs
- Task: TASK-574. Objective: falsify parser/freeze rejection and preserve the actual old plain artifact.
- Precondition: registered driver exists; producer archive/provenance is available. Postcondition: AC-1/AC-N1/AC-N2/AC-8 tests exist and reject wrong metadata/identities before calls.
- Allowed reads: parser/freeze APIs, producer archive summary, schema stage table, named docs/03 sections; production smoke driver.
- Allowed edits (3): `crates/slicer-wasm-host/tests/contract/preparation_transport_tdd.rs`, `crates/slicer-macros/tests/slicer_module_tdd.rs`, `docs/03_wit_and_manifest.md`.
- Out of bounds: replacing old archive with rebuilt guest, plain worlds/version assertions, selection/override integration or other packet docs.
- Blast radius: new tests only; old macro schema/ordinary version assertions remain intact, not weakened.
- Dispatch: question “Are missing/undeclared/incompatible and non-Layer preparation rejected on real native/WASM freeze, with zero prep invocations and old-artifact load?” Scope exact tests/driver; FACT <=5 lines. Cost M.
- Authority: docs/03 typed compatibility and ADR-0045/0056; docs/22. Orca refs: none.
- Verification: AC-1, AC-N1, AC-N2, AC-8 complete commands; macro preparation_transport_ tests and binding_surface_tdd regression target.
- Exit: unknown prep metadata tolerated, positive-only rejection test, empty stage population or fabricated old-artifact compatibility fails.

### Step 11: Fresh transport, exact names, checked bounds and decoder behavior
- Task: TASK-574. Objective: prove actual resource/facade serialized semantics, optional/required distinction and additive SDK contract.
- Precondition: production resources/fixture registered and fresh. Postcondition: AC-2/AC-3/AC-4/AC-5 and AC-N5 pass with independently pinned data.
- Allowed reads: production store/resources/facades, fixture codec, SDK native scope, docs/05 lifecycle/entry; docs/21/22 sections.
- Allowed edits (3): `crates/slicer-wasm-host/tests/contract/preparation_transport_tdd.rs`, `crates/slicer-sdk/tests/layer_module_tdd.rs`, `docs/05_module_sdk.md`.
- Out of bounds: host payload decoder, ordinary trait parameters/native envelope fields, producer fixtures and any weakening of assertions.
- Blast radius: new tests/SDK helper method assertions only; existing fields/constants unchanged. Overflow test was authored inside store in Step 4.
- Dispatch: question “Do clipped exact/max/zero/end/beyond reads and fatal required decode use actual adapter resources, with explicit literals rather than decoder-derived expectations?” Scope exact tests; FACT <=5 lines. Cost M.
- Authority: approved private-plan section and docs/22 oracle/nonvacuity rules; docs/05 state lifecycle. Orca refs: none.
- Verification: AC-2/AC-3/AC-4/AC-5/AC-N5 commands; SDK feature-test prefix; SDK doc section and lifecycle phrase greps.
- Exit: native algorithm shortcut, cursor interference, wrap, decoder-derived oracle or changed ordinary construction fails. On fresh/nonoverflowing counters, read(1,u64::MAX) must yield [5,11,17], read(u64::MAX,1) must be RangeError, and end/zero reads remain valid. Keep separate real counter-overflow tests returning ArithmeticOverflow; never use an oversized valid request as their rejection input.

### Step 12: Failure poison and geometry-free atomic extension gate
- Task: TASK-574. Objective: enforce no partial publication on all failure paths and lock row-06 extension seam.
- Precondition: real fixture negative modes/resources work. Postcondition: AC-N3/AC-N4 tests cover both backends and zero retained bytes/ready false on failure.
- Allowed reads: transaction/finalization/resource/native trap handling, fixture negative-mode definitions; ADR-0066 consequences; docs/01 ownership.
- Allowed edits (2): `crates/slicer-wasm-host/tests/contract/preparation_transport_tdd.rs`, `docs/01_system_architecture.md`.
- Out of bounds: geometric projections, row-06 IR/request schema, runtime cancellation orchestration, changing original ModuleError detail/fatal.
- Blast radius: no existing field/version changes; new error/validator tests only.
- Dispatch: question “Can catching any writer error, returning nonfatal ModuleError, trap/drop or rejecting validator leak pieces/ready?” Scope exact negative tests and actual adapter/store; FACT <=5 lines. Cost S.
- Authority: ADR-0066 and approved atomic private-plan requirements; docs/22 negative witnesses. Orca refs: none.
- Verification: complete AC-N3/AC-N4 commands and ownership-doc section grep.
- Exit: any staged piece/ready visible after failure or validation-after-ready succeeds fails.

### Step 13: Owner/print/concurrent isolation, disposal and metadata
- Task: TASK-574. Objective: prove owner-wide cursorless reads, fresh handles/TLS restoration, safe disposal and payload-redacted measured counters.
- Precondition: published production stores and two actual backend drivers exist. Postcondition: AC-6/AC-7 tests pin independent ranges/counters and all disposal effects.
- Allowed reads: owner guards/metadata/read facades and exact tests/driver; docs/01 ownership and ADR-0066 accounting/lifetime.
- Allowed edits (1): `crates/slicer-wasm-host/tests/contract/preparation_transport_tdd.rs`.
- Out of bounds: release orchestration, runtime execution event schemas, quota/spill/cache or geometric diagnostic storage.
- Blast radius: existing types unchanged; new test literal fields use constructors/waivers.
- Dispatch: question “Do actual parallel fresh-call resources remain owner/print isolated and dispose after guards without strong retained payload handles or payload diagnostics?” Scope exact tests/store/driver; FACT <=5 lines. Cost M.
- Authority: approved private-plan/accounting section; docs/03 concurrency, docs/22 counterfactuals. Orca refs: none.
- Verification: complete AC-6/AC-7 commands; count nonempty executed populations before equality loops.
- Exit: wrong-owner read, stale strong byte retention, disposal during active call, leaked native scope or raw payload output fails.

### Step 13a: Scoped canonical preparation-key naming exception
- Task: TASK-574. Objective: explicitly reconcile new external snake_case metadata with the canonical legacy naming rule, without changing legacy manifest/config policy.
- Precondition: Step 2's semantic naming test exists and passes; external input_reads and Rust input_reads() are fixed public spellings. Postcondition: docs/04 normative map contains only the explicit new-table exception and mapping, with legacy rows/policy preserved.
- Allowed reads: docs/04 “Manifest ↔ Runtime Naming Map (Normative)” bounded section; schema declaration accessors; new parser's exact naming test and legacy ingest_manifest_text/ir_reads symbol windows.
- Allowed edits (1): `docs/04_host_scheduler.md`, only the new exception paragraph/subsection within that naming map.
- Out of bounds: legacy map rows, legacy manifest key/config policy rewrites, canonical ADR amendments, other doc sections, source plan/backlog and other packets.
- Blast radius: documentation-only amendment; no public field/WIT/version/literal edits.
- Dispatch: question “Does the explicit exception limit snake_case to new [preparation] metadata, map exact external input_reads to input_reads(), preserve legacy policy, and have actual parser witnesses?” Scope naming subsection and AC-9 test; FACT <=5 lines. Cost S.
- Authority: docs/04 existing normative naming map and this packet's explicitly approved narrow exception; docs/03 declaration contract. Orca refs: none.
- Verification: full AC-9 command, including counted semantic parser execution, exact subsection grep and exact scoped-policy phrase grep.
- Exit: implicit authority conflict, renamed legacy keys/rows, missing semantic witness, input-reads accepted as an alias, or broad snake_case policy fails.

### Step 14: Bounded acceptance and forward export reconciliation
- Task: TASK-574. Objective: complete row-03 evidence without claiming TASK-574/framework/later packet closure.
- Precondition: every earlier narrow exit passed; dependency provenance/freshness is valid. Postcondition: every AC and full matrix gate passes and downstream ledger matches actual public shapes.
- Allowed reads: this packet's five documents; bounded named test logs/metadata and public symbols; no full build output.
- Allowed edits: none required for code; evidence/status changes within this packet require explicit closure authorization and do not activate later work.
- Out of bounds: source plan/backlog/other packet edits, workspace test suite, speculative ADR/IR reservations, activation or commit without authorization.
- Blast radius: no field/version/literal change at acceptance; any required repair returns to its owning bounded step.
- Dispatch: question “Do all counted ACs, --all-targets build/check/clippy, literal/test-quality report and freshness gates pass, with no unimplemented forward contract claimed shipped?” Scope matrix/logs/public ledger; FACT pass/fail <=5 lines. Cost S.
- Authority: packet.spec/requirements, docs/21/22 and guest freshness contract. Orca refs: none.
- Verification: every matrix command and all doc-impact greps; no cargo test --workspace.
- Exit: any zero-test run, stale/infrastructure guest check, touched unwaived quality issue, missing doc section or drifted downstream shape blocks closure.

## Per-Step Budget Roll-Up

| Steps | Cost | Bound |
| --- | --- | --- |
| 0 | S | Feature exists before feature-aware compilation |
| 1–2 | S each | Additive schema/parser only |
| 3–4 | M each | Facade and store separate |
| 5 | S | Native pair only |
| 6–7 | M each | Bindings split from imported resources |
| 8 | S | Three-file fixture only |
| 8a | S | Resolve dependency and create scaffold before registration |
| 9–11 | M each | Registration split from criteria authoring |
| 12 | S | Atomic failure cases/docs |
| 13 | M | Concurrent owner/disposal witnesses |
| 13a | S | One-section explicit naming-map exception |
| 14 | S | Bounded delegated gates |

Aggregate M, no L step. Largest reading hazard is canonical generated type identity; delegate symbol snippets rather than full bindgen expansions.

## Packet Completion Gate and Acceptance Ceremony

Re-dispatch every AC and matrix gate after earlier narrow verification passes. Record only real execution evidence and measurement gaps. Reconcile row-04/05/06 public contracts against the implementation, not private helper names. TASK-574 remains partial until its other approved rows complete; no backlog/source-plan changes are owned here. Generating this draft does not authorize implementation, activation or commit.
