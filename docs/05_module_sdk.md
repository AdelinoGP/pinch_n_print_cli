# ModularSlicer — Module Development SDK

## Overview

The SDK is a set of Rust crates that make writing, testing, and validating modules fast. A module author needs no knowledge of the host internals — only the SDK crates and the WIT interface.

```
slicer-sdk/
├── crates/
│   ├── slicer-sdk/        # Core: re-exports WIT types, provides helpers, registers exports
│   ├── slicer-test/       # Test harness: mock host, IR builders, assertion helpers
│   └── slicer-macros/     # Proc-macros: #[slicer_module], #[module_test]
└── cli/
    └── slicer-cli/        # `slicer new` / `slicer build` / `slicer test` / `slicer validate`
```

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
slicer-test = "1.0"

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
| `PrePass::SupportGeneration`      | `run_support_generation`                      | `SupportGenerationOutput`   |

Example: a `PrePass::SupportGeneration` module that emits one branch entry
per overhanging facet.

```rust
use slicer_sdk::prelude::*;

pub struct MySupportPlanner;

#[slicer_module]
impl PrepassModule for MySupportPlanner {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_support_generation(
        &self,
        objects: &[MeshObjectView],
        output: &mut SupportGenerationOutput,
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

The matching manifest declares `[stage] id = "PrePass::SupportGeneration"`,
`[claims] holds = ["support-planner"]`, `[ir-access] reads = ["MeshIR",
"SurfaceClassificationIR", "LayerPlanIR", "PaintRegionIR"]`, `writes =
["SupportPlanIR"]`, and `[module] wit-world = "slicer:world-prepass@1.0.0"`.

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
    PaintSegmentationOutput, SeamPlanningOutput, SupportGenerationOutput,
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
(produced by `PrePass::SupportGeneration`) read from `SupportPlanIR` via the
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

### Layer Stage Module Surface Rejections

Not all module surfaces are supported on the live execution path. Fan-speed and
cooling overrides are intentionally unsupported on the live Layer::PathOptimization
surface (TASK-152c). Module authors seeking to modulate cooling behavior during
path optimization should emit their intent as part of the entity sequence
decision (e.g. via role annotations) rather than via direct fan-speed control,
since the live surface does not expose a cooling override mechanism.

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

The SDK provides zero-cost geometry utilities built on top of the WIT types:

```rust
use slicer_sdk::geometry::{segment_path, distribute_points, path_length};

// Segment a straight line into chunks of at most `max_len` mm
let segments: Vec<(f32, f32)> = segment_path(x1, y1, x2, y2, max_len);

// Distribute N evenly-spaced points along a polyline
let pts: Vec<Point3WithWidth> = distribute_points(&path.points, n);

// Total arc length of a 3D path in mm
let len: f32 = path_length(&path.points);

// Compute 3D segment length with Z deviation
let len_3d: f32 = slicer_sdk::geometry::seg_len_3d(dx, dy, dz);

// Extrusion volume correction for non-planar segment
let flow: f32 = slicer_sdk::geometry::flow_correction(dx, dy, dz);
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

---

## `slicer-test` Crate

Every module can be tested in complete isolation — no running host, no WASM runtime, no real mesh.

### Mock Host

```rust
use slicer_test::MockHost;

let mut host = MockHost::new();

// Pre-program raycast responses
host.set_raycast_z_down("obj-1", 10.0, 20.0, 5.0, Some(4.8));
host.set_raycast_z_down("obj-1", 10.0, 20.0, 5.0, None);  // no surface

// Pre-program surface normals
host.set_surface_normal("obj-1", 0.0, 0.0, 1.0, Some(Point3 { x: 0.0, y: 0.0, z: 1.0 }));

// Capture log output for assertions
host.with_logging();
assert!(host.log_contains(LogLevel::Warn, "density near limit"));

// Verify polygon ops were called
assert_eq!(host.clip_polygons_call_count(), 3);
```

### IR Fixture Builders

```rust
use slicer_test::fixtures::*;

// Build a SliceRegionView from scratch
let region = SliceRegionViewBuilder::new()
    .object_id("test-obj")
    .region_id("42")
    .z(1.2)
    .effective_layer_height(0.2)
    .add_polygon(square_polygon(0.0, 0.0, 20.0))   // 20mm square
    .add_polygon(square_polygon(5.0, 5.0, 10.0))   // 10mm square inset (infill area)
    .build();

// Build a PerimeterRegionView
let perim_region = PerimeterRegionViewBuilder::new()
    .object_id("test-obj")
    .region_id("42")
    .add_outer_wall(rect_path(0.0, 0.0, 20.0, 0.4))
    .add_inner_wall(rect_path(0.4, 0.4, 19.2, 0.4))
    .build();

// Common polygon shapes
let sq:   ExPolygon = square_polygon(cx, cy, side);
let rect: ExPolygon = rect_polygon(x, y, w, h);
let circ: ExPolygon = circle_polygon(cx, cy, r, segments);
```

### Config Fixture Builder

```rust
use slicer_test::fixtures::ConfigViewBuilder;

let config = ConfigViewBuilder::new()
    .float("density",            0.20)
    .string("pattern",           "schwartz-d")
    .int("multiline-count",      2)
    .float("marching-cell-size", 0.40)
    .build();
```

### Output Capture

```rust
use slicer_test::capture::*;

let mut infill_output = InfillOutputCapture::new();
let mut perim_output  = PerimeterOutputCapture::new();

// Run the module
let module = MyInfillModule::on_print_start(&config).unwrap();
module.run_infill(0, &[region], &mut infill_output, &config).unwrap();

// Assert on captured output
let sparse = infill_output.sparse_paths();
assert!(!sparse.is_empty(), "expected infill paths to be generated");
assert!(sparse.iter().all(|p| p.role == ExtrusionRole::SparseInfill));

// Assert total path length is within expected range
let total_len: f32 = sparse.iter()
    .map(|p| path_length(&p.points))
    .sum();
assert!(total_len > 50.0, "total path length too short: {}", total_len);
```

### Assertion Helpers

```rust
use slicer_test::assert_paths::*;

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
    use slicer_test::prelude::*;

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

## `slicer-cli` — Developer CLI

```
slicer new <module-name> [--stage <stage>]
  Scaffold a new module with the correct directory structure,
  Cargo.toml, manifest template, and a passing test suite.

  Options:
    --stage   Layer::Infill | Layer::Perimeters | Layer::PerimetersPostProcess |
              Layer::InfillPostProcess | Layer::SlicePostProcess |
              PrePass::MeshAnalysis | PrePass::LayerPlanning |
              PostPass::GCodePostProcess | PostPass::TextPostProcess
              (default: Layer::Infill)

slicer build [--release]
  Compile the current module to WASM.
  Runs `cargo build --target wasm32-unknown-unknown [--release]`
  followed by `wit-component` to produce the Component Model binary.
  Output: target/slicer/<module-name>.wasm

slicer test [-- <cargo-test-args>]
  Run the module's test suite via `cargo nextest run`.
  Tests run natively (not in WASM) against the mock host.
  Coverage report written to target/slicer/coverage/.

slicer validate
  Validate the module manifest without building.
  Checks:
    - TOML schema validity
    - Stage ID is a known stage
    - Config field types and ranges
    - Cross-validate expression syntax
    - Claim names are recognized
    - wit-world version is supported by the current SDK

slicer run --model <file.stl> [--config <config.json>] [--output <file.gcode>]
  Run the local module against a real model using a host instance.
  Requires the host binary to be installed.
  Useful for integration testing during development.
  Output: writes G-code to --output (default: stdout)

slicer benchmark --model <file.stl> [--layers <N>]
  Run the module against N layers and report:
    - median / p95 / p99 time per layer invocation
    - WASM boundary crossing overhead
    - Peak memory per layer
```

### Scaffolded Directory Structure (`slicer new my-infill --stage Layer::Infill`)

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
1. slicer new my-infill --stage Layer::Infill
   └─ Scaffolds directory, generates passing test stub

2. Edit my-infill.toml
   └─ Add config schema fields, set claims, set compatibility

3. slicer validate
   └─ Catches manifest errors before writing any Rust

4. Write failing tests first (TDD)
   └─ tests/basic.rs: assert the geometry you expect

5. cargo test  (runs natively — fast feedback)
   └─ Tests fail (red)

6. Implement run_infill() in src/lib.rs
   └─ cargo test  (tests pass — green)

7. slicer build --release
   └─ Compiles to target/slicer/my-infill.wasm

8. slicer run --model test_model.stl
   └─ Verify G-code output visually in slicer frontend

9. slicer benchmark --model test_model.stl --layers 50
   └─ Confirm performance within acceptable range
```

---

## Python Bridge (TextPostProcess tier)

For post-processing scripts that are genuinely easier to implement in Python (e.g. legacy G-code text mutation), the host provides a Python bridge. Python modules live in the same directory as their `.toml` manifest.

```toml
# my-python-postprocessor.toml
[module]
id        = "com.example.my-postprocessor"
version   = "1.0.0"
wit-world = "slicer:world-postpass@1.0.0"

[stage]
id = "PostPass::TextPostProcess"

[python]
script = "postprocess.py"     # path relative to manifest
entry  = "process_gcode"      # function name in the script

[config.schema]
  [config.schema.amplitude]
  type = "float"
  default = 0.5
  min = 0.0
  max = 2.0
  display = "Wave Amplitude (mm)"
```

```python
# postprocess.py
def process_gcode(gcode_text: str, config: dict) -> str:
    amplitude = config.get("amplitude", 0.5)
    # ... text mutation ...
    return modified_text
```

The host invokes the Python interpreter (embedded via PyO3), calls `process_gcode(text, config_dict)`, and returns the result. The script runs in a restricted sandbox — no filesystem access, no network access, no subprocesses.

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

  [config.schema.point-distance]
  type    = "float"
  default = 0.8
  min     = 0.1
  max     = 3.0
  display = "Point Distance (mm)"
  group   = "Fuzzy Skin"

  [config.schema.apply-to-all]
  type    = "bool"
  default = false
  display = "Apply to Entire Model"
  description = "If false, apply only to painted regions"
  group   = "Fuzzy Skin"

[config.overridable-per-region]
keys = ["thickness", "point-distance", "apply-to-all"]

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
        let point_dist    = config.get_float("point-distance").unwrap_or(0.8) as f32;
        let apply_to_all  = config.get_bool("apply-to-all").unwrap_or(false);

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

## SDK Versioning

The SDK crate version tracks the WIT world version it targets:

- `slicer-sdk 1.x` → targets `slicer:world-layer@1.x`
- `slicer-sdk 2.x` → targets `slicer:world-layer@2.x` (breaking change)

The host specifies its supported WIT world version in the validation step. Modules built against an older SDK minor version always load on a newer host (additive compatibility). Modules built against a newer major version are rejected with a clear error.
