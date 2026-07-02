//! AC-3 / AC-N1 (packet 108, T-074b/T-074c/T-074d): non-planar shell emission.
//!
//! Our own extension — absent in OrcaSlicer's classic perimeter generator —
//! for regions whose `nonplanar_surface` resolved to a `SurfaceGroup`
//! (`crates/slicer-sdk/src/views.rs::SliceRegionView::surface_group`). Highest
//! precedence in `run_perimeters`: overrides `wall_count`, skips thin-wall,
//! gap-fill, extra_perimeters, and the narrow-island override entirely, and
//! leaves `infill_areas` empty.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{LoopType, SurfaceGroup};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};

fn make_surface_group(shell_count: u32) -> SurfaceGroup {
    SurfaceGroup {
        id: 7,
        facet_indices: vec![],
        z_min: 0.0,
        z_max: 1.0,
        area_mm2: 100.0,
        printable: true,
        shell_count,
    }
}

/// AC-3: a region backed by `SurfaceGroup { shell_count: 3, .. }` emits
/// exactly 3 walls, all `LoopType::NonPlanarShell`, and no infill.
#[test]
fn nonplanar_region_emits_shell_count_walls() {
    let config = ConfigViewBuilder::new().int("wall_count", 2).build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .surface_group(make_surface_group(3))
        .build();

    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        3,
        "expected exactly 3 wall loops (shell_count=3); got {}",
        walls.len()
    );
    assert!(
        walls
            .iter()
            .all(|w| w.loop_type == LoopType::NonPlanarShell),
        "expected every wall loop to be LoopType::NonPlanarShell; got {:?}",
        walls.iter().map(|w| w.loop_type).collect::<Vec<_>>()
    );
    assert!(
        !walls
            .iter()
            .any(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner),
        "non-planar region must not emit Outer/Inner walls"
    );
    assert!(
        output.infill_areas().iter().all(|areas| areas.is_empty()),
        "non-planar region must leave infill_areas empty"
    );
}

/// AC-N1: a non-planar region with `detect_thin_wall=true` emits zero
/// ThinWall loops — the non-planar branch skips thin-wall detection entirely.
#[test]
fn nonplanar_skips_thin_wall_case() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .bool("detect_thin_wall", true)
        .build();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .surface_group(make_surface_group(3))
        .build();

    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let thin_wall_count = output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::ThinWall)
        .count();
    assert_eq!(
        thin_wall_count, 0,
        "expected zero ThinWall loops for a non-planar region regardless of detect_thin_wall"
    );
    let gap_fill_count = output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::GapFill)
        .count();
    assert_eq!(
        gap_fill_count, 0,
        "expected zero GapFill loops for a non-planar region regardless of gap_infill_speed config"
    );
}
