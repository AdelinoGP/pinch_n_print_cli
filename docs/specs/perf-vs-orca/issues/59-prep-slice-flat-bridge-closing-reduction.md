# PrePass::Slice flat-bridge closing reduction: representation-identical analysis

Type: task
Label: wayfinder:task
Status: claimed (2026-10-14; first frontier ticket in order on map resumption — representation analysis take)
Assignee: 2026-10-14 OpenCode wayfinder session (map resumption; first frontier ticket in order)
Graduated from the map's fog by the 2026-10-09 perf refresh
([evidence](../evidence/perf-refresh-2026-10-09/FINDINGS.md)).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

`assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)
builds the flat-bridge mask with
`closed = closing_ex(&support, FLAT_BRIDGE_ENCLOSURE_RADIUS_MM, closing_join)`
and `refilled = difference(closed, support)`. Fresh attribution at HEAD
(2026-10-09 refresh) puts `polygon_ops::closing_ex` under `host:slice` as the
largest host scope on the base cells — **384–389 s accumulated thread-time**
(470 calls, ~0.82 s/call) on base classic supports-on/off and ~24–25 s (240
calls) benchy — with the same cluster carrying `offset` 127.6 s and
`clip_polygons` 48.3 s accumulated; `PrePass::Slice`'s own stage wall is 47–56 s
on base (ticket 27 probe, uninstrumented).

The prior candidate was falsified on a real layer, not on algebra: avoiding the
full-layer `difference` via the set identity changed `SliceIR.bridge_areas`
(105 candidate polygons vs 104 legacy on benchy layer 1 — see
`evidence/t27-serial-floor/FLAT-BRIDGE-EXPERIMENT.md`). Representation is the
binding constraint.

Deliverable — representation analysis first:

1. Enumerate what actually consumes the bridge mask / closing result and what
   representation each consumer needs (`derive_needs_support`'s bbox prefilter +
   exact intersection, bridge assembly, the visual-debug taps — per ticket 27's
   premise audit), on real layers.
2. Either (a) a candidate reduction proven representation-identical against a
   real-layer IR differential oracle — the prior attempt's lesson: the synthetic
   fixture is not an oracle — or (b) a documented **no safe reduction** verdict
   with the evidence.

Only after (a) may the standing paired A/B run.

## Acceptance

For (a): representation identity on real layers first, then the paired ordinary
+ accelerated A/B per the standing metric; measured keep/drop to the human; no
auto-commit. For (b): the verdict is a resolution with the representation
evidence attached; no code change.

## Notes

- This ticket carries the map's previous "flat-bridge reduction inside
  `PrePass::Slice`, representation analysis first" fog patch; the `apply_opening`
  geometry-parity question stays separate and out of scope for this map.
- Evidence: `evidence/perf-refresh-2026-10-09/FINDINGS.md` (host table),
  `evidence/t27-serial-floor/FLAT-BRIDGE-EXPERIMENT.md`,
  `evidence/t27-serial-floor/FINDINGS-SUBSTAGE-2.md` (probe-2 stage fuel),
   `evidence/t27-serial-floor/PREMISE-AUDIT.md` (consumer audit).

## Comments

### Benchy premise refresh — 2026-10-13 (no take started)

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
confirms `host:slice` still leads accumulated native work across both generators
and support states. `closing_ex` is 23.069–25.128 s accumulated thread-time
(240 calls in each profile), not slice wall. Profiling-off Arachne stage
brackets are 2.773/2.609 s off/on. The real-layer representation constraint and
this ticket's question remain unchanged; no replacement or timing gate was
started. Evidence: [fresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md).

### Claim — 2026-10-14 (map resumption; claim recorded before any work)

The frontier was re-derived on resume: the open front is this ticket, [60],
and [20] (parked below-fold); per the t63/t64 convention this session took the
first frontier ticket in order. Take plan, per this ticket's own acceptance:
(1) enumerate the real-layer consumers of the flat-bridge mask / closing
result and the representation each consumer needs; (2) attempt a reduction
shape against a real-layer IR differential oracle (the prior attempt's lesson:
the synthetic fixture is not an oracle) — either a candidate proven
representation-identical (then the standing paired ordinary + accelerated A/B,
keep/drop to the human, no auto-commit) or (3) a documented no-safe-reduction
verdict with the differential evidence. No production edit and no timing
campaign are authorized by the claim itself.
