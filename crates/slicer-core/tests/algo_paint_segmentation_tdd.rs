#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::{execute_paint_segmentation, PaintSegmentationError};
use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectConfig, ObjectMesh, Point3,
    SemVer, SurfaceClassificationIR, Transform3d,
};

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: p3(0.0, 0.0, 0.0),
        max: p3(200.0, 200.0, 200.0),
    }
}

fn empty_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![],
        build_volume: build_volume(),
    }
}

fn empty_surface_classification() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        schema_version: sv(1, 0, 0),
        per_object: HashMap::new(),
    }
}

fn empty_layer_plan() -> LayerPlanIR {
    LayerPlanIR::default()
}

#[test]
fn empty_mesh_produces_empty_paint_region() {
    let mesh_ir = Arc::new(empty_mesh());
    let sc_ir = Arc::new(empty_surface_classification());
    let lp_ir = Arc::new(empty_layer_plan());

    let result = execute_paint_segmentation(mesh_ir, sc_ir, lp_ir, false);
    assert!(result.is_ok());

    let paint_ir = result.unwrap();
    assert!(paint_ir.per_layer.is_empty());
}

#[test]
fn missing_surface_object_returns_error() {
    let mesh_ir = Arc::new(MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "obj1".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0)],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: Some(slicer_ir::FacetPaintData {
                layers: vec![slicer_ir::PaintLayer {
                    semantic: slicer_ir::PaintSemantic::SupportEnforcer,
                    facet_values: vec![Some(slicer_ir::PaintValue::Flag(true))],
                    strokes: vec![],
                }],
            }),
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    });
    let sc_ir = Arc::new(empty_surface_classification());
    let lp_ir = Arc::new(LayerPlanIR {
        global_layers: vec![slicer_ir::GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![slicer_ir::ActiveRegion {
                object_id: "obj1".to_string(),
                region_id: 0,
                resolved_config: slicer_ir::ResolvedConfig::default(),
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: {
            let mut m = HashMap::new();
            m.insert(
                "obj1".to_string(),
                vec![slicer_ir::ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 0,
                    effective_layer_height: 0.2,
                }],
            );
            m
        },
        ..Default::default()
    });

    let result = execute_paint_segmentation(mesh_ir, sc_ir, lp_ir, false);
    assert!(result.is_err());

    match result.unwrap_err() {
        PaintSegmentationError::MissingSurfaceObject { object_id } => {
            assert_eq!(object_id, "obj1");
        }
        other => panic!("expected MissingSurfaceObject, got {other:?}"),
    }
}
