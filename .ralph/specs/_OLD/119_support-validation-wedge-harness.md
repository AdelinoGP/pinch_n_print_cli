---
status: implemented
packet: 119_support-validation-wedge-harness
task_ids: [TASK-290]
---

# 119_support-validation-wedge-harness

## Goal

Stand up a current-contract wedge harness that runs the real prepass against `resources/regression_wedge.stl`, asserts the observable `SupportPlanIR.entries[*].branch_segments[*].points` invariants, and guards branch-count and endpoint drift with committed self-capture goldens. Close the public `dist_to_top_mm` and `raft_plan` seams needed by the source-plan invariants.

## Problem Statement

Block C support work (smoothing, multi-neighbour MST, build-plate pruning) needs validation infrastructure before any algorithm changes: the source plan's `TASK-260` collided with unrelated current work and was renumbered to `TASK-290`; the per-point `dist_to_top_mm` and the optional raft configuration seam were not public; and no current-contract invariant suite existed. ADR-0048 records the three closure resolutions (renumbering, per-point distance emission, raft config emission).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm). Assertions use public committed IR and canonical unit conversion only.
- The harness runs the REAL prepass driver (`prepare_prepass_context` → `SupportPlanIR`), not a stubbed plan; a disabled-support empty plan is reported explicitly, never treated as a pass.
- Golden regeneration is guarded: `SUPPORT_WEDGE_REGEN_GOLDEN=1` env var; normal tests compare committed self-captures without writing them.

## Data and Contract Notes

- IR: schema version 1.2.0 at close (per-point `dist_to_top_mm` on `Point3WithWidth`; `raft_plan: Option<RaftPlan>` configuration-only seam with `raft_layers`, `raft_first_layer_density`, `base_raft_layers`, `interface_raft_layers`). Raft geometry explicitly deferred (packet 124 / raft-default-module).
- WIT: the support seam point is a dedicated six-field record (NOT `seam-point3-with-width` reuse — ABI-safe decision); `push-raft-plan` on the support-geometry output resource.
- Invariants v1 (7 core): finite branch paths; collision endpoint handling (origin-contact tips exempt via `dist_to_top_mm <= 1e-6`; propagated endpoints outside outer contour excluding holes); layer Z match within 1e-4 mm; overhang coverage within `tree_support_branch_distance`; radius bounds ≤ `MAX_BRANCH_RADIUS_MM = 6.0`; disabled raft → no negative layer index; optional raft config exact-values / None checks.
- Self-capture goldens: `support_regression_wedge_branch_count.txt` (count, ±10%) and `support_regression_wedge_endpoints.txt` (sorted `(x,y,z)` triples, Hausdorff ≤ 0.5 mm). Self-captures, NOT Orca reference data.
- Negative drift detection: in-memory mutated count (>25% from baseline) must produce the `branch count drift > 10%` failure.

## Locked Assumptions and Invariants

- The harness is the gate for ALL later Block C packets (121-124 add invariants 8-13 to the same file).
- Origin-contact exemption is intentionally narrow: `dist_to_top_mm` within `1e-6` mm of zero.
- `support_raft_layers = 0` → `raft_plan.is_none()`; enabled raft emits exactly the configured values (2, 0.4, 1, 1).
- The disabled-test must not silently skip missing geometry (at least one propagated endpoint check required).

## Risks and Tradeoffs

- Self-captured baselines prove self-regression, not Orca parity (same limitation as D-109 — accepted; real Orca reference output remains deferred, `TASK-163b-orca-ref`).
- Golden re-capture is allowed only after prerequisite packets, guest freshness, and non-empty enabled output are confirmed.

## Implementation Deviations (recorded at close)

None beyond ADR-0048's recorded resolutions. Doc Impact: `docs/02_ir_schemas.md` (IR 9b), `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `docs/specs/support-modules-orca-port.md` §C1/Validation Strategy — all updated in-packet.
