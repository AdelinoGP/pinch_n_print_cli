---
status: superseded
superseded_by: 38-rev1_top-surface-ironing
packet: top-surface-ironing
task_ids:
  - TASK-168
---

# 38_top-surface-ironing

## Goal

Ship a new `Layer::InfillPostProcess` core module `top-surface-ironing` that emits a low-flow zigzag pass over committed `TopSolidInfill` polygons at the topmost surface layer, tagged with `ExtrusionRole::Ironing` and producing `;TYPE:Ironing` G-code blocks. Mirrors Orca's `Layer::make_ironing` semantics. Configurable via `ironing: bool`, `ironing_speed`, `ironing_flow`, `ironing_spacing`, `ironing_pattern`.

## Problem Statement

Today the live path lacks any ironing pass over top surfaces. Orca emits a low-flow zigzag pass over the topmost layer's `TopSolidInfill` polygons to smooth the visible top surface. Without ironing, the printed top surface shows extrusion lines and inter-line gaps.

This packet ships a new core module `top-surface-ironing` that runs as `Layer::InfillPostProcess` and emits the ironing pass. The module relies on packet 12-rev1's `is_top_surface` flag and packet 35's `top_solid_layers` plumbing to identify *only the topmost* top-solid layer (not interior layers of the top-solid stack). It uses the existing `support-surface-ironing` module pattern as a template.

## Architecture Constraints

- **Module-only packet.** No host changes (except possibly one line in `gcode_emit.rs`).
- **Transform-chain ordering** via `[ir-access].reads = ["InfillIR.regions"]` and `[ir-access].writes = ["InfillIR.regions"]`. Per `docs/04 §Composable Multi-Writer Patterns`, this establishes an A→B edge (fill module → ironing module) deterministically.
- **Topmost-layer detection** uses 12-rev1's `is_top_surface` AND 35's `top_solid_layers` (via `RegionMapIR`/config-view). The module's runtime check: this region is the *topmost* of its top-solid stack iff `is_top_surface == true` AND no further layer above this region has the region polygon overlap (or, simpler: there is no `is_top_surface == true` flag on the same region at layer N+1, which is equivalent for non-stepped objects).
- **Append-only output**: ironing paths are appended to `solid_infill`; existing `TopSolidInfill` paths are preserved unchanged. The fill module's output is the first stroke; ironing's output is the second stroke at same Z, low flow.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `InfillIR.regions[*].solid_infill` — transform-chain consumer + producer.
  - No schema-version bump (this packet adds a module that uses existing IRs and an existing `ExtrusionRole::Ironing` enum variant).
- WIT boundary considerations: none (the module uses existing WIT/SDK types).
- Determinism or scheduler constraints:
  - Transform-chain edge fill→ironing established by `reads ∩ writes`.
  - Output order deterministic: ironing paths appended after fill paths within each region.

## Locked Assumptions and Invariants

- `ExtrusionRole::Ironing` enum variant already exists (`crates/slicer-host/src/wit_host.rs:2572` confirms it does).
- `support-surface-ironing` module exists as a working template.
- The `Layer::InfillPostProcess` stage already runs on the live path (it's used by other modules; verify via SUMMARY of the dispatch list in `docs/04`).

## Risks and Tradeoffs

- **Topmost-layer detection is the trickiest part.** Without packet 35, every layer in a multi-layer top-solid stack would get ironed, which is wrong. Mitigation: this packet declares packet 35 as a hard prerequisite. The module reads the `top_solid_layers` config and only emits ironing on the highest layer of the stack.
- **Path ordering inside a region.** The append model assumes the host preserves the fill paths and runs ironing strokes after them. Verify via the transform-chain SUMMARY.
- **Default values may differ from Orca.** Delegate FACT for Orca defaults; document any chosen-different-from-Orca values in `docs/DEVIATION_LOG.md` if applicable.
