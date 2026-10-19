# Requirements: lightning-geometry-portability-gate

## Packet Metadata

- Task: `TASK-576`; canonical backlog: `docs/07_implementation_status.md` (delegated bounded lookup confirms this feasibility scope, no prerequisites and the TASK-577 blocking relationship; re-derive live task status at execution).
- Prefix/number/slug: `layer-module-preparation` / `07` / `lightning-geometry-portability-gate`; status: **draft**; aggregate context cost: M.
- The approved eight-row queue is the standing authoring approval. This packet has dependency `-`, not a dependency on unimplemented framework contracts.

## Problem Statement

Current lightning generation is a host algorithm, not a portable module-owned proof. Its region grouping and polygon-index pairing differ from canonical object-layer pooling. Existing native/WASM consumer parity injects `LightningTreeIR`; same-code determinism proves repeatability only. Neither is evidence for planning-domain fidelity, canonical grounding or a portable generator. Canonical source inspection establishes questions and recording locations, not executed oracle evidence.

## In Scope

- **Gate A:** execute a reproducible independent canonical recorder; check in portable input, expected-domain/tree/clipped-output and provenance fixtures; compare a concrete late-PrePass domain strategy against them, with deliberately wrong final-domain planning and meaningful pre-clip grounding/topology discriminators. Implement a NET-NEW direct canonical fixture hook in the opt-in recorder patch: immutable snapshot replay, fresh graph/cache state, no repeated bridge mutation, identical configuration/root-derived seed policy and faithful final clipping. Verify faithful replay against independently recorded production output before recording the labeled wrong-domain control; absence of a nonempty discriminator is inconclusive, never pass.
- Preserve canonical object-layer pooling across regions, holes, voids, temporary internal-bridge anchor alterations and subsequent restoration; distinguish planning domains from final fill expolygons. Audit changing-island correspondence without polygon-vector identity.
- Record final fill identity at `Layer::make_fills` in `Fill/Fill.cpp` and propagate it explicitly through the opt-in recorder context to `Filler::_fill_surface_single` in `FillLightning.cpp`; the callee has no standalone region/paint accessor. Instrument only disposable cloned upstream sources, with a three-file grounding/caller/clipping pass. Require an explicit independently checked scene/canonical-region/paint/surface crosswalk, not identity guessed from geometry.
- Preserve the complete boundary-to-source contributor mapping through union/split/subtraction/hole and offset/temporary-anchor-generated boundaries. Each contributor carries full object/region/ordered variant/source surface/ring/segment/checkpoint identity, with exact overlap or causal operation lineage; sorting handles valid ties but never repairs unknown identity. A builds mapping from earlier source records; B validates canonical-domain input mapping and returns it losslessly with newly computed graph/witnesses. Missing source/shell/solid-angle evidence or unrepresentable lineage is named fail/inconclusive and blocks08, never defaulted parameters or fabricated RegionKeys.
- Record both Classic and Arachne wall strategies for each of five fixture families, with explicit shell, internal-solid, internal-bridge, paint/modifier and effective-configuration identities. A missing family/strategy is incomplete required coverage, not an implicit pass. Include two objects, non-Lightning regions contributing read context, region collection permutations, translated cases and canonical changed-parameter companions.
- Specify a concrete candidate domain builder from fixture-encoded late-PrePass records. It may reconstruct planning inputs locally but must not read future Layer output or obtain its expected domains from candidate output. Each consumed field gets a current host/guest availability audit; unavailable exposure becomes a future migration requirement, not a claim of framework acceptance.
- Gate A construction is pure and receives only restricted GateASourceInput: earlier attributed source geometry/classification, raw typed effective configuration/transform/schedule/order and earlier-only provenance. It returns complete domains/lineage plus independently derived policy/scalars. Canonical planning maps, final fills, expected graph/grounding/output and resolved-policy/constructor references are separate oracle data; broad B KernelInput is never A input. Missing earlier fields/formulas remain fail/inconclusive, not copied resolved values or presumed production exposure.
- Keep strict source-only input/schema/extraction and separate source/config/provenance/B/reference identities in the same three fixture files. No nested arbitrary metadata/extension/config blob can bypass the whitelist. Add dynamic reference-poison and references-absent construction trials plus actual B-domain perturbation to the existing AC-N2 test; compare complete construction/mapping/policy results and unchanged projected source/config/provenance bytes/hashes, not source grep or same-output self-assertions. Comparator verdict may change when only its oracle changes; A construction cannot. Immutable canonical wrong-domain replay intentionally consumes separate reference snapshots and is not the candidate API.
- **Gate B:** implement an isolated owned kernel whose dependency closure excludes `host-algos` on both targets; separately exclude it from the wasm32 guest closure. The SDK legitimately enables host-algos for its native target, so native adapter/host dependencies may contain it, but adapter code cannot call a host lightning producer/driver/fallback. Execute the same owned source through both real adapters with input-only fixture bytes. Preserve complete directed graph nodes/edge occurrences/root-grounding/origin metadata and full clipped segment multisets through existing output fields; independently compare both results against canonical expectations and exercise missing/extra/reversed/duplicate/disconnected/corrupt-witness controls, including identical corruption on both sides. Use only losslessly transported WIT-supported paint variants; reject Custom or absent-origin probe records rather than repairing production SDK behavior. No injected ready trees or equality-only proof.
- Author the exact new host test module/fixture loader/adapter driver and aggregate registration needed for the ACs. An SDK ordinary-stage wrapper may transport the controlled whole-print fixture through a test-only config string; that is neither a preparation capability nor production input delivery.
- Record separate gate outcomes, strategy selection, provenance, parameter/grouping policy, supported/unsupported configuration and future migration requirements. Append a future outcome section to the source plan at execution, preserving its approved body. Future update of the existing TASK-576 block also records separate `TASK-576 Gate A outcome:` / `TASK-576 Gate B outcome:` lines and accurate checkbox state; failed/inconclusive/incomplete investigations never become `[x]` successful acceptance. Retain named blockers. Neither document is edited during authoring.
- Fail-closed row08 entry: all generic rows01–06 must have implemented executable acceptance, and both independently verified `pass` outcomes must be recorded in the source plan before generating a migration packet.

## Out of Scope

- Production preparation, routing/config repair, WIT/SDK/framework integration, plan storage/lifecycle/diagnostics, plan visualization and real-slice migration acceptance; these remain rows01–06/08 work.
- Editing the existing host generator, producer, production lightning consumer, paint reconstruction, typed `LightningTreeIR` or shared accessor. No retirement, deprecation, fallback or replacement rollout.
- Treating the current holder-name difference as a tested integration defect; the isolated gates neither repair nor certify that path.
- A machine-specific sibling checkout as a normal test dependency; fixture goldens captured from the PnP host; visual similarity; native/WASM equality as the sole oracle; a compile-only guest witness; canned/echo outputs; hidden dependency on draft preparation APIs.
- Public schema fields/version changes, production config keys, new ADRs or deviation allocation. No parity-config snippet is applicable: fixture parameter policy is being investigated, not production keys implemented.
- During **generation**, any edit outside these five packet files and all implementation/build/test/acceptance execution. Row08 stays blocked and ungenerated.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: bounded direct slices for settled requirements, workstreams, lightning witnesses, correctness questions and queue; independent gate, source-plan recording and row08 entry rules.
- `docs/adr/0066-private-layer-preparation-capability.md`: accepted, not implemented; module ownership and no approximate geometry fallback.
- `docs/01_system_architecture.md`, PrePass Stage Order; `docs/02_ir_schemas.md`, SliceIR/SlicedRegion and post-perimeter fill invariants: late whole-print products are not final wall-inset fill domains.
- `docs/03_wit_and_manifest.md`, WIT organization/boundary enforcement; `docs/05_module_sdk.md`, native macro entry/per-call lifetime/Test Support: controlled existing adapter seams, not new preparation contracts.
- `docs/08_coordinate_system.md`: exact integer XY conversion; Z remains millimeters.
- `docs/ORCASLICER_ATTRIBUTION.md`: standard porting header on every translated Rust file, with real original C++ source path.
- `docs/21_data_defaults_and_fixtures.md`, struct literal and fixture rules; `docs/22_test_quality.md`, independent oracle, nonvacuous populations and negative controls.
- `AGENTS.md`, Test Discipline and Guest WASM Staleness: required-feature awareness, freshness by exit code, all-target check/clippy and tee logs.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `Generator::Generator`, `generateInitialInternalOverhangs`, `generateTrees`: first-region scalar-source policy, pooled internal/void surfaces and top-down tree planning.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill`: layer-grouped unsupported internal-solid candidates, sparse-area anchor expansion, modified-surface generation and restoration.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` — `Filler::_fill_surface_single`: final supplied expolygon clipping and emitted geometry.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::make_fills`: caller-owned final region/configuration/surface identity, explicitly propagated to the clipping observer; shell/solid angle policy is recorded at its consumer, not assumed available in a guest.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.hpp` — `Fill::fixed_angle`: bool type authority for the separately captured fixed-direction policy, delegated read-only; no additional instrumentation edit.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` — `Layer::getBestGroundingLocation` / `Layer::attach`: independently observed winning boundary/anchor and actual directed graph attachment.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp` — `Node::convertToPolylines`: deterministic root-point-hash sampling seed policy; time-based diagnostic names are not semantic evidence.

Only the source authority is currently inspected: `D:/slicerProject/pinch_n_print_cli_2/OrcaSlicerDocumented/`. It is not a portable dependency or executed reference. Canonical citations identify file plus function, never a line number.

Canonical constructor policy is deliberately not replaced with a speculative per-region generator: the production `Generator::Generator` reads generator-wide scalars from `all_regions.front()->config()`, while pooling geometry from the object's regions. The recorder must preserve canonical region order and the chosen scalar-source region, separately from PnP's stable full-identity delivery order. Input-order permutations must not alter full-identity config association; if canonical's first-region scalar choice changes, the recorded oracle changes explicitly too.

## Acceptance Summary

- Positive contracts: AC-1 separately identified source/reference fixtures; AC-2 pure restricted A construction before independent truthful comparison; AC-3 full source-boundary/caller/required-angle coverage; AC-4 separate executed B with full provenance/multigraph/clipped correspondence; AC-5 recording/entry gate.
- Negative contracts: AC-N1 unchanged immutable canonical wrong-domain replay; AC-N2 dynamic reference poisoning/absence, recursive source-smuggling rejection and actual B perturbation without A dependency, plus all existing graph/canned/fallback/contributor/generated-lineage/shell/tag3 controls; AC-N3 missing/failed/inconclusive evidence/framework acceptance. Exact criteria remain only in packet.spec.md; no ninth test or filtered-out poison population.
- Scientific failure is a valid **reported result**, not successful feasibility. Tests that directly compare a mismatching candidate against canonical remain red; an outcome-validator test may pass by proving a faithfully recorded failure still blocks migration. Never weaken either kind of assertion to close the task.
- Cross-packet impact: row08 may consume only explicitly verified strategy/kernel/evidence exports. Row07 does not certify any framework row or production migration.

## Verification Commands

All commands are **future execution obligations**, run in non-interactive Bash from the workspace root. Each test command creates `target` before writing and uses `set -euo pipefail`, so Cargo, tee and count-check failures all fail. Plain Cargo's combined tee is console-suppressed; stdout contains only bounded result lines. Xtask captures the complete test log internally and its command/preflight output goes to separate `target/lightning-gate-command.log`. Inspect/archive both logs before overwrite; never rerun for truncation. Ordinary fixture checks require no canonical checkout. Step3 authors dependency-blocked tests; compilation/testing starts only after Step8's required dev dependency and lock wiring.

| Command | Purpose | Return |
| --- | --- | --- |
| `python tools/lightning_gate/record.py --capture --output crates/slicer-wasm-host/tests/fixtures/lightning_gate` | Opt-in canonical execution in a disposable pinned source copy; never capture candidate expectations | SUMMARY ≤200 words: provenance, exits, case identities or blocker |
| `python tools/lightning_gate/record.py --replay --manifest crates/slicer-wasm-host/tests/fixtures/lightning_gate/manifest.json --output target/lightning-canonical-replay` | Reproduce recorded reference using pinned source/build recipe; compare normalized hashes | FACT matched/mismatch/infrastructure failure |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::canonical_fixture_provenance -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-1 | FACT pass/fail; failure snippet ≤20 lines |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::canonical_domain_gate_decision -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-2 | FACT truthful decision plus A status |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::canonical_coverage_and_grounding -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-3 | FACT coverage complete/incomplete |
| `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_gate_tdd::portable_kernel_native_wasm -- --exact --nocapture >target/lightning-gate-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-4; guest freshness preflight and actual execution | FACT native/WASM execution and B status |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::outcome_recording_and_migration_block -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-5 | FACT verified recording/blocked state |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::wrong_final_domain_is_rejected -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-N1 | FACT positive domain difference and named topology discriminator |
| `set -euo pipefail; mkdir -p target; cargo xtask test --summary -p slicer-wasm-host --test integration lightning_gate_tdd::portable_kernel_rejects_canned_echo_fallback -- --exact --nocapture >target/lightning-gate-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-N2 | FACT every substitute rejected |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test integration lightning_gate_tdd::missing_failed_inconclusive_blocks_migration -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | AC-N3 | FACT named fail-closed blockers |
| `cargo xtask build-guests --check` | Freshness prerequisite for any guest failure attribution; use exit 0/1/3, not STALE grep | FACT exit code; rebuild exit1, stop infrastructure exit3 |
| `cargo xtask build-guests` | Build stale guests only, required before B execution | FACT build/freshness result |
| `cargo check --workspace --all-targets` | Compile host/test/bench targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Closure warning gate | FACT pass/fail |
| `cargo xtask check-literals` | FRU/exhaustive-waiver gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Review/fix or justified waiver for touched tests | SUMMARY touched findings only |
| `rg -q '^## Lightning geometry/portability gate outcomes$' docs/specs/layer-module-preparation-plan.md` | Future outcome-section placement, not evidence of correctness | FACT present/absent |
| `rg -q '^- \[[ x~]\] \*\*TASK-576\*\*' docs/07_implementation_status.md` | Future existing TASK-576 item placement | FACT present/absent |
| `rg -q '^  - TASK-576 Gate A outcome: (pass\|fail\|inconclusive)\b' docs/07_implementation_status.md` | Future task-scoped A outcome placement, not scientific success | FACT present/absent |
| `rg -q '^  - TASK-576 Gate B outcome: (pass\|fail\|inconclusive)\b' docs/07_implementation_status.md` | Future task-scoped B outcome placement, not scientific success | FACT present/absent |

The packet-level eight-test aggregation command lives in `packet.spec.md`. No workspace test suite is required or authorized by this packet. Guest/multi-crate runs use `cargo xtask test`; narrow non-guest fixture tests use plain Cargo. Required-feature/aggregation facts are fixed against current manifests in `design.md`; never interpret zero filtered tests as acceptance.

## Step Completion Expectations

- Canonical recording precedes choosing numerical tolerances and comparing candidates. Persist physical/canonical relationships and normalization rationale before kernel tuning.
- Gate B consumes canonical planning inputs directly even if the proposed late-PrePass domain strategy fails Gate A; this keeps the sub-gates distinct. It cannot rescue a failed A result.
- A failed execution or domain proof is reported before any dependent authoring proceeds. Incomplete proof leaves this packet draft/unaccepted; a completed negative feasibility investigation may mark TASK-576 investigated, but cannot claim both success outcomes or row08 readiness.
- Outcome recording is a future implementation obligation, never a generation-time ledger fact. Derive revision/provenance and framework acceptance at the time of recording; reserve no IDs or SHAs in this draft.

## Context Discipline Notes

Canonical source reads and recorder execution are delegated; do not browse the sibling tree directly. Avoid generated WIT bindings, full Cargo output, lockfile contents and oversized fixture files. Explicit-target dependency/driver surveys return SUMMARY ≤200 words; command dispatches return FACT plus bounded failure snippets. Test fixture records are data to validate, not an excuse to silently skip unavailable populations.
