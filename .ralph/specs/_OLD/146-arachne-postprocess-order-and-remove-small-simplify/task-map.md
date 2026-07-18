# Task Map: 146-arachne-postprocess-order-and-remove-small-simplify

This packet has `task_ids: none` (provenanced by findings N11 + N12 + N13).

## `docs/07` Crosswalk

| `docs/07` row | Packet | Status | This packet's relationship |
| --- | --- | --- | --- |
| M2 — P112 (T-225..T-227) — `stitch`/`simplify`/`remove_small` | 112 | implemented | E reworks all three post-process stages: order swap (N11), per-line `min_width` (N12), distance-gated simplify (N13). |
| M2 — P113a (T-226, DP→VW) | 113a | implemented | E supersedes 113a's DP→VW port for the simplify layer (iterative area-only sweep → canonical distance-gated single pass). |
| M2 — P141/P142/P143/P144/P145 (no TASK-###) | A1/A2/B/C/D | draft → implemented | E depends on D strictly (D's `is_odd` micro-loops interact with E's `removeSmallLines`). |

## `docs/DEVIATION_LOG.md` Crosswalk

| Entry | Status | This packet's action |
| --- | --- | --- |
| `D-112-SIMPLIFY-DP` | Closed (113a) | E adds a one-line addendum noting E supersedes the iterative area-only sweep with the canonical distance-gated single pass. No in-place edits. |
| `D-146-POSTPROCESS-ORDER` (NEW) | — | E creates this entry documenting the N11+N12+N13 fix. |
| `D-142-CONNECTJUNCTIONS-EMISSION` | Closed (A2) | Untouched by E (E doesn't change emission; A2's `is_odd` fix is what makes E's `removeSmallLines` correct). |

## OrcaSlicer Refs by Step

| Step | OrcaSlicer ref | Purpose |
| --- | --- | --- |
| Step 1 (N11) | `WallToolPaths.cpp:679-699` | Canonical post-process order. |
| Step 2 (N12) | `WallToolPaths.cpp:838-856` | `removeSmallLines` per-line `min_width` + divisor. |
| Step 3 (N13) | `ExtrusionLine.cpp:56-243` | `simplifyToolpaths` distance gates + area guard. |
| Step 3 (N13) | `WallToolPaths.cpp:868-872` | `meshfix_maximum_resolution`/`_deviation` sourcing. |