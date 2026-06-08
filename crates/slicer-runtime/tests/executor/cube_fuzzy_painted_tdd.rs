//! Paint pipeline tests for `cube_fuzzyPainted.3mf`.
//!
//! The cube (25mm side) has only FuzzySkin paint (no Material).
//!
//! Per-face paint layout (world-space after build transform translation 125,115,12.5):
//!   Front (-Y, Y≈102.5) — fully painted fuzzy
//!   Back  (+Y, Y≈127.5) — half fuzzy / half unpainted
//!   Right (+X, X≈137.5) — fuzzy circle (hex subdivision)
//!   Left  (-X, X≈112.5) — unpainted
//!   Top   (+Z, Z≈24.9)  — fuzzy circle (hex subdivision)
//!   Bottom(-Z, Z≈0.1)   — unpainted
//!
//! GREEN tests verify whole-facet fuzzy paint survives the pipeline.
//! RED tests document gaps where sub-facet fuzzy detail (circles, half-faces
//! from hex subdivision) is lost because `model_loader.rs:1627` hardcodes
//! `strokes: Vec::new()` for FuzzySkin and `execute_paint_segmentation`
//! discards `PaintLayer.strokes`.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ActiveRegion, FacetClass, GlobalLayer, LayerPlanIR, MeshIR, ObjectLayerRef, ObjectSurfaceData,
    PaintSemantic, PaintValue, Point2, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
    SurfaceClassificationIR,
};
use slicer_model_io::load_model;
use slicer_runtime::{
    execute_paint_segmentation, execute_slice_postprocess_paint_annotation,
    SlicePostProcessPaintAnnotationRequest,
};

const LAYER_COUNT: u32 = 50;
const LAYER_HEIGHT_MM: f32 = 0.5;
const EPSILON: i64 = 100;

fn cube_fuzzy_painted_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_fuzzyPainted.3mf"
    ))
}

fn load_cube_fuzzy_painted() -> MeshIR {
    let path = cube_fuzzy_painted_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    load_model(&path).expect("load cube_fuzzyPainted.3mf should succeed")
}

fn build_50_layer_plan(object_id: &str) -> Arc<LayerPlanIR> {
    let global_layer_indices: Vec<u32> = (0..LAYER_COUNT).collect();
    let layers: Vec<GlobalLayer> = global_layer_indices
        .iter()
        .map(|idx| GlobalLayer {
            index: *idx,
            z: LAYER_HEIGHT_MM * (*idx as f32 + 0.5),
            active_regions: vec![ActiveRegion {
                object_id: object_id.to_string(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: LAYER_HEIGHT_MM,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();
    Arc::new(LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: layers,
        object_participation: HashMap::from([(
            object_id.to_string(),
            global_layer_indices
                .iter()
                .copied()
                .enumerate()
                .map(|(local_idx, global_idx)| ObjectLayerRef {
                    local_layer_index: local_idx as u32,
                    global_layer_index: global_idx,
                    effective_layer_height: LAYER_HEIGHT_MM,
                })
                .collect(),
        )]),
    })
}

fn build_surface_classification(object_id: &str, facet_count: usize) -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object: HashMap::from([(
            object_id.to_string(),
            ObjectSurfaceData {
                facet_classes: vec![FacetClass::Normal; facet_count],
                surface_groups: Vec::new(),
                bridge_regions: Vec::new(),
                overhang_regions: Vec::new(),
            },
        )]),
    }
}

/// World-space face bounds for cube_fuzzyPainted after build transform (125,115,12.5).
/// Units: internal (1 unit = 100 nm, 1 mm = 10_000 units).
struct FaceBounds {
    x_min: i64,
    x_max: i64,
    y_min: i64,
    y_max: i64,
}

fn face_bounds() -> FaceBounds {
    FaceBounds {
        x_min: 1_125_000, // 112.5 mm
        x_max: 1_375_000, // 137.5 mm
        y_min: 1_025_000, // 102.5 mm
        y_max: 1_275_000, // 127.5 mm
    }
}

fn is_on_left_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.x - fb.x_min).abs() < EPSILON
}
fn is_on_right_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.x - fb.x_max).abs() < EPSILON
}
fn is_on_front_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.y - fb.y_min).abs() < EPSILON
}
fn is_on_back_face(p: Point2, fb: &FaceBounds) -> bool {
    (p.y - fb.y_max).abs() < EPSILON
}

/// Count FuzzySkin Flag(true) entries across segment_annotations contour points.
fn count_fuzzy_flag_true(slice_ir: &SliceIR) -> usize {
    slice_ir
        .regions
        .iter()
        .flat_map(|r| r.segment_annotations.get(&PaintSemantic::FuzzySkin))
        .flatten()
        .flatten()
        .filter(|pv| matches!(pv, Some(PaintValue::Flag(true))))
        .count()
}

/// Count FuzzySkin None entries across segment_annotations contour points.
fn count_fuzzy_none(slice_ir: &SliceIR) -> usize {
    slice_ir
        .regions
        .iter()
        .flat_map(|r| r.segment_annotations.get(&PaintSemantic::FuzzySkin))
        .flatten()
        .flatten()
        .filter(|pv| pv.is_none())
        .count()
}

// ---------------------------------------------------------------------------
// GREEN: whole-facet fuzzy paint survives the pipeline
// ---------------------------------------------------------------------------

/// Smoke: full pipeline does not panic.
#[test]
fn cube_fuzzy_painted_full_pipeline_no_panic() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result =
        execute_paint_segmentation(Arc::new(mesh), Arc::new(sc), Arc::clone(&lp), true)
            .expect("execute_paint_segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let _annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");
}

/// Paint segmentation produces FuzzySkin semantic regions across layers.
#[test]
fn cube_fuzzy_painted_paint_segmentation_emits_fuzzy_regions() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let has_fuzzy_region = paint_result
        .per_layer
        .values()
        .any(|lm| lm.semantic_regions.contains_key(&PaintSemantic::FuzzySkin));

    assert!(
        has_fuzzy_region,
        "paint segmentation must produce FuzzySkin regions for cube_fuzzyPainted"
    );

    let total_fuzzy_regions: usize = paint_result
        .per_layer
        .values()
        .filter_map(|lm| lm.semantic_regions.get(&PaintSemantic::FuzzySkin))
        .map(|r| r.len())
        .sum();

    assert!(
        total_fuzzy_regions > 0,
        "total FuzzySkin region count across all layers must be > 0"
    );
}

/// All 50 layers have a LayerPaintMap entry.
#[test]
fn cube_fuzzy_painted_all_50_layers_have_map_entries() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    assert_eq!(
        paint_result.per_layer.len() as u32,
        LAYER_COUNT,
        "expected per_layer.len() == LAYER_COUNT ({LAYER_COUNT}), got {}",
        paint_result.per_layer.len()
    );
    for i in 0..LAYER_COUNT {
        assert!(
            paint_result.per_layer.contains_key(&i),
            "layer index {i} must have a LayerPaintMap entry"
        );
    }
}

/// Front face (-Y, Y≈102.5): fully painted fuzzy. At Z=12.5mm, all front-edge
/// contour points must carry Flag(true).
#[test]
fn cube_fuzzy_painted_front_face_fully_fuzzy() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut front_paint_values: Vec<PaintValue> = Vec::new();
    if let Some(fuzzy_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::FuzzySkin)
    {
        for (poly_idx, poly_paint) in fuzzy_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_front_face(point, &fb) {
                    if let Some(pv) = paint_slot {
                        front_paint_values.push(pv.clone());
                    }
                }
            }
        }
    }

    assert!(
        !front_paint_values.is_empty(),
        "must have front-face contour points at Z=12.5mm"
    );
    for pv in &front_paint_values {
        assert_eq!(
            *pv,
            PaintValue::Flag(true),
            "front face must be fully Flag(true), got {:?}",
            pv
        );
    }
}

// ---------------------------------------------------------------------------
// RED: per-face paint correctness — vertical-face projection gap
// ---------------------------------------------------------------------------
// These tests assert the CORRECT per-face FuzzySkin values based on the
// declared 3MF paint layout. They fail today for the same reason as the
// cube_4color side-face tests: vertical facets project as zero-area strips
// on layer planes, so `point_in_paint_region()` cannot match contour points
// to side-face paint regions. The annotation falls back to
// `semantic_regions[0].value`, which for FuzzySkin is Flag(true), making
// unpainted faces appear painted.
// Gap: paint_segmentation.rs facet-to-plane projection for vertical surfaces.

/// Back face (+Y, Y≈127.5): half fuzzy, half unpainted (two separate triangles).
/// At Z=12.5mm, back-edge contour points should show BOTH Flag(true)
/// (from the painted triangle) AND None (from the unpainted triangle).
/// Today: all back-edge points get Flag(true) from paint fallback because
/// vertical-face projections don't cover contour points.
#[test]
fn cube_fuzzy_painted_back_face_half_half_requires_vertical_face_projection() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut back_true = 0usize;
    let mut back_none = 0usize;
    if let Some(fuzzy_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::FuzzySkin)
    {
        for (poly_idx, poly_paint) in fuzzy_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_back_face(point, &fb) {
                    match paint_slot {
                        Some(PaintValue::Flag(true)) => back_true += 1,
                        None => back_none += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    let total = back_true + back_none;
    assert!(total > 0, "must have back-face contour points at Z=12.5mm");
    assert!(
        back_true > 0 && back_none > 0,
        "RED: back face should have BOTH Flag(true) (painted triangle) and None \
         (unpainted triangle). Got back_true={back_true}, back_none={back_none}.\n\
         Gap: vertical face facets project as zero-area strips on layer planes \
         (paint_segmentation.rs facet projection). contour points on side faces \
         fall outside paint region polygons and receive the fallback \
         Flag(true) from semantic_regions[0]."
    );
}

/// Left face (-X, X≈112.5): unpainted.
/// At Z=12.5mm, left-edge contour points should all be None for FuzzySkin.
/// Today: gets Flag(true) from paint fallback.
#[test]
fn cube_fuzzy_painted_left_face_unpainted_requires_vertical_face_projection() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut left_true = 0usize;
    let mut left_none = 0usize;
    if let Some(fuzzy_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::FuzzySkin)
    {
        for (poly_idx, poly_paint) in fuzzy_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_left_face(point, &fb) {
                    match paint_slot {
                        Some(PaintValue::Flag(true)) => left_true += 1,
                        None => left_none += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    let total = left_true + left_none;
    assert!(total > 0, "must have left-face contour points at Z=12.5mm");
    assert!(
        left_true == 0,
        "RED: left face is unpainted — should have zero Flag(true). \
         Got left_true={left_true}, left_none={left_none}.\n\
         Gap: vertical face facets project as zero-area strips on layer planes \
         (paint_segmentation.rs facet projection). all contour points receive \
         the fallback Flag(true) from semantic_regions[0]."
    );
}

/// Bottom face (-Z): unpainted.
/// At Z=0.1mm, bottom contour points should all be None for FuzzySkin.
/// Today: gets Flag(true) from paint fallback.
#[test]
fn cube_fuzzy_painted_bottom_face_unpainted_requires_vertical_face_projection() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 0.1;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fuzzy_true = count_fuzzy_flag_true(&annotation.slice_ir);
    let fuzzy_none = count_fuzzy_none(&annotation.slice_ir);
    assert!(
        fuzzy_true + fuzzy_none > 0,
        "must have FuzzySkin contour points at Z=0.1mm"
    );
    assert!(
        fuzzy_true == 0,
        "RED: bottom face is unpainted — should have zero Flag(true). \
         Got fuzzy_true={fuzzy_true}, fuzzy_none={fuzzy_none}.\n\
         Gap: same vertical-face projection limitation as side faces. \
         contour points receive fallback Flag(true) from semantic_regions[0]."
    );
}

// ---------------------------------------------------------------------------
// GREEN: negative assertions and modifier overlay
// ---------------------------------------------------------------------------

/// cube_fuzzyPainted has no Material paint → requesting Material semantics
/// must produce a fatal error.
#[test]
fn cube_fuzzy_painted_no_material_in_segment_annotations() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        });

    assert!(
        result.is_err(),
        "requesting Material on a FuzzySkin-only model must fail"
    );
}

/// Modifier overlay on unpainted face: a FuzzySkin modifier projection
/// overrides None → Flag(true) on an already-annotated face.
#[test]
fn cube_fuzzy_painted_modifier_overlay_on_unpainted_face() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    use slicer_ir::ExPolygon;
    let left_half_projection = ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2 {
                    x: 1_125_000,
                    y: 1_020_000,
                },
                slicer_ir::Point2 {
                    x: 1_250_000,
                    y: 1_020_000,
                },
                slicer_ir::Point2 {
                    x: 1_250_000,
                    y: 1_280_000,
                },
                slicer_ir::Point2 {
                    x: 1_125_000,
                    y: 1_280_000,
                },
            ],
        },
        holes: vec![],
    };

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![left_half_projection],
            paint_region_rtree: None,
        })
        .expect("annotation with modifier overlay must succeed");

    let fuzzy_true = count_fuzzy_flag_true(&annotation.slice_ir);
    assert!(
        fuzzy_true > 0,
        "modifier projection on left half should overlay Flag(true); \
         got {fuzzy_true} Flag(true)"
    );

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut left_true = 0usize;
    if let Some(fuzzy_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::FuzzySkin)
    {
        for (poly_idx, poly_paint) in fuzzy_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_left_face(point, &fb) && matches!(paint_slot, Some(PaintValue::Flag(true)))
                {
                    left_true += 1;
                }
            }
        }
    }

    assert!(
        left_true > 0,
        "modifier projection must override unpainted left-face points: \
         expected left_true > 0, got {left_true}"
    );
}

// ---------------------------------------------------------------------------
// RED: sub-facet fuzzy detail lost — executable gap specifications
// ---------------------------------------------------------------------------

/// Right face (+X, X≈137.5): fuzzy circle (hex subdivision).
///
/// When fuzzy skin sub-facet strokes are preserved through the pipeline,
/// only contour points within the circle radius should carry Flag(true).
/// Adjacent points on the same edge should be None.
///
/// Today, the hex subdivision produces a single dominant state per facet,
/// so all right-face contour points get the same value. The circle detail
/// is lost because `model_loader.rs:1627` hardcodes `strokes: Vec::new()`
/// for FuzzySkin, and `paint_segmentation.rs:304-368` discards strokes.
#[test]
fn cube_fuzzy_painted_right_face_circle_requires_fuzzy_strokes() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut right_true = 0usize;
    let mut right_none = 0usize;
    if let Some(fuzzy_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::FuzzySkin)
    {
        for (poly_idx, poly_paint) in fuzzy_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_right_face(point, &fb) {
                    match paint_slot {
                        Some(PaintValue::Flag(true)) => right_true += 1,
                        None => right_none += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    let total_right = right_true + right_none;
    assert!(
        total_right > 0,
        "must have right-face contour points at Z=12.5mm"
    );

    assert!(
        right_true > 0 && right_none > 0,
        "RED: right face should have BOTH Flag(true) (inside circle) and None (outside circle) \
         at per-point granularity. Got right_true={right_true}, right_none={right_none}.\n\
         Gap: model_loader.rs:1627 hardcodes strokes: Vec::new() for FuzzySkin. \
         Hex subdivision circle detail is lost; only the dominant whole-facet state survives. \
         Once fuzzy strokes are stored and paint_segmentation consumes them, circles on the \
         right face will produce per-point variation."
    );
}

/// Top face (+Z, Z≈24.9mm): fuzzy circle (hex subdivision).
///
/// Same gap as right face: the circle detail is lost.
#[test]
fn cube_fuzzy_painted_top_face_circle_requires_fuzzy_strokes() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let facet_count = object_mesh.indices.len() / 3;

    let sc = build_surface_classification(&object_id, facet_count);
    let lp = build_50_layer_plan(&object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 24.9;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );

    let slice_ir = SliceIR {
        global_layer_index: (test_z / LAYER_HEIGHT_MM) as u32,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: LAYER_HEIGHT_MM,
            segment_annotations: HashMap::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            ..Default::default()
        }],
        ..Default::default()
    };

    let annotation =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions: paint_result,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut top_true = 0usize;
    let mut top_none = 0usize;
    if let Some(fuzzy_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::FuzzySkin)
    {
        for (poly_idx, poly_paint) in fuzzy_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                match paint_slot {
                    Some(PaintValue::Flag(true)) => top_true += 1,
                    None => top_none += 1,
                    _ => {}
                }
            }
        }
    }

    let total_top = top_true + top_none;
    assert!(total_top > 0, "must have contour points at Z=24.9mm");

    assert!(
        top_true > 0 && top_none > 0,
        "RED: top face should have BOTH Flag(true) (inside circle) and None (outside circle) \
         at per-point granularity. Got top_true={top_true}, top_none={top_none}.\n\
         Gap: model_loader.rs:1627 hardcodes strokes: Vec::new() for FuzzySkin. \
         Hex subdivision circle detail is lost; only the dominant whole-facet state survives. \
         Once fuzzy strokes are stored and paint_segmentation consumes them, top circles will \
         produce per-point variation."
    );
}
