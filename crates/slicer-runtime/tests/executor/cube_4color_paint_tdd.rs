//! Paint pipeline tests for `cube_4color.3mf`.
//!
//! The cube (25mm side) has 4 Material ToolIndex values:
//!   0=orange, 1=green, 2=blue, 3=red
//!
//! Per-face paint layout (world-space after build transform translation):
//!   Front (-Y, Y≈92.5) — 4 colors banded by height (subdivided hex)
//!   Back  (+Y, Y≈117.5) — fully blue (ToolIndex 2)
//!   Left  (-X, X≈112.5) — orange with 4 circles of each color (subdivided hex)
//!   Right (+X, X≈137.5) — green (ToolIndex 1)
//!   Top   (+Z, Z≈24.9) — half red (ToolIndex 3) / half orange (ToolIndex 0)
//!   Bottom(-Z, Z≈0.1)  — one blue (ToolIndex 2) / one unpainted
//!
//! GREEN tests verify whole-facet paint survives the pipeline.
//! RED tests document gaps where sub-facet paint detail (circles, bands, half-faces
//! from hex subdivision) is lost because `execute_paint_segmentation` discards
//! `PaintLayer.strokes`.

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
const EPSILON: i64 = 100; // 0.01 mm in internal units

fn cube_4color_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_4color.3mf"
    ))
}

fn load_cube_4color() -> MeshIR {
    let path = cube_4color_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    load_model(&path).expect("load cube_4color.3mf should succeed")
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

/// World-space face bounds for cube_4color after build transform (125,105,12.5).
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
        y_min: 925_000,   //  92.5 mm
        y_max: 1_175_000, // 117.5 mm
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

/// Collect all unique Material ToolIndex values across segment_annotations contour points.
fn unique_material_tool_indices(slice_ir: &SliceIR) -> std::collections::HashSet<u32> {
    slice_ir
        .regions
        .iter()
        .flat_map(|r| r.segment_annotations.get(&PaintSemantic::Material))
        .flatten()
        .flatten()
        .filter_map(|pv| match pv {
            Some(PaintValue::ToolIndex(t)) => Some(*t),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// GREEN: whole-facet paint survives the pipeline
// ---------------------------------------------------------------------------

/// Smoke: full pipeline does not panic.
#[test]
fn cube_4color_full_pipeline_no_panic() {
    let mesh = load_cube_4color();
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");
}

/// Paint segmentation produces at least 4 distinct Material ToolIndex regions
/// across the 50 layers.
#[test]
fn cube_4color_paint_segmentation_4_tool_indices_across_layers() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let facet_count = object.mesh.indices.len() / 3;

    let sc = build_surface_classification(object_id, facet_count);
    let lp = build_50_layer_plan(object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let mut tool_indices = std::collections::HashSet::new();
    for layer_map in paint_result.per_layer.values() {
        if let Some(regions) = layer_map.semantic_regions.get(&PaintSemantic::Material) {
            for region in regions {
                if let PaintValue::ToolIndex(t) = region.value {
                    tool_indices.insert(t);
                }
            }
        }
    }

    assert!(
        tool_indices.len() >= 4,
        "expected >=4 distinct Material ToolIndex values across all layers, got {}: {:?}",
        tool_indices.len(),
        tool_indices
    );
    for expected in [0, 1, 2, 3] {
        assert!(
            tool_indices.contains(&expected),
            "expected ToolIndex({expected}) in paint segmentation output"
        );
    }
}

/// All 50 layers have a LayerPaintMap entry (even if empty).
#[test]
fn cube_4color_all_50_layers_have_layer_map_entries() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let facet_count = object.mesh.indices.len() / 3;

    let sc = build_surface_classification(object_id, facet_count);
    let lp = build_50_layer_plan(object_id);

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

/// At Z=12.5mm, segment_annotations Material is non-empty and carries ToolIndex values.
#[test]
fn cube_4color_mid_layer_has_material_paint() {
    let mesh = load_cube_4color();
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let material_count: usize = annotation
        .slice_ir
        .regions
        .iter()
        .flat_map(|r| r.segment_annotations.get(&PaintSemantic::Material))
        .flatten()
        .flatten()
        .filter(|pv| matches!(pv, Some(PaintValue::ToolIndex(_))))
        .count();

    assert!(
        material_count > 0,
        "mid-layer at Z=12.5mm must have Material paint on contour points"
    );
    let tool_indices = unique_material_tool_indices(&annotation.slice_ir);
    eprintln!(
        "DIAGNOSTIC: mid-layer at Z=12.5mm tool indices = {:?}",
        tool_indices
    );
}

/// Modifier overlay: with a model that has Material paint, requesting FuzzySkin
/// as a required semantic without pre-existing FuzzySkin regions is expected
/// to fail deterministically. Modifier projections only overlay onto
/// already-annotated FuzzySkin regions.
#[test]
fn cube_4color_fuzzy_without_data_is_error() {
    let mesh = load_cube_4color();
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
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
            paint_region_rtree: None,
        });

    assert!(
        result.is_err(),
        "requesting FuzzySkin on Material-only model must return error (no FuzzySkin regions exist)"
    );
}

// ---------------------------------------------------------------------------
// RED: whole-facet projection coverage gaps
// ---------------------------------------------------------------------------

/// Top face (Z≈24.9mm): two triangles — ToolIndex 0 (orange) and ToolIndex 3 (red).
///
/// When facet-paint regions correctly cover contour points, segment_annotations
/// at Z=24.9mm should have both ToolIndex 0 and ToolIndex 3. Today: only
/// ToolIndex 0 appears because the ToolIndex 3 triangle's projected region
/// does not overlap any contour points. Uncovered points fall back to
/// `semantic_regions[0].value` (ToolIndex 0) or None.
///
/// Gap: facet-to-layer-plane projection in `paint_segmentation.rs` may produce
/// polygon regions that miss the sliced contour. Even horizontal-facet
/// projections can gap if the triangle's XY footprint doesn't intersect
/// the slice contour.
#[test]
fn cube_4color_top_face_two_tool_indices_requires_projection_coverage() {
    let mesh = load_cube_4color();
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let tool_indices = unique_material_tool_indices(&annotation.slice_ir);
    assert!(
        tool_indices.len() >= 2 && tool_indices.contains(&0) && tool_indices.contains(&3),
        "RED: top face at Z=24.9mm should have BOTH ToolIndex 0 (orange) and \
         ToolIndex 3 (red) from its two painted triangles. Got {} distinct ToolIndex \
         values: {tool_indices:?}.\n\
         Gap: facet projection in paint_segmentation.rs. One triangle's projected \
         region misses the sliced contour; uncovered points fall back to \
         semantic_regions[0].value or None.",
        tool_indices.len()
    );
}

/// Bottom face (Z≈0.1mm): half ToolIndex 2 (blue) / half unpainted (two triangles).
///
/// At Z=0.1mm, segment_annotations Material should show BOTH ToolIndex 2 (from the
/// painted triangle) AND None (from the unpainted triangle). Today: only
/// ToolIndex 2 or ToolIndex 0 appears; unpainted triangle's contour points
/// may receive a fallback instead of None.
///
/// Gap: same facet-projection issue — the unpainted triangle's absence of
/// paint may not be correctly reflected when nearby painted-facet projections
/// set the fallback value.
#[test]
fn cube_4color_bottom_face_painted_and_unpainted_requires_projection_coverage() {
    let mesh = load_cube_4color();
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let tool_indices = unique_material_tool_indices(&annotation.slice_ir);
    // Count None entries on contour points
    let none_count: usize = annotation
        .slice_ir
        .regions
        .iter()
        .flat_map(|r| r.segment_annotations.get(&PaintSemantic::Material))
        .flatten()
        .flatten()
        .filter(|pv| pv.is_none())
        .count();

    assert!(
        !tool_indices.is_empty() && none_count > 0,
        "RED: bottom face at Z=0.1mm should have BOTH a ToolIndex value (from the \
         painted triangle, ToolIndex 2=blue) AND None entries (from the unpainted \
         triangle). Got {} distinct ToolIndex values: {tool_indices:?}, \
         none_count={none_count}.\n\
         Gap: facet projection in paint_segmentation.rs. The unpainted triangle's \
         absence of paint may be masked when painted-facet regions set the \
         fallback value for all uncovered points.",
        tool_indices.len()
    );
}

/// Top face per-point variation: at Z=24.9mm, the two top-face triangles have
/// ToolIndex 0 (orange) and ToolIndex 3 (red). Adjacent contour points from
/// each triangle should carry different values. Today ToolIndex 3 is absent
/// because its triangle's projection misses the contour. This test filters
/// out back-face contamination (ToolIndex 2) to isolate the top-face values.
/// When 3 is absent, only 0 remains — no variation.
#[test]
fn cube_4color_top_face_per_point_variation() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let facet_count = object.mesh.indices.len() / 3;

    let sc = build_surface_classification(object_id, facet_count);
    let lp = build_50_layer_plan(object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 24.9;
    let sliced_polys = slice_mesh_ex(&object.mesh, &[test_z])
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut all_tool_indices: Vec<u32> = Vec::new();
    if let Some(material_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::Material)
    {
        for (poly_idx, poly_paint) in material_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                if let Some(PaintValue::ToolIndex(t)) = paint_slot {
                    all_tool_indices.push(*t);
                }
            }
        }
    }

    // Filter out contamination (back-face ToolIndex 2 bleed-through)
    let top_face_indices: Vec<u32> = all_tool_indices
        .iter()
        .copied()
        .filter(|&t| t == 0 || t == 3)
        .collect();

    assert!(
        !top_face_indices.is_empty(),
        "must have top-face contour points at Z=24.9mm"
    );

    let has_adjacent_change = top_face_indices.windows(2).any(|w| w[0] != w[1]);

    assert!(
        has_adjacent_change,
        "RED: adjacent contour points on the top face should carry different ToolIndex \
         values (0=orange and 3=red from the two top triangles). Got {} points with \
         ToolIndex 0 or 3: {:?}. After filtering out back-face contamination \
         (ToolIndex 2), all remaining points are uniform ToolIndex 0 — ToolIndex 3 \
         is absent. The red triangle's projected region misses the sliced contour. \
         See neighboring RED test `cube_4color_top_face_two_tool_indices_requires_projection_coverage`.",
        top_face_indices.len(),
        top_face_indices
    );
}

// ---------------------------------------------------------------------------
// RED: sub-facet detail lost — executable gap specifications
// ---------------------------------------------------------------------------

/// Front face (-Y, Y≈92.5): 4 colors banded by height.
///
/// When sub-facet strokes are consumed by `execute_paint_segmentation`,
/// different Z heights should produce different dominant ToolIndex values
/// on the front face. Today the dominant state from hex subdivision is
/// constant, so all Z heights yield the same ToolIndex.
///
/// Gap: `paint_segmentation.rs:304-368` never reads `layer.strokes`.
#[test]
fn cube_4color_front_face_banded_by_z_requires_subfacet_strokes() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let facet_count = object.mesh.indices.len() / 3;

    let sc = build_surface_classification(object_id, facet_count);
    let lp = build_50_layer_plan(object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let fb = face_bounds();

    // Collect front-face ToolIndex values at multiple Z heights
    let mut front_tool_indices_by_z: HashMap<u32, Vec<u32>> = HashMap::new();
    let test_zs = [2.0, 6.0, 10.0, 14.0, 18.0, 22.0];

    for &test_z in &test_zs {
        let sliced_polys = slice_mesh_ex(&object.mesh, &[test_z])
            .into_iter()
            .next()
            .unwrap_or_default();
        if sliced_polys.is_empty() {
            continue;
        }

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
                paint_regions: Arc::clone(&paint_result),
                required_semantics: vec![PaintSemantic::Material],
                modifier_projections: vec![],
                paint_region_rtree: None,
            })
            .expect("annotation must succeed");

        let polygons = &annotation.slice_ir.regions[0].polygons;
        let mut z_front_indices = Vec::new();
        if let Some(material_paint) = annotation.slice_ir.regions[0]
            .segment_annotations
            .get(&PaintSemantic::Material)
        {
            for (poly_idx, poly_paint) in material_paint.iter().enumerate() {
                if poly_idx >= polygons.len() {
                    continue;
                }
                for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                    if pt_idx >= polygons[poly_idx].contour.points.len() {
                        continue;
                    }
                    let point = polygons[poly_idx].contour.points[pt_idx];
                    if is_on_front_face(point, &fb) {
                        if let Some(PaintValue::ToolIndex(t)) = paint_slot {
                            z_front_indices.push(*t);
                        }
                    }
                }
            }
        }
        if !z_front_indices.is_empty() {
            front_tool_indices_by_z.insert(test_z as u32, z_front_indices);
        }
    }

    let unique_sets: std::collections::HashSet<Vec<u32>> =
        front_tool_indices_by_z.values().cloned().collect();

    assert!(
        unique_sets.len() >= 2,
        "RED: front face (banded by height) should produce different ToolIndex values at \
         different Z heights. Got {} unique ToolIndex sets across {} Z levels: {:?}\n\
         Gap: execute_paint_segmentation discards PaintLayer.strokes \
         (paint_segmentation.rs:304-368). The hex subdivision banding is lost; \
         only the dominant whole-facet state survives.",
        unique_sets.len(),
        front_tool_indices_by_z.len(),
        front_tool_indices_by_z
    );
}

/// Left face (-X, X≈112.5): orange (ToolIndex 0) with 4 circles of each color.
///
/// At Z=12.5mm, contour points on the left edge should show per-point variation:
/// some points carry the circle color and adjacent points carry the background
/// color (ToolIndex 0). Today, all left-edge points get the same dominant
/// whole-facet ToolIndex because strokes are discarded.
///
/// Gap: `paint_segmentation.rs:304-368` never reads `layer.strokes`.
#[test]
fn cube_4color_left_face_circles_produce_per_point_variation() {
    let mesh = load_cube_4color();
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let facet_count = object.mesh.indices.len() / 3;

    let sc = build_surface_classification(object_id, facet_count);
    let lp = build_50_layer_plan(object_id);

    let paint_result = execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), lp, true)
        .expect("paint segmentation must succeed");

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object.mesh, &[test_z])
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut left_tool_indices: Vec<u32> = Vec::new();
    if let Some(material_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::Material)
    {
        for (poly_idx, poly_paint) in material_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_left_face(point, &fb) {
                    if let Some(PaintValue::ToolIndex(t)) = paint_slot {
                        left_tool_indices.push(*t);
                    }
                }
            }
        }
    }

    assert!(
        !left_tool_indices.is_empty(),
        "must have left-face contour points at Z=12.5mm"
    );

    let unique_left: std::collections::HashSet<u32> = left_tool_indices.iter().copied().collect();
    assert!(
        unique_left.len() >= 2,
        "RED: left face should have >=2 distinct ToolIndex values (circle colors + background orange=0) \
         at per-point granularity. Got {} unique values: {:?}; all points: {:?}\n\
         Gap: execute_paint_segmentation discards PaintLayer.strokes \
         (paint_segmentation.rs:304-368). The circle subdivision detail is lost; \
         only the dominant whole-facet state survives.",
        unique_left.len(),
        unique_left,
        left_tool_indices
    );
}

// ---------------------------------------------------------------------------
// RED: per-face paint correctness — vertical-face projection gap
// ---------------------------------------------------------------------------
// These tests assert the CORRECT per-face paint values based on the
// declared 3MF paint layout. They fail today because `execute_paint_segmentation`
// projects vertical facets as zero-area strips on layer planes, so
// `point_in_paint_region()` cannot match contour points to side-face paint.
// The annotation falls back to `semantic_regions[0].value`, producing
// uniform values instead of per-face variation.
// Gap: paint_segmentation.rs facet-to-plane projection for vertical surfaces.

/// Right face (+X, X≈137.5): green (ToolIndex 1).
/// At Z=12.5mm, right-edge contour points should all carry ToolIndex 1.
/// Today: gets ToolIndex 0 (fallback from Material region[0]) because
/// vertical-face paint projections don't cover contour points.
#[test]
fn cube_4color_right_face_uniform_requires_vertical_face_projection() {
    let mesh = load_cube_4color();
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut right_tool_indices: Vec<u32> = Vec::new();
    if let Some(material_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::Material)
    {
        for (poly_idx, poly_paint) in material_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_right_face(point, &fb) {
                    if let Some(PaintValue::ToolIndex(t)) = paint_slot {
                        right_tool_indices.push(*t);
                    }
                }
            }
        }
    }

    assert!(
        !right_tool_indices.is_empty(),
        "must have right-face contour points at Z=12.5mm"
    );
    let unique: std::collections::HashSet<u32> = right_tool_indices.iter().copied().collect();
    assert!(
        unique.len() == 1 && unique.contains(&1),
        "RED: right face should be uniform ToolIndex(1)=green. Got values {unique:?}; \
         all points: {right_tool_indices:?}\n\
         Gap: vertical face facets project as zero-area strips on layer planes \
         (paint_segmentation.rs facet projection). contour points on side faces \
         fall outside paint region polygons and receive the fallback \
         ToolIndex from semantic_regions[0]."
    );
}

/// Back face (+Y, Y≈117.5): fully blue (ToolIndex 2).
/// At Z=12.5mm, back-edge contour points should all carry ToolIndex 2.
/// Today: gets ToolIndex 0 (same vertical-face projection gap as right face).
#[test]
fn cube_4color_back_face_uniform_requires_vertical_face_projection() {
    let mesh = load_cube_4color();
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
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("annotation must succeed");

    let fb = face_bounds();
    let polygons = &annotation.slice_ir.regions[0].polygons;
    let mut back_tool_indices: Vec<u32> = Vec::new();
    if let Some(material_paint) = annotation.slice_ir.regions[0]
        .segment_annotations
        .get(&PaintSemantic::Material)
    {
        for (poly_idx, poly_paint) in material_paint.iter().enumerate() {
            if poly_idx >= polygons.len() {
                continue;
            }
            for (pt_idx, paint_slot) in poly_paint.iter().enumerate() {
                if pt_idx >= polygons[poly_idx].contour.points.len() {
                    continue;
                }
                let point = polygons[poly_idx].contour.points[pt_idx];
                if is_on_back_face(point, &fb) {
                    if let Some(PaintValue::ToolIndex(t)) = paint_slot {
                        back_tool_indices.push(*t);
                    }
                }
            }
        }
    }

    assert!(
        !back_tool_indices.is_empty(),
        "must have back-face contour points at Z=12.5mm"
    );
    let unique: std::collections::HashSet<u32> = back_tool_indices.iter().copied().collect();
    assert!(
        unique.len() == 1 && unique.contains(&2),
        "RED: back face should be uniform ToolIndex(2)=blue. Got values {unique:?}; \
         all points: {back_tool_indices:?}\n\
         Gap: vertical face facets project as zero-area strips on layer planes \
         (paint_segmentation.rs facet projection). contour points on side faces \
         fall outside paint region polygons and receive the fallback \
         ToolIndex from semantic_regions[0]."
    );
}
