//! TDD tests for WIT/component-model data marshaling boundary.
//!
//! These tests prove that real data crosses the host<->guest component boundary:
//! - Config values are readable by the guest
//! - IR view data (slice regions) is readable by the guest
//! - Output builder writes arrive at the host
//! - Host services (logging) work
//! - Pool correctness with real data-bound calls

use std::collections::HashMap;

use slicer_wasm_host::host::{
    ConfigValueStorage, ConfigViewData, HostExecutionContext, HostExecutionContextBuilder,
    LayerModule, SliceRegionData,
};
use witness::{RawInfillWitness, RawInfillWitnessPoint1};

use crate::common::wasm_cache;

/// Process-shared wasmtime engine (with component model enabled) — a cheap
/// `Arc` clone of the same engine the rest of the test binary uses.
fn make_engine() -> wasmtime::Engine {
    wasm_cache::shared_engine().wasmtime_engine().clone()
}

/// Cached compiled guest component. `wasmtime::component::Component` is
/// internally `Arc`-backed so the clone is cheap.
fn cached_guest() -> wasmtime::component::Component {
    wasm_cache::compiled_guest("layer-infill-guest")
        .wasmtime_component()
        .clone()
}

fn make_ctx(module_id: impl Into<String>, layer_z: f32) -> HostExecutionContext {
    // mesh_ir is None â€” these WIT boundary tests exercise config/IR/output
    // paths and do not require live mesh data.
    HostExecutionContextBuilder::new(module_id.into(), layer_z, 1.0).build()
}

// â”€â”€ A: Config access across the boundary â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The guest reads `infill-spacing` from the config-view and uses its value
/// to determine the X extent of the emitted sparse path. We provide spacing=3.5
/// and verify the output path has x = 3.5 * 10 = 35.0 for the second point.
#[test]
fn guest_reads_config_value_and_uses_it_in_output() {
    let engine = make_engine();
    let component = cached_guest();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .expect("add_to_linker");

    let mut ctx = make_ctx("test-infill-module", 1.0);

    // Provide config with infill-spacing = 3.5
    let mut fields = HashMap::new();
    fields.insert("infill-spacing".into(), ConfigValueStorage::Float(3.5));
    let config_handle = ctx.push_config_view(ConfigViewData { fields }).unwrap();

    // Provide one slice region at z=1.0
    let region_handle = ctx
        .push_slice_region(SliceRegionData {
            object_id: "obj-1".into(),
            region_id: "1".into(),
            polygons: vec![],
            infill_areas: vec![],
            effective_layer_height: 0.2,
            z: 1.0,
            has_nonplanar: false,
            segment_annotations: vec![],
            needs_support: true,
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            held_claims: Vec::new(),
        })
        .unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).expect("instantiate");

    // Call run-infill
    let result = bindings.call_run_infill(
        &mut store,
        0, // layer_index
        &[resource_to_own(region_handle)],
        resource_to_own(output_handle),
        resource_to_own(config_handle),
    );

    assert!(result.is_ok(), "call_run_infill failed: {:?}", result.err());
    let inner = result.unwrap();
    assert!(inner.is_ok(), "module returned error: {:?}", inner.err());

    // Extract outputs from the context
    let ctx = store.into_data();

    // Verify config was read: spacing=3.5 â†’ second point x = 3.5*10 = 35.0
    assert_eq!(
        ctx.infill_output().sparse_paths.len(),
        1,
        "expected 1 sparse path"
    );
    let path = &ctx.infill_output().sparse_paths[0];
    assert_eq!(path.points.len(), 2, "expected 2 points");
    // Decode via RawInfillWitnessPoint1 field names (WIT-domain points; construct from fields)
    let spacing_x10 = path.points[1].x;
    let _ = RawInfillWitnessPoint1 { spacing_x10 };
    assert!(
        (spacing_x10 - 35.0).abs() < 0.001,
        "second point spacing_x10 should be 35.0 (spacing*10), got {}",
        spacing_x10
    );

    // Verify z was read from region data (RawInfillWitness::first_region_z = point[0].z)
    let first_region_z = path.points[0].z;
    let _ = RawInfillWitness {
        first_region_z,
        total_polys: path.points[0].width,
        region_count: path.points[0].flow_factor,
    };
    assert!(
        (first_region_z - 1.0).abs() < 0.001,
        "first_region_z should be 1.0 (from region), got {}",
        first_region_z
    );
}

// â”€â”€ B: IR/read-view access across the boundary â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The guest reads z from the slice-region-view and uses it in the output path.
/// We provide z=5.5 and verify the output path has z=5.5.
#[test]
fn guest_reads_region_z_from_ir_view() {
    let engine = make_engine();
    let component = cached_guest();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .unwrap();

    let mut ctx = make_ctx("test-ir-read", 5.5);

    let config_handle = ctx
        .push_config_view(ConfigViewData {
            fields: HashMap::new(),
        })
        .unwrap();
    let region_handle = ctx
        .push_slice_region(SliceRegionData {
            object_id: "obj-z-test".into(),
            region_id: "2".into(),
            polygons: vec![],
            infill_areas: vec![],
            effective_layer_height: 0.3,
            z: 5.5, // distinctive z value
            has_nonplanar: false,
            segment_annotations: vec![],
            needs_support: true,
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            held_claims: Vec::new(),
        })
        .unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    bindings
        .call_run_infill(
            &mut store,
            42,
            &[resource_to_own(region_handle)],
            resource_to_own(output_handle),
            resource_to_own(config_handle),
        )
        .unwrap()
        .unwrap();

    let ctx = store.into_data();
    assert_eq!(ctx.infill_output().sparse_paths.len(), 1);
    // RawInfillWitness layout: first_region_z = point[0].z
    let first_region_z = ctx.infill_output().sparse_paths[0].points[0].z;
    let _ = RawInfillWitness {
        first_region_z,
        total_polys: ctx.infill_output().sparse_paths[0].points[0].width,
        region_count: ctx.infill_output().sparse_paths[0].points[0].flow_factor,
    };
    assert!(
        (first_region_z - 5.5).abs() < 0.001,
        "first_region_z should be 5.5, got {first_region_z}"
    );
}

// â”€â”€ C: Output emission across the boundary â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The guest pushes a sparse path via the infill-output-builder.
/// Verify the host received the path with correct structure.
#[test]
fn guest_emits_output_via_infill_builder() {
    let engine = make_engine();
    let component = cached_guest();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .unwrap();

    let mut ctx = make_ctx("test-output", 2.0);
    let config_handle = ctx
        .push_config_view(ConfigViewData {
            fields: HashMap::new(),
        })
        .unwrap();
    let region_handle = ctx
        .push_slice_region(SliceRegionData {
            object_id: "obj-out".into(),
            region_id: "3".into(),
            polygons: vec![],
            infill_areas: vec![],
            effective_layer_height: 0.2,
            z: 2.0,
            has_nonplanar: false,
            segment_annotations: vec![],
            needs_support: true,
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            held_claims: Vec::new(),
        })
        .unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    bindings
        .call_run_infill(
            &mut store,
            0,
            &[resource_to_own(region_handle)],
            resource_to_own(output_handle),
            resource_to_own(config_handle),
        )
        .unwrap()
        .unwrap();

    let ctx = store.into_data();
    // Guest must have pushed exactly one sparse path
    assert_eq!(ctx.infill_output().sparse_paths.len(), 1);
    let path = &ctx.infill_output().sparse_paths[0];
    // Path must have 2 points
    assert_eq!(path.points.len(), 2);
    // Role must be sparse-infill
    assert!(matches!(
        path.role,
        slicer_wasm_host::host::ExtrusionRole::SparseInfill
    ));
    // Guest encodes region count in flow_factor and polygon count in width (RawInfillWitness layout).
    // 1 region passed with 0 polygons:
    let rw = RawInfillWitness {
        region_count: path.points[0].flow_factor,
        total_polys: path.points[0].width,
        first_region_z: path.points[0].z,
    };
    assert_eq!(rw.region_count, 1.0, "1 region passed");
    assert_eq!(rw.total_polys, 0.0, "0 polygons in the region");
    // Second point has standard padding width from guest code
    assert!((path.points[1].width - 0.4).abs() < 0.001);
}

// â”€â”€ D: Host services (logging) across the boundary â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The guest calls host-services.log. Verify the host received the log message.
#[test]
fn guest_logs_via_host_services() {
    let engine = make_engine();
    let component = cached_guest();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .unwrap();

    let mut ctx = make_ctx("test-log", 0.2);
    let config_handle = ctx
        .push_config_view(ConfigViewData {
            fields: HashMap::new(),
        })
        .unwrap();
    let region_handle = ctx
        .push_slice_region(SliceRegionData {
            object_id: "obj-log".into(),
            region_id: "4".into(),
            polygons: vec![],
            infill_areas: vec![],
            effective_layer_height: 0.2,
            z: 0.2,
            has_nonplanar: false,
            segment_annotations: vec![],
            needs_support: true,
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            held_claims: Vec::new(),
        })
        .unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    bindings
        .call_run_infill(
            &mut store,
            7,
            &[resource_to_own(region_handle)],
            resource_to_own(output_handle),
            resource_to_own(config_handle),
        )
        .unwrap()
        .unwrap();

    let ctx = store.into_data();
    // Guest logs "run-infill: layer=7, ..."
    assert!(
        !ctx.log_messages().is_empty(),
        "expected at least one log message"
    );
    let (level, msg) = &ctx.log_messages()[0];
    assert_eq!(level, "info");
    assert!(
        msg.contains("layer=7"),
        "log should contain layer=7, got: {msg}"
    );
    assert!(
        msg.contains("regions=1"),
        "log should mention 1 region, got: {msg}"
    );
}

// â”€â”€ E: Repeated calls with fresh context (pool correctness) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Multiple calls to the same component with fresh contexts produce
/// independent outputs â€” no cross-call data contamination.
#[test]
fn repeated_calls_produce_independent_outputs() {
    let engine = make_engine();
    let component = cached_guest();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .unwrap();

    for i in 0..3 {
        let z = (i + 1) as f32 * 10.0;
        let mut ctx = make_ctx(format!("call-{i}"), z);
        let config_handle = ctx
            .push_config_view(ConfigViewData {
                fields: HashMap::new(),
            })
            .unwrap();
        let region_handle = ctx
            .push_slice_region(SliceRegionData {
                object_id: format!("obj-{i}"),
                region_id: (i + 1).to_string(),
                polygons: vec![],
                infill_areas: vec![],
                effective_layer_height: 0.2,
                z,
                has_nonplanar: false,
                segment_annotations: vec![],
                needs_support: true,
                top_shell_index: None,
                bottom_shell_index: None,
                top_solid_fill: Vec::new(),
                bottom_solid_fill: Vec::new(),
                is_bridge: false,
                bridge_areas: vec![],
                bridge_orientation_deg: 0.0,
                held_claims: Vec::new(),
            })
            .unwrap();
        let output_handle = ctx.push_infill_output_builder().unwrap();

        let mut store = wasmtime::Store::new(&engine, ctx);
        let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

        bindings
            .call_run_infill(
                &mut store,
                i,
                &[resource_to_own(region_handle)],
                resource_to_own(output_handle),
                resource_to_own(config_handle),
            )
            .unwrap()
            .unwrap();

        let ctx = store.into_data();
        // Each call should have exactly one path
        assert_eq!(
            ctx.infill_output().sparse_paths.len(),
            1,
            "call {i}: expected 1 path"
        );
        // Each call should have the z from its own region (RawInfillWitness::first_region_z = point[0].z)
        let actual_z = ctx.infill_output().sparse_paths[0].points[0].z;
        let _ = RawInfillWitness {
            first_region_z: actual_z,
            total_polys: 0.0,
            region_count: 0.0,
        };
        assert!(
            (actual_z - z).abs() < 0.001,
            "call {i}: first_region_z should be {z}, got {actual_z}"
        );
        // No log messages from previous calls
        assert_eq!(
            ctx.log_messages().len(),
            1,
            "call {i}: expected 1 log message, got {}",
            ctx.log_messages().len()
        );
    }
}

// â”€â”€ F: Empty region list handled correctly â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// When no regions are provided, the guest returns Ok without pushing any
/// output (early-exit path in run_infill when regions is empty).
#[test]
fn empty_region_list_handled_gracefully() {
    let engine = make_engine();
    let component = cached_guest();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .unwrap();

    let mut ctx = make_ctx("test-empty", 0.0);
    let config_handle = ctx
        .push_config_view(ConfigViewData {
            fields: HashMap::new(),
        })
        .unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    // Call with empty regions list â€” guest returns Ok immediately, no paths pushed.
    bindings
        .call_run_infill(
            &mut store,
            0,
            &[], // no regions
            resource_to_own(output_handle),
            resource_to_own(config_handle),
        )
        .unwrap()
        .unwrap();

    let ctx = store.into_data();
    assert_eq!(ctx.infill_output().sparse_paths.len(), 0);
}

// â”€â”€ Helper: convert Resource to the right type â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Convert a `Resource<T>` to `Resource<U>` by preserving the rep.
/// This is needed because the call methods expect `Resource<WitType>`
/// but our push methods return `Resource<BackingData>`.
fn resource_to_own<T: 'static, U: 'static>(
    r: wasmtime::component::Resource<T>,
) -> wasmtime::component::Resource<U> {
    wasmtime::component::Resource::new_own(r.rep())
}
