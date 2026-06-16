# Task Map: 60_configurable-slicing-precision

This packet maps to a single new task ID, `TASK-201`, filed under the existing `DEV-009` umbrella deviation (`docs/07_implementation_status.md:184`). No existing TASK-### entry covered Douglas-Peucker simplification, Clipper2 arc tolerance, min-segment-length filtering, G-code XY decimals, or slice closing radius. Closest precedents (additive `declare_resolved_config!` packets) are TASK-153 (per-role feedrate, packet 52), TASK-154-cooling (packet 53), TASK-182 (overhang speed, packet 57), and TASK-181 (paint_config namespace).

At packet close, `docs/07_implementation_status.md` MUST be amended via worker dispatch (never load the full backlog) to add `TASK-201` under DEV-009 with a line such as:

```
- [x] TASK-201 — Configurable slicing precision (D-P at emit, arc tolerance, closing radius, gcode XY decimals; closed YYYY-MM-DD / packet 60).
```

## Step ↔ Task Mapping

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-201` | Step 1: Declare 7 config keys | `docs/02_ir_schemas.md` | `crates/slicer-ir/src/resolved_config.rs:328` | `libslic3r/libslic3r.h` (defaults) | `S` | Mirrors TASK-153 / TASK-182 additive macro pattern. Defaults sourced verbatim from OrcaSlicer constants. |
| `TASK-201` | Step 2: D-P + min-segment helpers | `docs/13_slicer_helpers_crate.md` | `crates/slicer-helpers/src/decimate.rs` | `libslic3r/MultiPoint.cpp:179` (delegated SUMMARY) | `M` | Iterative D-P, squared-distance, preserves endpoints. mm-space `f32`, not OrcaSlicer int64. |
| `TASK-201` | Step 3: slice_closing_radius at mesh slice | `docs/08_coordinate_system.md` | `crates/slicer-core/src/triangle_mesh_slicer.rs:341-360` | `libslic3r/PrintObjectSlice.cpp:192,1393` | `S` | `+r / -r` Clipper2 round-trip after `simplify_polygon_points`. Zero-cost when radius == 0. |
| `TASK-201` | Step 4: arc_tolerance on polygon_ops::offset | `docs/08_coordinate_system.md` | `crates/slicer-core/src/polygon_ops.rs:185-220` + caller pass-through | none (internal API) | `M` | Signature change; caller updates threaded through SDK / host / bench / modules. Modules upgrade from `0.0` to real value at Step 5. |
| `TASK-201` | Step 5: Per-module manifest + read-through | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `modules/core-modules/{classic,arachne}-perimeters/*.toml` + `src/lib.rs` | none | `S` | `[config.schema.perimeter_arc_tolerance]` follows existing wall_count shape. WASM guests rebuild required. |
| `TASK-201` | Step 6: Parameterize XYZ decimal output | `docs/08_coordinate_system.md` | `crates/slicer-host/src/gcode_emit.rs:1304` + 5 XYZ call sites | `libslic3r/GCode/GCodeWriter.hpp:234` | `S` | Adds sibling `format_xyz(value, decimals)`; leaves `format_coord` unchanged so F/E/temperature emit untouched. |
| `TASK-201` | Step 7: Per-role tolerance + min-segment at emit | `docs/01_system_architecture.md` | `crates/slicer-host/src/gcode_emit.rs` polyline-emit loops + `ExtrusionRole` dispatch | `libslic3r/MultiPoint.cpp:179` (reused SUMMARY) | `M` | `tolerance_for_role(role, cfg)` helper; exhaustive match (no wildcard) so new ExtrusionRole variants fail compile. |
| `TASK-201` | Step 8: WASM guest rebuild | `CLAUDE.md` Guest WASM Staleness | build scripts (`build-core-modules.sh`, `build-test-guests.sh`) | none | `S` | Mandatory after manifest/src edits + `slicer-ir`/`slicer-helpers`/`slicer-core` edits. |
| `TASK-201` | Step 9: Integration test — legacy vs default | none new | `crates/slicer-host/tests/slicing_precision_integration_tdd.rs` *(new)*, `tests/fixtures/golden/...` *(new)* | none | `M` | AC-10 (5%-fewer-lines) + NEG-2 (byte-identical legacy golden). Smallest viable fixture. |
| `TASK-201` | Step 10: Packet completion gate | `CLAUDE.md` Test Discipline | `packet.spec.md` (status), `docs/07_implementation_status.md` (append) | none | `S` | Re-dispatches every AC. `docs/07` append via worker dispatch only. |

Aggregate: `M`. No step is `L`. No cell is `L`.

## Cross-Packet Relationship

- This packet does NOT supersede or reopen any prior packet. It does NOT modify files in any other packet's `.ralph/specs/` directory.
- This packet UNBLOCKS:
  - A future XY/contour/hole/elephant-foot compensation packet — the precision-config plumbing now exists.
  - A future preset-enum packet (`slice_precision = draft|normal|high`) — numeric keys are in place and can be mapped.
  - A future F-decimal-adoption packet — `format_xyz` pattern can extend to F via a `format_feedrate(value, decimals)` sibling.
