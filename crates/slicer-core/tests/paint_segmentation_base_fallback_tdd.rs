//! Regression: the synthesized BASE region must be scoped to its object.

#![cfg(feature = "host-algos")]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_ir::{
    ConfigDelta, ConfigValue, ExPolygon, IndexedTriangleSet, MeshIR, ModifierScope, ModifierVolume,
    ObjectMesh, PaintValue, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SemVer, SliceIR, SlicedRegion, Transform3d,
    CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
};

const IDENTITY: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn box_mesh(y0: f32, y1: f32) -> IndexedTriangleSet {
    let p = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            p(0.0, y0, 0.0),
            p(10.0, y0, 0.0),
            p(10.0, y1, 0.0),
            p(0.0, y1, 0.0),
            p(0.0, y0, 2.0),
            p(10.0, y0, 2.0),
            p(10.0, y1, 2.0),
            p(0.0, y1, 2.0),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ],
    }
}

fn square(y0: f32, y1: f32) -> ExPolygon {
    let p = |x: f32, y: f32| Point2 {
        x: slicer_ir::mm_to_units(x),
        y: slicer_ir::mm_to_units(y),
    };
    ExPolygon {
        contour: Polygon {
            points: vec![p(0.0, y0), p(10.0, y0), p(10.0, y1), p(0.0, y1)],
        },
        holes: Vec::new(),
    }
}

fn fixture() -> (Arc<MeshIR>, Arc<Vec<SliceIR>>, Arc<RegionMapIR>) {
    let mut fields = HashMap::new();
    fields.insert(
        "subtype".to_owned(),
        ConfigValue::String("support_enforcer".to_owned()),
    );
    let volume = ModifierVolume {
        id: "paint-trigger".to_owned(),
        mesh: box_mesh(2.0, 8.0),
        config_delta: ConfigDelta { fields },
        priority: 0,
        applies_to: ModifierScope::Support,
    };
    let mesh = Arc::new(MeshIR {
        objects: vec![
            ObjectMesh {
                id: "a".to_owned(),
                mesh: box_mesh(0.0, 10.0),
                transform: Transform3d { matrix: IDENTITY },
                modifier_volumes: vec![volume],
                ..Default::default()
            },
            ObjectMesh {
                id: "b".to_owned(),
                mesh: box_mesh(50.0, 60.0),
                transform: Transform3d { matrix: IDENTITY },
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    let slice = Arc::new(vec![SliceIR {
        schema_version: SemVer {
            major: 4,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.5,
        regions: vec![
            SlicedRegion {
                object_id: "a".to_owned(),
                region_id: 0,
                polygons: vec![square(0.0, 10.0)],
                ..Default::default()
            },
            SlicedRegion {
                object_id: "b".to_owned(),
                region_id: 0,
                polygons: vec![square(50.0, 60.0)],
                ..Default::default()
            },
        ],
    }]);
    let mut entries = HashMap::new();
    entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "a".to_owned(),
            region_id: 1,
            variant_chain: vec![("material".to_owned(), PaintValue::ToolIndex(1))],
        },
        RegionPlan::default(),
    );
    (
        mesh,
        slice,
        Arc::new(RegionMapIR {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries,
            configs: vec![ResolvedConfig::default()],
        }),
    )
}

#[test]
fn paint_base_fallback_uses_own_object_contours() {
    let (mesh, slice, region_map) = fixture();
    let output = execute_paint_segmentation(mesh, slice, region_map).expect("segmentation");
    let base = output[0]
        .regions
        .iter()
        .find(|r| r.variant_chain.is_empty())
        .expect("synthesized BASE");
    assert_eq!(base.object_id, "a");
    let ys: Vec<i64> = base
        .polygons
        .iter()
        .flat_map(|p| p.contour.points.iter().map(|point| point.y))
        .collect();
    assert!(!ys.is_empty(), "BASE must retain geometry");
    assert!(
        ys.iter().all(|y| *y <= slicer_ir::mm_to_units(10.0)),
        "BASE fallback leaked object b contours: {ys:?}"
    );
}
