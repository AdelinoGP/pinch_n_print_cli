# ModularSlicer Coordinate System

> **This file is the single source of truth for coordinate conventions.**
> All other documentation defers to this file. When in doubt, read this first.

---

## The Rule

```
1 scaled integer unit = 100 nanometers = 10⁻⁴ mm
Scaling factor: multiply millimeters by 10_000 to get units
```

This applies to **every** `Point2`, `Polygon`, `ExPolygon`, and any other 2D integer coordinate in the codebase. No exceptions.

`f32` / `f64` fields (speeds, densities, layer heights) are always in **millimeters** unless the field name or doc comment says otherwise.

## Conversion & Determinism (Normative)

Canonical conversion rules:

- mm → units: `units = round(mm * 10_000.0)` (round half away from zero).
- units → mm: `mm = units / 10_000.0`.

Determinism bounds:

- One conversion round-trip (`units -> mm -> units`) must be identity.
- One float round-trip (`mm -> units -> mm`) has bounded error `<= 0.00005 mm`.
- Any pipeline step that accumulates more than `0.001 mm` absolute error in one axis across one layer is a contract violation.

## Z-Axis Convention (Normative)

- `z` and all layer-height values are stored and exchanged as millimeter floats (`f32`/`f64`) in IR and WIT.
- X/Y polygonal geometry uses scaled integers; Z does not.
- Any module converting Z to scaled integer units for internal math must convert back to mm before writing IR.
- `catchup_z_bottom` and `effective_layer_height` must remain finite, non-negative, and deterministic under the rounding policy in § "Conversion & Determinism (Normative)" above.

## Transform Application — Query-Time, Not Load-Time (Normative — packet 10)

Object mesh transforms (`ObjectMesh.transform`) are **not** baked into mesh
vertices at load time. Raw vertices stay in object-local space; transforms
apply at host-service query time (raycasts, normals, bounding queries).

Conventions:

- **Layout:** column-major `f64[16]`. Translation occupies indices `12`, `13`,
  `14` (column 3). The fourth row is `[0, 0, 0, 1]` (no projective component).
- **World-space Z is canonical for layer planning.** Object-local Z is never
  used by `PrePass::LayerPlanning` or the per-layer Z dispatch. Modules that
  need world Z must query via `host_services::object_bounds(object_id)` or
  `raycast_z_down`; the host applies the transform during the query.
- **Z extents:** if a transformed object has `z_max <= z_min`
  (degenerate / inside-out / non-finite), `object_world_z_extent(object_id)`
  returns `None` and the object contributes zero layers. This is not an
  error — it surfaces as a slicing warning in the per-object diagnostics.
- **Scale constraints:** non-uniform scale is rejected with fatal error
  `NON_UNIFORM_SCALE_UNSUPPORTED { object_id, scale_x, scale_y, scale_z }`.
  Mirroring (negative scale) is allowed if all three signs match (uniform
  inversion).
- **Floor enforcement:** if the transformed object's `z_min < 0.0` after the
  build-plate floor adjustment, the host emits fatal `WORLD_Z_BELOW_FLOOR
  { object_id, z_min }` — slicing below the build plate is never permitted.

## F-Token Formatting Convention (Normative — packet 52)

G-code F tokens are emitted in **mm/min**, not mm/s, matching OrcaSlicer's
wire format. Internally, every speed field in IR (`ExtrusionPath3D.speed`,
`ConfigView`'s `*_speed` keys, `TravelMove.speed`) is stored in **mm/s**.

The conversion to mm/min happens inside `DefaultGCodeEmitter::resolve_feedrate`
(see `crates/slicer-runtime/src/gcode_emit.rs`) — that function returns a
mm/min value ready for `F{:.0}` serialization. Modules must always work in
mm/s; emitting mm/min internally is a contract violation that double-scales
at the boundary.

### Speed-factor clamp (Normative — Packet 52)

`ExtrusionPath3D.speed_factor: f32` is a per-move multiplier applied at
F-token emission. `resolve_feedrate` clamps it to **`[0.05, 5.0]`** before
multiplying by the role-resolved base speed. The clamp rejects pathological
values (0.0 would emit `F0`; negative or NaN values would silently produce
wrong feedrates). OrcaSlicer parity confirmed against
`GCodeWriter::set_speed`.

---

## PaintStroke Vertex Conversion (Normative — packet 50a)

`PaintLayer.strokes` is populated only for subdivided 3MF facets. The 3MF
document supplies `<triangle>` vertices in **millimetres**, but
`PaintStroke.triangles: Vec<[Point3; 3]>` carries vertices in
**slicer units (1 unit = 100 nm)**. The 3MF loader applies `mm_to_units()`
to every component before committing the stroke. Forgetting this conversion
produces coordinates 10,000× too large — a silent contract violation that
would propagate through every downstream stage that consumes
`PaintLayer.strokes` (paint segmentation, region mapping).

The `mm_to_units()` helper lives in `slicer-helpers`. Tests covering the
3MF subdivision parser pin the conversion explicitly; any future format
that surfaces strokes in millimetres must apply the same conversion at
the loader boundary, never at consumption time.

---

## Quick Reference

| Real-world value               | In ModularSlicer units |
| ------------------------------ | ---------------------- |
| 1 mm                           | 10_000                 |
| 0.4 mm (nozzle diameter)       | 4_000                  |
| 0.2 mm (layer height)          | 2_000                  |
| 0.1 mm (min feature size)      | 1_000                  |
| 0.01 mm (hardware step)        | 100                    |
| 220 mm (typical build plate X) | 2_200_000              |
| 1 nm (resolution floor)        | 0.01 → rounds to 0     |

The smallest representable move is 100 nm. No FDM printer can position a nozzle more precisely than ~10,000 nm (10 µm), so this gives 100× more precision than any hardware can use — intentionally.

---

## Why Not OrcaSlicer's Coordinate System?

OrcaSlicer (and PrusaSlicer) use:

```
1 unit = 1 nanometer = 10⁻⁶ mm
Scaling factor: 1_000_000
```

**We do not use this.** The reasons:

1. A 20 mm square in OrcaSlicer has corners at `(20_000_000, 20_000_000)`.
   In ModularSlicer those corners are at `(200_000, 200_000)` — 100× smaller, readable at a glance in test output and debuggers.

2. Nanometer precision serves no physical purpose in FDM. The bead width is ~400,000 nm. The hardware step ceiling is ~10,000 nm.

3. 100 nm is a clean decimal step between OrcaSlicer's 1 nm and micrometer (1,000 nm). The conversion factor between the two systems is exactly 100, which makes porting arithmetic trivial.

4. `i32` is still safe: max value 2,147,483,647 covers a build plate of 214,748 mm — about 214 meters. No one is gonna build a printer that large.

---

## Conversion When Porting OrcaSlicer Code

When you port an algorithm from `OrcaSlicer_Documented/` and it contains scaled-integer coordinates or constants, apply this conversion:

```
ModularSlicer_units = OrcaSlicer_units / 100
OrcaSlicer_units = ModularSlicer_units * 100
```

### Common Constants

| OrcaSlicer constant     | OrcaSlicer value | ModularSlicer value        |
| ----------------------- | ---------------- | -------------------------- |
| `SCALED_EPSILON`        | 1                | — (do not port; see below) |
| `scale_(1.0)` (1mm)     | 1_000_000        | 10_000                     |
| `scale_(0.4)` (0.4mm)   | 400_000          | 4_000                      |
| `scale_(0.05)` (0.05mm) | 50_000           | 500                        |
| `scale_(0.01)` (0.01mm) | 10_000           | 100                        |
| `CLIPPER_OFFSET_SCALE`  | 100_000          | 1_000                      |

### `SCALED_EPSILON` Warning

OrcaSlicer's `SCALED_EPSILON = 1` (1 nm) is used throughout its codebase as a near-zero tolerance for polygon operations. **Do not port this value.**

In ModularSlicer, our unit is 100 nm, so a direct port would give `SCALED_EPSILON = 1` meaning 100 nm, which is 100× larger than intended.

Use a ModularSlicer constant instead. The convention is:

```rust
// SCALED_EPSILON: i64 = 1;  // 1 unit = 100 nm
// Equivalent to OrcaSlicer's SCALED_EPSILON of 1 (1 nm) divided by 100,
// then rounded up to nearest integer = 1. Effectively the same tolerance
// at our precision floor.
```

<!-- VERIFY: at the time of writing, there is no canonical `SCALED_EPSILON`
     constant exported from `crates/slicer-core/src/`. Choose or introduce
     the appropriate named constant in the consuming crate (see "Named
     epsilon constants" below) rather than re-using a bare `SCALED_EPSILON`. -->

If you find yourself tempted to use `100` as an epsilon "to match OrcaSlicer",
you are off by a factor of 100. The correct epsilon is `1`.

---

## SDK Helpers

Never write raw scaling arithmetic in module code. Use the SDK helpers:

```rust
use slicer_sdk::coords::{mm_to_units, units_to_mm, SCALING_FACTOR};

// Convert mm → units
let width_units: i64 = mm_to_units(0.4);    // = 4_000
let height_units: i64 = mm_to_units(1.2);   // = 12_000

// Convert units → mm
let width_mm: f32 = units_to_mm(4_000);     // = 0.4

// The scaling factor itself, if you need it
assert_eq!(SCALING_FACTOR, 10_000_i64);
```

The implementations are trivial but centralizing them means a future
precision change (if ever warranted) is a one-line fix in one file:

```rust
// crates/slicer-sdk/src/coords.rs
pub const SCALING_FACTOR: i64 = 10_000;

#[inline(always)]
pub fn mm_to_units(mm: f32) -> i64 {
    (mm * SCALING_FACTOR as f32).round() as i64
}

#[inline(always)]
pub fn units_to_mm(units: i64) -> f32 {
    units as f32 / SCALING_FACTOR as f32
}
```

---

## Newtype Wrapper

`Point2` uses a newtype to make accidental raw integer arithmetic a compile error rather than a silent wrong answer:

```rust
/// A 2D point in scaled integer coordinates.
/// 1 unit = 100 nm = 10⁻⁴ mm.
/// Use `mm_to_units()` and `units_to_mm()` for conversion.
/// Never construct with raw integer literals except in tests
/// that explicitly call `Point2::from_raw(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point2 {
    pub x: i64,
    pub y: i64,
}

impl Point2 {
    /// Construct from millimeter coordinates.
    pub fn from_mm(x: f32, y: f32) -> Self {
        Self { x: mm_to_units(x), y: mm_to_units(y) }
    }

    /// Construct from raw scaled-integer units.
    /// Use only in tests or when consuming external integer coordinates.
    pub fn from_raw(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    pub fn to_mm(&self) -> (f32, f32) {
        (units_to_mm(self.x), units_to_mm(self.y))
    }
}
```

Code review note: a PR that constructs `Point2 { x: 200_000, y: 200_000 }` without a comment explaining the raw value should be rejected and replaced with `Point2::from_mm(20.0, 20.0)`.

---

## Clipper2 Integration

Clipper2 accepts 64-bit integers natively. No intermediate scaling is needed when passing ModularSlicer coordinates to Clipper2.

The Clipper2 documentation recommends keeping values below 4.6 × 10¹⁸ (max i64). At our scaling factor of 10_000, a 1-meter build plate is 10_000_000 units — well within safe range. No overflow guards are needed for realistic print geometries.

---

## Epsilon Multipliers — The Primary Porting Hazard

`SCALED_EPSILON` itself ports correctly (OrcaSlicer value 1 → ModularSlicer value 1, meaning 1nm → 100nm, still well below hardware resolution). The danger is every place OrcaSlicer writes `SCALED_EPSILON * N`:

| OrcaSlicer expression   | OrcaSlicer meaning | Naive ModularSlicer port           | Correct ModularSlicer value     |
| ----------------------- | ------------------ | ---------------------------------- | ------------------------------- |
| `SCALED_EPSILON * 1`    | 1nm                | 100nm ✓                            | `POINT_COINCIDENCE_EPSILON = 1` |
| `SCALED_EPSILON * 10`   | 10nm               | 1µm ✓                              | `MIN_SEGMENT_LENGTH = 10`       |
| `SCALED_EPSILON * 100`  | 100nm              | 10µm ⚠️ hardware boundary          | use named constant              |
| `SCALED_EPSILON * 1000` | 1µm                | 100µm ✗ quarter nozzle width       | use named constant              |
| `SCALED_EPSILON²`       | 1nm²               | 10,000nm² ✓ coincidentally correct | `MIN_POLYGON_AREA`              |

**Rule: Never port `SCALED_EPSILON * N` directly.**

Every multiplied epsilon usage must be replaced with a named constant in the consuming crate (typically `slicer-core` or `slicer-helpers`) that documents the physical meaning. If the right named constant does not exist, add it with a full comment before using it. A PR containing `SCALED_EPSILON * N` for any N > 1 should be rejected in code review.

<!-- VERIFY: there is no single `crates/slicer-core/src/geometry.rs` file
     today; geometry utilities live across `aabb_lines_2d.rs`, `aabb_tree.rs`,
     `paint_region.rs`, `polygon_ops.rs`, and `triangle_mesh_slicer.rs`.
     Place new named epsilons next to the code that consumes them and
     re-export from `slicer_core::lib` if widely shared. -->

Suggested named epsilons and their physical meanings (define when first needed):

```rust
pub const POINT_COINCIDENCE_EPSILON: i64 = 1;       // 100 nm  — coincident point merge threshold
pub const MIN_SEGMENT_LENGTH:        i64 = 10;      // 1 µm    — degenerate edge collapse threshold
pub const MIN_POLYGON_AREA:          i64 = 250_000; // 50 µm²  — degenerate polygon discard
pub const MIN_PRINTABLE_WIDTH:       i64 = 100;     // 10 µm   — Arachne minimum bead width
```

---

## Porting Checklist

When porting any file from `OrcaSlicer_Documented/`:

- [ ] Identify every integer coordinate constant in the file
- [ ] Divide each by 100 to get the ModularSlicer equivalent
- [ ] Replace `scale_(x)` calls with `mm_to_units(x)`
- [ ] Replace `unscale(x)` calls with `units_to_mm(x)`
- [ ] Do NOT port `SCALED_EPSILON` directly — use ModularSlicer's constant
- [ ] Do NOT port `SCALED_EPSILON * N` for any N > 1 — define or re-use a named constant in the consuming `slicer-core` / `slicer-helpers` module instead
- [ ] If the ported logic uses Z, verify Z remains in millimeters and is not accidentally scaled like X/Y
- [ ] Add a porting comment at the top of the new file:
      `// Ported from OrcaSlicer_Documented/src/libslic3r/XYZ.cpp`
      `// Coordinate constants divided by 100 (OrcaSlicer: 1nm, ModularSlicer: 100nm)`
- [ ] Write a unit test that cross-checks a known OrcaSlicer output value against the ported function with coordinates divided by 100
- [ ] Add a round-trip assertion for representative values:
      `units_to_mm(mm_to_units(v)) ~= v` for each critical constant `v`
