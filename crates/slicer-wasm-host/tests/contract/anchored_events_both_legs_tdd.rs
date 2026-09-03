#![allow(missing_docs)]

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredEventRuntimeHooks, AnchoredGeometryContract,
    ExtrusionPath3D, ExtrusionRole, LayerStageCommit, OrderedEventCollection, Point3WithWidth,
};
use slicer_sdk::{
    builders::SupportOutputBuilder,
    layer_collection_builder::LayerCollectionBuilder,
    native::{NativeLayerResponse, NativeSupportOutput},
};
use slicer_wasm_host::dispatch::deconstruct_layer_ctx;
use slicer_wasm_host::host::HostExecutionContextBuilder;
use slicer_wasm_host::marshal::native::commit_native_layer_response;

fn collection(geometry: AnchoredGeometryContract, z: f32) -> OrderedEventCollection {
    OrderedEventCollection {
        anchor_global_layer_index: 7,
        // exhaustive: distinctive anchored-event contract fixture
        events: vec![AnchoredEntity {
            local_id: 19,
            anchor_global_layer_index: 7,
            geometry,
            input_capabilities: vec!["mesh".into()],
            output_capabilities: vec!["cooling".into()],
            provenance: AnchoredEntityProvenance {
                requesting_feature: "test-feature".into(),
                source_plan_entry: "plan-7".into(),
            },
            path_points: vec![Point3WithWidth {
                x: 1.0,
                y: 2.0,
                z,
                width: 0.45,
                flow_factor: 1.0,
                ..Default::default()
            }],
            role: ExtrusionRole::SupportMaterial,
        }],
        runtime_hooks: AnchoredEventRuntimeHooks {
            optimize_paths: false,
            account_cooling: true,
            account_time: false,
        },
    }
}

fn native_commit(input: OrderedEventCollection) -> LayerStageCommit {
    let mut builder = LayerCollectionBuilder::new();
    builder
        .set_anchored_event_collection(input)
        .expect("set anchored collection");
    // exhaustive: test-only response fixture names every stage slot explicitly
    let response = NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: Some(builder),
    };
    commit_native_layer_response(&response, "Layer::AnchoredEvents", 7, None)
        .expect("native anchored-events commit")
        .expect("anchored-events output")
}

#[test]
fn anchored_events_native_and_wasm_legs_agree() {
    let input = collection(AnchoredGeometryContract::Planar { z: 3000 }, 0.3);
    let mut ctx = HostExecutionContextBuilder::new("anchored-events-contract", 0.3, 0.2).build();
    ctx.anchored_events_mut().collection = Some(input.clone());
    let wasm = deconstruct_layer_ctx(
        "Layer::AnchoredEvents",
        "anchored-events-contract",
        7,
        None,
        ctx,
        None,
    )
    .expect("wasm anchored-events commit")
    .expect("wasm anchored-events output");
    assert_eq!(wasm, native_commit(input));
}

#[test]
fn native_support_postprocess_preserves_geometry_and_anchored_events() {
    let input = collection(AnchoredGeometryContract::Planar { z: 3000 }, 0.3);
    let mut output = SupportOutputBuilder::new();
    output
        .push_support_path(ExtrusionPath3D { // exhaustive: fixture pins every field
            // exhaustive: fixture pins every field
            points: vec![Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.3,
                width: 0.45,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
                overhang_distance_mm: None,
            }],
            role: ExtrusionRole::SupportMaterial,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
        })
        .expect("push support path");
    let mut builder = LayerCollectionBuilder::new();
    builder
        .set_anchored_event_collection(input.clone())
        .expect("set anchored collection");
    // Pins the whole NativeLayerResponse shape; the support arm must carry
    // BOTH payloads (ordinary output + anchored collection), per DEV-162.
    // exhaustive: arms intentional; both-payload commit pinned.
    let response = NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: Some(NativeSupportOutput {
            output,
            collection: builder,
        }),
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    };

    let commit = commit_native_layer_response(&response, "Layer::SupportPostProcess", 7, None)
        .expect("native support-postprocess commit")
        .expect("support-postprocess output");
    assert!(matches!(
        commit,
        LayerStageCommit::SupportPostProcessWithAnchoredEvents {
            anchored_events,
            ..
        } if anchored_events == vec![input]
    ));
}

#[test]
fn anchored_events_planar_geometry_is_rejected() {
    let input = collection(AnchoredGeometryContract::Planar { z: 3000 }, 0.9);
    let mut ctx = HostExecutionContextBuilder::new("anchored-events-contract", 0.3, 0.2).build();
    ctx.anchored_events_mut().collection = Some(input);
    let error = deconstruct_layer_ctx(
        "Layer::AnchoredEvents",
        "anchored-events-contract",
        7,
        None,
        ctx,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(error, slicer_ir::LayerStageError::FatalModule { ref message, .. } if message.contains("anchored entity planar z mismatch"))
    );
}

#[test]
fn anchored_events_z_spanning_geometry_is_rejected() {
    let input = collection(
        AnchoredGeometryContract::ZSpanning {
            min_z: 3000,
            max_z: 5000,
        },
        0.9,
    );
    let mut ctx = HostExecutionContextBuilder::new("anchored-events-contract", 0.3, 0.2).build();
    ctx.anchored_events_mut().collection = Some(input);
    let error = deconstruct_layer_ctx(
        "Layer::AnchoredEvents",
        "anchored-events-contract",
        7,
        None,
        ctx,
        None,
    )
    .unwrap_err();
    assert!(
        matches!(error, slicer_ir::LayerStageError::FatalModule { ref message, .. } if message.contains("anchored entity z-span violation"))
    );
}
