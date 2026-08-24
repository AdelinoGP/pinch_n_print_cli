//! AC-8: support eligibility suppresses auto contacts but preserves routing data.

use std::sync::Arc;

use slicer_ir::{
    ExPolygon, GlobalLayer, LayerPlanIR, MeshIR, ObjectSurfaceData, OverhangRegion, Point2,
    Polygon, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
    SurfaceClassificationIR,
};
use slicer_runtime::{
    builtins::support_analysis_producer::commit_support_analysis_builtin, Blackboard,
};

fn square(x: f32, y: f32, size: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, y),
                Point2::from_mm(x + size, y),
                Point2::from_mm(x + size, y + size),
                Point2::from_mm(x, y + size),
            ],
        },
        holes: Vec::new(),
    }
}

#[test]
fn needs_support_false_region_yields_no_auto_candidates() {
    let lower = square(1.0, 1.0, 3.0);
    let upper = square(0.0, 0.0, 5.0);
    let region_id = 17;
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR {
            global_layers: vec![
                GlobalLayer {
                    index: 0,
                    z: 0.2,
                    ..GlobalLayer::default()
                },
                GlobalLayer {
                    index: 1,
                    z: 0.4,
                    ..GlobalLayer::default()
                },
            ],
            ..LayerPlanIR::default()
        }))
        .unwrap();
    blackboard
        .commit_slice_ir(Arc::new(vec![
            SliceIR {
                global_layer_index: 0,
                regions: vec![SlicedRegion {
                    object_id: "object".into(),
                    region_id,
                    polygons: vec![lower],
                    ..SlicedRegion::default()
                }],
                ..SliceIR::default()
            },
            SliceIR {
                global_layer_index: 1,
                regions: vec![SlicedRegion {
                    object_id: "object".into(),
                    region_id,
                    polygons: vec![upper],
                    ..SlicedRegion::default()
                }],
                ..SliceIR::default()
            },
        ]))
        .unwrap();

    let mut region_map = RegionMapIR::default();
    let config_id = region_map.intern_config(ResolvedConfig::default());
    for global_layer_index in 0..=1 {
        region_map.entries.insert(
            RegionKey {
                global_layer_index,
                object_id: "object".into(),
                region_id,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: config_id,
                ..RegionPlan::default()
            },
        );
    }
    blackboard.commit_region_map(Arc::new(region_map)).unwrap();

    // The overhang footprint is disjoint from the sliced region, so the region
    // derives `needs_support == false`; there is no enforcer in this fixture.
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR {
            per_object: [(
                "object".into(),
                ObjectSurfaceData {
                    overhang_regions: vec![OverhangRegion {
                        xy_footprint: vec![square(20.0, 20.0, 1.0)],
                        ..OverhangRegion::default()
                    }],
                    ..ObjectSurfaceData::default()
                },
            )]
            .into_iter()
            .collect(),
            ..SurfaceClassificationIR::default()
        }))
        .unwrap();

    commit_support_analysis_builtin(
        &mut blackboard,
        &ResolvedConfig {
            support_enabled: true,
            ..ResolvedConfig::default()
        },
    )
    .unwrap();
    let analysis = blackboard.support_analysis().unwrap();

    assert!(
        analysis
            .candidates
            .iter()
            .all(|candidate| candidate.source.region_id != region_id),
        "an ineligible region must not produce an auto-detected candidate"
    );
    assert_eq!(
        analysis
            .family_assignments
            .get(&(String::from("object"), region_id)),
        Some(&String::from("traditional")),
        "suppression must preserve the structured family assignment for downstream decline"
    );
}
