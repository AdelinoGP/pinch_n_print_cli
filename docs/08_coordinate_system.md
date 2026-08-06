# Pinch 'n Print Coordinate System

**What this covers:** the unit system (1 unit = 100 nm), the Z-axis convention,
transform handling, and the constant-conversion rules for porting OrcaSlicer
code.

**Who it's for:** anyone writing geometry code or porting an algorithm from
OrcaSlicer — getting the scale wrong produces silently wrong toolpaths.

**Prerequisites:** none, but `02_ir_schemas.md` shows where these coordinates
live in the IR types.

> **This file is the single source of truth for coordinate conventions.**
> All other documentation defers to this file. When in doubt, read this first.

---

## The Rule

One scaled integer unit is 100 nanometers (`10⁻⁴ mm`). Multiply millimeters by
`10_000` to obtain units.

This applies to `Point2`, `Polygon`, `ExPolygon`, `BoundingBox2`, and other 2D
integer geometry. `Point3` and other floating-point position types remain in
millimeters.

Floating-point position and layer-height fields are in millimeters, speeds use
their documented rate unit, and densities/factors are unitless unless the field
name or doc comment says otherwise.

## Conversion & Determinism (Normative)

Canonical conversion rules:

- mm → units: `units = round(mm * 10_000.0)` (round half away from zero).
- units → mm: `mm = units / 10_000.0`.

`UNITS_PER_MM`, `mm_to_units`, and `units_to_mm` are defined in
`crates/slicer-ir/src/slice_ir.rs`. The SDK re-exports equivalent helpers from
`crates/slicer-sdk/src/coords.rs`.

The current implementations use `f32` at both conversion boundaries:
`mm_to_units` rounds the scaled value before casting to `i64`, while
`units_to_mm` casts the integer to `f32` before division. The integer grid gives
a nominal maximum quantization error of half a unit (`0.00005 mm`) when the
floating-point operations do not add a larger error.

Do not treat either conversion as a universal exact round trip. In particular,
`units_to_mm` narrows arbitrary `i64` values to `f32`. Retain integer units for
exact geometry identity and convert to millimeters only at a float boundary.
The existing `test_point2_coordinate_system` test in
`crates/slicer-ir/tests/ir_tests.rs` checks representative values, but it does
not establish a full-domain error bound or an accumulated per-layer budget.

## Z-Axis Convention (Normative)

- `z` and all layer-height values are stored and exchanged as millimeter floats (`f32`/`f64`) in IR and WIT; the WIT geometry and prepass records are defined in `crates/slicer-schema/wit/deps/types.wit` and `crates/slicer-schema/wit/deps/prepass-types.wit`.
- X/Y polygonal geometry uses scaled integers; Z does not.
- Any module converting Z to scaled integer units for internal math must convert back to mm before writing IR.
- Layer-plan output validates `z` as finite and non-negative and
  `effective_layer_height` as finite and positive at the host boundary. Z is
  not rounded through `mm_to_units`; catch-up layers use the
  `catchup_z_bottom`/`effective_layer_height` envelope defined by the layer
  IR.

## Transform Application (Normative)

For loaded models, 3MF build-item and component transforms are baked into mesh
vertices and paint-stroke vertices before `assemble_object` constructs the
`ObjectMesh`. STL and OBJ geometry has no input object transform. All three
loader paths use `assemble_object` in `crates/slicer-model-io/src/loader.rs`,
which stores an identity `ObjectMesh.transform`; loaded vertices therefore
already represent world-space coordinates. See the `ObjectMesh` Assembly
Contract in `docs/02_ir_schemas.md`.

The host-service transform machinery below still exists and **does** apply a
non-identity `ObjectMesh.transform` when one is present (exercised directly by
`crates/slicer-wasm-host/tests/unit/object_bounds_transform_tdd.rs` and
`crates/slicer-wasm-host/tests/unit/raycast_z_down_transformed_object_tdd.rs`).
For models produced by the loaders it is a no-op because the stored transform
is identity.

Conventions:

- **Layout:** `Transform3d.matrix` is a column-major `f64[16]`. For the affine
  3MF transforms parsed by the loader, translation occupies indices `12`, `13`,
  and `14`. The host transform helper also handles a non-unit homogeneous `w`.
- **World-space Z is canonical for planning and slicing.** Mesh analysis,
  layer slicing, `object_bounds`, and `raycast_z_down` apply the object
  transform. Modules that need world Z should use the host services rather
  than reimplementing transform math.
- **Z extents:** `object_world_z_extent` in
  `crates/slicer-model-io/src/loader.rs` returns `None` for an empty,
  non-finite, or degenerate extent (`z_max <= z_min`). The default layer
  planner skips objects with no positive queried height and fails with
  `no objects with positive height` if none remain; this is not a warning-only
  path.
- **Scale constraints:** non-uniform and negative scale are accepted by the
  transform paths. 3MF transforms are baked per-axis into mesh and paint data;
  host consumers apply the full transform for `ObjectMesh` values that retain
  one.
- **Floor validation:** when `validate_world_z_floor` in
  `crates/slicer-model-io/src/loader.rs` returns
  `ModelLoadError::WorldZBelowFloor` because the computed world-space `z_min`
  is below `0.0 mm`, the validator rejects the object. It does not perform an
  automatic floor adjustment.

## F-Token Formatting Convention (Normative)

G-code F tokens are emitted in **mm/min**, not mm/s, matching the configured
wire format. Internally, documented base speeds and feed-rate overrides are in
**mm/s**; `ExtrusionPath3D.speed_factor` is unitless. `TravelMove.f` and
retraction speed fields carry the same mm/s convention.

The conversion to mm/min happens inside `DefaultGCodeEmitter::resolve_feedrate`
(`crates/slicer-gcode/src/emit.rs`). Modules must work in mm/s; emitting mm/min
internally would double-scale at the boundary.

### Speed-factor clamp (Normative)

`ExtrusionPath3D.speed_factor: f32` is a per-move multiplier applied at
F-token emission. `resolve_feedrate` clamps it to **`[0.05, 5.0]`** before
multiplying by the role-resolved base speed.

---

## PaintStroke Vertex Coordinates (Normative)

`PaintLayer.strokes` is populated only for subdivided 3MF facets. The 3MF
document supplies `<triangle>` vertices in **millimetres**. `Point3` in
`crates/slicer-ir/src/slice_ir.rs` is the IR millimeter type, and
`PaintStroke.triangles` carries those millimeter values;
it does not carry scaled integer units. `decode_strokes_for_channel` in
`crates/slicer-model-io/src/loader.rs` copies the decoded `Point3` vertices into
the stroke, while `apply_transform_to_paint_data` applies any 3MF transform in
the same millimeter representation.

The previous documentation claim that the loader applied `mm_to_units()` to
strokes was stale. No such conversion occurs in the current loader, and
downstream paint consumers must therefore treat stroke vertices as world-space
millimeter `Point3` values. The WIT `paint-stroke-view` uses the same `point3`
contract in `crates/slicer-schema/wit/deps/prepass-types.wit`.

---

## Quick Reference

| Real-world value               | In Pinch 'n Print units |
| ------------------------------ | ---------------------- |
| 1 mm                           | 10_000                 |
| 0.4 mm (example width)         | 4_000                  |
| 0.2 mm (example layer height)  | 2_000                  |
| 0.1 mm (example feature size)  | 1_000                  |
| 0.01 mm (example increment)    | 100                    |
| 220 mm (example build plate X) | 2_200_000              |
| 1 nm (below one unit)          | 0.01 → rounds to 0     |

The smallest representable scaled-integer move is 100 nm.

---

## Why Not OrcaSlicer's Coordinate System?

OrcaSlicer uses 1 unit = 1 nanometer = `10⁻⁶ mm`, with a scaling factor of
`1_000_000`. This distinction is also recorded in `docs/02_ir_schemas.md`.

**We do not use this.** The reasons:

1. A 20 mm square in OrcaSlicer has corners at `(20_000_000, 20_000_000)`.
   In Pinch 'n Print those corners are at `(200_000, 200_000)` — 100× smaller, readable at a glance in test output and debuggers.

2. A 100 nm grid avoids carrying two extra decimal places of integer magnitude
   for this project's geometry contract.

3. 100 nm is a clean decimal step between OrcaSlicer's 1 nm and a micrometer
   (1,000 nm). The conversion factor between the two systems is exactly 100,
   which makes porting arithmetic straightforward.

4. Range is not a concern. `Point2` stores `i64`, but even a hypothetical `i32`
   (max 2,147,483,647) would cover a build plate of 214,748 mm — about 214
   meters. No one is going to build a printer that large, so the 100 nm scaling
   leaves enormous headroom.

---

## Conversion When Porting OrcaSlicer Code

When you port an algorithm from an OrcaSlicer checkout and it contains
scaled-integer coordinates or constants, divide linear OrcaSlicer units by
`100` for Pinch 'n Print units. Convert in the other direction by multiplying
by `100`.

### Common Constants

| OrcaSlicer constant     | OrcaSlicer value | Pinch 'n Print value        |
| ----------------------- | ---------------- | -------------------------- |
| `scale_(1.0)` (1mm)     | 1_000_000        | 10_000                     |
| `scale_(0.4)` (0.4mm)   | 400_000          | 4_000                      |
| `scale_(0.05)` (0.05mm) | 50_000           | 500                        |
| `scale_(0.01)` (0.01mm) | 10_000           | 100                        |

### `SCALED_EPSILON` Warning

Do not port `SCALED_EPSILON` by name. This workspace has no single exported
constant with that contract; current callers use algorithm-specific values,
including `SCALED_EPSILON_SQ` in `crates/slicer-core/src/medial_axis.rs` and
`MIN_SEGMENT_LENGTH` in
`crates/slicer-core/src/algos/paint_segmentation/colorize.rs`.
First identify whether an upstream value is a linear distance, a squared
distance, an area, or a unitless tolerance. A 1 nm linear threshold is below
the Pinch 'n Print resolution and cannot be represented exactly as an integer
unit; it must not be silently relabeled as equivalent to one 100 nm unit.

---

## Constant Conversion Table

The dimensional conversion rules are:

| Quantity | Conversion from OrcaSlicer units | Pinch 'n Print rule |
|----------|----------------------------------|--------------------|
| Linear length or distance | Divide by `100` | Round if an integer unit is required |
| Squared length or area | Divide by `10_000` | Preserve the squared-unit meaning |
| Angle or other unitless value | No scale conversion | Keep unchanged |
| Z coordinate | Not a scaled integer in this workspace | Keep in millimeters |

---

## SDK Helpers

Never write raw scaling arithmetic in module code. Use the SDK helpers:
`mm_to_units`, `units_to_mm`, and `SCALING_FACTOR` are available from
`slicer_sdk::coords`; their implementation is in
`crates/slicer-sdk/src/coords.rs`. The authoritative root constant is
`slicer_ir::UNITS_PER_MM` in `crates/slicer-ir/src/slice_ir.rs`, and the SDK
factor delegates to it.

---

## Point2 Wrapper

`Point2` (`crates/slicer-ir/src/slice_ir.rs`) holds two `i64` scaled-integer
fields (`x`, `y`), each `1 unit = 100 nm`. Its canonical constructors keep raw
scaling arithmetic out of call sites:

- `Point2::from_mm(x: f32, y: f32) -> Point2` — construct from millimeters.
- `Point2::to_mm(&self) -> (f32, f32)` — read back as millimeters.

Code review note: a PR that constructs `Point2 { x: 200_000, y: 200_000 }` from
a raw literal without a comment explaining the value should be rejected and
replaced with `Point2::from_mm(20.0, 20.0)`.

---

## Clipper2 Integration

`polygon_ops` in `crates/slicer-core/src/polygon_ops.rs` passes Pinch 'n Print
`Point2` coordinates to `clipper2-rust` as native 64-bit integer paths. No
additional scaling is needed at that boundary.

---

## Epsilon Multipliers — The Primary Porting Hazard

Do not multiply or copy an upstream epsilon by name without checking its
dimension. Convert linear distances by `100`, squared distances and areas by
`10_000`, and leave unitless tolerances unchanged. Keep the resulting
algorithm-specific constant next to the code that consumes it; do not invent a
workspace-wide epsilon contract in this document.

---

## Porting Checklist

When porting any file from an OrcaSlicer checkout:

- [ ] Identify every integer coordinate constant in the file
- [ ] Divide linear constants by 100; divide squared lengths and areas by 10,000
- [ ] Replace `scale_(x)` calls with `mm_to_units(x)`
- [ ] Replace `unscale(x)` calls with `units_to_mm(x)`
- [ ] Do NOT port `SCALED_EPSILON` by name; verify the consuming algorithm's dimensional meaning
- [ ] If the ported logic uses Z, verify Z remains in millimeters and is not accidentally scaled like X/Y
- [ ] Add the standard porting header from `docs/ORCASLICER_ATTRIBUTION.md` and identify the original source path
- [ ] Write a unit test that cross-checks a known OrcaSlicer output value against the ported function with coordinates divided by 100
- [ ] Test representative conversion values with a tolerance appropriate to their float boundary; do not assume universal identity
