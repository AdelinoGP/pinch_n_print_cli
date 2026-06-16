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

// ── J. Determinism and isolation for perimeter commit ──────────────────

#[test]
fn perimeter_conversion_deterministic_across_repeated_calls() {
    use slicer_runtime::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected, Point3,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    let mk_output = || PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth {
                        x: 1.0,
                        y: 2.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    },
                    Point3WithWidth {
                        x: 3.0,
                        y: 4.0,
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
                    tool_index: Some(0),
                    fuzzy_skin: true,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
                WallFeatureFlag {
                    tool_index: Some(0),
                    fuzzy_skin: true,
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
                x: 2.0,
                y: 1.0,
                z: 0.2,
            },
            0.9,
        )],
        ..Default::default()
    };

    let ir_a = convert_perimeter_output(&mk_output(), 0).unwrap();
    let ir_b = convert_perimeter_output(&mk_output(), 0).unwrap();
    let ir_c = convert_perimeter_output(&mk_output(), 0).unwrap();

    assert_eq!(ir_a, ir_b, "run 0 and 1 should be identical");
    assert_eq!(ir_b, ir_c, "run 1 and 2 should be identical");
}

#[test]
fn real_perimeter_region_data_visible_through_wall_postprocess_dispatch() {
    // Guest encodes region_count as perimeter_index; wall count + infill count as x/y.
    let mut fx = dispatch_fixture::for_stage("Layer::PerimetersPostProcess")
        .with_slice(
            ir_builders::slice_ir::with_count(2)
                .at_layer(1)
                .at_z(0.2)
                .build(),
        )
        .with_perimeter(
            ir_builders::perimeter_ir::with_count(2)
                .at_layer(1)
                .walls(3)
                .infill(1)
                .build(),
        )
        .build();
    let layer = slicer_ir::GlobalLayer {
        index: 1,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    fx.run_layer(&layer)
        .expect("Layer::PerimetersPostProcess dispatch+commit should succeed");

    // Post-process replaces perimeter slot with guest's committed output;
    // each input region produces its own committed PerimeterRegion.
    let perim = fx
        .arena
        .perimeter()
        .expect("perimeter slot should be populated");
    assert_eq!(
        perim.regions.len(),
        2,
        "one PerimeterRegion per input region"
    );
    for (i, r) in perim.regions.iter().enumerate() {
        assert_eq!(r.object_id, format!("obj-{i}"), "object_id preserved");
        assert_eq!(r.region_id, i as u64, "region_id preserved");
        assert_eq!(r.walls.len(), 1, "guest emitted one wall-loop per region");
        let w = &r.walls[0];
        assert_eq!(w.perimeter_index, 3, "each region has 3 walls in input");
        let p = &w.path.points[0];
        assert_eq!(p.x, 3.0, "each region sees its own 3 walls");
        assert_eq!(p.y, 1.0, "each region sees its own 1 infill polygon");
    }
}
