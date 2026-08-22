---
status: stub
packet: stub-support-independent-layer-z
task_ids: []
backlog_source: docs/specs/support-parity-gap-register.md
---

# Packet Stub: Independent support-layer Z

**Number unassigned** (2026-08-20, human decision: "no numbers, just the stubs for now").
Previously referred to as `225` in the gap register (that number is taken by an unrelated
packet).

## Goal

Implement support-layer Z independent of object-layer Z.

## Owned gaps

- G-02 — Independent support-layer Z. Partly reachable already: the blockers are
  `is_same_z_entity`'s on-grid filter (`crates/slicer-runtime/src/layer_executor.rs`) and
  `crates/slicer-runtime/src/pipeline.rs` never calling `execute_per_layer_with_anchored_events`.
  Unverified risk: `height_delta` (`crates/slicer-gcode/src/emit.rs`) may mis-scale flow for an
  off-grid entity.

## Notes

The current Orca references were regenerated with `independent_support_layer_height` disabled,
so this is a missing canonical feature, not a divergence measurable against them.
