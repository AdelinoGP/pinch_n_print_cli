# Requirements: lightning-migration

## Packet Metadata

- Grouped task: `TASK-577`; backlog authority `docs/07_implementation_status.md`; status **draft / blocked**; context cost M per bounded authoring or implementation slice.
- Early authoring is approved; activation/implementation is not. No acceptance has executed. The five contract files are the only authoring edits.

## Problem Statement

The current host producer and sampler do not establish canonical input generation or a portable module-owned planner. Old parity starts with a precommitted tree and cannot settle late-PrePass geometry fidelity, object-layer pooling, configuration sourcing or grounding. The accepted solution transfers algorithm ownership and interpretation to the consuming artifact while retaining generic host transport/access/lifetime responsibilities.

## In Scope

- Same selected `com.core.lightning-infill` artifact owns static preparation and ordinary `Layer::Infill` calls. Use the accepted combined-export native/WASM model and real production runtime prefix; no injected Ready plan.
- Derive preparation eligibility from shared full-identity selection, not the global default `sparse_fill_holder` or a host lightning-name branch. Preserve target source, key, ordered paint chain, merged members, effective config, held roles and non-region provenance. Selecting/replacing an artifact selects/replaces both halves together.
- Source-only planning reconstruction, object-layer pooling of internal/internal-void domains, shell/solid/bridge/fixed-angle policy and first-region scalar selection follow independently verified row07 evidence. Preserve holes, split/merge/subtraction/temporary-anchor lineage and complete contributor multiplicities. Audit and resolve the relevant paint reconstruction loss, rather than ignoring `internal_solid_fill`/`internal_bridge_areas`.
- Ordinary calls use fresh instances, immutable owner-private bounded piece reads, current final fill geometry and complete original caller identity from canonical `Layer::make_fills`. Grouped surface records are not original source provenance. Planning and final clipping domains need not be identical; independent topology/grounding and final-attributed output checks are both mandatory.
- Private whole-print/per-layer/sparse pieces stay opaque to runtime/CLI and do not retain LayerArena or live module state. Module-owned codec/partition/layout decisions require B3 evidence before activation. Empty successful preparation is Ready/no-work; missing/malformed required data and failed preparation are errors, never fallback triggers.
- One useful module-defined `planning_geometry` final-plan view, scene `lightning-plan`, using row06 typed scene/primitive geometry only. Classes are `planning_domain`, `tree_branch`, `tree_root`, `grounding`. No requested sink means no diagnostic geometry allocation; requested empty views complete explicitly. Plan/projection are atomic and capture is semantically neutral.
- Direct deletion of the legacy host producer, host lightning algorithm ownership, LightningTreeIR and related entry/version/slots/stage, WIT accessor, SDK accessors, planner/binding/dispatch/marshalling/runtime paths, stale registry requirements and obsolete tests/docs identified in the design inventory. No deprecated coexistence or accessor shim. Preserve useful canonical oracle tests by retargeting, not weakening.
- NET-NEW production-driving `lightning_migration_tdd` e2e tests and aggregator registration. Native prepared registration must come from the actual module's paired generated entry/schema; WASM selection must instantiate its real composed export. Independent portable row07 source/reference files are loaded separately; absent or incompatible fixtures fail loudly.
- NET-NEW module-local source bridge/plan/diagnostic helpers only after blocking shape decisions are resolved. A test-only evidence/negative-compile harness may be added in the e2e test file without manufacturing execution artifacts or owning production input generation.
- Update live normative architecture, IR, WIT, scheduler, SDK and visual-debug guidance after verified retirement, preserving accepted ADR authority and historical evidence. Record completion through the orchestration owner, not this authoring agent.

## Out of Scope

- Framework03–06 redesign, new host decoder, new shared algorithm IR, arbitrary preparation byte input, extra module world/export, host fallback, approximate domains, later preparation scheduling or migration to LayerFinalization.
- Unapproved WIT/codec/public struct/interface/arena changes or resolving unavailable source fields by copying B input/canonical references. Relevant source/config exposure gaps require an explicit design/scope decision and a revised bounded plan.
- Canonical recorder/kernel/oracle re-authoring in row07, changes to other packet contracts, production paint fixes unrelated to the verified migration requirement, or generic renderer/CLI lightning branches.
- Treating identical adapter outputs, a successful failure-reporting test, preflight, compile-only proof, an old committed tree, final clipping alone or fabricated provenance as successful scientific evidence.
- Blanket workspace tests during authoring or by default during implementation. Human latency/memory savings and peak allocator claims without measurement.

## Authoritative Docs

- `docs/specs/layer-module-preparation-plan.md`: targeted settled requirements, projection contract, late-PrePass geometry questions, queue/early-authoring exception; delegated authority survey.
- `docs/adr/0066-private-layer-preparation-capability.md`: direct accepted ownership/atomicity/lifetime/retirement clauses.
- `docs/00_project_overview.md`: direct normative map; relevant legacy producer/IR/accessor and anchored/native clauses in `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md` read in targeted windows and surveyed through delegation.
- `docs/08_coordinate_system.md`, `docs/17_agent_debugging.md`, `docs/19_visual_debug.md`: direct unit/transform and diagnostic/capture ranges.
- `docs/21_data_defaults_and_fixtures.md`, `docs/22_test_quality.md`: direct rules/fixtures/waivers and oracle/population/negative controls; `docs/ORCASLICER_ATTRIBUTION.md` supplies new ported file headers.
- Producer contracts in queue01–07 are forward dependencies; verify actual shapes and acceptance at activation rather than freeze their generation status as implementation facts.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` — `Generator::Generator`, `generateInitialInternalOverhangs`, `generateTrees`: constructor sourcing, pooled domains and directed graph grounding.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill`, `prepare_lightning_infill_data`: modified planning surfaces before restoration.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `group_fills`, `SurfaceFillParams::operator<`, `Layer::make_fills`: effective parameter grouping versus original caller provenance.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp`, `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` — `Filler::_fill_surface_single`, `Layer::convertToLines`: sampling/clipping/connection, not a sufficient final-source recorder alone.

Canonical checkout location is optional authoring evidence only. Acceptance uses independent checked-in portable records and their provenance, not a required sibling checkout.

## Acceptance Summary

- Positive obligations: AC-1 scientific production equivalence; AC-2 full identity/pooling/callers; AC-3 real fresh consumers/lifetime; AC-4 typed atomic neutral projection; AC-5 direct retirement.
- Negative obligations: AC-N1 honest entry evidence; AC-N2 failure/missing/malformed payload; AC-N3 independent oracle/corruption; AC-N4 selection/override/no-work/access.
- Cross-packet impact: row07 validates scientific feasibility separately; rows01–06 remain generic and must not acquire lightning-specific contract behavior. Retirement affects ordinary imported WIT and its generated bindings; B4 must settle compatibility/rebuild/literal fallout before activation.

## Verification Commands

All commands here are planned, not executed. The nine exact commands below use Bash, preserve xtask's exit, require exactly one executed test and leave combined output in `target/test-output.log`. Archive that file and `target/lightning-migration-command.log` before another invocation, then successfully remove the old test log before each xtask test command under `set -euo pipefail`. `test_command` (`xtask/src/test.rs`) currently reports summary log-write errors without replacing Cargo's successful exit; absent fresh logs must therefore fail the subsequent count guard, never reuse a stale pass. No test listed here exists yet.

| Command/filter | Protected surface | Return |
| --- | --- | --- |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::production_native_wasm_canonical -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Real module preparations/consumers independently match canonical records for supported strategy/case matrix | FACT pass/fail, bounded failure details |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::full_identity_pooling_and_callers -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Target/config/member/provenance preservation, canonical pooling/scalar sourcing, final caller attribution | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::fresh_consumers_and_print_lifetime -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Fresh scopes, guarded lifetime, actual transport accounting, cleanup | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::typed_projection_is_atomic_and_neutral -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Actual row06 typed committed view and on/off production neutrality | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::legacy_contract_is_retired -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Real scheduler/dispatch absence plus expected removed-symbol compile diagnostics | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::activation_evidence_requires_executed_gates -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Synthetic rejections AND separate mandatory real-evidence positive trial | FACT pass/fail; cannot replace row07 scientific commands |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::failed_or_malformed_plan_never_falls_back -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | NotReady/no partial publication/no consumers/error identity/cleanup | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::independent_oracle_rejects_domain_and_identity_corruption -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Reference-isolation and genuinely discriminating comparator controls | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_migration_tdd::selection_override_empty_and_foreign_owner -- --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log` | Projection eligibility, override precedence, Ready-empty and owner denial | FACT pass/fail |
| `cargo xtask build-guests --check` | Fresh ordinary shared artifacts: exit0 fresh, exit1 stale/rebuild/recheck, exit3 infrastructure blocked | FACT exit; never infer from missing STALE text |
| `cargo xtask build-guests` after exit1; `--force` only for verified in-tree staging mismatch | Refresh changed WIT/module/binding artifacts, then rerun exact failed test | FACT exit |
| `cargo check --workspace --all-targets` | Complete removal/type/literal/exhaustive-match fallout compiles | FACT exit |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint closure across test/bench/example targets | FACT exit |
| `cargo xtask check-literals` | FRU/exhaustive waiver discipline, no unwitnessed fixture churn | FACT exit |
| `cargo xtask check-test-quality --report` | Fix/justify touched-code findings; report-mode exit alone is not quality proof | FACT findings |
| Each prerequisite's actual acceptance commands, independently archived and verified | Activation-only conjunction of accepted framework and actual Gate A/B passes, not this packet's green validator | FACT per sub-gate |
| `python -c "import sys; sys.exit('BLOCKED packet08: B1-B5 unresolved; generic rows01-06 acceptance and independently verified executed Gate A/B outcomes required before activation or any production migration edit')"` | Current fail-closed draft authorization hold; replacement requires independently reviewed actual-evidence mapping under B5 | Nonzero BLOCKED; never scientific evidence |

Exact names/codes for new codec errors and compile witnesses require B3/B4 resolution before these planned filters can be implementation-ready. No future version constant is pinned. No workspace suite is required by this draft; reapproval is required if closure adds it, and it must use the gated xtask entry point.

## Step Completion Expectations

No production step starts before the entry conjunction and all design blockers resolve. Typed input extraction precedes A construction; canonical references remain test-only and inaccessible to construction; plan output precedes ordinary admission only through atomic Ready. Consumer completion and applicable host synthetic closure completion precede disposal. Diagnostics derive from the same final module plan, not host interpretation of pieces. Host legacy removal is one unreleased coordinated transition; no intermediate retirement edit or negative-control green run counts as accepted migration. Public type/version fallout belongs to the originating step, with a bounded pre-authored inventory rather than compiler-driven follow-up edits.

## Context Discipline Notes

Source plan, normative contracts and cross-crate retirement closure require bounded delegation/ranged reads. Do not load large fixture JSON, generated bindings, locks, target trees or canonical source wholesale. Delegate authoritative fact checks and execution; return FACT or SUMMARY at most200 words. Scientific strategy/source-exposure gaps are explicit blockers, not instructions to read broader source until a plausible answer appears.
