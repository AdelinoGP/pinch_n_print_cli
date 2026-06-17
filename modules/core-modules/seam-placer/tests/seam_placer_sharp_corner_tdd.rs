//! Edge-case coverage for seam-placer: a Sharp-reason candidate must be
//! preferred over an equal-score Aligned candidate (the `concave_preferred`
//! test covers Concave; this covers Sharp).

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{
    ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SeamCandidate, SeamReason,
    WallBoundaryType, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::{seam_candidate, PerimeterRegionViewBuilder};
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;

use seam_placer::SeamPlacer;

fn candidate(x: f32, score: f32, reason: SeamReason) -> SeamCandidate {
    seam_candidate(
        Point3WithWidth {
            x,
            y: 0.0,
            z: 1.0,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        score,
        reason,
    )
}

fn wall_from_candidates(candidates: &[SeamCandidate]) -> WallLoop {
    let points: Vec<_> = candidates
        .iter()
        .map(|c| Point3WithWidth {
            x: c.position.x,
            y: c.position.y,
            z: 1.0,
            width: c.position.width,
            flow_factor: c.position.flow_factor,
            overhang_quartile: c.position.overhang_quartile,
        })
        .collect();
    let flags = vec![
        WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new(),
        };
        points.len()
    ];
    let path = ExtrusionPath3D {
        points,
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    };
    PerimeterRegionViewBuilder::new()
        .add_outer_wall_with_flags(path, flags, WallBoundaryType::ExteriorSurface)
        .build()
        .wall_loops()[0]
        .clone()
}

fn region(candidates: Vec<SeamCandidate>) -> PerimeterRegionView {
    let mut view = PerimeterRegionView::default();
    view.set_object_id("obj-0".to_string());
    view.set_region_id(0);
    view.set_wall_loops(vec![wall_from_candidates(&candidates)]);
    view.set_infill_areas(vec![]);
    view.set_seam_candidates(candidates);
    view.set_resolved_seam(None);
    view
}

#[test]
fn sharp_preferred_over_aligned_at_same_score() {
    let cfg = ConfigView::from_map(HashMap::new()); // defaults to "nearest"
    let module = SeamPlacer::on_print_start(&cfg).unwrap();

    // Equal raw score; Sharp carries a negative reason bonus, Aligned does not,
    // so the Sharp candidate (x=1.0) must win on effective score.
    let candidates = vec![
        candidate(1.0, 0.5, SeamReason::Sharp),
        candidate(2.0, 0.5, SeamReason::Aligned),
    ];
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_wall_postprocess(0, &[region(candidates)], &mut output, &cfg)
        .unwrap();

    let seam = output.resolved_seam().expect("should resolve a seam");
    assert!(
        (seam.point.x - 1.0).abs() < 0.001,
        "Sharp must win over equal-score Aligned, expected x=1.0 got {}",
        seam.point.x
    );
}
