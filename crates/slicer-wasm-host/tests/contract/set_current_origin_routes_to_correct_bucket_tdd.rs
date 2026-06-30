//! Integration TDD for the `HostPerimeterOutputBuilder::set_current_origin` explicit
//! origin propagation path (AC-4).
//!
//! Pre-fix, origin-tagged pushes without a `current_perimeter_region` or
//! `current_slice_region` fell through to the anonymous fallback and emitted a
//! single `PerimeterRegion` with `object_id = ""`. AC-4 requires that an explicit
//! `set_current_origin` call sets `explicit_perimeter_origin`, which
//! `effective_perimeter_origin` then selects over the other two fallback sources.
//!
//! With ONLY `explicit_perimeter_origin` set, driving `set_infill_areas` and
//! `push_wall_loop` through the trait impl must produce a `PerimeterRegion`
//! whose `object_id` and `region_id` match the explicit origin.

#![allow(missing_docs)]

use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::HostPerimeterOutputBuilder;
use slicer_wasm_host::host::{
    convert_perimeter_output, ExPolygon, ExtrusionPath3d, ExtrusionRole,
    HostExecutionContextBuilder, Point2, Point3WithWidth, Polygon, WallFeatureFlag, WallLoopType,
    WallLoopView,
};

fn make_feature_flag() -> WallFeatureFlag {
    WallFeatureFlag {
        tool_index: None,
        fuzzy_skin: false,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: Vec::new(),
    }
}

const TEST_UUID: &str = "uuid-explicit-origin";
const TEST_REGION_ID: u64 = 42;

fn make_expolygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 10, y: 0 },
                Point2 { x: 10, y: 10 },
                Point2 { x: 0, y: 10 },
            ],
        },
        holes: Vec::new(),
    }
}

fn make_wall_loop() -> WallLoopView {
    WallLoopView {
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
        feature_flags: vec![make_feature_flag(), make_feature_flag()],
    }
}

#[test]
fn set_current_origin_routes_to_correct_bucket() {
    // Build a context with no ambient origin sources at all.
    let mut ctx = HostExecutionContextBuilder::new("com.test.explicit-origin", 0.0, 0.2).build();
    assert!(
        ctx.current_slice_region().is_none(),
        "test precondition: no slice region is set"
    );
    assert!(
        ctx.current_perimeter_region().is_none(),
        "test precondition: no perimeter region is set"
    );

    // Set an explicit origin through the WIT trait method.
    let origin_handle = ctx
        .push_perimeter_output_builder()
        .expect("push perimeter output builder for set_current_origin");
    let origin_result = <slicer_wasm_host::host::HostExecutionContext as HostPerimeterOutputBuilder>::set_current_origin(
        &mut ctx,
        origin_handle,
        TEST_UUID.to_string(),
        TEST_REGION_ID.to_string(),
    )
    .expect("host call must succeed");
    assert!(
        origin_result.is_ok(),
        "set_current_origin: {origin_result:?}"
    );

    // Drive `set_infill_areas` with a fresh builder handle.
    let infill_handle = ctx
        .push_perimeter_output_builder()
        .expect("push perimeter output builder for set_infill_areas");
    let infill_result = <slicer_wasm_host::host::HostExecutionContext as HostPerimeterOutputBuilder>::set_infill_areas(
        &mut ctx,
        infill_handle,
        vec![make_expolygon()],
    )
    .expect("host call must succeed");
    assert!(infill_result.is_ok(), "set_infill_areas: {infill_result:?}");

    // Drive `push_wall_loop` with a fresh builder handle.
    let wall_handle = ctx
        .push_perimeter_output_builder()
        .expect("push perimeter output builder for push_wall_loop");
    let wall_result = <slicer_wasm_host::host::HostExecutionContext as HostPerimeterOutputBuilder>::push_wall_loop(
        &mut ctx,
        wall_handle,
        make_wall_loop(),
    )
    .expect("host call must succeed");
    assert!(wall_result.is_ok(), "push_wall_loop: {wall_result:?}");

    // Convert the collected output to IR and assert the region is tagged with
    // the explicit origin — not the pre-fix empty string anonymous fallback.
    let perimeter_ir = convert_perimeter_output(ctx.perimeter_output(), 0)
        .expect("convert_perimeter_output must succeed");

    assert_eq!(
        perimeter_ir.regions.len(),
        1,
        "all tagged pushes share the explicit origin, so one region: {:#?}",
        perimeter_ir.regions
    );
    let region = &perimeter_ir.regions[0];
    assert_eq!(
        region.object_id, TEST_UUID,
        "regression: pre-fix this was empty string — set_current_origin must route to its explicit object_id"
    );
    assert_eq!(
        region.region_id, TEST_REGION_ID,
        "region_id must round-trip from set_current_origin"
    );
    assert_eq!(
        region.walls.len(),
        1,
        "the pushed wall loop must land in the explicit-origin region"
    );
    assert_eq!(
        region.infill_areas.len(),
        1,
        "the set infill area must land in the explicit-origin region"
    );
}
