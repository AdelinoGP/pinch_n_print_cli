# Task Map: 116b-arachne-canonical-connectjunctions-emission

This packet has `task_ids: none` (provenanced by the second-pass Arachne parity
audit findings N2 + N4). The task map documents the crosswalk to
`docs/07_implementation_status.md` and `docs/DEVIATION_LOG.md`.

## `docs/07` Crosswalk

| `docs/07` row | Packet | Status | This packet's relationship |
| --- | --- | --- | --- |
| M2 — P112 (T-223) — `generate_toolpaths` | 112 | implemented | A2 reworks P112's line-assembly layer (`chain_junctions_for_bead`/`emit_chain_lines`/`generate_toolpaths`) — the whole-chain-polyline-per-bead + width-merge + sequence-position scheme P112 shipped is what N2 flags. |
| M2 — P113c (no TASK-###) | 113c | implemented | A2 builds on 113c's graph topology (inherited via A1). |
| M2 — P116a (no TASK-###) | 116a (A1) | draft → implemented | A2 depends on A1's upward-half-edge junction fans; A2 supersedes A1 for the junction-metadata + emission layer. |

## `docs/DEVIATION_LOG.md` Crosswalk

| Entry | Status | This packet's action |
| --- | --- | --- |
| `D-116A-JUNCTION-BANDS` | Closed (A1) | A2 adds a one-line addendum noting A2 supersedes A1 for the junction-metadata + emission layer (A1 owns geometry; A2 owns metadata + emission). No in-place edits. |
| `D-116B-CONNECTJUNCTIONS-EMISSION` (NEW) | — | A2 creates this entry documenting the N2+N4 fix. |
| `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` | Closed | Untouched by A2 (A1 already added the inheritance addendum). |
| `D-113B-CONNECTJUNCTIONS` | Closed | Untouched by A2 (113c already superseded it for graph construction; A2's emission rewrite is the next layer). |

## OrcaSlicer Refs by Step

| Step | OrcaSlicer ref | Purpose |
| --- | --- | --- |
| Step 1 (N2) | `SkeletalTrapezoidation.cpp:2283-2327` (`connectJunctions`) | Per-quad from/to pairing + pop-back merge. |
| Step 1 (N2) | `SkeletalTrapezoidation.cpp:2198-2234` (`addToolpathSegment`) | Extend-vs-new-line + `new_domain_start`. |
| Step 1 (N2) | `SkeletalTrapezoidation.cpp:2064-2077` | `perimeter_index = junction_idx`. |
| Step 2 (N4) | `SkeletalTrapezoidation.cpp:2344-2354` | Canonical `is_odd` per-segment rule. |
| Step 2 (N4) | `SkeletalTrapezoidation.cpp:2355-2361` | `passed_odd_edges` physical-edge key. |
| Step 2 (N4) | `ExtrusionLine.hpp:62-70` | `is_odd` semantics. |
| Step 2 (N4) | `WallToolPaths.cpp:838-856` | `removeSmallLines` eligibility gate. |