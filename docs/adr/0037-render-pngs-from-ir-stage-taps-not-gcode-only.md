# ADR-0037 — Render PNGs From IR Stage Taps, Not G-code Only

## Status

Accepted. **Amended 2026-07-15 (packet 161)** — the synthetic-diagram clause is
revised; see "Amendment" below. All other clauses stand.

## Context

Serialized G-code can show the final defect but cannot show when it was
introduced. The host already has typed, host-owned IR at post-stage commit
boundaries: Blackboard outputs are immutable during per-layer work and
per-layer IR is owned by `LayerArena` until it becomes `LayerCollectionIR`.
The scheduler's fixed stage order provides stable boundaries without adding
module-visible APIs.

## Decision

Visual debug renders intermediate PNGs from typed, post-stage, post-host-hook
IR taps. It also renders final G-code as an independent final-stage path, but
that path is not the sole source of visual evidence. Taps remain runtime-owned
and request-gated; they do not change WIT, manifest IR access, module
ownership, or scheduler dependencies.

Stages without directly renderable 2-D geometry are handled per the 2026-07-15
Amendment below; the governing principle is that the renderer never fabricates
geometry.

## Consequences

- Agents can localize the first stage that diverges instead of comparing only
  the end artifact.
- Typed adapters and contract tests must evolve with IR schema changes.
- Capture must respect Blackboard and LayerArena lifetimes, and must not retain
  unbounded snapshots for unselected layers.
- Final G-code parsing remains useful for printer-facing artifacts and
  standalone investigations, but cannot substitute for intermediate evidence.

## Alternatives Considered

- **G-code-only post-hoc rendering.** Rejected: it cannot identify the stage
  that first created a defect.
- **A debug WASM module.** Rejected: it would require new module access and
  ownership contracts merely to observe host-owned IR.
- **Per-module snapshots.** Rejected for v1: a post-stage committed snapshot is
  deterministic and bounds artifact growth.

## Amendment — 2026-07-15 (packet 161)

The original decision directed: *"Stages without 2-D printable geometry emit
stage-specific synthetic diagrams of trace-relevant documented IR fields rather
than fabricated geometry."* Grounding the full tap inventory during packet 161
showed no implemented tap needs that mechanism, so the synthetic-diagram render
mode is retired:

- **RegionMapping renders real geometry.** It joins `RegionMapIR.entries` to the
  committed `SliceIR` regions on `(global_layer_index, object_id, region_id,
  variant_chain)` (`SlicedRegion` carries all four) and draws each region's
  polygons tinted/labeled by its `RegionPlan` (`config` resolved via
  `config_for()`, `stage_modules`, `paint_overrides`) — real geometry with a
  configuration overlay, not a fabricated diagram.
- **LayerPlanning has no standalone tap.** It is the only stage with no joinable
  geometry; its sync-layer / non-planar / active-region signal is exposed instead
  as an opt-in `diagnostic_overlay` annotation on geometry-bearing taps, so the
  planning signal is preserved without a geometry-less image.
- **No synthetic-diagram render mode is built.** Every implemented tap produces
  real geometry or a point/path/annotation overlay through one geometry/overlay
  renderer.

This amendment retires only the synthetic-diagram *mechanism*; the original
principle — never fabricate geometry — is preserved, as are all other clauses
(runtime-owned, request-gated taps reading committed IR with no new
module/WIT/Blackboard API). Capture also spans three mechanisms (Blackboard-read,
per-layer arena, PostPass whole-print) per ADR-0040. Recorded as
`D-161-ADR-0037-AMENDED` in `docs/DEVIATION_LOG.md`.
