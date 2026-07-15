# Task Map: 113c-arachne-faithful-graph-construction

This packet spans 10 sequential steps that replace Arachne's graph-construction and
junction-connection layer with a faithful port of OrcaSlicer's real algorithm, re-validate
every downstream stage, and correct the deviation-log record left by packet 113b's incomplete
closure. No `TASK-###` entries exist in `docs/07_implementation_status.md` for this work; the
provenance is a `/diagnose` session against a live user-reported bug plus
`docs/DEVIATION_LOG.md`'s `D-112-MMU-TOPOLOGY`/`D-113B-CONNECTJUNCTIONS` entries.

## Task Crosswalk

| Packet Step | Deviation Touched | Provenance | OrcaSlicer Ref |
|---|---|---|---|
| Step 1: Per-cell Voronoi metadata | (foundation for all below) | `/diagnose` session finding | `boostvoronoi::Cell` (vendored crate, no OrcaSlicer ref) |
| Step 2: Cell-range-walk spike | (design gate for Step 3) | — | `VoronoiUtils.cpp` (`compute_segment_cell_range`/`compute_point_cell_range`) |
| Step 3: Faithful per-cell graph construction (L, exception documented) | Supersedes the graph-construction gap in `D-112-MMU-TOPOLOGY`/`D-113B-CONNECTJUNCTIONS` | Root cause of both | `SkeletalTrapezoidation.cpp:431-560`, `:157-257`; `SkeletalTrapezoidationGraph.cpp:452-482` |
| Step 4: Faithful `connectJunctions` | Supersedes `D-113B-CONNECTJUNCTIONS`'s "pragmatic minimum" closure | — | `SkeletalTrapezoidation.cpp:2260-2368` |
| Step 5: Centrality/bead_count re-validation | (cascade, no deviation closed directly) | — | N/A (re-validation only) |
| Step 6: `insert_node` re-audit (dedicated) | Re-verifies the "busy-hub" fix from `D-112-MMU-TOPOLOGY`'s 6th pass against the new topology | — | `SkeletalTrapezoidationGraph.cpp:310-431` |
| Step 7: Stitch/simplify/remove_small re-validation | (cascade, no deviation closed directly) | — | N/A |
| Step 8: Invariant suite + `test_voronoi.cpp` triage | — | — | `test_voronoi.cpp` (triage only, not connectJunctions faithfulness) |
| Step 9: Fixture re-baseline + deviation log + glossary | Registers `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`; supersedes `D-112-MMU-TOPOLOGY` + `D-113B-CONNECTJUNCTIONS` | — | N/A |
| Step 10: End-to-end verification + workspace gate | Closes `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` with real evidence | — | N/A |

## Deviation Disposition (Post-Packet)

| Deviation | Status Before 113c | Status After 113c | Mechanism |
|---|---|---|---|
| `D-112-MMU-TOPOLOGY` | `Closed` (11th pass — a test-harness realignment, never touched graph construction) | **Superseded** — one-line addendum added, existing narrative untouched | Step 9 addendum, pointing to `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` |
| `D-113B-CONNECTJUNCTIONS` | `Closed` (the central-only domain-walk generalization, now proven insufficient) | **Superseded** — one-line addendum added, existing narrative untouched | Step 9 addendum, pointing to `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` |
| `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` (new) | N/A | **Closed** (Step 10, against real evidence: the `cube_4color.3mf` end-to-end closure check going from 100% to 0% failure) | Steps 3-10 |
| `D-112-SELFCAPTURED-BASELINES` | Open (accepted) | Unchanged — still open, still accepted; this packet's fixture re-baselines fall under the same precedent | N/A |

**Why supersession, not in-place edit:** an explicit user decision during this packet's grilling
session. The established `docs/DEVIATION_LOG.md` convention is append-only (every entry already
grows via dated "Nth pass" notes, never deleted) — the user chose to extend that convention with
a fresh top-level ID rather than continuing to grow the same two entries indefinitely, since
those two entries' own "Closed" status is itself part of what needs correcting and a fresh ID
makes that correction unambiguous to a future reader skimming the log.

## Cross-Packet Dependencies

- **Depends on packet 113b** (`status: implemented`): the existing Arachne pipeline source,
  fixtures, and host-service bridge that this packet supersedes in part (graph construction,
  `connectJunctions`) and re-validates in part (centrality, bead_count, propagation, stitch,
  simplify, remove_small).

- **Unblocks:** a real (not just claimed) M2-faithful closure for the Arachne perimeter-parity
  roadmap — packet 113b's own closure claimed this but, per this packet's diagnosis, did not
  actually achieve it for the graph-construction/connectJunctions layer.

## OrcaSlicer Reference Paths (per `requirements.md` §OrcaSlicer Reference Obligations)

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:431-560` —
  `constructFromPolygons()` (Steps 2-3, relaxed contract)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:157-257` —
  `transferEdge()` (Step 3, relaxed contract)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452-482` —
  `makeRib()` (Step 3, relaxed contract)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:183-193` —
  `getNextUnconnected()` (Step 4, default contract; already pre-seeded in `design.md`)
- `OrcaSlicerDocumented/src/libslic3r/Geometry/VoronoiUtils.cpp`
  (`compute_segment_cell_range`/`compute_point_cell_range`) (Step 2, relaxed contract)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260-2368` —
  `connectJunctions()` (Step 4, default contract)
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:310-431` —
  `insertRib()`/`insertNode()` (Step 6, default contract)
- `OrcaSlicerDocumented/tests/libslic3r/test_voronoi.cpp` (Step 8 triage, LOCATIONS contract —
  confirmed zero OrcaSlicer unit tests exist above this layer, i.e. nothing for Arachne/
  `SkeletalTrapezoidation`/`WallToolPaths`/`connectJunctions` specifically)

## Step Dependency Graph

```
Step 1 (per-cell metadata) — S
    └── Step 2 (cell-range spike) — S
            └── Step 3 (faithful graph construction) — L, exception documented
                    └── Step 4 (faithful connectJunctions) — M
                            ├── Step 5 (centrality/bead_count) — M
                            ├── Step 6 (insert_node re-audit, dedicated) — M
                            └── Step 7 (stitch/simplify/remove_small) — S
                                    └── Step 8 (invariant suite + triage) — M
                                            └── Step 9 (fixtures + deviation log + glossary) — S
                                                    └── Step 10 (end-to-end + workspace gate) — S
```

Steps 5, 6, and 7 can run in parallel after Step 4 lands (they all read the new graph/
connectJunctions output but don't depend on each other's own changes) — though Step 6's
dedicated gating (per the grilling decision) means its own exit condition must still be
independently confirmed before Step 8 assumes propagation is trustworthy. Step 8 depends on
Steps 5-7 (the invariant suite exercises the fully re-validated pipeline). Step 9 depends on
Step 8 (fixture re-baseline happens once the invariant suite has stabilized behavior). Step 10
depends on all prior steps (final end-to-end + workspace gate).

## L-Step Exception (Step 3)

Same category and justification as packet 113b's own L-exception for its Step 1 (`makeRib`
pass), re-confirmed during this packet's grilling session: spine-chain construction and rib
interleaving are mutually dependent — building spine-only first and "adding ribs later" is
exactly 113b's own failure mode being fixed here, so there is no safe split point. Recommended
internal (non-gated) checkpoints for risk control are documented in `implementation-plan.md`'s
Step 3 block and `design.md`'s §Risks and Tradeoffs. If subsequent design work (e.g. Step 2's
spike surfacing more complexity than expected) reveals a natural split point, the packet SHOULD
be split before Step 3 begins — this exception is scoped to the current understanding of the
algorithm, not an unconditional license.

## Grilling-Session Decisions (traceability)

This packet's shape was set across a grilling session (per the `/grilling` + `/domain-modeling`
skills) that resolved 8 explicit forks before authoring began — recorded here so a future
reader understands why the packet looks the way it does, not just what it says:

1. One 10-step packet, not split into 113c/113d (everything cascades from Step 3).
2. Step 3's L-step exception approved, same precedent as 113b.
3. Deviation-log correction uses new-ID supersession, not in-place edits.
4. `cube_4color_arachne_per_color_footprint_within_bbox` strengthened in place, not frozen +
   duplicated.
5. `packet.spec.md` status `active` immediately (no blocking packet).
6. New ADR (`0034`) authored alongside packet authoring, not deferred to an implementation step.
7. Provenance via `source_index()` + side table, verified safe against the vendored
   `boostvoronoi` source directly (not just assumed) — with the point-cell dedup-ambiguity
   caveat folded into Step 2's spike.
8. `propagation.rs::insert_node`'s re-audit split into its own dedicated step (Step 6), given
   its documented bug history, rather than folded into Step 5.
