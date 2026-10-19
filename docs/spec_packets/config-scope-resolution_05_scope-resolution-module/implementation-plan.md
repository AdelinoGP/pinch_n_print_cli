# Implementation Plan: scope-resolution-module

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-566`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Reconcile both FORWARD-DEPs before editing; no compatibility guess is permitted.
- Every cargo invocation is delegated and tee'd to `target/test-output.log`.

## Steps

### Step 1: Reconcile prerequisites and freeze call-site inventories

- Task IDs: `TASK-566`
- Objective: prove packets 03/04 landed with consumable shapes and capture the old-resolver/WIT blast radii.
- Precondition: packets 03 and 04 are implemented in the working tree.
- Postcondition: bounded inventories name every old resolver caller and every layer-planning v1 package/signature assertion; dependency shapes are recorded without code edits.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-config/src/**` - symbol definitions only
  - `docs/spec_packets/config-scope-resolution_03_typed-scope-ingestion/{packet.spec.md,design.md}` - exports only
  - `docs/spec_packets/config-scope-resolution_04_automatic-value-expansion/{packet.spec.md,design.md}` - exports only
- Files allowed to edit (at most 3):
  - None (read-only discovery step).
- Files explicitly out of bounds:
  - Plan, packets 01–04, `OrcaSlicerDocumented/`, generated bindings, artifacts.
- Expected sub-agent dispatches:
  - Question: report landed packet-03/04 symbol shapes and any mismatch; scope: paths above; return: `FACT: <5 lines or fewer>`.
  - Question: locate all six removed resolver definitions/calls; scope: `crates/**/*.rs`; return: `LOCATIONS: <at most 20 file:line entries, one context line each>`.
  - Question: locate every layer-planning v1 identity/signature site; scope: `crates/**`, `modules/core-modules/layer-planner-default/**`; return: `LOCATIONS: <at most 20 file:line entries, one context line each>`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - Resolution and queue row 5.
- OrcaSlicer refs:
  - None; canonical inspection is deferred to the docs/seam step.
- Verification:
  - `rg -n "fn (overlay_resolved|apply_overlay|resolve_global_config|resolve_per_object_configs|resolve_per_paint_semantic_configs|resolve_per_tool_configs)|slicer:prepass-layer-planning@1\.0\.0|layer-planning@1\.0\.0" crates modules/core-modules/layer-planner-default` - return bounded LOCATIONS, not raw output.
- Exit condition: every prerequisite mismatch is resolved or raised as a packet-blocking finding before implementation begins, and both inventories are complete.

### Step 2: Drive and implement the unified resolver

- Task IDs: `TASK-566`
- Objective: add independently expected tests, `ResolvedObjectLayerConfig`, `ResolutionTarget`, `ResolutionError`, `resolve_scope_stack`, and `query_z_grid`.
- Precondition: Step 1 confirms dependency shapes.
- Postcondition: the new module resolves explicit-default overrides, deterministic scope order, Phase-B expansion, sorted Z-grid records, and atomic invalid-height failure.
- Files allowed to read, with ranges when over 300 lines:
  - Landed ingestion/expansion modules - public APIs only
  - `crates/slicer-ir/src/resolved_config.rs` - relevant setters/fields only
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/resolution.rs`
  - `crates/slicer-config/src/lib.rs`
  - `crates/slicer-config/tests/scope_resolution_tdd.rs`
- Files explicitly out of bounds:
  - Scheduler/runtime/core call sites; WIT; layer-range source/types.
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `ResolvedObjectLayerConfig` is net-new, so there are no pre-existing struct literals. Tests must use `..` FRU where future fields are irrelevant or an exhaustive waiver where exact five-field shape is the assertion.
- Expected sub-agent dispatches:
  - Question: run the new test binary and return verdict; scope: workspace command; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` - scope deltas and Phase-B placement.
  - `docs/22_test_quality.md` - independent expectations.
- OrcaSlicer refs:
  - None; no layer-range behavior is implemented.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_resolution_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
- Exit condition: all resolver tests pass with literal expected values, and no test computes expected output through `resolve_scope_stack` or `query_z_grid`.

### Step 3: Migrate scheduler resolution and preserve rejection coverage

- Task IDs: `TASK-566`
- Objective: route scheduler resolution tests/exports through `slicer-config` and remove the five old scheduler functions.
- Precondition: Step 2 API is green.
- Postcondition: no production or test caller imports the old functions; existing type/bounds/alias rejection remains covered through the unified API.
- Files allowed to read, with ranges when over 300 lines:
  - Step-1 old-resolver call inventory only
  - `crates/slicer-scheduler/tests/integration/main.rs` - registrations only
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/config_resolution.rs`
  - `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`
  - `crates/slicer-scheduler/tests/integration/config_resolution_paint_semantic_tdd.rs`
- Files explicitly out of bounds:
  - Runtime/core callers; WIT; unrelated scheduler validation.
- Expected sub-agent dispatches:
  - Question: run scheduler integration config-resolution tests; scope: command below; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - config namespace and precedence sections.
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-scheduler --all-targets --test scheduler_integration config_resolution -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
- Exit condition: scheduler tests preserve all previous rejection assertions and `rg` finds none of the five removed function definitions.

### Step 4: Replace region overlays and converge runtime entry points

- Task IDs: `TASK-566`
- Objective: make RegionMapping and both setup paths consume the two unified queries.
- Precondition: Step 3 removed scheduler ownership.
- Postcondition: modifier/paint/tool composition uses `resolve_scope_stack`; both entry points build identical typed Z-grid records without formatted object prefixes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` - only `run_slice_with_collector` and `prepare_prepass_context`
  - `crates/slicer-runtime/src/prepass.rs` - configured resolution/RegionMapping sections only
  - `crates/slicer-core/src/algos/region_mapping.rs` - `overlay_resolved` callers only
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/region_mapping.rs`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/src/prepass.rs`
- Files explicitly out of bounds:
  - WIT and guests; model loader; layer-range implementation.
- Expected sub-agent dispatches:
  - Question: verify old call-site inventory is empty in production; scope: `crates/{slicer-core,slicer-runtime}/src/**/*.rs`; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - LayerPlanning and RegionMapping sections.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo check -p slicer-core -p slicer-runtime --all-targets` - FACT pass/fail.
  - `! rg -n "overlay_resolved|resolve_(global_config|per_object_configs|per_paint_semantic_configs|per_tool_configs)|format!\(\"(object_height|object_config):" crates/slicer-core/src crates/slicer-runtime/src` - FACT pass/fail.
- Exit condition: production sources contain neither old resolver calls nor the two object-prefix format sites, and both crates check with all targets.

### Step 5: Add the runtime convergence regression

- Task IDs: `TASK-566`
- Objective: prove both production entry points produce the same hand-authored resolution results.
- Precondition: Step 4 compiles.
- Postcondition: a registered integration test exercises both paths and independently asserts all five planning fields plus representative region values.
- Files allowed to read, with ranges when over 300 lines:
  - Existing runtime integration harness modules - fixture constructors only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/scope_resolution_module_tdd.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `crates/slicer-runtime/tests/common/perimeter_harness.rs`
- Files explicitly out of bounds:
  - Production code; external fixtures; visual-debug tests.
- Expected sub-agent dispatches:
  - Question: run only the new integration module; scope: command below; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/22_test_quality.md` - production oracle and independent expectations.
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration scope_resolution_module_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
- Exit condition: the test fails if either entry point bypasses the unified API or any expected planning/region value changes.

### Step 6: Define layer-planning WIT v2 and update schema/macro glue

- Task IDs: `TASK-566`
- Objective: declare the exact typed record/new parameter and update package identity plus guest adapter generation.
- Precondition: typed host record shape is green in Step 2.
- Postcondition: canonical WIT is `@2.0.0`; schema and macro glue expose the five-field record and new argument.
- Files allowed to read, with ranges when over 300 lines:
  - Step-1 WIT blast-radius inventory
  - `docs/03_wit_and_manifest.md` - prepass package table only
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`
  - `crates/slicer-schema/src/lib.rs`
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out of bounds:
  - Generated bindings; host dispatch; guests; other WIT packages.
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `object-layer-config` is net-new; Step 1's package/signature inventory is the compile/assertion blast radius. This step updates declaration/generation; Steps 7–9 own every concrete constructor and old-version assertion before the coordinated gate.
- Expected sub-agent dispatches:
  - Question: type-check schema/macros tests and return first concrete error only; scope: command below; return: `FACT: <5 lines or fewer>` or `SNIPPETS: <at most 3 verbatim snippets, 30 lines each, with file:line>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - adding an export parameter requires a major bump.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo check -p slicer-schema -p slicer-macros --all-targets` - FACT pass/fail; expected to remain incomplete only at downstream constructors owned by Steps 7–9, not in these crates.
- Exit condition: WIT declares exactly five record fields, `run` has the new list parameter, and schema metadata contains only `@2.0.0` for this package.

### Step 7: Update SDK and default layer-planner guest

- Task IDs: `TASK-566`
- Objective: adapt the SDK trait and default guest to typed per-object records, deleting config-prefix helpers.
- Precondition: Step 6 WIT/glue shape exists.
- Postcondition: the default planner consumes typed values only; its functional tests construct records directly.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/traits.rs` - `run_layer_planning` only
  - `modules/core-modules/layer-planner-default/src/lib.rs` - constructor/run/helper sections only
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/traits.rs`
  - `modules/core-modules/layer-planner-default/src/lib.rs`
  - `modules/core-modules/layer-planner-default/tests/layer_planning_tdd.rs`
- Files explicitly out of bounds:
  - Host dispatch; other guests; WIT declaration.
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Update every `PrepassModule::run_layer_planning` implementation reported by Step 1 or add a compatibility adapter in the trait macro-owned path; do not wait for broad check to discover implementations.
- Expected sub-agent dispatches:
  - Question: run the default planner's test target; scope: command below; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/05_module_sdk.md` - prepass module trait contract, relevant range only.
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p layer-planner-default --all-targets --test layer_planning_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
  - `! rg -n "fn (object_layer_height|object_height)|format!\(\"(layer_height|object_height):" modules/core-modules/layer-planner-default/src/lib.rs` - FACT pass/fail.
- Exit condition: planner tests pass using typed records and no guest source formats or parses host config namespaces.

### Step 8: Update host dispatch and test guest

- Task IDs: `TASK-566`
- Objective: marshal host `ResolvedObjectLayerConfig` values into WIT v2 and prove typed component dispatch.
- Precondition: Steps 6 and 7 define both sides.
- Postcondition: host dispatch passes object IDs and matching typed records; test guest accepts the new argument; a contract test rejects mismatched/omitted data before guest planning output is accepted.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/dispatch.rs` - LayerPlanning arm only
  - `crates/slicer-wasm-host/src/host.rs` - layer-planning bindgen/re-exports only
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/test-guests/prepass-layer-planning-guest/src/lib.rs`
  - `crates/slicer-wasm-host/tests/contract/prepass_layer_planning_v2_tdd.rs`
- Files explicitly out of bounds:
  - Other dispatch arms/guests; generated bindings; runtime tests.
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - Construct all five generated WIT fields exhaustively in dispatch. The new test guest signature and contract fixture are part of the same step.
- Expected sub-agent dispatches:
  - Question: build/check the affected host and guest after freshness handling; scope: relevant commands; return: `FACT: <5 lines or fewer>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - typed-instantiation compatibility contract.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo check -p slicer-wasm-host --all-targets` - FACT pass/fail.
- Exit condition: host dispatch compiles with all five fields and the test guest's v2 `run` signature, with no v1 fallback branch.

### Step 9: Update package assertions and register the WIT contract test

- Task IDs: `TASK-566`
- Objective: finish the package/version assertion blast radius and make the contract test runnable in the real bucket.
- Precondition: Step 8 added the contract test.
- Postcondition: all hard-coded layer-planning exports assert `@2.0.0`; the test is registered; no `@1.0.0` identity remains.
- Files allowed to read, with ranges when over 300 lines:
  - Step-1 package assertion inventory only
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/tests/contract/main.rs`
  - `crates/slicer-macros/tests/binding_surface_tdd.rs`
  - `modules/core-modules/layer-planner-default/tests/slicer_module_binding_tdd.rs`
- Files explicitly out of bounds:
  - Production code; other package assertions.
- Expected sub-agent dispatches:
  - Question: run macro binding and wasm-host contract tests; scope: commands below; return: `FACT: <5 lines or fewer>`.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - qualified export identity.
- OrcaSlicer refs:
  - None.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-macros --all-targets --test binding_surface_tdd prepass_layer_planning_reports_prepass_world -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q "test prepass_layer_planning_reports_prepass_world .* ok" target/test-output.log'` - FACT pass/fail.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --all-targets --test contract prepass_layer_planning_v2_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
- Exit condition: `rg -n "slicer:prepass-layer-planning@1\.0\.0|layer-planning@1\.0\.0" crates modules/core-modules/layer-planner-default` returns no matches.

### Step 10: Document the unified seam and settled future overlap rule

- Task IDs: `TASK-566`
- Objective: align architecture/WIT/scheduler docs without implying row-9 code exists.
- Precondition: implementation and exact package shape are green.
- Postcondition: docs name both APIs, the five-field record, v2 identity, total precedence, and the exact deferred overlap/error policy.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - Config Key Namespaces and modifier resolution only
  - `docs/03_wit_and_manifest.md` - package inventory and prepass interfaces only
  - `docs/04_host_scheduler.md` - LayerPlanning and RegionMapping only
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/04_host_scheduler.md`
- Files explicitly out of bounds:
  - Plan, ADRs, packet directories, code.
- Expected sub-agent dispatches:
  - Question: verify canonical later-starting-wins behavior only; scope: `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp`; return: `SUMMARY: <at most 200 words, no code unless requested>`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - precedence and overlap rules.
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` - resolution ownership.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` - delegate `layer_height_profile_from_ranges`/`layer_height_profile` behavior; cite function names only.
- Verification:
  - Run `AC-6` and `AC-N2` commands - FACT pass/fail.
- Exit condition: docs pass both checks and explicitly say row 9, not row 5, implements and verifies layer-range overlap handling.

### Step 11: Coordinated guest freshness and closure gates

- Task IDs: `TASK-566`
- Objective: validate the complete change without stale artifacts or hidden targets.
- Precondition: Steps 2–10 pass narrowly.
- Postcondition: guest artifacts are fresh and all packet-level gates pass.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - failure snippets only
- Files allowed to edit (at most 3):
  - Guest `.component.wasm` artifacts rebuilt by `cargo xtask build-guests` only when `--check` reports stale.
- Files explicitly out of bounds:
  - Source changes unrelated to a concrete gate failure; lockfiles unless lock convergence explicitly reports stale and the coordinator approves `--sync-locks`.
- Expected sub-agent dispatches:
  - Question: run freshness, targeted matrices, check, clippy, literal, and test-quality gates; scope: commands in `requirements.md`; return: `FACT: <5 lines or fewer>` per command.
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - artifact-verified WIT compatibility.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo xtask build-guests --check` - FACT exit code; if stale, run `cargo xtask build-guests`, then re-run `--check`.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask check-literals` and `cargo xtask check-test-quality --report` - FACT pass/fail/findings.
- Exit condition: freshness exits 0, every AC command passes, all-target check/clippy pass, and touched files have no unaddressed literal/test-quality finding.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Bounded prerequisite/blast-radius inventories |
| Step 2 | M | New resolver and independent oracle |
| Step 3 | M | Scheduler migration/rejection preservation |
| Step 4 | M | Core/runtime production convergence |
| Step 5 | M | Production-path regression |
| Step 6 | M | WIT declaration and generated glue source |
| Step 7 | M | SDK/default guest adaptation |
| Step 8 | M | Host dispatch/test guest |
| Step 9 | S | Package assertions/registration |
| Step 10 | S | Bounded docs and canonical summary |
| Step 11 | M | Delegated closure gates |

Aggregate remains `M` because steps are sequential and each dispatch returns bounded facts; no step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile packets 03/04 from FORWARD-DEP to landed dependencies.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk, especially WIT v2 ecosystem compatibility and the intentionally deferred row-9 range path.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations use `--all-targets` so test, bench, and example targets compile.
