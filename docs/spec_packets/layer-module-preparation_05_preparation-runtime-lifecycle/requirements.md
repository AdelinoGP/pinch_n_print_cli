# Requirements: preparation-runtime-lifecycle

## Packet Metadata

- Grouped task: `TASK-574`; backlog authority: `docs/07_implementation_status.md`.
- Queue authority: `docs/specs/layer-module-preparation-plan.md`, queue 05.
- Status: **draft; generated specification only, not implemented**.
- Aggregate context cost: M, decomposed into bounded S/M steps.

## Problem Statement

The optional preparation transport is not production activation. A print must freeze the artifact actually selected by existing override precedence, derive eligible work from the shared selection authority, prepare each participating owner once using final committed products, and keep immutable results available through every applicable ordinary and anchored invocation. The runtime must not accidentally prepare an unused module, call a prepared module with no meaningful work, use a different native artifact, expose half-written data, or free owners while calls remain in flight.

## In Scope

- Automatic production lifecycle, without a feature-specific opt-in or lightning prerequisite. Frozen selected bindings retain complete module identity, version, selected artifact/backend, preparation capability, config and provenance. Existing external-over-integrated precedence applies independently to preparation and ordinary invocation of the **same** binding.
- Derive whole-print participation from #01's shared stage-aware selected-target/eligibility result and #04's lossless committed input/selection facades. Do not confuse the legacy `invoke` flag with eligible prepared work. Preserve legacy plain calls with empty targets. No fill-holder-only roster, projection of unfinished Layer outputs, guessed future geometry, or duplicated selection logic.
- Eligible nondefault region selection even when global default differs; model paint/modifiers, perimeter members, multi-role ownership, support carriers, raft and non-region config/provenance. Actual selected `Layer::AnchoredEvents` executes in the ordinary module stage loop and can read its plan. Capability-derived synthetic `Anchored::Event` / `host:anchored-events` commits clone entities without module dispatch; they neither create module targets nor prove consumption. Preserve those builtin semantics and retain the print scope until their applicable host closure finishes.
- Late PrePass closure after final committed region/paint/shell/support products and ordinary prepass completion, before any LayerArena or anchored consumer starts. Gather once per eligible owner across the entire print; sort complete frozen identities deterministically. The immutable serialized plan, not an instance or unfinished arena, survives for the whole print including applicable anchored consumers.
- Ordinary module reconstruction by `from_config` remains per call. Mint fresh view/resource wrappers per ordinary/anchored call; permit concurrent cursorless immutable plan reads only within existing `layer_parallel_safe`/instance-pool limits. Separate concurrent prints, even when identity, config and artifact match.
- Atomic readiness including a deliberately empty ready plan. Required preparation return error, abort, trap, poisoned/missing readiness or required-piece decoder failure stops the slice **before** dependent consumers. Preserve original `ModuleError` fields, particularly `fatal: false`; required-dependency abort is a distinct host decision, not a rewritten module error. General ordinary-call non-fatal policy and its documented host limitation are not repaired here.
- Cooperative cancellation checkpoints before preparation, between owners and before consumers; no preemption of an active WASM call. Abort transaction and cancel outcome after any active preparation call returns. Join actual ordinary module calls (including Layer::AnchoredEvents) and finish/unwind applicable synthetic host work before cleanup. #03 disposal returns Busy rather than waiting; the runtime caller joins first. Actual retained/read-transfer accounting is not an allocator peak metric.
- Separate static capability/module/DAG descriptions from executed owner lifecycle records. Actual start/ready/no-work/failure/cancel/dispose observations name full owner identity, module/version and lifecycle phase separately from the ordinary consumer stage. Only instrumented observations have measured durations. Lifecycle records contain names/sizes, never raw prepared payloads.
- A metadata-only runtime preparation capture entry point for #06 through the same loading, committed prepass, frozen binding and preparation closure as production. It stops before LayerArena/module calls/synthetic host closure, snapshots names/lengths, lossless typed selected targets, lifecycle and actual production accounting, then cleans the store. It never opens/reads/copies/decodes opaque plan pieces. Q17/ADR-0066 diagnostics exclude raw plan exports; #06 alone adds opt-in atomic typed diagnostic snapshots, with no host plan-format decoder.
- Explicit future production `slicer-schema = { path = "../slicer-schema" }` dependency in runtime Cargo: move its current dev-only entry to `[dependencies]`, remove the redundant dev entry and keep fixture dependencies dev-only. Production runtime exposes PreparationExportSchema and cannot rely on a dev/transitive import. Schema has no manifest dependencies, so the direct runtime → schema edge introduces no cycle.
- NET-NEW private production `PreparedLayerStageRunner` wraps the real ordinary/prepared dispatcher and print scope in the existing layer runner slot. It forwards selected prepared `Layer::AnchoredEvents` and other Layer calls to `run_prepared_stage`; synthetic host collections remain outside that dispatch. No new anchored-stage architecture.
- Explicitly author a real runtime slice fixture, native and component adapter shims and fault injection if absent. Tests must invoke `run_slice`/the production closure; pre-populated Ready plans are not evidence of activation. Positive executed-test count, guest freshness, fixture presence and nonempty control populations must fail loudly.
- Reuse #02's schema-derived prepared/plain coverage corpus as **inputs**, not as the expected-output oracle. Independently hand-derive behavioral assertions from fixture selections and output markers. Include a non-infill prepared module through real native and WASM adapters, not a fake infill-only dispatcher.
- Update only the exact normative sections named in `packet.spec.md`'s Doc Impact Statement when implementing this packet.

## Out of Scope

- Generation does not implement, execute gates, activate, commit, edit the queue/backlog or any other packet.
- #01/#02/#03/#04 implementation, ownership/transport/schema redesign, capability-enumeration changes, new WIT interfaces or config keys, new public version constants, direct plan injection as activation proof.
- Lightning algorithm/migration/removal of `LightningTreeIR` or `PrePass::LightningTreeGen`: host legacy path remains until queue #08 separately resolves its blockers. No OrcaSlicer translation or parity claim.
- Between-stage barriers, resident instances, cross-print caches, mutable plans, unfinished arenas, ordinary error-policy fixes, synthetic-host-to-module dispatch conversion, raw opaque payload exports/host decoding, visual-debug schema, #06 typed diagnostic snapshots, allocator/report redesign or default full-workspace tests.

## Authoritative Docs

- `docs/00_project_overview.md`: delegated normative doc-map survey.
- `docs/01_system_architecture.md`: bounded reads of Data Ownership Rules, Memory Model and Priority tiers; native/WASM selected-artifact authority and arena lifetime.
- `docs/03_wit_and_manifest.md`: Concurrency & Instance Isolation; preparation transport itself is a forward dependency on #03, not present capability.
- `docs/04_host_scheduler.md`: bounded reads of Anchored invocation closure, PrePass Execution/Stage Prerequisites, Cooperative Cancellation, Error Handling Policy and Non-Fatal → FatalModule Host Limitation.
- `docs/05_module_sdk.md`: Module State Lifecycle; ordinary `from_config` reconstruction is unchanged.
- `docs/16_slicer_report.md`: Global allocator contract; logical retention does not claim process allocator peaks.
- `docs/17_agent_debugging.md`: DAG introspection and instrumentation are static versus executed evidence, respectively.
- `docs/21_data_defaults_and_fixtures.md` §1; `docs/22_test_quality.md` §1/§2: FRU, independent oracle, non-vacuity and registered test targets.
- `docs/adr/0029-lightning-prepass-tree-generator.md` Decision/Future-Reviewer Notes: retain legacy lightning; framework work does not amend this ADR.
- `docs/adr/0066-private-layer-preparation-capability.md` diagnostics/private-plan contract and source plan Q17: bounded delegated grounding; no raw opaque capture, module-owned interpretation, #06-only opt-in typed atomic diagnostics.
- Producer packet public contracts, frozen at authoring by a bounded export survey. They are generated drafts, **not implemented dependencies**.

## Acceptance Summary

- Positive: AC-1 through AC-10 in `packet.spec.md`.
- Negative: AC-N1 through AC-N5 in `packet.spec.md`.
- #06 consumes the exact net-new lifecycle/capture public exports from `packet.spec.md`; #08 remains blocked independently. No criterion's forward dependency is evidence that the criterion currently passes.

## Verification Commands

The authoritative matrix follows. These are **future execution** commands; generation runs none of them. All guest-driving AC/step invocations first require the freshness row to return exit zero. On stale exit 1 rebuild and recheck; exit 3 is an infrastructure blocker, not cleanliness. `cargo xtask test` is the required entrypoint if a later separately authorized whole-suite/multi-crate run is needed; this packet requests no full workspace suite.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Every ordinary artifact freshness gate, including new auto-discovered lifecycle guest; inspect exit code, not text absence | FACT exit 0 fresh / 1 stale / 3 infrastructure; rebuild stale before attribution |
| `cargo xtask build-guests` then `cargo xtask build-guests --check` | Only if freshness reports stale; new fixture is a WASM build input | FACT build result and fresh exit 0 |
| `cargo check --workspace --all-targets` | Production/test/bench/example compile gate | FACT pass/fail |
| `python -c 'import pathlib,tomllib; r=tomllib.loads(pathlib.Path("crates/slicer-runtime/Cargo.toml").read_text()); s=tomllib.loads(pathlib.Path("crates/slicer-schema/Cargo.toml").read_text()); assert r["dependencies"]["slicer-schema"]["path"] == "../slicer-schema"; assert "slicer-schema" not in r.get("dev-dependencies",{}); assert not s.get("dependencies") and not s.get("build-dependencies") and not any(t.get("dependencies") or t.get("build-dependencies") for t in s.get("target",{}).values()); print("production schema edge; no schema back edge")'` | Future exact production dependency/no redundant dev entry and dependency-free schema graph authority, paired with all-target compile gate; Python requires tomllib | FACT exit/message; changed schema dependency graph requires re-grounding, never bypass the assertion |
| `cargo clippy --workspace --all-targets -- -D warnings` | All-target warnings gate | FACT pass/fail |
| `cargo check --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures` | Explicit forward feature introduced by #03; ensure its controlled fixture graph also compiles | FACT pass/fail; this feature is not needed by final committed-input runtime tests |
| `cargo clippy --workspace --all-targets --features slicer-wasm-host/preparation-test-fixtures -- -D warnings` | Same explicit controlled-fixture feature graph, all targets | FACT pass/fail |
| `cargo xtask check-literals` | Watched new test structs use FRU/justified waiver; no weakened assertions | FACT exit and violation summary |
| `cargo xtask check-test-quality --report` | Fix/justify findings in touched code; report exit alone is not enforcement | FACT findings in touched files and resolution |
| Every full AC command from packet.spec AC-1 through AC-10 and AC-N1 through AC-N5 | Exact one-test e2e/library assertions and corresponding doc-section greps; AC-10 is real module-owned anchored read plus separate synthetic lifetime control | FACT per command, positive executed count, bounded failure snippet |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e preparation_runtime_lifecycle_tdd 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | All newly registered full-driver cases together, no fixture skip | FACT positive executed count/result; read existing log on failure |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --lib preparation_lifecycle::tests::missing_ready_blocks_consumer -- --exact 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Real production readiness guard malformed-completion negative case | FACT positive executed count/result |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test e2e run_slice_api_tdd::run_slice_against_wedge_returns_nonempty_gcode -- --exact 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Existing real full-slice baseline compatibility, not new lifecycle evidence | FACT positive executed count/result |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --test integration hybrid_pilot_external_override_tdd::hybrid_pilot_external_override_forces_wasm -- --exact 2>&1 \| tee target/test-output.log >/dev/null; rg -q '^test result: ok\. [1-9][0-9]* passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Existing external-over-integrated dispatch compatibility; not preparation activation evidence | FACT positive executed count/result |

`\|` in this Markdown table escapes the cell separator; execute it as the normal Bash `|` pipe. Runtime has default `report`, no e2e `required-features` or aggregator `#![cfg(feature)]`, and unconditional `slicer-core/host-algos`; no silent feature-free empty binary is accepted. Library negative tests live inside the new runtime module and need no new Cargo feature. The forward #03 controlled fixture feature is named explicitly in its compile gates only.

### Exact Test/Fixture Homes

NET-NEW guest `crates/slicer-wasm-host/test-guests/preparation-lifecycle-guest/{Cargo.toml,src/lib.rs,module.toml}`; paired native rlib supplied via runtime dev-dependency; component staged by existing discovery as `crates/slicer-wasm-host/test-guests/preparation-lifecycle-guest.component.wasm`. Real prepared macro/final typed input adapters—not `read-fixture` or pilot-only stores—are used. The new fixture is independently authored here because no current runtime preparation fixture exists.

NET-NEW `crates/slicer-wasm-host/test-guests/preparation-lifecycle-anchored-guest/{Cargo.toml,src/lib.rs,module.toml}`, native dev-dependency plus discovered `preparation-lifecycle-anchored-guest.component.wasm`. Its actual `LayerModule::run_anchored_events` reads/decodes required `probe` and uses existing `LayerCollectionBuilder::set_anchored_event_collection` to emit nonempty anchored geometry. Literal native/WASM X expectations and a changed-piece negative control prove module-owned consumption. The observer separately traces synthetic host closure Start/Finished without treating it as module dispatch or a producer read guard. The required fixture is authored here, not assumed provided by #04 input-only witnesses.

The #04 `whole-print-input-{path,perimeters,infill,support,anchored}` fixtures are forward produced inputs, not currently runnable artifacts. After their gate, stage-accurate target-kind cases reuse their component/native entry families and add the necessary native library dev-dependencies in runtime Cargo without editing those fixtures. Resolve actual Cargo package names from their implemented manifests. Reuse #02's schema-derived prepared/plain input corpus for coverage, never its expectation-building logic. A single PathOptimization fixture is not claimed to execute every stage family.

NET-NEW `crates/slicer-runtime/tests/e2e/preparation_runtime_lifecycle_tdd.rs`, registered by `mod preparation_runtime_lifecycle_tdd;` in existing `tests/e2e/main.rs`. Its full-driver and explicit extension fixtures use the verified existing wedge driver, real native registrations, staged selected WASM components, actual anchored-module runner boundary and ordinary output comments. Observer records retain errors even when the slice fails. Synchronization is explicit, not inferred from elapsed time. Embedded `crates/slicer-runtime/src/preparation_lifecycle.rs::tests::missing_ready_blocks_consumer` uses the actual production all-Ready guard and a malformed-completion shim, never a pre-injected Ready plan.

Known cancellation is cooperative: phase/layer checkpoints, no active WASM interruption. This packet adds owner checkpoints and safe publication/cleanup, not a claim existing prepass supports in-flight cancellation. Existing run_slice anchored input is empty; extensions supply only synthetic host entities. Real prepared anchored consumption is selected ordinary Layer::AnchoredEvents dispatch through the new forwarding runner, not those entities. `prepare_prepass_context` omits startup validation; metadata-only capture uses the full run_slice prefix instead.

## Step Completion Expectations

Production schema dependency and fixture registration precede compilation of public runtime types/filtered tests. Frozen binding/eligibility precede transactions; committed inputs precede prepare; Ready precedes module dispatch. No cleanup until actual module calls and applicable host closure finish; caller joins before disposal, preserving Busy semantics. Capture carries names/lengths/selection only; transferred total is successful production returned-piece bytes, not raw copies, metadata sizes, requested lengths or put traffic. Ready handles never cross prints. No existing field/WIT/version/config shortcut; unexpected changes require full blast-radius inventory/split/reapproval.

## Context Discipline Notes

Runtime driver/executor/dispatch files are long: locate the named symbols first, then read at most ±40 lines; never read them wholesale. Delegate macro/generic tracing and guest freshness/gates with bounded FACT returns. Skip unrelated geometry algorithms, fixture JSON, generated bindings, lockfiles, target output and Orca source. Test logs are inspected from disk after execution, never regenerated just to recover truncated output.

Authoring-size diagnosis: the necessary detail is the native/WASM same-artifact closure, full-identity target kinds, required-failure/cancellation/cleanup witnesses, independently observable arena lifetime, and #06's exact public capture exports. Repeated scope/criteria/code-surface prose was avoided by assigning those owners to separate files; repeated narrow AC commands are deliberate independent executable contracts. Do not trim negative cases, atomic exits or producer shapes to meet a rough line-size guideline.
