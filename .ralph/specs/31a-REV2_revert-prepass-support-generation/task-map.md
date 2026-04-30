# Task Map: 31a-REV2_revert-prepass-support-generation

This packet spans three task IDs in `docs/07_implementation_status.md` and supersedes two prior packets while normalizing references in three more. The mapping below ties each `docs/07` task ID to the implementation steps that satisfy it, the docs that govern it, the code surface it touches, and the per-step context cost.

| docs/07 task ID | Packet step(s) | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-161` | Steps 1–10, 12 | `docs/01_system_architecture.md` (lines 100–230, 370–410, 525–540), `docs/02_ir_schemas.md` (lines 680–700), `docs/03_wit_and_manifest.md` (lines 540–565), `docs/04_host_scheduler.md` (lines 95–110, 660–680, 905–920), `docs/07_implementation_status.md` (line 98) | `wit/world-prepass.wit`, `wit/deps/ir-types.wit`, `crates/slicer-host/src/{prepass,dispatch,execution_plan,wit_host,blackboard,support_geometry}.rs`, `crates/slicer-sdk/src/{prelude,traits,prepass_builders}.rs`, `crates/slicer-schema/src/lib.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-ir/src/slice_ir.rs`, `modules/core-modules/support-planner/{src/lib.rs,support-planner.toml,Cargo.toml}`, all `tests/prepass_support_geometry_*` and `tests/live_layer_support_tdd.rs` | None | M | TASK-161 originally introduced `PrePass::SupportGeneration`; under this packet it is rewritten in place to describe the consolidated outcome (`SupportPlanIR` + cross-layer support planning produced inside `PrePass::SupportGeometry`). Checkbox stays `[ ]` until Step 12 closes it. The substantive carry-forward of `SupportPlanIR` and `SupportPlanEntry` from packet 28 lands here. |
| `TASK-162` | Steps 1, 2, 4, 6, 7, 9, 10, 12 | `docs/04_host_scheduler.md` (lines 660–680), `docs/05_module_sdk.md` (lines 130–220), `docs/07_implementation_status.md` (line 99) | `wit/world-prepass.wit` (the merged signature carries `layer-plan-view` + `region-segmentation-view` parameters that TASK-162 originally introduced), `crates/slicer-sdk/src/traits.rs` (`run_support_geometry` signature receives both views), `crates/slicer-host/src/{prepass,dispatch}.rs` (projector calls), `modules/core-modules/support-planner/src/lib.rs` (planner walks real `LayerPlanView` + `RegionSegmentationView`) | None | S | TASK-162 stays `[x]`. The work it captured (planner emits one entry per `(layer, object, region)` triple from real `LayerPlanIR.layers` and `RegionMapIR.entries`) is preserved verbatim in semantics; only the export name and stage id change. Step 9 documents the rename in the host-scheduler `required_slots()` table. |
| `TASK-163-foundation` | Steps 1, 4, 5, 6, 7, 9, 12 | `docs/01_system_architecture.md` (lines 100–230), `docs/02_ir_schemas.md` (lines 75–85, 680–700), `docs/04_host_scheduler.md` (lines 660–680, 905–920), `docs/07_implementation_status.md` (line 100) | `crates/slicer-ir/src/slice_ir.rs` (`SupportGeometryIR` shape preserved), `crates/slicer-host/src/blackboard.rs` (slot + accessors preserved), `crates/slicer-host/src/support_geometry.rs` (host built-in preserved), `wit/world-prepass.wit` (`support-geometry-view` records preserved), `modules/core-modules/support-planner/support-planner.toml` and `modules/core-modules/tree-support/tree-support.toml` (config keys preserved) | None | S | This row tracks only the still-correct foundation work originally scoped under packet `31a`'s side of TASK-163. The TASK-163 algorithmic completion (avoidance/collision cache, radius taper, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys) remains scoped under packet `31b` and is explicitly out of this packet's scope. The `docs/07` row for TASK-163 is not modified by this packet. |

## Per-step coverage cross-check

| Step | Task IDs covered |
| --- | --- |
| Step 1 (WIT contracts) | TASK-161, TASK-162, TASK-163-foundation |
| Step 2 (SDK rename) | TASK-161, TASK-162 |
| Step 3 (Schema + macros) | TASK-161 |
| Step 4 (Host stage routing) | TASK-161, TASK-162 |
| Step 5 (Host WIT impl + blackboard) | TASK-161, TASK-162 |
| Step 6 (support-planner repurpose) | TASK-161, TASK-162 |
| Step 7 (Tests rename + rewrite) | TASK-161, TASK-162 |
| Step 8 (Comment sweep) | TASK-161 |
| Step 9 (Docs rewrite) | TASK-161, TASK-162, TASK-163-foundation |
| Step 10 (docs/07 reconciliation) | TASK-161, TASK-162 |
| Step 11 (Spec-packet sweep) | TASK-161, TASK-162, TASK-163-foundation |
| Step 12 (Backpressure gates) | TASK-161, TASK-162, TASK-163-foundation |

Each task ID is covered by at least one step that produces a verifiable artifact. TASK-161's checkbox transitions to `[x]` in Step 12 — that single transition is the marker for the entire task closure, not a per-step partial credit.

## Aggregate context cost

Sum across the rows above is **M**. No row is `L`. If during execution any task ID begins to feel like it warrants `L` work (e.g., the WIT-bindings rename ripples into more files than discovered), the implementer must stop and either narrow the step or escalate the packet for split.

## Predecessor reconciliation (cross-packet)

Although not a `docs/07` row, the cross-packet impact ledger is part of the packet's responsibility and is repeated here for the implementer:

| Packet | Status before | Status after | Step |
| --- | --- | --- | --- |
| `28_tree-support-multi-layer-propagation` | implemented | implemented (HEAD admonition + body rewrite) | Step 11a |
| `30_support-planner-prepass-wit-plumbing` | implemented | implemented (HEAD admonition + body rewrite) | Step 11b |
| `31a_support-geometry-prepass-and-layer-height` | draft | superseded (frontmatter flip + admonition; body verbatim) | Step 11c |
| `31a-REV1_support-geometry-prepass-and-layer-height` | draft | superseded (frontmatter flip + admonition; body verbatim) | Step 11c |
| `31b_support-planner-algorithmic-parity` | draft | draft (HEAD note + reference normalization; algorithmic content unchanged) | Step 11d |

The carry-forward AC absorption table in `requirements.md` documents the substance migration from `31a` and `31a-REV1` into this packet's ACs, ensuring no work is lost.
