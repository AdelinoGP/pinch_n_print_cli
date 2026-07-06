# Task Map: 143-arachne-transition-ends-and-extra-ribs

This packet has `task_ids: none` (provenanced by findings N3 + N8). The task
map documents the crosswalk.

## `docs/07` Crosswalk

| `docs/07` row | Packet | Status | This packet's relationship |
| --- | --- | --- | --- |
| M2 — P112 (T-222) — `propagation` (`D-112-PROPAGATION-ADAPT`) | 112 | implemented | B reworks P112's `apply_transitions` (single-mid-split → end-based splitting) + adds `filterTransitionMids`/`generateAllTransitionEnds`/`generateExtraRibs`. |
| M2 — P111 (T-210..T-218) — `BeadingStrategy` stack | 111 | implemented | B extends the trait P111 shipped with 3 new methods. |
| M2 — P141/P142 (no TASK-###) | 141/142 | draft → implemented | B depends on A1/A2's junction fans + emission. |

## `docs/DEVIATION_LOG.md` Crosswalk

| Entry | Status | This packet's action |
| --- | --- | --- |
| `D-112-PROPAGATION-ADAPT` | Closed | B adds a one-line addendum noting B supersedes the single-mid-split scheme with canonical end-based splitting. No in-place edits. |
| `D-143-TRANSITION-ENDS` (NEW) | — | B creates this entry documenting the N3+N8 fix. |
| `D-141-JUNCTION-BANDS` / `D-142-CONNECTJUNCTIONS-EMISSION` | Closed (A1/A2) | Untouched by B (B reads A1/A2's output but doesn't change them). |

## OrcaSlicer Refs by Step

| Step | OrcaSlicer ref | Purpose |
| --- | --- | --- |
| Step 1 (trait ext) | `BeadingStrategy.h` | `getTransitioningLength`/`getTransitionAnchorPos`/`getNonlinearThicknesses` signatures. |
| Step 2 (N3) | `SkeletalTrapezoidation.cpp:881-915` | `generateTransitioningRibs` pipeline. |
| Step 2 (N3) | `SkeletalTrapezoidation.cpp:1007-1076` | `filterTransitionMids`. |
| Step 2 (N3) | `SkeletalTrapezoidation.cpp:1247-1403` | `generateAllTransitionEnds`. |
| Step 2 (N3) | `SkeletalTrapezoidation.cpp:1487-1543` | `applyTransitions` at ends. |
| Step 2 (N8) | `SkeletalTrapezoidation.cpp:1579-1633` | `generateExtraRibs`. |
| Step 2 (N3) | `SkeletalTrapezoidation.cpp:1712-1721` | `generateSegments` beading interpolation. |