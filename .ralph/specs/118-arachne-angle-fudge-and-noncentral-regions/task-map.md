# Task Map: 118-arachne-angle-fudge-and-noncentral-regions

This packet has `task_ids: none` (provenanced by findings N5 + N6). The task
map documents the crosswalk.

## `docs/07` Crosswalk

| `docs/07` row | Packet | Status | This packet's relationship |
| --- | --- | --- | --- |
| M2 — P112 (T-220) — centrality (`D-112-PROPAGATION-ADAPT`, `D-112-CENTRALITY-ADAPT`) | 112 | implemented | C reworks P112's centrality angle parameter (π hack → configured `wall_transition_angle`) + adds `filterNoncentralRegions` (absent from P112). |
| M2 — P116a/P116b (no TASK-###) | 116a/116b | draft → implemented | C depends on A1/A2 strictly (the π hack is load-bearing for A1's scheme until A2 lands). |
| M2 — P117 (no TASK-###) | 117 (B) | draft → implemented | C is independent of B's transition machinery (different code path), but B lands before C in the linear graph. |

## `docs/DEVIATION_LOG.md` Crosswalk

| Entry | Status | This packet's action |
| --- | --- | --- |
| `D-116A-JUNCTION-BANDS` | Closed (A1) | C adds a one-line addendum noting C removes the π hack A1 left in place (load-bearing until A1/A2 landed). No in-place edits. |
| `D-118-ANGLE-FUDGE-NONCENTRAL` (NEW) | — | C creates this entry documenting the N5+N6 fix. |
| `D-112-CENTRALITY-ADAPT` | Closed | Untouched by C (C normalizes the angle parameter but doesn't re-open the centrality adaptation itself). |
| `D-112-PROPAGATION-ADAPT` | Closed (B reopens) | Untouched by C (B owns the propagation supersession). |

## OrcaSlicer Refs by Step

| Step | OrcaSlicer ref | Purpose |
| --- | --- | --- |
| Step 1 (N5) | `BeadingStrategy.h:78` | Canonical `getTransitioningAngle` default (60°). |
| Step 1 (N5) | `BeadingStrategyFactory.hpp:49` | Factory default (π/4). |
| Step 1 (N5) | `SkeletalTrapezoidation.cpp:716-730` | Dead `filterCentral` (gotcha — DO NOT wire). |
| Step 2 (N6) | `SkeletalTrapezoidation.cpp:811-862` | `filterNoncentralRegions` port. |
| Step 2 (N6) | `SkeletalTrapezoidation.cpp:633` | Call site (after `updateBeadCount`). |