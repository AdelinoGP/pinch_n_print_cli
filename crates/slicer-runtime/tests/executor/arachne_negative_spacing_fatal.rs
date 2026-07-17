//! D-162 e2e: a config whose wall line width is too small for its layer
//! height must FAIL the slice with the negative-spacing error — not silently
//! feed a raw width into the spacing-domain beading stack (the pre-D-162
//! fallback) or produce over-wide beads.
//!
//! Canonical `Flow::rounded_rectangle_extrusion_spacing` throws
//! `FlowErrorNegativeSpacing` iff `width - height * (1 - PI/4) <= 0`, and
//! nothing on the slicing path catches it. The chosen config sits inside the
//! manifest-valid ranges (width >= 0.1, layer_height <= 1.0) but below the
//! spacing threshold: `0.1 - 1.0 * (1 - PI/4) = -0.1146`.

#![allow(missing_docs)]

use slicer_ir::{mm_to_units, ExPolygon, Point2, Polygon, SemVer, SliceIR, SlicedRegion};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{Blackboard, CompiledModuleBuilder, LayerArena, WasmRuntimeDispatcher};
use std::sync::Arc;

use crate::common::wasm_cache;
use crate::common::TestModuleBundle;

#[test]
fn arachne_negative_spacing_config_fails_the_slice_with_actionable_error() {
    let layer_index = 0u32;
    let layer_z = 1.0_f32;
    let object_id = "obj-a";
    let region_id: u64 = 0;

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates
        .parent()
        .unwrap() // pinch_n_print
        .join("modules/core-modules/arachne-perimeters/arachne-perimeters.wasm");
    assert!(
        wasm_path.exists(),
        "arachne-perimeters.wasm not found at {}. Build it with: `cargo xtask build-guests`",
        wasm_path.display()
    );
    let component = wasm_cache::compiled_component_at(&wasm_path);

    let loaded = LoadedModuleBuilder::new(
        "com.core.arachne-perimeters",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Perimeters",
        "slicer:world-layer@1.0.0",
        wasm_path.clone(),
    )
    .ir_reads(vec!["SliceIR".to_string(), "PaintRegionIR".to_string()])
    .ir_writes(vec!["PerimeterIR".to_string()])
    .claims(vec!["perimeter-generator".to_string()])
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();

    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );

    // Manifest-valid but spacing-negative: 0.1mm walls at 1.0mm layer height.
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "layer_height".to_string(),
        slicer_ir::ConfigValue::Float(1.0),
    );
    fields.insert(
        "inner_wall_line_width".to_string(),
        slicer_ir::ConfigValue::Float(0.1),
    );
    fields.insert(
        "outer_wall_line_width".to_string(),
        slicer_ir::ConfigValue::Float(0.1),
    );

    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(fields)))
        .build();

    let bundle = TestModuleBundle {
        module,
        pool,
        component: Some(component),
    };

    let side = mm_to_units(10.0);
    let mut arena = LayerArena::new();
    arena
        .set_slice(SliceIR {
            global_layer_index: layer_index,
            z: layer_z,
            regions: vec![SlicedRegion {
                object_id: object_id.to_string(),
                region_id,
                polygons: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: side, y: 0 },
                            Point2 { x: side, y: side },
                            Point2 { x: 0, y: side },
                        ],
                    },
                    holes: Vec::new(),
                }],
                infill_areas: Vec::new(),
                nonplanar_surface: None,
                effective_layer_height: 1.0,
                segment_annotations: std::collections::HashMap::new(),
                variant_chain: Vec::new(),
                top_shell_index: None,
                bottom_shell_index: None,
                top_solid_fill: Vec::new(),
                bottom_solid_fill: Vec::new(),
                is_bridge: false,
                bridge_areas: vec![],
                bridge_orientation_deg: 0.0,
                sparse_infill_area: Vec::new(),
            }],
            ..Default::default()
        })
        .expect("set_slice must succeed");

    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);

    let layer = slicer_ir::GlobalLayer {
        index: layer_index,
        z: layer_z,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let result = crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::Perimeters",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    );

    let err = result.expect_err(
        "a 0.1mm wall line width at 1.0mm layer height has negative flow spacing \
         (0.1 - 1.0*(1 - pi/4) = -0.1146mm) and must abort the stage — a silent \
         success means the pre-D-162 raw-width fallback is back",
    );
    let msg = format!("{err:?}");
    assert!(
        msg.contains("too small for layer height"),
        "stage error must carry the NegativeSpacingError message naming the \
         config fix, got: {msg}"
    );
}
