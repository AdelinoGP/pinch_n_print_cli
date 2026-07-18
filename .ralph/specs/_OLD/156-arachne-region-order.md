---
status: implemented
packet: 156-arachne-region-order
task_ids:
  - none
---

# 156-arachne-region-order

## Goal

Close G12 with an end-to-end faithful port of OrcaSlicer's Arachne region
ordering. The selected `wall_sequence` must survive module configuration, the
WASM boundary, final `WallLoop` commitment, and path optimization without
being collapsed, re-sorted, or inverted.

## Problem

The previous G12 implementation added a partial core reorder but did not
faithfully port Orca's constraint construction, applied it before final line
post-processing, collapsed `wall_sequence` into a boolean, dropped that value
at the WASM boundary, and allowed the perimeter module and path optimizer to
override the result. A green direct-core fixture therefore did not establish
production parity.

## Data and Contract Notes

- `arachne-params` changes from a derived bool to a three-state sequence value.
- `WallLoop` and `ExtrusionLine` IR shapes remain unchanged.
- No new config key is introduced; the module transports the existing resolved
  `wall_sequence` value.

## Locked Assumptions and Invariants

- The module owns configuration interpretation and final committed wall order.
- WIT/SDK/host transport the resolved sequence without substituting a default.
- Region ordering consumes finalized lines and is a permutation.
- The optimizer may not invert committed wall sequence.
