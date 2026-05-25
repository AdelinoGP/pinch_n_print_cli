//! Packet 67: 3MF fixture end-to-end integration tests.
//!
//! Loads real on-disk 3MF fixtures through `load_model()` and exercises the full
//! pipeline: paint segmentation, negative-part subtract, and modifier-volume
//! metadata inspection.
//!
//! Expected: 11 GREEN tests pass, 1 RED test fails with a specific assertion message.
//! The RED test is documented with `// RED — passes after Packet 68` comments.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_core::slice_mesh_ex;
use slicer_host::model_loader::load_model;
use slicer_host::negative_part_subtract::apply_negative_part_subtract;
use slicer_host::paint_segmentation::execute_paint_segmentation;
use slicer_host::{
    build_execution_plan, execute_region_mapping_with_cap, ExecutionPlan, ExecutionPlanRequest,
};
use slicer_ir::{
    ActiveRegion, ConfigValue, ExPolygon, FacetClass, GlobalLayer, LayerPaintMap, LayerPlanIR,
    MeshIR, ModifierVolume, ObjectLayerRef, ObjectSurfaceData, PaintRegionIR, PaintSemantic,
    PaintValue, Polygon, RegionMapIR, ResolvedConfig, SliceIR, SlicedRegion,
    SurfaceClassificationIR, CURRENT_SLICE_IR_SCHEMA_VERSION,
};

// ─────────────────────────────────────────────────────────────────────────
// Path helpers
// ─────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────
// Area helpers (mirrored from threemf_subtypes_synthetic_e2e_tdd.rs)
// ─────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────
// Build helpers for paint segmentation with on-disk fixtures
// ─────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────
// region_map_for_fixture: shared scaffolding for AC-Mod-* tests.
// Loads a 3MF fixture and runs paint_segmentation + region_mapping. Returns
// None if the fixture is missing.
// ─────────────────────────────────────────────────────────────────────────

fn empty_execution_plan() -> ExecutionPlan {
    let req = ExecutionPlanRequest {
        sorted_stages: Vec::new(),
        module_bindings: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };
    build_execution_plan(&req).expect("empty execution plan should build")
}

fn region_map_for_fixture(name: &str) -> Option<RegionMapIR> {
    let path = fixture(name);
    if skip_if_missing(&path) {
        return None;
    }
    let mesh_ir: MeshIR =
        load_model(&path).unwrap_or_else(|e| panic!("load_model({name}) failed: {e:?}"));
    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);
    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::new(mesh_ir), Arc::new(sc), Arc::new(lp.clone()), true)
            .expect("execute_paint_segmentation must succeed");
    let plan = empty_execution_plan();
    let empty_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    let result = execute_region_mapping_with_cap(
        &lp,
        &plan,
        Some(&paint_result),
        &empty_semantic_configs,
        1024,
    )
    .expect("execute_region_mapping_with_cap must succeed");
    Some(result)
}

// ─────────────────────────────────────────────────────────────────────────
// AC-1: negative_part_subtracts_via_full_pipeline
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn negative_part_subtracts_via_full_pipeline() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load cube_positive_n_negative.3mf");

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
            boundary_paint: HashMap::new(),
            is_top_surface: false,
            is_bottom_surface: false,
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
         (pre={pre_area:.4} mm², post={post_area:.4} mm²)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// AC-2: negative_part_transform_baked_correctly
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn negative_part_transform_baked_correctly() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load cube_positive_n_negative.3mf");

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

// ─────────────────────────────────────────────────────────────────────────
// AC-3: modifier_volumes_populated_with_correct_metadata
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn modifier_volumes_populated_with_correct_metadata() {
    let path = fixture("cube_positive_n_negative.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load cube_positive_n_negative.3mf");

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

// ─────────────────────────────────────────────────────────────────────────
// AC-4: support_enforcer_emits_paint_regions_from_disk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn support_enforcer_emits_paint_regions_from_disk() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load bridge_support_enforcers.3mf");

    assert!(
        mesh_ir.objects.len() >= 2,
        "bridge_support_enforcers.3mf must have 2 objects"
    );

    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);

    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::new(mesh_ir), Arc::new(sc), Arc::new(lp), true)
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

// ─────────────────────────────────────────────────────────────────────────
// AC-5: support_blocker_emits_paint_regions_from_disk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn support_blocker_emits_paint_regions_from_disk() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load bridge_support_enforcers.3mf");

    assert!(
        mesh_ir.objects.len() >= 2,
        "bridge_support_enforcers.3mf must have 2 objects"
    );

    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);

    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::new(mesh_ir), Arc::new(sc), Arc::new(lp), true)
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

// ─────────────────────────────────────────────────────────────────────────
// AC-6: modifier_part_benchy_regression
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn modifier_part_benchy_regression() {
    let path = fixture("benchy_4color.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf");

    assert!(
        !mesh_ir.objects.is_empty(),
        "benchy_4color.3mf must have at least one object"
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
        "benchy_4color.3mf must have modifier_part volumes"
    );

    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 20, 0.2);

    let paint_result =
        execute_paint_segmentation(Arc::new(mesh_ir), Arc::new(sc), Arc::new(lp), true);

    assert!(
        paint_result.is_ok(),
        "execute_paint_segmentation must succeed for benchy_4color"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// AC-7: model_without_negative_skips_subtract
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn model_without_negative_skips_subtract() {
    let path = fixture("benchy_4color.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf");

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
        "benchy_4color.3mf must not contain negative_part modifiers"
    );

    let obj = mesh_ir.objects.first().expect("benchy must have an object");
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
            boundary_paint: HashMap::new(),
            is_top_surface: false,
            is_bottom_surface: false,
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

// ─────────────────────────────────────────────────────────────────────────
// AC-8: two_objects_produce_separate_modifier_volumes
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn two_objects_produce_separate_modifier_volumes() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load bridge_support_enforcers.3mf");

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

// ─────────────────────────────────────────────────────────────────────────
// AC-9: duplicate_part_id_handled_gracefully
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn duplicate_part_id_handled_gracefully() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }

    let mesh_ir: MeshIR = load_model(&path).expect("load bridge_support_enforcers.3mf");

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

// ─────────────────────────────────────────────────────────────────────────
// AC-N1: missing_fixture_returns_error
// ─────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────
// AC-Loader-2: load_model populates ObjectConfig.data from sidecar
// ─────────────────────────────────────────────────────────────────────────
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
    // `ObjectMesh.config.data` after load_model.
    let fixtures = [
        "cube_positive_n_negative.3mf",
        "benchy_4color.3mf",
        "bridge_support_enforcers.3mf",
    ];
    for name in fixtures {
        let path = fixture(name);
        if skip_if_missing(&path) {
            continue;
        }
        let mesh_ir: MeshIR =
            load_model(&path).unwrap_or_else(|e| panic!("load_model({name}) failed: {e:?}"));
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
    let bridge_mesh: MeshIR = load_model(&bridge_path).expect("load bridge");
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

// ─────────────────────────────────────────────────────────────────────────
// AC-Mod-1 (RED): negative_part_stamps_extruder_into_extensions
// RED until Packet 68 lands `stamp_modifier_config_deltas`. Asserts that at
// least one RegionPlan.config.extensions entry carries extruder=Int(0) from
// the cube fixture's negative_part modifier (whose config_delta has
// extruder=0). OrcaSlicer parity: negative_part IS in the stamp list
// (MODEL_PART | NEGATIVE_VOLUME | PARAMETER_MODIFIER per
// PrintApply.cpp:590-594).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn negative_part_stamps_extruder_into_extensions() {
    let Some(region_map) = region_map_for_fixture("cube_positive_n_negative.3mf") else {
        return;
    };

    let stamped = region_map.entries.values().any(|plan| {
        matches!(
            plan.config.extensions.get("extruder"),
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

// ─────────────────────────────────────────────────────────────────────────
// AC-Mod-2 (RED): modifier_part_stamps_fuzzy_skin_into_extensions
// RED until Packet 68 lands `stamp_modifier_config_deltas`. The only
// non-extruder config_delta key across all three Packet 67 fixtures is
// benchy_4color's modifier_part fuzzy_skin="external". This is the canonical
// "non-extruder key propagation" assertion.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn modifier_part_stamps_fuzzy_skin_into_extensions() {
    let Some(region_map) = region_map_for_fixture("benchy_4color.3mf") else {
        return;
    };

    let stamped = region_map.entries.values().any(|plan| {
        matches!(
            plan.config.extensions.get("fuzzy_skin"),
            Some(ConfigValue::String(s)) if s == "external"
        )
    });

    assert!(
        stamped,
        "RED: stamp_modifier_config_deltas (Packet 68) must stamp modifier_part \
         config_delta[\"fuzzy_skin\"]=String(\"external\") into at least one \
         RegionPlan.config.extensions. Fixture: benchy_4color.3mf has a modifier_part \
         with fuzzy_skin=external."
    );
}

// ─────────────────────────────────────────────────────────────────────────
// AC-Mod-3 (RED): modifier_part_stamps_extruder_into_extensions
// RED until Packet 68 lands `stamp_modifier_config_deltas`. Symmetric with
// AC-Mod-2 but for the extruder key. Confirms modifier_part subtype is in
// the stamp list (OrcaSlicer parity: PARAMETER_MODIFIER per
// PrintApply.cpp:590-594).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn modifier_part_stamps_extruder_into_extensions() {
    let Some(region_map) = region_map_for_fixture("benchy_4color.3mf") else {
        return;
    };

    let stamped = region_map.entries.values().any(|plan| {
        matches!(
            plan.config.extensions.get("extruder"),
            Some(ConfigValue::Int(0))
        )
    });

    assert!(
        stamped,
        "RED: stamp_modifier_config_deltas (Packet 68) must stamp modifier_part \
         config_delta[\"extruder\"]=Int(0) into at least one RegionPlan.config.extensions. \
         Fixture: benchy_4color.3mf has a modifier_part with extruder=0."
    );
}

// ─────────────────────────────────────────────────────────────────────────
// AC-Mod-4 (GREEN regression guard): support_enforcer_config_delta_not_stamped
// OrcaSlicer parity guard. The bridge_support_enforcers.3mf fixture has only
// support_enforcer modifier_volumes on obj4 — no negative_part, no
// modifier_part. Per PrintApply.cpp:590-594, SUPPORT_ENFORCER is excluded
// from region config merging. NO RegionPlan.config.extensions should carry
// the support_enforcer's config_delta keys.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn support_enforcer_config_delta_not_stamped() {
    let Some(region_map) = region_map_for_fixture("bridge_support_enforcers.3mf") else {
        return;
    };

    let leaked = region_map
        .entries
        .values()
        .any(|plan| plan.config.extensions.contains_key("extruder"));

    assert!(
        !leaked,
        "OrcaSlicer parity (PrintApply.cpp:590-594): support_enforcer config_delta MUST \
         NOT stamp into RegionPlan.config.extensions. If this fails after Packet 68 lands, \
         Packet 68 forgot the ENFORCER/BLOCKER subtype filter in \
         stamp_modifier_config_deltas."
    );
}

// ─────────────────────────────────────────────────────────────────────────
// AC-Mod-5 (GREEN regression guard): support_blocker_config_delta_not_stamped
// OrcaSlicer parity guard, symmetric with AC-Mod-4. The blocker side of the
// bridge fixture (obj5) carries only support_blocker modifier_volumes;
// SUPPORT_BLOCKER is also excluded by PrintApply.cpp:590-594. Asserts via
// the same fixture as AC-Mod-4 — kept separate so each subtype's parity
// contract is independently findable in test output.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn support_blocker_config_delta_not_stamped() {
    let Some(region_map) = region_map_for_fixture("bridge_support_enforcers.3mf") else {
        return;
    };

    let leaked = region_map
        .entries
        .values()
        .any(|plan| plan.config.extensions.contains_key("extruder"));

    assert!(
        !leaked,
        "OrcaSlicer parity (PrintApply.cpp:590-594): support_blocker config_delta MUST \
         NOT stamp into RegionPlan.config.extensions. If this fails after Packet 68 lands, \
         Packet 68 forgot the ENFORCER/BLOCKER subtype filter in \
         stamp_modifier_config_deltas. (See also AC-Mod-4 for the enforcer side.)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// AC-Mod-6 (GREEN parity guard): support_enforcer_paint_value_is_flag_not_tool_index
// OrcaSlicer parity guard at the paint-segmentation surface. SupportEnforcer
// SemanticRegions MUST carry PaintValue::Flag(_), never PaintValue::ToolIndex(_).
// Per paint_segmentation.rs:416, value is hardcoded to Flag(true). If someone
// re-wires the divergent extruder→ToolIndex routing that the withdrawn AC-R1
// was testing for, this test catches the regression. See D6.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn support_enforcer_paint_value_is_flag_not_tool_index() {
    let path = fixture("bridge_support_enforcers.3mf");
    if skip_if_missing(&path) {
        return;
    }
    let mesh_ir: MeshIR = load_model(&path).expect("load bridge_support_enforcers.3mf");
    let sc = surface_classification_for_mesh(&mesh_ir);
    let lp = layer_plan_for_mesh(&mesh_ir, 15, 0.2);
    let paint_result: Arc<PaintRegionIR> =
        execute_paint_segmentation(Arc::new(mesh_ir), Arc::new(sc), Arc::new(lp), true)
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
                     extruder→PaintValue::ToolIndex path that AC-R1 was testing for \
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
