// dispatch_perimeter_output_tdd.rs — Layer::Perimeters / Layer::PerimetersPostProcess output commitment
// (wall loops, infill areas, seam candidates)

use crate::common::*;

#[test]
fn perimeter_output_converts_wall_loops_and_commits_to_arena() {
    use slicer_runtime::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected, Point3,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    Point3WithWidth {
                        x: 10.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
                WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: vec![(
            Point3 {
                x: 5.0,
                y: 0.0,
                z: 0.2,
            },
            0.8,
        )],
        ..Default::default()
    };

    let ir = convert_perimeter_output(&output, 3).expect("valid perimeter output should convert");
    assert_eq!(ir.global_layer_index, 3);
    assert_eq!(ir.regions.len(), 1);
    assert_eq!(ir.regions[0].walls.len(), 1);
    assert_eq!(ir.regions[0].walls[0].perimeter_index, 0);
    assert_eq!(ir.regions[0].walls[0].loop_type, slicer_ir::LoopType::Outer);
    assert_eq!(ir.regions[0].walls[0].path.points.len(), 2);
    assert_eq!(ir.regions[0].walls[0].feature_flags.len(), 2);
    assert_eq!(ir.regions[0].seam_candidates.len(), 1);
    assert_eq!(ir.regions[0].seam_candidates[0].score, 0.8);
}

#[test]
fn perimeter_output_rejects_nan_in_wall_loop_path() {
    use slicer_runtime::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![Point3WithWidth {
                    x: f32::NAN,
                    y: 0.0,
                    z: 0.0,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                }],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            }],
        }],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "NaN in wall loop path should be rejected");
    let msg = result.unwrap_err();
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
}

#[test]
fn perimeter_output_rejects_feature_flags_cardinality_mismatch() {
    use slicer_runtime::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    // 2 points but only 1 feature flag → cardinality mismatch per docs/03
    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    Point3WithWidth {
                        x: 10.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
                // Missing second flag
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(
        result.is_err(),
        "feature flag cardinality mismatch should be rejected"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("feature_flags length") && msg.contains("path points length"),
        "error should describe cardinality mismatch: {msg}"
    );
}

#[test]
fn perimeter_output_rejects_nan_seam_candidate() {
    use slicer_runtime::wit_host::{convert_perimeter_output, PerimeterOutputCollected, Point3};

    let output = PerimeterOutputCollected {
        wall_loops: Vec::new(),
        infill_areas: Vec::new(),
        seam_candidates: vec![(
            Point3 {
                x: f32::NAN,
                y: 0.0,
                z: 0.0,
            },
            1.0,
        )],
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "NaN seam candidate should be rejected");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("seam_candidate"),
        "error should identify seam: {msg}"
    );
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
}

#[test]
fn empty_perimeter_output_does_not_populate_arena() {
    let mut fx = dispatch_fixture::for_stage("Layer::Perimeters").build();

    let layer = slicer_ir::GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    fx.run_layer(&layer)
        .expect("Layer::Perimeters dispatch+commit should succeed");
    assert!(
        fx.arena.perimeter().is_none(),
        "perimeter slot should be empty for no-op"
    );
}
