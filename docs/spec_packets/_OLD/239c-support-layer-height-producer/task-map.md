# Task Map: 239c-support-layer-height-producer

Crosswalk for `docs/07_implementation_status.md`. Registration is **packet-owned closure work**:
Step 8 (`TASK-522`) appends these rows verbatim through a worker dispatch; nothing is registered
at authoring time. **Re-derive the tail of docs/07 before appending** — the task high-water mark
is a ledger fact and is mutable shared state. It was `TASK-507` at split time and `TASK-508`..
`TASK-514` are claimed by `239b-anchored-wit-contract`; confirm both before writing.

This packet supersedes `239-support-independent-layer-z` (jointly with `239a-anchored-host-seams`
and `239b-anchored-wit-contract`) and depends on both of those packets reaching
`status: implemented`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-515` | `Step 1` | `CLAUDE.md` §Config Key Naming Convention | `modules/core-modules/{tree,traditional}-support-planner/*.toml`; `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` | `PrintConfig.cpp` `init_fff_params` | M | key declared `bool`/default true on both planners; red-first `UndeclaredConfigKey` guard (AC-N3); AC-N1 baseline `;Z:` sequence captured before any behaviour change |
| `TASK-516` | `Step 2` | `docs/specs/support-independent-layer-z-split-plan.md` F8 | `modules/core-modules/{tree,traditional}-support-planner/src/lib.rs` | `SupportMaterial.cpp` `bottom_contact_layer`; `SupportCommon.cpp` `generate_support_layers`; `Slicing.cpp` gap rounding | M | `SupportPlanEntry.anchor_z` becomes the declared support print plane; enabled = free-floating canonical stepping, disabled = `sync_gap_with_object_layer` grid copy; no new IR field |
| `TASK-517` | `Step 3` | `docs/21_data_defaults_and_fixtures.md` (churn gate) | `modules/core-modules/{tree,traditional}-support-planner/tests/*_family_tdd.rs` | none (semantics resolved in Step 2) | S | AC-2 / AC-3: off-grid planes when enabled, integer-equal grid planes when disabled, per family |
| `TASK-518` | `Step 4` | `docs/05_module_sdk.md`; `docs/03_wit_and_manifest.md`; `docs/spec_packets/239b-anchored-wit-contract/packet.spec.md` | `modules/core-modules/{tree,traditional}-support/src/lib.rs`; `modules/core-modules/tree-support/tests/tree_family_tdd.rs` | `GCode.cpp` `collect_layers_to_print` (compatibility only; 239a owns the merge) | M | AC-4: `region.z()` retired as the support Z source; off-grid paths leave via 239b's drain, reached through the `collection: &mut LayerCollectionBuilder` parameter 239b's two-builder `layer-support` `run` / `LayerModule::run_support` supplies. Seam resolved; **no fallback branch remains** |
| `TASK-519` | `Step 5` | `docs/specs/support-parity-gap-register.md` row `G-02` | none — **measurement only**; `docs/07_implementation_status.md` record | `GCode.cpp` `_extrude`; `Flow.cpp` `Flow::mm3_per_mm` (comparison targets only) | S | verdict `MISSCALE_FIXED` / `CONSISTENT` plus three numbers for the off-grid pass and three for the following object pass (observation O-1), recorded **before** any fix decision. Empty-diff check on `emit.rs` is the step's own falsification guard |
| `TASK-520` | `Step 6` | `docs/02_ir_schemas.md` (`LayerCollectionIR`, confirm no bump) | `crates/slicer-gcode/tests/gcode_emit_tdd.rs`; `crates/slicer-gcode/src/emit.rs` (**fix branch only**) | `Flow.cpp` `Flow::mm3_per_mm` | M | AC-5 verdict test names the recorded branch; on `MISSCALE_FIXED` the E-assertion blast radius is swept by `LOCATIONS` before editing and no tolerance is widened |
| `TASK-521` | `Step 7` | `docs/specs/support-families-anchored-entities-plan.md` §7 E2, §8, §13 T11; `docs/19_visual_debug.md` | evidence only: `tmp/239c-human-validation.md`, `tmp/p239c-support-indep-{tree,normal}.gcode`, `tmp/vd-p239c/` | none — comparison is human inspection only | M | freshness gate (exit `0`) immediately before **each** artifact; reference existence gate recorded verbatim as `REFS-PRESENT` / `REFS-ABSENT-GATE-OPEN` (**verified absent at authoring**); trap T11 — the VOID "205 vs 150" figure is never requoted |
| `TASK-522` | `Step 8` | `docs/07_implementation_status.md`; `docs/15_config_keys_reference.md` (generated) | docs only: docs/07, gap register, split plan, regenerated key reference | none | S | registration + `G-02` closure + queue row 3 update; ledger facts (`TASK` high water, next `G-`, next `DEV-`) re-derived at edit time, never quoted |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate is `M`; no row
is `L`. The Step 4 seam is resolved (`design.md` §Open Questions `[RESOLVED]`): the renderers
reach the anchored drain through the `collection` parameter 239b's two-builder
`layer-support` `run` / `LayerModule::run_support` supplies, so no fallback can push `TASK-518`
to `L`.
