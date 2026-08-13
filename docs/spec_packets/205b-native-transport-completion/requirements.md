# Requirements: 205b-native-transport-completion

## Packet Metadata

- Grouped task IDs: `ADR-0056`, `ADR-0057`; no `docs/07_implementation_status.md` TASK row exists for this program.
- Backlog source: `docs/specs/multi-edition-distribution-plan.md` §"Also unscheduled".
- Packet status: `implemented`
- Aggregate context cost: `M`

## Problem Statement

Packet 202 left the `Layer::PathOptimization` output commit and postpass gcode-command application as fatal native transport errors. Packet 205a integrates every other committable module, so the two remaining modules cannot enter the Integrated edition until these transports are complete and proven equivalent to wasm.

## In Scope

- Native path-optimization output commit and postpass gcode-command application in `crates/slicer-wasm-host/src/marshal/native.rs`.
- Feature-gated registry entries and native entries for `path-optimization-default` and `machine-gcode-emit`.
- One independent dual-dispatch parity contract test per module.
- External override integration coverage and matching `pnp-cli` passthrough features.
- Closure proof that `cargo xtask dist --edition integrated` plans all integrated modules without external staging.

## Out of Scope

- Geometry call sites in either module, dispatch routing, macro emission, CLI behavior, edition membership, WIT schema, or `docs/07`.
- Any existing packet 206 or 207 directory.
- Platform builds and unrelated module changes.

## Acceptance Summary

- Positive: `AC-1` through `AC-8` in `packet.spec.md`; registration and native-entry counts must follow the existing registry contract, and parity must be independent per module.
- Negative: `AC-N1` through `AC-N3`; comparators must reject dropped paths, external overrides must force wasm, and the coverage gate must still reject an uncovered feature.
- Cross-packet impact: only the two registry features and two CLI passthrough features are added; `dist/editions.toml` remains unchanged.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set` | AC-1 | FACT pass/fail |
| `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_native_entry_families_match_stage_ids` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- integrated_parity_path_optimization` | AC-3 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- integrated_parity_machine_gcode_emit` | AC-4 | FACT pass/fail |
| `cargo test -p xtask editions_config_declares_three_editions` | AC-5 | FACT pass/fail |
| AC-6's `sh -c` command (see `packet.spec.md`) | integrated plan covers every registered core module stem, `external` empty | FACT `PASS` / `FAIL` |
| AC-7's `sh -c` command (see `packet.spec.md`) | pnp-cli passthrough feature **bodies** delegate correctly | FACT `PASS` / `FAIL` |
| AC-8's `sh -c` command (see `packet.spec.md`) | no rayon in the two crates | FACT `PASS` / `FAIL` |
| `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_path` | AC-N1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm` | AC-N2 | FACT pass/fail |
| `cargo test -p xtask dist_registry_coverage_rejects_missing_pnp_cli_feature` | AC-N3 | FACT pass/fail |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness | FACT clean or `STALE:` list |

## Step Completion Expectations

- A native transport error is never converted into success without applying its output.
- Every emitted path and supported gcode command is represented in the committed IR/accumulator result.
- Each parity gate is independently red or green, and every negative comparator test proves a real invariant.
- Module names are re-derived from manifests and registry conventions at implementation time.

## Context Discipline Notes

- Read only the bounded native transport arms and relevant IR definitions.
- Delegate every cargo run and authoritative-doc fact-check.
- Respect the shared context bands; this packet is `M` aggregate and has no `L` step.
