---
status: stub
packet: stub-support-raft
task_ids: []
backlog_source: docs/specs/support-parity-gap-register.md
---

# Packet Stub: Raft geometry

**Number unassigned** (2026-08-20, human decision: "no numbers, just the stubs for now").
Previously referred to as `227` in the gap register (that number is taken by an unrelated
packet).

## Goal

Implement raft geometry.

## Owned gaps

- G-06 — Raft geometry. `RaftPlan` (`crates/slicer-ir/src/slice_ir.rs`) is built and rendered
  by nothing — the IR exists, the consumer does not. All raft config keys are dead in the four
  support modules and stay as-is (dead) rather than removed or wired, per packet 224
  `requirements.md` §Out of Scope.

## Notes

None.
