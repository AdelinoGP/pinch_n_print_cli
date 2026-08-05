---
status: implemented
packet: 28_tree-support-multi-layer-propagation
task_ids:
  - TASK-120
  - TASK-120b
  - TASK-161
---

# 28_tree-support-multi-layer-propagation

## Goal

Introduce a new `PrePass::SupportGeometry` stage plus a `SupportPlanIR` blackboard contract that carries per-layer organic branch geometry produced by OrcaSlicer-style multi-layer propagation. Ship a first core-module planner (`support-planner`) that holds a new `support-planner` claim and implements a simplified port of OrcaSlicer's `TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes` (top-down `SupportNode` propagation + per-layer Prim MST for branch merging), without the full avoidance/collision/radius-tapering machinery. Update the `tree-support` per-layer `Layer::Support` module so its `run_support` implementation emits extrusion paths directly from `SupportPlanIR` when it is committed on the blackboard, with the existing grid-MST filler preserved as a fallback for when no planner module is loaded. Update `traditional-support`'s manifest and source comments to make explicit that it remains a per-layer scan-line filler and does not consume `SupportPlanIR`, so the two support modules coexist cleanly under the new contract.

## Problem Statement

After packet `26_live-support-module-evidence` landed, the `tree-support.wasm` core module does emit `SupportIR.support_paths` on the live Benchy path, but only by running a naive **single-layer grid-MST filler**: for each `ExPolygon` on a given layer it samples a dense interior grid, builds a Prim minimum-spanning tree across those samples, and emits tree edges as extrusion paths. The output is a radial line pattern confined to one layer, not an organic branching structure, and has no relation to OrcaSlicer's actual tree-support algorithm.

OrcaSlicer's tree support in `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` is fundamentally a **multi-layer, top-down propagation** algorithm (`TreeSupport::drop_nodes`): it detects overhang contact points across all layers, places `SupportNode` instances at those contacts, then walks layers top → bottom moving each node by `tan(angle) * layer_height` per layer, merging close nodes via a per-layer minimum spanning tree, and only emits extrusion paths after propagation finishes. The per-layer MST is used only as a merge-decision helper; the decisive work is the cross-layer propagation.

The current slicer-host architecture cannot host OrcaSlicer's algorithm inside `Layer::Support` because that stage runs in the rayon-parallel per-layer tier (layers are computed in arbitrary order). Multi-layer propagation must execute sequentially during `PrePass` — before per-layer work begins — and write its branch geometry to the blackboard so per-layer emitters can consume it.

This packet introduces that architectural change. It adds a new `PrePass::SupportGeometry` stage, a new `SupportPlanIR` blackboard contract, and a first core-module implementation (`support-planner`) that performs a simplified version of `detect_overhangs` + `drop_nodes` (without avoidance/collision grids, radius tapering, raft handling, or interface-layer logic — those are deferred to future packets). The per-layer `tree-support` module is updated so its `Layer::Support` stage consumes `SupportPlanIR` when present and emits branches from pre-planned geometry; the existing grid-MST filler remains as a fallback path so the module continues to work when no planner is installed. The `traditional-support` module stays on the per-layer `Layer::Support` tier — its scan-line fill is algorithmically per-layer and correctly declines to consume `SupportPlanIR`; we only add a module-level doc comment making this explicit, plus (if needed) an assertion that its `[ir-access].reads` does not claim `SupportPlanIR`.

This packet does **not** supersede packet 26. Packet 26's commit-path and real live-dispatch tiers stay in place; this packet extends them with a new "planner-consuming" tier.

## Architecture Constraints

- **PrePass is sequential, per-stage; per-layer tier is rayon-parallel.** `PrePass::SupportGeometry` runs once, globally, before any per-layer work. `Layer::Support` runs per-layer in rayon. Propagation across layers must happen in the PrePass stage — a per-layer reader can never propagate because it has no cross-layer view guaranteed-consistent under parallelism.
- **Prerequisite chain in `ensure_stage_prerequisites` is a single source of truth.** The new stage must declare its deps there and nowhere else; do not add bespoke checks inside the support-planner module.
- **Blackboard slots are write-once.** The `support_plan` slot uses `OnceCell` like the other prepass slots; duplicate commits must error via the existing `LayerArenaError`-style pattern (`commit_seam_plan` is the precedent).
- **Claim uniqueness.** `support-planner` is a new claim, orthogonal to `support-generator`. Two modules holding `support-planner` in the same stage get first-winner alphabetical dedup with an Info diagnostic (as per `dedup_same_claim_modules` in `execution_plan.rs`).
- **WIT contract changes trigger a rebuild cascade.** Any change to `world-prepass.wit` means rebuilding every prepass wasm. The CLAUDE.md §"WIT/Type Changes Checklist" enumerates the search surface.
- **The SDK trait for prepass modules must support the new stage.** Inspect `PrepassModule` in `crates/slicer-sdk/src/traits.rs`; add `fn run_support_geometry(...)` with a default-unimplemented body so existing prepass modules continue to compile.
- **Coordinate system invariant.** All IR coordinates use `1 unit = 100 nm` (docs/08). `SupportPlanEntry.branch_segments` uses `ExtrusionPath3D` which carries mm-valued `Point3WithWidth` — match the existing support path convention so `tree-support`'s emitter can pass segments through without re-scaling.

## Data and Contract Notes

- `SupportPlanIR.entries` keying: `(global_layer_index, object_id, region_id)`. Multiple entries may exist for the same `(layer, object)` across different `region_id`s — match `SeamPlanIR.entries`'s multiplicity contract.
- **v1 single-region limitation:** `MeshObjectView` does not currently surface per-region segmentation, so the v1 planner emits every entry under the canonical `region_id = 0` bucket. Single-region objects (the Benchy fixture and the live-dispatch test geometries) match correctly because `tree-support`'s `support_plan_segments_for(object_id, region_id)` is invoked with `region_id = 0` for those regions. Multi-region objects will collapse all branches into the first region until a follow-up packet plumbs region info through the prepass WIT.
- `SupportPlanEntry.branch_segments: Vec<ExtrusionPath3D>` — each segment is a polyline (typically 2-point but may be multi-point for long branches). Points use `ExtrusionRole::SupportMaterial` and `speed_factor` computed from the planner's `support_speed` config key (identical to existing tree-support formula: `support_speed / BASE_SPEED`).
- `tree-support`'s `run_support` must preserve deterministic ordering: iterate `SupportPlanIR.entries` in input order, not via HashMap iteration, and emit `ExtrusionPath3D` in that order.
- `traditional-support` does not read `SupportPlanIR`. If this invariant is violated (e.g. a future refactor adds the read), the contract audit in `core_module_ir_access_contract_tdd.rs` must fail. Add an assertion there if one does not already cover support modules.

## Risks and Tradeoffs

- **Risk:** Simplified propagation without avoidance produces branches that pass through model walls on complex geometries (overhangs that hang over solid body below). *Mitigation:* clamp each propagated node to the active region's bounding contour on each move; accept the resulting "branches that hug wall surfaces" as a v1 limitation documented in `requirements.md` out-of-scope. A follow-up packet adds `TreeSupportData` avoidance cache.
- **Risk:** `PrePass::SupportGeometry` adds a new hard dependency on `SurfaceClassificationIR`, which is host-built-in today. If a test harness ever disables the built-in mesh analysis, the planner silently becomes a no-op. *Mitigation:* the negative AC on missing `LayerPlanIR` covers the equivalent failure for layer planning; add a parallel test on missing surface classification if the built-in execution ever becomes optional.
- **Risk:** Two support modules sharing `support-planner` claim drop silently via alphabetical dedup. *Mitigation:* covered by AC-dedup negative test; the Info diagnostic is surfaced on stderr by `main.rs` so operators see it.
- **Tradeoff:** Per-layer MST is still O(V²) Prim. OrcaSlicer carries the same complexity. For very dense contact sets this still spikes, but V is bounded by `support_max_branches_per_layer` (default 1024), so worst-case MST work is ≤ ~10⁶ ops per layer per part — well under a pathological grid-MST case.
- **Tradeoff:** Introducing a new core module grows the module set and therefore the cold-start module-load time. Acceptable; the prepass is gated on `support-planner` being installed and does nothing when it's absent.
