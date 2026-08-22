---
status: stub
packet: stub-support-agg-rasterizer
task_ids: []
backlog_source: docs/specs/support-parity-gap-register.md
---

# Packet Stub: SupportGridPattern AGG rasterizer

**Number unassigned** (2026-08-20, human decision: "no numbers, just the stubs for now").
Previously referred to as `224a` in the gap register.

## Goal

Port or otherwise resolve the canonical `SupportGridPattern` AGG rasterizer
(`SupportMaterial.cpp`) — an antialiased scanline rasterizer over a byte grid plus a 4-direction
seed fill and marching-squares contour extraction.

## Owned gaps

- G-07 — `SupportGridPattern` AGG rasterizer. **Needs-research first**: validate that the
  rasterizer is actually required before implementing it. The open question is whether
  grid-snapping and contour simplification affect anything this project needs; they change
  support outline shape but not termination, coverage, collision freedom, interfaces, or
  independent heights. See `docs/spec_packets/224-support-family-orca-closure/design.md`
  §Deviation to file.

## Notes

Packet 224 implements the *semantic* (propagate without growth, trim per layer at
`support_object_xy_distance`), not the rasterizer. This stub owns the research question and any
subsequent port.
