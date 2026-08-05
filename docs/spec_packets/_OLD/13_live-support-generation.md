---
status: implemented
packet: live-support-generation
task_ids:
  - TASK-120b
---

# 13_live-support-generation

## Goal

Restore support generation on the live Benchy path by making the real `Layer::Support` stage commit non-empty `SupportIR` content on the production host path, with tree-support as the canonical acceptance target for the final Benchy run and traditional-support retained as the control generator for unit-level role and paint-precedence coverage.

## Problem Statement

Support generators exist and have unit coverage, but the live Benchy path still lacks committed support content. The missing slice is the production `Layer::Support` handoff and commit path. This packet keeps the scope small by treating tree-support as the canonical live acceptance target for the final Benchy run while using traditional-support as a control generator to guard the shared host path and the documented paint precedence rules.

## Architecture Constraints

- The packet restores the live host path, not just standalone module geometry.
- Tree-support is the canonical live acceptance target because the parent TASK-120 acceptance run expects tree supports enabled.
- Traditional-support remains in-scope only as a control path for shared host behavior and documented paint precedence.
- The packet must keep exact `ExtrusionRole::SupportMaterial` semantics so packet `11` can serialize them later.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `SupportIR.support_paths`, `interface_paths`, `raft_paths`
  - `ExtrusionRole::SupportMaterial`
  - `PaintSemantic::SupportBlocker` and `SupportEnforcer`
  - support-generator claim selection on the live stage path
- WIT boundary considerations:
  - no schema widening is required; the packet stays on existing support-stage inputs and outputs
- Determinism or scheduler constraints:
  - identical support-stage inputs must produce the same committed `SupportIR` across repeated runs

## Locked Assumptions and Invariants

- Tree-support is the acceptance target for the live Benchy path.
- Control-path traditional-support coverage must not become a second acceptance target that expands the packet's scope into generic support parity.

## Risks and Tradeoffs

- Risk: unit tests may already pass while host dispatch still drops committed support. Mitigation: keep host integration tests primary.
- Risk: tree-support behavior may still diverge from traditional-support in legitimate ways. Mitigation: only share role/commit/paint-precedence assertions across both generators.
