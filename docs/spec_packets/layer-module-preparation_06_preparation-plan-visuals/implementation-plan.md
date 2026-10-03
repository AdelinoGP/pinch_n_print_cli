# Implementation Plan: preparation-plan-visuals

## Execution Rules

Draft only. Do not execute until prerequisites and independent preflight pass. Work in the listed order; each step maps to TASK-575. Use tests first where a production-driver home exists, then the narrowest falsifying check. Intermediate WIT/binding tranche steps may compile only their owned crate; do not claim workspace readiness until the composed adapter gate passes. Do not commit an incomplete tranche. Every test command below uses combined-output tee, pipefail and a nonzero-test assertion; read the saved log rather than rerunning for output.

## Steps

### Step 1 — schema diagnostic error
- Task IDs: TASK-575. Objective: add the separate typed projection error enum without changing transport/input variants.
- Precondition: #03/#04 schema module and pilot accepted. Postcondition: exact ten projection variants in design resolve independently.
- Allowed reads: `crates/slicer-schema/src/preparation.rs` error/declaration symbol windows; #03/#04 public exports; design typed publication section.
- Allowed edits (1): `crates/slicer-schema/src/preparation.rs`.
- Out of bounds: all producer packet documents, ordinary WIT, IR, runtime and CLI.
- Blast radius: additive enum only, no existing fields/version literals. Dispatch: precise error-shape reconciliation, FACT <=5 lines.
- Context S. Authorities: ADR-0066 committed diagnostics and approved plan Projection data and publication; no Orca refs.
- Verification: `cargo check -p slicer-schema --all-targets`, FACT pass/fail.
- Exit: all variants compile and original preparation error identities remain unchanged; no ambiguous shared transport error.

### Step 2 — SDK types and optional native sink
- Task IDs: TASK-575. Objective: establish typed scene/primitive/sink API and no-request None behavior.
- Precondition: Step1 enum. Postcondition: SDK exports the exact DTOs/trait/output method, host constructors and callback dispatch; add projection tests to existing driver-capable SDK home.
- Allowed reads: `crates/slicer-sdk/src/preparation.rs` output constructor/method windows; `crates/slicer-sdk/tests/layer_module_tdd.rs` existing native adapter tests; design DTO inventory.
- Allowed edits (3): `crates/slicer-sdk/src/preparation_visual.rs`, `crates/slicer-sdk/src/preparation.rs`, `crates/slicer-sdk/tests/layer_module_tdd.rs`.
- Out of bounds: ordinary traits/native entries/worlds and other packets.
- Blast radius: no existing public fields; optional callback private representation initialized at every output constructor in preparation.rs; new DTO test literals obey docs21. Dispatch: facade constructor/None behavior, FACT <=5 lines.
- Context M. Authorities: docs05 Module State Lifecycle, ADR-0066 optional sink, docs21/22; no Orca refs.
- Verification: SDK `layer_module_tdd preparation_projection` matrix command with `--features test`; `cargo check -p slicer-sdk --all-targets --features test`.
- Exit: callbacks preserve coordinates/labels/ids and output.projection() None adds zero publications; negative callback returns exact enum, not successful empty fallback.

### Step 3 — canonical typed projection WIT
- Task IDs: TASK-575. Objective: define imported sink/resource/records and optional resource presence in the preparation capability.
- Precondition: existing accepted #03/#04 preparation WIT composition. Postcondition: design's exact kebab-case shapes use canonical point2/ex-polygon identity.
- Allowed reads: `crates/slicer-schema/wit/deps/types.wit` geometry interface, preparation-types resource section and #02 compiled composition result.
- Allowed edits (1): `crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit`.
- Out of bounds: all ordinary WIT deps/worlds, stage package versions and shared IR.
- Blast radius: only unreleased preparation draft extends; generated host/SDK handlers are reconciled in Steps4–7 before whole binding tranche gate; no unrelated version assertions change. Dispatch: typed resource identity review, SNIPPETS <=3 x30 lines.
- Context S. Authorities: docs03 canonical WIT source, approved plan optional sink and ADR-0066; no Orca refs.
- Verification: `cargo check -p slicer-schema --all-targets`; composed compile/instantiate exit remains Step9, not falsely claimed here.
- Exit: canonical records are exact and no ordinary dependency changed; malformed typed shape is rejected by WIT parsing/build in the tranche gate, not tolerated as opaque bytes.

### Step 4 — actual atomic store attachment
- Task IDs: TASK-575. Objective: implement owner-bound typed staging, checked validation, snapshot Arc/accounting and publication wrapper.
- Precondition: Steps1–3 and #03 finish/poison precedence. Postcondition: one existing finish commits plan+all requested scenes or neither.
- Allowed reads: preparation_store.rs owner/begin/finish/dispose/metadata windows, preparation.rs frozen-adapter windows; approved plan atomic publication section.
- Allowed edits (3): `crates/slicer-wasm-host/src/preparation_projection.rs`, `crates/slicer-wasm-host/src/preparation_store.rs`, `crates/slicer-wasm-host/src/preparation.rs`.
- Out of bounds: decoder/codecs, ordinary host IR, runtime scheduling, producer docs.
- Blast radius: private store state only; initialize new attachment/poison/accounting at all private constructors in these files, expose metadata by method not fields. Add embedded production-validator tests here for topology/overflow and finish before Ready. Dispatch: predicate/atomicity audit, SUMMARY <=200 words.
- Context M. Authorities: ADR-0066, docs08 integer/float boundary, docs22 negative controls; no Orca refs.
- Verification: `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --lib preparation_projection 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` after Steps5–7 restore full generated Host compilation; capture partial-tranche compile limitations explicitly before that point.
- Exit: independent valid/invalid ring, hole, extreme-coordinate, duplicate/missing/undeclared scene, foreign-owner and counter-overflow controls exercise actual production validator; requested empty completion succeeds; catching write faults cannot produce Ready.

### Step 5 — imported host sink and native callback
- Task IDs: TASK-575. Objective: owner/call-local optional sink on both real backings, without resource-index retention.
- Precondition: Steps2–4 typed/store API. Postcondition: handlers forward to same transaction and native facade, not a second collector.
- Allowed reads: preparation_host.rs/resource table windows, preparation_native.rs SDK constructor adaptation and store projection methods.
- Allowed edits (2): `crates/slicer-wasm-host/src/preparation_host.rs`, `crates/slicer-wasm-host/src/preparation_native.rs`.
- Out of bounds: HostExecutionContext fields, ordinary resource identities and runtime.
- Blast radius: private backing constructors in these files own all their new optional sink initializers; no public-field churn. Dispatch: owner mismatch/call-local guard wiring, SNIPPETS <=3 x30 lines.
- Context M. Authorities: docs03 imported resource identity, docs05 native facade, ADR-0066 atomicity; no Orca refs.
- Verification: `cargo check -p slicer-wasm-host --all-targets` after Step6; GateA commands in Step9 provide real behavior.
- Exit: all projection methods return exact typed errors; None makes no geometry collector, stale/foreign handles AccessDenied without mutating another owner's transaction.

### Step 6 — composed bindings/WASM adapter
- Task IDs: TASK-575. Objective: link new imports once and marshal identical typed primitives.
- Precondition: Step5 handlers. Postcondition: fresh prepared component stores reuse canonical geometry/resource mappings and diagnostic publication wrapper.
- Allowed reads: preparation_bindings.rs bindgen with-map windows, preparation_wasm.rs actual invoke/linker windows, SDK DTO methods.
- Allowed edits (2): `crates/slicer-wasm-host/src/preparation_bindings.rs`, `crates/slicer-wasm-host/src/preparation_wasm.rs`.
- Out of bounds: ordinary host linker registrations/signatures, instance retention, runtime.
- Blast radius: no public envelopes; new Host trait requirements/mappers handled here and Step5; checked conversions cannot silently truncate. Dispatch: linker/resource identity, SNIPPETS <=3 x30 lines.
- Context M. Authorities: docs03 typed instantiation, #02 proof/#03 wrapper identity, ADR-0066; no Orca refs.
- Verification: `cargo check -p slicer-wasm-host --all-targets`; Step4 embedded matrix command; guest freshness as requirements before runtime attribution.
- Exit: typed Host impls compile for actual composed supported stages; duplicate-import/wrong-identity controls cannot instantiate as success.

### Step 7 — macro guest conversion tranche
- Task IDs: TASK-575. Objective: expose output.projection() through real macro-generated guest preparation adapter.
- Precondition: Steps2/3/6. Postcondition: guest/native same-source API compiles, plain exports unchanged.
- Allowed reads: slicer-macros lib generate_slicer_module_impl preparation token windows; binding_surface_tdd compiled schema assertions; SDK conversion patterns.
- Allowed edits (2): `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/tests/binding_surface_tdd.rs`.
- Out of bounds: ordinary stage schema/version repins, other macro tests and packet documents.
- Blast radius: additive preparation conversion tokens only; original hard stage package assertions remain pinned. Dispatch: actual generated mapping coverage, FACT <=5 lines.
- Context M. Authorities: docs03 canonical WIT and docs05 authoring, docs22 compile witness limitations; no Orca refs.
- Verification: `set -euo pipefail; mkdir -p target; cargo test -p slicer-macros --test binding_surface_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`; `cargo build --tests`.
- Exit: exact DTO/sink resource identity compiles through macro and original plain binding assertions pass; compilation is not claimed as dispatch coverage.

### Step 8 — same-source actual visual guest
- Task IDs: TASK-575. Objective: authored fixture with independently testable plan/output/geometry and negative modes.
- Precondition: Step7 macro contract. Postcondition: dual native/WASM prepared PathOptimization artifact declares probe_xy and actual snake_case config modes.
- Allowed reads: #05 lifecycle PathOptimization fixture and actual SDK run_path_optimization signature, design fixture seeds; guest-builder discover_guests named window.
- Allowed edits (3): `crates/slicer-wasm-host/test-guests/preparation-visual-guest/Cargo.toml`, `crates/slicer-wasm-host/test-guests/preparation-visual-guest/src/lib.rs`, `crates/slicer-wasm-host/test-guests/preparation-visual-guest/module.toml`.
- Out of bounds: producer fixtures, production modules, xtask registration and legacy lightning.
- Blast radius: new fixture only; production literals exhaustive, new watched test literals FRU/waived. Dispatch: fixture scope/oracle review, SUMMARY <=200 words.
- Context M. Authorities: docs05 from_config/paired adapter, docs08, docs22 and ADR-0066; no Orca refs.
- Verification: `cargo xtask build-guests --check`, rebuild stale via requirements; `cargo build --tests` after freshness is proven.
- Exit: actual component/native library exist; no-request ordinary comments and nonempty probe geometry are meaningful independent controls; fault modes report actual returned errors.

### Step 9a — production driver and fixture dev graph
- Task IDs: TASK-575. Objective: establish actual native/component driver, not test-local transport.
- Precondition: Step8 artifacts and composed adapter compilation. Postcondition: driver directly calls frozen production adapters and fixture native libraries are legal dev dependencies.
- Allowed reads: #03 transport driver HostExecutionContext/store windows, design fixture seeds, host/runtime Cargo dev dependency sections.
- Allowed edits (3): `crates/slicer-wasm-host/Cargo.toml`, `crates/slicer-runtime/Cargo.toml`, `crates/slicer-wasm-host/tests/contract/preparation_projection_driver.rs`.
- Out of bounds: other Cargo production graphs, producer driver/storage implementations and packet docs.
- Blast radius: dev dependencies/new driver only, no public fields. Dispatch: paired fixture/native/component setup FACT <=5 lines.
- Context S. Authorities: docs03 typed instantiation and #03 production driver authority, docs21/22; no Orca refs.
- Verification: freshness matrix; `cargo check -p slicer-wasm-host --all-targets --features preparation-test-fixtures`; driver compilation is completed by explicit registration in 9b, not falsely claimed from an unregistered source file.
- Exit: both dependencies resolve and driver source uses production adapters with loud artifact absence; no ready-plan injection or test-local Host implementation.

### Step 9b — registered gate A contract acceptance
- Task IDs: TASK-575. Objective: close publication/ownership/rollback with actual native/component calls.
- Precondition: Step9a driver/dependencies and Step8 artifacts. Postcondition: every gateA exact test and driver is registered under the real contract binary.
- Allowed reads: `crates/slicer-wasm-host/tests/contract/main.rs`, preparation_projection_driver.rs entrypoints, design validator/seeds and packet.spec gateA criteria.
- Allowed edits (2): `crates/slicer-wasm-host/tests/contract/preparation_projection_tdd.rs`, `crates/slicer-wasm-host/tests/contract/main.rs`. Register both driver/test modules with `#[cfg(feature = "preparation-test-fixtures")] mod ...;`.
- Out of bounds: Cargo manifests, producer tests/driver/storage implementation and packet documents.
- Blast radius: new test literals obey docs21, no public field changes. Dispatch: registrations/features/nonzero test counts FACT <=5 lines.
- Context M. Authorities: docs03/08/21/22 and #03 store atomicity; no Orca refs.
- Verification: freshness matrix followed by gateA contract matrix; each AC-1/2/N1 exact command; `cargo check -p slicer-wasm-host --all-targets --features preparation-test-fixtures`.
- Exit: both legs match independent literals; named negative mutations reject both attachments, original errors/traps win, lifetime/overflow/accounting controls are observed. No success-only or zero-test gate.

### Step 10 — shared snapshot capture and renderer enum
- Task IDs: TASK-575. Objective: integrate generic diagnostic projection into actual StageCapture/render path.
- Precondition: Step9 gateA passed. Postcondition: shared Arc variant, pinned projection schema, generic XY rendering/bounds, explicit ordinary-style refusal and public-scene-only serialization under the existing kind/value envelope.
- Allowed reads: layer_executor CapturedIr/schema_version_string/StageCapture windows; renderer Projector/geometry_points_mm/shapes_for/styled/silhouette match windows; render_tap exhaustive match.
- Allowed edits (3): `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/visual_debug_render.rs`, `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs`.
- Out of bounds: StageCapture fields, ordinary IR schema constants, algorithms, CLI serializers.
- Blast radius: all inventoried exhaustive CapturedIr arms in these exact files owned here; preserve its actual serde::Serialize derive and kind/value tagging. New tuple field alone gets serialize_with="serialize_preparation_projection"; the private adapter in layer_executor.rs serializes only CommittedPreparationProjection::scene(), not the private committed type/Arc/store/opaque pieces. No struct-literal widening or Serialize on private host types. Existing if-let/wildcard callers need no edits. Dispatch: rederive exhaustive matches/unchanged schema assertions LOCATIONS <=20.
- Context M. Authorities: docs19 Projector/shared bounds, docs08, docs22; no Orca refs.
- Verification: render_tap matrix command, including new `preparation_projection_serializes_only_scene`; `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test visual_debug_render_tap_tdd preparation_projection_serializes_only_scene -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`; `cargo check -p slicer-runtime --all-targets`.
- Exit: point placement, open-line endpoints, hole background and outline control pixels match independent literals; unsupported dimension/tool/overlay styles never render an empty success. Same-file real native preparation helper obtains validated Ready scenes; nonempty enum serialization equals complete independent literal JSON with only kind/value and the four public scene fields, shared-Arc captures have ptr_eq/identical JSON, and explicitly completed empty scenes serialize primitives=[] after private-store disposal. Exact-key/equality checks reject every additional private field. Tests live in the existing auto-discovered render_tap binary; no new module/aggregator or fourth edit file is needed. Step16's AC-5 CLI test checks the matching typed_capture envelope, with AC-6's empty-scene control; this step closes their serialization trace without duplicating ACs.

### Step 11 — private runtime visual companion
- Task IDs: TASK-575. Objective: opt-in views and typed snapshots without metadata envelope churn.
- Precondition: #05 public exports exact and Step10. Postcondition: design's companion/new request and extension builder registered, private ownership allows typed post-disposal snapshots only.
- Allowed reads: preparation_lifecycle private extension constructors/print scope/observer windows, run_slice_preparation_capture API; design companion shape.
- Allowed edits (3): `crates/slicer-runtime/src/preparation_lifecycle.rs`, `crates/slicer-runtime/src/preparation_visual_capture.rs`, `crates/slicer-runtime/src/lib.rs`.
- Out of bounds: PreparationCapture/record public fields, SliceRunOptions, ProgressEvent and raw plan payload access.
- Blast radius: private extension constructors new/Default initialized together; new companion getters, no historical literals. Root lib initially exports companion; function export lands in Step12. Dispatch: capture redaction/Arc ownership FACT <=5 lines.
- Context M. Authorities: ADR-0066, #05 shared prefix/no-arena/observer authority, docs17; no Orca refs.
- Verification: `cargo check -p slicer-runtime --all-targets`.
- Exit: no raw payload/plan handle in snapshot; rendered/actual execution sets are distinct; no-request extension remains semantically default.

### Step 12 — production prefix and requested-tap completion
- Task IDs: TASK-575. Objective: run one preparation for preparation-only or mixed tap captures through validated real startup.
- Precondition: Step11 companion and #05 PreparedLayerStageRunner/closure. Postcondition: exact visual run facade forwards through production prefix; requested owner/view resolved before invocation and schedule/eligibility before prepare.
- Allowed reads: run.rs shared prefix/dispatcher windows, pipeline core PrePass/completion windows, actual capture executor named windows.
- Allowed edits (3): `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/pipeline.rs`, `crates/slicer-runtime/src/lib.rs`.
- Out of bounds: loader/routing resolver forks, runtime public envelope fields, other packets.
- Blast radius: private completion enum/forwarders only; public metadata capture signature unchanged; root function reexport here. Dispatch: production/capture prefix reachability SUMMARY <=200 words.
- Context M. Authorities: docs19 closure classes, #05 lifecycle/all-Ready/disposal contract, ADR-0066; no Orca refs.
- Verification: `cargo check -p slicer-runtime --all-targets`; Step14 driver tests establish behavior after executor forwarding.
- Exit: one real prepared owner invocation, no second slice or metadata-as-payload shortcut; inactive/unresolved requests exit before module entry, preparation-only mode cannot enter layer/anchored closure.

### Step 13 — lifecycle-aware tap executor forwarding
- Task IDs: TASK-575. Objective: mixed ordinary taps consume the committed plan using the same ready-guarded runner and captured scenes.
- Precondition: Step12 tap completion. Postcondition: selected-layer/required-postpass closure reaches real PreparedLayerStageRunner, snapshots before disposal, and records actual execution.
- Allowed reads: layer_executor execute_captured_stages_with_support_tools/private stage loop; preparation_visual_capture companion; pipeline private forwarding windows.
- Allowed edits (3): `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/preparation_visual_capture.rs`, `crates/slicer-runtime/src/pipeline.rs`.
- Out of bounds: ordinary public capture struct widening, synthetic anchored commits converted to dispatch, stage-order/IR changes.
- Blast radius: additive private forwarding path with existing public functions default forwarding; StageCapture shared Arcs not full clones. Dispatch: execution evidence/joins review SUMMARY <=200 words.
- Context M. Authorities: docs19 per-layer/postpass closure, #05 real-vs-synthetic anchored distinction, ADR-0066; no Orca refs.
- Verification: `cargo check -p slicer-runtime --all-targets`; Step14 exact driver commands.
- Exit: ordinary and postpass taps preserve their actual closure; no LayerArena in preparation-only mode; cancellation/error unwind drains then disposes, without forged layer expansions.

### Step 14 — real runtime gate and neutrality tests
- Task IDs: TASK-575. Objective: falsify prefix divergence, payload leakage, missing readiness and output perturbation.
- Precondition: Steps12/13 and fixture dev dependency Step9a. Postcondition: real native/external-WASM fixtures use run facades, e2e driver registered.
- Allowed reads: existing e2e run_slice_api_tdd driver and common helpers, #05 observer actual signatures, visual fixture.
- Allowed edits (2): `crates/slicer-runtime/tests/e2e/preparation_visual_capture_tdd.rs`, `crates/slicer-runtime/tests/e2e/main.rs` (`mod preparation_visual_capture_tdd;`).
- Out of bounds: producer runtime tests/fixtures, injecting Ready plans and ordinary output assertions weakened for capture.
- Blast radius: new tests only; FRU/exhaustive waiver for watched types. Dispatch: executed tests/features FACT <=5 lines.
- Context M. Authorities: docs22 independent values, docs21 literals, #05 metadata/read accounting and docs17 evidence; no Orca refs.
- Verification: freshness gate then e2e preparation_visual_capture_tdd matrix; AC-3/4 exact commands.
- Exit: neutral G-code/independent literal decoder outputs, zero no-request diagnostic allocation, real lifecycle/arena counts, shared prefix and cancellation/error cleanup all observed; timing is absent unless measured/instrumented.

### Step 15 — strict versioned request and bundle serializer
- Task IDs: TASK-575. Objective: add preparation selector/1.4.0 without widening public structs or breaking legacy serializers.
- Precondition: Step14 runtime acceptance. Postcondition: main visual-debug path delegates new requests to companion, strict syntax/errors/dimensions and private bundle writer use existing fail-closed boundary.
- Allowed reads: visual_debug.rs TapSelector/validate_request/tap_name/version helpers/model capture/writer windows, current Manifest/ImageEntry shapes.
- Allowed edits (2): `crates/pnp-cli/src/visual_debug.rs`, `crates/pnp-cli/src/visual_debug_preparation.rs` (path child registration in parent).
- Out of bounds: public request/options/manifest struct fields, CLI command proliferation, legacy constant repins.
- Blast radius: two exhaustive TapSelector matches in validator/tap_name are local to visual_debug.rs; enum variant added there. New helper serializers have no historical literals. New VERSION_1_4 keeps legacy 1.0–1.3 and legend literals locked; strict_options/schema_supported/silhouette acceptance explicitly include 1.4. Legacy assertions stay in original tests and run in Step16. Dispatch: deserialization fallback/version inventory LOCATIONS <=20.
- Context M. Authorities: docs19 schema compatibility/framing/write boundary, docs08, ADR-0066; no Orca refs.
- Verification: `cargo check -p pnp-cli --all-targets`; Step16 behavior commands own new and old schema fallout before closure.
- Exit: structured selector cannot disappear via untagged Detail fallback; front/side/G-code/unknown/inactive requests have named errors, validation/render finish before overwrite, and legacy writer serializes unchanged output.

### Step 16 — real CLI gate B
- Task IDs: TASK-575. Objective: prove bundle metadata, pixels, snapshot sharing and old schema compatibility.
- Precondition: Step15 and actual component staged via module_dirs. Postcondition: new ungated auto-discovered CLI integration binary drives public run_visual_debug and CLI process.
- Allowed reads: CLI visual_debug_request_bundle_tdd helpers, visual_debug_agent_determinism_tdd pins, layer_range_scope_visual_debug_tdd legacy schedule assertions and runtime companion getters.
- Allowed edits (1): `crates/pnp-cli/tests/preparation_visual_bundle_tdd.rs`.
- Out of bounds: editing legacy expectation pins, test-local renderer/store/resolver and producer fixtures.
- Blast radius: new tests plus untouched legacy test assertion matrix; all watched literals FRU/waived. No new CLI aggregator/Cargo entry needed, existing Cargo auto-discovery verified. Dispatch: independent pixels/oracles and legacy serializers SUMMARY <=200 words.
- Context M. Authorities: docs19 bundle/compatibility, docs22 falsifiable tests, docs21 literals, docs08; no Orca refs.
- Verification: freshness gate, CLI new bundle matrix and all existing legacy matrix commands; AC-5/6/7/N2 exact commands. Add documented_request_shape here for AC-8.
- Exit: nonempty ordinary+plan images share bounds; hole/background/point/line control pixels match independent expectations; subset full-scene Arc/color/framing, empty-vs-missing, exact named errors and unchanged overwrite sentinel/legacy outputs are falsified by controls.

### Step 17a — normative typed transport and authoring docs
- Task IDs: TASK-575. Objective: document accepted sink, resource identities and atomicity.
- Precondition: gateA/gateB behavior passed. Postcondition: exact transport and SDK headings from AC-8 reflect actual types.
- Allowed reads: docs03 canonical types/manifest sections, docs05 preparation authoring/lifecycle windows, implemented sink signatures.
- Allowed edits (2): `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`.
- Out of bounds: ADRs/source plan/backlog/producer packet docs and unrelated normative sections.
- Blast radius: no code/version fields. Dispatch: typed contract fact-check SUMMARY <=200 words.
- Context S. Authorities: approved plan Q18/Q21 and ADR-0066; no Orca refs.
- Verification: `rg -q '^### Preparation diagnostic projection transport \(Normative\)$' docs/03_wit_and_manifest.md; rg -q '^### Optional preparation projection sink \(Normative\)$' docs/05_module_sdk.md` with `set -euo pipefail`; AC-8 complete command follows 17b.
- Exit: optional typed sink and original plan/private-codec boundary are explicit, with exact API examples and no new ordinary IR read permission.

### Step 17b — normative visual and execution evidence docs
- Task IDs: TASK-575. Objective: document actual versioned requests/bundles and accounting exclusions.
- Precondition: Step17a documentation and gateB behavior. Postcondition: visual and instrumentation headings/examples complete AC-8.
- Allowed reads: docs19 schema/framing/tap closure and docs17 lifecycle/instrumentation named windows, implemented request/manifest serializers.
- Allowed edits (2): `docs/19_visual_debug.md`, `docs/17_agent_debugging.md`.
- Out of bounds: docs03/05, ADRs/source plan/backlog/producer packet docs and unrelated normative sections.
- Blast radius: no code/version fields. Dispatch: execution-versus-rendering evidence review SUMMARY <=200 words.
- Context S. Authorities: approved plan Q17–Q21 and ADR-0066; no Orca refs.
- Verification: AC-8 exact command with all four doc greps.
- Exit: example passes production validation, executed-versus-rendered layers/owners are distinct, logical bytes exclude allocator overhead and ordinary transfer accounting excludes typed diagnostics; no canonical/measurement claim without evidence.

## Per-Step Budget Roll-Up

| Steps | Context | Notes |
| --- | --- | --- |
| 1,3,9a,17a,17b | S each | Error/WIT/dev graph/docs bounded |
| 2,4–8,9b,10–16 | M each | Typed transport, actual execution and render boundaries |

Aggregate M working set, largest step M; bounded sequential tranches avoid retaining every implementation surface together. Stop and split if any step needs L. 9a/9b and 17a/17b are separate edit steps, not permission to combine their files.

## Packet Completion Gate and Acceptance Ceremony

Both internal gates pass, all AC exact commands run nonzero tests, docs match actual APIs, guest freshness is exit0, cargo build --tests/check/clippy and literal/test-quality matrix complete. Re-dispatch every AC/gate with FACT pass/fail and bounded failures; review the full saved result summaries for multi-target runs. Record gaps/unintended effects and stop if any remains unresolved. No workspace test command is authorized here. No backlog/queue/status edit occurs in this authoring session; orchestrator owns subsequent generation state and activation/closure authority.
