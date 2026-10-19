# Implementation Plan: lightning-migration

## Execution Rules

**No step in this file is authorization to implement now.** All production/test edits are blocked by packet.spec's entry conjunction and design B1–B5. This is a conditional bounded plan, not a claim that scientific strategy/codec/retirement design is resolved. The retirement production sequence intentionally cannot be executed until Step2 has replaced its blocked partition with complete atomic contracts.

Each step maps to TASK-577, allows at most3 edited files and S/M context, and uses bounded delegated fact checks. TDD RED is recorded as failure evidence, not accepted output. Tests never fabricate a Ready plan or expose canonical references to A construction. New translated files receive required attribution. Test literals use FRU/shared fixture bases or specific exhaustive waivers; production literals stay exhaustive. Field/version removal must own every literal/constructor/exhaustive-match/old-value assertion in its approved originating step, not a surprise follow-up. If that cannot meet the file cap, stop and approve a split before activation.

Archive previous combined/command logs before overwriting, then successfully remove `target/test-output.log` before every proposed xtask test invocation under `set -euo pipefail`. This prevents tolerated summary log-write failure in `xtask/src/test.rs::test_command` from reusing stale passing counts: absent fresh output fails the count guard. Preserve this reset when delegating the AC commands or final aggregate; it is not permission to execute them during authoring. The plain Cargo decoder command retains its combined-output tee and pipefail guard.

## Step 1 — Read-only entry evidence and design resolution

- Task IDs: TASK-577. Objective: independently establish the real entry conjunction and resolve scientific/source/representation/evidence questions without production work.
- Precondition: row07 and generic rows01–06 have completed their own implementation/execution ceremonies. A generated draft or green failure-reporting test is insufficient. Otherwise STOP; this draft remains blocked.
- Postcondition: independently verified archived actual outcomes, source-plan recorded strategy/config/grouping/provenance/source availability, exact source/codec/caller/diagnostic decisions and remaining compatibility inventory are available to the authoring/review owner. Any unavailable evidence keeps B1–B5 open; no invented pass or default source.
- Allowed reads: source-plan Packet Queue/Lightning planning geometry/outcome appendix, ADR-0066 consequences; predecessor acceptance criteria/exports; bounded actual artifacts/command exits/log hashes through evidence workers; symbol-centered whole-print DTO/source producer/kernel windows. Delegate files over300 lines and canonical sources.
- Allowed edits: **none**. No source-plan/backlog/packet status or code update.
- Out of bounds: production/tests/build execution by this worker, all other packet edits, entire target/lock/generated/vendor trees, canonical source direct reads/edits.
- Dispatch: precise per-gate evidence verification question, scope accepted rows01–07 and their actual archived evidence; return FACT at most5 lines per group outside thinking. Source-field/caller/grouping mapping scope accepted producers/kernel plus canonical file/functions; SUMMARY at most200 words per bounded group.
- Context: M. Authorities: source-plan queue/geometry/projection requirements, ADR-0066, docs00 map; docs01–05 bounded contracts and docs08/21/22 discipline. Orca refs: canonical functions enumerated in requirements, delegated only.
- Narrow decisive verification: re-dispatch row07's exact `canonical_domain_gate_decision`, `canonical_coverage_and_grounding`, `portable_kernel_native_wasm`, `wrong_final_domain_is_rejected`, `portable_kernel_rejects_canned_echo_fallback` and the generic framework's required acceptance sub-gates **using their own guarded commands**; archive actual exits and compare source-plan recorded outcomes independently. This step cannot substitute row07's outcome-reporting or synthetic rejection tests for these commands.
- Exit: every required scientific direct assertion and generic sub-gate actually passed, source/provenance/strategy mapping is concrete and independent, and reviewer names any still-open compatibility/partition decision. Any failed/inconclusive/unexecuted/missing coverage aborts the migration sequence, even if a status writer test is green.

## Step 2 — Resolve and re-author the blocked atomic retirement contract

- Task IDs: TASK-577. Objective: make the conditional production plan complete only after independent entry evidence and actual blast-radius decisions exist.
- Precondition: Step1 evidence exists; B1/B2/B3/B5 answers are concrete, and a bounded closure survey enumerates all B4 field/type/version/registry/test/docs fallout. Scope-expanding source/config/WIT/arena decisions require explicit user approval first.
- Postcondition: design B1–B5 contain evidence-backed resolutions; the exact live deletion/compiler-witness/compatibility surfaces and every affected literal/assertion are allocated to bounded steps, each with its own narrow decisive command and no shim. Source bridge/config/piece/error/id shapes are no longer unresolved. No production edit yet.
- Allowed reads: this packet, bounded current retirement symbols/literal locations returned by closure workers, accepted predecessor export shapes, source-plan recorded evidence, normative compatibility/units/test sections. No wholesale cross-crate browsing.
- Allowed edits: `docs/spec_packets/layer-module-preparation_08_lightning-migration/design.md`; `docs/spec_packets/layer-module-preparation_08_lightning-migration/implementation-plan.md`. Contract/criteria crosswalk revision is a separate at-most3-file authoring slice for packet.spec/requirements/task-map; it must finish before preflight, not become a production step with hidden extra edits.
- Out of bounds: all production/tests, other packet contracts, source plan/backlog, canonical checkout, generated/lock/vendor trees and automatic status activation.
- Dispatch: exact symbol/literal/old-version assertion inventory per ownership group, full workspace paths, LOCATIONS at most20 entries; compatibility decision/canonical caller evidence, SUMMARY at most200 words. Independently review after re-authoring; the author does not self-award preflight.
- Context: M. Authorities: ADR-0066, docs02/03/04 version/resource/caller ownership; docs21 literal fixture rules; docs22 independent compile/behavior witnesses; skill atomic edit cap. Orca refs: delegated group_fills/Layer::make_fills/Generator constructor functions as needed.
- Narrow decisive verification: independent `/spec-review docs/spec_packets/layer-module-preparation_08_lightning-migration --preflight`, with symbol/literal/registration/compatibility survey evidence and B1–B5 resolutions. No Cargo execution by the authoring worker.
- Exit: independent PREFLIGHT PASS and explicit activation approval only after the complete gate conjunction. An unallocated field/constant/literal deletion, unspecified codec/error/source or guessed WIT version is PREFLIGHT BLOCKED, not permission to start Step3. This step has no settled production retirement edit list yet; design's known retirement inventory is mandatory work, not optional follow-up.

## Step 3 — Register the production-driving independent test harness

- Task IDs: TASK-577. Objective: create the NET-NEW planned e2e tests/fixture bridge and genuine real-evidence validator after activation, without implementing a host algorithm or injecting plans.
- Precondition: Step2 completed, actual source/codec/compatibility/test dependency decisions accepted, no blocker remains. Native and WASM artifact registration dependencies are enumerated before editing Cargo.
- Postcondition: `lightning_migration_tdd` is registered; real source/reference fixture loading, actual paired module registration, independent exact comparators and corrupted/missing evidence controls compile. Required populations/fixture availability fail loudly. Production equivalence may be RED while migration is absent; that RED is not acceptance.
- Allowed reads: `crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs::run_slice_against_wedge_returns_nonempty_gcode`, existing run_slice/extension fixtures and common helper windows; accepted kernel/source/reference types; source-plan actual evidence; docs21/22 test checklist.
- Allowed edits: NET-NEW `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`; `crates/slicer-runtime/tests/e2e/main.rs`; `crates/slicer-runtime/Cargo.toml` only for the Step2-inventoried dev-dependencies. Do not widen public runtime structs.
- Out of bounds: module production, host planner/fallback, oracle recorder/fixtures, other packet contracts, generic framework implementation, canonical checkout edits and source-plan/backlog edits.
- Dispatch: does every exact registered test drive actual production selection/prepare/consume and compare independent references, with actual populations? Scope new e2e file and real driver windows; SUMMARY at most200 words. Execution worker returns FACT and archives complete logs.
- Context: M. Authorities: accepted rows05–07 exports, docs04/05 real dispatch, docs21/22 literal/test quality. Orca refs: no additional source read; portable recorded provenance is mandatory.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::activation_evidence_requires_executed_gates -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`.
- Exit: exactly one test executes both synthetic negative trials and a separately required actual-evidence positive trial; changing/removing any real gate/source/framework artifact fails that positive trial. This only checks the validator and its real inputs, never substitutes for Step1 scientific execution.

## Step 4 — Module-private immutable piece contract and decoder

- Task IDs: TASK-577. Objective: implement only the evidence-selected B3 layout/read/identity/error contract, with bounded fresh facade reads and no host byte interpretation.
- Precondition: Step3 harness exists; exact piece names/format/version/partition/bounds/error literals and source/caller mapping were settled in Step2. No codec may be invented during this step.
- Postcondition: module-local publication/decoding helpers preserve final plan/source/caller identities; native fresh facades and guest-compatible helper code enforce the selected bounds/errors. Empty successful plans and absent/malformed pieces remain distinct.
- Allowed reads: accepted kernel final plan structures, row03 piece/read/error API, B3 resolution, `modules/core-modules/lightning-infill/src/lib.rs` module/type windows and existing module test fixtures.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/private_plan.rs`; `modules/core-modules/lightning-infill/src/lib.rs` helper registration only; `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` add NET-NEW top-level `private_piece_decode_and_identity` with literal independently authored expected geometry/identity and selected malformed/overflow cases.
- Out of bounds: generic SDK/WIT/schema/arena/runtime surfaces, other module/kernel algorithms, source plan/backlog/other packet files, canonical records and host fallback.
- Dispatch: do bounded immutable reads/decoder errors preserve every selected identity and reject exact malformed inputs without host lookup? Scope module helper and production read facades; SUMMARY at most200 words. Execute narrow test through worker, not controller.
- Context: M. Authorities: accepted B3 contract, ADR-0066, row03 transport, docs08 units, docs21/22 fixtures/oracle/compile discipline. Orca refs: no new translation is inferred; any translated helper uses actual attribution.
- Narrow verification: `set -euo pipefail; mkdir -p target; cargo test -p lightning-infill --test lightning_infill_tdd private_piece_decode_and_identity -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`.
- Exit: literal expected identity/geometry and each bounds/malformed case execute; source-only helpers contain no canonical reference or host producer path. This local codec proof is not scientific production equivalence.

## Step 5 — Prepare from audited earlier typed inputs and consume in fresh calls

- Task IDs: TASK-577. Objective: implement the exact accepted source bridge and same-artifact planning/consumption path, preserving object-layer pooling and original final caller attribution.
- Precondition: Steps1–4 passed their actual exits; Step2 named exact DTO/config/grant/formula fields and any separately approved preservation work is already accepted. Accepted owned kernel package path/name is verified, not guessed.
- Postcondition: actual `LayerPreparation::prepare_print` calls restricted A construction from declared earlier typed data, then owned planning; ordinary `LightningInfill::run_infill` decodes its current owner pieces and clips/emits against current caller geometry/config. No B/reference input or committed tree generates the plan. `#[slicer_module(preparation)]` emits paired native/WASM exports.
- Allowed reads: accepted preparation typed input/macro/output/private-plan APIs; kernel `build_planning_domain`/`plan_lightning`/`clip_lightning` windows; B2 resolved source/config/first-region/formula/grouping/caller mapping; current Layer output APIs and shared selection.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/preparation.rs`; `modules/core-modules/lightning-infill/src/lib.rs`; `modules/core-modules/lightning-infill/Cargo.toml` for the verified owned-kernel dependency only. No public struct/version change or host algorithm adapter.
- Out of bounds: host producer/legacy type retirement (requires the new B4 atomic sequence), generic framework/arena/WIT input expansion, reference fixtures/recorder, other packet/source-plan/backlog and canonical checkout edits.
- Dispatch: does real typed preparation source closure exclude resolved canonical/B fields and preserve every full target/config/source contributor? Scope module source bridge plus accepted typed DTO producer windows; SUMMARY at most200 words. Real native/WASM scientific execution returns FACT with archived logs.
- Context: M. Authorities: B1/B2 resolved evidence, source plan late timing/pooling, ADR-0066, rows01/03/04/05/07, docs08/21/22. Orca refs: delegated Generator constructor/generateTrees and Layer::make_fills/group_fills for exact verified sourcing/attribution.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::production_native_wasm_canonical -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`.
- Exit: actual native/WASM preparation and fresh consumers independently match the accepted canonical matrix with nonempty populations; source changes reach actual outputs. If manifest opt-in/registration is required before this command can execute, Step2 must reorder/partition Step6's manifest activation before this step, not quietly broaden the edit cap. No legacy coexistence state is accepted or released.

## Step 6 — Publish useful typed diagnostics and exact module declarations

- Task IDs: TASK-577. Objective: expose final-plan geometry only through row06, with verified read/config declarations and neutral atomic publication.
- Precondition: Step5 production helpers and Step2's exact declaration/config list exist. If module capability declaration is required for Step5 verification, its earlier bounded declaration step must already have been authored/approved in Step2; this draft does not imply impossible executable ordering is settled.
- Postcondition: actual manifest declares accepted `slicer:layer-preparation/prepare@1.0.0`, evidence-selected `input_reads` and `views = ["planning_geometry"]`; prepare publishes requested final-plan `lightning-plan` typed geometry atomically. B3 resolved id/label/class mapping is deterministic and no-request avoids diagnostics.
- Allowed reads: module final private-plan/helper windows, row06 projection validation/capture APIs, accepted module manifest/config filtering, B2/B3 resolved declaration/id mapping; existing binding test setup.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/projection.rs`; `modules/core-modules/lightning-infill/src/lib.rs`; `modules/core-modules/lightning-infill/lightning-infill.toml`. Binding-test fallout is explicitly allocated to a separate at-most3-file step by Step2 if changed compiled exports require it; it cannot be postponed as compiler-discovered work.
- Out of bounds: generic CLI/renderer/projection schema changes, host decoders/opaque serialization, public runtime capture fields, other packet/source-plan/backlog edits and canonical source edits.
- Dispatch: does scene geometry derive from the same final module plan and does capture on/off preserve semantic plan/output? Scope module projection and actual runtime capture; SUMMARY at most200 words. Exact future test execution returns FACT outside thinking.
- Context: M. Authorities: row06 typed publication/visual contracts, source plan projection requirements, ADR-0066, docs08/19/21/22. Orca refs: independent recorded graph/source references, no new broad canonical read.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::typed_projection_is_atomic_and_neutral -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`.
- Exit: exact typed primitives/identity and empty/missing/invalid cases are asserted, metadata-only and mixed-tap production closures are truthful, and on/off semantic output independently matches. No private bytes appear in serialized diagnostics or host APIs.

## Step 7 — Direct retirement sequence: BLOCKED pending B4 partition

- Task IDs: TASK-577. Objective: delete every identified live legacy producer/IR/accessor/slot/catalog/glue/test/doc surface without a shim, preserving unrelated obligations.
- Precondition: **B4 resolves and Step2 replaces this section with complete individual atomic steps.** Design's concrete closure inventory includes multiple public literal/macro/WIT/test surfaces and cannot honestly be represented as a settled at-most3-file production step now.
- Postcondition of this read-only planning step: approved deletion/version/literal/compiler-witness/doc partition is complete; after those future steps, no live host production/accessor/IR contract remains and all affected guests/type/test targets compile. Historical/example mirror treatment is explicit, not silent generated-source edits.
- Allowed reads: only bounded paths/symbols in design's retirement inventory and returned live catalog closure; affected literal/assertion registrations and normative sections. All cross-crate tracing delegated.
- Allowed edits in this unresolved step: **none**. The future retirement steps must each name at most3 exact edits; no worker may substitute the entire design inventory as an edit list.
- Out of bounds: every production/test/doc edit until Step2 completes the partition; canonical source, generated/lock/vendor/target dumps and other packet contracts.
- Dispatch: enumerate full field/version literal/exhaustive-match/assertion fallout and supported package binding removal treatment, LOCATIONS at most20 entries per group; reviewer verifies originating step owns every site. Missing enumerated site blocks activation; no compiler-led cleanup exception.
- Context: M per partition; any L partition requires approved split. Authorities: ADR-0066 replacing ADR-0029 ownership, docs01–05 compatibility/contracts, docs21/22 literal/compile-witness quality; Orca refs: no additional source needed for legacy deletion.
- Narrow future decisive verification after the approved deletion sequence: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::legacy_contract_is_retired -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`, plus `cargo check --workspace --all-targets` for the whole coordinated type/binding removal. Neither is runnable proof of a completed deletion today.
- Exit: independent reviewer accepts the complete bounded partition before any production edit; final deletion exit additionally requires actual removed-symbol compiler diagnostics, real scheduler/dispatch absence and canonical production behavior. No static source grep alone proves behavior or successful feasibility.

## Step 8 — Closure verification and truthful completion

- Task IDs: TASK-577. Objective: verify all migration/retirement/projection/negative obligations after the complete approved production sequence, then report actual acceptance.
- Precondition: Step2 has eliminated unresolved step ordering and retirement gaps; all implemented atomic steps passed. Changed guests are fresh, including staged in-tree artifacts; synthetic negatives and local decoder proof have not been mistaken for scientific evidence.
- Postcondition: archived per-AC exact commands/exits/logs and independent comparison artifacts support completion, or the packet remains incomplete with direct red assertions. No claim of successful feasibility is manufactured from reporting/cleanup success.
- Allowed reads: this packet's finalized ACs, accepted source-plan/gate evidence and bounded actual test logs/artifacts; live doc headings/contract symbols from design through workers.
- Allowed edits: **none by verification worker**. Packet/backlog/source-plan completion recording is a separately approved at-most3-file orchestration slice after verified acceptance; re-derive mutable statuses/evidence at use.
- Out of bounds: weakening tests/canonical implementation, changing fixtures to candidate output, unrelated work and automatic activation/completion without evidence.
- Dispatch: each AC's own exact command and relevant compile/lint/literal/quality/freshness gate through execution worker; FACT actual pass/fail and bounded assertion on failure, full combined test output to `target/test-output.log` and archived before overwrite.
- Context: M. Authorities: packet.spec ACs, requirements verification matrix, docs21/22 quality, ADR-0066 gates; Orca refs: portable independently recorded canonical provenance.
- Narrow decisive verification: each of the nine AC commands executes exactly its planned test; `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report` with touched findings fixed/justified. No default workspace suite or new benchmark is authorized.
- Exit: every final AC/direct scientific obligation/negative/compiler/doc gate passes and recorded evidence still matches source/config/provenance/artifact identity; otherwise keep status incomplete. Independent final review, not this author self-review, decides closure.

## Budget and Completion

| Step | Context | State |
| --- | --- | --- |
| 1 | M | Blocked read-only evidence resolution |
| 2 | M | Required contract re-authoring and independent preflight |
| 3 | M | Conditional harness; max3 edits |
| 4 | M | Conditional module-only decoder; max3 edits |
| 5 | M | Conditional source bridge/consumer; max3 edits; ordering must resolve |
| 6 | M | Conditional projection/declarations; max3 edits; binding fallout must allocate |
| 7 | M per bounded partition | Production retirement sequence not yet authorized or complete |
| 8 | M | Verification-only closure |

Aggregate work is conducted as bounded M-context slices, not a single broad read/edit session. These costs do not waive the requirement to split any indivisible L blast radius. Packet completion requires every re-authored atomic exit, all ACs and independent closure; source-plan/backlog updates require separate authorized orchestration. This draft must not change status to active/implemented while a conditional ordering/retirement/codec/evidence blocker remains.
