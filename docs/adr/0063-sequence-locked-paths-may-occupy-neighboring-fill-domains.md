# ADR-0063 Sequence-locked paths may occupy neighboring fill domains

Status: accepted (landed with packet 245).

## Context

The fill-partition contract says each fill-role holder emits over exactly one
pre-partitioned polygon. Faithful wave bridge fill cannot satisfy this:
canonical `WaveOverhangs.cpp` deliberately extrudes an anchor band INTO
supported material adjacent to the overhang (`generate_wave_overhang_seeds` +
seed expansion), because first fronts must bond to solid ground. Without an
exception, the linker clips those anchor sections away against the partitioned
polygon.

## Decision

Order-locked paths are self-clipping: the producer guarantees the ENTIRE swept
footprint (variable-width segment trapezoids + round disks at every vertex)
lies inside its legal domain. The infill linker neither clips nor links them and
differences untagged fill of every role in the same region by that swept
footprint. Band geometry is producer-owned: depth comes from module config
(`wave_overhang_anchor_depth_mm`, default = canonical-auto
`min(3 mm, bridge extrusion spacing × (wall_count + 1))`), never from the host
partition.

This amends the four-canonical-fill-polygons invariant in
`docs/02_ir_schemas.md` and the `CONTEXT.md` Infill entry. It builds on the
`order_lock` semantics contract from ADR-0062: locked paths bypass
linking/clipping/simplification in all three consumers (linker, optimizer,
emitter), and the carve pass differences the swept footprint of order-locked
paths out of untagged fill of the same region.

## Considered options

- Host-carved fifth partition polygon (`bridge_anchor_area`) — rejected:
  encodes producer-config-derived geometry into the generic partition; adds
  SlicedRegion/WIT/builder/marshal surface for one consumer.
- Self-limiting waves to `bridge_areas` — rejected: removes supported-side
  bonding; materially weaker waves; unfaithful port.

## Consequences

- The four-canonical-fill-polygons invariant gains a self-clipping exception for
  order-locked paths; ordinary (untagged) fill of every role in the same region
  is differenced by the locked paths' swept footprint.
- Anchor-band geometry is producer-owned and config-derived, never a partition
  polygon of its own.
- Locked paths bypass linking/clipping/simplification in the linker, optimizer,
  and emitter, consistent with ADR-0062.
