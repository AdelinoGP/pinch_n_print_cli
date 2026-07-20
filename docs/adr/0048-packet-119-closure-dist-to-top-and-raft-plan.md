# ADR-0048 — Packet 119 closure: `dist_to_top_mm` per-point, `raft_plan: Option<RaftPlan>`, and `TASK-260` re-numbered to `TASK-290`

<!-- filename: 0048-packet-119-closure-dist-to-top-and-raft-plan -->

## Status

Accepted (2026-07-19). Authored during packet 119 (`119_support-validation-wedge-harness`) closure to resolve the three `[BLOCK]` items named in that packet's `design.md` `Risks and Tradeoffs`/`Resolved Closure Decisions` sections: the source-plan `TASK-260` collision, the missing `PlannedSupportNode.dist_to_top` exposure, and the missing `SupportPlanIR.raft_plan` field.

## Context

Packet 119's source plan (`docs/specs/support-modules-orca-port.md` §C1, §C5, §C6) specified three current-contract support invariants:

1. **C1 distance exposure**: the planner's `PlannedSupportNode.dist_to_top` value needed to survive the `plan_for_object` emission boundary as a public per-point field.
2. **C6 raft seam**: `SupportPlanIR.raft_plan` needed to be a structured optional field populated from support configuration, without claiming that packet 119 generates raft geometry.
3. **C1 source-plan `TASK-260`** is the backlog ID for the C1 slice; `docs/07_implementation_status.md` already uses `TASK-260` for unrelated gyroid-infill work, so the IDs collide.

At packet 119's design-time, all three were marked `[BLOCK]` because the public IR did not yet expose the fields and the source-plan ID collided with existing work. The packet's completion gate made the `[BLOCK]` items owner-decision prerequisites for `status: implemented`.

Three pieces of evidence changed during the packet's implementation:

- The planner computes `PlannedSupportNode.dist_to_top: u32` in `modules/core-modules/support-planner/src/lib.rs`, uses it in `tapered_radius`, and now copies it into `Point3WithWidth.dist_to_top_mm` while `plan_for_object` emits branch points. OrcaSlicer's analogous public per-node value is `SupportNode::dist_mm_to_top` (`TreeSupport.hpp`). The PnP field is a per-point contract addition, not a claim of numerical parity.
- The planner emits no raft geometry. `run_support_geometry` emits the configuration-only `RaftPlan` through `SupportGeometryOutput::push_raft_plan`; packet 124 (`124_support-plan-raft-plan-and-raftinfill-role`) owns actual raft geometry.
- OrcaSlicer's raft is synthesized by `SupportCommon::generate_raft_base` and related support-material generation paths (`SupportCommon.cpp` and `SupportMaterial.cpp`); there is no single Orca `RaftPlan` struct. The PnP `raft_plan` field is a seam projection, not a direct port.

The packet author flagged the absence as `[BLOCK]`, intending them to be resolved in subsequent packets. The user, as owner, decided during the closure pass to resolve them now inside packet 119 rather than carry them as open follow-ups.

## Decision

Four concrete changes ship in packet 119:

1. **`TASK-260` re-numbered to `TASK-290`.** Packet 119's source-plan reference is updated from `TASK-260` to `TASK-290` across its packet docs and task map. The existing `TASK-260` row in `docs/07_implementation_status.md` is the gyroid-infill work; that row is unchanged. The new `TASK-290` row records packet 119's planner work. The collision is closed by re-numbering, not by editing either row to a fake value.

2. **`dist_to_top_mm: f32` added to `Point3WithWidth` in `crates/slicer-ir/src/slice_ir.rs` (per-point).** `plan_for_object` copies its existing `PlannedSupportNode.dist_to_top: u32` layer count into the field in millimeters using the current effective layer height. Per-point is the shape needed by downstream consumers and the current wedge harness; this ADR makes no numerical parity claim. `SupportPlanIR.schema_version` is 1.2.0.

3. **`raft_plan: Option<RaftPlan>` added to `SupportPlanIR` in `crates/slicer-ir/src/slice_ir.rs`.** `RaftPlan` has `raft_layers: u32`, `raft_first_layer_density: f32`, `base_raft_layers: u32`, and `interface_raft_layers: u32`. `run_support_geometry` emits `Some(RaftPlan { ... })` through `SupportGeometryOutput::push_raft_plan` when `support_raft_layers > 0`; otherwise the harvested option is `None`. No geometry is computed in packet 119; packet 124 owns actual raft geometry.

4. **Origin-contact tips are the resolved AC-2 exception.** A finite endpoint with `dist_to_top_mm` within `1e-6` mm of `0.0` is an origin contact, so the planner emits the raw contact centroid even when it lies on or inside the model collision outline; projecting that point would break the required overhang contact. Every propagated endpoint with `dist_to_top_mm > 0.0` remains subject to the existing collision checks and propagation clamping, MST collision guard, and scan-line collision guard. The wedge test retains the outer-contour/holes predicate for those endpoints, requires at least one propagated endpoint, and reports exemption/check counts. AC-4 is unchanged: every qualifying non-base downward facet must still have an endpoint within `tree_support_branch_distance` of its centroid.

The packet's three `[BLOCK]` items are closed: `TASK-260` becomes `TASK-290` (re-numbered, recorded in `docs/07`); `dist_to_top_mm` is now in the public IR and the planner emits it; `raft_plan: Option<RaftPlan>` is now in the public IR and the planner emits it from config. The AC-2 origin-contact exception is a separate resolved contract clarification, not a relaxation of propagated-node collision safety or AC-4 centroid coverage.

## Consequences

**Positive**:
- The wedge-harness now exercises the current public `dist_to_top_mm` and raft configuration seams. The ACs that were the source of the `[BLOCK]` items are testable in the same packet.
- The `Point3WithWidth` shape gains a per-point support distance field that downstream code can consume without further IR churn. This is an architectural shape decision, not a numerical OrcaSlicer parity claim.
- Re-numbering the source-plan ID eliminates a real cross-packet collision that would have produced a confusing duplicate-`TASK-260` row in `docs/07` if the packet had been closed without addressing it.
- Packet 124 owns the remaining raft geometry and rendering work behind the configuration seam.

**Negative**:
- `SupportPlanIR.schema_version` bumps to a new minor version. All consumers that match on schema version must update. The current consumer set is small (the test harness in this packet and `slicer-runtime::run::prepare_prepass_context`); both are updated in this packet.
- `Point3WithWidth` grows by one `f32` field. The IR serializes to JSON in the test harness's `branch_endpoints` extraction; the new field is dropped from the golden output (only `x, y, z` are captured, as before). No regression in golden stability.
- The planner emission forwards `dist_to_top_mm` from `plan_for_object` and emits the optional raft configuration from `run_support_geometry`.
- The three additive config keys `raft_first_layer_density`, `base_raft_layers`, and `interface_raft_layers` default to `0.4`, `1`, and `0` and are emitted as-is into the IR. No geometry or geometry-specific validation is added in this packet; packet 124 owns the validation required by raft geometry.

**Trade-offs we explicitly accept**:
- `raft_plan` is a config-mirror struct, not a geometry struct, in this packet. A future reviewer may ask "why is the planner emitting raft config that it doesn't use?" The answer: the IR seam is the contract; packet 124 fills the geometry behind the seam. Mirror-then-fill is a smaller diff than fill-the-IR-and-IR-in-the-same-step.
- The `dist_to_top_mm` field is redundant with the planner's internal `dist_to_top: u32`; emitting both creates two sources of truth. The trade-off is that the IR field is needed for external consumers and the planner's internal counter is needed for the planner's `tapered_radius` math. They are kept in sync by emitting at the same point.

## Future-Reviewer Notes

- **Do not re-suggest folding raft geometry into this packet.** The geometry computation belongs in packet 124 per the existing `docs/specs/fork-gaps-wave2-plan.md` (raft-plan-and-raftinfill-role note) and `docs/specs/support-modules-orca-port.md` §C6 cross-spec dependencies. This packet adds the IR seam; 124 fills it.
- **Do not re-suggest per-entry `dist_to_top_mm`.** Per-point is the Orca-aligned shape and the only one that captures chain monotonicity. Per-entry would lose the chain information that source-plan invariant 3 requires.
- **Do not re-suggest undoing the `TASK-290` re-numbering.** The collision was real; the re-numbering is the smallest fix.

## References

- `docs/specs/support-modules-orca-port.md` §C1, §C5, §C6 — source-plan invariants and the current raft seam.
- `docs/07_implementation_status.md` — existing `TASK-260` gyroid-infill row and packet 119's `TASK-290` row.
- `crates/slicer-ir/src/slice_ir.rs` — `Point3WithWidth`, `RaftPlan`, `SupportPlanIR`, and `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION`.
- `modules/core-modules/support-planner/src/lib.rs` — `PlannedSupportNode`, `run_support_geometry`, `plan_for_object`, and `tapered_radius`.
- Canonical OrcaSlicer `SupportNode::dist_mm_to_top` (`TreeSupport.hpp`) — per-node distance analogue.
- Canonical OrcaSlicer `SupportCommon::generate_raft_base` (`SupportCommon.cpp`) — raft synthesis reference; Orca has no single `RaftPlan` struct.
- Packet 124 `124_support-plan-raft-plan-and-raftinfill-role/packet.spec.md` — owner of raft geometry.
- Packet 123 `123_support-planner-to-buildplate-pruning` — owner of any future buildplate-pruning consumer of support distance.
