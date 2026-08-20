---
status: stub
packet: stub-support-eligibility-classification
task_ids: []
backlog_source: docs/specs/support-parity-gap-register.md
---

# Packet Stub: needs_support eligibility classification

**Number unassigned** (2026-08-20, human decision: "no numbers, just the stubs for now").
New gap filed by packet 224 decision 2 (2026-08-20).

## Goal

Make the per-region `needs_support` eligibility flag carry real signal: a producer must set it
false where canonical would decline support, and the planner must consume it.

## Owned gaps

- G-17 — `needs_support` is hardcoded `true` in `classify_object`
  (`crates/slicer-core/src/algos/mesh_analysis.rs`) and in `SliceRegionView`'s
  `Default`/`from_ir` (`crates/slicer-sdk/src/views.rs`); no producer ever sets it false.

## Notes

Packet 224 decision 2 kept the renderer-side inversion
(`planned_region_renders_regardless_of_eligibility_flag` — the toolpath generator prints what
was planned; the planner owns eligibility) and deleted the vacuous
`enforcer_overrides_needs_support_false` test from
`modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs`. This stub owns the
classification work that gives the flag its signal back.
