# Implementation Plan: whole-print-input-views

## Execution Rules

Draft only. Do not execute until implemented #01/#03 and their pilot gates, independent preflight and explicit activation approval. Every step maps TASK-574 and at most three file edits. Cargo/doc audits are delegated with FACT pass/fail outside thinking; failures include at most 20 relevant lines. No workspace suite. Test commands use pipefail/tee `target/test-output.log` and exact executed-test assertion, as in the independently runnable packet.spec ACs.

### Step 1: Activation inventories and schema/literal boundary

- Task IDs: TASK-574. Objective: reconcile draft exports with implemented producers before touching transport.
- Precondition: #01/#03 executable gates passed and explicit execution authorization; postcondition: accepted symbol/type/literal/WIT inventory and no interface drift.
- Allowed reads: producer packet.spec exports; design Read-Only Context named windows; implemented preparation public exports and macro prepared-glue windows located by grep.
- Allowed edits: none.
- Out of bounds: other packet edits, plan/backlog, generated code, target artifacts, Orca, runtime/algorithm implementations.
- Blast radius: no existing public IR field/schema constant is changed. Inventory for old IR literals/assertions is therefore **none requiring edits**, not a deferred check repair. New private-field backing avoids external literals. List all new DTO construction sites explicitly: SDK preparation_input.rs, host preparation_input.rs/preparation_host.rs, macro generated conversion tokens, fixture observations.rs/stage wrappers, driver/test files. WIT consumers: preparation_bindings.rs, preparation_host.rs, preparation_native.rs, preparation_wasm.rs, SDK preparation.rs/preparation_input.rs, macro lib.rs and the fixture. Capability-owned paint/key conversion affects region chains/annotations, config keys, seam keys, selected Model key/members and mesh facet/stroke values; include Custom exhaustively. Existing ordinary ir-types/prepass-types declarations and version assertions stay unchanged; row-03 prepared-surface assertions are extended, not rewritten as ordinary snapshots.
- Dispatch: LOCATIONS <=20 entries for those typed consumers/new DTO construction sites and existing old-version assertions, grouped by source IR; FACT <=5 lines for dependency status/shape and Cargo metadata feature reconciliation. Stop on any unlisted required mutation; split a new <=3-edit substep before proceeding, never improvise a fourth edit.
- Context M; authority docs/03 source-of-truth, ADR-0066/0045/0056 Decisions via FACT; Orca refs none.
- Verification: delegated inventories, no cargo required for read-only step.
- Exit: every imported producer symbol matches packet.spec exactly, old assertion map shows no changed IR/ordinary version, and every WIT consumer has an owning step; mismatch falsifies readiness.

### Step 2: Schema errors and typed canonical WIT

- Task IDs TASK-574; objective: author exact field/method/error contracts and capability-owned lossless paint/key/annotation/mesh records without widening shared WIT.
- Precondition: Step 1 export match; postcondition: separate input errors and typed methods without ordinary widening.
- Allowed reads: schema preparation enum and layer-preparation WIT only; ir-types.wit 101–187 and common.wit 44–52; prepass-types.wit all.
- Allowed edits: `crates/slicer-schema/src/preparation.rs`, `crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit`.
- Out of bounds: all IR layouts/constants, root ordinary worlds, scheduler and other packets.
- Blast radius: new enum/records only, no old struct-literal or old-schema-assertion fallout. Preparation paint/key fields use preparation-paint-value with flag/scalar/tool-index/custom, not existing incomplete ordinary paint-value/region-key/annotation imports. Keep seven piece errors/qualified interface unchanged; new input-error mappings belong to later SDK/host steps.
- Dispatch: FACT field inventory equals design, imports use canonical identities and native PaintValue has exactly Flag/Scalar/ToolIndex/Custom; context M; authority docs/02 IR 2/4/6 + docs/03 and slice_ir.rs PaintValue/modifier_sub_region_id symbol windows; Orca none.
- Verification: delegated `cargo build -p slicer-schema --all-targets`; independent contract field review; compile failure caused by unconverted planned consumers is recorded as expected staged red, never acceptance.
- Exit: all design table fields and distinct optional/error signatures exist in canonical WIT; any host-only surface field fails the exit.

### Step 3: SDK DTOs and production input facade

- Task IDs TASK-574; objective: public typed native/guest method shapes.
- Precondition: Step 2 canonical surface; postcondition: DTOs, PrintPreparationRead and additive constructor/methods registered.
- Allowed reads: SDK preparation facade from #03; SDK views.rs 1–45 and existing config conversions by named-symbol grep; design method/result table.
- Allowed edits: `crates/slicer-sdk/src/preparation_input.rs`, `crates/slicer-sdk/src/preparation.rs`.
- Out of bounds: SDK ordinary view widening, IR fields, plan format decoders, runtime.
- Blast radius: NET-NEW DTO literals only in these files; old fixtures untouched. Register PreparationPaintValue/PreparationVariantChain/PreparationRegionKey/PreparationPaintStroke/PreparationPaintLayer in the public preparation facade; exhaustive native roundtrip conversion retains Custom strings and signed-zero scalar bits. Keep row-03 from_host_reader/read-fixture constructor intact for controlled tests.
- Dispatch: FACT native/WASM owned DTO signatures and config filtering agree; context M; authorities docs/05 call-local lifecycle and docs/08 coordinate rule; Orca none.
- Verification: delegated `cargo build -p slicer-sdk --all-targets --features test`; later Step 7 proves guest conversions.
- Exit: every packet.spec method returns exact typed Result/Option, no host borrow masquerades as guest API.

### Step 4: Committed backing, deterministic indices and shared projection

- Task IDs TASK-574; objective: owner-bound prevalidation/batches without whole-print clones.
- Precondition: Steps 2–3, implemented shared selection; postcondition: CommittedPreparationInputs/PreparationSelectionContext/from_committed and errors reexported.
- Allowed reads: #01 selection.rs public seam and implementation relevant branches only; slice_ir.rs RegionKey/SlicedRegion/GlobalLayer windows; implemented #03 owner/binding/declaration interfaces.
- Allowed edits: `crates/slicer-wasm-host/src/preparation_input.rs`, `crates/slicer-wasm-host/src/preparation.rs`.
- Out of bounds: shared selection edits, runtime invocation policy, future output slots, raw config fallback.
- Blast radius: existing PreparationInputData fields private; no external literals. Add PreparationFailure::Input match handling within these files; remaining native/WASM enum matches explicitly mapped in Steps 5–6. New DTO/key/value construction and native full-key sorting occur only in this helper; use exact native modifier_sub_region_id(parent_region_id,object_id,footprint_geo) where needed, never a two-argument substitute.
- Dispatch: LOCATIONS exhaustive enum matches and exact source field mapping; FACT no independent target resolver; context M; authority docs/01 order, docs/02 identity, ADR-0066; Orca none.
- Verification: delegated `cargo check -p slicer-wasm-host --all-targets`; any remaining binding errors assigned to Steps 5–7 stay staged red, never a pass claim.
- Exit: missing required and missing exact authority fail before a transaction/call, perimeter=None is explicit, all original Arc products survive without cloning payload vectors.

### Step 5: Native production readers and host query guard

- Task IDs TASK-574; objective: enforce grants/owner/phase on native preparation input and mesh services.
- Precondition: indexed backing and SDK trait; postcondition: native adapter uses the same permission/range/backing functions as WASM.
- Allowed reads: preparation input helper; existing host.rs mesh helpers 2306–2423 and transform_mesh_point symbol window only; preparation_native.rs and prepared host service adapter windows.
- Allowed edits: `crates/slicer-wasm-host/src/preparation_native.rs`, `crates/slicer-wasm-host/src/preparation_host.rs`, `crates/slicer-wasm-host/src/host.rs`.
- Out of bounds: ordinary host service policy, algorithm code, independent test transport.
- Blast radius: new trait/DTO consumers and PreparationFailure::Input matches in the two preparation files; host.rs pure query helper visibility/bounds extraction only. Preserve existing ordinary bounds/query results; no existing public struct widening or literal churn.
- Dispatch: FACT query guards prohibit ordinary-import bypass inside preparation and preserve ordinary-call behavior; context M; authority docs/03 boundary and docs/08 units; Orca none.
- Verification: delegated `cargo check -p slicer-wasm-host --all-targets`; real negative adapter tests owned by Step 12.
- Exit: an undeclared mesh/optional read is denied before product/range lookup and controlled fixture access remains production-denied.

### Step 6: WASM imported resource handlers and binding identity

- Task IDs TASK-574; objective: typed guest input projection, not host-only backing.
- Precondition: WIT/backing/native readers; postcondition: fresh WASM preparation store maps input resource and all projection records/errors.
- Allowed reads: canonical preparation WIT; prep host/bindings/WASM adapters at generated-trait implementation windows only.
- Allowed edits: `crates/slicer-wasm-host/src/preparation_bindings.rs`, `crates/slicer-wasm-host/src/preparation_host.rs`, `crates/slicer-wasm-host/src/preparation_wasm.rs`.
- Out of bounds: generated files, ordinary world signatures and live-store retention.
- Blast radius: all host typed new-record conversion sites and remaining input-error enum arms in these three files; preparation-region-key/paint-value/annotations/mesh values include Custom and never route through incomplete ordinary records. No IR/schema assertion fallout.
- Dispatch: FACT imported resource identity and every enriched slice/surface/support field maps, no provider argument; context M; authority docs/03 WIT contract and ADR-0045; Orca none.
- Verification: delegated `cargo build --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures` (mandatory test-target build after WIT); compile errors caused by pending guest glue are explicitly staged red until Step 7.
- Exit: actual generated host trait methods exist for every SDK method and all errors map without flattening absent/denied.

### Step 7: Macro composed guest conversions and compile witness

- Task IDs TASK-574; objective: prepared guest reads same typed shape while plain worlds remain unchanged.
- Precondition: SDK/host canonical binding alignment; postcondition: macro glue encodes all DTO/error variants and plain entry surfaces are preserved.
- Allowed reads: macro lib.rs relevant include_str/composed-preparation helpers located by grep; preparation SDK/ WIT; existing binding_surface_tdd.rs corresponding prepared tests.
- Allowed edits: `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/tests/binding_surface_tdd.rs`.
- Out of bounds: generated expansion dumps, ordinary stage roster/signature changes, non-preparation tests.
- Blast radius: generated DTO conversion literals live in lib.rs tokens, including capability-owned keys/paint/annotations/mesh and exhaustive Custom handling; extend producer's prepared binding compile assertions only. All existing ordinary/schema literal assertions remain unchanged. No hardcoded new IR version.
- Dispatch: FACT every model/non-model/full-chain/error branch compiled and no ordinary mandatory prep imports; context M; authority docs/03 and ADR-0045/0056; Orca none.
- Verification: delegated `cargo build --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures`; Step 10 guest build validates wasm target.
- Exit: all target build green without weakening prior assertions, and prepared worlds type-link with host-owned resource identity.

### Step 8: Single-source literal fixture package

- Task IDs TASK-574; objective: author meaningful typed observations via real resource methods.
- Precondition: typed SDK/macro; postcondition: same-source native/guest fixture package exists and declared reads/config keys are exact.
- Allowed reads: row-03 preparation-transport-path fixture package only via bounded exported-source summary, SDK preparation methods; existing guest directory conventions.
- Allowed edits, separate atomic substeps: 8a only `crates/slicer-wasm-host/test-guests/whole-print-input-path/src/observations.rs`; 8b only that package's `Cargo.toml`, `src/lib.rs`, `module.toml`; 8c only `whole-print-input-perimeters/{Cargo.toml,src/lib.rs,module.toml}`; 8d only `whole-print-input-infill/{Cargo.toml,src/lib.rs,module.toml}`; 8e only `whole-print-input-support/{Cargo.toml,src/lib.rs,module.toml}`; 8f only `whole-print-input-anchored/{Cargo.toml,src/lib.rs,module.toml}`, all under `crates/slicer-wasm-host/test-guests/`. Each substep is M or smaller and <=3 edits; wrappers include the shared source and exactly one actual ordinary stage.
- Out of bounds: real community modules, production algorithm guest changes, fixture-local substitute stores.
- Blast radius: new fixture DTO inspections/literals only; add standalone workspace sentinel, library rlib/cdylib and SDK/schema dependencies matching producer pattern. No manually generated lockfile or artifact commit.
- Dispatch: FACT fixture invokes every input resource method and writes observations to production output; context M; authority docs/05 SDK + docs/21/22; Orca none.
- Verification: static API/declaration review now; discovery/build and real invocation in Steps 9–10.
- Exit for 8a: shared observation source calls typed preparation methods only; exit for each 8b–8f: package schema/native entry/manifest/preparation all agree on its real single stage, native/guest use the shared observations, and ordinary body is stage-appropriate no-work. Never bind a PathOptimization owner with Perimeters/Infill/Support/AnchoredEvents selection context. Independent pinned observation format includes all fields and negative modes, not merely leg equality or call success.

### Step 9: Driver and test registration (red first)

- Task IDs TASK-574; objective: give ACs an actual production preparation driver and registered test home.
- Precondition: fixture package; postcondition: native fixture dev-dependency, driver and registrations compile.
- Allowed reads: contract/main.rs and production_guest_smoke_tdd.rs typed driver; row-03 production preparation driver by bounded summary; feature metadata.
- Allowed edits: `crates/slicer-wasm-host/Cargo.toml`, `crates/slicer-wasm-host/tests/contract/whole_print_input_driver.rs`, `crates/slicer-wasm-host/tests/contract/main.rs`.
- Out of bounds: runtime run_slice/scheduler driver invention, test-only input/store implementations, guest artifact silence skips.
- Blast radius: new dev dependency only, no broad production dependency changes; register driver now under preparation-test-fixtures. Tests module registration occurs Step 10 when its file exists, keeping this step compilable.
- Dispatch: FACT driver freezes the correct one of five actual paired native/WASM fixtures, matches owner stage/module/claims, calls prepare/storage facades and returns observations/call markers; context M; authorities docs/22 actual falsifying witnesses and #03 forward exports; Orca none.
- Verification: delegated `cargo build -p slicer-wasm-host --all-targets --features preparation-test-fixtures`.
- Exit: driver creates literal committed IR through real constructors and can prove pre-call rejection without invoking either leg.

### Step 10: Enriched slices/surface/model-target acceptance tests

- Task IDs TASK-574; objective: TDD AC-1/2 with nonempty independent literal expectations.
- Precondition: registered real driver; postcondition: new tests module registered and the two field/selection tests pass.
- Allowed reads: driver, canonical DTO mapping table, producer selection exports; IR fields named in design windows.
- Allowed edits: `crates/slicer-wasm-host/tests/contract/whole_print_input_tdd.rs`, `crates/slicer-wasm-host/tests/contract/main.rs`, `crates/slicer-wasm-host/src/preparation_input.rs`.
- Out of bounds: paint producers, selection resolver, algorithms, other tests.
- Blast radius: new tests use FRU or exhaustive reason per docs/21; helper fixes restricted to projection defect exposed by these tests, not changing input authorities.
- Dispatch: FACT native/WASM literals independently checked, meaningful selected/unselected and shell/bridge geometry present; context M; authorities docs/02 fields and docs/22; Orca none.
- Verification: first capture failing tests, then delegated `cargo xtask build-guests` and `cargo xtask build-guests --check`, then AC-1 and AC-2 commands independently. Doc grep portions become acceptance-complete in Step 14; retain test results now without claiming final AC pass.
- Exit: both tests execute once, all table fields checked, and guest rather than host borrow supplies observations.

### Step 11: Non-model/mesh/seam/support/batch acceptance tests

- Task IDs TASK-574; objective: TDD AC-3/4/5/6 over the same production driver, including meaningful Custom-chain permutations and native reverse-conversion.
- Precondition: model/surface positive witness; postcondition: complete source/products/optional-state/batch positive tests.
- Allowed reads: support/seam IR windows, mesh-query signatures and driver; canonical ordering rules in design.
- Allowed edits: `crates/slicer-wasm-host/tests/contract/whole_print_input_tdd.rs`, `crates/slicer-wasm-host/src/preparation_input.rs`, `crates/slicer-wasm-host/src/preparation_host.rs`.
- Out of bounds: support rendering predicates, Layer outputs, independent targeting, query algorithm changes.
- Blast radius: new fixture records use FRU/waivers; test instrumentation observes real Arc/backing/counters and does not implement an equivalent store. Dedicated custom_paint_identity_literals test covers all four slice/map insertion permutations, nonempty selected geometry, Model members/config/seam keys, Custom annotations/mesh values and signed-zero bits on each leg. Expected literals must not call the conversion under test. Shared fixture observation support was authored in Step 8a; no fourth fixture edit here. Expose controlled introspection only under existing preparation-test-fixtures.
- Dispatch: FACT nonempty carrier/support/seam witnesses, literal triangle query and order/range expectations; context M; authorities docs/01 order + docs/08 units; Orca none.
- Verification: red tests, freshness check/rebuild if needed, AC-3/4/5/6 test portions individually with exact tee commands; doc greps land Step 14.
- Exit: all four execute once on both production legs; absent vs present-empty and original carrier entry index preserved; all Custom full identities/annotation/mesh values survive exact independent literal roundtrip/permutations without ordinary WIT changes; no unsupported rendering assertion.

### Step 12: Denied/missing/invalid negative controls

- Task IDs TASK-574; objective: TDD AC-N1/N2/N3 before-call and actual-resource negatives.
- Precondition: positive driver working; postcondition: before-call rejection has zero invocation counters/markers, while real inside-call resource rejection has nonzero markers and exact resource errors; returned fixture failure publishes no staged pieces without inventing read-induced output poison.
- Allowed reads: owner checks, production input trait/resource handlers, row-03 transaction semantics; test driver.
- Allowed edits: `crates/slicer-wasm-host/tests/contract/whole_print_input_tdd.rs`, `crates/slicer-wasm-host/src/preparation_input.rs`, `crates/slicer-wasm-host/src/preparation_host.rs`.
- Out of bounds: weakening row-03 errors, fixture fallback, foreign provider access, ordinary ir-access grants.
- Blast radius: no field/schema additions; private production overflow counters/test-hook are mapped only here under test fixture feature. Invalid fixtures use exact root-cause/input pairs. Snapshot/required/missing-authority/unsupported-token cases drive constructor prevalidation; ranges/unknown object/undeclared read/foreign handle/handler overflow drive actual entered native/guest handlers. Shared Step 8a observation modes record the input error before deliberately returning ModuleError, or catch it and succeed, to distinguish transaction failure from output poison.
- Dispatch: FACT each prevalidation case asserts exact variant plus zero invocations/no transaction, and each resource case asserts exact input error plus nonzero markers; failure-returning fixtures assert NotReady/no staged publication and never PoisonedOutput, while caught-read success can publish; context M; authority docs/03 boundary and docs/22 non-vacuity; Orca none.
- Verification: capture red first, freshness check/rebuild, three negative AC test portions individually; mandatory log inspected, not rerun for output truncation.
- Exit: each exact test executes once. Constructor/from_committed negatives have zero invocations; resource negatives have nonzero invocation markers with AccessDenied/RangeError/UnknownObject/ArithmeticOverflow as appropriate. Fixture-returned failure leaves NotReady/no publication; caught input errors do not poison output and the explicit success control publishes normally. Required missing and malformed snapshots never reach a guest handler.

### Step 13: Full WIT binding/compatibility acceptance

- Task IDs TASK-574; objective: reconcile all typed boundaries and old ordinary/schema assertions without changes to their contracts.
- Precondition: nine exact tests green individually; postcondition: all-target compile/gates, guest freshness and prior ordinary compatibility witness remain green.
- Allowed reads: bounded gate FACT summaries, design typed-consumer inventory, implemented row-03 plain compatibility test.
- Allowed edits: `crates/slicer-wasm-host/src/preparation_bindings.rs`, `crates/slicer-wasm-host/src/preparation_wasm.rs`, `crates/slicer-macros/src/lib.rs` only for integration defects; any other mutation requires a new bounded substep.
- Out of bounds: IR versions/fields, ordinary snapshot expected values, self-baseline weakening, full workspace tests.
- Blast radius: Step 1 inventory re-derived against final tree; **every old-schema assertion remains unchanged** because no IR/ordinary version changes. Prepared WIT compile assertions are accounted for in Step 7. No missing conversion is deferred to another packet.
- Dispatch: FACT all-target build/check/clippy, literals/test-quality, freshness and row-03 plain artifact witness; context M; authority docs/03 identity, ADR-0045/0056; Orca none.
- Verification: complete requirements matrix, plus producer's exact ordinary compatibility AC command (tee/executed count). Build after WIT must include --all-targets; no zero-test feature pass.
- Exit: every listed binding consumer builds, old archived ordinary artifact instantiates unchanged, artifact check exit 0, and touched literal/test-quality findings fixed or justified.

### Step 14: Canonical documentation sections

- Task IDs TASK-574; objective: document contracts without algorithm/scheduling claims.
- Precondition: transport evidence complete; postcondition: exact new sections in docs/01/02/03/05 and all doc greps pass.
- Allowed reads: those docs' named affected sections via bounded FACT summaries, accepted field/permission/source inventories.
- Allowed edits substep 14a (S): `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`; substep 14b (S): `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`. Each atomic substep edits only two files.
- Out of bounds: other docs/ADRs, scheduler normative order, plan/backlog, packet prerequisites.
- Blast radius: doc-only, no literal/schema assertions.
- Dispatch: FACT field/permission/doc-phase correctness and precise section greps, then every complete independent AC command; context S each substep; authorities ADR-0066 and source plan read context, docs/02 exact fields; Orca none.
- Verification: exact four section greps in packet.spec plus semantic FACT review of capability-owned paint/key mappings and prevalidation/handler failure distinction; all nine complete AC commands rerun only as final acceptance, not to retrieve output.
- Exit: all doc sections reflect optional absence/denied, source membership/authority, PrePass phase/paint gaps and no Layer-output fiction.

## Per-Step Budget Roll-Up

| Steps | Cost | Slice |
| --- | --- | --- |
| 1–7 | M each | bounded inventory/schema/SDK/backing/adapters/guest glue |
| 8–13 | M each | literal fixture/driver/positive/negative/binding verification |
| 14a,14b | S each | exact doc sections |

Aggregate M: sequential disposable worker contexts, no single L step or controller full-source accumulation. Split further before execution if any worker needs L.

## Completion Gate and Acceptance Ceremony

Every exit, every full AC and requirements gate must pass through delegated FACT reports with exact execution counts. No independent preflight/implementation result is asserted by this author. Keep TASK-574 open until #03–#05 collectively close; backlog/queue updates and any status transition belong to the separately authorized coordinator, not an implementation worker's implicit extra edit. No workspace suite is required, no activation/commit performed here. Record known paint-preservation and lightning geometry evidence boundaries rather than calling them resolved.
