# Requirements: 205e-integrated-parity-harness

## Packet Metadata

- Grouped task IDs: `TASK-331`
- Backlog source: `docs/specs/integrated-modules-architecture-205c-205e-plan.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The integrated parity gate is sound but expensive to extend: 21 files repeat WASM loading, dispatcher construction, native and WASM live bindings, blackboard/arena setup, and stage execution. The comparator module also repeats family-level container scaffolding. This packet removes test-only shallow modules while preserving the gate's structural and negative evidence.

## In Scope

- Shared parity execution harness in `tests/common`.
- Migration of all 21 integrated parity tests onto the harness.
- Shared comparator entry/container helpers where they preserve family-specific diagnostics.
- All existing parity self-tests and module-specific fixtures/configuration.

## Out of Scope

- Production dispatch, SDK types, registry implementation, or module algorithms.
- Changing tolerance values or replacing structural comparisons with byte equality.
- Adding OrcaSlicer fixtures or adjudicating existing deviations.
- Removing any parity test, negative self-test, or family comparator.

## Acceptance Summary

- Positive: `AC-1` through `AC-4`.
- Negative: `AC-N1`.
- Cross-packet impact: depends on 205c's stable dispatch seam and 205d's registry inventory; no production dependency is introduced.

## Verification Commands

| Command | Purpose | Return format hint |
|---|---|---|
| `cargo test -p slicer-runtime --test contract --all-targets` | All 21 parity gates and self-tests | FACT pass/fail |
| `cargo check --workspace --all-targets` | Test helper and module dependency blast radius | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Refactor lint contract | FACT pass/fail |

## Step Completion Expectations

Migration must be incremental: introduce the harness, migrate a representative layer/prepass/finalization/postpass family, then migrate the remaining tests and comparator scaffolding. The final test count and self-test names must be compared with the pre-migration inventory.

## Context Discipline Notes

Do not read all 21 test files in full. Delegate a structural inventory and read only representative files from each stage family plus the shared helpers.
