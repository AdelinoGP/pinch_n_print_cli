# Design: transform-aware-world-z

## Controlling Code Paths

### Primary Code Path — Layer Planning with Object Transforms

```
MeshIR (ObjectMesh with transform field)
  │
  ├─► PrePass::MeshAnalysis ──► SurfaceClassificationIR
  │       │
  │       └─► mesh_analysis.rs:classify_object
  │               └─► apply_transform() to each vertex for world-space normals and Z bounds
  │
  └─► PrePass::LayerPlanning ──► LayerPlanIR
          │
          ├─► model_loader.rs:object_world_z_extent(object)
          │       └─► Applies ObjectMesh.transform to each vertex, extracts (z_min, z_max)
          │
          └─► Populates object_height:{id} config keys (main.rs:153)
                  └─► Consumed by LayerPlanning module to compute per-object Z ranges
```

### Existing Transform Application Points

| Location | Behavior |
|---|---|
| `model_loader.rs:object_world_z_extent` | Applies transform to all vertices, extracts Z min/max. Used to supply `object_height:{id}` config. Zero-matrix fallback = identity. Returns `None` if degenerate. |
| `mesh_analysis.rs:apply_transform` (lines 234-251) | Applies transform to vertex for world-space normals and Z bounds classification. Same column-major convention. |
| `mesh_analysis.rs:classify_object` (lines 154-156) | Calls `apply_transform` per vertex for surface classification. |

### Neighboring Tests or Fixtures

| File | What it tests |
|---|---|
| `model_loader_tdd.rs` | Unit tests for `object_world_z_extent` — identity, translation (+10 Z), rotation about X (90° lay-flat → degenerate), uniform scale (×2), zero-matrix fallback, empty mesh |
| `prepass_executor_tdd.rs` | Prepassexecution including LayerPlanning stage |
| `layer_planning_tdd.rs` (to be created) | LayerPlanIR generation with transform-aware Z |
| `mesh_analysis_tdd.rs` | MeshAnalysis with transform application |

## Architecture Constraints

1. **Transform is not applied to mesh geometry at load time.** The raw vertices remain in object-local space. The transform is applied at query time (`object_world_z_extent`, `mesh_analysis::apply_transform`).

2. **Column-major transform convention.** `Transform3d.matrix` is stored column-major (index = col*4 + row). Translation is in column 3 (indices 12, 13, 14 for X, Y, Z). This matches the WASM matrix layout.

3. **Z is in millimeters.** `GlobalLayer.z` is `f32` in mm. `object_world_z_extent` returns `(f32, f32)` in mm. The 100-nm scaling applies only to X/Y `Point2` coordinates.

4. **World-space Z is the canonical surface for planning.** All Z-plane sequencing in `PrePass::LayerPlanning` must use world-space Z, never object-local Z. This invariant must be enforced by tests.

5. **Degenerate extent = no print surface.** When `z_max <= z_min` (e.g., lay-flat rotation collapses vertical extent), `object_world_z_extent` returns `None`. LayerPlanning must handle this gracefully (object contributes zero layers).

## Code Change Surface

### Selected Approach

**TASK-157 — Integration fixtures**: Create 5 new TDD test files in `crates/slicer-host/tests/`. Each test constructs a `MeshIR` with a specific transform, runs `PrePass::LayerPlanning` (or the planning path end-to-end), and asserts on the resulting `LayerPlanIR.global_layers[*].z` values. No production code changes required for the fixture tests themselves — they serve as the regression lock proving the existing `object_world_z_extent` wiring is correct at integration level.

**TASK-158 — Canonical surface**: Two options. The implementation must choose one:

- **Option A (IR field)**: Add `ObjectMesh.world_z_extent: Option<(f32, f32)>` as a derived field, computed once at `MeshIR` construction time (in `model_loader.rs`) and cached on `ObjectMesh`. No schema version bump needed — v1.0.0 not yet released. This makes world-space Z explicit in the IR contract.

- **Option B (config-only documentation)**: Update `docs/02_ir_schemas.md` to document that `object_height:{id}` config keys (populated at `main.rs:153` from `object_world_z_extent`) are the canonical world-space Z supply. Add explicit "do not read local mesh Z for planning purposes" guidance. No schema change.

The packet implementation should prefer Option A if the IR schema change is lightweight and non-disruptive. If `ObjectMesh` is deeply embedded in serialization and the change risk is high, fall back to Option B with comprehensive documentation.

### Exact Functions, Traits, Manifests, Tests

**Production code changes (Option A — IR field)**:

- `crates/slicer-ir/src/` — Add `world_z_extent: Option<(f32, f32)>` to `ObjectMesh` struct (no schema bump — v1.0.0 not released)
- `crates/slicer-host/src/model_loader.rs` — Compute and cache `world_z_extent` at `ObjectMesh` construction time
- `crates/slicer-host/src/main.rs:153` — Update `object_height:{id}` population to use the cached field

**Production code changes (Option B — config-only)**:

- `docs/02_ir_schemas.md` — Add world-space Z canonical supply documentation to `ObjectMesh` section
- `crates/slicer-host/src/model_loader.rs` — Ensure `object_world_z_extent` remains the canonical supply function

**Test files to create**:

- `crates/slicer-host/tests/transformed_model_world_z_tdd.rs` — General transformed model fixture test
- `crates/slicer-host/tests/translated_object_z_floor_tdd.rs` — `translate(0,0,10mm)` → first layer z >= 10.0
- `crates/slicer-host/tests/rotated_object_world_extent_tdd.rs` — `rotate_x(90deg)` world extent correct
- `crates/slicer-host/tests/multi_object_transform_world_z_tdd.rs` — Multi-object LCM with transforms
- `crates/slicer-host/tests/world_z_canonical_surface_tdd.rs` — Canonical surface regression lock
- `crates/slicer-host/tests/non_uniform_scale_tdd.rs` — `NON_UNIFORM_SCALE_UNSUPPORTED` error
- `crates/slicer-host/tests/world_z_below_floor_tdd.rs` — `WORLD_Z_BELOW_FLOOR` error

**Existing files that may need updating**:

- `crates/slicer-host/tests/model_loader_tdd.rs` — Existing `object_world_z_extent` unit tests (should remain; new integration tests complement)
- `crates/slicer-host/tests/mesh_analysis_tdd.rs` — Existing transform+classification tests (should remain)

### Rejected Alternatives

- **Applying transforms at mesh load time**: Rejected because it would change `MeshIR.mesh` semantics (vertices would no longer be object-local), breaking modules that expect local-space geometry. The transform-on-query approach is the established convention.
- **Adding world Z as a WIT-facing field**: Rejected because mesh geometry never crosses the WIT boundary (modules query via host services). World Z is a host-side planning concern, not a module-facing contract.

## Data and Contract Notes

### IR or Manifest Contracts Touched

- `MeshIR.ObjectMesh` — potentially adds `world_z_extent: Option<(f32, f32)>` (Option A) or documents existing behavior (Option B)
- `LayerPlanIR.GlobalLayer.z` — the output that must be world-space when transforms are present
- `MeshIR.schema_version` — no change needed (v1.0.0 not released)

### WIT Boundary Considerations

None. World-space Z is computed entirely on the host side. No WIT types are changed.

### Determinism or Scheduler Constraints

- `object_world_z_extent` must be deterministic: same `ObjectMesh` (same vertex positions and transform matrix) must produce the same `(z_min, z_max)` across runs.
- LayerPlanning with transforms must be deterministic: same input `MeshIR` with transforms must produce identical `LayerPlanIR.global_layers[*].z` values.
- Non-uniform scale detection must be deterministic: `scale_x != scale_y != scale_z` triggers `NON_UNIFORM_SCALE_UNSUPPORTED`.

## Locked Assumptions and Invariants

1. **Transform is column-major.** `matrix[col*4 + row]` is the correct indexing. This is verified by the existing `object_world_z_extent_applies_rotation_about_x` test.

2. **Zero matrix = identity fallback.** Fixtures or generated meshes that leave `Transform3d.matrix` all-zero must behave as identity. Verified by `object_world_z_extent_zero_matrix_treated_as_identity` test.

3. **World-space Z is the only correct Z for planning.** No code path in `PrePass::LayerPlanning` may read object-local vertex Z for Z-plane sequencing.

4. **Degenerate extent is `None`.** When `z_max <= z_min`, the object contributes zero layers. This is not an error — it is a valid (if trivial) print scenario.

## Risks and Tradeoffs

### Risk: Option A Schema Change
If `ObjectMesh` is deeply embedded in serialization (serde JSON, binary formats), adding a derived field may have unexpected side effects. Mitigation: verify the field is marked `#[serde(skip)]` or equivalent, since it is derived, not input.

### Risk: LayerPlanning depends on config keys
Today, world-space Z flows through `object_height:{id}` config keys into LayerPlanning. If Option B is chosen (config-only), this dependency is explicit. If Option A is chosen, the cached field should be used directly rather than re-computing via config keys.

### Open Question: Scale Application
When a uniform scale is applied (e.g., `scale_z = 2.0`), the world-space Z extent is correctly scaled. However, the layer height is specified in world-space mm. Should `effective_layer_height` also be scaled? The current architecture says no — layer height is a user-facing config in mm, applied in world space. The scale applies to the mesh geometry, not to the layer height.

Resolution: Document this clearly in the canonical surface decision. If the question cannot be answered definitively before packet completion, mark the packet as `draft` until resolved.

## Open Questions

~~1. **Should `ObjectMesh.world_z_extent` be a cached IR field (Option A) or documented config-only behavior (Option B)?**~~ **RESOLVED → Option A (no schema bump — v1.0.0 not released).** `ObjectMesh.world_z_extent: Option<(f32, f32)>` will be added as a derived IR field, computed once at load time and cached on `ObjectMesh`. Serialization risk is mitigated by marking the field `#[serde(skip)]`. No schema version bump required since v1.0.0 has not been released. This makes world-space Z a first-class IR contract surface. Option B (config-only) was rejected because world-space Z canonical supply would only be documented, not enforced — any future refactor of `object_world_z_extent` or LayerPlanning could accidentally read object-local Z without a compile-time brake.

~~2. **Does uniform scale affect `effective_layer_height`?**~~ **RESOLVED → No.** `effective_layer_height` comes directly from the `layer_height` config value (or `first_layer_height` for the first layer), never scaled by the object's transform. When `scale_z=2.0` is applied to a 20mm-tall mesh, `object_world_z_extent` correctly returns world extent 0..40mm, but layers are generated at 0.2mm world-space intervals (not 0.1mm scale-compensated). Layer height is a user-facing config in mm, applied in world space. The scale applies to mesh geometry only.

~~3. **What is the print volume floor for `WORLD_Z_BELOW_FLOOR`?**~~ **RESOLVED → 0.0mm, not configurable.** There is no `print_volume_z_min` or equivalent config in the current codebase. `WORLD_Z_BELOW_FLOOR` error is planned but not yet implemented. The test case for Step 9 should assert that an object with `translate(0,0,-5mm)` (world Z < 0) emits a fatal diagnostic with code `WORLD_Z_BELOW_FLOOR`.

~~4. **For multi-object LCM synchronization with transforms**, are the sync Z planes computed in world space, and do all objects' world-space Z ranges correctly contribute to the LCM computation?~~ **RESOLVED → Yes.** `layer-planner-default`'s `merge_different_heights()` collects all per-object Z values via `generate_object_layers(plan)`, which uses `plan.height` (from `object_height:{id}` config, i.e., world-space extent) and `plan.layer_height` (from `layer_height:{id}` config). `all_zs` is built from the union of all per-object world Z sequences, sorted and deduplicated. `last_z[i]` tracks the last world Z at which each object participated. All objects' world-space Z ranges correctly contribute to the LCM computation because `plan.height` (derived from `object_world_z_extent`) is world-space.
