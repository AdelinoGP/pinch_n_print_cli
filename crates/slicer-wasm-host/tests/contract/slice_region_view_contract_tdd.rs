//! Direct host-trait contract coverage for `slice-region-view`.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;

use slicer_wasm_host::host::layer::slicer::config::config_types::HostConfigView;
use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::HostSliceRegionView;
use slicer_wasm_host::host::{
    layer_perimeters, ConfigValueStorage, ConfigViewData, ExPolygon, HostExecutionContext,
    HostExecutionContextBuilder, PaintRegionLayerData, Point2, Polygon, SliceRegionData,
};
use slicer_wasm_host::marshal::OriginId;
use wasmtime::component::Resource;

fn own<T>(rep: u32) -> Resource<T> {
    Resource::new_own(rep)
}

fn square() -> ExPolygon {
    square_with_bounds(0, 10)
}

fn square_with_bounds(min: i64, max: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min, y: min },
                Point2 { x: max, y: min },
                Point2 { x: max, y: max },
                Point2 { x: min, y: max },
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
        .push_slice_region(
            // exhaustive: slice-region view fixture supplies every stored field
            SliceRegionData {
                prev_layer_boundary: Vec::new(),
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
            },
        )
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

#[test]
fn prev_layer_boundary_reaches_live_perimeters_guest_view() {
    let boundary = square_with_bounds(-20_000, 120_000);
    let mut ctx = HostExecutionContextBuilder::new("slice-region-live", 0.4, 0.2).build();
    let mut fields = HashMap::new();
    fields.insert("wall_count".into(), ConfigValueStorage::Int(2));
    fields.insert(
        "inner_wall_line_width".into(),
        ConfigValueStorage::Float(0.4),
    );
    fields.insert(
        "outer_wall_line_width".into(),
        ConfigValueStorage::Float(0.4),
    );
    let config_handle = ctx
        .push_config_view(ConfigViewData { fields })
        .expect("config resource");
    let region_handle = ctx
        .push_slice_region(
            // exhaustive: live-perimeter fixture supplies every stored field
            SliceRegionData {
                prev_layer_boundary: vec![boundary.clone()],
                object_id: "object-live".into(),
                region_id: "7".into(),
                polygons: vec![square_with_bounds(0, 100_000)],
                infill_areas: Vec::new(),
                effective_layer_height: 0.2,
                z: 0.4,
                has_nonplanar: false,
                segment_annotations: Vec::new(),
                variant_chain: Vec::new(),
                needs_support: true,
                top_shell_index: None,
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
            },
        )
        .expect("slice-region resource");
    let rep = region_handle.rep();
    let returned_boundary = HostSliceRegionView::prev_layer_boundary(&mut ctx, own(rep)).unwrap();
    assert_eq!(returned_boundary.len(), 1);
    assert_eq!(returned_boundary[0].holes.len(), boundary.holes.len());
    assert_eq!(
        returned_boundary[0].contour.points.len(),
        boundary.contour.points.len()
    );
    for (actual, expected) in returned_boundary[0]
        .contour
        .points
        .iter()
        .zip(boundary.contour.points.iter())
    {
        assert_eq!(actual.x, expected.x);
        assert_eq!(actual.y, expected.y);
    }
    let paint_handle = ctx
        .push_paint_region_layer_view(
            // exhaustive: paint-region view fixture supplies every stored field
            PaintRegionLayerData {
                layer_index: 0,
                regions_by_semantic: HashMap::new(),
                custom_regions: HashMap::new(),
                support_plan_segments: HashMap::new(),
                support_plan_entries: HashMap::new(),
                lightning_tree_segments: HashMap::new(),
            },
        )
        .expect("paint resource");
    let output_handle = ctx
        .push_perimeter_output_builder()
        .expect("perimeter output resource");
    ctx.set_current_slice_region(Some(OriginId {
        object_id: "object-live".into(),
        region_id: 7,
    }));

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let component_path = manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .expect("slicer-wasm-host manifest directory should have a workspace root")
        .join("modules")
        .join("core-modules")
        .join("arachne-perimeters")
        .join("arachne-perimeters.wasm");
    let engine = crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone();
    let component = wasmtime::component::Component::from_file(&engine, &component_path)
        .expect("load production arachne-perimeters component");
    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    layer_perimeters::LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
        &mut linker,
        |ctx| ctx,
    )
    .expect("add perimeters linker");
    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = layer_perimeters::LayerModule::instantiate(&mut store, &component, &linker)
        .expect("instantiate production arachne-perimeters component");
    bindings
        .slicer_layer_perimeters_perimeters()
        .call_run(
            &mut store,
            0,
            &[own(region_handle.rep())],
            own(paint_handle.rep()),
            own(output_handle.rep()),
            own(config_handle.rep()),
        )
        .expect("call run-perimeters")
        .expect("guest run-perimeters result");

    let ctx = store.into_data();
    let perimeter = slicer_wasm_host::host::convert_perimeter_output(ctx.perimeter_output(), 0)
        .expect("convert perimeter output");
    let points = perimeter
        .regions
        .iter()
        .flat_map(|region| region.walls.iter())
        .flat_map(|wall| wall.path.points.iter())
        .collect::<Vec<_>>();
    assert!(!points.is_empty(), "guest should emit perimeter points");
    assert!(
        points
            .iter()
            .all(|point| point.overhang_distance_mm.is_some()),
        "a fully supported region must receive distance stamps through the live adapter"
    );
}
