# Requirements: anchored-entity-execution

## Packet Metadata

- Grouped task IDs: `TASK-330`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The live tree models `GlobalLayer` as an unsigned-index worker and `LayerCollectionIR` as one flat per-layer output (`crates/slicer-ir/src/slice_ir.rs:1015-1026`, `2323-2349`). The approved plan requires work below, at, and above a model event while retaining global-layer barriers. Existing `Layer::PathOptimization` and staged commit seams must be generalized rather than introducing a second scheduler.

## In Scope

- Add anchored entity identity, anchor index, planar or atomic Z-span geometry contract, capability sets, and provenance in the IR/SDK boundary.
- Derive the stage closure for each anchor from capabilities and preserve the existing `layer-parallel-safe` manifest hint.
- Change global-layer worker output from one assumed flat event to ordered event collections while retaining signed negative raft-prefix entries outside the anchored model.
- Validate planar Z and Z-spanning range/atomicity at path commit.
- Run optimization and cooling/time accounting per physical event without crossing event boundaries.
- Add deterministic forced-serial/forced-parallel and rejection tests in the existing scheduler/runtime integration harnesses, registering any new modules in their `main.rs` aggregators.

## Out of Scope

- `PrePass::SupportAnalysis`, support-family planners/renderers, or universal support geometry contracts.
- A global cross-layer entity scheduler.
- Replacing raft entries `-1..=-raft_layers` or changing `claim:raft-fill`.
- Implementing a nonlinear perimeter, non-planar wall, milling, or inspection producer.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - 430 lines; direct read, especially §§1-2 and invariants 8-12.
- `docs/adr/0059-support-families-and-anchored-entities.md` - delegated SUMMARY; anchored ordering and global-layer barrier.
- `docs/adr/0009-raft-as-layer-infill-role.md` - delegated SUMMARY; raft ordering constraint.
- `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` - delegated SUMMARY; current `LayerStageCommit` seam.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md`.
- Negative: `AC-N1` rejects out-of-range Z-spanning output.
- Cross-packet impact: TASK-331 consumes `AnchoredEntity`, `AnchoredGeometryContract`, `CapabilityDerivedEventClosure`, `OrderedEventCollection`, and `AnchoredEventRuntimeHooks` by the exact names/shapes exported below.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-scheduler --test scheduler_integration capability_derived_anchor_closure -- --exact` | Prove capability closure and parallel hint behavior | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration anchored_event_ordering -- --exact` | Prove event order and same-Z handling | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p slicer-runtime --test integration anchored_z_span_validation -- --exact` | Prove planar/range rejection and atomicity | FACT pass/fail; bounded failure SNIPPETS |
| `cargo check --workspace --all-targets` | Compile struct-literal and WIT blast radius | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint all targets | FACT pass/fail |

## Step Completion Expectations

- Global-layer ordering remains the only cross-layer barrier.
- Every event collection is immutable after commit and deterministic under serial/parallel execution.
- Raft prefix entries remain signed negative global-layer entries and are not converted to anchored entities.

## Context Discipline Notes

The scheduler and runtime files exceed 300 lines; workers must read only the ranges named by `design.md` and delegate cargo commands. Generated WIT bindings and `target/` remain unread.
