# Waiver Inventory — Packet 196 Literal Sweep (core, ir, gcode)

This is the packet-196 waiver inventory, generated at close (2026-08-08) from the
live tree for packet 199's audit. The authoritative source is
`rg -n '// exhaustive:'` over `crates/slicer-ir`, `crates/slicer-core`,
`crates/slicer-gcode`. Do not hand-edit; regenerate from the live tree.

## slicer-ir

- crates/slicer-ir/tests/extrusion_line_roundtrip.rs:19 — carrier/roundtrip test asserts every field travels
- crates/slicer-ir/tests/ir_tests.rs:12 — file-local base; no Default impl for ModifierVolume (packet 196)
- crates/slicer-ir/tests/ir_tests.rs:302 — carrier/roundtrip test asserts every field travels
- crates/slicer-ir/tests/ir_validation_tdd.rs:50 — file-local base; sdk fixture home would pull host-algos into this crate's dev graph (packet 196 [FWD])
- crates/slicer-ir/tests/entity_id_invariants_tdd.rs:55 — file-local base; sdk fixture home would pull host-algos into this crate's dev graph (packet 196 [FWD])
- crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs:15 — carrier/roundtrip test asserts every field travels

## slicer-core

- crates/slicer-core/src/perimeter_utils.rs:852 — file-local base; no Default impl for WallLoop (packet 196)
- crates/slicer-core/src/voronoi.rs:413 — no Default impl for Segment; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/colorize.rs:397 — no Default impl for PaintedLine; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs:425 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs:456 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs:473 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs:559 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/mod.rs:2045 — no Default impl for ModifierVolume; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs:248 — no Default impl for ModifierVolume; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:282 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:297 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:312 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:327 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:397 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:453 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:467 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:512 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs:546 — no Default impl for MmuArc; every field is a fixture input (packet 196)
- crates/slicer-core/tests/flow_tdd.rs:149 — every role-width precedence input is intentionally explicit
- crates/slicer-core/tests/voronoi.rs:16 — no Default impl for Segment; every field is a fixture input (packet 196)
- crates/slicer-core/tests/voronoi_panic_regression.rs:27 — file-local base; no Default impl for Segment (packet 196)
- crates/slicer-core/tests/wall_sequence_reorder_tdd.rs:14 — file-local base; sdk fixture home would pull host-algos into this crate's dev graph (packet 196 [FWD])
- crates/slicer-core/tests/voronoi_stress.rs:23 — no Default impl for Segment; every field is a fixture input (packet 196)

## slicer-gcode

- (none — no `// exhaustive:` waivers present)

## Totals

- slicer-ir: 6
- slicer-core: 23
- slicer-gcode: 0
- grand total: 29
