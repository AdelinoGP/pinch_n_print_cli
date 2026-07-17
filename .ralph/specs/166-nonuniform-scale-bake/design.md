# Design: 166-nonuniform-scale-bake

## Controlling Code Paths

- Primary code path: 3MF load → `parse_3mf_model_xml` builds objects → build-item transform picked up at `crates/slicer-model-io/src/loader.rs:1911` (`item.transform.unwrap_or_else(identity_3mf_transform)`) → component resolution applies `apply_transform_to_mesh` (loader.rs:517) and `apply_transform_to_paint_data` (loader.rs:520) with the composed transform → `ObjectMesh` constructed with `identity_transform()` (loader.rs:228).
- Dead code being removed: `validate_non_uniform_scale` (loader.rs:2551-2567), `ModelLoadError::NonUniformScaleUnsupported` (loader.rs:49-56), its `Display` arm (loader.rs:81-84).
- Neighboring tests/fixtures: `crates/slicer-model-io/tests/model_loader_tdd.rs` builds in-memory 3MF zips via `zip::ZipWriter` writing `3D/3dmodel.model` XML (see its helper around lines 179-184) — the new `nonuniform_scale_bake_tdd.rs` copies this pattern with a `<item ... transform="...">` carrying per-axis scale. `tests/world_z_below_floor_tdd.rs` is the sibling-validator regression oracle. `tests/non_uniform_scale_tdd.rs` is deleted.
- OrcaSlicer comparison: not applicable — no C++ port; Orca supports non-uniform scale natively, and this packet removes a PNP-only restriction.

## Architecture Constraints

- The loader stores mm-space `f64`→`f32` vertex coordinates in `IndexedTriangleSet`; the new tests assert mm-space `Point3` floats and need no unit conversion.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- `ObjectMesh.transform` remains identity after 3MF load (loader.rs:228 convention); downstream consumers apply the full 4×4 via `slicer_core::transform_point3` (`crates/slicer-core/src/lib.rs:67`), which is non-uniform-capable by construction. Do not change this contract.

## Code Change Surface

- Selected approach: pure deletion of dead policy code plus positive proof tests. Grounding showed the validator has zero production call sites, so no call-site unwiring is needed — `cargo check --workspace --all-targets` is the safety net for any missed reference.
- Exact changes:
  - `crates/slicer-model-io/src/loader.rs`: remove the `NonUniformScaleUnsupported` variant + doc comment (around lines 46-56), its `Display` arm (around lines 81-84), and `validate_non_uniform_scale` + doc comment (around lines 2538-2567). Also remove any `#[cfg(test)]` unit tests inside loader.rs that reference the deleted symbols (grep before editing; the in-file test helper `make_object` at loader.rs:2825 may serve other tests — keep it if so).
  - Delete `crates/slicer-model-io/tests/non_uniform_scale_tdd.rs`.
  - `crates/slicer-model-io/Cargo.toml`: remove a `[[test]] name = "non_uniform_scale_tdd"` block only if present (verify first; test binaries may be auto-discovered).
  - New `crates/slicer-model-io/tests/nonuniform_scale_bake_tdd.rs`: three tests — `nonuniform_scale_bakes_vertices_per_axis` (scale (1,2,3) build-item transform; vertex (1,1,1) → (1,2,3) ± 1e-4; `ObjectMesh.transform` identity), `nonuniform_scale_bakes_paint_triangles` (same 3MF with a `paint_color` stroke; stroke triangle vertices scaled per-axis), `uniform_scale_baking_unchanged` (scale 2 uniform; vertex (1,1,1) → (2,2,2) ± 1e-4).
- Rejected alternatives:
  - Keeping the validator but never calling it: leaves a misleading "unsupported" API and error string that a future caller could rewire; deleting is safer and matches the plan.
  - Repurposing `non_uniform_scale_tdd.rs` in place: the file's 10 tests all assert rejection; a fresh file with a baking-oriented name is clearer and keeps AC-3's grep meaningful.

## Files in Scope (read + edit)

- `crates/slicer-model-io/src/loader.rs` — role: owns the dead validator and error variant; expected change: three deletions, no additions.
- `crates/slicer-model-io/tests/nonuniform_scale_bake_tdd.rs` — role: new proof tests; expected change: new file, 3 tests.
- `crates/slicer-model-io/tests/non_uniform_scale_tdd.rs` — role: obsolete rejection tests; expected change: deleted.
- (`crates/slicer-model-io/Cargo.toml` — conditional 4th edit only if a `[[test]]` block names the deleted test file.)

## Read-Only Context

- `crates/slicer-model-io/src/loader.rs` (2980 lines) — lines 40-95 (error enum + Display), 445-530 (transform bake path), 1890-1930 (build-item transform), 2470-2610 (validators + `identity_transform`) only.
- `crates/slicer-model-io/tests/model_loader_tdd.rs` — lines 150-260 only, to copy the in-memory 3MF zip construction pattern.
- `crates/slicer-model-io/tests/world_z_below_floor_tdd.rs` — skim test names only (regression oracle; do not edit).

## Out-of-Bounds Files

- `crates/slicer-core/`, `crates/slicer-runtime/`, `crates/slicer-ir/` — audit via delegated LOCATIONS/FACT only; never browse or edit.
- `.claude/worktrees/**` — stale worktree copies of the same files; never load or count in greps.
- `OrcaSlicerDocumented/` — not applicable; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.

## Expected Sub-Agent Dispatches

- Question: "Does any consumer of `ObjectMesh.transform` or mesh geometry extract a single scalar scale factor or otherwise assume uniform scale? Check `transform_point3` call sites in `crates/slicer-core/src/algos/prepass_slice.rs` (lines 100, 153, 554, 771), `mesh_analysis.rs:120`, `paint_segmentation/mod.rs:1012`, `paint_segmentation/painted_line_collection.rs:349`, plus a workspace grep for `sqrt` over transform columns and identifiers matching `scale(_factor)?[^_xyz]`"; scope: `crates/slicer-core/src`, `crates/slicer-runtime/src`, `crates/slicer-ir/src`; return: `LOCATIONS` (suspect sites) + closing `FACT` (clean / not clean); purpose: Step 1 audit.
- Question: "Does `crates/slicer-model-io/Cargo.toml` declare a `[[test]]` block for `non_uniform_scale_tdd`?"; scope: `crates/slicer-model-io/Cargo.toml`; return: `FACT`; purpose: Step 2 cleanup.
- All `cargo` invocations (check/clippy/test) dispatched with `FACT pass/fail` returns.

## Data and Contract Notes

- IR/manifest contracts: none touched. `ModelLoadError` is a `slicer-model-io` public enum, not a WIT type; removing a variant is a source-level breaking change caught by `cargo check --workspace --all-targets`.
- WIT boundary: none.
- Determinism/scheduler constraints: none — load-time-only change; uniform-scale outputs must be byte-identical (AC-4).

## Locked Assumptions and Invariants

- `validate_non_uniform_scale` has zero production call sites (grounded 2026-07-17 via workspace grep). If Step 1's audit finds a caller introduced since, stop and re-scope — do not silently unwire it.
- `ObjectMesh.transform` stays identity for 3MF-loaded objects; the packet must not start baking or un-baking anything new.

## Risks and Tradeoffs

- Risk: a downstream consumer with a hidden uniform-scale assumption (e.g. an inscribed-sphere or offset radius derived from one axis) produces silently wrong geometry for non-uniform input. Mitigated by the Step 1 audit ordered before deletion, and by the grounded fact that all known consumers use full-matrix `transform_point3`.
- Risk: deleting a public API (`validate_non_uniform_scale` is `pub`) breaks an out-of-tree caller. Accepted: the fork calls `pnp_cli`, not this crate's Rust API.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: the Step 1 uniform-scale-assumption audit; `LOCATIONS` capped at 20 entries + closing `FACT`.

## Open Questions

- [FWD] If the 3MF paint stroke fixture proves awkward to author inline (paint XML attributes are verbose), the implementer may base the AC-2 test on the smallest existing painted fixture pattern in `tests/paint_studio_output_tdd.rs` instead of hand-written XML — the assertion (per-axis scaled stroke triangles) must not change.
