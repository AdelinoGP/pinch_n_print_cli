# Task Map: 166-nonuniform-scale-bake

This packet mints a new backlog task; the crosswalk below is the authoritative mapping to add to `docs/07_implementation_status.md` at closure (append as a `- [x] TASK-272 — …` row in the same style as TASK-270/271).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-272` (new — mint at closure: "Non-uniform scale support: delete the dead `validate_non_uniform_scale` rejection + `NonUniformScaleUnsupported` variant; prove per-axis transform baking with loader tests. Spec: packet 166-nonuniform-scale-bake.") | Steps 1-4 | `docs/02_ir_schemas.md` | `crates/slicer-model-io/src/loader.rs`, `crates/slicer-model-io/tests/` | none (removes a PNP-only restriction; no port) | S | The baking tests + deletion grep + regression sweep jointly prove the task; the Step 1 audit proves deletion safety. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
