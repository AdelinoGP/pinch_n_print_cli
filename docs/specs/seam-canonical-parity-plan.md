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
| 1 | `178-seam-region-aware-planning` | Replace contour-ordinal seam identity with active-region `RegionKey` planning over per-region `SliceIR` geometry, annotations, and scoring width. | `TASK-294` | - | generated | `.ralph/specs/178-seam-region-aware-planning/` |
| 2 | `179-seam-canonical-algorithm-fidelity` | Restore canonical Orca comparator, seeded visibility, seam-string retry/gap-anchor behavior, and full-pivot B-spline fitting, preferring `faer` with a local exact fallback. | `TASK-277` | #1 | generated | `.ralph/specs/179-seam-canonical-algorithm-fidelity/` |
| 3 | `180-seam-final-placement-default` | Project aligned seams onto final wall geometry continuously, preserve wall metadata, report degraded fallback, and make aligned the default. | `TASK-283` | #2 | generated | `.ralph/specs/180-seam-final-placement-default/` |

Packets 178, 179, 180 and this plan must be committed together. Packet 179
must consume the identity and input-view exports from packet 178; packet 180
must consume packet 179's canonical seam target and fallback semantics.

**Re-derived 2026-07-22 against `docs/07_implementation_status.md`:** the
queue's original `TASK-281`/`TASK-282` row IDs collide with closed rows from
packet 117 (`support-planner::tapered_radius` and avoidance-cache, both
closed 2026-07-19). `TASK-283` is the only free ID that preserves the
row-3 ordinal; row 1 takes the newly derived free `TASK-294`; row 2 takes `TASK-277`
(gap above `TASK-276`, which is the only non-monotonic assignment, and
`TASK-285` is closed under packet 120). The three packets' `task_ids` and
`task-map.md` crosswalks must be re-derived independently at refine time.

**Preflight verdict:** the prior "PREFLIGHT PASS" line is **withdrawn**.
Packet 178 has been re-derived to `TASK-294` with a `supersedes:` row; the
caller must run `/spec-review 178 --preflight` to produce the real verdict
before flipping `status: active`. Packets 179 and 180 remain `status: draft`
and have not been re-derived or preflighted. Packet 3 still carries an
accepted FORWARD-DEP on packets 1 and 2; the deviation row
`D-283-ADR-0046-AMENDED` to amend ADR-0046 (default `seam_mode` = `aligned`)

**Ownership reconciliation:** packet 176 support-preview retains `TASK-291`; packet 178 seam-region-aware-planning owns `TASK-294`.
must be added to `docs/DEVIATION_LOG.md` during packet 3's implementation.
