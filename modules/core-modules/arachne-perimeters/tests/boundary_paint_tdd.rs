//! TDD tests for D-154: arachne-perimeters segment_annotations propagation.
//!
//! Prior to this fix, arachne-perimeters never read `region.segment_annotations()`
//! at all — every wall got default (unpainted) `WallFeatureFlags` and a
//! `boundary_type` computed purely from `line.inset_idx == 0`. These tests verify
//! arachne now propagates Material→tool_index and FuzzySkin→fuzzy_skin into
//! `WallFeatureFlags` via the shared `build_wall_flags` helper, for BOTH outer
//! and inner walls.
//!
//! Unlike classic-perimeters (whose outer wall is a direct polygon offset with
//! 1:1 vertex correspondence to the original contour), arachne's walls —
//! including the outer one — come from Voronoi beading and have no such
//! correspondence. So arachne always uses `build_wall_flags`'s geometric
//! reprojection path, never the index-based fallback. The
//! `outer_wall_stays_exterior_surface_when_painted_without_transitions` test
//! below specifically guards the regression this fix had to correct: the
//! reprojection branch's `boundary_type` fallback was hardcoded to `Interior`
//! (correct only when it was reachable exclusively via `!is_outer`, before this
//! fix); once reprojection became reachable for `is_outer == true` too, that
//! fallback had to become `is_outer`-aware, matching the index-based branch.

use std::collections::HashMap;

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, PaintSemantic, PaintValue, WallBoundaryType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// `optimal_width`/`preferred_bead_width_outer` are `unit = "units"` keys
/// (1 unit = 100 nm), unlike classic-perimeters' bare-mm `line_width` key —
/// see `arachne_parity_outer_wall_boundary_type_tdd.rs`'s identical helper.
fn make_config(wall_count: u32, line_width_mm: f32) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count as i64)
        .float("inner_wall_line_width", line_width_mm as f64)
        .float("outer_wall_line_width", line_width_mm as f64)
        .build()
}

fn make_region(poly: slicer_ir::ExPolygon) -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-1".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![poly]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region
}

#[test]
fn unpainted_region_produces_default_flags() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region(square_polygon(0.0, 0.0, 10.0));

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "should produce at least one wall loop");
    for wall in walls {
        for flags in &wall.feature_flags {
            assert_eq!(
                flags.tool_index, None,
                "unpainted arachne wall should have no tool_index"
            );
            assert!(
                !flags.fuzzy_skin,
                "unpainted arachne wall should have fuzzy_skin=false"
            );
        }
    }
}

#[test]
fn material_paint_sets_tool_index_on_outer_and_inner_walls() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon(0.0, 0.0, 10.0);
    let num_points = poly.contour.points.len();
    let material_paint = vec![vec![Some(PaintValue::ToolIndex(2)); num_points]];
    let mut segment_annotations = HashMap::new();
    segment_annotations.insert(PaintSemantic::Material, material_paint);

    let mut region = make_region(poly);
    region.set_segment_annotations(segment_annotations);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let outer_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    let inner_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index > 0).collect();
    assert!(!outer_walls.is_empty(), "should have an outer wall");
    assert!(
        !inner_walls.is_empty(),
        "wall_count=2 should also produce at least one inner wall"
    );

    for wall in outer_walls.iter().chain(inner_walls.iter()) {
        for flags in &wall.feature_flags {
            assert_eq!(
                flags.tool_index,
                Some(2),
                "Material paint should set tool_index via reprojection on every \
                 arachne wall (perimeter_index={}), not just walls whose vertices \
                 happen to align with the original contour",
                wall.perimeter_index
            );
        }
    }
}

#[test]
fn fuzzy_skin_paint_sets_flag_on_arachne_walls() {
    let config = make_config(1, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon(0.0, 0.0, 10.0);
    let num_points = poly.contour.points.len();
    let fuzzy_paint = vec![vec![Some(PaintValue::Flag(true)); num_points]];
    let mut segment_annotations = HashMap::new();
    segment_annotations.insert(PaintSemantic::FuzzySkin, fuzzy_paint);

    let mut region = make_region(poly);
    region.set_segment_annotations(segment_annotations);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "should produce wall loops");
    for wall in walls {
        for flags in &wall.feature_flags {
            assert!(
                flags.fuzzy_skin,
                "FuzzySkin paint should set fuzzy_skin on arachne walls"
            );
        }
    }
}

/// Regression guard (D-154): before this fix, `build_wall_flags`'s
/// geometric-reprojection branch hardcoded `Interior` as its no-transition
/// fallback, which was safe only because reprojection was unreachable for
/// `is_outer == true`. Extending reprojection to arachne's outer wall without
/// also making that fallback `is_outer`-aware would have silently regressed
/// every painted arachne outer wall from `ExteriorSurface` to `Interior`.
#[test]
fn outer_wall_stays_exterior_surface_when_painted_without_transitions() {
    let config = make_config(2, 0.4_f32);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let poly = square_polygon(0.0, 0.0, 10.0);
    let num_points = poly.contour.points.len();
    // Uniform paint: present, but no adjacent-tool transitions anywhere.
    let material_paint = vec![vec![Some(PaintValue::ToolIndex(2)); num_points]];
    let mut segment_annotations = HashMap::new();
    segment_annotations.insert(PaintSemantic::Material, material_paint);

    let mut region = make_region(poly);
    region.set_segment_annotations(segment_annotations);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let outer_wall = output
        .wall_loops()
        .iter()
        .find(|w| w.perimeter_index == 0)
        .expect("a wall loop with perimeter_index == 0 must be emitted");

    assert_eq!(
        outer_wall.boundary_type,
        WallBoundaryType::ExteriorSurface,
        "uniformly-painted outer wall (no material transitions) must still be \
         ExteriorSurface, got {:?}",
        outer_wall.boundary_type
    );
}
