use crate::common::*;
use slicer_ir::{
    ExPolygon, GlobalLayer, PaintSemantic, PaintValue, Point2, Polygon, SliceIR, SlicedRegion,
};
use slicer_runtime::wit_host::*;
use std::collections::HashMap;

#[test]
fn empty_guest_output_does_not_populate_arena() {
    let mut fx = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .no_wasm()
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };

    fx.run_layer(&layer).expect("dispatch should succeed");

    assert!(
        fx.arena.support().is_none(),
        "support slot should be empty for no-op stage"
    );
}

#[test]
fn real_paint_region_data_visible_through_production_support_dispatch() {
    use slicer_sdk::traits::{PaintRegionLayerView, SupportPaintPolicy};
    use std::sync::Arc;

    fn enclosing_square() -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(-10.0, -10.0),
                    Point2::from_mm(10.0, -10.0),
                    Point2::from_mm(10.0, 10.0),
                    Point2::from_mm(-10.0, 10.0),
                ],
            },
            holes: vec![],
        }
    }
    fn test_probe_polygon() -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(-1.0, -1.0),
                    Point2::from_mm(1.0, -1.0),
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(-1.0, 1.0),
                ],
            },
            holes: vec![],
        }
    }
    fn region_with(polygons: Vec<ExPolygon>, semantics: &[PaintSemantic]) -> SlicedRegion {
        let mut segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> =
            HashMap::new();
        for sem in semantics {
            segment_annotations.insert(sem.clone(), vec![vec![Some(PaintValue::Flag(true))]]);
        }
        SlicedRegion {
            object_id: "obj1".to_string(),
            region_id: 0u64,
            polygons,
            segment_annotations,
            ..Default::default()
        }
    }
    fn view(semantics: &[PaintSemantic]) -> PaintRegionLayerView {
        let slice = SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.2,
            regions: vec![region_with(vec![enclosing_square()], semantics)],
        };
        PaintRegionLayerView::new(0).with_slice_ir(Arc::new(slice))
    }

    let probe = test_probe_polygon();

    // (a) BLOCKER alone → Blocked.
    assert_eq!(
        view(&[PaintSemantic::SupportBlocker]).paint_policy_for(&probe),
        SupportPaintPolicy::Blocked,
        "production support dispatch must surface SupportBlocker as Blocked"
    );

    // (b) ENFORCER alone → Enforced.
    assert_eq!(
        view(&[PaintSemantic::SupportEnforcer]).paint_policy_for(&probe),
        SupportPaintPolicy::Enforced,
        "production support dispatch must surface SupportEnforcer as Enforced"
    );

    // (c) BLOCKER + ENFORCER → Blocked (precedence per docs/10 §"Scenario Trace 2").
    assert_eq!(
        view(&[
            PaintSemantic::SupportBlocker,
            PaintSemantic::SupportEnforcer,
        ])
        .paint_policy_for(&probe),
        SupportPaintPolicy::Blocked,
        "blocker > enforcer precedence must hold when both annotations apply"
    );

    // (d) Neither → DefaultEligible.
    assert_eq!(
        view(&[]).paint_policy_for(&probe),
        SupportPaintPolicy::DefaultEligible,
        "absent annotations must surface as DefaultEligible (defer to overhang-angle / needs_support)"
    );

    // (e) No SliceIR attached → DefaultEligible (host hasn't wired the view).
    assert_eq!(
        PaintRegionLayerView::new(0).paint_policy_for(&probe),
        SupportPaintPolicy::DefaultEligible,
        "view without attached SliceIR must default to eligible (no policy override)"
    );
}

#[test]
fn support_postprocess_empty_bypass_when_no_slice_regions() {
    let mut fx = dispatch_fixture::for_stage("Layer::SupportPostProcess").build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    fx.run_layer(&layer).unwrap();

    assert!(
        fx.arena.support().is_none(),
        "empty-input post-process: empty bypass preserved"
    );
}

#[test]
fn support_postprocess_commit_preserves_distinct_region_identities() {
    let mut fx = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .with_slice(ir_builders::slice_ir::with_count(2).build())
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    fx.run_layer(&layer).unwrap();

    let support = fx.arena.support().expect("support populated");
    assert_eq!(
        support.support_paths.len(),
        2,
        "two origin-tagged paths preserved"
    );
    assert_eq!(
        support.support_paths[0].points[0].x, 1.0,
        "region 0 has 1 polygon"
    );
    assert_eq!(
        support.support_paths[1].points[0].x, 1.0,
        "region 1 has 1 polygon"
    );
}

#[test]
fn support_postprocess_identity_isolation_across_dispatches() {
    let mut fx1 = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .with_slice(ir_builders::slice_ir::with_count(3).build())
        .build();
    let mut fx2 = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .with_slice(ir_builders::slice_ir::with_count(1).build())
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    fx1.run_layer(&layer).unwrap();
    fx2.run_layer(&layer).unwrap();

    assert_eq!(
        fx1.arena.support().unwrap().support_paths.len(),
        3,
        "dispatch 1 kept its 3 regions"
    );
    assert_eq!(
        fx2.arena.support().unwrap().support_paths.len(),
        1,
        "dispatch 2 kept its 1 region (no leak)"
    );
}

#[test]
fn support_output_rejects_untagged_push_in_identity_mode() {
    // Manual collected output with mixed tagged/untagged pushes.
    let mk_path = || ExtrusionPath3d {
        points: vec![Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        }],
        role: ExtrusionRole::SupportMaterial,
        speed_factor: 1.0,
    };
    let output = SupportOutputCollected {
        support_paths: vec![mk_path(), mk_path()],
        support_path_origins: vec![Some(("obj-0".into(), 0)), None],
        ..Default::default()
    };

    let result = convert_support_output(&output, 0);
    assert!(result.is_err(), "untagged push in identity mode must fail");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("active slice source region") || msg.contains("without an active"),
        "diagnostic should explain missing region context: {msg}"
    );
}

#[test]
fn slice_postprocess_downstream_propagation_preserves_per_region_shape() {
    let mut fx = dispatch_fixture::for_stage("Layer::SlicePostProcess")
        .with_slice(ir_builders::slice_ir::with_count(3).build())
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    fx.run_layer(&layer).unwrap();

    // Now dispatch a downstream stage that consumes slice regions (Support).
    let mut fx_sup = dispatch_fixture::for_stage("Layer::SupportPostProcess")
        .with_slice(fx.arena.slice().unwrap().clone())
        .build();
    fx_sup.run_layer(&layer).unwrap();

    assert!(fx_sup.arena.support().is_some());
}
