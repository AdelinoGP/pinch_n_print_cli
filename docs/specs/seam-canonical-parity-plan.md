# Seam Canonical Parity Plan

This plan follows the implemented `168-seam-aligned-modes` packet. The goal is
algorithmic canonical parity with OrcaSlicer while retaining PNP's required
whole-print prepass plus per-layer application split. The only permitted
implementation divergence is where PNP's execution model cannot expose final
perimeters to a cross-layer module; those seams must preserve canonical inputs,
ordering, and observable behavior as closely as the available IR permits.

Decisions locked during the grilling session:

- Parity means canonical algorithmic behavior, not byte identity.
- Candidate geometry comes from per-region `SliceIR` after region and paint
  preparation, not mesh contour ordinals.
- Alignment identity is the full active-region key, including `variant_chain`.
- `aligned` becomes the default mode, matching OrcaSlicer.
- Canonical visibility constants, seeded canonical sampling, comparator gates,
  retry behavior, and spline fitting are required.
- `faer` is preferred for guest-side matrix infrastructure; missing full-pivot
  behavior must be supplied locally. If `faer` is unusable, fall back to a
  local full-pivot Householder implementation, never to a lower-fidelity QR or
  normal-equation solver.
- Seam enforcer/blocker annotations participate before cross-layer chaining.
- Final placement projects onto continuous wall geometry where necessary and
  preserves feature flags and width profiles.
- Missing plans use canonical local selection as degraded success and report a
  non-fatal module error; they never silently emit an aligned-success result.
- Active-region gaps use a bounded continuity anchor: no synthetic seam entry,
  canonical `4 * flow_width` resume search, and a new string when no candidate
  qualifies.

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | `178-seam-region-aware-planning` | Replace contour-ordinal seam identity with active-region `RegionKey` planning over per-region `SliceIR` geometry, annotations, and scoring width. | `TASK-281` | - | generated | `.ralph/specs/178-seam-region-aware-planning/` |
| 2 | `179-seam-canonical-algorithm-fidelity` | Restore canonical Orca comparator, seeded visibility, seam-string retry/gap-anchor behavior, and full-pivot B-spline fitting, preferring `faer` with a local exact fallback. | `TASK-282` | #1 | generated | `.ralph/specs/179-seam-canonical-algorithm-fidelity/` |
| 3 | `180-seam-final-placement-default` | Project aligned seams onto final wall geometry continuously, preserve wall metadata, report degraded fallback, and make aligned the default. | `TASK-283` | #2 | generated | `.ralph/specs/180-seam-final-placement-default/` |

Packets 178, 179, 180 and this plan must be committed together. Packet 179
must consume the identity and input-view exports from packet 178; packet 180
must consume packet 179's canonical seam target and fallback semantics.

**Preflight verdict (all three packets):** PREFLIGHT PASS. Packet 3 carries
an accepted FORWARD-DEP on packets 1 and 2 (both `status: draft`). Packet 3
amends ADR-0046 via deviation `D-283-ADR-0046-AMENDED` to set the default
`seam_mode` to `aligned`; the deviation row must be added to
`docs/DEVIATION_LOG.md` during implementation.