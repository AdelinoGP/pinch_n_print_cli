# Implementation Plan: preparation-capability-pilot

## Execution Rules

This is a draft plan, not permission to execute. Work one atomic contract at a time after independent preflight and activation. TDD uses real typed boundaries and independently specified fixture outputs. Each dispatch returns outside thinking, escaped, with FACT pass/fail and explicit execution state; reject full Cargo output. All test commands use pipefail and combined tee, then reject zero executed tests. Read `target/test-output.log` after failures; do not rerun merely to recover truncated output.

Existing public structs, ordinary WIT signatures and version constants are not widened/bumped. Every step owns all literals/assertions of its new types in its allowed files. A discovery of necessary existing-field/version fallout is a scope blocker: inventory it with LOCATIONS and split the step before proceeding, never a follow-up unbounded check repair. The only known pre-existing literal fallout is embedded WIT include/watch set assertions in `xtask::wit_verify`, owned with macro embedding below.

## Steps

### Step 1: Canonical capability types and composed worlds

- Task IDs: TASK-573.
- Objective: Introduce NET-NEW imported preparation resources, exported method and sidecar identity from the forward summary.
- Precondition: independent preflight/activation; destination code symbols absent; ordinary stage authority verified. Before any SDK/macro/WIT edits, run the existing `cargo xtask build-guests --check` (rebuild if stale, do not treat exit 3 as clean), copy the actual `modules/core-modules/classic-perimeters/classic-perimeters.wasm` to `target/preparation-pilot/plain-before.component.wasm`, and record SHA-256 provenance beside it. These are implementation-time generated evidence artifacts, not additional source edits. No such snapshot is claimed captured by this author.
- Postcondition: preserved pre-edit plain artifact/hash exists; canonical parser resolves prepared worlds including unchanged real stage imports; sidecar does not change `SlicerModuleSchema`/`StageSpec` fields.
- Allowed reads: `crates/slicer-schema/src/lib.rs` StageSpec/STAGES 45–189 and schema definitions by symbol; existing small Layer WIT files; ADR-0045 decision 54–118; ADR-0066.
- Allowed edits (2): NET-NEW `crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit`; `crates/slicer-schema/src/lib.rs`.
- Out of bounds: existing WIT deps/root, IR/schema version constants, scheduler/runtime, all other packets.
- Blast radius: only new resource/sidecar declarations and their local literals; no existing fields/version literals change. Derive all module-backed Layer worlds from STAGES, not a frozen count.
- Dispatch: question "does canonical WIT resolve every schema-derived composed world and imported resource identity?"; scope these two edits and existing Layer deps; return FACT at most five lines.
- Cost: S.
- Authorities: docs/03 Host-Boundary Access and package compatibility (100–189), ADR-0066.
- Verification: `cargo check -p slicer-schema --all-targets`; FACT exit. Parser/instantiation completion is intentionally deferred to the actual driver gate, not claimed by this Rust-only check.
- Falsifying exit: stop if composition requires changed plain signatures, exported resource ownership, or a second scheduled stage; otherwise declarations ready for macro and real-driver tests.

### Step 2: Required SDK trait and serialized read facades

- Task IDs: TASK-573.
- Objective: Introduce `LayerPreparation` without a default method and lazy call-local facade wrappers.
- Precondition: new canonical operation/error names fixed by Step 1; no production input projection is assumed.
- Postcondition: SDK can express controlled input/output, implicit-owner open/len/read, error propagation and unwind-safe native read binding without retaining indices or copying the whole plan.
- Allowed reads: `crates/slicer-sdk/src/lib.rs` 18–60; `crates/slicer-sdk/src/traits.rs` 377–405, 558–578; existing error type by symbol; SDK test target Cargo metadata; docs/05 lifecycle 958–1001.
- Allowed edits (3): NET-NEW `crates/slicer-sdk/src/preparation.rs`; `crates/slicer-sdk/src/lib.rs`; `crates/slicer-sdk/tests/layer_module_tdd.rs`.
- Out of bounds: `LayerModule` signatures, production PrePass projections, host algorithm APIs, existing config/IR shape changes.
- Blast radius: new facade structs only; update all new test literals in layer_module_tdd with FRU/waiver. No existing version assertion fallout.
- Dispatch: question "do new SDK tests fail on a missing required method and scoped-read leakage?"; scope three allowed edits; return FACT plus at most 20 assertion lines on failure.
- Cost: S.
- Authorities: docs/05 Module State Lifecycle and Test Support; docs/21 and docs/22.
- Verification: `set -o pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test layer_module_tdd 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- Falsifying exit: scoped native binding must restore after unwind and reject missing current plan; any retained instance/index or default-success trait fails the step. Compile-fail actual macro opt-in is additionally proved in Step 9.

### Step 3: Native paired entry sidecar

- Task IDs: TASK-573.
- Objective: Add generated-entry envelope without widening ordinary native request/response/enum shapes.
- Precondition: SDK preparation wrappers compile and required method has no body.
- Postcondition: new `NativePreparedStageEntry` holds an ordinary `NativeStageEntry` and required preparation pointer with the exact forward signature; each consumer still constructs via ordinary from_config.
- Allowed reads: `crates/slicer-sdk/src/native.rs` 17–65 and 149–165; `crates/slicer-sdk/src/preparation.rs`; `crates/slicer-sdk/tests/layer_module_tdd.rs` targeted added tests; ADR-0056 decision 44–92.
- Allowed edits (2): `crates/slicer-sdk/src/native.rs`; `crates/slicer-sdk/tests/layer_module_tdd.rs`.
- Out of bounds: all existing native envelope fields, scheduler registry/override, algorithm-native objects.
- Blast radius: new two-field struct and local construction tests only; no old literals/constants change.
- Dispatch: question "is paired entry a function-pointer envelope with serialized scoped reads and unchanged ordinary shapes?"; scope these edits; return FACT at most five lines.
- Cost: S.
- Authorities: ADR-0056, ADR-0066, docs/05 lifecycle.
- Verification: `set -o pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test layer_module_tdd 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- Falsifying exit: native paired entry compiles and calls required preparation; a host algorithm object/persisted module or changed ordinary envelope blocks completion.

### Step 4: Same-artifact macro composition and freshness identity

- Task IDs: TASK-573.
- Objective: Emit opted-in real preparation plus ordinary stage guest/native glue, and teach artifact validation the recognized capability.
- Precondition: Steps 1–3 exports, SDK trait and sidecar are available.
- Postcondition: opted-in expansion resolves combined resources, exposes required native pointer and separate preparation schema; plain expansion remains unchanged; checker retains one ordinary-stage candidate and strictly validates preparation exports/resources.
- Allowed reads: macro `slicer_module`/`generate_slicer_module_impl` 28–187, `emit_world_preamble` symbol window and per-stage glue windows; macro build.rs; xtask `resolve_stage_from_world`/`compare_worlds` 191–349 and include/watch assertions 1280–1459; canonical capability WIT; docs/03 package compatibility.
- Allowed edits (3): `crates/slicer-macros/src/lib.rs`; `crates/slicer-macros/build.rs`; `xtask/src/wit_verify.rs`.
- Out of bounds: ordinary WIT bodies, schema/IR constants and request structs, manifest parsing, production dispatch/activation.
- Blast radius: new WIT include/watch entry changes pre-existing include-set/watch-set assertions in `wit_verify::tests`; resolve exact updated sets from actual include/build watch/schema authority in this same step. Preserve full-equality assertions and negative unknown/multiple stage tests; do not merely relax counts or add capability to an unchecked shared-package allowlist.
- Dispatch: question "does opted-in glue resolve real resources while checker distinguishes one stage plus validated capability?"; scope three edits; return FACT; failing concrete signature SNIPPETS at most 20 lines.
- Cost: M.
- Authorities: ADR-0045 export/version/resource decisions, ADR-0066, docs/03 host ownership, docs/05 single-stage constraint.
- Verification: `cargo check --workspace --all-targets`; `set -o pipefail; mkdir -p target; cargo test -p xtask --bin xtask wit_verify::tests 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`; new `preparation_pilot_` resolver tests must be included in this unit module.
- Falsifying exit: unknown/multiple stage or malformed/missing exported capability must remain rejected. Binding failure requiring changed plain stage resource identity stops dependent work for design decision, not workaround.

### Step 5: Macro opt-in regression checks

- Task IDs: TASK-573.
- Objective: Test explicit opt-in parsing and retain single scheduled stage behavior.
- Precondition: macro composition and paired adapter generation implemented.
- Postcondition: attribute has semantics rather than ignored tokens; ordinary no-opt-in metadata/export remains plain, extra scheduled-stage impl remains rejected.
- Allowed reads: `crates/slicer-macros/tests/slicer_module_tdd.rs` symbol windows; macro parser/generator 28–187 and new opt-in seam; `crates/slicer-schema/src/lib.rs` STAGES.
- Allowed edits (1): `crates/slicer-macros/tests/slicer_module_tdd.rs`.
- Out of bounds: guest fixtures not yet authored, production scheduler/manifest, acceptance claims from source-grep alone.
- Blast radius: test-local new sidecar references/literals only; existing schema fields/constants stay unchanged.
- Dispatch: question "do macro regressions exercise opt-in metadata and one-stage rejection?"; scope target slicer_module_tdd; return FACT, failing assertion at most 20 lines.
- Cost: S.
- Authorities: docs/05 Single-Stage-Per-Impl Constraint; docs/22 compile witness limits.
- Verification: `set -o pipefail; mkdir -p target; cargo test -p slicer-macros --test slicer_module_tdd 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- Falsifying exit: opt-in attribute must select prepared composition, no opt-in must select original composition. This exit does not claim runtime execution; actual missing-method compile failure is Step 9.

### Step 6: Dual-target infill authoring example

- Task IDs: TASK-573.
- Objective: Author a real same-source module with explicit macro opt-in, numeric codec and complete named-piece preparation.
- Precondition: preparation trait/macro generated paired adapters exist; freshness checker recognizes the capability.
- Postcondition: new `preparation-pilot-infill` crate is discovered, stages `preparation-pilot-infill.component.wasm`, compiles native rlib and guest cdylib, and supplies required preparation/ordinary methods.
- Allowed reads: `xtask/src/build_guests.rs` discover_guests 217–299; schema ordinary infill WIT; macro infill glue 3598–3641; SDK InfillOutputBuilder by symbol; IR Point3WithWidth 2614–2644 and ExtrusionPath3D 2818–2838; existing guest Cargo manifests located through glob, not invented names.
- Allowed edits (3): NET-NEW `crates/slicer-wasm-host/test-guests/preparation-pilot-infill/Cargo.toml`; NET-NEW `crates/slicer-wasm-host/test-guests/preparation-pilot-infill/src/lib.rs`; `crates/slicer-wasm-host/Cargo.toml` (native dev-dependency).
- Out of bounds: community/core production modules, host algorithm kernel, production declarations/activation.
- Blast radius: every new guest struct literal authored exhaustively with existing IR fields; no IR/WIT version mutation. Test driver uses FRU/waivers later. Empty `[workspace]` sentinel plus explicit dependency paths follows current standalone discovery; no new per-guest target directory.
- Dispatch: question "is the numeric fixture a discovered real native/guest macro module?"; scope guest crate, discovery entry and artifact gate; return FACT at most five lines.
- Cost: S.
- Authorities: docs/05 Guest Build Invariants, docs/08, docs/22 independent oracle, ADR-0066.
- Verification: `cargo xtask build-guests --list` (FACT named entry); `cargo xtask build-guests`; `cargo xtask build-guests --check`; `cargo check -p slicer-wasm-host --all-targets`.
- Falsifying exit: missing discovery/artifact or nonzero freshness status prevents acceptance; compilation alone does not satisfy AC-1/2.

### Step 7: Dual-target non-infill authoring example

- Task IDs: TASK-573.
- Objective: Prove the authoring shape is not Infill-specific using PathOptimization's actual perimeter/collection/G-code input/output family.
- Precondition: macro composition and first fixture discover/build; no production routing needed.
- Postcondition: new `preparation-pilot-path` native rlib/guest cdylib artifact stages correctly and module uses its own text codec and ordinary comment builder.
- Allowed reads: path-optimization.wit complete small file; SDK traits 558–578; GcodeOutputBuilder::push_comment/commands symbol windows in postpass_builders.rs; discover_guests 217–299; approved design fixture contract.
- Allowed edits (3): NET-NEW `crates/slicer-wasm-host/test-guests/preparation-pilot-path/Cargo.toml`; NET-NEW `crates/slicer-wasm-host/test-guests/preparation-pilot-path/src/lib.rs`; `crates/slicer-wasm-host/Cargo.toml` (second native dev-dependency).
- Out of bounds: real path optimizer algorithm/production modules, manifests/routing/activation, infill fixture's independent numeric codec.
- Blast radius: new local fixture literals only, no existing type/version changes.
- Dispatch: question "does the distinct text fixture compile its real non-infill stage through the same opt-in adapter?"; scope new crate and discovery artifact; return FACT at most five lines.
- Cost: S.
- Authorities: ADR-0056/0066; docs/03 ordinary stage signatures; docs/22 compile versus dispatch.
- Verification: `cargo xtask build-guests --list` (FACT named entry); `cargo xtask build-guests`; `cargo xtask build-guests --check`; `cargo check -p slicer-wasm-host --all-targets`.
- Falsifying exit: exact PathOptimization composed export and staged artifact must exist/fresh; merely renaming an Infill signature fails.

### Step 8: Real imported resource host and typed driver

- Task IDs: TASK-573.
- Objective: Implement pilot-local imported resources, canonical shared type aliases and typed combined driver without changing production HostExecutionContext.
- Precondition: both real fixture artifacts and native adapters exist and checker can validate them.
- Postcondition: NET-NEW pilot host has fresh capability resource tables, ordinary host projection, owner/ready/staging storage and actual typed calls; preparer store drops before consumer stores are constructed.
- Allowed reads: host.rs canonical definer/alias 363–458, context 1188–1212 and resource insertion/output getter symbol windows; production_guest_smoke_tdd.rs complete small file; contract main.rs; new capability WIT and native fixture entries; docs/03 isolation 100–142.
- Allowed edits (2): NET-NEW `crates/slicer-wasm-host/tests/contract/preparation_pilot_host.rs`; `crates/slicer-wasm-host/tests/contract/main.rs` (register helper).
- Out of bounds: production host.rs/context fields, runtime runner/lifecycle/blackboard, manifest activation, ordinary WIT resource defs.
- Blast radius: all new host backings/ctx/storage literals local to helper; no `HostExecutionContext` fields or native envelope expansion. Keep mapped capability resource types identical across prepare/plan-access bindings.
- Dispatch: question "can actual composed artifacts instantiate and call imported host resources with ordinary shared identities?"; scope helper and two artifacts; return FACT, concrete type error at most 20 lines.
- Cost: M.
- Authorities: ADR-0066 resource identity, ADR-0045 typed instantiation, docs/03, docs/21/22.
- Verification: `cargo build --workspace --tests`; `cargo check -p slicer-wasm-host --all-targets`; `cargo xtask build-guests --check`. Runtime proof is next step, not a success-only compile claim.
- Falsifying exit: actual driver compiles with ordinary aliases and fresh resource tables; inability to do so under agreed ownership stops dependents for design review. No import-free fallback.

### Step 9: Behavioral, negative, compatibility and schema-surface witnesses

- Task IDs: TASK-573.
- Objective: Execute the real public transport/typed adapter seam, not injected-ready scaffolding, for AC-1–4 and AC-N1–N4.
- Precondition: actual resource host and both native/guest fixtures compile; guests checked fresh.
- Postcondition: actual tests exercise store destruction, independently expected output, owner/print isolation, empty/missing state, publication poison/failure/trap, checked bounds/denied input, malformed codec, export mismatch, plain pre-existing guest and schema-derived prepared/plain compilation with negative required-method control.
- Allowed reads: new helper/fixtures and new SDK facades; `crates/slicer-schema/src/lib.rs` STAGES 45–189; contract aggregator; production smoke small file; original per-stage WIT signatures complete small files; docs/22 17–151.
- Allowed edits (3): NET-NEW `crates/slicer-wasm-host/tests/contract/preparation_capability_pilot_tdd.rs`; `crates/slicer-wasm-host/tests/contract/main.rs` (register actual tests); `crates/slicer-wasm-host/tests/contract/preparation_pilot_host.rs` (required driver refinements only).
- Out of bounds: ordinary production pipeline scheduling, other test aggregators, hand-maintained stage rosters, fixture-skip/ignored correctness tests, external Orca sources.
- Blast radius: all test new struct literals FRU/waived; no old fields/constants. Generate scratch compile source and dependencies from actual schema/guest manifests, not invented signatures; scratch outputs are artifacts, not code edit scope.
- Dispatch: question "does each pilot test execute nonzero and falsify its specified input through actual native/guest calls?"; scope registered preparation_pilot_ contract tests; return FACT per category, at most 20 failure lines. Separate question "does compile driver cover exact schema-derived set and fail missing required trait?"; same test scope; return FACT.
- Cost: M.
- Authorities: docs/22, ADR-0066, docs/03 isolation/resource authority, docs/05 lifecycle.
- Verification: `set -o pipefail; mkdir -p target; cargo xtask build-guests --check && cargo test -p slicer-wasm-host --test contract preparation_pilot_ -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- Falsifying exit: every named behavioral/negative test passes with actual outputs and compile driver nonempty exact coverage; false-green zero tests, ready injection in primary proof, missing guest silent skip or feasibility workaround blocks completion.

### Step 10: Measure and bound the pilot transport evidence

- Task IDs: TASK-573.
- Objective: Add AC-5 measured retained accounting/transfers/timings with metadata-only output.
- Precondition: behavioral/negative correctness witnesses pass; no numbers inferred from expectations are reported as runtime measurements.
- Postcondition: measured representative preparation and fresh-read records include separate payload/name UTF-8 retention, transfer bytes/counts and observed durations; retention released after last call/abort; no payload metadata or allocator/process peak claim.
- Allowed reads: new helper/test instrumented boundary windows; source plan private lifetime/accounting 261–301 and required measurements 444–471; docs/22 oracle restrictions.
- Allowed edits (2): `crates/slicer-wasm-host/tests/contract/preparation_pilot_host.rs`; `crates/slicer-wasm-host/tests/contract/preparation_capability_pilot_tdd.rs`.
- Out of bounds: retained quotas, disk/cross-print cache, production telemetry schema, benchmark targets/process peak estimates.
- Blast radius: all new accounting fields and every helper/test construction site are owned by these two files; metadata whitelist is explicit. No existing schema/version change.
- Dispatch: question "are accounting and timing records actual observations with metadata payload exclusion and no peak-memory inference?"; scope preparation_pilot_accounting output and retained release assertions; return FACT at most five lines, measured records only.
- Cost: S.
- Authorities: ADR-0066 Q11/Q16 ownership; source plan accounting; docs/22.
- Verification: `set -o pipefail; mkdir -p target; cargo test -p slicer-wasm-host --test contract preparation_pilot_accounting -- --nocapture 2>&1 | tee target/test-output.log >/dev/null && python -c "import re,pathlib; s=pathlib.Path('target/test-output.log').read_text(); n=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',s))); assert n>0; print('PASS executed='+str(n))"`
- Falsifying exit: UTF-8-name and payload totals equal independent sums, transfer records/timings measured, raw payload metadata rejected, release observed; unmeasured/forecast numbers cannot close AC-5.

### Step 11: Record only executed pilot evidence and authoring contract

- Task IDs: TASK-573.
- Objective: Add exact doc-impact sections with compiled authoring example, commands/results, limitations and forward production boundaries.
- Precondition: actual pilot tests/compile matrix and accounting evidence passed; feasibility outcome known.
- Postcondition: docs explain composed resource identities, required opt-in/native spelling, measured results and what rows 03–06 still own; no claim production preparation is active.
- Allowed reads: exact doc sections docs/03 package compatibility 144–189, docs/05 entry point 179–210/single-stage 467–484/lifecycle 958–1001; ADR-0066; pilot test log/evidence bounded summaries; forward export summary.
- Allowed edits (3): `docs/03_wit_and_manifest.md`; `docs/05_module_sdk.md`; `docs/adr/0066-private-layer-preparation-capability.md`.
- Out of bounds: source plan/queue, glossary/other ADR decisions, production availability declarations, TASK-574 completion.
- Blast radius: no code fields/WIT/version changes; existing accepted ADR decision retained and evidence appended.
- Dispatch: question "do exact sections describe executed evidence and still-pending production work truthfully?"; scope three added sections; return FACT at most five lines.
- Cost: S.
- Authorities: ADR-0066/0045/0056 and executed pilot reports; in-tree file+symbol citation discipline.
- Verification: `rg -q '^## Preparation capability pilot composition' docs/03_wit_and_manifest.md && rg -q '^### Preparation capability pilot authoring' docs/05_module_sdk.md && rg -q '^## Executable pilot evidence' docs/adr/0066-private-layer-preparation-capability.md`.
- Falsifying exit: all exact headings and truthful executed evidence present; no stub/compile-only probe described as runtime proof and no dependent production work marked complete.

## Per-Step Budget Roll-Up

| Steps | Cost | Bounded deliverable |
| --- | --- | --- |
| 1–3 | S each | Additive WIT/schema and SDK/native types, no old shape fallout |
| 4 | M | Macro resource composition plus known freshness literal fallout |
| 5–7 | S each | Regression tests and two small fixture crates |
| 8–9 | M each | Resource driver and registered behavioral witnesses |
| 10–11 | S each | Measured evidence and exact docs |

Aggregate M across isolated workers; no step L. Controller holds only contract/evidence summaries; do not load all macro/host source or Cargo output into one session.

## Packet Completion Gate

- Run each pipe-suffixed AC exactly, after docs exist; each returns nonzero executed summary plus exit 0. Tests use narrow targets; check/clippy use --all-targets.
- Full matrix in requirements: WIT test-target build, guest freshness exit 0, check/clippy, literals and touched test-quality findings resolved. No default workspace test ceremony.
- Independent spec-review closure must examine actual executed artifacts/native adapters and required negative controls. Pilot feasibility failure stops row 03 finalization; report precise contract conflict for design decision.
- Only after accepted actual implementation, worker updates TASK-573 in `docs/07_implementation_status.md`; this is not an authoring edit, never a full backlog read. Return FACT. No other task completion is implied.
- Status remains draft through authoring/independent preflight. Later implementation closure may mark implemented only after all real gates; no activation/commit belongs to this delegated authoring task.

## Acceptance Ceremony

Redispatch every AC and packet-level gate, capture compact actual pass/fail and executed counts; keep the complete log on disk. Record measurements with provenance and unmeasured allocator/process memory gaps. Confirm ordinary artifact remains loadable, paired native/guest source is real, no instances/indices survived, and all scope boundaries remain intact. Independent preflight is required before activation; author never self-certifies PREFLIGHT PASS.
