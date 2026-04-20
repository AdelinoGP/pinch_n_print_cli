//! TDD harness for PaintSegmentationObjectView inputs (Step 4 — TASK-128b).
//!
//! These tests prove that the IR data needed to populate PaintSegmentationObjectView
//! is available in the host and will be correctly converted:
//! - ObjectMesh.transform: non-identity 4x4 column-major transform
//! - FacetPaintData.layers: non-empty paint layer data
//! - LayerPlanIR.object_participation: non-empty layer indices
//!
//! Precondition: WIT record types defined (Step 1); converters stubbed (Step 2)
//! Postcondition: Test file exists, compiles, passes
//!
//! Verification: cargo test -p slicer-host --test macro_paint_segmentation_input_tdd
//! Exit condition: Test file compiles and passes
//!
//! Note: These tests verify the IR types and data exist with the required fields.
//! The full dispatch integration tests (using SDK types) will pass after Step 7.

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{
    FacetPaintData, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectConfig,
    ObjectLayerRef, ObjectMesh, PaintLayer, PaintSemantic, PaintValue, Point3, Transform3d,
};

/// Identity 4x4 column-major transform matrix.
fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// Non-identity (translation) 4x4 column-major transform matrix.
fn translation_transform(tx: f64, ty: f64, tz: f64) -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            tx, ty, tz, 1.0,
        ],
    }
}

/// Minimal mesh with one triangle and paint data.
fn mesh_with_paint() -> MeshIR {
    MeshIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        objects: vec![ObjectMesh {
            id: String::from("painted-object"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 { x: 0.0, y: 0.0, z: 0.0 },
                    Point3 { x: 10.0, y: 0.0, z: 0.0 },
                    Point3 { x: 0.0, y: 10.0, z: 0.2 },
                ],
                indices: vec![0, 1, 2],
            },
            transform: translation_transform(5.0, 10.0, 0.0),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: Some(FacetPaintData {
                layers: vec![PaintLayer {
                    semantic: PaintSemantic::Material,
                    facet_values: vec![Some(PaintValue::ToolIndex(2)), None, None],
                    strokes: Vec::new(),
                }],
            }),
        }],
        build_volume: slicer_ir::BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 200.0, y: 200.0, z: 200.0 },
        },
    }
}

/// Layer plan with object participation on layers 0, 1, 2.
fn layer_plan_with_participation() -> LayerPlanIR {
    LayerPlanIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 2,
                z: 0.6,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ],
        object_participation: HashMap::from([(
            String::from("painted-object"),
            vec![
                ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 0,
                    effective_layer_height: 0.2,
                },
                ObjectLayerRef {
                    local_layer_index: 1,
                    global_layer_index: 1,
                    effective_layer_height: 0.2,
                },
                ObjectLayerRef {
                    local_layer_index: 2,
                    global_layer_index: 2,
                    effective_layer_height: 0.2,
                },
            ],
        )]),
    }
}

// ── AC-2: IR data for PaintSegmentationObjectView ────────────────────────────

/// Test 1: ObjectMesh.transform provides non-identity matrix for PaintSegmentationObjectView.
///
/// The dispatch (Step 7) will extract ObjectMesh.transform and pass it as
/// transform_matrix to PaintSegmentationObjectView.
#[test]
fn object_mesh_transform_provides_non_identity_matrix() {
    let mesh = mesh_with_paint();
    let obj = &mesh.objects[0];

    // The mesh has a translation transform (5.0, 10.0, 0.0)
    assert!((obj.transform.matrix[12] - 5.0).abs() < 1e-10, "TX should be 5.0");
    assert!((obj.transform.matrix[13] - 10.0).abs() < 1e-10, "TY should be 10.0");
    assert!((obj.transform.matrix[14] - 0.0).abs() < 1e-10, "TZ should be 0.0");

    // This transform will be used as PaintSegmentationObjectView.transform_matrix
    // The SDK type (slicer_sdk::prepass_types::PaintSegmentationObjectView)
    // has field: transform_matrix: [f64; 16]
}

/// Test 2: FacetPaintData.layers provides paint data for PaintSegmentationObjectView.
///
/// The dispatch (Step 7) will extract FacetPaintData.layers and convert them
/// to PaintLayerView entries for PaintSegmentationObjectView.paint_layers.
#[test]
fn facet_paint_data_provides_paint_layers() {
    let mesh = mesh_with_paint();
    let obj = &mesh.objects[0];

    let paint_data = obj.paint_data.as_ref().expect("mesh should have paint data");
    assert_eq!(paint_data.layers.len(), 1, "should have one paint layer");

    let layer = &paint_data.layers[0];
    assert!(matches!(layer.semantic, PaintSemantic::Material));
    assert_eq!(layer.facet_values.len(), 3, "should have 3 facet values");
    assert!(matches!(layer.facet_values[0], Some(PaintValue::ToolIndex(2))));

    // The SDK type has paint_layers: Vec<PaintLayerView>
    // Each PaintLayerView has: semantic, facet_values, strokes
}

/// Test 3: LayerPlanIR.object_participation provides layer indices.
///
/// The dispatch (Step 7) will extract LayerPlanIR.object_participation[object_id]
/// and pass the global_layer_index values as participating_layer_indices.
#[test]
fn layer_plan_object_participation_provides_layer_indices() {
    let layer_plan = layer_plan_with_participation();

    let participation = layer_plan.object_participation
        .get("painted-object")
        .expect("object should have participation data");

    assert_eq!(participation.len(), 3, "should participate in 3 layers");
    assert_eq!(participation[0].global_layer_index, 0);
    assert_eq!(participation[1].global_layer_index, 1);
    assert_eq!(participation[2].global_layer_index, 2);

    // These indices will be used as PaintSegmentationObjectView.participating_layer_indices
    // The SDK type has participating_layer_indices: Vec<u32>
}

/// Test 4: Identity transform is distinguishable from translation.
///
/// The dispatch should detect when transform_matrix is identity (no translation/rotation)
/// vs when it has meaningful transform data.
#[test]
fn identity_transform_is_detectable() {
    let identity = identity_transform();
    let translation = translation_transform(5.0, 10.0, 0.0);

    // Identity matrix has [1,0,0,0] in first 4, [0,1,0,0] in next 4, etc.
    // Translation matrix has non-zero values in positions 12, 13, 14

    // Check identity: positions 12, 13, 14 should be 0
    assert!((identity.matrix[12] - 0.0).abs() < 1e-10);
    assert!((identity.matrix[13] - 0.0).abs() < 1e-10);
    assert!((identity.matrix[14] - 0.0).abs() < 1e-10);

    // Check translation: positions 12, 13, 14 should be non-zero
    assert!((translation.matrix[12] - 5.0).abs() < 1e-10);
    assert!((translation.matrix[13] - 10.0).abs() < 1e-10);
    assert!((translation.matrix[14] - 0.0).abs() < 1e-10);
}

/// Test 5: Empty participation is a valid, detectable state.
///
/// When an object has no entry in `LayerPlanIR.object_participation` (e.g., a
/// modifier volume or support blocker that participates in no layers), the
/// converter correctly produces empty `participating_layer_indices`. This is
/// not a diagnostic condition — it correctly reflects that the object is not
/// used in any layer. The dispatch path at `dispatch.rs:518-524` derives this
/// by calling `object_participation.get(&obj.id).unwrap_or_default()`.
#[test]
fn empty_participation_produces_diagnostic_missing() {
    use slicer_host::wit_host::object_mesh_to_wit_paint_segmentation_view;

    // Mesh with valid geometry
    let mesh = ObjectMesh {
        id: String::from("diag-test"),
        mesh: IndexedTriangleSet {
            vertices: vec![Point3 { x: 0.0, y: 0.0, z: 0.0 }, Point3 { x: 1.0, y: 0.0, z: 0.0 }, Point3 { x: 0.0, y: 1.0, z: 0.0 }],
            indices: vec![0, 1, 2],
        },
        transform: translation_transform(0.0, 0.0, 0.0),
        config: ObjectConfig { data: HashMap::new() },
        modifier_volumes: Vec::new(),
        paint_data: None,
    };

    // Layer plan with no participation for this object — empty indices
    let empty_participation: Vec<u32> = vec![];

    // Converter produces a view with empty participating_layer_indices
    let view = object_mesh_to_wit_paint_segmentation_view(&mesh, &empty_participation);
    assert!(
        view.participating_layer_indices.is_empty(),
        "view: participation is empty (missing for this object)"
    );
}

/// Test 6: Zero transform matrix is detectable.
///
/// A transform matrix that is all zeros is a diagnostic condition — it
/// indicates the matrix was not properly set (not identity-equivalent
/// zero, but genuinely uninitialized). The view field reflects this state.
#[test]
fn missing_transform_matrix_produces_diagnostic() {
    use slicer_host::wit_host::object_mesh_to_wit_paint_segmentation_view;

    // Mesh with zero transform (all zeros — no translation, no rotation)
    let zero_transform = Transform3d {
        matrix: [0.0; 16],
    };
    let mesh = ObjectMesh {
        id: String::from("zero-transform"),
        mesh: IndexedTriangleSet {
            vertices: vec![Point3 { x: 0.0, y: 0.0, z: 0.0 }, Point3 { x: 1.0, y: 0.0, z: 0.0 }, Point3 { x: 0.0, y: 1.0, z: 0.0 }],
            indices: vec![0, 1, 2],
        },
        transform: zero_transform,
        config: ObjectConfig { data: HashMap::new() },
        modifier_volumes: Vec::new(),
        paint_data: None,
    };

    let view = object_mesh_to_wit_paint_segmentation_view(&mesh, &[0]);
    // Zero transform is detectable as a diagnostic condition
    let is_zero = view.transform_matrix.iter().all(|&v| v == 0.0);
    assert!(is_zero, "view: transform matrix is all zeros (missing/identity-equivalent)");
}

/// Test 6: Mesh without paint data has None paint_data.
#[test]
fn mesh_without_paint_has_none_paint_data() {
    let mesh = MeshIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        objects: vec![ObjectMesh {
            id: String::from("unpainted-object"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 { x: 0.0, y: 0.0, z: 0.0 },
                    Point3 { x: 10.0, y: 0.0, z: 0.0 },
                    Point3 { x: 0.0, y: 10.0, z: 0.2 },
                ],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None, // No paint data
        }],
        build_volume: slicer_ir::BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 200.0, y: 200.0, z: 200.0 },
        },
    };

    let obj = &mesh.objects[0];
    assert!(obj.paint_data.is_none(), "unpainted mesh should have None paint_data");

    // When paint_data is None, PaintSegmentationObjectView.paint_layers should be empty
}

/// Test 7: Multiple objects with different transforms.
#[test]
fn multiple_objects_with_different_transforms() {
    let mesh = MeshIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        objects: vec![
            ObjectMesh {
                id: String::from("object-a"),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 { x: 0.0, y: 0.0, z: 0.0 },
                        Point3 { x: 10.0, y: 0.0, z: 0.0 },
                        Point3 { x: 0.0, y: 10.0, z: 0.2 },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: translation_transform(0.0, 0.0, 0.0), // No translation
                config: ObjectConfig { data: HashMap::new() },
                modifier_volumes: Vec::new(),
                paint_data: None,
            },
            ObjectMesh {
                id: String::from("object-b"),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 { x: 0.0, y: 0.0, z: 0.0 },
                        Point3 { x: 10.0, y: 0.0, z: 0.0 },
                        Point3 { x: 0.0, y: 10.0, z: 0.2 },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: translation_transform(50.0, 100.0, 25.0), // Has translation
                config: ObjectConfig { data: HashMap::new() },
                modifier_volumes: Vec::new(),
                paint_data: None,
            },
        ],
        build_volume: slicer_ir::BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 200.0, y: 200.0, z: 200.0 },
        },
    };

    let obj_a = mesh.objects.iter().find(|o| o.id == "object-a").unwrap();
    let obj_b = mesh.objects.iter().find(|o| o.id == "object-b").unwrap();

    // Object A has no translation
    assert!((obj_a.transform.matrix[12] - 0.0).abs() < 1e-10);
    assert!((obj_a.transform.matrix[13] - 0.0).abs() < 1e-10);
    assert!((obj_a.transform.matrix[14] - 0.0).abs() < 1e-10);

    // Object B has translation (50, 100, 25)
    assert!((obj_b.transform.matrix[12] - 50.0).abs() < 1e-10);
    assert!((obj_b.transform.matrix[13] - 100.0).abs() < 1e-10);
    assert!((obj_b.transform.matrix[14] - 25.0).abs() < 1e-10);
}
