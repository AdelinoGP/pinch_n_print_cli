---
status: implemented
packet: 166-nonuniform-scale-bake
task_ids:
  - TASK-272
---

# 166-nonuniform-scale-bake

## Goal

Delete the dead `validate_non_uniform_scale` policy rejection and its `NonUniformScaleUnsupported` error variant from `crates/slicer-model-io/src/loader.rs`, and prove with tests that non-uniform-scale 3MF transforms are baked per-axis into mesh vertices and paint-data triangles by the existing transform-baking path.

## Problem Statement

The OrcaSlicer-frontend fork (fork-gaps wave-1 plan, Packet C / item 6) needs non-uniformly-scaled objects to slice. The plan framed `validate_non_uniform_scale` (defined in `crates/slicer-model-io/src/loader.rs`) as a "deliberate policy rejection"; **grounding falsified this** — the function has zero production call sites (only its definition and `tests/non_uniform_scale_tdd.rs` reference it), so the rejection never fires on the live load path. The 3MF loader already fully bakes build-item and component transforms into vertices via `apply_transform_to_mesh` (defined and invoked in `crates/slicer-model-io/src/loader.rs` during component resolution, driven from the build-item transform picked up near the top of the 3MF load path) and into paint strokes via `apply_transform_to_paint_data` (defined and invoked in the same file), then sets `ObjectMesh.transform` to identity at object assembly. The remaining work is therefore: (1) delete the dead validator, its `NonUniformScaleUnsupported` error variant, its `Display` arm, and its TDD test file so the false "unsupported" signal cannot be resurrected; (2) prove per-axis baking with positive tests that do not exist today; (3) audit downstream consumers for hidden uniform-scale assumptions.

## Architecture Constraints

- The loader stores mm-space `f64`→`f32` vertex coordinates in `IndexedTriangleSet`; the new tests assert mm-space `Point3` floats and need no unit conversion.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- `ObjectMesh.transform` remains identity after 3MF load (the `identity_transform()` convention at object assembly in `crates/slicer-model-io/src/loader.rs`); downstream consumers apply the full 4×4 via `slicer_core::transform_point3` (in `crates/slicer-core/src/lib.rs`), which is non-uniform-capable by construction. Do not change this contract.

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
