use std::collections::HashMap;

use arachne_perimeters::ArachnePerimeters;
use classic_perimeters::ClassicPerimeters;
use slicer_ir::{units_to_mm, PaintSemantic, PaintValue};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_support::fixtures::{square_polygon, ConfigViewBuilder};
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn run_region<M: LayerModule>(
    module: &M,
    region: SliceRegionView,
    config: &slicer_ir::ConfigView,
) -> Vec<(slicer_ir::Point3, f32)> {
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &[region], &paint, &mut output, config)
        .expect("run_perimeters");
    output.seam_candidates().to_vec()
}

#[test]
fn arachne_and_classic_exclude_same_painted_corner() {
    let config = ConfigViewBuilder::new()
        .int("wall_loops", 1)
        .float("outer_wall_line_width", 0.4)
        .float("inner_wall_line_width", 0.4)
        .build();
    let polygon = square_polygon(0.0, 0.0, 10.0);
    let blocked_corner = polygon.contour.points[0];
    let blocked_x_mm = units_to_mm(blocked_corner.x);
    let blocked_y_mm = units_to_mm(blocked_corner.y);
    let blocker_radius = 0.1_f32;
    let mut annotations = HashMap::new();
    annotations.insert(
        PaintSemantic::Custom("seam_blocker".to_string()),
        vec![vec![Some(PaintValue::Flag(true)), None, None, None]],
    );

    let mut region = SliceRegionView::default();
    region.set_object_id("obj-1");
    region.set_region_id(1);
    region.set_z(0.2);
    region.set_polygons(vec![polygon.clone()]);
    region.set_infill_areas(vec![polygon.clone()]);
    region.set_segment_annotations(annotations.clone());

    let arachne = ArachnePerimeters::from_config(&config).expect("from_config");
    let mut baseline_region = region.clone();
    baseline_region.set_segment_annotations(HashMap::new());
    let baseline_candidates = run_region(&arachne, baseline_region, &config);
    assert!(
        baseline_candidates.iter().any(|candidate| {
            let dx = candidate.0.x - blocked_x_mm;
            let dy = candidate.0.y - blocked_y_mm;
            dx * dx + dy * dy <= blocker_radius.powi(2)
        }),
        "the selected corner must qualify as an unpainted Arachne seam candidate"
    );
    let arachne_candidates = run_region(&arachne, region.clone(), &config);
    assert!(
        arachne_candidates.iter().all(|candidate| {
            let dx = candidate.0.x - blocked_x_mm;
            let dy = candidate.0.y - blocked_y_mm;
            dx * dx + dy * dy > blocker_radius.powi(2)
        }),
        "Arachne emitted a candidate inside the painted blocker corner"
    );

    let mut classic_region = SliceRegionView::default();
    classic_region.set_object_id("obj-1");
    classic_region.set_region_id(1);
    classic_region.set_z(0.2);
    classic_region.set_polygons(vec![polygon.clone()]);
    classic_region.set_infill_areas(vec![polygon]);
    classic_region.set_segment_annotations(annotations);
    let classic = ClassicPerimeters::from_config(&config).expect("from_config");
    let classic_candidates = run_region(&classic, classic_region, &config);
    assert!(
        classic_candidates.iter().all(|candidate| {
            let dx = candidate.0.x - blocked_x_mm;
            let dy = candidate.0.y - blocked_y_mm;
            dx * dx + dy * dy > blocker_radius.powi(2)
        }),
        "Classic emitted a candidate at the painted blocker corner"
    );
}
