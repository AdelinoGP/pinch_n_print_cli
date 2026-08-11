# Implementation Plan: support-interface-layers

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-325`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Each step has at most three edit paths; split work rather than hiding a fourth edit.

## Steps

### Step 1: Inventory contract fallout

- Task IDs: `TASK-325`
- Objective: enumerate all `SupportPlanIR` literals, schema assertions, WIT owners, SDK builder/types, macro conversion, host collection/marshal, and diagnostics.
- Precondition: current tree contains the grounded `1.3.0` constant and existing branch-only plan.
- Postcondition: every affected owner is assigned to a later step.
- Files allowed to read, with ranges: `crates/slicer-ir/src/slice_ir.rs:253-259,1144-1217`; `crates/slicer-sdk/src/prepass_types.rs:260-287`; WIT `:1-61`; macro `:2300-2343`; host `:1203-1218,4160-4195`; marshal `:709-768`; runtime/IR tests at grep locations.
- Files allowed to edit (at most 3): none; discovery only.
- Files explicitly out of bounds: generated WASM, `target/`, Orca sources.
- Expected sub-agent dispatches: Question: enumerate literal/assertion and transport owners; scope `crates/**/*.rs,modules/**/*.rs,crates/slicer-schema/wit/**/*.wit`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md` delegated sections.
- OrcaSlicer refs: none.
- Verification: `rg -n 'SupportPlanIR \{|CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION|push_support_plan_entry|SupportPlanEntry' crates modules` - bounded inventory.
- Exit condition: no affected literal, assertion, or transport owner remains unassigned.

### Step 2: Add IR field and schema minor bump

- Task IDs: `TASK-325`
- Objective: add `SupportInterfacePlanEntry`, `SupportInterfaceKind`, `SupportPlanIR.interface_plan`, defaulting, and bump `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` from `1.3.0` to `1.4.0`.
- Precondition: Step 1 inventory complete.
- Postcondition: IR and all explicit literals compile with the new field; old hard assertions are updated.
- Files allowed to read: `crates/slicer-ir/src/slice_ir.rs:253-259,1144-1217`; exact literal sites from Step 1.
- Files allowed to edit (at most 3): `crates/slicer-ir/src/slice_ir.rs`; `crates/slicer-ir/tests/ir_tests.rs`; `crates/slicer-runtime/tests/visual_debug_render_tap_tdd.rs`.
- Files explicitly out of bounds: WIT/SDK/host/macro until this shape is stable; generated WASM and `target/`.
- Blast-radius discipline: update every assertion of old `1.3.0` and every `SupportPlanIR { ... }` field list owned by this step; the two remaining runtime literal files are explicitly owned by Step 3 before any consumer work begins.
- Expected sub-agent dispatches: Question: confirm every literal and old-version assertion; scope Step 1 locations; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` delegated.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-ir --test ir_tests --all-targets`; `cargo test -p slicer-runtime --test visual_debug_render_tap_tdd --all-targets`.
- Exit condition: canonical name is singular, version is `1.4.0`, all literals/assertions compile, and no `SupportInterfacePlan` spelling exists.

### Step 3: Update runtime literal fallout

- Task IDs: `TASK-325`
- Objective: update every explicit runtime `SupportPlanIR` literal to include `interface_plan: Vec::new()` and retain the live `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION`.
- Precondition: Step 2 IR shape and version bump are complete.
- Postcondition: all runtime visual/debug and executor fixtures compile against the additive field.
- Files allowed to read: `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` at `SupportPlanIR` literal locations; `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` at `SupportPlanIR` literal locations.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`; `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`.
- Files explicitly out of bounds: WIT, SDK, host, macro, generated WASM, and `target/`.
- Expected sub-agent dispatches: Question: verify no additional explicit runtime literals remain; scope `crates/slicer-runtime/**/*.rs`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: `docs/02_ir_schemas.md` delegated.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-runtime --test visual_debug_blackboard_tap_tdd --all-targets`; `cargo test -p slicer-runtime --test live_layer_support_tdd --all-targets`.
- Exit condition: `rg -n 'SupportPlanIR \\{' crates/slicer-runtime/tests` returns only literals containing `interface_plan`.

### Step 4: Wire WIT and SDK output

- Task IDs: `TASK-325`
- Objective: add the WIT record/method and SDK `SupportInterfacePlanEntry`/kind plus builder storage/accessors.
- Precondition: Step 2 IR shape is stable.
- Postcondition: a guest can push a typed interface plan record through the SDK output builder.
- Files allowed to read: WIT `:1-61`; SDK `prepass_types.rs:260-287`; `prepass_builders.rs:294-360`.
- Files allowed to edit (at most 3): `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`; `crates/slicer-sdk/src/prepass_types.rs`; `crates/slicer-sdk/src/prepass_builders.rs`.
- Files explicitly out of bounds: host/macro/planner until the wire shape is defined; generated bindings.
- Expected sub-agent dispatches: Question: confirm generated WIT naming and enum conversion shape; scope WIT and SDK paths; return: `FACT`.
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` delegated.
- OrcaSlicer refs: none.
- Verification: `cargo check -p slicer-sdk --all-targets`; `rg -q 'support-interface-plan-entry|push-interface-plan|SupportInterfacePlanEntry' crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit crates/slicer-sdk/src`.
- Exit condition: WIT and SDK expose one typed `SupportInterfacePlanEntry` contract with kind, density, and spacing.

### Step 5: Wire macro and host transport

- Task IDs: `TASK-325`
- Objective: convert SDK plan data in the macro, collect the WIT record in host context, and marshal it into IR.
- Precondition: Step 4 wire shape exists.
- Postcondition: a pushed record survives host collection and `harvest_support_plan_ir` into `SupportPlanIR.interface_plan`.
- Files allowed to read: macro `crates/slicer-macros/src/lib.rs:2300-2343`; host `:1203-1218,4160-4195`; dispatch `:2232-2244`; marshal `:709-768`.
- Files allowed to edit (at most 3): `crates/slicer-macros/src/lib.rs`; `crates/slicer-wasm-host/src/host.rs`; `crates/slicer-wasm-host/src/marshal/in_.rs` (dispatch remains a read-only routing owner).
- Files explicitly out of bounds: generated bindings, planner, support consumers.
- Expected sub-agent dispatches: Question: verify host generated enum names and raw collection type; scope `crates/slicer-wasm-host/src`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` delegated.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-wasm-host --test prepass_output_builder_validation_tdd --all-targets`; `cargo check -p slicer-wasm-host --all-targets`.
- Exit condition: macro push, host collection, and marshal each use the canonical semantic name and preserve every field.

### Step 6: Emit planner records and remove warning

- Task IDs: `TASK-325`
- Objective: make `support-planner` emit top/bottom plan records and remove planner-side scan-line construction and code `1003`.
- Precondition: Steps 2-5 transport checks pass.
- Postcondition: planner output contains records only; no interface geometry or code `1003`.
- Files allowed to read: `modules/core-modules/support-planner/src/lib.rs:319-340,696-756,1382-1462`; diagnostics tests.
- Files allowed to edit (at most 3): `modules/core-modules/support-planner/src/lib.rs`; `modules/core-modules/support-planner/tests/diagnostics_tdd.rs`; `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`.
- Files explicitly out of bounds: support consumers and manifests until planner tests pass.
- Expected sub-agent dispatches: Question: identify existing config-to-layer and region eligibility helpers; scope support-planner; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: approved plan and verified findings; direct bounded sources above.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` delegated `SUMMARY`.
- Verification: `cargo test -p support-planner --test diagnostics_tdd --all-targets`; `cargo xtask build-guests --check`.
- Exit condition: no `push_interface_scan_lines(` call, top/bottom records have exact fields, and AC-N1 passes.

### Step 7: Consume plans in tree-support

- Task IDs: `TASK-325`
- Objective: generate planned top/bottom dense paths with precedence in tree-support.
- Precondition: planner and transport pass.
- Postcondition: tree-support emits only planned interface paths with correct role, flag, and spacing.
- Files allowed to read: `modules/core-modules/tree-support/src/lib.rs:130-210`; `tree-support/tests/interface_layers_tdd.rs`; SDK paint-view methods.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support/src/lib.rs`; `modules/core-modules/tree-support/tests/interface_layers_tdd.rs`; `modules/core-modules/tree-support/tree-support.toml` only if its existing read requires a contract correction.
- Files explicitly out of bounds: traditional-support and planner.
- Expected sub-agent dispatches: Question: confirm `SupportOutputBuilder::push_interface_path` and paint precedence signatures; scope `crates/slicer-sdk/src,modules/core-modules/tree-support`; return: `FACT`.
- Context cost: `M`
- Authoritative docs: `docs/01_system_architecture.md`, `docs/08_coordinate_system.md` delegated.
- OrcaSlicer refs: delegated `TreeSupport.cpp` `SUMMARY`.
- Verification: `cargo test -p tree-support --test interface_layers_tdd --all-targets`.
- Exit condition: tree tests prove top/bottom counts, booleans, role, spacing, and precedence.

### Step 8: Consume plans in traditional-support and declare manifest read

- Task IDs: `TASK-325`
- Objective: make traditional-support read `SupportPlanIR`, generate planned interfaces, and prove the same contract as tree-support.
- Precondition: Step 7 establishes the consumer test shape.
- Postcondition: traditional-support is eligible for the same plan and emits no unplanned interface fill.
- Files allowed to read: traditional source, manifest, and focused test; tree consumer only as a delegated shape reference.
- Files allowed to edit (at most 3): `modules/core-modules/traditional-support/src/lib.rs`; `modules/core-modules/traditional-support/traditional-support.toml`; `modules/core-modules/traditional-support/tests/interface_layers_tdd.rs`.
- Files explicitly out of bounds: tree-support and planner.
- Expected sub-agent dispatches: Question: verify module manifest access enforcement and current support-plan view; scope traditional-support and scheduler contract tests; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: `docs/01_system_architecture.md`, `docs/03_wit_and_manifest.md` delegated.
- OrcaSlicer refs: delegated `TreeSupport.cpp` `SUMMARY`.
- Verification: `cargo test -p traditional-support --test interface_layers_tdd --all-targets`; `rg -q 'reads.*SupportPlanIR' modules/core-modules/traditional-support/traditional-support.toml`.
- Exit condition: traditional-support manifest and implementation consume `SupportPlanIR`, and its focused tests pass.

### Step 9: Author visual fixture and IR documentation

- Task IDs: `TASK-325`
- Objective: add exact interface config/request and update the SupportPlanIR documentation.
- Precondition: Steps 6-8 pass and guests are fresh.
- Postcondition: the exact visual request selects bottom `0` and top `125`, and the IR schema documentation names `SupportInterfacePlanEntry`.
- Files allowed to read: existing support request/config fixtures and the three delegated docs.
- Files allowed to edit (at most 3): `tmp/support-config-interface.json`; `tmp/visual-debug-support-interface.json`; `docs/02_ir_schemas.md`.
- Files explicitly out of bounds: generated output, Orca sources, unrelated docs.
- Expected sub-agent dispatches: Question: identify exact documentation insertion anchors and visual request schema; scope `docs/01_system_architecture.md,docs/02_ir_schemas.md,docs/03_wit_and_manifest.md,tmp/*.json`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: the three packet doc-impact sections; delegated relevant ranges.
- OrcaSlicer refs: none.
- Verification: `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-interface.json --output target/vd-support-interface --overwrite >/dev/null && jq -e '([.images[] | select(.tap == "Layer::Support" and .layer_index == 0) | .typed_capture.value.interface_paths[]? | select(.[1] == false)] | length > 0) and ([.images[] | select(.tap == "Layer::Support" and .layer_index == 125) | .typed_capture.value.interface_paths[]? | select(.[1] == true)] | length > 0)' target/vd-support-interface/manifest.json`; `rg -q 'SupportInterfacePlanEntry' docs/02_ir_schemas.md`.
- Exit condition: AC-5's request is authored with exact layers and the IR schema anchor exists.

### Step 10: Close architecture and manifest documentation

- Task IDs: `TASK-325`
- Objective: document the planner/module boundary and the `traditional-support` `SupportPlanIR` manifest read.
- Precondition: Steps 6-9 pass.
- Postcondition: architecture and WIT/manifest docs contain the packet's exact contract anchors.
- Files allowed to read: relevant sections of `docs/01_system_architecture.md` and `docs/03_wit_and_manifest.md`.
- Files allowed to edit (at most 3): `docs/01_system_architecture.md`; `docs/03_wit_and_manifest.md`.
- Files explicitly out of bounds: generated output, Orca sources, unrelated docs.
- Expected sub-agent dispatches: Question: identify exact architecture and manifest insertion anchors; scope `docs/01_system_architecture.md,docs/03_wit_and_manifest.md`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: packet doc-impact section; delegated relevant ranges.
- OrcaSlicer refs: none.
- Verification: `rg -q 'interface_plan' docs/01_system_architecture.md && rg -q 'traditional-support.*SupportPlanIR|SupportPlanIR.*traditional-support' docs/03_wit_and_manifest.md`; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`.
- Exit condition: both documentation greps and workspace gates pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | inventory |
| Step 2 | M | IR/version and literal fallout |
| Step 3 | S | runtime literal fallout |
| Step 4 | M | WIT/SDK |
| Step 5 | M | macro/host |
| Step 6 | M | planner |
| Step 7 | M | tree consumer |
| Step 8 | M | traditional consumer/manifest |
| Step 9 | S | fixture and IR docs |
| Step 10 | S | architecture/manifest docs |

The packet is intentionally sequenced; each M step is independently bounded even though the aggregate is M under the packet context convention.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is updated through a worker dispatch.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC and packet-level gate command.
- Record remaining fixture risk.
- Confirm context stayed within the standard band.

All cargo commands use `--all-targets` where applicable.
