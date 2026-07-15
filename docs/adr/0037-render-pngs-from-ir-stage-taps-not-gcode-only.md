# ADR-0037 — Render PNGs From IR Stage Taps, Not G-code Only

## Status

Accepted.

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

Stages without 2-D printable geometry emit stage-specific synthetic diagrams
of trace-relevant documented IR fields rather than fabricated geometry.

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
