---
status: implemented
packet: 205e-integrated-parity-harness
task_ids:
  - TASK-331
---

# 205e-integrated-parity-harness

## Goal

Consolidate integrated native/WASM parity setup and comparator scaffolding so new parity gates keep the full structural, tolerance, and negative-test contract without repeating transport boilerplate.

## Problem Statement

The integrated parity gate is sound but expensive to extend: 21 files repeat WASM loading, dispatcher construction, native and WASM live bindings, blackboard/arena setup, and stage execution. The comparator module also repeats family-level container scaffolding. This packet removes test-only shallow modules while preserving the gate's structural and negative evidence.

## Architecture Constraints

- ADR-0042 forbids weakening structural invariants into byte equality or self-captured snapshot equality.
- ADR-0056 requires both dispatch paths for every integrated module.
- Keep `ParityTolerance` defaults exact: `1e-3`, `1e-3`, and `2.0`.

## Data and Contract Notes

- Native and WASM inputs remain equivalent and are constructed by the harness, not by production code.
- Structural comparator diagnostics must retain family names and region/stage identity.
- Test fixtures remain module-specific to preserve locality of failures.

## Locked Assumptions and Invariants

- 21 integrated parity modules remain mounted and executed.
- Six comparator families and their negative self-tests remain.
- No tolerance loosening, ignored tests, or byte-equality substitution.

## Risks and Tradeoffs

- Over-generalizing setup can make failures less local; keep stage-family adapters explicit.
- A closure-based harness can produce opaque type errors; use small concrete helper structs where compiler diagnostics are clearer.
