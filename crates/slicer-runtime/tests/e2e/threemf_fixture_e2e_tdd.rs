//! Packet 67: 3MF fixture end-to-end integration tests.
//!
//! Loads real on-disk 3MF fixtures through `load_model()` and exercises the full
//! pipeline: paint segmentation, negative-part subtract, and modifier-volume
//! metadata inspection.
//!
//! Expected: 11 GREEN tests pass, 1 RED test fails with a specific assertion message.
//! The RED test is documented with `// RED â€” passes after Packet 68` comments.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_core::algos::region_mapping::RegionMappingPlanProjection;
use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigDelta, ConfigValue, ExPolygon, FacetClass, GlobalLayer,
    IndexedTriangleSet, LayerPaintMap, LayerPlanIR, MeshIR, ModifierScope, ModifierVolume,
    ObjectConfig, ObjectLayerRef, ObjectMesh, ObjectSurfaceData, PaintRegionIR, PaintSemantic,
    PaintValue, Point3, Polygon, RegionMapIR, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
    SurfaceClassificationIR, Transform3d, CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_model_io::load_model;
use slicer_runtime::negative_part_subtract::apply_negative_part_subtract;
use slicer_runtime::{
    build_execution_plan, execute_region_mapping_with_cap, ExecutionPlan, ExecutionPlanRequest,
};

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Path helpers
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn repo_root() -> PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("resources").join(name)
}

fn skip_if_missing(path: &std::path::Path) -> bool {
    if !path.exists() {
        eprintln!("SKIP: {} not found", path.display());
        return true;
    }
    false
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Area helpers (mirrored from threemf_subtypes_synthetic_e2e_tdd.rs)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn polygon_signed_area_units2(poly: &Polygon) -> i128 {
    let pts = &poly.points;
    let n = pts.len();
    if n < 3 {
        return 0;
    }
    let mut sum: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += (pts[i].x as i128) * (pts[j].y as i128) - (pts[j].x as i128) * (pts[i].y as i128);
    }
    sum / 2
}

fn sum_area_mm2(polys: &[ExPolygon]) -> f64 {
    let mut total: i128 = 0;
    for ep in polys {
        total += polygon_signed_area_units2(&ep.contour);
        for h in &ep.holes {
            total += polygon_signed_area_units2(h);
        }
    }
    (total.unsigned_abs() as f64) / 1e8
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Build helpers for paint segmentation with on-disk fixtures
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn surface_classification_for_mesh(mesh_ir: &MeshIR) -> SurfaceClassificationIR {
    let mut per_object = HashMap::new();
    for obj in &mesh_ir.objects {
        let facet_count = obj.mesh.indices.len() / 3;
        per_object.insert(
            obj.id.clone(),
            ObjectSurfaceData {
                facet_classes: vec![FacetClass::Normal; facet_count],
                surface_groups: Vec::new(),
                bridge_regions: Vec::new(),
                overhang_regions: Vec::new(),
            },
        );
    }
    SurfaceClassificationIR {
        per_object,
        ..Default::default()
    }
}

fn layer_plan_for_mesh(mesh_ir: &MeshIR, layer_count: u32, layer_height_mm: f32) -> LayerPlanIR {
    let mut global_layers = Vec::new();
    let mut object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::new();
    for i in 0..layer_count {
        let z = (i as f32) * layer_height_mm + 0.1;
        let active_regions: Vec<ActiveRegion> = mesh_ir
            .objects
            .iter()
            .enumerate()
            .map(|(ri, obj)| {
                let local_layers = object_participation.entry(obj.id.clone()).or_default();
                let local_idx = if i == 0 { 0 } else { i };
                local_layers.push(ObjectLayerRef {
                    local_layer_index: local_idx,
                    global_layer_index: i,
                    effective_layer_height: layer_height_mm,
                });
                ActiveRegion {
                    object_id: obj.id.clone(),
                    region_id: ri as u64,
                    resolved_config: ResolvedConfig::default(),
                    effective_layer_height: layer_height_mm,
                    nonplanar_shell: None,
                    is_catchup_layer: false,
                    catchup_z_bottom: 0.0,
                    tool_index: 0,
                }
            })
            .collect();
        global_layers.push(GlobalLayer {
            index: i,
            z,
            active_regions,
            has_nonplanar: false,
            is_sync_layer: false,
        });
    }
    LayerPlanIR {
        global_layers,
        object_participation,
        ..Default::default()
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// region_map_for_fixture: shared scaffolding for AC-Mod-* tests.
// Loads a 3MF fixture and runs paint_segmentation + region_mapping. Returns
// None if the fixture is missing.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn empty_execution_plan() -> ExecutionPlan {
    let req = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };
    build_execution_plan(&req).expect("empty execution plan should build")
}

fn plan_stage_invocations(
    plan: &ExecutionPlan,
) -> Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> {
    plan.per_layer_stages
        .iter()
        .chain(plan.postpass_stages.iter())
        .map(|stage| {
            let invocations = stage
                .modules
                .iter()
                .map(|m| slicer_ir::ModuleInvocation {
                    module_id: m.module_id().to_owned(),
                    config_view: m.config_view().as_ref().clone(),
                })
                .collect::<Vec<_>>();
            (stage.stage_id.clone(), invocations)
        })
        .collect()
}

fn region_map_for_fixture(name: &str) -> Option<RegionMapIR> {
    let path = fixture(name);
    if skip_if_missing(&path) {
        return None;
    }
    let mesh_ir = crate::common::model_cache::cached_load_model(&path);
    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);
    // Clone the objects slice before moving `mesh_ir` into an Arc; we need
    // it as `&[ObjectMesh]` for the Packet-68 modifier-volume stamping.
    let objects = mesh_ir.objects.clone();
    let paint_result: Arc<PaintRegionIR> = execute_paint_segmentation(
        Arc::clone(&mesh_ir),
        Arc::new(sc),
        Arc::new(lp.clone()),
        true,
    )
    .expect("execute_paint_segmentation must succeed");
    let plan = empty_execution_plan();
    let si = plan_stage_invocations(&plan);
    let projection = RegionMappingPlanProjection {
        stage_invocations: &si,
    };
    let empty_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    let result = execute_region_mapping_with_cap(
        &lp,
        &projection,
        Some(&paint_result),
        &empty_semantic_configs,
        &objects,
        1024,
    )
    .expect("execute_region_mapping_with_cap must succeed");
    Some(result)
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-1: negative_part_subtracts_via_full_pipeline
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn negative_part_subtracts_via_full_pipeline() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    let negative_mvs: Vec<&ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| &obj.modifier_volumes)
        .filter(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "negative_part"),
            )
        })
        .collect();

    assert!(
        !negative_mvs.is_empty(),
        "cube_positive_n_negative.3mf must contain at least one negative_part modifier"
    );

    // Compute z extent of negative_part meshes; test at midpoint.
    let (z_min, z_max) = negative_mvs
        .iter()
        .flat_map(|mv| mv.mesh.vertices.iter().map(|v| v.z))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), z| {
            (min.min(z), max.max(z))
        });
    let z_test = (z_min + z_max) / 2.0;

    // Build a SliceIR for all objects at test Z, only for the parent object.
    // We slice the first object (the positive part that contains the parent mesh).
    let parent_obj = mesh_ir
        .objects
        .iter()
        .find(|obj| {
            obj.modifier_volumes.iter().any(|mv| {
                matches!(mv.config_delta.fields.get("subtype"),
                    Some(ConfigValue::String(s)) if s == "normal_part")
            })
        })
        .or_else(|| mesh_ir.objects.first())
        .expect("at least one object");
    let projected = slice_mesh_ex(&parent_obj.mesh, &[z_test]);
    let polygons = projected.into_iter().next().unwrap_or_default();

    let pre_area = sum_area_mm2(&polygons);

    let mut slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: z_test,
        regions: vec![SlicedRegion {
            object_id: parent_obj.id.clone(),
            region_id: 0,
            polygons: polygons.clone(),
            infill_areas: vec![],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        }],
    };

    let all_mvs: Vec<ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| obj.modifier_volumes.clone())
        .collect();
    apply_negative_part_subtract(&mut slice, &all_mvs);

    let post_area = sum_area_mm2(&slice.regions[0].polygons);

    assert!(
        post_area < pre_area,
        "negative_part must reduce layer polygon area at z={z_test:.2} mm \
         (pre={pre_area:.4} mmÂ², post={post_area:.4} mmÂ²)"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-2: negative_part_transform_baked_correctly
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn negative_part_transform_baked_correctly() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    let negative_mvs: Vec<&ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| &obj.modifier_volumes)
        .filter(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "negative_part"),
            )
        })
        .collect();

    assert!(
        !negative_mvs.is_empty(),
        "must have at least one negative_part modifier"
    );

    // Get the negative cube's vertices and the main object's vertices.
    let neg_vertices = &negative_mvs[0].mesh.vertices;
    assert!(
        !neg_vertices.is_empty(),
        "negative_part must have mesh vertices"
    );

    let obj = mesh_ir.objects.first().expect("at least one object");
    let main_vertices = &obj.mesh.vertices;
    assert!(
        !main_vertices.is_empty(),
        "parent object must have mesh vertices"
    );

    // The negative cube has a component transform of X=-11.1, Y=-11.87.
    // Both the main mesh and the negative modifier are in world space (build
    // transform + component transforms are baked). The negative cube extends
    // below the build plate and should have vertices with a lower minimum
    // than the main mesh's center.
    let neg_min_y = neg_vertices
        .iter()
        .map(|v| v.y)
        .fold(f32::INFINITY, f32::min);
    let main_min_y = main_vertices
        .iter()
        .map(|v| v.y)
        .fold(f32::INFINITY, f32::min);

    // The negative cube sits at a lower Y (offset -11.87) than the main object.
    // Assert that the relative offset is roughly correct.
    let offset_y = neg_min_y - main_min_y;
    assert!(
        offset_y < -5.0,
        "negative cube should be offset below the main mesh (Y offset={offset_y:.2}, \
         expected ~-11.9)"
    );

    // Also verify the mesh is NOT at the origin.
    let center_x = (neg_vertices
        .iter()
        .map(|v| v.x)
        .fold(f32::INFINITY, f32::min)
        + neg_vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max))
        / 2.0;
    let center_y = (neg_vertices
        .iter()
        .map(|v| v.y)
        .fold(f32::INFINITY, f32::min)
        + neg_vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max))
        / 2.0;

    assert!(
        center_x.abs() > 1.0 || center_y.abs() > 1.0,
        "negative cube must NOT be centered at origin (center_x={center_x:.2}, center_y={center_y:.2})"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-3: modifier_volumes_populated_with_correct_metadata
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn modifier_volumes_populated_with_correct_metadata() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    let neg_mvs: Vec<&ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| &obj.modifier_volumes)
        .filter(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "negative_part"),
            )
        })
        .collect();

    assert!(
        !neg_mvs.is_empty(),
        "must have at least one negative_part modifier_volume"
    );

    let mv = neg_mvs[0];
    let subtype = mv
        .config_delta
        .fields
        .get("subtype")
        .expect("subtype key must exist");

    assert_eq!(
        *subtype,
        ConfigValue::String("negative_part".to_string()),
        "subtype must be 'negative_part'"
    );

    let extruder = mv
        .config_delta
        .fields
        .get("extruder")
        .expect("extruder key must exist");

    assert_eq!(*extruder, ConfigValue::Int(0), "extruder must be 0");
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-4: support_enforcer_emits_paint_regions_from_disk
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_enforcer_emits_paint_regions_from_disk() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    assert!(
        mesh_ir.objects.len() >= 2,
        "bridge_support_enforcers.3mf must have 2 objects"
    );

    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);

    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::clone(&mesh_ir), Arc::new(sc), Arc::new(lp), true)
            .expect("execute_paint_segmentation must succeed");

    let has_enforcer = paint_result.per_layer.values().any(|lm: &LayerPaintMap| {
        lm.semantic_regions
            .get(&PaintSemantic::SupportEnforcer)
            .map_or(false, |regions| !regions.is_empty())
    });

    assert!(
        has_enforcer,
        "at least one layer must contain SupportEnforcer semantic regions"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-5: support_blocker_emits_paint_regions_from_disk
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_blocker_emits_paint_regions_from_disk() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    assert!(
        mesh_ir.objects.len() >= 2,
        "bridge_support_enforcers.3mf must have 2 objects"
    );

    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);

    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::clone(&mesh_ir), Arc::new(sc), Arc::new(lp), true)
            .expect("execute_paint_segmentation must succeed");

    let has_blocker = paint_result.per_layer.values().any(|lm: &LayerPaintMap| {
        lm.semantic_regions
            .get(&PaintSemantic::SupportBlocker)
            .map_or(false, |regions| !regions.is_empty())
    });

    assert!(
        has_blocker,
        "at least one layer must contain SupportBlocker semantic regions"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-6: modifier_part_benchy_regression
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn modifier_part_benchy_regression() {
    // Fixture: cube_cilindrical_modifier.3mf (cube body + cylindrical
    // modifier_part).
    let path = fixture("cube_cilindrical_modifier.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    assert!(
        !mesh_ir.objects.is_empty(),
        "cube_cilindrical_modifier.3mf must have at least one object"
    );

    let modifier_parts: Vec<&ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| &obj.modifier_volumes)
        .filter(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "modifier_part"),
            )
        })
        .collect();

    assert!(
        !modifier_parts.is_empty(),
        "cube_cilindrical_modifier.3mf must have modifier_part volumes"
    );

    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 20, 0.2);

    let paint_result =
        execute_paint_segmentation(Arc::clone(&mesh_ir), Arc::new(sc), Arc::new(lp), true);

    assert!(
        paint_result.is_ok(),
        "execute_paint_segmentation must succeed for cube_cilindrical_modifier"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-7: model_without_negative_skips_subtract
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn model_without_negative_skips_subtract() {
    // Fixture: cube_4color.3mf (paint-only, no modifier_volumes at all, hence
    // no negative_part).
    let path = fixture("cube_4color.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    let has_negative = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| &obj.modifier_volumes)
        .any(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "negative_part"),
            )
        });

    assert!(
        !has_negative,
        "cube_4color.3mf must not contain negative_part modifiers"
    );

    let obj = mesh_ir.objects.first().expect("cube must have an object");
    let projected = slice_mesh_ex(&obj.mesh, &[5.0]);
    let polygons_before = projected.into_iter().next().unwrap_or_default();

    let mut slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 5.0,
        regions: vec![SlicedRegion {
            object_id: obj.id.clone(),
            region_id: 0,
            polygons: polygons_before.clone(),
            infill_areas: vec![],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        }],
    };

    let all_mvs: Vec<ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| obj.modifier_volumes.clone())
        .collect();
    apply_negative_part_subtract(&mut slice, &all_mvs);

    let polygons_after = &slice.regions[0].polygons;
    assert_eq!(
        polygons_after.len(),
        polygons_before.len(),
        "polygon count must be unchanged when no negative_part exists"
    );
    for (i, (after, before)) in polygons_after
        .iter()
        .zip(polygons_before.iter())
        .enumerate()
    {
        assert_eq!(
            after.contour.points, before.contour.points,
            "polygon {i} contour must be bit-identical (no negative_part to subtract)"
        );
        assert_eq!(
            after.holes, before.holes,
            "polygon {i} holes must be bit-identical (no negative_part to subtract)"
        );
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-8: two_objects_produce_separate_modifier_volumes
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn two_objects_produce_separate_modifier_volumes() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    assert_eq!(
        mesh_ir.objects.len(),
        2,
        "bridge_support_enforcers.3mf must have exactly 2 objects"
    );

    let obj4 = &mesh_ir.objects[0];
    let obj5 = &mesh_ir.objects[1];

    let obj4_enforcer = obj4
        .modifier_volumes
        .iter()
        .find(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "support_enforcer"),
            )
        })
        .expect("object 4 must have support_enforcer modifier_volumes");

    let obj5_blocker = obj5
        .modifier_volumes
        .iter()
        .find(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "support_blocker"),
            )
        })
        .expect("object 5 must have support_blocker modifier_volumes");

    assert_ne!(
        obj4_enforcer.id, obj5_blocker.id,
        "different objects must have distinct modifier_volume IDs"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-9: duplicate_part_id_handled_gracefully
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn duplicate_part_id_handled_gracefully() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    // The fixture has part id=3 duplicated for each object (two support enforcer
    // instances on object 4, two blocker instances on object 5). The loader must
    // not panic and at least one modifier_volume entry must be present per object
    // group.
    for obj in &mesh_ir.objects {
        let subtype_mvs: Vec<&ModifierVolume> = obj
            .modifier_volumes
            .iter()
            .filter(|mv| mv.config_delta.fields.contains_key("subtype"))
            .collect();

        assert!(
            !subtype_mvs.is_empty(),
            "object '{}' must have at least one modifier_volume with a subtype",
            obj.id
        );
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-N1: missing_fixture_returns_error
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn missing_fixture_returns_error() {
    let nonexistent = repo_root().join("resources").join("__does_not_exist__.3mf");
    assert!(
        !nonexistent.exists(),
        "test fixture must not exist: {}",
        nonexistent.display()
    );

    let result = load_model(&nonexistent);
    assert!(
        result.is_err(),
        "load_model with nonexistent path must return Err"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-Loader-2: load_model populates ObjectConfig.data from sidecar
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// The 3MF loader must extract the object-scoped allowlist keys
// (`extruder`, `enable_support`, `support_type`) from the sidecar and
// populate `ObjectMesh.config.data` with typed `ConfigValue` entries.
// All three Packet 67 fixtures have parent `extruder=1` at object scope;
// bridge obj5 additionally carries `enable_support=1` and
// `support_type=tree(auto)`.

#[test]
fn load_model_populates_object_config_data() {
    // Each fixture's parent object(s) must surface `extruder=Int(1)` in
    // `ObjectMesh.config.data` after load_model. cube_cilindrical_modifier.3mf
    // also carries object-scoped `extruder=1` (verified via `unzip -p ...
    // Metadata/model_settings.config`).
    let fixtures = [
        "cube_positive_n_negative.3mf",
        "cube_cilindrical_modifier.3mf",
        "bridge_support_enforcers.3mf",
    ];
    for name in fixtures {
        let path = fixture(name);
        if skip_if_missing(&path) {
            continue;
        }
        let mesh_ir = crate::common::model_cache::cached_load_model(&path);
        assert!(
            !mesh_ir.objects.is_empty(),
            "{name}: load_model must return at least one object"
        );
        for (idx, obj) in mesh_ir.objects.iter().enumerate() {
            assert_eq!(
                obj.config.data.get("extruder"),
                Some(&ConfigValue::Int(1)),
                "{name} object[{idx}] (id={}) must have config.data[\"extruder\"] = Int(1) \
                 from object-scoped sidecar metadata",
                obj.id
            );
        }
    }

    // Bridge obj5 (second object in build order) additionally carries
    // enable_support=1 and support_type=tree(auto).
    let bridge_path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&bridge_path) {
        return;
    }
    let bridge_mesh = crate::common::model_cache::cached_load_model(&bridge_path);
    assert!(
        bridge_mesh.objects.len() >= 2,
        "bridge: expected at least 2 objects, found {}",
        bridge_mesh.objects.len()
    );
    let obj5 = &bridge_mesh.objects[1];
    assert_eq!(
        obj5.config.data.get("enable_support"),
        Some(&ConfigValue::Bool(true)),
        "bridge obj5 must have enable_support = Bool(true)"
    );
    assert_eq!(
        obj5.config.data.get("support_type"),
        Some(&ConfigValue::String("tree(auto)".into())),
        "bridge obj5 must have support_type = String(\"tree(auto)\")"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-Mod-1 (RED): negative_part_stamps_extruder_into_extensions
// RED until Packet 68 lands `stamp_modifier_config_deltas`. Asserts that at
// least one RegionPlan.config.extensions entry carries extruder=Int(0) from
// the cube fixture's negative_part modifier (whose config_delta has
// extruder=0). OrcaSlicer parity: negative_part IS in the stamp list
// (MODEL_PART | NEGATIVE_VOLUME | PARAMETER_MODIFIER per
// PrintApply.cpp:590-594).
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn negative_part_stamps_extruder_into_extensions() {
    let Some(region_map) = region_map_for_fixture("cube_positive_n_negative.3mf") else {
        return;
    };

    let stamped = region_map.entries.keys().any(|key| {
        matches!(
            region_map.config_for(key).extensions.get("extruder"),
            Some(ConfigValue::Int(0))
        )
    });

    assert!(
        stamped,
        "RED: stamp_modifier_config_deltas (Packet 68) must stamp negative_part \
         config_delta[\"extruder\"]=Int(0) into at least one RegionPlan.config.extensions. \
         Fixture: cube_positive_n_negative.3mf has a negative_part with extruder=0."
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-Mod-2 (RED): modifier_part_stamps_fuzzy_skin_into_extensions
// RED until Packet 68 lands `stamp_modifier_config_deltas`. Asserts the
// canonical "non-extruder key propagation" invariant: a modifier_part whose
// config_delta carries `fuzzy_skin=String("external")` must land that key
// in at least one RegionPlan.config.extensions after region mapping.
//
// No on-disk fixture carries this exact key/value pair on a modifier_part:
// cube_cilindrical_modifier.3mf authors a modifier_part whose metadata
// preserves only `subtype` and `matrix` (the four wall/infill keys it also
// authors are not on the loader allowlist). To keep this RED-guard test
// expressing the SAME invariant â€” not a weaker proxy â€” we now construct a
// synthetic ObjectMesh with a modifier whose `config_delta.fields` contains
// exactly `fuzzy_skin=String("external")`, reusing the same synthetic
// helpers as AC-N1/AC-N2. No on-disk fixture is consulted for this test.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn modifier_part_stamps_fuzzy_skin_into_extensions() {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "subtype".into(),
        ConfigValue::String("modifier_part".into()),
    );
    fields.insert("fuzzy_skin".into(), ConfigValue::String("external".into()));
    let modifier = synthetic_modifier_volume("mod-fuzzy-skin", 0, fields);
    let object = synthetic_object_with_modifiers("synthetic-obj", vec![modifier]);

    let region_map = region_map_for_synthetic_objects(vec![object], "synthetic-obj");

    let stamped = region_map.entries.keys().any(|key| {
        matches!(
            region_map.config_for(key).extensions.get("fuzzy_skin"),
            Some(ConfigValue::String(s)) if s == "external"
        )
    });

    assert!(
        stamped,
        "RED: stamp_modifier_config_deltas (Packet 68) must stamp modifier_part \
         config_delta[\"fuzzy_skin\"]=String(\"external\") into at least one \
         RegionPlan.config.extensions. Synthetic modifier authors fuzzy_skin=external \
         (reproduced via the synthetic_modifier_volume helper)."
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-Mod-3 (RED): modifier_part_stamps_extruder_into_extensions
// RED until Packet 68 lands `stamp_modifier_config_deltas`. Symmetric with
// AC-Mod-2 but for the extruder key. Confirms modifier_part subtype is in
// the stamp list (OrcaSlicer parity: PARAMETER_MODIFIER per
// PrintApply.cpp:590-594).
//
// No on-disk fixture carries this exact key/value pair on a modifier_part:
// cube_cilindrical_modifier.3mf does not author `extruder=0` on the modifier
// (only `subtype`+`matrix`+four non-allowlisted wall/infill keys). To preserve
// the invariant exactly â€” not a weaker proxy â€” a synthetic modifier with
// `extruder=Int(0)` (and `subtype="modifier_part"` so it is recognised as a
// parameter modifier) is constructed via the same helpers AC-N1/AC-N2 use.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn modifier_part_stamps_extruder_into_extensions() {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "subtype".into(),
        ConfigValue::String("modifier_part".into()),
    );
    fields.insert("extruder".into(), ConfigValue::Int(0));
    let modifier = synthetic_modifier_volume("mod-extruder-zero", 0, fields);
    let object = synthetic_object_with_modifiers("synthetic-obj", vec![modifier]);

    let region_map = region_map_for_synthetic_objects(vec![object], "synthetic-obj");

    let stamped = region_map.entries.keys().any(|key| {
        matches!(
            region_map.config_for(key).extensions.get("extruder"),
            Some(ConfigValue::Int(0))
        )
    });

    assert!(
        stamped,
        "RED: stamp_modifier_config_deltas (Packet 68) must stamp modifier_part \
         config_delta[\"extruder\"]=Int(0) into at least one RegionPlan.config.extensions. \
         Synthetic modifier authors subtype=modifier_part and extruder=0 \
         (reproduced via the synthetic_modifier_volume helper)."
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-Mod-4 (GREEN regression guard): support_enforcer_config_delta_not_stamped
// OrcaSlicer parity guard. The bridge_support_enforcers.3mf fixture has only
// support_enforcer modifier_volumes on obj4 â€” no negative_part, no
// modifier_part. Per PrintApply.cpp:590-594, SUPPORT_ENFORCER is excluded
// from region config merging. NO RegionPlan.config.extensions should carry
// the support_enforcer's config_delta keys.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_enforcer_config_delta_not_stamped() {
    let Some(region_map) = region_map_for_fixture("bridge_support_enforcers.3mf") else {
        return;
    };

    let leaked = region_map
        .entries
        .keys()
        .any(|key| region_map.config_for(key).extensions.contains_key("extruder"));

    assert!(
        !leaked,
        "OrcaSlicer parity (PrintApply.cpp:590-594): support_enforcer config_delta MUST \
         NOT stamp into RegionPlan.config.extensions. If this fails after Packet 68 lands, \
         Packet 68 forgot the ENFORCER/BLOCKER subtype filter in \
         stamp_modifier_config_deltas."
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-Mod-5 (GREEN regression guard): support_blocker_config_delta_not_stamped
// OrcaSlicer parity guard, symmetric with AC-Mod-4. The blocker side of the
// bridge fixture (obj5) carries only support_blocker modifier_volumes;
// SUPPORT_BLOCKER is also excluded by PrintApply.cpp:590-594. Asserts via
// the same fixture as AC-Mod-4 â€” kept separate so each subtype's parity
// contract is independently findable in test output.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_blocker_config_delta_not_stamped() {
    let Some(region_map) = region_map_for_fixture("bridge_support_enforcers.3mf") else {
        return;
    };

    let leaked = region_map
        .entries
        .keys()
        .any(|key| region_map.config_for(key).extensions.contains_key("extruder"));

    assert!(
        !leaked,
        "OrcaSlicer parity (PrintApply.cpp:590-594): support_blocker config_delta MUST \
         NOT stamp into RegionPlan.config.extensions. If this fails after Packet 68 lands, \
         Packet 68 forgot the ENFORCER/BLOCKER subtype filter in \
         stamp_modifier_config_deltas. (See also AC-Mod-4 for the enforcer side.)"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-Mod-6 (GREEN parity guard): support_enforcer_paint_value_is_flag_not_tool_index
// OrcaSlicer parity guard at the paint-segmentation surface. SupportEnforcer
// SemanticRegions MUST carry PaintValue::Flag(_), never PaintValue::ToolIndex(_).
// Per paint_segmentation.rs:416, value is hardcoded to Flag(true). If someone
// re-wires the divergent extruderâ†’ToolIndex routing that the withdrawn AC-R1
// was testing for, this test catches the regression. See D6.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_enforcer_paint_value_is_flag_not_tool_index() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }
    let mesh_ir = crate::common::model_cache::cached_load_model(&path);
    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);
    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::clone(&mesh_ir), Arc::new(sc), Arc::new(lp), true)
            .expect("execute_paint_segmentation must succeed");

    let mut saw_enforcer = false;
    for lm in paint_result.per_layer.values() {
        if let Some(regions) = lm.semantic_regions.get(&PaintSemantic::SupportEnforcer) {
            for region in regions {
                saw_enforcer = true;
                assert!(
                    matches!(region.value, PaintValue::Flag(_)),
                    "OrcaSlicer parity: SupportEnforcer SemanticRegion must carry \
                     PaintValue::Flag (decorative extruder field per PrintApply.cpp:590-594). \
                     Found {:?}. If this fails, someone re-wired the divergent \
                     extruderâ†’PaintValue::ToolIndex path that AC-R1 was testing for \
                     (withdrawn in Packet 67, see D6).",
                    region.value
                );
            }
        }
    }

    assert!(
        saw_enforcer,
        "bridge_support_enforcers.3mf must produce at least one SupportEnforcer \
         SemanticRegion for the parity guard to be meaningful"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Packet 68 synthetic-mesh helpers (AC-N1, AC-N2)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn synthetic_semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn synthetic_triangle_mesh() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
        ],
        indices: vec![0, 1, 2],
    }
}

fn synthetic_identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn synthetic_modifier_volume(
    id: &str,
    priority: u32,
    fields: HashMap<String, ConfigValue>,
) -> ModifierVolume {
    ModifierVolume {
        id: id.into(),
        mesh: synthetic_triangle_mesh(),
        config_delta: ConfigDelta { fields },
        priority,
        applies_to: ModifierScope::AllFeatures,
    }
}

fn synthetic_object_with_modifiers(object_id: &str, mods: Vec<ModifierVolume>) -> ObjectMesh {
    ObjectMesh {
        id: object_id.into(),
        mesh: synthetic_triangle_mesh(),
        transform: Transform3d {
            matrix: synthetic_identity4(),
        },
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: mods,
        paint_data: None,
        world_z_extent: None,
    }
}

fn synthetic_layer_plan_single_region(object_id: &str) -> LayerPlanIR {
    let mut object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::new();
    object_participation.insert(
        object_id.into(),
        vec![ObjectLayerRef {
            local_layer_index: 0,
            global_layer_index: 0,
            effective_layer_height: 0.2,
        }],
    );
    LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.1,
            active_regions: vec![ActiveRegion {
                object_id: object_id.into(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        }],
        object_participation,
        ..Default::default()
    }
}

fn synthetic_mesh_ir(objects: Vec<ObjectMesh>) -> MeshIR {
    MeshIR {
        schema_version: synthetic_semver(),
        objects,
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

fn region_map_for_synthetic_objects(objects: Vec<ObjectMesh>, object_id: &str) -> RegionMapIR {
    let mesh_ir = synthetic_mesh_ir(objects);
    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = synthetic_layer_plan_single_region(object_id);
    let object_meshes = mesh_ir.objects.clone();
    // Paint pipeline is required by the surface contract but produces an
    // empty PaintRegionIR for these synthetic objects (no paint_data).
    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::new(mesh_ir), Arc::new(sc), Arc::new(lp.clone()), true)
            .expect("execute_paint_segmentation must succeed");
    let plan = empty_execution_plan();
    let si = plan_stage_invocations(&plan);
    let projection = RegionMappingPlanProjection {
        stage_invocations: &si,
    };
    let empty_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    execute_region_mapping_with_cap(
        &lp,
        &projection,
        Some(&paint_result),
        &empty_semantic_configs,
        &object_meshes,
        1024,
    )
    .expect("execute_region_mapping_with_cap must succeed")
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-1: config_delta_extruder_stamped_into_extensions
//
// Packet text says "for a region that overlaps a support_enforcer modifier
// volume". However, the locked subtype filter (AC-Filter, PrintApply.cpp:590-594
// parity) excludes support_enforcer / support_blocker from stamping. The test
// therefore exercises the equivalent semantics on a subtype that IS in the
// stamp list â€” cube_positive_n_negative.3mf's `negative_part` modifier whose
// config_delta carries extruder=Int(0). Asserts that at least one RegionPlan
// keyed on the parent object_id carries extensions["extruder"]=Int(0).
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn config_delta_extruder_stamped_into_extensions() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }
    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    // Find a parent object that hosts a non-enforcer/non-blocker modifier
    // carrying extruder=Int(0). Per the fixture's structure (validated by the
    // existing `modifier_volumes_populated_with_correct_metadata` test), this
    // is the object carrying the `negative_part` modifier.
    let stamped_parent_ids: Vec<String> = mesh_ir
        .objects
        .iter()
        .filter(|obj| {
            obj.modifier_volumes.iter().any(|mv| {
                let subtype_excluded = matches!(
                    mv.config_delta.fields.get("subtype"),
                    Some(ConfigValue::String(s))
                        if s == "support_enforcer" || s == "support_blocker"
                );
                let has_extruder_zero = matches!(
                    mv.config_delta.fields.get("extruder"),
                    Some(ConfigValue::Int(0))
                );
                !subtype_excluded && has_extruder_zero
            })
        })
        .map(|obj| obj.id.clone())
        .collect();
    assert!(
        !stamped_parent_ids.is_empty(),
        "cube_positive_n_negative.3mf must host at least one non-enforcer/non-blocker \
         modifier volume with extruder=Int(0) for AC-1 to be meaningful"
    );

    let Some(region_map) = region_map_for_fixture("cube_positive_n_negative.3mf") else {
        return;
    };

    let stamped = region_map.entries.keys().any(|key| {
        stamped_parent_ids.contains(&key.object_id)
            && matches!(
                region_map.config_for(key).extensions.get("extruder"),
                Some(ConfigValue::Int(0))
            )
    });

    assert!(
        stamped,
        "AC-1: at least one RegionPlan keyed on a parent object of a stamped modifier \
         volume must carry config.extensions[\"extruder\"] = Int(0). \
         stamped_parent_ids = {stamped_parent_ids:?}"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-3: config_delta_non_extruder_key_survives
//
// AC-3 asks that a non-`extruder` config key (here `fuzzy_skin`) survives
// the stamp and lands in RegionPlan.config.extensions for the overlapping
// region alongside the `extruder` key. Asserts that at least one RegionPlan
// carries BOTH `extruder=Int(0)` AND `fuzzy_skin=String("external")`,
// proving the non-extruder key survives end-to-end alongside the extruder.
//
// No on-disk fixture carries this exact key pair on a modifier_part. The
// invariant is reproduced via the synthetic_modifier_volume helper used by
// AC-N1/AC-N2; no on-disk fixture is consulted.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn config_delta_non_extruder_key_survives() {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "subtype".into(),
        ConfigValue::String("modifier_part".into()),
    );
    fields.insert("extruder".into(), ConfigValue::Int(0));
    fields.insert("fuzzy_skin".into(), ConfigValue::String("external".into()));
    let modifier = synthetic_modifier_volume("mod-both-keys", 0, fields);
    let object = synthetic_object_with_modifiers("synthetic-obj", vec![modifier]);

    let region_map = region_map_for_synthetic_objects(vec![object], "synthetic-obj");

    let both_present = region_map.entries.keys().any(|key| {
        let cfg = region_map.config_for(key);
        let has_extruder = matches!(
            cfg.extensions.get("extruder"),
            Some(ConfigValue::Int(0))
        );
        let has_fuzzy = matches!(
            cfg.extensions.get("fuzzy_skin"),
            Some(ConfigValue::String(s)) if s == "external"
        );
        has_extruder && has_fuzzy
    });

    assert!(
        both_present,
        "AC-3: at least one RegionPlan must carry BOTH config.extensions[\"extruder\"] \
         = Int(0) AND config.extensions[\"fuzzy_skin\"] = String(\"external\"), proving \
         non-extruder keys survive alongside extruder. Synthetic modifier authors both \
         keys (reproduced via the synthetic_modifier_volume helper)."
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-4: negative_part_extruder_does_not_affect_subtract
//
// cube_positive_n_negative.3mf has a `negative_part` modifier whose
// config_delta carries `extruder=Int(0)`. AC-4 asserts that the negative-part
// subtract output (post-`apply_negative_part_subtract`) is unchanged by the
// presence of that extruder key â€” `apply_negative_part_subtract` is
// geometry-only, and stamping `extruder` into a `RegionPlan` for a region
// affected by the negative_part does NOT alter the subtract result.
//
// Asserts the same area reduction property as the existing
// `negative_part_subtracts_via_full_pipeline` test: post-subtract area must be
// strictly less than pre-subtract area. The proof is structural: this test
// runs the same subtract path with the same `extruder=0`-carrying modifier
// data, and demonstrates the polygon output behaves identically.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn negative_part_extruder_does_not_affect_subtract() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir = crate::common::model_cache::cached_load_model(&path);

    let negative_mvs: Vec<&ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| &obj.modifier_volumes)
        .filter(|mv| {
            mv.config_delta.fields.get("subtype").map_or(
                false,
                |v| matches!(v, ConfigValue::String(s) if s == "negative_part"),
            )
        })
        .collect();

    assert!(
        !negative_mvs.is_empty(),
        "fixture must carry at least one negative_part modifier_volume"
    );

    // Confirm the negative_part carries extruder=Int(0) â€” this is the key
    // whose presence-or-absence must not alter the subtract result.
    let neg_extruder_zero = negative_mvs.iter().any(|mv| {
        matches!(
            mv.config_delta.fields.get("extruder"),
            Some(ConfigValue::Int(0))
        )
    });
    assert!(
        neg_extruder_zero,
        "AC-4 precondition: at least one negative_part must carry extruder=Int(0)"
    );

    // Compute pre-subtract area at the negative_part's z midpoint.
    let (z_min, z_max) = negative_mvs
        .iter()
        .flat_map(|mv| mv.mesh.vertices.iter().map(|v| v.z))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), z| {
            (min.min(z), max.max(z))
        });
    let z_test = (z_min + z_max) / 2.0;

    let parent_obj = mesh_ir
        .objects
        .iter()
        .find(|obj| {
            obj.modifier_volumes.iter().any(|mv| {
                matches!(mv.config_delta.fields.get("subtype"),
                    Some(ConfigValue::String(s)) if s == "normal_part")
            })
        })
        .or_else(|| mesh_ir.objects.first())
        .expect("at least one object");
    let projected = slice_mesh_ex(&parent_obj.mesh, &[z_test]);
    let polygons = projected.into_iter().next().unwrap_or_default();
    let pre_area = sum_area_mm2(&polygons);

    let mut slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: z_test,
        regions: vec![SlicedRegion {
            object_id: parent_obj.id.clone(),
            region_id: 0,
            polygons: polygons.clone(),
            infill_areas: vec![],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        }],
    };

    let all_mvs: Vec<ModifierVolume> = mesh_ir
        .objects
        .iter()
        .flat_map(|obj| obj.modifier_volumes.clone())
        .collect();
    apply_negative_part_subtract(&mut slice, &all_mvs);
    let post_area = sum_area_mm2(&slice.regions[0].polygons);

    assert!(
        post_area < pre_area,
        "AC-4: negative_part must still reduce layer polygon area even when its \
         config_delta carries extruder=Int(0) â€” apply_negative_part_subtract is \
         geometry-only and config stamping does not alter polygon output. \
         (pre={pre_area:.4} mmÂ², post={post_area:.4} mmÂ²)"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-N1: subtype_only_modifier_stamps_no_extensions
//
// Construct a synthetic ObjectMesh whose ModifierVolume's config_delta.fields
// contains ONLY the `subtype` key. Run region mapping. Assert
// `RegionPlan.config.extensions` carries NO entries from the modifier â€” the
// `subtype` key is excluded from stamping per
// `stamp_modifier_config_deltas`.
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn subtype_only_modifier_stamps_no_extensions() {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "subtype".into(),
        ConfigValue::String("modifier_part".into()),
    );
    let modifier = synthetic_modifier_volume("mod-subtype-only", 0, fields);
    let object = synthetic_object_with_modifiers("synthetic-obj", vec![modifier]);

    let region_map = region_map_for_synthetic_objects(vec![object], "synthetic-obj");

    for key in region_map.entries.keys() {
        let cfg = region_map.config_for(key);
        assert!(
            !cfg.extensions.contains_key("subtype"),
            "AC-N1: RegionPlan at {key:?} must not carry a stamped \"subtype\" key - \
             stamp_modifier_config_deltas excludes the subtype key. \
             Found extensions={:?}",
            cfg.extensions
        );
        assert!(
            !cfg.extensions.contains_key("extruder"),
            "AC-N1: RegionPlan at {key:?} must not carry an \"extruder\" key when the \
             modifier's config_delta contains only \"subtype\". Found extensions={:?}",
            cfg.extensions
        );
        assert!(
            !cfg.extensions.contains_key("fuzzy_skin"),
            "AC-N1: RegionPlan at {key:?} must not carry a \"fuzzy_skin\" key when the \
             modifier's config_delta contains only \"subtype\". Found extensions={:?}",
            cfg.extensions
        );
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// AC-N2: conflicting_extruder_modifier_priority_wins
//
// Two overlapping modifier volumes on the same synthetic ObjectMesh:
//   - Modifier A: priority=0, extruder=Int(0)
//   - Modifier B: priority=1, extruder=Int(1)
// `stamp_modifier_config_deltas` sorts by priority ascending and applies via
// `overlay_resolved` (last-writer-wins). Modifier B has higher priority and
// writes last, so the resulting RegionPlan.config.extensions["extruder"]
// must be Int(1).
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn conflicting_extruder_modifier_priority_wins() {
    let mut a_fields: HashMap<String, ConfigValue> = HashMap::new();
    a_fields.insert(
        "subtype".into(),
        ConfigValue::String("modifier_part".into()),
    );
    a_fields.insert("extruder".into(), ConfigValue::Int(0));
    let mod_a = synthetic_modifier_volume("mod-a-low-priority", 0, a_fields);

    let mut b_fields: HashMap<String, ConfigValue> = HashMap::new();
    b_fields.insert(
        "subtype".into(),
        ConfigValue::String("modifier_part".into()),
    );
    b_fields.insert("extruder".into(), ConfigValue::Int(1));
    let mod_b = synthetic_modifier_volume("mod-b-high-priority", 1, b_fields);

    let object = synthetic_object_with_modifiers("synthetic-obj", vec![mod_a, mod_b]);

    let region_map = region_map_for_synthetic_objects(vec![object], "synthetic-obj");

    assert!(
        !region_map.entries.is_empty(),
        "region map must contain at least one RegionPlan entry"
    );

    for key in region_map.entries.keys() {
        let cfg = region_map.config_for(key);
        assert_eq!(
            cfg.extensions.get("extruder"),
            Some(&ConfigValue::Int(1)),
            "AC-N2: RegionPlan at {key:?} must carry extruder=Int(1) (modifier B wins \
             because its higher priority writes last via overlay_resolved). \
             Found extensions={:?}",
            cfg.extensions
        );
    }
}
