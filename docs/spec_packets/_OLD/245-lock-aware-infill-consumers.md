---
status: implemented
packet: 245-lock-aware-infill-consumers
task_ids:
  - TASK-355
---

# 245-lock-aware-infill-consumers

## Goal

Make the three infill consumers — the infill linker, the path optimizer, and G-code emission — honor
`ExtrusionPath3D.order_lock` sequences: locked paths bypass linking/clipping/simplification and are
appended verbatim, untagged fill is carved around their swept footprint, and all-`None` slices remain
byte-identical to today.

## Problem Statement

Packet 244 (implemented) introduces the `order_lock: Option<u64>` carrier and the host enforcement
contract, but no consumer honors the field yet. Three downstream stages still destroy locked sequences: the infill
linker re-clips, chains, and reverses bridge-role paths; path optimization nearest-neighbor permutes
role groups and may reverse entities; G-code emission runs Douglas-Peucker and `min_segment_length`
pruning that drops authored interior points. Until these three consumers honor locks, a producer
that mints locks (packet 246's wave-overhangs module) would have its physically load-bearing print
order silently destroyed. This packet closes that gap with three consumer changes plus structural
parity proof that all-`None` slices are unchanged.

## Architecture Constraints

- The carve pass is module-local to `infill-linker` per ADR-0026 (single caller, single home). No new
  shared geometry helper is extracted to `slicer-core` or `slicer-sdk`; the swept-footprint builder
  lives in the linker and is the only caller of the carve.
- Locked paths are self-clipping: the producer guarantees the entire swept footprint lies inside its
  legal domain, so the linker neither clips nor links them. This is the ADR-0063 exception to the
  four-canonical-fill-polygons invariant.
- The optimizer treats a locked block as one non-reversible candidate, mirroring ADR-0011's
  wall-subsequence precedent (walls are committed in final print order and never reordered within a
  region).
- No schema/version constant is bumped by this packet; the change is purely behavioral and gated on
  `order_lock.is_some()`.

## Data and Contract Notes

- IR/manifest contracts: none changed. `order_lock` is read-only here; the field's semantics are
  fixed by ADR-0062 (packet 244).
- WIT boundary: none changed.
- Determinism/scheduler constraints: the carve and locked-passthrough must be deterministic
  (discovery order); the optimizer's block coalescing must not change the tool-cluster ordering.

## Locked Assumptions and Invariants

- Locked paths are self-clipping (ADR-0063): the producer guarantees the swept footprint lies inside
  its legal domain; the linker differences untagged fill by that footprint and never clips the locked
  path itself.
- A locked block is atomic and contiguous within one `(layer, object, region)`; the optimizer may
  move the block as a unit but never split, reverse, or internally reorder it.
- Speed/flow side mutations of locked paths remain legal; only sequence and geometry (points,
  widths) are protected.

## Risks and Tradeoffs

- The round-disk vertex approximation adds polygon vertices; if the disk is too coarse, the carve
  leaves slivers of untagged fill under the caps. Mitigation: a fixed segment count per disk (e.g.
  16) and a test asserting no untagged fill overlaps the swept area.
- The carve runs after bucket fill, so it must not reorder buckets or disturb the locked passthrough
  already appended. Mitigation: carve operates on the untagged buckets only, keyed by region.
