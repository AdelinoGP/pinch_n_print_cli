---
status: draft
packet: 140_lightning-module-rewrite
task_ids:
  - TASK-265
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 140_lightning-module-rewrite

## Goal

Rewrite `modules/core-modules/lightning-infill` as a per-layer sampler: read the layer's
tree segments from the packet-137 `LightningTreeIR` view (accessed via the
`PaintRegionLayerView` SDK accessor `lightning_tree_segments_for(object_id, region_id)`),
emit them as raw `ExtrusionPath3D` polylines with `ExtrusionRole::SparseInfill` and the
config-derived `speed_factor`, delete the single-layer stub (the `build_branches` function
at `lib.rs:234` and the inline grid-sampling machinery in `run_infill`/`fill_expolygon`),
close DEV-081, and run the contained lightning re-bless + roadmap-close workspace ceremony.

## Scope Boundaries

One module rewrite plus roadmap closure: the module becomes ~sample-and-emit (the
generation intelligence lives host-side per ADR-0029), lightning output flows through
the 133 linker like every other infill, DEV-081 flips to Closed, and lightning-affected
expectations are re-blessed in one justified event. Manifest claims stay
`["claim:sparse-fill"]`; no WIT/IR change. The 138/139 producer surface is the
**read-only input** here — defects found are recorded deviations routed back to those
packets, not patched in this packet.

## Prerequisites and Blockers

- Depends on: `137_lightning-prepass-contract` (view, `LightningTreeIR`),
  `138_lightning-distancefield-treenode` (primitives), `139_lightning-layer-generator`
  (real trees committed), `133_infill-linker-module` (the linker connects the emission).
- Unblocks: — (roadmap end).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a lightning-configured object with committed `LightningTreeIR` trees,
  **when** `run_infill` dispatches, **then** the module emits exactly the layer's tree
  segments as raw `ExtrusionPath3D` polylines with `role == ExtrusionRole::SparseInfill`
  and the config-derived `speed_factor` — count and endpoint equality against the IR
  view (the module adds NO geometry of its own). | `cargo test -p lightning-infill -- samples_tree_ir_raw_emit 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the module source, **when** grepped, **then** `build_branches` and the
  inline grid-sampling machinery are deleted (zero definitions) and no
  `clip_polyline`/`connect_branches` call remains. | `rg -c 'fn build_branches|fn nearest_boundary_point|fn polygon_bbox_mm|fn point_in_expolygon|fn point_in_polygon|fn sample_grid|clip_polyline|connect_branches' modules/core-modules/lightning-infill/src/lib.rs | grep -q '^0$' && echo STUB-GONE`
- **AC-3. Given** an end-to-end lightning-configured slice with the linker active, **when**
  `Layer::InfillPostProcess` commits, **then** the sparse bucket contains linked multi-point
  polylines derived from tree segments (mean points-per-path > 2) — lightning flows through
  Architecture A like every other module. | `cargo test -p slicer-runtime --test executor -- lightning_pipeline_linked 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** `docs/DEVIATION_LOG.md`, **when** grepped, **then** the DEV-081 row's
  status column reads `Closed` (or the open status is replaced by a `Closed … packet
  140` suffix per the live log's convention — FACT at the time of editing). |
  `rg -q 'DEV-081.*[Cc]losed.*140|DEV-081.*140.*[Cc]losed' docs/DEVIATION_LOG.md && echo DEV-CLOSED`
- **AC-5. Given** lightning-affected test expectations, **when** this packet closes,
  **then** each re-bless carries a closure-log justification and was captured from two
  consecutive identical runs (contained lightning bless — the roadmap's second and final
  bless event). | `cargo test -p lightning-infill 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a default-config slice (no lightning holder) of
  `resources/regression_wedge.stl`, **when** run, **then** the g-code SHA is byte-identical
  (the rewrite touches nothing outside the lightning path). | `cargo test -p slicer-runtime --test e2e -- wedge 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** a lightning-configured layer whose committed `LightningTreeIR` has zero
  segments (e.g. no overhangs), **when** the module runs, **then** it emits nothing for
  that layer and the slice completes (no panic, no fallback to the deleted stub). |
  `cargo test -p lightning-infill -- empty_trees_emit_nothing 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p lightning-infill 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo xtask test --workspace --summary` (roadmap-close ceremony — dispatch; FACT
  verdict only)
- `cargo xtask build-guests --check` (rebuild if STALE)

## Authoritative Docs

- `docs/specs/lightning-infill-parity.md` §Phase L4 — full read (short).
- `docs/adr/0029-lightning-prepass-tree-generator.md` — module-sampler contract;
  delegate SUMMARY.
- `docs/DEVIATION_LOG.md` — DEV-081 row (the closure target).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` (37 lines) / `.hpp` (42 lines) — `Filler::_fill_surface_single`: what per-layer transformation (if any) Orca applies between `getTreesForLayer` and emission — the module must mirror only the sampling side (generation is host-side per ADR-0029; linking is the 133 linker's).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` — DEV-081 status → `Closed` (packet 140) —
  `rg -q 'DEV-081.*[Cc]losed' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` — TASK-262…TASK-265 closure sweep —
  `rg -q 'TASK-265.*[Cc]losed' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
