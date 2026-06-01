# ModularSlicer — Module Development SDK

## Overview

The SDK is a set of Rust crates that make writing, testing, and validating modules fast. A module author needs no knowledge of the host internals — only the SDK crates and the WIT interface.

The relevant crates live directly under the workspace root:

```
crates/
├── slicer-sdk/      # Core: re-exports WIT types, provides helpers, registers exports
│                    # (test support lives inside slicer-sdk under the `test` feature)
├── slicer-macros/   # Proc-macros: #[slicer_module], #[module_test]
└── pnp-cli/         # Single binary `pnp_cli`: includes `module new|diagnose|config-schema` verbs.
```

> **Source of truth.** This document is the authoring guide. For the exact
> trait signatures, output-builder methods, and return types, read
> `crates/slicer-sdk/src/traits.rs` and `crates/slicer-sdk/src/prelude.rs`.
> Examples below are illustrative; if a signature here disagrees with the
> SDK source, the SDK source wins.

---

## `slicer-sdk` Crate

### `Cargo.toml` (for a module author)

```toml
[package]
name = "my-infill-module"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]   # required for WASM component output

[dependencies]
slicer-sdk = "1.0"

[dev-dependencies]
slicer-sdk = { version = "1.0", features = ["test"] }

[profile.release]
opt-level = "s"    # optimize for size in WASM output
lto = true
```

### Module Entry Point (`#[slicer_module]`)

The `#[slicer_module]` macro generates the WIT export bindings, validates that the impl matches the declared stage, and wires up the `on-print-start` / `on-print-end` lifecycle.

```rust
use slicer_sdk::prelude::*;

// The struct name is arbitrary. The macro reads your manifest's stage field
// to determine which WIT export to implement.
pub struct MyInfillModule {
    // module-level state initialized in on_print_start
    // must be Send + Sync for parallel-safe modules
}

#[slicer_module]
impl LayerModule for MyInfillModule {

    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        // Validate config, initialize expensive resources once per print.
        // Called before the per-layer loop starts.
        let density = config.get_float("density").unwrap_or(0.15);
        if density <= 0.0 || density >= 1.0 {
            return Err(ModuleError::fatal(1, "density must be in (0, 1)"));
        }
        Ok(Self {})
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    // Implement the function matching your manifest's stage.
    // The macro enforces at compile time that you implement exactly one.
    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let density = config.get_float("density").unwrap_or(0.15) as f32;
        let pattern = config.get_string("pattern")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "schwartz-d".into());

        for region in regions {
            let infill_areas = region.infill_areas();
            let z = region.z();
            let layer_height = region.effective_layer_height();

            let paths = match pattern.as_str() {
                "schwartz-d"    => generate_schwartz_d(&infill_areas, z, density, layer_height),
                "fischer-koch-s"=> generate_fischer_koch(&infill_areas, z, density, layer_height),
                _ => return Err(ModuleError::non_fatal(2, format!("Unknown pattern: {}", pattern))),
            };

            for path in paths {
                output.push_sparse_path(path)
                    .map_err(|e| ModuleError::non_fatal(3, e))?;
            }
        }
        Ok(())
    }
}

fn generate_schwartz_d(
    areas: &[ExPolygon],
    z: f32,
    density: f32,
    layer_height: f32,
) -> Vec<ExtrusionPath3D> {
    // pure Rust geometry — no host calls needed for most infill generation
    todo!()
}
```

### PrePass Module Authoring Pattern

PrePass modules implement the `PrepassModule` trait. The `#[slicer_module]`
macro routes the module's manifest `stage.id` to the matching trait method.
Existing prepass stages each have a default-no-op trait method so a module
only needs to override the one for its own stage.

| Manifest `stage.id`               | Trait method called                           | Output builder              |
|-----------------------------------|-----------------------------------------------|-----------------------------|
| `PrePass::MeshSegmentation`       | `run_mesh_segmentation`                       | `MeshSegmentationOutput`    |
| `PrePass::MeshAnalysis`           | `run_mesh_analysis`                           | `MeshAnalysisOutput`        |
| `PrePass::LayerPlanning`          | `run_layer_planning`                          | `LayerPlanOutput`           |
| `PrePass::PaintSegmentation`      | `run_paint_segmentation`                      | `PaintSegmentationOutput`   |
| `PrePass::SeamPlanning`           | `run_seam_planning`                           | `SeamPlanningOutput`        |
| `PrePass::SupportGeometry`        | `run_support_geometry`                        | `SupportGeometryOutput`     |

Example: a `PrePass::SupportGeometry` module that emits one branch entry
per overhanging facet.

```rust
use slicer_sdk::prelude::*;

pub struct MySupportPlanner;

#[slicer_module]
impl PrepassModule for MySupportPlanner {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    // Real signature (see `crates/slicer-sdk/src/traits.rs::PrepassModule`):
    // takes `objects`, `layer_plan`, `region_segmentation`, `support_geometry`,
    // `output`, and `config` — read those for the current view types.
    fn run_support_geometry(
        &self,
        objects: &[MeshObjectView],
        _layer_plan: &LayerPlanView,
        _region_segmentation: &RegionSegmentationView,
        _support_geometry: &SupportGeometryView,
        output: &mut SupportGeometryOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        for obj in objects {
            // Compute branches across layers (top-down propagation, etc.)
            let entry = SupportPlanEntry {
                global_layer_index: 5,
                object_id: obj.object_id.clone(),
                region_id: "0".to_string(),
                branch_segments: vec![vec![
                    Point3WithWidth { x: 1.0, y: 2.0, z: 1.0, width: 0.4, flow_factor: 1.0 },
                    Point3WithWidth { x: 7.0, y: 8.0, z: 1.0, width: 0.4, flow_factor: 1.0 },
                ]],
            };
            output.push_support_plan(entry).map_err(|e| {
                ModuleError::fatal(1, format!("push_support_plan failed: {e}"))
            })?;
        }
        Ok(())
    }
}
```

The matching manifest declares `[stage] id = "PrePass::SupportGeometry"`,
`[claims] holds = ["support-planner"]`, `[ir-access] reads = ["MeshIR",
"SurfaceClassificationIR", "LayerPlanIR", "PaintRegionIR"]`, `writes =
["SupportPlanIR"]`, and `[module] wit-world = "slicer:world-prepass@1.0.0"`.

### Single-Stage-Per-Impl Constraint

`#[slicer_module]` is single-stage per impl block. The macro at
`crates/slicer-macros/src/lib.rs:43-52` raises a `compile_error!` when
`detect_stage_methods()` (lib.rs:106-119) finds more than one stage method on
the impl. There is no `#[slicer_module(stage = "...")]` attribute argument —
the only stage selector is the method name lookup against the `STAGES` table
in `crates/slicer-schema/src/lib.rs`. Additionally, the macro hardcodes the
WIT export module name per world (e.g.
`__slicer_prepass_world_export` at lib.rs:2024;
`__slicer_postpass_world_export` at lib.rs:689;
`__slicer_finalization_world_export` at lib.rs:989;
`__slicer_layer_world_export` at lib.rs:2306). Two `#[slicer_module]` impl
blocks in the same crate that target the same world will fail to link with
duplicate-symbol errors.

Workaround: when one trait permits multiple stages (e.g. `PrepassModule`
permits `run_mesh_analysis`, `run_paint_segmentation`, `run_mesh_segmentation`,
`run_layer_planning`, `run_seam_planning`, `run_support_geometry`), author one
sibling crate per stage. Each sibling overrides only the one stage method it
implements and relies on the trait's default `Ok(())` bodies for the rest. The
test guests `crates/slicer-runtime/test-guests/sdk-prepass-paintseg-guest/` and
`crates/slicer-runtime/test-guests/sdk-prepass-meshseg-guest/` are reference exemplars: each is a
standalone crate (empty `[workspace]` table; lists `slicer-sdk`, `slicer-ir`,
`slicer-schema`, `wit-bindgen` as deps) with exactly one `#[slicer_module]
impl PrepassModule for ...` block overriding `on_print_start` plus the one
prepass stage method.

Macro authors note (relevant when the prepass world inline WIT or the
`segmentation_helpers` quote block in `build_prepass_world_glue` is touched):
wit-bindgen 0.24 does not generate flat type re-exports for world-level `use`
items whose alias `TypeId` lacks `TypeInfo.owned`/`borrowed` (i.e. `modes_of()`
returns empty). The prepass world's paint_seg_arm constructs WIT geometry via
bare `Polygon { ... }` and `Point2 { ... }` names; for those names to resolve,
the inline WIT needs `use geometry.{ex-polygon, polygon, point2};` (declarative
intent) AND the `segmentation_helpers` quote block needs explicit
`use self::slicer::world_prepass::geometry::{Polygon, Point2};` statements
(matching the finalization-world pattern at lib.rs:998). Both are required;
the WIT-level fix alone is insufficient under wit-bindgen 0.24.

### SDK Type Re-Exports

The SDK re-exports all WIT-generated types under clean names:

```rust
// slicer_sdk::prelude::* exports all of these:
pub use slicer_wit::layer_module::{
    LayerModule,
    SliceRegionView,
    PerimeterRegionView,
    InfillOutputBuilder,
    PerimeterOutputBuilder,
    SlicePostprocessBuilder,
    ConfigView,
    ExPolygon, Polygon, Point2, Point3, Point3WithWidth,
    ExtrusionPath3D, ExtrusionRole, WallLoopView, WallLoopType,
};

// PrePass module authoring:
pub use slicer_sdk::prepass_builders::{
    LayerPlanOutput, MeshAnalysisOutput, MeshSegmentationOutput,
    PaintSegmentationOutput, SeamPlanningOutput, SupportGeometryOutput,
};
pub use slicer_sdk::prepass_types::{
    MeshObjectView, PaintLayerView, PaintSegmentationObjectView, SeamPlanEntry,
    SupportPlanEntry,
};
pub use slicer_sdk::traits::{LayerModule, PrepassModule, PaintRegionLayerView};

pub use slicer_sdk::error::ModuleError;
pub use slicer_sdk::geometry::*;   // convenience geometry helpers
pub use slicer_sdk::host;          // host service wrappers (log, raycast, clip_polygons, etc.)
```

### Consuming a PrePass IR from a Layer Stage

`Layer::Support` modules that want to emit pre-planned tree-support branches
(produced by guests of `PrePass::SupportGeometry`) read from `SupportPlanIR` via the
`PaintRegionLayerView` accessor:

```rust
fn run_support(
    &self,
    layer_index: u32,
    regions: &[SliceRegionView],
    paint: &PaintRegionLayerView,
    output: &mut SupportOutputBuilder,
    _config: &ConfigView,
) -> Result<(), ModuleError> {
    for region in regions {
        let planned = paint
            .support_plan_segments_for(region.object_id().as_str(), *region.region_id());
        if !planned.is_empty() {
            // Plan-driven path: emit committed branches with SupportMaterial role.
            for segment in planned {
                let mut path = segment.clone();
                path.role = ExtrusionRole::SupportMaterial;
                output.push_support_path(path).ok();
            }
            continue;
        }
        // Fallback path: per-layer filler (grid-MST, scan-line, etc.).
    }
    Ok(())
}
```

The module must declare `SupportPlanIR` in its manifest `[ir-access].reads`
to receive a non-empty plan. Modules whose algorithm is inherently per-layer
(e.g. `traditional-support`'s scan-line filler) intentionally omit the
declaration so the audit contract reflects that they ignore the plan.

For native host-side tests, `SliceRegionView` also exposes a convenience
`boundary_paint()` accessor over the documented WIT `boundary-paint` data so
perimeter generators can consume contour-parallel annotations ergonomically.

### `LayerCollectionBuilder` — Path Optimization (packet 32)

`Layer::PathOptimization` modules receive a `LayerCollectionView` (read-only)
and write their reorder decision into a `LayerCollectionBuilder` resource.

```rust
fn run_path_optimization(
    &self,
    layer: LayerCollectionView,
    output: &mut LayerCollectionBuilder,
) -> Result<(), ModuleError> {
    // Read current entity order.
    let mut entities: Vec<_> = layer.ordered_entities().collect();

    // Apply nearest-neighbour reorder (or any module-specific algorithm).
    let order = nearest_neighbour_order(&entities, layer.z());

    // Each tuple: (entity_index_in_input_order, reverse_direction).
    output.set_entity_order(order.iter().map(|&i| (i as u32, false)).collect());
    Ok(())
}
```

The host validates `set_entity_order`: indices must be unique and in
`0..ordered_entities().len()`. `get_ordered_entities()` returns the
previously-set ordering as `Vec<OrderedEntityView>` for diagnostic use.

### Host Service Wrappers

Direct calls to host services are ergonomic:

```rust
use slicer_sdk::host;

// Logging
host::log_info("Processing layer {}", layer_index);
host::log_warn("Density near limit: {}", density);

// Mesh queries
let surface_z: Option<f32> = host::raycast_z_down(object_id, x, y, start_z);
let normal: Option<Point3> = host::surface_normal_at(object_id, x, y, z);

// Geometry (delegates to host-side Clipper2)
let clipped: Vec<ExPolygon> = host::clip_polygons(&subject, &clip, ClipOp::Intersection);
let offset:  Vec<ExPolygon> = host::offset_polygons(&polys, -0.2, JoinType::Miter);
let simple:  Polygon        = host::simplify_polygon(&poly, 0.05);

// Timing
let t0 = host::now_us();
// ... work ...
host::log_debug("Took {} µs", host::now_us() - t0);

// Paint region queries
use slicer_sdk::paint::{PaintSemantic, PaintValue};

// Test whether a 2D point falls within any painted region of a given semantic.
// Returns the paint value if inside, None if outside all regions.
let fuzzy: Option<PaintValue> = host::point_in_paint_region(
    &paint_view,
    PaintSemantic::FuzzySkin,
    x_units, y_units,
);

// Test all points of a path segment against painted regions.
// Returns a Vec<bool> parallel to the segment points.
let flags: Vec<bool> = host::segment_in_paint_region(
    &paint_view,
    PaintSemantic::FuzzySkin,
    &segment_points,
);
```

#### Host Call Performance Contract (Normative)

- Boundary crossings are not free; modules must avoid per-point host calls in hot loops when batch alternatives exist.
- Prefer `segment_in_paint_region()` over repeated `point_in_paint_region()` for path annotations.
- For geometry transforms, aggregate polygon sets and invoke clipping/offset in fewer larger calls.

Recommended budgeting:

- Target host-service call count per module invocation should scale with region count, not vertex count.
- Module benchmarks should report boundary-crossing counts alongside elapsed time.

#### Config Lookup Complexity

- `ConfigView` lookups are expected to be amortized O(1).
- Modules should cache frequently used keys once per invocation (`density`, `pattern`, etc.) instead of querying repeatedly in inner loops.

### Geometry Helpers

Geometry utilities for module authors live in the `slicer-core` crate (add `slicer-core = { path = "..." }` to `[dependencies]` — already declared by `arachne-perimeters`, `classic-perimeters`, `rectilinear-infill`, `traditional-support`, and `tree-support`).

```rust
use slicer_core::{segment_path, distribute_points, path_length, seg_len_3d, flow_correction};
use slicer_ir::Point2;

// Subdivide a 2D segment so no piece exceeds `max_len_mm`; endpoints are preserved.
let segments: Vec<Point2> = segment_path(start, end, max_len_mm);

// Distribute exactly `count` evenly-spaced points along a polyline (endpoints kept).
let pts: Vec<Point3WithWidth> = distribute_points(&path.points, count);

// Total arc length of a 3D path in mm.
let len: f32 = path_length(&path.points);

// Euclidean length of a 3D segment given its component deltas.
let len_3d: f32 = seg_len_3d(dx, dy, dz);

// Extrusion volume correction factor for a segment with Z deviation.
let flow: f32 = flow_correction(dx, dy, dz);
```

### `ModuleError` Builder

```rust
// Fatal: host aborts the current slice
return Err(ModuleError::fatal(code, "message"));

// Non-fatal: host logs and continues with unmodified IR for this layer
return Err(ModuleError::non_fatal(code, "message"));

// Convenience — non-fatal from a string error
.map_err(ModuleError::from_str)?
```

### Module State Lifecycle (Normative)

- `on_print_start()` creates one logical module state per WASM instance.
- For `layer-parallel-safe` modules, multiple instances may exist simultaneously.
- Module state must not assume global singleton semantics across instances.
- `on_print_end()` is best-effort cleanup; correctness must not depend on it running after fatal abort.

### Stable Entity IDs (packet 39)

Each `PrintEntity` and `TravelMove` carries a `u64 entity_id` populated at construction by a per-layer `LayerEntityIdGen`. Module producers (perimeter, infill, support modules) receive the generator via the per-layer context and call `id_gen.next()` once per entity. Finalization receives fresh IDs from the host at insert time. Travel anchors reference entities by `entity_id`, not positional index, so finalization mutations cannot invalidate them. See `docs/02_ir_schemas.md` IR 10 "Stable entity IDs" for the full contract.

---

## Test Support (slicer-sdk feature)

Every module can be tested in complete isolation — no running host, no WASM runtime, no real mesh. Test-support APIs are part of `slicer-sdk` itself, gated behind a Cargo feature named `test`. The convention is to depend on the SDK twice — once for production code, and once as a dev-dependency with the `test` feature enabled:

```toml
[dependencies]
slicer-sdk = { path = "../../crates/slicer-sdk", default-features = false }

[dev-dependencies]
slicer-sdk = { path = "../../crates/slicer-sdk", features = ["test"] }
```

Per ADR-0004 (quoted from its `## Decision` section):

> Test-support APIs are owned by `slicer-sdk` and exposed as
> `slicer_sdk::test_support` (this packet, 77) and — in packet 78 — re-exported
> through a curated `slicer_sdk::test_prelude`. The module is gated behind a Cargo
> feature named `test` (also auto-enabled under `cfg(test)`), so production guest
> WASM builds pay no cost.
>
> The fold direction is deliberate: **test support lives inside slicer-sdk** so
> that module authors get test-support APIs from the same crate they use to author
> modules, the `#[slicer_module]` macro can emit a single fully-qualified path
> (`::slicer_sdk::test_support::…`) that always resolves, and the documented public
> surface becomes honest.

The `test_prelude` is whole-module gated with `#![cfg(any(test, feature = "test"))]` and lives separately from the production `prelude`. The production `slicer_sdk::prelude::*` stays test-free and is what `use slicer_sdk::prelude::*;` brings into scope inside module source files; the test helpers below come in via `use slicer_sdk::test_prelude::*;` from test modules only.

### Mock Host

```rust
use slicer_sdk::prelude::*;
use slicer_sdk::test_prelude::*;
use slicer_sdk::host;

#[module_test]
fn my_test() {
    MockHost::new()
        .with_raycast_hit(Some(4.8))
        .with_object_bounds(/* slicer_ir::BoundingBox3 { ... } */)
        .install();

    // ... module-under-test code that calls host::raycast_z_down ...
    let z = host::raycast_z_down("obj-1", 0.0, 0.0, 5.0);
    assert_eq!(z, Some(4.8));
}
```

The installed `MockHost` automatically routes through `slicer_sdk::host::log_warn`
once a capture sink is in place; check captured warnings with the static
`MockHost::log_contains("density near limit")`. For independent
"did this branch run?" assertions, use the `record_call` / `call_count`
counter — e.g. `host.call_count("clip_polygons")`.

### IR Fixture Builders

```rust
use slicer_sdk::test_prelude::*;

// Build a SliceRegionView from scratch
let region = SliceRegionViewBuilder::new()
    .object_id("test-obj")
    .region_id(42)
    .z(1.2)
    .effective_layer_height(0.2)
    .add_polygon(square_polygon(0.0, 0.0, 20.0))   // 20mm square
    .add_polygon(square_polygon(5.0, 5.0, 10.0))   // 10mm square inset (infill area)
    .build();

// Build a PerimeterRegionView
let perim_region = PerimeterRegionViewBuilder::new()
    .object_id("test-obj")
    .region_id(42)
    .add_outer_wall(rect_path(0.0, 0.0, 20.0, 0.4))
    .add_inner_wall(rect_path(0.4, 0.4, 19.2, 0.4))
    .build();

// Common shapes
let sq:   ExPolygon       = square_polygon(cx, cy, side);
let path: ExtrusionPath3D = rect_path(cx, cy, side, width);
```

The fixture builders are re-exported through `slicer_sdk::test_prelude`; the underlying definitions live at `slicer_sdk::test_support::fixtures` for callers that prefer fully-qualified paths.

### Config Fixture Builder

```rust
use slicer_sdk::test_prelude::*;   // re-exports ConfigViewBuilder

let config = ConfigViewBuilder::new()
    .float("density",            0.20)
    .string("pattern",           "schwartz-d")
    .int("multiline_count",      2)
    .float("marching_cell_size", 0.40)
    .build();
```

### Output Capture

```rust
use slicer_sdk::prelude::*;
use slicer_sdk::test_prelude::*;

let mut infill_output = InfillOutputCapture::new();
let mut perim_output  = PerimeterOutputCapture::new();

// Run the module
let module = MyInfillModule::on_print_start(&config).unwrap();
module.run_infill(0, &[region], &mut infill_output, &config).unwrap();

// Assert on captured output
let sparse = infill_output.sparse_paths();
assert!(!sparse.is_empty(), "expected infill paths to be generated");
assert!(sparse.iter().all(|p| p.role == ExtrusionRole::SparseInfill));
```

### Assertion Helpers

```rust
use slicer_sdk::test_prelude::*;

// All Z values equal the expected layer Z (module is planar)
assert_paths_planar(&sparse, 1.2, 1e-3);

// No path segment longer than max_len mm
assert_max_segment_length(&sparse, 2.0);

// All extrusion widths within expected range
assert_extrusion_width_range(&sparse, 0.3, 0.5);

// Paths lie within a polygon boundary
assert_paths_inside_polygon(&sparse, &boundary_polygon);

// No two paths intersect
assert_no_path_intersections(&sparse);
```

The assertion helpers, capture types, and other test seams are all owned by `slicer_sdk::test_support::*` and surface through `slicer_sdk::test_prelude::*`; nothing under those paths is reachable in a production guest WASM build, because the `test` feature is off by default and `cargo xtask build-guests` never enables it.

### Extended fixture surfaces (packet 79)

Packet 79 added a batch of additive fixture surfaces — all re-exported through `slicer_sdk::test_prelude` and routed through `slicer_sdk::test_support::fixtures` — covering shapes, tool-change events, seam metadata, layer-collection assembly, and top/bottom/bridge region setters:

- `rect_polygon(cx, cy, w, h)` — freestanding fixture producing an axis-aligned rectangular `ExPolygon` (complements `square_polygon` when width != height).
- `print_entity(...)` — freestanding fixture producing a `PrintEntity` for plan-IR / scheduler tests.
- `tool_change(...)` — freestanding fixture producing a tool-change event, used together with `LayerCollectionFixtureBuilder` to assemble `LayerCollectionIR` inputs for skirt-brim and wipe-tower-style module tests that need multi-layer / multi-extruder context.
- `seam_candidate(...)` — freestanding fixture producing a seam candidate record for seam-placement module tests.
- `LayerCollectionFixtureBuilder` — struct that assembles `LayerCollectionIR` from per-layer pieces (and the `tool_change` fixture above), enabling skirt-brim / wipe-tower-style tests that exercise cross-layer behaviour without spinning up a real pipeline.
- `PerimeterRegionViewBuilder::add_outer_wall_with_flags(...)` — method overload of `add_outer_wall` that lets tests stamp seam/overhang/bridge flag bits onto the outer-wall path being added.
- `SliceRegionViewBuilder` gains seven new top/bottom/bridge setters — `top_shell_index`, `top_solid_fill`, `bottom_shell_index`, `bottom_solid_fill`, `is_bridge`, `bridge_areas`, and `bridge_orientation_deg` — covering the shell-classification and bridge-detection fields that top/bottom infill and bridging modules consume.

---

## `slicer-macros` Crate

### `#[slicer_module]`

Applied to an `impl LayerModule for T` block. At compile time it:

- Reads the module's `Cargo.toml` to find the manifest path
- Parses the manifest's `[stage]` field
- Verifies the impl contains exactly the function matching that stage
- Emits a compile error if a mismatch is found
- Generates the WIT WASM export bindings

```rust
// Compile error example:
// manifest declares stage = "Layer::Infill"
// but impl provides run_perimeters() instead of run_infill()
//
// error[E0000]: stage mismatch
//   manifest declares `Layer::Infill` but impl provides `run_perimeters`
//   expected: fn run_infill(...)
```

### `#[module_test]`

Wrapper around `#[test]` that automatically sets up the mock host, installs the SDK's test panic handler, and resets global state between tests.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use slicer_sdk::test_prelude::*;

    #[module_test]
    fn test_schwartz_d_fills_square() {
        let config = ConfigViewBuilder::new()
            .string("pattern", "schwartz-d")
            .float("density", 0.20)
            .build();

        let region = SliceRegionViewBuilder::new()
            .z(1.0)
            .effective_layer_height(0.2)
            .add_infill_area(square_polygon(0.0, 0.0, 20.0))
            .build();

        let module = MyInfillModule::on_print_start(&config).unwrap();
        let mut output = InfillOutputCapture::new();
        module.run_infill(0, &[region], &mut output, &config).unwrap();

        assert!(!output.sparse_paths().is_empty());
        assert_paths_inside_polygon(output.sparse_paths(), &square_polygon(0.0, 0.0, 20.0));
        assert_paths_planar(output.sparse_paths(), 1.0, 1e-3);
    }

    #[module_test]
    fn test_invalid_density_returns_error() {
        let config = ConfigViewBuilder::new()
            .float("density", 0.0)  // invalid
            .build();
        let result = MyInfillModule::on_print_start(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().fatal);
    }
}
```

---

## `pnp_cli` — Developer CLI

### Building a module (canonical two-step)

The canonical path for compiling a module to a Component Model binary is:

```bash
cargo build --target wasm32-unknown-unknown --release
wasm-tools component new \
    target/wasm32-unknown-unknown/release/<name_underscored>.wasm \
    -o target/slicer/<name_kebab>.wasm
```

`<name_underscored>` is the package name with hyphens replaced by underscores — the Rust `cdylib` artifact naming convention. `<name_kebab>` is the package name as declared in `Cargo.toml`.

`pnp_cli` deliberately has no `build` verb — `cargo` is the canonical build tool. Wrapping it would duplicate flag surface and add failure modes without adding value.

> **Workspace contributors** rebuilding the in-tree guest set (`modules/core-modules/**/wit-guest` and `crates/slicer-runtime/test-guests/*`) should use `cargo xtask build-guests`. Freshness can be verified with `cargo xtask build-guests --check`. This is generative — adding a new guest crate matching the validated discovery predicate (cdylib + `[workspace]` sentinel + correct dep shape) is picked up automatically; no hardcoded module list to maintain.

### Other verbs

```
pnp_cli module new <module-name> [--stage <stage>]
  Scaffold a new module with the correct directory structure,
  Cargo.toml, manifest template, and a passing test suite.

  Options:
    --stage   Layer::Infill | Layer::Perimeters | Layer::PerimetersPostProcess |
              Layer::InfillPostProcess | Layer::SlicePostProcess |
              PrePass::MeshAnalysis | PrePass::LayerPlanning |
              PostPass::GCodePostProcess | PostPass::TextPostProcess
              (default: Layer::Infill)

pnp_cli module test [-- <cargo-test-args>]
  Run the module's test suite via `cargo nextest run`.
  Tests run natively (not in WASM) against the mock host.
  Coverage report written to target/slicer/coverage/.

pnp_cli module validate
  Validate the module manifest without building.
  Checks:
    - TOML schema validity
    - Stage ID is a known stage
    - Config field types and ranges
    - Cross-validate expression syntax
    - Claim names are recognized
    - wit-world version is supported by the current SDK

pnp_cli slice --model <file.stl> [--config <config.json>] [--output <file.gcode>]
  Slice a model using the loaded module set.
  Output: writes G-code to --output (default: stdout)

pnp_cli module benchmark --model <file.stl> [--layers <N>]
  Run the module against N layers and report:
    - median / p95 / p99 time per layer invocation
    - WASM boundary crossing overhead
    - Peak memory per layer
```

### Scaffolded Directory Structure (`pnp_cli module new my-infill --stage Layer::Infill`)

```
my-infill/
├── Cargo.toml
├── my-infill.toml            # manifest (stage, claims, config schema)
├── src/
│   └── lib.rs                # module impl with #[slicer_module]
└── tests/
    ├── basic.rs              # basic correctness tests (auto-generated stubs)
    └── fixtures/
        └── square_20mm.json  # pre-built SliceRegionView fixture
```

---

## Module Development Workflow

```
1. pnp_cli module new my-infill --stage Layer::Infill
   └─ Scaffolds directory, generates passing test stub

2. Edit my-infill.toml
   └─ Add config schema fields, set claims, set compatibility

3. pnp_cli module validate
   └─ Catches manifest errors before writing any Rust

4. Write failing tests first (TDD)
   └─ tests/basic.rs: assert the geometry you expect

5. cargo test  (runs natively — fast feedback)
   └─ Tests fail (red)

6. Implement run_infill() in src/lib.rs
   └─ cargo test  (tests pass — green)

7. cargo build --target wasm32-unknown-unknown --release && wasm-tools component new target/wasm32-unknown-unknown/release/my_infill.wasm -o target/slicer/my-infill.wasm
   └─ Compiles to target/slicer/my-infill.wasm

8. pnp_cli slice --model test_model.stl
   └─ Verify G-code output visually in slicer frontend

9. pnp_cli module benchmark --model test_model.stl --layers 50
   └─ Confirm performance within acceptable range
```

---

## Worked Example: Fuzzy Skin as a Native Module

```toml
# fuzzy-skin.toml
[module]
id           = "com.core.fuzzy-skin"
version      = "1.0.0"
display-name = "Fuzzy Skin"
wit-world    = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::PerimetersPostProcess"

[ir-access]
reads  = [
    "PerimeterIR.regions.walls.feature_flags",
    "PerimeterIR.regions.walls.path",
]
writes = ["PerimeterIR.regions.walls.path"]

[claims]
holds = ["fuzzy-skin-generator"]

[compatibility]
incompatible-with = ["*.nonplanar-wall-modulator"]

[config.schema]

  [config.schema.thickness]
  type    = "float"
  default = 0.3
  min     = 0.05
  max     = 2.0
  display = "Fuzzy Skin Thickness (mm)"
  group   = "Fuzzy Skin"

  [config.schema.point_distance]
  type    = "float"
  default = 0.8
  min     = 0.1
  max     = 3.0
  display = "Point Distance (mm)"
  group   = "Fuzzy Skin"

  [config.schema.apply_to_all]
  type    = "bool"
  default = false
  display = "Apply to Entire Model"
  description = "If false, apply only to painted regions"
  group   = "Fuzzy Skin"

[config.overridable-per-region]
keys = ["thickness", "point_distance", "apply_to_all"]

[hints]
layer-parallel-safe = true
```

```rust
// src/lib.rs
use slicer_sdk::prelude::*;

pub struct FuzzySkinModule;

#[slicer_module]
impl LayerModule for FuzzySkinModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_wall_postprocess(
        &self,
        _layer_index: u32,
        regions: &[PerimeterRegionView],
        output: &mut PerimeterOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let thickness     = config.get_float("thickness").unwrap_or(0.3) as f32;
        let point_dist    = config.get_float("point_distance").unwrap_or(0.8) as f32;
        let apply_to_all  = config.get_bool("apply_to_all").unwrap_or(false);

        for region in regions {
            let mut walls = region.wall_loops();
            for wall in &mut walls {
                let should_apply = apply_to_all
                    || wall.feature_flags.iter().any(|f| f.fuzzy_skin);

                if !should_apply {
                    output.push_wall_loop(wall.clone())
                        .map_err(|e| ModuleError::non_fatal(1, e))?;
                    continue;
                }

                let mut fuzzed = wall.clone();
                fuzzed.path = apply_fuzzy_skin(
                    &wall.path,
                    &wall.feature_flags,
                    apply_to_all,
                    thickness,
                    point_dist,
                );
                output.push_wall_loop(fuzzed)
                    .map_err(|e| ModuleError::non_fatal(2, e))?;
            }
        }
        Ok(())
    }
}

fn apply_fuzzy_skin(
    path: &ExtrusionPath3D,
    flags: &[WallFeatureFlags],
    apply_to_all: bool,
    thickness: f32,
    point_dist: f32,
) -> ExtrusionPath3D {
    // For each segment: if apply_to_all OR flags[i].fuzzy_skin,
    // subdivide the segment and add random perpendicular XY perturbation.
    // Otherwise copy the segment geometry through unchanged.
    // Pure geometry — no host calls needed.
    todo!()
}
```

---

## Finalization Mutation API (`PostPass::LayerFinalization`)

`PostPass::LayerFinalization` modules hold exclusive mutable access to the
full `Vec<LayerCollectionIR>` after the parallel per-layer stage completes.
Four mutation primitives are available on the finalization output builder
(packets 40 + 41):

### `push_entity_with_priority`

```rust
output.push_entity_with_priority(
    layer_index: u32,
    path: ExtrusionPath3D,
    region_key: RegionKey,
    priority: u32,
) -> Result<(), String>
```

Inserts a new `PrintEntity` into the layer at `layer_index`. The
`ExtrusionRole` is carried inside `ExtrusionPath3D`, so there is no
separate `role` parameter. The host stamps a fresh `entity_id` at insert
time (packet 39 + packet 40). Use `ExtrusionRole::default_priority()` as
`priority` if no override is needed — see `docs/02_ir_schemas.md` IR 10
"Extrusion-role default priority" for the full priority table.

### `modify_entity`

```rust
output.modify_entity(
    layer: u32,
    entity_id: u64,
    mutation: EntityMutation,
) -> Result<(), String>
```

Applies a serialisable `EntityMutation` to the entity identified by
`entity_id`. The mutation variants currently defined in
`crates/slicer-sdk/src/traits.rs::EntityMutation` are:

| Variant | Effect |
|---|---|
| `SetSpeedFactor(f32)` | Override the entity's path-level speed factor. |
| `SetFlowFactor(f32)`  | Scale the entity's extrusion flow. |

Every variant is serialisable across the WIT boundary. This replaces the
closure-based draft from packet 40 so that all mutations are fully
serialisable (packet 41 refactor).

### `sort_layer_by`

```rust
output.sort_layer_by(
    layer: u32,
    key: SortKey,
) -> Result<(), String>
```

Reorders the layer's entities by a serialisable `SortKey` (packet 41).
The current `crates/slicer-sdk/src/traits.rs::SortKey` variants are
`ByPriorityAndEntityId`, `ByEntityId`, and `ByObjectIdThenPriority`. Sort
is stable — ties preserve insertion order.

### `insert_synthetic_layer_after`

```rust
output.insert_synthetic_layer_after(
    idx: u32,
    data: SyntheticLayerData,
) -> Result<(), String>
```

Inserts a new `LayerCollectionIR` after the layer at index `idx`. Useful
for wipe-tower or purge slices (packet 41). `SyntheticLayerData` carries
the new layer's Z plus its extrusion paths:

```rust
pub struct SyntheticLayerData {
    pub z: f32,
    pub paths: Vec<ExtrusionPath3D>,
}
```

### `push_entity_to_layer` (canonical finalization surface, packet 16)

```rust
output.push_entity_to_layer(
    layer_index: u32,
    path: ExtrusionPath3D,
    region_key: RegionKey,
) -> Result<(), String>
```

Convenience wrapper around `push_entity_with_priority` that records the
entity at priority `0`. The `ExtrusionRole` is carried inside
`ExtrusionPath3D`. This is the canonical surface for live skirt / brim /
wipe-tower emission introduced in packet 16; the legacy
`process(&mut Vec<LayerCollectionIR>)` vector-mutation path is retired and
must not be reintroduced.

Synthetic region-key convention for finalization-stage geometry:

| Geometry kind | `RegionKey.object_id`         |
|---------------|-------------------------------|
| Skirt         | `"__skirt__"`                 |
| Brim          | `"__brim__"`                  |
| Wipe tower    | `"__wipe_tower__"`            |
| Prime tower   | `"__prime_tower__"`           |

The host emits `T{n}` tool changes only at transitions in `RegionKey.region_id`,
not `object_id`; synthetic objects therefore never trigger spurious tool
changes. `RegionKey.region_id` for synthetic finalization entities is `0`.

## Layer Stage Module Surface Rejections

### `Layer::PathOptimization` rejects fan-speed and cooling overrides (packet 19, locked)

`Layer::PathOptimization` is **not** an emit-time fan / cooling surface. Calls
to fan-speed or cooling-related output-builder methods at this stage return
fatal `FatalModule` diagnostics. This is an **architectural lock** — cooling
lives at `PostPass::LayerFinalization` (packet 53 `part-cooling` module). The
lock is retained even after packet 53 because the per-layer surface cannot
see neighbouring layers' timing budgets, which are required for valid fan
modulation.

Accepted at `Layer::PathOptimization`:
`set-entity-order`, `push-tool-change` (deferred — see `docs/04_host_scheduler.md`
"Deferred Tool-Change Queue"), `push-comment`, `push-raw`, `push-z-hop`.

Rejected at `Layer::PathOptimization`:
`push-fan-speed`, `push-temperature`, `push-move`, `push-retract`,
`push-unretract` — these belong to either `PostPass::LayerFinalization`
(fan/temperature/cooling) or the live wall/infill stage outputs
(move/retract/unretract).

### `Layer::Support` paint precedence (packet 13)

Modules holding the `support-generator` claim must apply paint-driven
overrides **before** any geometric overhang test:

1. `PaintSemantic::SupportBlocker` region → emit nothing; skip the geometry
   path entirely for the blocked region.
2. `PaintSemantic::SupportEnforcer` region → emit support, regardless of
   `needs_support` or the configured overhang angle.
3. Otherwise → run the module's algorithm (overhang threshold, planner
   consumption).

SDK helpers `PaintRegionLayerView::support_enforcer_polygons_for(...)` and
`support_blocker_polygons_for(...)` return the per-region polygons; modules
should intersect / difference them against the layer's slice polygons before
the geometric scan.

### G-code Serializer Helpers

#### Relative vs. absolute extrusion (packet 54)

`DefaultGCodeSerializer::with_extrusion_mode(mode: ExtrusionMode) -> Self`
where `ExtrusionMode { Absolute, Relative }`.

- Default is `Relative` (M83). The serializer emits `M82` / `M83` **once**
  in the preamble and inserts `G92 E0` on mode transitions.
- The config key `use_relative_e_distances` selects the default mode
  (`true` → Relative / M83, `false` → Absolute / M82). Modules typically
  do not need to override this — it is a printer-level setting resolved
  before finalization runs.

---

## SDK Versioning

The SDK crate version tracks the WIT world version it targets:

- `slicer-sdk 1.x` → targets `slicer:world-layer@1.x`
- `slicer-sdk 2.x` → targets `slicer:world-layer@2.x` (breaking change)

The host specifies its supported WIT world version in the validation step. Modules built against an older SDK minor version always load on a newer host (additive compatibility). Modules built against a newer major version are rejected with a clear error.
