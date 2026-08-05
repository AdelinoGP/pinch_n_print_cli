---
status: implemented
packet: 123_support-planner-to-buildplate-pruning
task_ids: [TASK-288]
---

# 123_support-planner-to-buildplate-pruning

## Goal

Add `to_buildplate: bool` tracking to `PlannedSupportNode`, prune branches whose move target lies inside `collision_polys` AND cannot reach the build plate, and honor a new `support_on_build_plate_only` config (default `false`): when true, every contact whose `to_buildplate` would be `false` is rejected at creation time. Add a `build_plate_only_emits_no_to_model_branches` wedge invariant.

## Problem Statement

The planner had no `to_buildplate` notion and no unsupported-branch pruning; `support_on_build_plate_only` config was unhonored. Orca tracks `to_buildplate` per node (`generate_contact_points` init) and prunes branches that can't reach the build plate (`drop_nodes` / `unsupported_branch_leaves`). Source-plan `TASK-264` collided with unrelated ledger work (now Lightning Layer) — renumbered to `TASK-288`.

## Architecture Constraints

- The `to_buildplate` flag is INTERNAL to the planner — it does NOT cross the IR/WIT boundary.
- Contact creation sets `to_buildplate = true` if the contact XY lies outside the object's projected footprint at that layer (sourced from `SupportGeometryView.outlines`, the same data `LayerCollisionCache.collision_polys[L]` carries); `false` otherwise.
- The new `to_buildplate = true`-only prune is ADDITIVE to the existing all-node collision drop — a tightening, not a relaxation; `to_buildplate = false` nodes keep pre-packet behavior (AC-N2 pins this).
- `support_on_build_plate_only` plumbed through `on_print_start` config read + new `SupportPlanner` struct field; new `[config.schema]` entry (bool, default `false`).
- A moved node inherits its source's `to_buildplate` during propagation (single- or multi-neighbour aggregation).

## Data and Contract Notes

- Pruning-chain semantics (as delivered): pruning stops propagation at the failed layer and does NOT retroactively remove already-emitted ancestor segments.
- Invariant 10 (wedge harness): `build_plate_only_emits_no_to_model_branches` — with `support_on_build_plate_only = true` forced, every emitted branch endpoint lies outside the object's per-layer collision outline, with the only exemption a fresh contact tip (`dist_to_top_mm <= 1e-6`) on the overhang's origin layer. Endpoints at the build plate (`z <= 1e-3 mm`) and at the overhang tip accepted.
- The `node-clamped-out` diagnostic (code 1002, packet 118) is emitted on the new prune path.
- Golden shift minimal (default config is `false`; only to-model branches already dropped by `clamp_to_avoidance` change).
- Task renumber: source-plan `TASK-264` → `TASK-288`.

## Locked Assumptions and Invariants

- All prior wedge invariants stay green (13 total including the new one at close).
- Default config does NOT reject to-model contacts (AC-N1).
- `PlannedSupportNode` field addition requires auditing every struct-literal site (`rg 'PlannedSupportNode \{'`).
- Raft contact initialization must also set `to_buildplate` (raft block untouched but verified).

## Risks and Tradeoffs

- Packet 122 multi-neighbour aggregation is a soft dependency — the prune works against both single- and multi-neighbour propagation.
- Future `to_model`-vs-`to_buildplate` strategy selection is deferred.

## Implementation Deviations (recorded at close)

None. Doc Impact: `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list extension + §C5 pruning-chain semantics clarification (2026-08-05 doc audit); `docs/15_config_keys_reference.md` key row.
