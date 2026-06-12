//! Paint pipeline tests for `cube_fuzzyPainted.3mf` — migrated to v2 driver.
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
//! from hex subdivision) is lost because the v2 kernel requires `host-algos` feature
//! and sub-facet stroke support.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigDelta, ConfigValue, ExPolygon, FacetPaintData, GlobalLayer,
    IndexedTriangleSet, LayerPlanIR, MeshIR, ModifierScope, ModifierVolume, ObjectConfig,
    ObjectLayerRef, ObjectMesh, PaintLayer, PaintSemantic, PaintValue, Point2, Point3, Polygon,
    RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion, Transform3d,
    CURRENT_MESH_IR_SCHEMA_VERSION, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_model_io::load_model;

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

/// Return all SlicedRegions in `slice_ir` whose polygons contain at least one point
/// satisfying `predicate`. Used for positional assertions against a specific face.
fn regions_covering<'a>(
    slice_ir: &'a slicer_ir::SliceIR,
    predicate: impl Fn(Point2) -> bool,
) -> Vec<&'a SlicedRegion> {
    slice_ir
        .regions
        .iter()
        .filter(|r| {
            r.polygons
                .iter()
                .any(|exp| exp.contour.points.iter().any(|pt| predicate(*pt)))
        })
        .collect()
}

/// Build initial `Vec<SliceIR>` by slicing `object_mesh` at each layer Z.
fn build_initial_slice_ir(
    object_id: &str,
    object_mesh: &slicer_ir::IndexedTriangleSet,
    layer_plan: &LayerPlanIR,
) -> Vec<SliceIR> {
    let zs: Vec<f32> = layer_plan.global_layers.iter().map(|l| l.z).collect();
    let slabs = slice_mesh_ex(object_mesh, &zs);
    zs.iter()
        .enumerate()
        .map(|(idx, &z)| {
            let polys = slabs.get(idx).cloned().unwrap_or_default();
            SliceIR {
                global_layer_index: idx as u32,
                z,
                regions: vec![SlicedRegion {
                    object_id: object_id.to_string(),
                    region_id: 0,
                    polygons: polys.clone(),
                    infill_areas: polys,
                    effective_layer_height: LAYER_HEIGHT_MM,
                    segment_annotations: HashMap::new(),
                    ..Default::default()
                }],
                ..Default::default()
            }
        })
        .collect()
}

/// Build a minimal `RegionMapIR` with one BASE entry per layer.
fn build_region_map(object_id: &str, layer_count: u32) -> Arc<RegionMapIR> {
    let mut entries = HashMap::new();
    for i in 0..layer_count {
        entries.insert(
            RegionKey {
                global_layer_index: i,
                object_id: object_id.to_string(),
                region_id: 0,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
    }
    Arc::new(RegionMapIR {
        schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
        entries,
        configs: vec![ResolvedConfig::default()],
    })
}

/// Run the v2 pipeline on cube_fuzzyPainted with 50 layers.
fn run_v2(mesh: Arc<MeshIR>, layer_plan: &LayerPlanIR) -> Arc<Vec<SliceIR>> {
    let object_id = &mesh.objects[0].id;
    let object_mesh = mesh.objects[0].mesh.clone();
    let initial = build_initial_slice_ir(object_id, &object_mesh, layer_plan);
    let region_map = build_region_map(object_id, LAYER_COUNT);
    execute_paint_segmentation(mesh, Arc::new(initial), region_map)
        .expect("execute_paint_segmentation must succeed")
}

// ---------------------------------------------------------------------------
// GREEN: whole-facet fuzzy paint survives the pipeline
// ---------------------------------------------------------------------------

/// Smoke: full v2 pipeline does not panic.
#[test]
fn cube_fuzzy_painted_full_pipeline_no_panic() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let object_mesh = mesh.objects[0].mesh.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let test_z = 12.5;
    let sliced_polys = slice_mesh_ex(&object_mesh, &[test_z])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert!(
        !sliced_polys.is_empty(),
        "must have sliced polygons at Z={test_z}"
    );
    assert_eq!(
        new_slice_ir.len() as u32,
        LAYER_COUNT,
        "v2 output must have {LAYER_COUNT} layers"
    );
}

/// Paint segmentation v2 emits FuzzySkin semantic data via variant_chain entries.
///
/// v2 contract: FuzzySkin paint lives in variant_chain (sem_name "fuzzy_skin"),
/// NOT in segment_annotations[PaintSemantic::FuzzySkin].
///
/// NOTE: requires host-algos kernel. Without it, variant_chains remain empty.
/// RED gate until host-algos is enabled. TODO(closure-log P95).
#[test]
fn cube_fuzzy_painted_paint_segmentation_emits_fuzzy_regions() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    // v2 contract: FuzzySkin paint lives in variant_chain (sem_name "fuzzy_skin").
    let has_fuzzy_region = new_slice_ir.iter().any(|layer| {
        layer.regions.iter().any(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "fuzzy_skin" && matches!(pv, PaintValue::Flag(true)))
        })
    });

    assert!(
        has_fuzzy_region,
        "v2 pipeline must produce at least one SlicedRegion with variant_chain \
         [(\"fuzzy_skin\", Flag(true))] for cube_fuzzyPainted.\n\
         Gap: host-algos kernel not enabled — variant_chains are empty without it. \
         RED gate until host-algos is wired into the integration-test build."
    );
}

/// All 50 layers are present in the v2 output.
#[test]
fn cube_fuzzy_painted_all_50_layers_have_map_entries() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    assert_eq!(
        new_slice_ir.len() as u32,
        LAYER_COUNT,
        "expected v2 output length == LAYER_COUNT ({LAYER_COUNT}), got {}",
        new_slice_ir.len()
    );
    for i in 0..LAYER_COUNT {
        assert!(
            new_slice_ir.iter().any(|s| s.global_layer_index == i),
            "layer index {i} must be present in v2 output"
        );
    }
}

/// Front face (-Y, Y≈102.5): fully painted fuzzy. At Z≈12.5mm, all SlicedRegions
/// corresponding to the front face must carry variant_chain [("fuzzy_skin", Flag(true))].
///
/// v2 contract: FuzzySkin paint lives in variant_chain, not segment_annotations.
/// Requires host-algos kernel.
#[test]
fn cube_fuzzy_painted_front_face_fully_fuzzy() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let mid_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=12.5mm");

    // v2 contract: fully-fuzzy front face means at least one SlicedRegion with
    // variant_chain [("fuzzy_skin", Flag(true))]. No region with front-face
    // polygons should have an empty variant_chain (which would mean unpainted).
    let fuzzy_region_count = mid_layer
        .regions
        .iter()
        .filter(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "fuzzy_skin" && *pv == PaintValue::Flag(true))
        })
        .count();

    assert!(
        fuzzy_region_count > 0,
        "mid-layer at Z≈12.5mm must have at least one SlicedRegion with \
         variant_chain [(\"fuzzy_skin\", Flag(true))] for the fully-fuzzy front face.\n\
         Gap: host-algos kernel not enabled — variant_chains are empty without it. \
         RED gate until host-algos is wired in."
    );
}

// ---------------------------------------------------------------------------
// RED: per-face paint correctness — vertical-face projection gap
// ---------------------------------------------------------------------------

/// Back face (+Y, Y≈127.5): half fuzzy, half unpainted (two separate triangles).
/// v2 contract: fuzzy triangle → variant_chain [("fuzzy_skin", Flag(true))];
/// unpainted triangle → variant_chain [] (BASE region).
/// Requires host-algos kernel + vertical-face projection fix.
#[test]
fn cube_fuzzy_painted_back_face_half_half_requires_vertical_face_projection() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let mid_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=12.5mm");

    // v2 contract: expect both a fuzzy variant_chain region AND a BASE region.
    let has_fuzzy_region = mid_layer.regions.iter().any(|r| {
        r.variant_chain
            .iter()
            .any(|(sem, pv)| sem == "fuzzy_skin" && *pv == PaintValue::Flag(true))
    });
    let has_base_region = mid_layer.regions.iter().any(|r| r.variant_chain.is_empty());

    assert!(
        has_fuzzy_region && has_base_region,
        "RED: back face at Z≈12.5mm should have BOTH a fuzzy variant_chain region \
         (painted triangle) AND a BASE variant_chain region (unpainted triangle). \
         has_fuzzy_region={has_fuzzy_region}, has_base_region={has_base_region}.\n\
         Gap: host-algos kernel not enabled or vertical-face projection gap."
    );
}

/// Left face (-X, X≈112.5): unpainted.
/// v2 contract: at Z≈12.5mm, SlicedRegion(s) whose polygons cover the left face position
/// (x≈x_min) must NOT have variant_chain [("fuzzy_skin", Flag(true))].
///
/// Positional assertion: only checks regions geometrically covering the left face.
/// Other faces (front, right) may have fuzzy paint — that is correct and irrelevant here.
#[test]
fn cube_fuzzy_painted_left_face_unpainted_requires_vertical_face_projection() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let mid_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=12.5mm");

    // Positional query: find regions whose polygons touch the left face (x ≈ x_min).
    let fb = face_bounds();
    // Tolerance: 1% of cube width (25mm) = 0.25mm = 2500 units.
    let tol = (fb.x_max - fb.x_min) / 100;
    let left_face_regions = regions_covering(mid_layer, |pt| pt.x <= fb.x_min + tol);

    // v2 contract: the left face is unpainted — none of its covering regions should carry fuzzy.
    let fuzzy_on_left_count = left_face_regions
        .iter()
        .filter(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "fuzzy_skin" && *pv == PaintValue::Flag(true))
        })
        .count();

    assert!(
        fuzzy_on_left_count == 0,
        "left face (x≈{}) is unpainted — the {} region(s) covering it must have zero \
         fuzzy_skin/Flag(true) variant_chain entries at Z≈12.5mm. Got {fuzzy_on_left_count}.\n\
         Gap: vertical face projection must not bleed fuzzy paint onto the unpainted left face.",
        fb.x_min,
        left_face_regions.len()
    );
}

/// Bottom face (-Z, Z≈0): unpainted horizontal face.
/// v2 contract: at Z≈0.1mm (bottom layer), the cross-section IS the bottom face.
/// No SlicedRegion in this layer should have variant_chain [("fuzzy_skin", Flag(true))].
///
/// Positional: all polygons at Z≈0.1mm are the bottom face cross-section; regions_covering
/// uses the whole layer (x within x_min..x_max and y within y_min..y_max = all polygons).
/// Asserts the bottom face (as a whole layer slice) is free of fuzzy paint.
#[test]
fn cube_fuzzy_painted_bottom_face_unpainted_requires_vertical_face_projection() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let bot_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 0.1f32)
                .abs()
                .partial_cmp(&(b.z - 0.1f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=0.1mm");

    // Positional query: at Z≈0.1mm the entire cross-section is the bottom face.
    // Select all regions with any polygon point inside the cube's XY footprint.
    let fb = face_bounds();
    let bottom_face_regions = regions_covering(bot_layer, |pt| {
        pt.x >= fb.x_min && pt.x <= fb.x_max && pt.y >= fb.y_min && pt.y <= fb.y_max
    });

    // v2 contract: the unpainted bottom face must carry no fuzzy_skin/Flag(true) paint.
    let fuzzy_on_bottom_count = bottom_face_regions
        .iter()
        .filter(|r| {
            r.variant_chain
                .iter()
                .any(|(sem, pv)| sem == "fuzzy_skin" && *pv == PaintValue::Flag(true))
        })
        .count();

    assert!(
        fuzzy_on_bottom_count == 0,
        "bottom face is unpainted — the {} region(s) covering it at Z≈0.1mm must have zero \
         fuzzy_skin/Flag(true) variant_chain entries. Got {fuzzy_on_bottom_count}.\n\
         Gap: vertical face projection must not bleed fuzzy paint onto the unpainted bottom face.",
        bottom_face_regions.len()
    );
}

// ---------------------------------------------------------------------------
// GREEN: negative assertions and modifier overlay
// ---------------------------------------------------------------------------

/// D14 invariant: modifier-volume SupportEnforcer annotations appear in
/// `segment_annotations[SupportEnforcer]` on the BASE variant chain only
/// (variant_chain.is_empty()), and are absent from any painted variant chains.
///
/// Uses a synthetic 1mm cube MeshIR with:
///  - one FuzzySkin PaintLayer (so `mesh_has_any_paint` passes the short-circuit)
///  - one SupportEnforcer ModifierVolume overlapping the slice polygon
///
/// D14 contract is unconditional (not behind host-algos feature); modifier-volume
/// slicing runs regardless of whether the Voronoi kernel fires.
#[test]
fn cube_fuzzy_painted_modifier_overlay_on_unpainted_face() {
    let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };
    let pt3 = |x, y, z| Point3 { x, y, z };

    // 1×1×1 mm body mesh (cube 0..1 in all axes).
    let body_mesh = IndexedTriangleSet {
        vertices: vec![
            pt3(0.0, 0.0, 0.0),
            pt3(1.0, 0.0, 0.0),
            pt3(1.0, 1.0, 0.0),
            pt3(0.0, 1.0, 0.0),
            pt3(0.0, 0.0, 1.0),
            pt3(1.0, 0.0, 1.0),
            pt3(1.0, 1.0, 1.0),
            pt3(0.0, 1.0, 1.0),
        ],
        #[rustfmt::skip]
        indices: vec![
            0, 2, 1, 0, 3, 2,   // bottom
            4, 5, 6, 4, 6, 7,   // top
            0, 1, 5, 0, 5, 4,   // front
            2, 3, 7, 2, 7, 6,   // back
            0, 4, 7, 0, 7, 3,   // left
            1, 2, 6, 1, 6, 5,   // right
        ],
    };

    // 0.5×0.5×0.5 mm SupportEnforcer modifier volume (fits inside body).
    let mv_mesh = IndexedTriangleSet {
        vertices: vec![
            pt3(0.0, 0.0, 0.0),
            pt3(0.5, 0.0, 0.0),
            pt3(0.5, 0.5, 0.0),
            pt3(0.0, 0.5, 0.0),
            pt3(0.0, 0.0, 0.5),
            pt3(0.5, 0.0, 0.5),
            pt3(0.5, 0.5, 0.5),
            pt3(0.0, 0.5, 0.5),
        ],
        #[rustfmt::skip]
        indices: vec![
            0, 2, 1, 0, 3, 2,
            4, 5, 6, 4, 6, 7,
            0, 1, 5, 0, 5, 4,
            2, 3, 7, 2, 7, 6,
            0, 4, 7, 0, 7, 3,
            1, 2, 6, 1, 6, 5,
        ],
    };
    let mut mv_fields = HashMap::new();
    mv_fields.insert(
        "subtype".to_string(),
        ConfigValue::String("support_enforcer".to_string()),
    );
    let mv = ModifierVolume {
        id: "mv_enforcer".to_string(),
        mesh: mv_mesh,
        config_delta: ConfigDelta { fields: mv_fields },
        priority: 0,
        applies_to: ModifierScope::AllFeatures,
    };

    // FuzzySkin PaintLayer: one facet painted Flag(true) so mesh_has_any_paint passes.
    let paint_layer = PaintLayer {
        semantic: PaintSemantic::FuzzySkin,
        facet_values: {
            let mut v = vec![None; 12]; // 12 triangles for 6-face cube
            v[0] = Some(PaintValue::Flag(true));
            v
        },
        strokes: Vec::new(),
    };

    let identity = Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };

    let mesh = Arc::new(MeshIR {
        schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
        objects: vec![ObjectMesh {
            id: "syn_obj".to_string(),
            mesh: body_mesh,
            transform: identity,
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![mv],
            paint_data: Some(FacetPaintData {
                layers: vec![paint_layer],
            }),
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 250.0,
                y: 210.0,
                z: 220.0,
            },
        },
    });

    // One layer at z=0.5mm (midpoint of the 1mm cube body, inside the 0.5mm enforcer too).
    let slice_polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: u(0.0),
                    y: u(0.0),
                },
                Point2 {
                    x: u(1.0),
                    y: u(0.0),
                },
                Point2 {
                    x: u(1.0),
                    y: u(1.0),
                },
                Point2 {
                    x: u(0.0),
                    y: u(1.0),
                },
            ],
        },
        holes: Vec::new(),
    };
    let initial_slice = Arc::new(vec![SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![SlicedRegion {
            object_id: "syn_obj".to_string(),
            region_id: 0,
            polygons: vec![slice_polygon.clone()],
            infill_areas: vec![slice_polygon],
            ..Default::default()
        }],
    }]);

    let mut region_map_entries = HashMap::new();
    region_map_entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "syn_obj".to_string(),
            region_id: 0,
            variant_chain: vec![],
        },
        RegionPlan::default(),
    );
    let region_map = Arc::new(RegionMapIR {
        schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
        entries: region_map_entries,
        configs: vec![ResolvedConfig::default()],
    });

    let result = execute_paint_segmentation(mesh, initial_slice, region_map)
        .expect("execute_paint_segmentation must succeed for synthetic D14 test");

    // There must be at least one layer.
    assert!(!result.is_empty(), "result must have at least one layer");

    let layer = &result[0];

    // D14: BASE variant chain must exist.
    let base_region = layer
        .regions
        .iter()
        .find(|r| r.variant_chain.is_empty())
        .expect("BASE region (empty variant_chain) must exist — D14 requires it");

    // D14: BASE must carry SupportEnforcer (or SupportBlocker) in segment_annotations.
    let has_enforcer = base_region
        .segment_annotations
        .contains_key(&PaintSemantic::SupportEnforcer);
    assert!(
        has_enforcer,
        "D14: BASE region segment_annotations must contain SupportEnforcer key \
         when a SupportEnforcer modifier volume overlaps the slice polygon. \
         Got keys: {:?}",
        base_region.segment_annotations.keys().collect::<Vec<_>>()
    );

    // D14: painted variant chains (if any) must NOT carry modifier-volume annotations.
    for region in &layer.regions {
        if !region.variant_chain.is_empty() {
            assert!(
                !region
                    .segment_annotations
                    .contains_key(&PaintSemantic::SupportEnforcer),
                "D14: modifier-volume annotations must NOT appear on painted variant chain {:?}; \
                 segment_annotations keys: {:?}",
                region.variant_chain,
                region.segment_annotations.keys().collect::<Vec<_>>()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RED: sub-facet fuzzy detail lost — executable gap specifications
// ---------------------------------------------------------------------------

/// Right face (+X, X≈137.5): fuzzy circle (hex subdivision).
/// v2 contract: the circle area is a variant_chain [("fuzzy_skin", Flag(true))] region;
/// the surrounding area is a BASE region (empty variant_chain).
/// Requires host-algos kernel + sub-facet stroke support.
#[test]
fn cube_fuzzy_painted_right_face_circle_requires_fuzzy_strokes() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let mid_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 12.5f32)
                .abs()
                .partial_cmp(&(b.z - 12.5f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=12.5mm");

    // v2 contract: expect both a fuzzy variant_chain region (circle) AND a BASE region
    // (outside the circle), both intersecting the right face at Z≈12.5mm.
    let has_fuzzy_region = mid_layer.regions.iter().any(|r| {
        r.variant_chain
            .iter()
            .any(|(sem, pv)| sem == "fuzzy_skin" && *pv == PaintValue::Flag(true))
    });
    let has_base_region = mid_layer.regions.iter().any(|r| r.variant_chain.is_empty());

    assert!(
        has_fuzzy_region && has_base_region,
        "RED: right face (circle) at Z≈12.5mm should have BOTH a fuzzy variant_chain region \
         (inside circle) AND a BASE variant_chain region (outside circle). \
         has_fuzzy_region={has_fuzzy_region}, has_base_region={has_base_region}.\n\
         Gap: host-algos kernel not enabled or sub-facet stroke support missing.",
    );
}

/// Top face (+Z, Z≈24.9mm): fuzzy circle (hex subdivision).
/// v2 contract: circle area is variant_chain [("fuzzy_skin", Flag(true))];
/// surrounding area is BASE (empty variant_chain).
/// Requires host-algos kernel + sub-facet stroke support.
#[test]
fn cube_fuzzy_painted_top_face_circle_requires_fuzzy_strokes() {
    let mesh = load_cube_fuzzy_painted();
    let object_id = mesh.objects[0].id.clone();
    let lp = build_50_layer_plan(&object_id);

    let new_slice_ir = run_v2(Arc::new(mesh), &lp);

    let top_layer = new_slice_ir
        .iter()
        .min_by(|a, b| {
            (a.z - 24.9f32)
                .abs()
                .partial_cmp(&(b.z - 24.9f32).abs())
                .unwrap()
        })
        .expect("must have a layer near Z=24.9mm");

    // v2 contract: expect both a fuzzy variant_chain region AND a BASE region.
    let has_fuzzy_region = top_layer.regions.iter().any(|r| {
        r.variant_chain
            .iter()
            .any(|(sem, pv)| sem == "fuzzy_skin" && *pv == PaintValue::Flag(true))
    });
    let has_base_region = top_layer.regions.iter().any(|r| r.variant_chain.is_empty());

    assert!(
        has_fuzzy_region && has_base_region,
        "RED: top face (circle) at Z≈24.9mm should have BOTH a fuzzy variant_chain region \
         (inside circle) AND a BASE variant_chain region (outside circle). \
         has_fuzzy_region={has_fuzzy_region}, has_base_region={has_base_region}.\n\
         Gap: host-algos kernel not enabled or sub-facet stroke support missing.",
    );
}
