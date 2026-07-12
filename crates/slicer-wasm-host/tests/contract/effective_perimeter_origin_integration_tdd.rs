//! Integration TDD for the `Layer::Perimeters` origin-fallback path
//! (`HostExecutionContext::effective_perimeter_origin`).
//!
//! Pre-fix, `Layer::Perimeters` guests (classic-perimeters, arachne-perimeters)
//! consumed `SliceRegionView` and never touched a `PerimeterRegionView`, so
//! `current_perimeter_region` stayed `None`. Origin-tagged pushes
//! (`set_infill_areas`, `push_wall_loop`) fell through to the "untagged" path
//! in `convert_perimeter_output`, which emitted a single `PerimeterRegion`
//! with `object_id = ""`. The downstream host region-partition then missed
//! every `(object_id, region_id)` HashMap lookup, leaving `sparse_infill_area`
//! empty — the cube "no sparse infill" symptom.
//!
//! AC-4: with only `current_slice_region` set, driving the host trait impls
//! for `set_infill_areas` and `push_wall_loop` and then converting the
//! collected output must produce a `PerimeterRegion` carrying the slice
//! region's UUID (NOT empty string).

#![allow(missing_docs)]

use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::HostPerimeterOutputBuilder;
use slicer_wasm_host::host::{
    convert_perimeter_output, ExPolygon, ExtrusionPath3d, ExtrusionRole,
    HostExecutionContextBuilder, Point2, Point3WithWidth, Polygon, WallFeatureFlag, WallLoopType,
    WallLoopView, WitWallBoundaryType,
};
use slicer_wasm_host::marshal::OriginId;

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

const TEST_UUID: &str = "uuid-perimeter-origin";
const TEST_REGION_ID: u64 = 11;

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
        boundary_type: WitWallBoundaryType::ExteriorSurface,
    }
}

#[test]
fn layer_perimeters_origin_falls_back_to_slice_region_through_host_trait() {
    // Build a context shaped like a `Layer::Perimeters` call: only the slice
    // region is set, the perimeter region is `None` (no `PerimeterRegionView`
    // exists at this stage).
    let mut ctx = HostExecutionContextBuilder::new("com.test.perimeters-origin", 0.0, 0.2).build();
    ctx.set_current_slice_region(Some(OriginId {
        object_id: TEST_UUID.to_string(),
        region_id: TEST_REGION_ID,
    }));
    assert!(
        ctx.current_perimeter_region().is_none(),
        "test precondition: Layer::Perimeters guests run without a PerimeterRegionView"
    );

    // Drive `set_infill_areas` and `push_wall_loop` through the trait impl —
    // exactly the path that origin-tagged pushes follow at runtime.
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

    // Convert the collected output to IR and assert the regions are tagged
    // with the slice region's UUID — not the pre-fix empty string.
    let perimeter_ir = convert_perimeter_output(ctx.perimeter_output(), 0)
        .expect("convert_perimeter_output must succeed");

    assert_eq!(
        perimeter_ir.regions.len(),
        1,
        "all tagged pushes share the same origin, so one region: {:#?}",
        perimeter_ir.regions
    );
    let region = &perimeter_ir.regions[0];
    assert_eq!(
        region.object_id, TEST_UUID,
        "regression: pre-fix this was empty string — the fallback to current_slice_region must surface the slice UUID"
    );
    assert_eq!(
        region.region_id, TEST_REGION_ID,
        "region_id must round-trip from the slice-region origin"
    );
    assert_eq!(
        region.walls.len(),
        1,
        "the pushed wall loop must land in the origin-tagged region"
    );
    assert_eq!(
        region.infill_areas.len(),
        1,
        "the set infill area must land in the origin-tagged region"
    );
}
