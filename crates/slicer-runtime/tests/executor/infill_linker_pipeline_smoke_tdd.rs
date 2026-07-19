#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion, MeshIR,
    PerimeterIR, PerimeterRegion, Point2, Point3WithWidth, Polygon, SemVer, SliceIR, SlicedRegion,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{Blackboard, CompiledModuleBuilder, LayerArena, WasmRuntimeDispatcher};

use crate::common::{wasm_cache, TestModuleBundle};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn sparse_segment(x_start: f32, x_end: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: x_start,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            Point3WithWidth {
                x: x_end,
                y: 5.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn infill_ir() -> InfillIR {
    InfillIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "object-a".to_string(),
            region_id: 7,
            sparse_infill: vec![
                sparse_segment(1.0, 3.0),
                sparse_segment(3.0, 5.0),
                sparse_segment(5.0, 7.0),
            ],
            solid_infill: Vec::new(),
            ironing: Vec::new(),
        }],
    }
}

fn slice_ir() -> SliceIR {
    SliceIR {
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "object-a".to_string(),
            region_id: 7,
            polygons: vec![square()],
            infill_areas: vec![square()],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            sparse_infill_area: vec![square()],
        }],
        ..Default::default()
    }
}

fn perimeter_ir() -> PerimeterIR {
    PerimeterIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "object-a".to_string(),
            region_id: 7,
            walls: Vec::new(),
            infill_areas: vec![square()],
            seam_candidates: Vec::new(),
            resolved_seam: None,
        }],
    }
}

fn linker_bundle() -> TestModuleBundle {
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/infill-linker/infill-linker.wasm");
    assert!(
        wasm_path.exists(),
        "infill-linker.wasm not found at {}. Build it with: `cargo xtask build-guests`",
        wasm_path.display()
    );

    let loaded = LoadedModuleBuilder::new(
        "com.core.infill-linker",
        semver(0, 1, 0),
        "Layer::InfillPostProcess",
        slicer_schema::WORLD_LAYER,
        wasm_path.clone(),
    )
    .ir_reads(vec!["SliceIR".to_string(), "InfillIR".to_string()])
    .ir_writes(vec!["InfillIR".to_string()])
    .claims(vec!["claim:infill-link".to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(3, 0, 0))
    .max_ir_schema(semver(5, 0, 0))
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
    let config = slicer_ir::ConfigView::from_map(HashMap::from([
        ("infill_overlap".to_string(), ConfigValue::Float(0.45)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
        ("infill_density".to_string(), ConfigValue::Float(0.2)),
    ]));
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(config))
        .build();

    TestModuleBundle {
        module,
        pool,
        component: Some(wasm_cache::compiled_component_at(&wasm_path)),
    }
}

#[test]
fn infill_linker_pipeline_smoke() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let layer = slicer_ir::GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut arena = LayerArena::new();
    arena.set_slice(slice_ir()).expect("SliceIR must stage");
    arena
        .set_perimeter(perimeter_ir())
        .expect("PerimeterIR must stage");
    arena
        .set_infill(infill_ir())
        .expect("raw InfillIR must stage");

    crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::InfillPostProcess",
        &layer,
        &linker_bundle(),
        &blackboard,
        &mut arena,
    )
    .expect("Layer::InfillPostProcess dispatch+commit must succeed");

    let committed = arena.infill().expect("InfillIR must be committed");
    let region = committed
        .regions
        .iter()
        .find(|region| region.object_id == "object-a" && region.region_id == 7)
        .expect("linker must preserve the input region");
    assert!(
        region
            .sparse_infill
            .iter()
            .any(|path| path.points.len() > 2),
        "linker must join raw 2-point sparse segments into a polyline"
    );
}
