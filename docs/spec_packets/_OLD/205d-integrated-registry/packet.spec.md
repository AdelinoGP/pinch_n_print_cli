---
status: implemented
packet: 205d-integrated-registry
task_ids:
  - TASK-330
backlog_source: docs/specs/integrated-modules-architecture-205c-205e-plan.md (queue row 2)
context_cost_estimate: M
---

# Packet Contract: 205d-integrated-registry

## Goal

Derive integrated manifest registrations, native entries, and coverage checks from one registry authority while preserving all 21 feature names, module IDs, origin labels, stage families, and edition behaviour.

## Scope Boundaries

This packet owns the repetition inside `slicer-integrated-modules` and the tests that assert its registry invariants. It may add a registry table or declarative macro and update the generated registration/entry/coverage surfaces. It does not change module features, pnp-cli passthrough feature names, edition membership, loader priority, or dispatch routing.

## Prerequisites and Blockers

- Depends on: `205c-native-dispatch-seam` draft; activation requires 205c to be implemented. 205b's 21-module feature/entry surface remains the input contract.
- Unblocks: 205e's parity harness can consume a stable registry inventory.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the full 21-feature build of `slicer-integrated-modules`, **when** the registry is compiled, **then** `integrated_registrations()` and `native_entries()` are both derived from one registry authority and expose exactly the same 21 module IDs, with no second hand-maintained per-module push list. | `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set 2>&1 | rg -q '^test result: ok'`
- **AC-2. Given** each integrated registry row, **when** its manifest and entry are inspected, **then** the manifest ID is `com.core.<name>`, its origin label is `integrated://<name>`, and its `NativeStageEntry` variant matches the manifest stage family (`Layer`, `Prepass`, `Finalization`, or `Postpass`). | `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_native_entry_families_match_stage_ids 2>&1 | rg -q '^test result: ok'`
- **AC-3. Given** the developer, hybrid, and integrated edition feature combinations, **when** the registry is built, **then** the default feature set remains empty, the hybrid pilot remains the three existing modules, and the integrated edition can still enable all 21 modules through the existing `integrated-<name>` passthrough features. | `cargo test -p xtask editions_config_declares_three_editions 2>&1 | rg -q '^test result: ok'`
- **AC-4. Given** a new registry row is added in a future change, **when** the registry coverage tests run, **then** the row cannot be present in only registrations or only native entries; both generated surfaces and the stage-family assertion must include it. | `rg -q 'full_coverage_registrations_match_registered_set' crates/slicer-integrated-modules/src/lib.rs && rg -q 'full_coverage_native_entry_families_match_stage_ids' crates/slicer-integrated-modules/src/lib.rs && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a registry row whose manifest ID and native-entry ID differ, **when** full coverage tests run, **then** the test fails with the mismatched ID rather than silently accepting a partial registry. | `rg -q 'assert_eq!\(actual, expected\)' crates/slicer-integrated-modules/src/lib.rs && echo PASS`
- **AC-N2. Given** an external module with the same ID as an integrated registry row, **when** live modules are loaded, **then** the external module still has `native_entry: None` and a WASM component, preserving ADR-0056 override semantics. | `cargo test -p slicer-runtime --test integration --all-targets -- full_coverage_external_override_forces_wasm 2>&1 | rg -q '^test result: ok'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-integrated-modules --all-targets --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower`

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` - direct read of manifest/entry identity and override rules.
- `docs/adr/0057-three-editions-and-integrated-tier.md` - direct read of edition membership and feature semantics.
- `docs/spec_packets/205b-native-transport-completion/packet.spec.md` - direct read of the existing 21-module registry contract.
- `CONTEXT.md` - delegated lookup for integrated/external module terminology.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/07_implementation_status.md` - mark `TASK-330` complete when this packet closes; verify with `rg -q 'TASK-330' docs/07_implementation_status.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
