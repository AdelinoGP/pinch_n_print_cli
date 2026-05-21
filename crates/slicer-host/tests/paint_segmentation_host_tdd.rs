#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{execute_paint_segmentation, PaintSegmentationError};
use slicer_ir::{
    ActiveRegion, BoundingBox3, FacetClass, FacetPaintData, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectConfig, ObjectLayerRef, ObjectMesh, ObjectSurfaceData, PaintLayer,
    PaintRegionIR, PaintSemantic, PaintValue, Point3, ResolvedConfig, SemVer,
    SurfaceClassificationIR, Transform3d,
};

// ── Fixture helpers ─────────────────────────────────────────────────────────

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn translation_transform(tx: f64, ty: f64) -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, 0.0, 1.0,
        ],
    }
}

fn point3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn single_triangle_mesh() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            point3(0.0, 0.0, 0.0),
            point3(10.0, 0.0, 0.0),
            point3(0.0, 10.0, 0.2),
        ],
        indices: vec![0, 1, 2],
    }
}

fn two_triangle_mesh() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            point3(0.0, 0.0, 0.0),
            point3(10.0, 0.0, 0.0),
            point3(0.0, 10.0, 0.2),
            point3(10.0, 10.0, 0.2),
        ],
        indices: vec![0, 1, 2, 1, 3, 2],
    }
}

fn schema_version() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn mesh_fixture(objects: Vec<ObjectMesh>) -> MeshIR {
    MeshIR {
        schema_version: schema_version(),
        objects,
        build_volume: BoundingBox3 {
            min: point3(0.0, 0.0, 0.0),
            max: point3(200.0, 200.0, 200.0),
        },
    }
}

fn surface_fixture(objects: &[(&str, usize)]) -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        schema_version: schema_version(),
        per_object: objects
            .iter()
            .map(|(id, count)| {
                (
                    (*id).to_owned(),
                    ObjectSurfaceData {
                        facet_classes: vec![FacetClass::Normal; *count],
                        surface_groups: Vec::new(),
                        bridge_regions: Vec::new(),
                        overhang_regions: Vec::new(),
                    },
                )
            })
            .collect(),
    }
}

fn layer_fixture(global_layers: &[u32], object_participation: &[(&str, &[u32])]) -> LayerPlanIR {
    LayerPlanIR {
        schema_version: schema_version(),
        global_layers: global_layers
            .iter()
            .copied()
            .map(|idx| GlobalLayer {
                index: idx,
                z: 0.2 * (idx as f32 + 1.0),
                active_regions: vec![ActiveRegion {
                    object_id: String::from("placeholder"),
                    region_id: 0,
                    resolved_config: ResolvedConfig::default(),
                    effective_layer_height: 0.2,
                    nonplanar_shell: None,
                    is_catchup_layer: false,
                    catchup_z_bottom: 0.0,
                    tool_index: 0,
                }],
                has_nonplanar: false,
                is_sync_layer: false,
            })
            .collect(),
        object_participation: object_participation
            .iter()
            .map(|(id, layers)| {
                (
                    (*id).to_owned(),
                    layers
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(local_idx, global_idx)| ObjectLayerRef {
                            local_layer_index: local_idx as u32,
                            global_layer_index: global_idx,
                            effective_layer_height: 0.2,
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}

fn single_object_mesh(
    id: &str,
    mesh: IndexedTriangleSet,
    transform: Transform3d,
    layers: Vec<PaintLayer>,
) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh,
        transform,
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData { layers }),
        world_z_extent: None,
    }
}

fn run_segmentation(
    id: &str,
    mesh: IndexedTriangleSet,
    transform: Transform3d,
    layers: Vec<PaintLayer>,
    global_layer_indices: &[u32],
    participation: &[u32],
) -> Result<Arc<PaintRegionIR>, PaintSegmentationError> {
    let obj = single_object_mesh(id, mesh, transform, layers);
    let mesh_ir = Arc::new(mesh_fixture(vec![obj]));
    let surface = Arc::new(surface_fixture(&[(
        id,
        mesh_ir.objects[0].mesh.indices.len() / 3,
    )]));
    let layer_plan = Arc::new(layer_fixture(global_layer_indices, &[(id, participation)]));
    execute_paint_segmentation(mesh_ir, surface, layer_plan, true)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn execute_segmentation_defaults() {
    let result = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        identity_transform(),
        vec![PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: vec![Some(PaintValue::ToolIndex(1))],
            strokes: Vec::new(),
        }],
        &[0],
        &[0],
    );
    assert!(
        result.is_ok(),
        "execute_paint_segmentation should succeed with valid inputs"
    );
}

#[test]
fn no_paint_data_produces_empty_regions() {
    let obj = ObjectMesh {
        id: String::from("obj1"),
        mesh: single_triangle_mesh(),
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    };
    let mesh_ir = Arc::new(mesh_fixture(vec![obj]));
    let surface = Arc::new(surface_fixture(&[("obj1", 1)]));
    let layer_plan = Arc::new(layer_fixture(&[0], &[("obj1", &[0])]));

    let paint_regions = execute_paint_segmentation(mesh_ir, surface, layer_plan, true)
        .expect("segmentation should succeed with no paint data");
    assert!(
        paint_regions.per_layer[&0].semantic_regions.is_empty(),
        "no regions should be emitted when there are no paint layers"
    );
}

#[test]
fn single_facet_single_layer_region() {
    let paint_regions = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        identity_transform(),
        vec![PaintLayer {
            semantic: PaintSemantic::SupportEnforcer,
            facet_values: vec![Some(PaintValue::Flag(true))],
            strokes: Vec::new(),
        }],
        &[0],
        &[0],
    )
    .expect("single facet segmentation should succeed");

    let regions = paint_regions.get(0, &PaintSemantic::SupportEnforcer);
    assert_eq!(regions.len(), 1, "should produce exactly 1 region");
    let region = &regions[0];
    assert_eq!(region.object_id, "obj1");
    assert_eq!(region.value, PaintValue::Flag(true));
    assert_eq!(region.paint_order, 0);
    assert_eq!(
        region.polygons[0].contour.points.len(),
        3,
        "triangle has 3 vertices"
    );
}

#[test]
fn paint_region_polygons_reflect_transform() {
    let paint_layers = vec![PaintLayer {
        semantic: PaintSemantic::Material,
        facet_values: vec![Some(PaintValue::Flag(true))],
        strokes: Vec::new(),
    }];

    let identity_result = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        identity_transform(),
        paint_layers.clone(),
        &[0],
        &[0],
    )
    .expect("identity transform should succeed");

    let translated_result = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        translation_transform(10.0, 20.0),
        paint_layers,
        &[0],
        &[0],
    )
    .expect("translated transform should succeed");

    let id_pts = &identity_result.per_layer[&0].semantic_regions[&PaintSemantic::Material][0]
        .polygons[0]
        .contour
        .points;
    let tr_pts = &translated_result.per_layer[&0].semantic_regions[&PaintSemantic::Material][0]
        .polygons[0]
        .contour
        .points;

    for i in 0..3 {
        let dx = tr_pts[i].x - id_pts[i].x;
        let dy = tr_pts[i].y - id_pts[i].y;
        assert!(
            (dx - 100000).abs() < 2,
            "X offset should be 100000 units (10 mm), got {dx}"
        );
        assert!(
            (dy - 200000).abs() < 2,
            "Y offset should be 200000 units (20 mm), got {dy}"
        );
    }
}

#[test]
fn multiple_semantics_produce_multiple_region_entries() {
    let paint_regions = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        identity_transform(),
        vec![
            PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::Flag(true))],
                strokes: Vec::new(),
            },
            PaintLayer {
                semantic: PaintSemantic::FuzzySkin,
                facet_values: vec![Some(PaintValue::Scalar(0.5))],
                strokes: Vec::new(),
            },
        ],
        &[0],
        &[0],
    )
    .expect("multiple semantics should succeed");

    let material_regions = paint_regions.get(0, &PaintSemantic::Material);
    let fuzzy_regions = paint_regions.get(0, &PaintSemantic::FuzzySkin);
    assert_eq!(
        material_regions.len(),
        1,
        "should produce 1 material region"
    );
    assert_eq!(fuzzy_regions.len(), 1, "should produce 1 fuzzy_skin region");
}

#[test]
fn paint_order_preserved_across_layers() {
    let paint_regions = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        identity_transform(),
        vec![
            PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::Flag(true))],
                strokes: Vec::new(),
            },
            PaintLayer {
                semantic: PaintSemantic::FuzzySkin,
                facet_values: vec![Some(PaintValue::ToolIndex(1))],
                strokes: Vec::new(),
            },
        ],
        &[0],
        &[0],
    )
    .expect("paint order segmentation should succeed");

    let material = paint_regions.get(0, &PaintSemantic::Material);
    let fuzzy = paint_regions.get(0, &PaintSemantic::FuzzySkin);
    assert_eq!(material.len(), 1);
    assert_eq!(fuzzy.len(), 1);
    assert_eq!(material[0].paint_order, 0);
    assert_eq!(fuzzy[0].paint_order, 1);
}

#[test]
fn same_value_grouped_under_identical_key() {
    let paint_regions = run_segmentation(
        "obj1",
        two_triangle_mesh(),
        identity_transform(),
        vec![PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: vec![Some(PaintValue::Flag(true)), Some(PaintValue::Flag(true))],
            strokes: Vec::new(),
        }],
        &[0],
        &[0],
    )
    .expect("grouped segmentation should succeed");

    let regions = paint_regions.get(0, &PaintSemantic::Material);
    let region = &regions[0];
    assert_eq!(region.object_id, "obj1");
    assert_eq!(region.value, PaintValue::Flag(true));
    assert_eq!(region.paint_order, 0);
    assert_eq!(
        region.polygons.len(),
        1,
        "two triangles with same key should be unioned into 1 polygon"
    );
}

#[test]
fn empty_objects_produce_empty_paint_regions() {
    let mesh_ir = Arc::new(MeshIR {
        schema_version: schema_version(),
        objects: vec![],
        build_volume: BoundingBox3::default(),
    });
    let surface = Arc::new(SurfaceClassificationIR {
        schema_version: schema_version(),
        per_object: HashMap::new(),
    });
    let layer_plan = Arc::new(LayerPlanIR {
        schema_version: schema_version(),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: HashMap::new(),
    });

    let paint_regions = execute_paint_segmentation(mesh_ir, surface, layer_plan, true)
        .expect("empty objects should succeed");
    assert!(
        paint_regions.per_layer[&0].semantic_regions.is_empty(),
        "no regions should be emitted when there are no objects"
    );
}

#[test]
fn paint_applied_to_each_participating_layer() {
    let paint_regions = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        identity_transform(),
        vec![PaintLayer {
            semantic: PaintSemantic::SupportEnforcer,
            facet_values: vec![Some(PaintValue::Flag(true))],
            strokes: Vec::new(),
        }],
        &[0, 1, 2],
        &[0, 1, 2],
    )
    .expect("multi-layer participation should succeed");

    assert_eq!(
        paint_regions.get(0, &PaintSemantic::SupportEnforcer).len(),
        1,
        "should produce 1 region at layer 0"
    );
    assert_eq!(
        paint_regions.get(1, &PaintSemantic::SupportEnforcer).len(),
        1,
        "should produce 1 region at layer 1"
    );
    assert_eq!(
        paint_regions.get(2, &PaintSemantic::SupportEnforcer).len(),
        1,
        "should produce 1 region at layer 2"
    );
}

#[test]
fn unpainted_facets_do_not_produce_regions() {
    let paint_regions = run_segmentation(
        "obj1",
        two_triangle_mesh(),
        identity_transform(),
        vec![PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: vec![Some(PaintValue::Flag(true)), None],
            strokes: Vec::new(),
        }],
        &[0],
        &[0],
    )
    .expect("unpainted facets segmentation should succeed");

    let regions = paint_regions.get(0, &PaintSemantic::Material);
    assert_eq!(
        regions.len(),
        1,
        "only the painted facet should produce a region"
    );
}

#[test]
fn malformed_facet_values_returns_error() {
    let result = run_segmentation(
        "obj1",
        single_triangle_mesh(),
        identity_transform(),
        vec![PaintLayer {
            semantic: PaintSemantic::Material,
            facet_values: vec![Some(PaintValue::Flag(true)), Some(PaintValue::Flag(false))],
            strokes: Vec::new(),
        }],
        &[0],
        &[0],
    );

    assert!(
        result.is_err(),
        "should return error on facet_values length mismatch"
    );
    match result {
        Err(PaintSegmentationError::MalformedFacetValues { .. }) => {}
        _ => panic!("expected MalformedFacetValues error"),
    }
}
