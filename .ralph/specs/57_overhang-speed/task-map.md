# Task Map: 57_overhang-speed

Bridges the packet steps back to `docs/07_implementation_status.md` TASK-182 and to the DEV-009 remediation in `docs/DEVIATION_LOG.md`. Included because the packet spans the WIT boundary and the host pipeline simultaneously, even though the formal task-ID count is one.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-182` | Step 0 — WIT record extension + binding fan-out (GATE) | `docs/03_wit_and_manifest.md`; `CLAUDE.md` *WIT/Type Changes Checklist* | `wit/deps/types.wit`; `crates/slicer-ir/src/slice_ir.rs:1218`; `crates/slicer-ir/src/lib.rs` (schema-version constant); enumerated WIT↔Rust conversion sites (≤ 20, found via LOCATIONS dispatch) | none | `M` | Gate: `cargo build --tests --workspace` must be green before Step 1. The conversion-site enumeration is the highest-risk dispatch in the packet. |
| `TASK-182` | Step 1 — Author RED TDD scaffold | `docs/08_coordinate_system.md` | `crates/slicer-host/tests/overhang_speed_tdd.rs` (new); `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs` (new or extension) | none | `S` | Test scaffold covers AC-1…AC-5, AC-6, AC-N1. All must be RED. |
| `TASK-182` | Step 2 — `LinesDistancer2D` in slicer-core | `docs/13_slicer_helpers_crate.md` | `crates/slicer-core/src/aabb_lines_2d.rs` (new); `crates/slicer-core/src/lib.rs` | none (utility) | `S` | Linear scan + bbox prefilter; BVH deferred. In-module unit tests cover signed-distance sign for CW and CCW loops. |
| `TASK-182` | Step 3 — `overhang_classifier` in slicer-host | `docs/02_ir_schemas.md`; `docs/08_coordinate_system.md` | `crates/slicer-host/src/overhang_classifier.rs` (new); `crates/slicer-host/src/lib.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp:71,147,397,514,535`; `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6599-6618,6639` | `M` | Threshold endpoint `< / <=` convention SNIPPETS dispatch is mandatory. `debug_assert!(q >= 1 && q <= 4)` enforces AC-N1. |
| `TASK-182` | Step 4 — `resolve_feedrate` extension + pipeline wire-in | `docs/02_ir_schemas.md` | `crates/slicer-host/src/gcode_emit.rs:154,388,443`; `crates/slicer-host/src/pipeline.rs` (both arms) | none new | `M` | Per-point loop at `gcode_emit.rs:362-393` is structurally unchanged; only the new arg threads at `:388`. All seven AC tests go GREEN at the end of this step. |
| `TASK-182` | Step 5 — Regression sweep + clippy gate | none | none (pure dispatch) | none | `S` | `gcode_feedrate_emission_tdd`, `gcode_emit_tdd`, `orca_comment_contract_tdd`, clippy, `cargo check --workspace`. |
| `TASK-182` | Step 6 — Documentation updates | `docs/DEVIATION_LOG.md` (DEV-009 row) | `docs/DEVIATION_LOG.md` | none | `S` | Append remediation clause for packet 57 to the DEV-009 row. |
| `TASK-182` | Step 7 — Packet completion gate / acceptance ceremony | `CLAUDE.md` *Test Discipline* section | `.ralph/specs/57_overhang-speed/packet.spec.md`; `docs/07_implementation_status.md` (TASK-182 row) | none | `M` | Re-dispatch every AC. Run `cargo test --workspace` as the close-time ceremony only. Flip `status: implemented` and close TASK-182. |

## Aggregate

- Sum of per-step costs: `4×M + 4×S` ≈ aggregate `M`. No single step rated `L`. Packet may proceed to activation.

## DEV-009 Linkage

DEV-009 (`docs/DEVIATION_LOG.md`) tracks the Phase H gap for `.gcode` correctness on the live path. Packet 52 closed the per-role feedrate slice; packet 57 closes the quartile-modulated wall slowdown slice. Future packets remain for: curled-edge slowdown, smoothed-speed mode, `ConfigValue::FloatOrPercent`, bridge-perimeter as a distinct role, and the `ADD_INTERSECTIONS` mid-segment splitting refinement.

## Cross-Reference

- Predecessor packet: `52_gcode-feedrate-emission` (`status: implemented`). Packet 57 consumes packet 52's `FeedrateConfig.overhang_{1,2,3,4}_4_speed` fields without modification.
- No packet is superseded by this work.
