# Task Map: 141-arachne-beading-propagation-and-junction-bands

This packet has `task_ids: none` (provenanced by the second-pass Arachne parity
audit `target/arachne_parity_audit_20260706_020657.md` findings N7 + N1, encoded
as committed red tests at `b2ea52b7`; no `docs/07` `TASK-###` exists for N1–N13,
matching 113c's `none` precedent). The task map documents the crosswalk to
`docs/07_implementation_status.md`'s M2 Real Arachne section and the
`docs/DEVIATION_LOG.md` entries this packet reopens/supersedes.

## `docs/07` Crosswalk

| `docs/07` row | Packet | Status | This packet's relationship |
| --- | --- | --- | --- |
| M2 — P112 (T-220..T-233) — `arachne-extrusion-and-wire-up` | 112 | implemented (T-234 closure pending) | A1 reworks P112's `generate_toolpaths` (T-223, the `generate_junctions` body) — the centrality-gated / both-half-edges / clamp scheme P112 shipped is what N1 flags as divergent. |
| M2 — P113c (no TASK-###) — `113c-arachne-faithful-graph-construction` | 113c | implemented (Steps 1-8b; Steps 9-10 inherited) | A1 supersedes 113c for the junction-generation layer only; 113c's graph construction (Steps 1-3) and `insert_node` re-audit (Step 6) remain canonical. A1 inherits 113c's deferred Steps 9-10 (fixture re-baseline + e2e closure) per the distributed-per-packet policy. |

## `docs/DEVIATION_LOG.md` Crosswalk

| Entry | Status | This packet's action |
| --- | --- | --- |
| `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` | Closed (Steps 1-8b) | A1 adds a one-line addendum noting Steps 9-10 are inherited by this packet chain (not silently absorbed) and that the junction-generation layer is superseded by `D-141-JUNCTION-BANDS`. No in-place edits to 113c's narrative. |
| `D-112-MMU-TOPOLOGY` | Closed | Untouched by A1 (downstream stitching, not junction generation). |
| `D-113B-CONNECTJUNCTIONS` | Closed | Untouched by A1 (A2 owns the `connectJunctions` emission layer). |
| `D-141-JUNCTION-BANDS` (NEW) | — | A1 creates this entry documenting the N1+N7 fix. |

## OrcaSlicer Refs by Step

| Step | OrcaSlicer ref | Purpose |
| --- | --- | --- |
| Step 1 (N7) | `SkeletalTrapezoidation.cpp:2091-2127` (`getBeading`/`getNearestBeading`) | Side-table lookup shape + 0.1 mm radius. |
| Step 1 (N7) | `SkeletalTrapezoidation.cpp:1833-1899` (`propagateBeadingsDownward`) | `ratio_of_top` width/location blend + central-edge skip. |
| Step 1 (N7) | `SkeletalTrapezoidation.cpp:1669-1672` (`upward_quad_mids`) | Confirm no centrality filter. |
| Step 2 (N1) | `SkeletalTrapezoidation.cpp:2013-2079` (`generateJunctions`) | Upward-skip / in-band-break / middle-index-start loop structure. |
| Step 2 (N1) | `SkeletalTrapezoidation.cpp:2024-2027` | Flat/same-bead-count skip. |
| Step 2 (N1) | `SkeletalTrapezoidation.cpp:2064-2077` | In-band bead loop + near-`start_R` snap. |