# Implementation Plan: authored-value-oracle

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-563`.
- This is test-only TDD: the principal behavior test must close red, while its controls close green.
- Reconcile packet 01 before editing; do not edit predecessor/dependent packets or production code outside the `requirements.md` retype exception (the two registry-assembly retypes plus their census follow-through and lock refresh are owned here).
- Delegate cargo runs and fixture inspection with bounded returns; every test run tees combined output to `target/test-output.log`.

## Steps

### Step 1: Reconcile the registry contract and prove the observation matrix is driveable

- Task IDs: `TASK-563`
- Objective: verify packet-01's landed API, current dependency state, exact fixture facts, manifest-derived ownership, and the arachne/classic module keep/drop sets before authoring the oracle.
- Precondition: packet 01 has landed; packet 02 remains `draft` and no in-scope file has been edited.
- Postcondition: a bounded fact record confirms the complete `RegistryEntry` shape including `values`, confirms `zip` already exists and `slicer-config` is absent from runtime dev-dependencies, and confirms both exact perimeter owner IDs can be exposed by changing only `wall_generator`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/config-scope-resolution_01_config-schema-registry/design.md` — §Code Change Surface only.
  - `crates/slicer-config/src/lib.rs` — named public type/function definitions only.
  - `crates/slicer-runtime/Cargo.toml` — dependency tables only.
  - `crates/slicer-scheduler/src/execution_plan.rs` — `dedup_same_claim_modules_with_wall_generator`, `bind_module_config_view`, and `CompiledModuleStatic` only.
  - `crates/slicer-runtime/src/run.rs` — `PrepassContext` and `prepare_prepass_context` only.
  - `resources/cube_4color.3mf` and `modules/core-modules/*/*.toml` — programmatic bounded derivation only.
- Files allowed to edit (at most 3):
  - None; this is a read-only decision gate.
- Files explicitly out of bounds:
  - packet 01/03/04 files, approved-plan queue, backlog, source edits, fixture edits, `Cargo.lock`, and `target/` reads.
- Blast-radius discipline: no struct field or schema/version constant changes; the step records the complete promised shape rather than constructing literals.
- Expected sub-agent dispatches:
  - Question: verify the exact packet-01 exports and signatures listed in `design.md`; scope: named symbols in `crates/slicer-config/src/lib.rs`; return: `FACT` ≤5 lines.
  - Question: report existing runtime dev-dependencies for `zip`, `serde_json`, `slicer-model-io`, and `slicer-config`; scope: `crates/slicer-runtime/Cargo.toml`; return: `FACT` ≤5 lines.
  - Question: derive exact declaring module IDs for the five catalogued keys and arachne/classic keep/drop sets; scope: loaded core manifests plus production dedup; return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — Packet Queue rows 1-3 and registry/ingestion decisions.
  - `docs/04_host_scheduler.md` — perimeter and support claim selection sections.
  - `docs/22_test_quality.md` — derivation checklist.
- OrcaSlicer refs:
  - None; no OrcaSlicer source behavior is being ported.
- Verification:
  - `rg -n 'pub struct RegistryEntry|pub values: Option<Vec<String>>|pub struct ModuleDeclaration|pub struct HostChannels|pub struct AssemblyOutcome|pub fn assemble_registry' crates/slicer-config/src/lib.rs` — `LOCATIONS` ≤20, then compare every field/signature to `design.md`.
  - `rg -n '^zip =|^serde_json =|^slicer-model-io =|^slicer-config =' crates/slicer-runtime/Cargo.toml` — FACT: first three present, `slicer-config` absent.
- Exit condition: stop if packet 01 has not landed, any promised shape differs, `zip` is absent, `slicer-config` is already present, or either perimeter owner cannot be exposed through the production selector path; amend this packet before implementation rather than improvising.

### Step 2: Author and register the red oracle with three green controls

- Task IDs: `TASK-563`
- Objective: implement the independent raw-value derivation, manifest/registry ownership, two-plan selector matrix, exact owner observation, principal red assertion, and selector/population/comparator controls.
- Precondition: Step 1 confirms the exact API/dependency state and driveability facts.
- Postcondition: the executor lists exactly four oracle tests; the principal test fails only with sorted `MISMATCH` diagnostics and no `MISSING_MODULE`; the other three tests pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/src/loader.rs` — `read_3mf_project_settings` and `coerce_string_to_config_value` only.
  - `crates/slicer-runtime/src/run.rs` — `PrepassContext` and `prepare_prepass_context` only.
  - `crates/slicer-scheduler/src/manifest.rs` — `LoadedModule` accessors and `load_modules_from_roots` only.
  - `crates/slicer-scheduler/src/execution_plan.rs` — `bind_module_config_view`, dedup function, and compiled accessors only.
  - `crates/slicer-ir/src/slice_ir.rs` — `ConfigValue` and `ConfigView::get` only.
  - `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` — named compiled-view observation test only.
  - `crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs` — named multi-owner binding test only.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs`
  - `crates/slicer-runtime/tests/executor/main.rs`
  - `crates/slicer-runtime/Cargo.toml`
- Retype files additionally authorized (the `requirements.md` Out-of-Scope exception; registry-assembly prerequisites owned here, packet 03 keeps the ingestion fix):
  - `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-ir/src/feedrate.rs`, `crates/slicer-core/src/flow.rs`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, `crates/slicer-gcode/src/emit.rs`, `crates/slicer-config/src/lib.rs`, `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-config/tests/registry_census_tdd.rs`, `crates/slicer-config/tests/registry_assembly_tdd.rs`, `crates/slicer-ir/tests/{resolved_config_defaults_tdd.rs,feedrate_default_tdd.rs,feedrate_from_raw_config_tdd.rs}`, `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (speed-typed `ConfigViewBuilder` fixture), `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`, `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`, `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`, and `crates/slicer-sdk/src/test_support/fixtures.rs`.
- Files explicitly out of bounds:
  - module manifests/sources, fixtures, packet 01/03/04, approved plan, backlog, and generated artifacts.
- Blast-radius discipline: no existing public struct gains a field and no schema/version constant changes, except the two retypes named in the `requirements.md` exception (`initial_layer_line_width`, `internal_bridge_speed`). Any `ModuleDeclaration` test literal uses FRU or an `// exhaustive:` waiver if required by `check-literals`; `RegistryEntry` is borrowed and never reconstructed.
- Expected sub-agent dispatches:
  - Question: return the smallest existing fixture-path/mesh-load and compiled-view access idioms; scope: the two named neighboring tests; return: `SNIPPETS` ≤3 snippets/30 lines.
  - Question: run AC-5 list command before behavioral tests; scope: executor binary; return: `FACT` ≤5 lines.
  - Question: run AC-1 through AC-4 and report exact pass/fail plus only mismatch/missing-module lines; scope: executor oracle filter; return: `FACT` ≤5 lines or `SNIPPETS` ≤20 lines on failure.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — row-2 population and red-oracle contract.
  - `docs/22_test_quality.md` — independent oracle, roster, vacuity, scaffolding, and negative-control rules.
  - `docs/04_host_scheduler.md` — real selector matrix semantics.
- OrcaSlicer refs:
  - None.
- Verification:
  - `packet.spec.md` AC-5 command — FACT exactly four tests registered.
  - `packet.spec.md` AC-1 command — FACT accepted red with all seven exact owner/value lines and no `MISSING_MODULE`.
  - `packet.spec.md` AC-2, AC-3, and AC-4 commands — FACT green for each control.
  - `cargo test -p slicer-config --test registry_census_tdd` — FACT census green (4 passed) with the retyped speed field type.
  - `cargo test -p slicer-config --test registry_assembly_tdd` — FACT assembly green (21 passed) with the retyped host/speed channels.
  - `cargo test -p slicer-ir --test feedrate_from_raw_config_tdd && cargo test -p slicer-ir --test feedrate_default_tdd && cargo test -p slicer-ir --test resolved_config_defaults_tdd && cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd` — FACT retyped-value focused tests green.
- Exit condition: stop if the principal test passes, fails before comparison, omits any exact AC-1 line, reports a missing owner, or any control fails; do not weaken the population, ownership join, or comparator to obtain the intended red state.

### Step 3: Enforce anti-false-green and compile/lint closure gates

- Task IDs: `TASK-563`
- Objective: prove the red test is unsuppressed, roster-free, test-quality compliant, and compilable across all targets without changing production behavior.
- Precondition: Step 2 has the exact red/green split and all changes remain within the three authorized files.
- Postcondition: AC-N1/AC-N2, all-target check/clippy, literal, and touched-test quality gates satisfy the packet contract; packet remains `draft` until independent preflight/activation.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs` — whole touched test file.
  - `crates/slicer-runtime/tests/executor/main.rs` — module list only.
  - `crates/slicer-runtime/Cargo.toml` — dev-dependencies only.
  - `docs/21_data_defaults_and_fixtures.md` — struct-literal rule and waiver format only.
  - `docs/22_test_quality.md` — negative-control and gate sections only.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs`
  - `crates/slicer-runtime/tests/executor/main.rs`
  - `crates/slicer-runtime/Cargo.toml`
- Retype files additionally authorized (same `requirements.md` exception as Step 2; edits here are limited to closure fallout of the retypes):
  - `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-ir/src/feedrate.rs`, `crates/slicer-core/src/flow.rs`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, `crates/slicer-gcode/src/emit.rs`, `crates/slicer-config/src/lib.rs`, `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-config/tests/registry_census_tdd.rs`, `crates/slicer-config/tests/registry_assembly_tdd.rs`, `crates/slicer-ir/tests/{resolved_config_defaults_tdd.rs,feedrate_default_tdd.rs,feedrate_from_raw_config_tdd.rs}`, `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` (speed-typed `ConfigViewBuilder` fixture), `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`, `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`, `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`, and `crates/slicer-sdk/src/test_support/fixtures.rs`.
- Doc-generator files additionally authorized (same `requirements.md` exception; `xtask/src/gen_config_docs.rs` honors an optional per-key `type` before scalar inference, and doc 15's host-speeds table is generated output):
  - `xtask/src/gen_config_docs.rs`, `docs/config/host-keys.toml`, and the regenerated `docs/15_config_keys_reference.md`.
- Files explicitly out of bounds:
  - fixture/manifest changes, other packet directories, approved plan, backlog, and generated artifacts — except the three doc-generator paths named immediately above.
- Blast-radius discipline: fixes may only improve the test/registration/dev-dependency implementation or close retype fallout from the `requirements.md` exception; any further finding that requires production or contract edits blocks this packet and is forwarded to packet 03.
- Expected sub-agent dispatches:
  - Question: run AC-N1 and AC-N2; scope: oracle source and exact principal test; return: `FACT` ≤5 lines.
  - Question: run all-target check/clippy and literal/test-quality gates; scope: workspace compile plus touched tests; return: `FACT` ≤5 lines, with `SNIPPETS` ≤20 lines only on failure.
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` — watched literal discipline.
  - `docs/22_test_quality.md` — no false-green patterns.
  - `AGENTS.md` — all-target and test-log rules.
- OrcaSlicer refs:
  - None.
- Verification:
  - `packet.spec.md` AC-N1 command — FACT unsuppressed and red.
  - `packet.spec.md` AC-N2 command — FACT no five-key Rust string roster.
  - `cargo test -p slicer-config --test registry_census_tdd && cargo test -p slicer-config --test registry_assembly_tdd` — FACT census (4 passed) and assembly (21 passed) suites green after the retypes.
  - `cargo check --workspace --all-targets` — FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail.
  - `cargo xtask check-literals` — FACT pass/fail.
  - `cargo xtask gen-config-docs --check` — FACT exit 0 after regenerating doc 15's host-speeds table.
  - `rg -n 'internal_bridge_speed' docs/15_config_keys_reference.md` — FACT the host-speeds (`feedrate.rs::FeedrateConfig`) row reports `float_or_percent`.
  - `cargo xtask check-test-quality --report` — FACT no unjustified touched-file findings.
- Exit condition: every gate has the expected result, only the principal oracle is red, no file outside the two authorized lists changed, and TASK-563 is ready for packet-close reporting without implementation of packet 03.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Forward API/dependency/driveability gate |
| Step 2 | M | Independent derivation plus two live selector plans |
| Step 3 | S | Anti-false-green and closure gates |

Aggregate remains `M`; no step is `L`.

## Packet Completion Gate

- All steps and falsifying exits complete.
- Every pipe-suffixed AC command returns PASS, where PASS for AC-1/AC-N1 means the exact principal test is red for value mismatches.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo xtask check-literals` passes; touched-file test-quality findings are fixed or carry a justified named waiver.
- Update `docs/07_implementation_status.md` only through the orchestrator's bounded worker dispatch.
- Confirm packet 01/03/04, approved plan, fixture, and manifests are unchanged by this packet; `Cargo.lock`/guest lockfiles change only through the authorized retype dependency resolution and refresh.
- `packet.spec.md` remains `draft` until the independent activation gate; packet 02's implementation completion does not turn the oracle green.

## Acceptance Ceremony

- Re-dispatch every AC and packet-level gate command.
- Record that the sole expected failing test is `oracle_authored_values_reach_owning_module_config_views` and retain its exact mismatch evidence for packet 03.
- Confirm both selector plans expose the exact intended perimeter owner and all eligible observations found exact module IDs.
- Record remaining packet-local risk and the packet-01 landed export versions without changing predecessor files.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations use `--all-targets` so test, bench, and example targets compile.
