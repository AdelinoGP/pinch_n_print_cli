---
status: implemented
packet: 177-arachne-baselines-to-structural-invariants
task_ids: []
---

# 177-arachne-baselines-to-structural-invariants

## Goal

Replace the perimeter suite's self-captured JSON oracles with source-geometry structural
invariants. The correctness gate is a measured Arachne-versus-classic coverage
floor over reproducible Arachne perimeter inputs, with the D5 bow defect kept as
a synthetic discriminator so a 0.668 coverage ratio cannot pass.

## Problem Statement

The Arachne JSON corpus is captured from Pinch 'n Print's own output. It proves
only that the pipeline still resembles an earlier snapshot, not that the
geometry is correct. ADR-0042 requires structural properties and a known-good
reference for the D5 coverage failure class.

The correction must not replace one self-ratifying artifact with another. A
**coverage subject** therefore means source geometry that can be run through
both classic and Arachne at the same aligned Z planes. A serialized snapshot is
not a coverage subject.

## Architecture Constraints

- Coordinate units remain `1 unit = 100 nm`; use `Point2::from_mm` or
  `mm_to_units` at every boundary.
- Classic and Arachne measurements compare the same source geometry at the
  same `global_layer_index`/Z plane. A ratio across misaligned planes is
  invalid.
- Arachne bead widths are flow-spacing values. Name the cap
  `2 * optimal_spacing_mm`; do not compare raw extrusion widths to it. The D4
  `19.6 mm` value is a historical failure observation.
- `WallToolPaths.cpp::generate` is the authority for an always-even
  `2 * inset_count` maximum. `LimitedBeadingStrategy.cpp::compute` does not
  have the claimed odd-count giant-center branch; do not cite one.
- Absolute-coordinate equality against a captured snapshot is forbidden.
- The threshold is a floor. If repeatability is greater than `0.02`, or if the
  derived floor admits `0.668`, stop rather than tune.

## Risks and Tradeoffs

- The threshold is measured only over source inputs with a real paired path;
  this is narrower than the old JSON corpus but materially stronger evidence.
- A repeatability delta above `0.02` exposes nondeterminism instead of hiding it
  behind tolerance.
- Deleting snapshots sacrifices historical numeric diff visibility. That is
  intentional: they were self-captured correctness oracles, not independent
  evidence.
- The existing D-104f red test remains open and is not absorbed into this
  packet's structural claims.
