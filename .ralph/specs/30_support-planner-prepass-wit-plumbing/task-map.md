# Task Map: 30_support-planner-prepass-wit-plumbing → docs/07

This packet closes the two correctness carve-outs documented as v1 limitations in packet `28_tree-support-multi-layer-propagation` (layer-height-agnostic; single-region per object). It is additive — packet 28 stays `implemented` and is not superseded.

## Task ID Mapping

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-162` | Step 1 | `docs/02`, `docs/03`, `docs/04`, `docs/05` | none | none | Read-only discovery of existing prepass projector pattern. |
| `TASK-162` | Step 2 | `docs/05` | `crates/slicer-sdk/src/prepass_types.rs`, `crates/slicer-sdk/src/prelude.rs` | none | New host-side types + re-exports. |
| `TASK-162` | Step 3 | `docs/03` | `wit/world-prepass.wit` | none | Add 4 records and extend `run-support-generation` parameters. |
| `TASK-162` | Step 4 | `docs/03`, `docs/05` | `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs` | none | Trait signature + macro arg threading. |
| `TASK-162` | Step 5 | `docs/03`, `docs/04` | `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs` | none | Deterministic projectors + dispatcher wiring. |
| `TASK-162` | Step 6 | `docs/04` | `crates/slicer-host/src/prepass.rs` | none | `required_slots` extension. |
| `TASK-162` | Step 7 | `docs/03` | `modules/core-modules/support-planner/support-planner.toml` | none | Manifest reads list + comment scrub. |
| `TASK-162` | Step 8 | `docs/02`, `docs/04` | `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` (new), `crates/slicer-host/tests/live_support_generation_tdd.rs` (extension) | none | TDD scaffolding — fail before implementation. |
| `TASK-162` | Step 9 | `docs/05` | `modules/core-modules/support-planner/src/lib.rs`, `modules/core-modules/support-planner/wit-guest/src/lib.rs` | none | Drop `DEFAULT_LAYER_HEIGHT_MM`; loop over real region IDs; fatal on empty layer plan. |
| `TASK-162` | Step 10 | `docs/04` | `crates/slicer-host/tests/live_support_generation_tdd.rs` | none | Multi-region tree-support live test. |
| `TASK-162` | Step 11 | `docs/03` | every `modules/core-modules/*/wit-guest/`, every `modules/core-modules/*/*.wasm` | none | Cascade rebuild. |
| `TASK-162` | Step 12 | `docs/07` | `docs/07_implementation_status.md` | none | Backlog row. |
| `TASK-162` | Step 13 | `docs/11`, `docs/12` | none | none | Packet completion gate. |

## Superseding Relationship

Packet 30 does **not** supersede any prior packet.

- Packet `28_tree-support-multi-layer-propagation` (status: `implemented`). Packet 28 deliberately documented limitations (1) and (2) as v1 carve-outs in `modules/core-modules/support-planner/src/lib.rs`. Packet 30 closes both gaps additively. Packet 28's tests continue to pass without modification because they use uniform layer heights and single-region fixtures.
- Packet `26_live-support-module-evidence` (status: `implemented`). Unchanged — its grid-MST fallback path is independent of the WIT plumbing.

## docs/07 Reconciliation Note

`TASK-162` is a new task added by this packet. Draft line for `docs/07_implementation_status.md` Workstream 3:

```
- [ ] TASK-162 Surface `LayerPlanIR.layers` and `RegionMapIR.entries` to the prepass guest via new WIT views (`layer-plan-view`, `region-segmentation-view`) so `support-planner` walks the real layer plan and emits one entry per `(layer, object, region)`. Closes the v1 layer-height-agnostic and single-region carve-outs from packet `28_tree-support-multi-layer-propagation`. Wired by packet `30_support-planner-prepass-wit-plumbing`.
```

`TASK-161` (packet 28's deliverable) is unchanged — packet 30 extends it without reopening it. A short note may optionally be appended to the TASK-161 closure block after this packet ships ("Further upgraded YYYY-MM-DD by packet `30_support-planner-prepass-wit-plumbing`: planner now reads the real `LayerPlanIR` and emits per-region entries.") — optional, since packet 161's evidence is still accurate for what packet 28 shipped.

## Parallelism Note

Packet 30 is serial by construction. It must run after packet 28 closes and before packet 31a activates. No parallel tracks.
