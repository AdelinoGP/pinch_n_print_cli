//! Direct host-trait contract coverage for `slice-region-view`.

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_wasm_host::host::layer::slicer::config::config_types::HostConfigView;
use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::HostSliceRegionView;
use slicer_wasm_host::host::{
    ConfigValueStorage, ExPolygon, HostExecutionContextBuilder, Point2, Polygon, SliceRegionData,
};
use wasmtime::component::Resource;

fn own<T>(rep: u32) -> Resource<T> {
    Resource::new_own(rep)
}

fn square() -> ExPolygon {
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

#[test]
fn slice_region_view_contract() {
    let mut ctx = HostExecutionContextBuilder::new("slice-region-contract", 1.25, 0.2).build();
    let mut config = HashMap::new();
    config.insert("region_metadata".to_string(), ConfigValueStorage::Int(17));
    ctx.set_default_config_fields(config);

    let polygon = square();
    let region = ctx
        .push_slice_region(SliceRegionData {
            object_id: "object-172".into(),
            region_id: "7".into(),
            polygons: vec![polygon.clone()],
            infill_areas: vec![polygon],
            effective_layer_height: 0.18,
            z: 1.32,
            has_nonplanar: true,
            segment_annotations: Vec::new(),
            variant_chain: Vec::new(),
            needs_support: true,
            top_shell_index: Some(2),
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
            held_claims: Vec::new(),
            overhang_areas: Vec::new(),
            overhang_quartile_polygons: Vec::new(),
            surface_group: None,
        })
        .unwrap();
    let rep = region.rep();

    assert_eq!(
        HostSliceRegionView::object_id(&mut ctx, own(rep)).unwrap(),
        "object-172"
    );
    assert_eq!(
        HostSliceRegionView::region_id(&mut ctx, own(rep)).unwrap(),
        "7"
    );
    assert_eq!(HostSliceRegionView::z(&mut ctx, own(rep)).unwrap(), 1.32);
    assert_eq!(
        HostSliceRegionView::effective_layer_height(&mut ctx, own(rep)).unwrap(),
        0.18
    );
    assert_eq!(
        HostSliceRegionView::polygons(&mut ctx, own(rep))
            .unwrap()
            .len(),
        1
    );
    assert!(HostSliceRegionView::has_nonplanar(&mut ctx, own(rep)).unwrap());

    let config = HostSliceRegionView::config(&mut ctx, own(rep)).unwrap();
    let config_rep = config.rep();
    assert_eq!(
        HostConfigView::get_int(&mut ctx, own(config_rep), "region_metadata".into()).unwrap(),
        Some(17)
    );
    HostConfigView::drop(&mut ctx, own(config_rep)).unwrap();

    HostSliceRegionView::drop(&mut ctx, own(rep)).unwrap();
    assert!(HostSliceRegionView::object_id(&mut ctx, own(rep)).is_err());
}
