//! Packet 56b â€” Modifier-part IR routing E2E tests against `benchy_4color.3mf`.

#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ActiveRegion, ConfigValue, ExPolygon, FacetClass, GlobalLayer, LayerPaintMap, LayerPlanIR,
    MeshIR, ObjectLayerRef, ObjectSurfaceData, PaintRegionIR, PaintSemantic, PaintValue, Point2,
    Polygon, ResolvedConfig, SemVer, SemanticRegion, SliceIR, SlicedRegion,
    SurfaceClassificationIR,
};
use slicer_runtime::model_loader::load_model;
use slicer_runtime::{
    execute_paint_segmentation, execute_region_mapping, execute_slice_postprocess_paint_annotation,
    ExecutionPlan, SlicePostProcessPaintAnnotationRequest, SlicePostProcessPaintAnnotationResult,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn benchy_4color_3mf() -> PathBuf {
    repo_root().join("resources/benchy_4color.3mf")
}

fn benchy_painted_3mf() -> PathBuf {
    repo_root().join("resources/benchy_painted.3mf")
}

/// Build a minimal single-region `LayerPlanIR` at the given Z height (mm).
///
/// This is used by Step 4 RED tests to drive `execute_region_mapping`
/// with a synthetic layer plan so we can inspect whether modifier-volume
/// config deltas are stamped into `RegionPlan.config.extensions`.
fn single_region_layer_plan(layer_index: u32, z_mm: f32) -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: layer_index,
            z: z_mm,
            active_regions: vec![ActiveRegion {
                object_id: "obj_0".to_string(),
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
        }],
        object_participation: Default::default(),
        ..Default::default()
    }
}

/// Build a minimal `PaintRegionIR` with a tiny FuzzySkin semantic region for the
/// given layer index. The region polygon is too small to cover any contour point,
/// so all points get the default `Flag(false)`. Modifier projections then overlay
/// `Flag(true)` on overlapping points.
fn fuzzy_paint_regions(layer_index: u32) -> PaintRegionIR {
    PaintRegionIR {
        per_layer: HashMap::from([(
            layer_index,
            LayerPaintMap {
                global_layer_index: layer_index,
                semantic_regions: HashMap::from([(
                    PaintSemantic::FuzzySkin,
                    vec![SemanticRegion {
                        object_id: "obj_0".to_string(),
                        polygons: vec![ExPolygon {
                            contour: Polygon {
                                points: vec![
                                    Point2::from_mm(0.0, 0.0),
                                    Point2::from_mm(1.0, 0.0),
                                    Point2::from_mm(0.0, 1.0),
                                ],
                            },
                            holes: vec![],
                        }],
                        value: PaintValue::Flag(false),
                        paint_order: 0,
                        aabb: None,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// REQ-MODIFIER-001 / REQ-MODIFIER-002
// Modifier geometry must be excluded from the solid mesh and appear in
// modifier_volumes instead.
// ---------------------------------------------------------------------------

#[test]
fn modifier_part_excluded_from_solid_mesh() {
    let path = benchy_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf should succeed");

    // The primary solid object must have exactly 225_240 triangles (Benchy hull
    // without the modifier cube merged in). Currently fails because the modifier
    // geometry is merged and schema_version is 1.0.0.
    let solid_obj = &mesh_ir.objects[0];
    let tri_count = solid_obj.mesh.indices.len() / 3;
    assert_eq!(
        tri_count, 225_240,
        "solid mesh has {tri_count} triangles, expected 225_240"
    );

    assert_eq!(
        mesh_ir.schema_version,
        SemVer {
            major: 1,
            minor: 1,
            patch: 0
        },
        "expected schema_version 1.1.0, got {:?}",
        mesh_ir.schema_version
    );
}

// ---------------------------------------------------------------------------
// REQ-MODIFIER-003
// ModifierVolume must carry typed metadata from the sidecar.
// ---------------------------------------------------------------------------

#[test]
fn modifier_volume_carries_typed_metadata() {
    let path = benchy_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf should succeed");

    let solid_obj = &mesh_ir.objects[0];

    assert_eq!(
        solid_obj.modifier_volumes.len(),
        1,
        "expected 1 modifier volume"
    );

    let mv = &solid_obj.modifier_volumes[0];

    let fuzzy = mv.config_delta.fields.get("fuzzy_skin");
    assert_eq!(
        fuzzy,
        Some(&ConfigValue::String("external".to_string())),
        "config_delta[fuzzy_skin] = {:?}",
        fuzzy
    );

    let subtype = mv.config_delta.fields.get("subtype");
    assert_eq!(
        subtype,
        Some(&ConfigValue::String("modifier_part".to_string())),
        "config_delta[subtype] = {:?}",
        subtype
    );
}

// ---------------------------------------------------------------------------
// REQ-MODIFIER-004
// Modifier world-space AABB centroid must be in the positive octant.
// ---------------------------------------------------------------------------

#[test]
fn modifier_world_aabb_matches_composition() {
    let path = benchy_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf should succeed");

    let solid_obj = &mesh_ir.objects[0];

    assert!(
        !solid_obj.modifier_volumes.is_empty(),
        "modifier_volumes is empty"
    );

    let mv = &solid_obj.modifier_volumes[0];
    let verts = &mv.mesh.vertices;
    assert!(!verts.is_empty(), "modifier mesh has no vertices");

    let n = verts.len() as f32;
    let cx: f32 = verts.iter().map(|v| v.x).sum::<f32>() / n;
    let cy: f32 = verts.iter().map(|v| v.y).sum::<f32>() / n;
    let cz: f32 = verts.iter().map(|v| v.z).sum::<f32>() / n;
    const EXPECTED_CX: f32 = 113.51964;
    const EXPECTED_CY: f32 = 90.13154;
    const EXPECTED_CZ: f32 = 7.070797;

    assert!(
        (cx - EXPECTED_CX).abs() < 0.01,
        "centroid X mismatch: got {cx}, expected {EXPECTED_CX}"
    );
    assert!(
        (cy - EXPECTED_CY).abs() < 0.01,
        "centroid Y mismatch: got {cy}, expected {EXPECTED_CY}"
    );
    assert!(
        (cz - EXPECTED_CZ).abs() < 0.01,
        "centroid Z mismatch: got {cz}, expected {EXPECTED_CZ}"
    );

    // The modifier cube sits somewhere within the Benchy hull (positive octant).
}

/// Validates that `execute_slice_postprocess_paint_annotation` stamps
/// `FuzzySkin=Flag(true)` on contour points whose XY projection intersects
/// a modifier volume at the same Z, and leaves points untouched when the
/// layer Z is below the modifier's Z-band.
#[test]
fn modifier_projections_annotate_contour_points() {
    let path = benchy_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());

    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf");
    let solid_obj = &mesh_ir.objects[0];

    assert!(!solid_obj.modifier_volumes.is_empty());
    let mv = &solid_obj.modifier_volumes[0];
    assert!(!mv.mesh.vertices.is_empty());

    let verts = &mv.mesh.vertices;
    let z_min = verts.iter().map(|v| v.z).fold(f32::INFINITY, f32::min);
    let z_max = verts.iter().map(|v| v.z).fold(f32::NEG_INFINITY, f32::max);
    assert!(z_max > z_min);

    // Slice the modifier mesh at a Z inside its band to get projections.
    let test_z = (z_min + z_max) / 2.0;
    let layers = slice_mesh_ex(&mv.mesh, &[test_z]);
    let modifier_projections = layers.into_iter().next().unwrap();
    assert!(
        !modifier_projections.is_empty(),
        "modifier must produce at least one ExPolygon at Z={test_z}"
    );

    // Build a synthetic SliceIR at the same Z using the modifier's own
    // ExPolygon projection as the region polygon â€” its contour points
    // are on the modifier boundary, so boundary_paint annotation works.
    let region_polygon = modifier_projections[0].clone();

    let slice_ir = SliceIR {
        global_layer_index: 0,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: "obj_0".to_string(),
            region_id: 0,
            polygons: vec![region_polygon.clone()],
            infill_areas: vec![region_polygon],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            boundary_paint: HashMap::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        }],
        ..Default::default()
    };

    let paint_regions = Arc::new(fuzzy_paint_regions(0));

    assert!(
        !modifier_projections.is_empty(),
        "modifier_projections is empty"
    );

    let result: SlicePostProcessPaintAnnotationResult =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: modifier_projections.clone(),
            paint_region_rtree: None,
        })
        .expect("paint annotation must succeed for in-band layer");

    // At least one contour point must have FuzzySkin=Flag(true).
    let fuzzy_painted: usize = result
        .slice_ir
        .regions
        .iter()
        .flat_map(|r| r.boundary_paint.get(&PaintSemantic::FuzzySkin))
        .flatten()
        .flatten()
        .filter(|pv| matches!(pv, Some(PaintValue::Flag(true))))
        .count();
    assert!(
        fuzzy_painted > 0,
        "expected at least one contour point with FuzzySkin=Flag(true) in-band"
    );

    // --- Below-band layer ---
    let below_z = z_min - 1.0;
    let below_layers = slice_mesh_ex(&mv.mesh, &[below_z]);
    let below_projections = below_layers.into_iter().next().unwrap();
    // At below_z, no triangles intersect the modifier â€” projections are empty.
    assert!(
        below_projections.is_empty(),
        "modifier must produce zero ExPolygons at Z={below_z} (below band)"
    );

    let below_region_polygon = modifier_projections[0].clone();

    let below_slice_ir = SliceIR {
        global_layer_index: 1,
        z: below_z,
        regions: vec![SlicedRegion {
            object_id: "obj_0".to_string(),
            region_id: 0,
            polygons: vec![below_region_polygon.clone()],
            infill_areas: vec![below_region_polygon],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            boundary_paint: HashMap::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        }],
        ..Default::default()
    };

    let below_paint_regions = Arc::new(fuzzy_paint_regions(1));

    let below_result: SlicePostProcessPaintAnnotationResult =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: below_slice_ir,
            paint_regions: below_paint_regions,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: below_projections,
            paint_region_rtree: None,
        })
        .expect("paint annotation must succeed for below-band layer");

    // No contour point should have FuzzySkin=Flag(true) below the band.
    let below_fuzzy_painted: usize = below_result
        .slice_ir
        .regions
        .iter()
        .flat_map(|r| r.boundary_paint.get(&PaintSemantic::FuzzySkin))
        .flatten()
        .flatten()
        .filter(|pv| matches!(pv, Some(PaintValue::Flag(true))))
        .count();
    assert_eq!(
        below_fuzzy_painted, 0,
        "expected zero contour points with FuzzySkin=Flag(true) below the modifier Z-band"
    );
}

/// Validates Z-band restriction: modifier projections only paint contour
/// points for layers inside the modifier's vertical extent, not below it.
#[test]
fn modifier_projection_z_band_restriction() {
    let path = benchy_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());

    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf");
    let solid_obj = &mesh_ir.objects[0];

    assert_eq!(
        solid_obj.modifier_volumes.len(),
        1,
        "expected 1 modifier volume"
    );
    let mv = &solid_obj.modifier_volumes[0];
    assert!(
        !mv.mesh.vertices.is_empty(),
        "modifier mesh has no vertices"
    );

    let verts = &mv.mesh.vertices;
    let z_min = verts.iter().map(|v| v.z).fold(f32::INFINITY, f32::min);
    let z_max = verts.iter().map(|v| v.z).fold(f32::NEG_INFINITY, f32::max);
    assert!(z_max > z_min);

    // In-band layer
    let in_z = z_min + 0.5;
    let in_projections = slice_mesh_ex(&mv.mesh, &[in_z]).into_iter().next().unwrap();
    assert!(
        !in_projections.is_empty(),
        "modifier must project at in-band Z={in_z}"
    );
    // Use the modifier's own ExPolygon as region polygon â€” its contour
    // points lie on the modifier boundary and will be annotated.
    let in_region = in_projections[0].clone();

    let make_slice_ir = |z: f32, idx: u32, poly: ExPolygon| -> SliceIR {
        SliceIR {
            global_layer_index: idx,
            z,
            regions: vec![SlicedRegion {
                object_id: "obj_0".to_string(),
                region_id: 0,
                polygons: vec![poly.clone()],
                infill_areas: vec![poly],
                nonplanar_surface: None,
                effective_layer_height: 0.2,
                boundary_paint: HashMap::new(),
                top_shell_index: None,
                bottom_shell_index: None,
                top_solid_fill: Vec::new(),
                bottom_solid_fill: Vec::new(),
                is_bridge: false,
                bridge_areas: vec![],
                bridge_orientation_deg: 0.0,
            }],
            ..Default::default()
        }
    };

    let layer0_paint_regions = Arc::new(fuzzy_paint_regions(0));
    let layer1_paint_regions = Arc::new(fuzzy_paint_regions(1));

    let in_result: SlicePostProcessPaintAnnotationResult =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: make_slice_ir(in_z, 0, in_region.clone()),
            paint_regions: layer0_paint_regions.clone(),
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: in_projections,
            paint_region_rtree: None,
        })
        .expect("in-band annotation");

    // Below-band layer (control)
    let below_z = z_min - 1.0;
    let below_projections = slice_mesh_ex(&mv.mesh, &[below_z])
        .into_iter()
        .next()
        .unwrap();
    // At below_z, no triangles intersect â€” projections are empty.
    assert!(below_projections.is_empty());
    let below_result: SlicePostProcessPaintAnnotationResult =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: make_slice_ir(below_z, 1, in_region.clone()),
            paint_regions: layer1_paint_regions,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: below_projections,
            paint_region_rtree: None,
        })
        .expect("below-band annotation");

    let count_flag_true = |result: &SlicePostProcessPaintAnnotationResult| -> usize {
        result
            .slice_ir
            .regions
            .iter()
            .flat_map(|r| r.boundary_paint.get(&PaintSemantic::FuzzySkin))
            .flatten()
            .flatten()
            .filter(|pv| matches!(pv, Some(PaintValue::Flag(true))))
            .count()
    };

    assert!(
        count_flag_true(&in_result) > 0,
        "in-band layer must have at least one FuzzySkin=Flag(true) point"
    );
    assert_eq!(
        count_flag_true(&below_result),
        0,
        "below-band layer must have zero FuzzySkin=Flag(true) points"
    );
}

/// Negative-invariant: when a model has no modifier volumes (`benchy_painted.3mf`
/// has no sidecar), `execute_region_mapping` must not stamp any modifier-derived
/// keys into `RegionPlan.config.extensions`.
#[test]
fn empty_modifier_volume_stamps_no_regions() {
    // benchy_painted.3mf has no model_settings.config sidecar â†’ no
    // modifier volumes are parsed â†’ modifier_volumes is empty.
    let path = benchy_painted_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());

    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_painted.3mf");

    // Confirm: no modifier volumes on any object.
    let total_modifier_volumes: usize = mesh_ir
        .objects
        .iter()
        .map(|o| o.modifier_volumes.len())
        .sum();
    assert_eq!(
        total_modifier_volumes, 0,
        "benchy_painted.3mf must have 0 modifier volumes (no sidecar), \
         got {total_modifier_volumes}"
    );

    // Run region mapping on a minimal plan covering Z = 10.0 mm.
    let layer_plan = single_region_layer_plan(0, 10.0);
    let plan = ExecutionPlan::default();
    let paint_semantic_configs = BTreeMap::new();

    let region_map = execute_region_mapping(&layer_plan, &plan, None, &paint_semantic_configs, &[])
        .expect("execute_region_mapping must succeed with empty modifier volumes");

    // No modifier volumes â†’ no modifier-derived keys in any region.
    let stamped_count = region_map
        .entries
        .values()
        .filter(|rp| rp.config.extensions.contains_key("fuzzy_skin.apply_to_all"))
        .count();

    assert_eq!(
        stamped_count, 0,
        "empty modifier_volumes must produce 0 stamped regions, \
         got stamped_count={stamped_count}"
    );
}

// ---------------------------------------------------------------------------
// Full pipeline paint diagnostic: prepass extraction â†’ per-layer annotation
// ---------------------------------------------------------------------------

/// Full pipeline paint-region diagnostic for `benchy_4color.3mf`.
///
/// Runs the host-side pipeline (paint extraction + paint annotation) end-to-end
/// on a real 4-color painted model and reports exactly at which stage Material
/// ToolIndex values are lost.
///
/// Failure mode 1: `STAGE=prepass_paint_extraction` â€” `execute_paint_segmentation`
///     produced fewer than 4 distinct Material ToolIndex values. The 3MF paint
///     strokes exist, but the paint-extraction IR pipeline dropped them.
/// Failure mode 2: `STAGE=prepass_material_semantic` â€” the extracted
///     `PaintRegionIR` has no `PaintSemantic::Material` key in any layer. The
///     paint data reached the IR but with a wrong semantic label.
/// Failure mode 3: `STAGE=per_layer_paint_annotation` â€” `execute_slice_postprocess_
///     paint_annotation` produced zero contour points with `Material/ToolIndex`
///     boundary_paint. The PaintRegionIR has correct semantics, but the annotation
///     step did not project them onto SlicedRegion contour points. This could be
///     a bounding-box mismatch, polygon emptiness, or semantic routing bug.
#[test]
fn benchy_4color_full_pipeline_paint_diagnostic() {
    let path = benchy_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());

    let mesh = load_model(&path).expect("load benchy_4color.3mf should succeed");
    let object = &mesh.objects[0];
    let object_id = &object.id;
    let facet_count = object.mesh.indices.len() / 3;

    // ---- Prepass phase: SurfaceClassificationIR + LayerPlanIR ----
    let sc = SurfaceClassificationIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object: HashMap::from([(
            object_id.clone(),
            ObjectSurfaceData {
                facet_classes: vec![FacetClass::Normal; facet_count],
                surface_groups: Vec::new(),
                bridge_regions: Vec::new(),
                overhang_regions: Vec::new(),
            },
        )]),
    };

    let layer_count = 20u32;
    let global_layer_indices: Vec<u32> = (0..layer_count).collect();
    let lp = Arc::new(LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: global_layer_indices
            .iter()
            .map(|idx| GlobalLayer {
                index: *idx,
                z: 0.2 * (*idx as f32 + 1.0),
                active_regions: vec![ActiveRegion {
                    object_id: object_id.clone(),
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
        object_participation: HashMap::from([(
            object_id.clone(),
            global_layer_indices
                .iter()
                .copied()
                .enumerate()
                .map(|(local_idx, global_idx)| ObjectLayerRef {
                    local_layer_index: local_idx as u32,
                    global_layer_index: global_idx,
                    effective_layer_height: 0.2,
                })
                .collect(),
        )]),
    });

    // ---- Prepass execution: extract paint regions from 3MF strokes ----
    let paint_result =
        execute_paint_segmentation(Arc::new(mesh.clone()), Arc::new(sc), Arc::clone(&lp), true)
            .expect("execute_paint_segmentation must succeed for benchy_4color");

    // CHECK 1: At least 4 distinct ToolIndex Material regions after prepass
    let mut paint_tool_indices = BTreeSet::new();
    let mut material_region_count = 0usize;
    for layer_map in paint_result.per_layer.values() {
        if let Some(regions) = layer_map.semantic_regions.get(&PaintSemantic::Material) {
            material_region_count += regions.len();
            for region in regions {
                if let PaintValue::ToolIndex(t) = region.value {
                    paint_tool_indices.insert(t);
                }
            }
        }
    }

    assert!(
        !paint_result.per_layer.is_empty(),
        "STAGE=prepass_paint_extraction â€” paint_regions.per_layer is empty; \
         the 3MF paint strokes produced zero per-layer paint data."
    );
    assert!(
        material_region_count > 0,
        "STAGE=prepass_paint_extraction â€” zero Material semantic regions found; \
         paint strokes may have been parsed under a different semantic."
    );
    eprintln!(
        "DIAGNOSTIC: paint_tool_indices after prepass extraction = {:?}",
        paint_tool_indices
    );
    assert!(
        paint_tool_indices.len() >= 4,
        "STAGE=prepass_paint_extraction â€” expected >= 4 distinct Material ToolIndex \
         values, got {}: {:?}. 3MF paint strokes exist but the paint-extraction IR \
         pipeline dropped them.",
        paint_tool_indices.len(),
        paint_tool_indices
    );

    // CHECK 2: Material semantic exists in required_semantics (derived from per_layer keys)
    let has_material_semantic = paint_result
        .per_layer
        .values()
        .any(|lm| lm.semantic_regions.contains_key(&PaintSemantic::Material));
    assert!(
        has_material_semantic,
        "STAGE=prepass_material_semantic â€” PaintSemantic::Material is absent from all \
         per_layer semantic_regions keys. The IR contains no Material semantic, so the \
         per-layer paint annotator will have nothing to project."
    );

    // ---- Per-layer phase: slice the mesh and run paint annotation ----
    // Pick a layer Z where the mesh has sliced polygons.
    let test_z = 2.0;
    let test_layer_idx = 10u32;

    let sliced_polys: Vec<ExPolygon> = slice_mesh_ex(&object.mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();

    assert!(
        !sliced_polys.is_empty(),
        "sliced_polys are empty at Z={test_z}; test cannot proceed. \
         Pick a Z that intersects the benchy_4color mesh."
    );

    let paint_regions = paint_result;
    let required_semantics = vec![PaintSemantic::Material];

    let slice_ir = SliceIR {
        global_layer_index: test_layer_idx,
        z: test_z,
        regions: vec![SlicedRegion {
            object_id: object_id.clone(),
            region_id: 0,
            polygons: sliced_polys.clone(),
            infill_areas: sliced_polys,
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            boundary_paint: HashMap::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        }],
        ..Default::default()
    };

    let annotation_result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            required_semantics,
            modifier_projections: vec![],
            paint_region_rtree: None,
        })
        .expect("execute_slice_postprocess_paint_annotation must succeed");

    // CHECK 3: boundary_paint has at least one contour point with Material/ToolIndex
    let material_tool_count: usize = annotation_result
        .slice_ir
        .regions
        .iter()
        .flat_map(|r| r.boundary_paint.get(&PaintSemantic::Material))
        .flatten()
        .flatten()
        .filter(|pv| matches!(pv, Some(PaintValue::ToolIndex(_))))
        .count();

    let boundary_tool_vals: BTreeSet<u32> = annotation_result
        .slice_ir
        .regions
        .iter()
        .flat_map(|r| r.boundary_paint.get(&PaintSemantic::Material))
        .flatten()
        .flatten()
        .filter_map(|pv| match pv {
            Some(PaintValue::ToolIndex(t)) => Some(*t),
            _ => None,
        })
        .collect();

    eprintln!(
        "DIAGNOSTIC: boundary_paint Material tool indices on layer Z={test_z} = {:?}",
        boundary_tool_vals
    );
    eprintln!("DIAGNOSTIC: boundary_paint Material contour point count = {material_tool_count}");

    assert!(
        material_tool_count > 0,
        "STAGE=per_layer_paint_annotation â€” zero contour points have \
         PaintSemantic::Material / PaintValue::ToolIndex(...) in boundary_paint. \
         PaintRegionIR contains {material_region_count} Material regions with \
         tool indices {paint_tool_indices:?}, but execute_slice_postprocess_paint_annotation \
         did not project them onto SlicedRegion contour points. Possible causes: \
         (a) polygon contour points fall outside paint region bounding boxes, \
         (b) bounding-box mismatch between mesh and paint stroke coordinates, \
         (c) semantic routing mismatch (paint stored under a different PaintSemantic key)."
    );
}
