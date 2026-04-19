//! TDD tests for WIT/component-model data marshaling boundary.
//!
//! These tests prove that real data crosses the host<->guest component boundary:
//! - Config values are readable by the guest
//! - IR view data (slice regions) is readable by the guest
//! - Output builder writes arrive at the host
//! - Host services (logging) work
//! - Pool correctness with real data-bound calls

use std::collections::HashMap;

use slicer_host::wit_host::{
    ConfigValueStorage, ConfigViewData, HostExecutionContext, LayerModule, SliceRegionData,
};

/// Path to the pre-built test guest component.
const GUEST_COMPONENT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-guests/layer-infill-guest.component.wasm");

/// Load the test guest component bytes, or skip the test if not built.
fn load_guest_component() -> Vec<u8> {
    std::fs::read(GUEST_COMPONENT_PATH).unwrap_or_else(|e| {
        panic!(
            "Test guest component not found at {}: {}. \
             Build it with: cd test-guests/layer-infill-guest && \
             cargo build --target wasm32-unknown-unknown --release && \
             wasm-tools component new target/wasm32-unknown-unknown/release/layer_infill_guest.wasm \
             -o ../../test-guests/layer-infill-guest.component.wasm",
            GUEST_COMPONENT_PATH, e
        )
    })
}

/// Create a wasmtime engine with component model enabled.
fn make_engine() -> wasmtime::Engine {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    wasmtime::Engine::new(&config).unwrap()
}

// ── A: Config access across the boundary ────────────────────────────────

/// The guest reads `infill-spacing` from the config-view and uses its value
/// to determine the X extent of the emitted sparse path. We provide spacing=3.5
/// and verify the output path has x = 3.5 * 10 = 35.0 for the second point.
#[test]
fn guest_reads_config_value_and_uses_it_in_output() {
    let wasm_bytes = load_guest_component();
    let engine = make_engine();
    let component = wasmtime::component::Component::new(&engine, &wasm_bytes)
        .expect("compile component");

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
        .expect("add_to_linker");

    let mut ctx = HostExecutionContext::new("test-infill-module".into(), 0.0, 0.0, None);

    // Provide config with infill-spacing = 3.5
    let mut fields = HashMap::new();
    fields.insert("infill-spacing".into(), ConfigValueStorage::Float(3.5));
    let config_handle = ctx.push_config_view(ConfigViewData { fields }).unwrap();

    // Provide one slice region at z=1.0
    let region_handle = ctx.push_slice_region(SliceRegionData {
        object_id: "obj-1".into(),
        region_id: "reg-1".into(),
        polygons: vec![],
        infill_areas: vec![],
        effective_layer_height: 0.2,
        z: 1.0,
        has_nonplanar: false,
        boundary_paint: vec![],
    }).unwrap();

    // Provide infill output builder
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker)
        .expect("instantiate");

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

    // Verify config was read: spacing=3.5 → second point x = 3.5*10 = 35.0
    assert_eq!(ctx.infill_output.sparse_paths.len(), 1, "expected 1 sparse path");
    let path = &ctx.infill_output.sparse_paths[0];
    assert_eq!(path.points.len(), 2, "expected 2 points");
    assert!((path.points[1].x - 35.0).abs() < 0.001,
        "second point x should be 35.0 (spacing*10), got {}", path.points[1].x);

    // Verify z was read from region data
    assert!((path.points[0].z - 1.0).abs() < 0.001,
        "point z should be 1.0 (from region), got {}", path.points[0].z);
}

// ── B: IR/read-view access across the boundary ──────────────────────────

/// The guest reads z from the slice-region-view and uses it in the output path.
/// We provide z=5.5 and verify the output path has z=5.5.
#[test]
fn guest_reads_region_z_from_ir_view() {
    let wasm_bytes = load_guest_component();
    let engine = make_engine();
    let component = wasmtime::component::Component::new(&engine, &wasm_bytes).unwrap();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx).unwrap();

    let mut ctx = HostExecutionContext::new("test-ir-read".into(), 0.0, 0.0, None);

    let config_handle = ctx.push_config_view(ConfigViewData {
        fields: HashMap::new(),
    }).unwrap();
    let region_handle = ctx.push_slice_region(SliceRegionData {
        object_id: "obj-z-test".into(),
        region_id: "reg-z-test".into(),
        polygons: vec![],
        infill_areas: vec![],
        effective_layer_height: 0.3,
        z: 5.5, // distinctive z value
        has_nonplanar: false,
        boundary_paint: vec![],
    }).unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    bindings.call_run_infill(
        &mut store, 42, &[resource_to_own(region_handle)],
        resource_to_own(output_handle), resource_to_own(config_handle),
    ).unwrap().unwrap();

    let ctx = store.into_data();
    assert_eq!(ctx.infill_output.sparse_paths.len(), 1);
    let z = ctx.infill_output.sparse_paths[0].points[0].z;
    assert!((z - 5.5).abs() < 0.001, "z should be 5.5, got {z}");
}

// ── C: Output emission across the boundary ──────────────────────────────

/// The guest pushes a sparse path via the infill-output-builder.
/// Verify the host received the path with correct structure.
#[test]
fn guest_emits_output_via_infill_builder() {
    let wasm_bytes = load_guest_component();
    let engine = make_engine();
    let component = wasmtime::component::Component::new(&engine, &wasm_bytes).unwrap();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx).unwrap();

    let mut ctx = HostExecutionContext::new("test-output".into(), 0.0, 0.0, None);
    let config_handle = ctx.push_config_view(ConfigViewData {
        fields: HashMap::new(),
    }).unwrap();
    let region_handle = ctx.push_slice_region(SliceRegionData {
        object_id: "obj-out".into(),
        region_id: "reg-out".into(),
        polygons: vec![],
        infill_areas: vec![],
        effective_layer_height: 0.2,
        z: 2.0,
        has_nonplanar: false,
        boundary_paint: vec![],
    }).unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    bindings.call_run_infill(
        &mut store, 0, &[resource_to_own(region_handle)],
        resource_to_own(output_handle), resource_to_own(config_handle),
    ).unwrap().unwrap();

    let ctx = store.into_data();
    // Guest must have pushed exactly one sparse path
    assert_eq!(ctx.infill_output.sparse_paths.len(), 1);
    let path = &ctx.infill_output.sparse_paths[0];
    // Path must have 2 points
    assert_eq!(path.points.len(), 2);
    // Role must be sparse-infill
    assert!(matches!(
        path.role,
        slicer_host::wit_host::ExtrusionRole::SparseInfill
    ));
    // Guest encodes region count in flow_factor and polygon count in width.
    // 1 region passed with 0 polygons:
    assert_eq!(path.points[0].flow_factor, 1.0, "1 region passed");
    assert_eq!(path.points[0].width, 0.0, "0 polygons in the region");
    // Second point has standard width from guest code
    assert!((path.points[1].width - 0.4).abs() < 0.001);
}

// ── D: Host services (logging) across the boundary ──────────────────────

/// The guest calls host-services.log. Verify the host received the log message.
#[test]
fn guest_logs_via_host_services() {
    let wasm_bytes = load_guest_component();
    let engine = make_engine();
    let component = wasmtime::component::Component::new(&engine, &wasm_bytes).unwrap();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx).unwrap();

    let mut ctx = HostExecutionContext::new("test-log".into(), 0.0, 0.0, None);
    let config_handle = ctx.push_config_view(ConfigViewData {
        fields: HashMap::new(),
    }).unwrap();
    let region_handle = ctx.push_slice_region(SliceRegionData {
        object_id: "obj-log".into(),
        region_id: "reg-log".into(),
        polygons: vec![],
        infill_areas: vec![],
        effective_layer_height: 0.2,
        z: 0.2,
        has_nonplanar: false,
        boundary_paint: vec![],
    }).unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    bindings.call_run_infill(
        &mut store, 7, &[resource_to_own(region_handle)],
        resource_to_own(output_handle), resource_to_own(config_handle),
    ).unwrap().unwrap();

    let ctx = store.into_data();
    // Guest logs "run-infill: layer=7, ..."
    assert!(!ctx.log_messages.is_empty(), "expected at least one log message");
    let (level, msg) = &ctx.log_messages[0];
    assert_eq!(level, "info");
    assert!(msg.contains("layer=7"), "log should contain layer=7, got: {msg}");
    assert!(msg.contains("regions=1"), "log should mention 1 region, got: {msg}");
}

// ── E: Repeated calls with fresh context (pool correctness) ─────────────

/// Multiple calls to the same component with fresh contexts produce
/// independent outputs — no cross-call data contamination.
#[test]
fn repeated_calls_produce_independent_outputs() {
    let wasm_bytes = load_guest_component();
    let engine = make_engine();
    let component = wasmtime::component::Component::new(&engine, &wasm_bytes).unwrap();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx).unwrap();

    for i in 0..3 {
        let mut ctx = HostExecutionContext::new(format!("call-{i}"), 0.0, 0.0, None);
        let z = (i + 1) as f32 * 10.0;
        let config_handle = ctx.push_config_view(ConfigViewData {
            fields: HashMap::new(),
        }).unwrap();
        let region_handle = ctx.push_slice_region(SliceRegionData {
            object_id: format!("obj-{i}"),
            region_id: format!("reg-{i}"),
            polygons: vec![],
            infill_areas: vec![],
            effective_layer_height: 0.2,
            z,
            has_nonplanar: false,
            boundary_paint: vec![],
        }).unwrap();
        let output_handle = ctx.push_infill_output_builder().unwrap();

        let mut store = wasmtime::Store::new(&engine, ctx);
        let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

        bindings.call_run_infill(
            &mut store, i as u32, &[resource_to_own(region_handle)],
            resource_to_own(output_handle), resource_to_own(config_handle),
        ).unwrap().unwrap();

        let ctx = store.into_data();
        // Each call should have exactly one path
        assert_eq!(ctx.infill_output.sparse_paths.len(), 1,
            "call {i}: expected 1 path");
        // Each call should have the z from its own region
        let actual_z = ctx.infill_output.sparse_paths[0].points[0].z;
        assert!((actual_z - z).abs() < 0.001,
            "call {i}: z should be {z}, got {actual_z}");
        // No log messages from previous calls
        assert_eq!(ctx.log_messages.len(), 1,
            "call {i}: expected 1 log message, got {}", ctx.log_messages.len());
    }
}

// ── F: Empty region list handled correctly ──────────────────────────────

/// When no regions are provided, the guest still runs and produces output
/// with z=0.0 (the fallback).
#[test]
fn empty_region_list_handled_gracefully() {
    let wasm_bytes = load_guest_component();
    let engine = make_engine();
    let component = wasmtime::component::Component::new(&engine, &wasm_bytes).unwrap();

    let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(&engine);
    LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx).unwrap();

    let mut ctx = HostExecutionContext::new("test-empty".into(), 0.0, 0.0, None);
    let config_handle = ctx.push_config_view(ConfigViewData {
        fields: HashMap::new(),
    }).unwrap();
    let output_handle = ctx.push_infill_output_builder().unwrap();

    let mut store = wasmtime::Store::new(&engine, ctx);
    let bindings = LayerModule::instantiate(&mut store, &component, &linker).unwrap();

    // Call with empty regions list
    bindings.call_run_infill(
        &mut store, 0, &[], // no regions
        resource_to_own(output_handle), resource_to_own(config_handle),
    ).unwrap().unwrap();

    let ctx = store.into_data();
    assert_eq!(ctx.infill_output.sparse_paths.len(), 1);
    // z should be 0.0 (fallback when no regions)
    let z = ctx.infill_output.sparse_paths[0].points[0].z;
    assert!((z - 0.0).abs() < 0.001, "z should be 0.0, got {z}");
}

// ── Helper: convert Resource to the right type ──────────────────────────

/// Convert a `Resource<T>` to `Resource<U>` by preserving the rep.
/// This is needed because the call methods expect `Resource<WitType>`
/// but our push methods return `Resource<BackingData>`.
fn resource_to_own<T: 'static, U: 'static>(r: wasmtime::component::Resource<T>) -> wasmtime::component::Resource<U> {
    wasmtime::component::Resource::new_own(r.rep())
}
