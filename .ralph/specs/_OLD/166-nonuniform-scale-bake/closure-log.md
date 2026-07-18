# Closure Log: 166-nonuniform-scale-bake

## Step 1 — Downstream uniform-scale-assumption audit

- Verdict: **clean**
- Sites checked:
  - `crates/slicer-core/src/algos/prepass_slice.rs` (`transform_point3` call sites — multiple)
  - `crates/slicer-core/src/algos/mesh_analysis.rs` (`transform_point3` call site)
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (`transform_point3` call sites)
  - `crates/slicer-core/src/algos/paint_segmentation/painted_line_collection.rs` — `world_z_to_local` reads `matrix[10]` as the explicit Z-axis scale and `matrix[14]` as Z translation under its documented no-Z-shear assumption. This does not assume uniform X/Y/Z scale; for diagonal scale `(1, 2, 3)`, Z maps as `3 * local_z + tz`. 3MF loading bakes that transform before constructing an identity `ObjectMesh.transform`, so this path sees baked geometry and identity.
- Workspace grep (production, non-test, non-comment, in `crates/slicer-core/src`, `crates/slicer-runtime/src`, `crates/slicer-ir/src`): no uniform-scale extraction, `uniform_scale`, `scale_factor`, `paint_uniform`, `unit_scale`, or `sqrt(... transform ...)` patterns.
- Ground zero for `validate_non_uniform_scale`: only its definition in `crates/slicer-model-io/src/loader.rs` and `crates/slicer-model-io/tests/non_uniform_scale_tdd.rs` reference it. Zero production call sites.
- Step exit condition met. Proceeding to Step 2.

## Step 2 — Write baking tests (expected GREEN immediately)

- File: `crates/slicer-model-io/tests/nonuniform_scale_bake_tdd.rs` (new)
- 3 tests added: `nonuniform_scale_bakes_vertices_per_axis`, `nonuniform_scale_bakes_paint_triangles`, `uniform_scale_baking_unchanged`
- `cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd`: 3 passed; 0 failed.
- Step exit condition met. Proceeding to Step 3.

## Step 3 — Delete dead code

- `crates/slicer-model-io/src/loader.rs`: deleted `validate_non_uniform_scale` (function + doc comment), `ModelLoadError::NonUniformScaleUnsupported` (variant + doc comment), and its `Display` arm.
- `crates/slicer-model-io/tests/non_uniform_scale_tdd.rs`: deleted.
- `crates/slicer-model-io/Cargo.toml`: no `[[test]]` block for the deleted file (auto-discovery), left untouched.
- AC-3 grep: zero matches in `crates/` and `modules/`.
- `cargo check --workspace --all-targets`: clean.
- `cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd` re-run: 3 passed; 0 failed.
- Step exit condition met. Proceeding to Step 4.

## Step 4 — Regression sweep + docs/07 crosswalk

- AC-N1 (`cargo test -p slicer-model-io --test world_z_below_floor_tdd`): 10 passed; 0 failed.
- AC-N2 (`cargo test -p slicer-model-io` whole-crate regression): zero failed test results.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (only the pre-existing third-party future-incompatibility notice for `nom v3.2.1` and `quick-xml v0.22.0`).
- `docs/07_implementation_status.md`: TASK-272 row appended.
- Step exit condition met.

## Final review

- Full packet-scope `spec-review` initially returned **CHANGES REQUESTED**: AC-2 checked one transformed paint corner rather than every paint-triangle vertex, and the Step 1 inventory omitted `world_z_to_local`. Fixed: AC-2 now compares every scaled paint-triangle vertex with the unscaled fixture, and Step 1 records the Z-axis-only path and its baked-identity safety.
- Status: implemented.
