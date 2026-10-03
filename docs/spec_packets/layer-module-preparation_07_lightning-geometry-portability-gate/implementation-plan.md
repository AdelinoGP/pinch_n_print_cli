# Implementation Plan: lightning-geometry-portability-gate

## Execution Rules

All steps below are future work for TASK-576, not actions taken during generation. Commands use non-interactive Bash, not PowerShell. Execute one bounded step at a time using TDD and independent expected data. Maintain separate A/B outcomes and never manufacture a pass to satisfy closure. No production migration or framework implementation is authorized. Every source edit list is exhaustive and has at most three actual files; Cargo-generated lockfiles are explicitly budgeted, never manually read or edited.

### Step1 — Reproducible canonical recorder

- Task: TASK-576. Objective: implement opt-in independent canonical capture/replay tooling and explicit provenance.
- Precondition: clean disposable source copy can be obtained; upstream build/slice invocation and recorder identity hooks are verified by delegation, not assumed.
- Postcondition: capture records restricted earlier geometry/raw config/origin records separately from B canonical domains/scalars and all resolved-policy/graph/output reference branches. GateASourceInput contains no later snapshot/lineage results or resolved policy. Caller grouping/clipping identity propagation and mutation-free canonical replay remain unchanged, with replay explicitly reference-only. Every permitted A field has independently audited earlier availability; missing source/formula evidence cannot be filled from the oracle.
- Allowed reads: source-plan Lightning planning geometry and acceptance windows; attribution/coordinates/test-quality docs; delegated canonical `Generator::Generator`, `generateInitialInternalOverhangs`, `generateTrees`, `PrintObject::bridge_over_infill`, `Layer::make_fills` in Fill.cpp, `Layer::getBestGroundingLocation`/`attach` in Lightning/Layer.cpp and `Filler::_fill_surface_single`; upstream build/CLI entry symbols only through delegation.
- Allowed edits (3): `tools/lightning_gate/record.py`; `tools/lightning_gate/canonical-recorder.patch`; `tools/lightning_gate/README.md`.
- Forbidden: sibling source checkout edits; candidate/host expected-data capture; all production/framework code, other packets, source-plan body and backlog.
- Blast radius: no PnP public fields/constants change. Opt-in patch applies only to a disposable clone. PassA owns exactly Lightning/Generator.cpp, Lightning/Generator.hpp and PrintObject.cpp; all shared probe declarations/implementation reside there. PassB owns exactly Lightning/Layer.cpp, Fill/Fill.cpp and Fill/FillLightning.cpp, including guarded Generator.hpp include in Fill.cpp and use/guard adjustment of the existing include in FillLightning.cpp. Capture grouped caller identity, numeric angle and fixed_angle bool before callee clipping. No header re-edit/fourth file in passB; FillBase.hpp is read-only type authority. No sibling mutation or extra tool file; Step2 separately owns fixture output paths.
- Dispatch: precise canonical CLI/build/identity question, SUMMARY ≤200 words; execution prerequisite probe, FACT available or named missing dependency. Never absorb full upstream build logs.
- Cost: M. Authorities: approved source plan/ADR0066, attribution, coordinate and test-quality docs.
- Canonical refs: the scoped generator/bridge/caller/grounding/clipping functions listed above, delegated only; especially Fill.cpp::Layer::make_fills, not a fabricated callee identity accessor.
- Verification: `python tools/lightning_gate/record.py --help`; delegation returns FACT capture/replay flags exist and missing prerequisites are a nonzero/inconclusive result. Verify recorder patch instruments real data rather than manufactured expectations.
- Falsifying exit: unresolved identity propagation, source acquisition or executable recorder support is a named inconclusive blocker, not completed oracle proof; stop before treating any capture as canonical.

### Step2 — Execute recording and publish portable fixtures

- Task: TASK-576. Objective: execute independent canonical recording for the complete discriminating matrix and freeze portable expected relationships/provenance.
- Precondition: Step1 executable recorder is ready; canonical scalar policy, wall strategy config spelling and source toolchain are recorded from actual source/build.
- Postcondition: five case families, two wall strategies, permutations, translations and changed-parameter companions have independent input/checkpoint/graph/final-fill records. Faithful immutable-snapshot replay first matches separately recorded production output. Direct `final_domain_only` replay uses fresh graph/cache state, identical scalars/configuration/root-point-hash seed policy and the same faithful final expolygons/clipping, without repeating any bridge anchor mutation; hashes before/after prove snapshot immutability. Both graphs must be nonempty and a preclip directed-topology/root-grounding discriminator plus positive domain mismatch must exist; otherwise A is inconclusive. Wrong/faithful records are separately labeled.
- Allowed reads: three recorder files; approved Lightning witness sections; recorder output/provenance only through bounded SUMMARY; no oversized JSON load.
- Allowed edits (3): `crates/slicer-wasm-host/tests/fixtures/lightning_gate/inputs.json`; `crates/slicer-wasm-host/tests/fixtures/lightning_gate/canonical.json`; `crates/slicer-wasm-host/tests/fixtures/lightning_gate/manifest.json`.
- Forbidden: kernel/host expectations; production paths; other packets; canonical checkout mutation; source plan/backlog.
- Blast radius: the same three fixture files use design's GateInputsFile/GateOracleFile schemas, explicit a_source/b_input/reference separation and distinct source/raw-config/earlier-provenance/B/reference hashes. Raw A inputs cannot contain expected operation/boundary maps or resolved policy; those remain reference/B branches. No extra source/poison fixture file is implicit. Derive revision/provenance at execution; extra partitions need bounded approval.
- Dispatch: execute capture/replay in disposable canonical source, SUMMARY ≤200 words giving each case identity, actual phase exits and hash match; delegate canonical function fact-checks.
- Cost: M. Authorities: source-plan gate witnesses, docs08/attribution/22.
- Canonical refs: PrintObject.cpp::bridge_over_infill original/temporary/restored checkpoints; Generator.cpp::generateTrees/generateInitialInternalOverhangs domains/graph; Fill.cpp::Layer::make_fills final caller identity/angles propagated to FillLightning.cpp::Filler::_fill_surface_single clipped lines.
- Verification: `python tools/lightning_gate/record.py --capture --output crates/slicer-wasm-host/tests/fixtures/lightning_gate`; then `python tools/lightning_gate/record.py --replay --manifest crates/slicer-wasm-host/tests/fixtures/lightning_gate/manifest.json --output target/lightning-canonical-replay`; return FACT phase exits and independent normalized hash match.
- Falsifying exit: missing case/strategy, unchanged negative-control topology, unreplayable provenance or self-captured expected data prevents A pass; never fill missing data with host output.

### Step3 — Register oracle/evidence tests before candidate code

- Task: TASK-576. Objective: author real provenance and fail-closed tests, parser/comparator support and exact aggregate registration.
- Precondition: portable fixtures from Step2 exist or an explicit inconclusive recording failure is retained; missing fixtures must fail loudly in ordinary verification.
- Postcondition: author registered provenance/fail-closed tests plus separate input/oracle envelopes and real source-only project_gate_a/extract_gate_a_source declarations. They select only a_source and do not require B/reference deserialization before A. Missing normal evidence stays loud in separate validators. This source remains dependency-blocked until Step8; no early compiled/tested claim or test-local expected planner.
- Allowed reads: three fixture files, bounded current host Cargo target declarations, `tests/integration/main.rs`, current common cache interface; docs21/22; design's evidence shape.
- Allowed edits (3): `crates/slicer-wasm-host/tests/integration/main.rs`; `crates/slicer-wasm-host/tests/integration/lightning_gate_tdd.rs`; `crates/slicer-wasm-host/tests/integration/lightning_gate_support.rs`.
- Forbidden: Cargo/kernel/guest/production edits; other packets; source-plan body/backlog; external checkout dependency in ordinary tests.
- Blast radius: add `mod lightning_gate_tdd;` to the aggregator; include sibling support from the test file via `#[path = "lightning_gate_support.rs"] mod support;`, not a nested default module path or independent Cargo target. New evidence types have no prior literal sites/version assertions; existing fixture records follow FRU/explicit waiver discipline.
- Dispatch: source/manifest registration check only, FACT target integration/no required feature/new `mod` and support-path declarations; no Cargo execution before Step8 dependencies exist.
- Cost: S. Authorities: docs03/05 Test Support/21/22 and repository Test Discipline.
- Canonical refs: fixture provenance citations only; no new source inspection required.
- Verification: bounded worker FACT declaration inventory contains `canonical_fixture_provenance` and `missing_failed_inconclusive_blocks_migration`, aggregate `mod lightning_gate_tdd` and explicit sibling support inclusion; this is source-order authoring validation only, never test acceptance. Step8 runs both exact commands after dependencies exist.
- Falsifying exit: omitted registration, silent fixture skip, manufactured expectations or fabricated pass records blocks handoff; compile/test acceptance remains unearned until Step8, and missing fixtures stay red.

### Step4 — Isolated module-owned typed kernel boundary

- Task: TASK-576. Objective: create the standalone owned kernel crate/model with explicit input, parameter and graph identities.
- Precondition: fixture scalar-source/order/coordinate semantics fixed independently; no dependency on preparation framework types.
- Postcondition: model.rs/lib.rs define/reexport restricted GateASourceInput, EarlierLayerInput/EarlierRegionInput/EarlierSurface and closed phase/producer/kind enums, RawObjectConfig/RawRegionConfig/RawWidthSetting, GateAConstruction/DerivedRegionPolicy; broad KernelInput/RegionInput remain B-only. Preserve all source/boundary/operation/graph/error DTOs. build_planning_domain accepts only the restricted input and returns complete construction/derived policy/scalars; no broad-A overload. Strict recursive serde rejects nested reference smuggling; no extra model file.
- Allowed reads: bounded root dependency declarations and IR Point2/ExPolygon/PaintValue definitions; fixtures/model contract; docs08/attribution/21/22.
- Allowed edits (3): `modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml`; `modules/core-modules/lightning-infill/feasibility-kernel/src/lib.rs`; `modules/core-modules/lightning-infill/feasibility-kernel/src/model.rs`.
- Forbidden: production module lib/manifest, root Cargo manifest, host driver, SDK/WIT and other packets. No build here that creates an unbudgeted lockfile.
- Blast radius: model.rs owns all new DTO/KernelError/EvidenceLocation definitions, required-presence Option deserialization and strict nested serde coverage; lib.rs owns reexports/declarations. No extra serializer file, public IR/schema/version edit or existing-literal fallout. Declared next-step modules remain an incomplete intermediate state, not a compile/portability claim; Step9 owns executable full-serde/roundtrip/negative assertions.
- Dispatch: bounded dependency audit for guest portability, SUMMARY ≤200 words; no broad trait tracing in controller.
- Cost: S. Authorities: design/approved gate, docs08/attribution/21/22.
- Canonical refs: delegated constructor/scalar formula audit from Generator.cpp if needed, file+function.
- Verification: worker FACT restricted A input whitelist has only earlier geometry/raw configuration/source-origin metadata and cannot contain B/reference/later-checkpoint/resolved-policy/expected-map data; pure API has no external references/callbacks. B retains canonical domains/scalars but no graph/output expectations. No host driver/host-algos dependency. Authoring inventory is not executed isolation proof; Step9's dynamic poison witness is mandatory.
- Falsifying exit: a required prepared-framework type, gated host dependency or existing public schema change stops for scope/design approval.

### Step5 — Candidate domain and portable polygon primitives

- Task: TASK-576. Objective: implement the concrete A reconstruction and portable math needed by owned B, preserving holes and temporary anchors.
- Precondition: Step4 model exists; Step2's canonical input/expected-domain relationships cannot be regenerated from candidate code.
- Postcondition: pure build_planning_domain(&GateASourceInput) returns GateAConstruction: locally computed domains/atomic boundary lineage plus derived shell/solid/bridge/fixed-direction policy and constructor scalars/source order from earlier surfaces/raw config/transform/schedule. No resolved oracle input, broad B argument, fixture lookup or cached reference fallback. Missing source/width formula/parameter/exposure produces the named fail/inconclusive error, never copied reference values or default identity/angle.
- Allowed reads: bounded fixtures/owned model; current unconditional polygon math/closest-point helper signatures for reference; docs02 SliceIR fill invariants/08/22; delegated canonical anchor/domain functions.
- Allowed edits (3): `modules/core-modules/lightning-infill/feasibility-kernel/src/domain.rs`; `modules/core-modules/lightning-infill/feasibility-kernel/src/polygon.rs`; `modules/core-modules/lightning-infill/feasibility-kernel/src/distance_field.rs`.
- Forbidden: current host helper/driver modifications, production paint fixes, SDK/WIT exposure changes, other packets and source-plan body.
- Blast radius: new local records/helpers only; every translated file gets the standard header with actual original path. No public field/version fallout.
- Dispatch: exact canonical temporary-anchor/void policy question, SUMMARY ≤200 words; bounded current host/guest consumed-field availability table, LOCATIONS ≤20 entries.
- Cost: M. Authorities: source-plan Lightning geometry, docs02/08/attribution/22.
- Canonical refs: `PrintObject::bridge_over_infill`, `Generator::generateInitialInternalOverhangs`, delegated.
- Verification: worker FACT each expected domain/policy comes only from separate reference loading and candidate construction receives only project_gate_a's restricted a_source result, never inputs.json's b_input. Compile/behavior verification follows Step6/9; this source-only inventory is not the dynamic isolation proof.
- Falsifying exit: needing final Layer fill output as a PrePass input or dropping shell/bridge/holes invalidates the proposed strategy and records A failure, without changing production timing.

### Step6 — Portable topology/grounding kernel

- Task: TASK-576. Objective: implement actual top-down graph planning and non-canned output using canonical-domain input.
- Precondition: typed model/local polygon/distance-field code exists; graph expectations and tolerances are independent recorded data.
- Postcondition: `plan_lightning` computes all directed nodes/edge occurrences, parent/root IDs and grounding witnesses with object-layer pooling; canonical-policy sampling fills `sampled_segments`, then `clip_lightning` emits full-region-keyed final segments. B can consume canonical planning inputs even if A fails. No region/island positional pairing or output-only geometry welding is borrowed from the host driver.
- Allowed reads: owned model/domain/polygon/distance files; bounded current lightning tree/layer/generator source for audit (never a proof); delegated canonical tree/root functions; docs08/attribution/22.
- Allowed edits (3): `modules/core-modules/lightning-infill/feasibility-kernel/src/tree.rs`; `modules/core-modules/lightning-infill/feasibility-kernel/src/planner.rs`; `modules/core-modules/lightning-infill/feasibility-kernel/Cargo.lock` (Cargo-generated only).
- Forbidden: lib/model changes unless a separately bounded corrective step is declared; all host/production/framework paths and expectation fixture alterations to fit output.
- Blast radius: no existing structs/constants changed; standard attribution on translations. The Step4 lib already declares/reexports these planned modules/functions.
- Dispatch: compile new standalone crate and inspect target dependency features, FACT compile result and no gated host dependency; errors ≤20 lines.
- Cost: M. Authorities: source-plan canonical gate/ADR0066, docs08/attribution/22.
- Canonical refs: `Generator::generateTrees` and subordinate grounded tree functions located by delegation, file+function only.
- Verification: `cargo check --manifest-path modules/core-modules/lightning-infill/feasibility-kernel/Cargo.toml --all-targets`; not portability evidence, actual adapters still required. Capture any failed canonical comparison in Step9 without weakening the independent expected graph.
- Falsifying exit: host generator calls, empty placeholder output, changed expectations/tolerances to hide differences or index-based island identity is failure.

### Step7 — Existing-stage guest adapter, no new WIT

- Task: TASK-576. Objective: compile a discovered test guest wrapping exactly the owned kernel through current native/WASM macro adapters.
- Precondition: complete owned kernel; existing `slicer:layer-infill` typed world and SDK signature are authoritative. Arbitrary custom WIT is rejected by artifact freshness and is not this design.
- Postcondition: guest emits the existing infill export/native entry; the same body computes graph/witnesses and emits planned tags1/2 plus full tag3 provenance, or clipped two-point segments for the validated `probe_final_dispatch` request. All final dispatches are later exercised by Step9 with canonical caller/contributor correspondence. Codec/model/policy fields match Step4 and explicit origins use WIT-supported variants. Custom inputs fail before dispatch. No SDK/WIT changes or separate codec file.
- Allowed reads: `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/Cargo.toml` and `SdkLayerInfillModule::run_infill` (`crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs`); owned kernel exports; current native entry/LayerModule/run_infill and canonical infill WIT symbol windows; bounded `discover_guests`/freshness sections; docs03/05/08/22.
- Allowed edits (3): `crates/slicer-wasm-host/test-guests/lightning-portability-guest/Cargo.toml`; `crates/slicer-wasm-host/test-guests/lightning-portability-guest/src/lib.rs`; `crates/slicer-wasm-host/test-guests/lightning-portability-guest/Cargo.lock` (Cargo-generated only).
- Forbidden: canonical WIT, SDK/macros, production module guest/lib and custom preparation export; other guest lockfiles without a separately scoped convergence repair.
- Blast radius: this step's src/lib.rs owns exhaustive production-style existing Point3WithWidth/ExtrusionPath3D adapter literals and checks metadata-integer representability/Z/positive width. No public changes/version assertions. The manifest/source/generated guest lock are the only three edits; no separate codec source or expected fixture enters guest linking.
- Dispatch: guest build/freshness execution, FACT artifact stage identity plus exact exit; scope new guest, do not dump all WASM/WIT output.
- Cost: S. Authorities: docs03/05/08/22, AGENTS guest freshness and standard attribution.
- Canonical refs: already recorded parameter/graph references, no new direct source read.
- Verification: `cargo xtask build-guests`; `cargo xtask build-guests --check` (exit0 only is fresh; exit1 stale/lock issue; exit3 infrastructure stop). This produces shared-target and staged guest artifacts; it is compile/instantiation readiness only, not B success.
- Falsifying exit: missed discovery, incompatible WIT, unusable freshness, host-algos in the owned kernel or wasm32 guest closure, hidden host planner calls, unsupported identity degradation or expectation-bearing config prevents B pass. Legitimate SDK nonwasm dependencies are not rejected. Unrelated stale/lock repairs require bounded scope approval, not a widened step.

### Step8 — Native dependency wiring and lock ownership

- Task: TASK-576. Objective: make native guest/kernel available to the actual host test binary with explicitly enabled SDK test support and direct serde/serde_json/sha2 dev dependencies for the fixture/evidence validator.
- Precondition: guest/kernel packages now exist with real native entry; Step3's authored tests/registration intentionally await JSON/hash/SDK dependencies.
- Postcondition: host dev dependency/lock wiring enables serde/serde_json/sha2, explicit SDK test support and native guest/kernel imports. All-target compilation and Step3's two exact test exits are now actually run; failures return to a separately bounded Step3 correction, not unauthorized Step8 source edits. No test-local replacement planner.
- Allowed reads: bounded host Cargo dev dependencies, guest manifest/native export and SDK test feature documentation; no lockfile content load.
- Allowed edits (2): `crates/slicer-wasm-host/Cargo.toml` (dev-dependencies only); root `Cargo.lock` (Cargo-generated only).
- Forbidden: host production dependencies/functions, guest/kernel source, SDK/WIT/macros and other packets.
- Blast radius: only host Cargo.toml/root Cargo.lock edits; Step3 owns earlier parser/evidence literals, Step7 adapter source literals and Step9 all new driver fixtures. No public field/version assertions. Native SDK/host dependencies legitimately contain host-algos; the owned kernel and wasm32 guest closures cannot, and no native adapter may call the host lightning producer/fallback.
- Dispatch: compile/conditional-dependency audit, FACT same path package/native entry compiled, SDK's nonwasm feature allowed, owned-kernel/wasm32 exclusions and both nonzero exact tests. Do not assert native SDK dependency exclusion.
- Cost: S. Authorities: docs05 native/Test Support, AGENTS all-target gate; design's exact native entry shape.
- Canonical refs: none beyond fixed fixture provenance.
- Verification: `cargo check -p slicer-wasm-host --all-targets`; then `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::canonical_fixture_provenance -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`; archive its log before `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::missing_failed_inconclusive_blocks_migration -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`. Package/fixture verification alone is not B evidence; a compiler/test failure leaves the exit incomplete.
- Falsifying exit: native linking requires a host-owned algorithm adapter or schema change; stop rather than introducing that shortcut.

### Step9 — Actual independent A/B drivers and counterfactuals

- Task: TASK-576. Objective: execute Gate A comparison and real native/WASM Gate B calls, checking canonical domains/graphs/final output and wrong/canned controls.
- Precondition: independent fixtures, complete owned kernel, discovered fresh guest and native dev dependency exist; the eight tests are not allowed to skip missing input.
- Postcondition: all eight tests preserve full tag3/provenance/multigraph/dispatch positives and negatives. AC-N2 additionally uses the actual project_gate_a → extract_gate_a_source → build_planning_domain path on baseline and independently poisoned/removed/replaced B/reference branches with identical permitted-source bytes; compare every constructed domain/catalog/contributor/operation multiset and derived policy/scalar, plus before/after source/config/provenance hashes. Source-only references-absent runs must construct supported cases; poison reference must change comparator verdict only. Actual native/WASM B-domain perturbation must alter B result or reject bad B input without changing A. Whitelist-smuggling controls fail separately. No skipped source population, test-local candidate stand-in or digest-only self-assertion. Keep immutable canonical replay independent; archive diagnostics/logs.
- Allowed reads: fixtures, owned public API, bounded `wit_boundary_tdd::guest_reads_config_value_and_uses_it_in_output`, native request/response/builder `sparse_paths`/`sparse_path_origins`, host collected origin vectors, generated infill drain and current common cache; docs08/21/22. Respect the existing Custom-degradation/None-origin caveats; no production repair is authorized.
- Allowed edits (2): `crates/slicer-wasm-host/tests/integration/lightning_gate_tdd.rs`; `crates/slicer-wasm-host/tests/integration/lightning_gate_support.rs`.
- Forbidden: fixture expectation rebaselines from candidate, production host/paint/module changes, WIT/framework integration, other packets; no fallback to current host producer.
- Blast radius: the same two test files own real extractor/projection integration, in-memory GateFixtureView mutation recipes, restricted source/result/raw-config literal and strict-serde assertions, full output/map/policy comparisons and actual native/WASM B-poison driver exits. Existing tag3/SDK/IR literal ownership and FRU/protected-boundary waivers remain. All poison trials run inside the existing exact AC-N2 function; eight-test count is unchanged, no fourth fixture/helper or extra target. Missing/extra field and old-broad-A call compilation fallout is fixed only in these files and existing authorized kernel model/domain/lib steps; public SDK/IR/schema changes remain forbidden.
- Dispatch: every Cargo command to debugger worker, FACT exit/testcount plus actual gate status; canonical difference audit SUMMARY ≤200 words. Reuse saved logs rather than rerun for truncation.
- Cost: M. Authorities: source-plan gate witnesses, docs03/05/08/21/22 and AGENTS test/freshness discipline.
- Canonical refs: the independent recorded domain/graph/clip functions; further reads delegated.
- Verification: dispatch each AC-specific exact command from `packet.spec.md`, preserving command/test log hashes and process exits. Then `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_gate_tdd:: -- --nocapture >target/lightning-gate-command.log 2>&1; rg -q '^test result: ok\. 8 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`. Xtask owns combined test capture; archive both logs before overwrite. Cargo, tee, preflight and missing/nonzero count failures all fail the command; no truncated-output rerun.
- Isolation exit: AC-N2 must report named, nonzero dynamically executed baseline/reference-removal/reference-replacement/reference-poison/source-only/B-perturbation/source-smuggling populations and compare saved projected-source/config/provenance bytes/hashes before and after **each** actual construction. Full GateAConstruction equality includes every domain/source/contributor/operation occurrence, derived policy/scalar and source selection; one altered output, skipped mutation, unavailable supported source or oracle-dependent projection fails/inconclusive, never a passing isolation exit. A reference-only comparator mismatch must be observed separately without mutating A; B trials must actually call both adapters rather than only edit fixture JSON. Existing exact AC-N2 command/count remains unchanged and all original controls still execute.
- Falsifying exit: zero exact tests, omitted matrix populations, source-only evidence, native/WASM equality-only assertions, cached/echo/empty fallback acceptance or unmeasured wrong-domain discriminator is not a successful gate. Direct scientific mismatch tests remain red; outcome-validator tests cannot erase them.

### Step10 — Truthful evidence and future source-plan outcome recording

- Task: TASK-576. Objective: validate actual evidence and append both outcomes/selected strategy to the source plan, leaving row08 fail-closed.
- Precondition: both experiments were genuinely attempted; collect nonzero tested populations, all commands/exits/hashes and failure/inconclusive reasons, never guessed values. Generic framework acceptance is re-derived separately, not inferred from generated drafts.
- Postcondition: source plan has separate A/B records plus strategy/provenance/scalar-grouping/config matrix/future migration requirements. The existing TASK-576 item contains subordinate `TASK-576 Gate A outcome:` and `TASK-576 Gate B outcome:` lines with actual evidence/status; `[x]` requires both verified passes and successful packet acceptance, while failed/inconclusive/incomplete cases retain truthful `[~]`/`[ ]` and named blockers. Neither task completion nor generic generation status automatically unlocks row08.
- Allowed reads: narrowly captured `target/lightning-gate/outcome.json` and per-run evidence, this packet's ACs, source-plan recording/queue windows and delegated TASK-576 row; generic rows01–06 implemented-status/acceptance evidence only via bounded dispatch.
- Allowed edits (3): `crates/slicer-wasm-host/tests/integration/lightning_gate_support.rs` (final evidence writer/validation); `docs/specs/layer-module-preparation-plan.md` (append outcome section only); `docs/07_implementation_status.md` (TASK-576 bounded truthful status only).
- Forbidden: source-plan approved design body and unrelated queue/backlog entries, ADRs/deviations, other packets, production source; no row08 authoring or automatic migration activation.
- Blast radius: evidence-local records only. Re-derive recorder revision and framework evidence at execution, reserve no ledger IDs. Any new evidence-field literals are confined to this file and use appropriate defaults/waivers.
- Dispatch: exact source-plan/backlog edits to worker, SUMMARY ≤200 words; independent evidence/eligibility review, FACT both statuses with named blockers. A future scope-changing correction requires approval rather than rewriting approved decisions.
- Cost: S. Authorities: source-plan row07 recording/row08 entry rules and Q13, ADR0066, docs22/AGENTS ledger discipline.
- Canonical refs: retain exact recorded file/function and derived revision provenance; no source-only pass claim.
- Verification: AC-5 and AC-N3 exact commands from `packet.spec.md`, each directory-safe, bounded and fail-closed. Placement-only checks: `rg -q '^## Lightning geometry/portability gate outcomes$' docs/specs/layer-module-preparation-plan.md`; `rg -q '^- \[[ x~]\] \*\*TASK-576\*\*' docs/07_implementation_status.md`; `rg -q '^  - TASK-576 Gate A outcome: (pass|fail|inconclusive)\b' docs/07_implementation_status.md`; `rg -q '^  - TASK-576 Gate B outcome: (pass|fail|inconclusive)\b' docs/07_implementation_status.md`. Validate task block semantics through AC-5/AC-N3; bounded diff proves approved body unchanged, not parity correctness.
- Falsifying exit: either record lacks executed evidence/complete coverage, failed results are relabeled pass, or `migration_eligible` is true without both verified passes and generic implemented acceptance. Preserve explicit fail/inconclusive report and migration block.

## Per-Step Budget Roll-Up

| Step | Cost | Reason |
| --- | --- | --- |
| 1 | M | Independent recorder checkpoints/build recipe |
| 2 | M | Canonical matrix and negative discriminator |
| 3 | S | Registered fixture/evidence tests |
| 4 | S | Local typed boundary |
| 5 | M | Domain/portable polygon reconstruction |
| 6 | M | Top-down graph/grounding kernel |
| 7 | S | Current-stage guest adapter |
| 8 | S | Native manifest/lock wiring |
| 9 | M | Real adapters and independent comparisons |
| 10 | S | Truthful outcome/entry record |

Aggregate M through bounded sequential workers; no L step. Stop/redelegate before any step exceeds its budget; this table does not authorize an unbounded all-code read.

## Packet Completion Gate and Acceptance Ceremony

- Re-dispatch all exact AC commands and the three packet-level gate commands; no workspace test suite is required. All check/clippy gates use `--all-targets`; targeted tests retain their explicit binary and exact nonzero filter.
- Also execute `cargo xtask check-literals` and `cargo xtask check-test-quality --report`, fixing or justified-waiving touched findings, never removing assertions to make a pass.
- Validate both evidence records independently and append their real outcomes. Missing/incomplete experimentation keeps closure blocked. A completed failed feasibility investigation is reported as **failed feasibility with migration blocked**, not successful verified outcomes; direct scientific assertions may remain red and the packet must not claim successful implemented acceptance.
- Only verified A/B passes plus independently implemented generic framework acceptance and their source-plan records permit later row08 generation. A preflight pass or this draft's existence is not evidence.
- Capture final risks and exact narrow failures; preserve shared work and do not edit other packets. No automatic activation, implementation or acceptance has occurred during this authoring session.
