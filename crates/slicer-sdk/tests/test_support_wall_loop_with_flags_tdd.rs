//! TDD tests for `PerimeterRegionViewBuilder::add_outer_wall_with_flags`.
//!
//! Exercises the seam-placer's `wall_at_z` shape: a non-empty
//! `feature_flags` vector and `WallBoundaryType::ExteriorSurface`.

use slicer_ir::{LoopType, WallBoundaryType, WallFeatureFlags};
use slicer_sdk::test_prelude::*;

#[test]
fn add_outer_wall_with_flags_threads_feature_flags() {
    let path = rect_path(0.0, 0.0, 10.0, 0.4);
    let n = path.points.len();
    let flag = WallFeatureFlags {
        is_bridge: true,
        ..Default::default()
    };
    let flags = vec![flag; n];

    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, flags.clone(), WallBoundaryType::ExteriorSurface)
        .build();

    assert_eq!(view.wall_loops().len(), 1);
    let wl = &view.wall_loops()[0];
    assert_eq!(wl.feature_flags.len(), n);
    for (got, want) in wl.feature_flags.iter().zip(flags.iter()) {
        assert_eq!(got, want);
    }
}

#[test]
fn add_outer_wall_with_flags_threads_exterior_surface_boundary() {
    let path = rect_path(0.0, 0.0, 10.0, 0.4);
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, Vec::new(), WallBoundaryType::ExteriorSurface)
        .build();
    assert_eq!(
        view.wall_loops()[0].boundary_type,
        WallBoundaryType::ExteriorSurface
    );
}

#[test]
fn add_outer_wall_with_flags_preserves_outer_loop_type_and_index() {
    let path = rect_path(0.0, 0.0, 10.0, 0.4);
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, Vec::new(), WallBoundaryType::ExteriorSurface)
        .build();
    let wl = &view.wall_loops()[0];
    assert_eq!(wl.loop_type, LoopType::Outer);
    assert_eq!(wl.perimeter_index, 0);
}

#[test]
fn add_outer_wall_with_flags_keeps_uniform_width_profile_from_path() {
    let width = 0.55;
    let path = rect_path(0.0, 0.0, 10.0, width);
    let n = path.points.len();
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, Vec::new(), WallBoundaryType::ExteriorSurface)
        .build();
    let wl = &view.wall_loops()[0];
    assert_eq!(wl.width_profile.widths.len(), n);
    for w in &wl.width_profile.widths {
        assert!((*w - width).abs() < f32::EPSILON);
    }
}

#[test]
fn add_outer_wall_with_flags_supports_material_boundary() {
    let path = rect_path(0.0, 0.0, 10.0, 0.4);
    let view = PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(
            path,
            Vec::new(),
            WallBoundaryType::MaterialBoundary { adjacent_tool: 2 },
        )
        .build();
    assert_eq!(
        view.wall_loops()[0].boundary_type,
        WallBoundaryType::MaterialBoundary { adjacent_tool: 2 }
    );
}
