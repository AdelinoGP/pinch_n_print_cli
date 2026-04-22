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

#[test]
fn material_paint_preserves_tool_index_and_only_populates_intersected_authoritative_layers() {
    let custom_surface = scene_surface_fixture(&[("painted-object", 2)]);
    let layer_plan = layer_plan_fixture(&[0, 1, 2], &[("painted-object", &[0, 2])]);
    let mesh = Arc::new(mesh_fixture(vec![painted_material_object()]));

    let paint_regions = execute_paint_segmentation(
        Arc::clone(&mesh),
        Arc::new(custom_surface),
        Arc::new(layer_plan),
    )
    .expect("material paint should project into PaintRegionIR using the authoritative layer plan");

    assert_layer_keys(&paint_regions, &[0, 1, 2]);
    assert!(paint_regions.per_layer[&1].semantic_regions.is_empty());

    for layer_index in [0_u32, 2_u32] {
        let material_regions = paint_regions.get(layer_index, &PaintSemantic::Material);
        assert_eq!(material_regions.len(), 1);
        assert_eq!(material_regions[0].object_id, "painted-object");
        assert_eq!(material_regions[0].value, PaintValue::ToolIndex(2));
        assert_eq!(material_regions[0].paint_order, 0);
        assert!(!material_regions[0].polygons.is_empty());
    }
}

#[test]
fn support_and_fuzzy_semantics_are_emitted_as_independent_paint_region_families() {
    let mesh = Arc::new(mesh_fixture(vec![painted_semantic_family_object()]));

    let paint_regions = execute_paint_segmentation(
        Arc::clone(&mesh),
        Arc::new(scene_surface_fixture(&[("semantic-object", 1)])),
        Arc::new(layer_plan_fixture(&[0], &[("semantic-object", &[0])])),
    )
    .expect("all documented built-in semantic families should appear in PaintRegionIR");

    assert_eq!(
        semantic_values(&paint_regions, 0, &PaintSemantic::FuzzySkin),
        vec![PaintValue::Flag(true)]
    );
    assert_eq!(
        semantic_values(&paint_regions, 0, &PaintSemantic::SupportEnforcer),
        vec![PaintValue::Flag(true)]
    );
    assert_eq!(
        semantic_values(&paint_regions, 0, &PaintSemantic::SupportBlocker),
        vec![PaintValue::Flag(true)]
    );
}

#[test]
fn custom_semantics_preserve_module_key_and_stable_paint_order() {
    let semantic = PaintSemantic::Custom(String::from("com.example.texture/roughness@1"));
    let mesh = Arc::new(mesh_fixture(vec![painted_custom_object(semantic.clone())]));

    let paint_regions = execute_paint_segmentation(
        Arc::clone(&mesh),
        Arc::new(scene_surface_fixture(&[("custom-object", 2)])),
        Arc::new(layer_plan_fixture(&[0], &[("custom-object", &[0])])),
    )
    .expect("custom semantics should be preserved for downstream module ownership");

    let custom_regions = paint_regions.get(0, &semantic);
    assert_eq!(custom_regions.len(), 2);
    assert_eq!(custom_regions[0].object_id, "custom-object");
    assert_eq!(custom_regions[0].value, PaintValue::Scalar(0.25));
    assert_eq!(custom_regions[0].paint_order, 0);
    assert_eq!(custom_regions[1].value, PaintValue::Scalar(0.75));
    assert_eq!(custom_regions[1].paint_order, 1);
}

#[test]
fn authoritative_layers_without_paint_still_get_empty_layer_maps() {
    let mesh = Arc::new(mesh_fixture(vec![unpainted_object()]));

    let paint_regions = execute_paint_segmentation(
        Arc::clone(&mesh),
        Arc::new(scene_surface_fixture(&[("plain-object", 1)])),
        Arc::new(layer_plan_fixture(&[0, 1], &[("plain-object", &[0])])),
    )
    .expect("every authoritative layer should produce a LayerPaintMap entry");

    assert_layer_keys(&paint_regions, &[0, 1]);
    assert_eq!(paint_regions.per_layer[&0].global_layer_index, 0);
    assert_eq!(paint_regions.per_layer[&1].global_layer_index, 1);
    assert!(paint_regions.per_layer[&0].semantic_regions.is_empty());
    assert!(paint_regions.per_layer[&1].semantic_regions.is_empty());
}

#[test]
fn equal_precedence_conflicting_custom_values_fail_fatally_and_deterministically() {
    let semantic = PaintSemantic::Custom(String::from("com.example.texture/roughness@1"));
    let mesh = Arc::new(mesh_fixture(vec![conflicting_custom_object(
        semantic.clone(),
    )]));

    assert_eq!(
        execute_paint_segmentation(
            Arc::clone(&mesh),
            Arc::new(scene_surface_fixture(&[("conflict-object", 2)])),
            Arc::new(layer_plan_fixture(&[0], &[("conflict-object", &[0])])),
        ),
        Err(PaintSegmentationError::DeterministicConflict {
            global_layer_index: 0,
            object_id: String::from("conflict-object"),
            semantic,
            paint_order: 0,
        })
    );
}

#[test]
fn missing_required_upstream_object_data_is_reported_as_a_fatal_contract_error() {
    let mesh = Arc::new(mesh_fixture(vec![painted_material_object()]));

    assert_eq!(
        execute_paint_segmentation(
            Arc::clone(&mesh),
            Arc::new(scene_surface_fixture(&[])),
            Arc::new(layer_plan_fixture(&[0], &[("painted-object", &[0])])),
        ),
        Err(PaintSegmentationError::MissingSurfaceObject {
            object_id: String::from("painted-object"),
        })
    );
}

fn assert_layer_keys(paint_regions: &PaintRegionIR, expected_layers: &[u32]) {
    let mut observed = paint_regions.per_layer.keys().copied().collect::<Vec<_>>();
    observed.sort_unstable();
    assert_eq!(observed, expected_layers);
}

fn semantic_values(
    paint_regions: &PaintRegionIR,
    layer_index: u32,
    semantic: &PaintSemantic,
) -> Vec<PaintValue> {
    paint_regions
        .get(layer_index, semantic)
        .iter()
        .map(|region| region.value)
        .collect()
}

fn painted_material_object() -> ObjectMesh {
    ObjectMesh {
        id: String::from("painted-object"),
        mesh: two_triangle_mesh(),
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::ToolIndex(2)), None],
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: None,
    }
}

fn painted_semantic_family_object() -> ObjectMesh {
    ObjectMesh {
        id: String::from("semantic-object"),
        mesh: single_triangle_mesh(),
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![
                PaintLayer {
                    semantic: PaintSemantic::FuzzySkin,
                    facet_values: vec![Some(PaintValue::Flag(true))],
                    strokes: Vec::new(),
                },
                PaintLayer {
                    semantic: PaintSemantic::SupportEnforcer,
                    facet_values: vec![Some(PaintValue::Flag(true))],
                    strokes: Vec::new(),
                },
                PaintLayer {
                    semantic: PaintSemantic::SupportBlocker,
                    facet_values: vec![Some(PaintValue::Flag(true))],
                    strokes: Vec::new(),
                },
            ],
        }),
        world_z_extent: None,
    }
}

fn painted_custom_object(semantic: PaintSemantic) -> ObjectMesh {
    ObjectMesh {
        id: String::from("custom-object"),
        mesh: two_triangle_mesh(),
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![
                PaintLayer {
                    semantic: semantic.clone(),
                    facet_values: vec![Some(PaintValue::Scalar(0.25)), None],
                    strokes: Vec::new(),
                },
                PaintLayer {
                    semantic,
                    facet_values: vec![None, Some(PaintValue::Scalar(0.75))],
                    strokes: Vec::new(),
                },
            ],
        }),
        world_z_extent: None,
    }
}

fn unpainted_object() -> ObjectMesh {
    ObjectMesh {
        id: String::from("plain-object"),
        mesh: single_triangle_mesh(),
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    }
}

fn conflicting_custom_object(semantic: PaintSemantic) -> ObjectMesh {
    ObjectMesh {
        id: String::from("conflict-object"),
        mesh: overlapping_triangle_mesh(),
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic,
                facet_values: vec![Some(PaintValue::Scalar(0.2)), Some(PaintValue::Scalar(0.8))],
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: None,
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

fn scene_surface_fixture(objects: &[(&str, usize)]) -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        schema_version: schema_version(),
        per_object: objects
            .iter()
            .map(|(object_id, facet_count)| {
                (
                    (*object_id).to_owned(),
                    ObjectSurfaceData {
                        facet_classes: vec![FacetClass::Normal; *facet_count],
                        surface_groups: Vec::new(),
                        bridge_regions: Vec::new(),
                        overhang_regions: Vec::new(),
                    },
                )
            })
            .collect(),
    }
}

fn layer_plan_fixture(
    global_layers: &[u32],
    object_participation: &[(&str, &[u32])],
) -> LayerPlanIR {
    LayerPlanIR {
        schema_version: schema_version(),
        global_layers: global_layers.iter().copied().map(global_layer).collect(),
        object_participation: object_participation
            .iter()
            .map(|(object_id, layers)| {
                (
                    (*object_id).to_owned(),
                    layers
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(local_layer_index, global_layer_index)| ObjectLayerRef {
                            local_layer_index: local_layer_index as u32,
                            global_layer_index,
                            effective_layer_height: 0.2,
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}

fn global_layer(index: u32) -> GlobalLayer {
    GlobalLayer {
        index,
        z: 0.2 * (index as f32 + 1.0),
        active_regions: vec![ActiveRegion {
            object_id: String::from("placeholder-object"),
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
    }
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

fn overlapping_triangle_mesh() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            point3(0.0, 0.0, 0.0),
            point3(10.0, 0.0, 0.0),
            point3(0.0, 10.0, 0.2),
            point3(0.0, 0.0, 0.1),
            point3(10.0, 0.0, 0.1),
            point3(0.0, 10.0, 0.3),
        ],
        indices: vec![0, 1, 2, 3, 4, 5],
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn point3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn schema_version() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}
