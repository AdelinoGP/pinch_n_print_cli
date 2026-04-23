# Design: 23-rev1_prepass-seam-planning-orca-parity

## Controlling Code Paths

- **Primary code path:** `dispatch.rs:PrePass::SeamPlanning` dispatch → `wit_host::object_mesh_to_wit_mesh_object_view` → `bindings.call_run_seam_planning` → WASM module `run_seam_planning`
- **Neighboring tests or fixtures:**
  - `dispatch_tdd.rs:prepass_seam_planning_commits_seam_plan_ir` — primary integration test
  - `live_seam_path_tdd.rs:seam_plan_ir_is_injected_into_wall_postprocess_region_view` — downstream injection test
  - `benchy_end_to_end_tdd.rs:benchy_prepass_seam_plan_matches_live_outer_wall_start` — live pipeline test
  - `core_module_ir_access_contract_tdd.rs:seam_planner_default_declares_prepass_contract_roots` — manifest contract test
- **OrcaSlicer comparison surface:** `SeamPlacer.cpp` visibility scoring — used only as a reference for the *problem domain* (corner detection), not for direct code borrowing. The modular slicer's curvature-based corner detector is the chosen approach.

## Architecture Constraints

- All geometry crosses the WIT boundary as value types (`MeshObjectView`), not handles. This is already established by `MeshSegmentation` and `PaintSegmentation`.
- `object_mesh_to_wit_mesh_object_view` already exists at `wit_host.rs:2031` — no new conversion needed.
- The seam_arm macro must produce type-compatible SDK objects for the `PrepassModule::run_seam_planning` trait signature.

## Code Change Surface

- **Selected approach:** Mirror the `MeshSegmentation` pattern (Option A from packet spec). Pass `list<MeshObjectView>` through WIT. The module already expects `&[MeshObjectView]` in its trait signature — only the WIT type and macro were wrong.
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**

| File | Change |
| --- | --- |
| `wit/world-prepass.wit:106` | `objects: list<object-id>` → `objects: list<MeshObjectView>` |
| `crates/slicer-host/src/dispatch.rs:688-691` | `let object_ids: Vec<String>` → `let mesh_object_views: Vec<_>` using `object_mesh_to_wit_mesh_object_view` |
| `crates/slicer-macros/src/lib.rs:1713` | `Vec<::slicer_ir::ObjectId>` → `Vec<::slicer_sdk::prepass_types::MeshObjectView>` |
| `modules/core-modules/seam-planner-default/src/lib.rs:144` | `if curvature > 0.5` → `if curvature > 0.2` |
| `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` | Test 4 already added — verify it stays green |

- **Rejected alternatives that were considered:**
  - **Option B (host-query only):** Rewrite `seam-planner-default` to use `raycast_z_down` host service for corner detection. Rejected because the module already calls `obj.triangles.len()` expecting direct geometry access — changing to host queries requires a complete module rewrite and is out of scope for this packet.
  - **Option C (pass object-id + use WIT object MeshObjectView query):** Add a WIT resource type `MeshObjectView` returned by `get-mesh-object-view(object-id)` and have the module call it. Rejected because it adds a new WIT resource and a new host service, which is heavier than the `MeshSegmentation` precedent already established.

## Data and Contract Notes

- **WIT boundary considerations:** `MeshObjectView` is a pure value type (owned `list` contents) that crosses the boundary by value. This is consistent with `MeshSegmentation` and `PaintSegmentation` usage of the same type.
- **The `SeamPlanIR` contract** requires entries with `(global_layer_index, object_id, region_id)` keys; duplicates are forbidden at blackboard commit.
- **`MeshObjectView` is already self-contained:** vertices (`Vec<[f32; 3]>`), triangles (`Vec<[u32; 3]>`), and `paint_layers`. No additional fields need to be populated.

## Locked Assumptions and Invariants

- `wit_host::object_mesh_to_wit_mesh_object_view` produces a non-empty `MeshObjectView` for any `ObjectMesh` with at least one triangle — if the source mesh is empty, the module receives an empty view and must handle it gracefully (AC-7/AC-8).
- The curvature threshold `0.2` is derived from empirical testing on cube geometry. Ordinary cube corners produce angular gaps near `0.28` (calculated from face normals 90° apart → `|1.0 - 0.0| = 1.0` curvature, but actual per-vertex normal averaging reduces this). A threshold of `0.2` allows these real corners to pass while still filtering flat-surface noise.
- `seam-planner-default` already declares its IR access contract correctly (`MeshIR` + `SurfaceClassificationIR` + `LayerPlanIR` reads, `SeamPlanIR` writes) — the bug is purely in the WIT parameter passing, not the manifest.

## Risks and Tradeoffs

1. **WIT compatibility risk:** Changing `run-seam-planning` signature from `list<object-id>` to `list<MeshObjectView>` breaks any external caller that was using the old signature. However, this is an internal WIT world (`world-prepass.wit`) and no external callers exist — the only caller is `dispatch.rs` which is being updated in this same packet.
2. **Macro type safety:** The `seam_arm` change from `Vec<ObjectId>` to `Vec<MeshObjectView>` must exactly match the trait signature `fn run_seam_planning(&self, objects: &[MeshObjectView], ...)` in `slicer-sdk/src/traits.rs`. If the WIT types are regenerated before the macro is updated, there will be a compile error — which is the correct behavior.
3. **Curvature threshold calibration:** `0.2` is a heuristic. If it produces too many false-positive corner candidates on complex geometry, the threshold may need further tuning. This is a runtime quality issue, not a correctness issue — the acceptance tests verify that at least one candidate is produced, not the exact count.

## Open Questions

- None. The `MeshSegmentation` precedent fully resolves the WIT boundary pattern. The curvature threshold was chosen empirically to allow cube corners while filtering noise. The macro type fix is straightforward.
