---
status: stub
packet: stub-support-patterns-expansion-bottom-z
task_ids: []
backlog_source: docs/specs/support-parity-gap-register.md
---

# Packet Stub: Base/interface patterns, support_expansion, support_bottom_z_distance

**Number unassigned** (2026-08-20, human decision: "no numbers, just the stubs for now").
Previously referred to as `226` in the gap register (that number is taken by an unrelated
packet).

## Goal

Implement the support base-pattern and interface-pattern generators, `support_expansion`,
`support_bottom_z_distance`, and the related tree-renderer/flow gaps.

## Owned gaps

- G-03 — Support base-pattern and interface-pattern generators (`support_base_pattern`,
  `support_base_pattern_spacing`).
- G-04 — `support_expansion`.
- G-05 — `support_bottom_z_distance`.
- G-08 — No `support_line_width` (PnP's `line_width` is global).
- G-09 — `effective_layer_height` disagrees across transports.
- G-10 — Tree branch bodies are filled; `support_density` percent/fraction mis-scale.
- G-11 — PnP over-extrudes support 1.107x versus Orca (uniform flow model).
- G-12 — `MAX_BRANCH_RADIUS_MM = 6.0` vs canonical 10.0.
- G-13 — Missing canonical "raise radius to `base_radius` when `support_interface_top_layers > 0`".
- G-16 — Undeclared config keys in `tree-support-planner`.
- G-18 — Canonical roof/floor layer-count semantics (PnP traditional 2 interface blocks vs
  Orca 3 at `top_layers=2`/`bottom_layers=2`).

## Notes

The renderers themselves were in scope for packet 224 (RC-8/RC-9/RC-10/RC-12); the pattern
*generators* were not.
