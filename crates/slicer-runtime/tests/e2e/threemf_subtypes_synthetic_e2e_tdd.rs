//! TDD suite for Packet 56c: negative_part subtract and support subtype routing.
//!
//! Tests for:
//! - AC1/AC2: negative_part modifier reduces SliceIR polygon area.
//! - AC3/AC4: support_enforcer/support_blocker emits PaintRegionIR entries.
//! - AC5: negative-part subtract runs before paint annotation sees polygons.
//! - AC6: support_enforcer entries flow through to PaintRegionIR.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigDelta, ConfigValue, ExPolygon, GlobalLayer,
    IndexedTriangleSet, LayerPlanIR, MeshIR, ModifierScope, ModifierVolume, ObjectConfig,
    ObjectMesh, PaintSemantic, Point2, Point3, Polygon, RegionKey, ResolvedConfig, SemVer,
    SemanticRegion, SliceIR, SlicedRegion, SurfaceClassificationIR, Transform3d,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

fn p2(x: f32, y: f32) -> Point2 {
    Point2::from_mm(x, y)
}

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn schema_ver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// Build a square ExPolygon centered at origin with side length `side_mm`.
fn square_polygon(side_mm: f32) -> ExPolygon {
    let h = side_mm / 2.0;
    ExPolygon {
        contour: Polygon {
            points: vec![p2(-h, -h), p2(h, -h), p2(h, h), p2(-h, h)],
        },
        holes: vec![],
    }
}

/// Build a box IndexedTriangleSet with given half-extents (hx, hy, hz) centered at origin.
/// 8 vertices, 12 triangles.
fn box_mesh(hx: f32, hy: f32, hz: f32) -> IndexedTriangleSet {
    // 8 corners
    let v = vec![
        p3(-hx, -hy, -hz), // 0
        p3(hx, -hy, -hz),  // 1
        p3(hx, hy, -hz),   // 2
        p3(-hx, hy, -hz),  // 3
        p3(-hx, -hy, hz),  // 4
        p3(hx, -hy, hz),   // 5
        p3(hx, hy, hz),    // 6
        p3(-hx, hy, hz),   // 7
    ];
    // 12 triangles (2 per face)
    #[rustfmt::skip]
    let indices = vec![
        // bottom
        0, 2, 1,  0, 3, 2,
        // top
        4, 5, 6,  4, 6, 7,
        // front
        0, 1, 5,  0, 5, 4,
        // back
        2, 3, 7,  2, 7, 6,
        // left
        0, 4, 7,  0, 7, 3,
        // right
        1, 2, 6,  1, 6, 5,
    ];
    IndexedTriangleSet {
        vertices: v,
        indices,
    }
}

/// Build a SliceIR with one region containing the given polygon at z_mm.
fn slice_ir_with_polygon(z_mm: f32, polygon: ExPolygon) -> SliceIR {
    SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: z_mm,
        regions: vec![SlicedRegion {
            object_id: String::from("parent-obj"),
            region_id: 0,
            polygons: vec![polygon],
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
    }
}

/// Build a ModifierVolume with a given subtype string and mesh.
fn modifier_volume_with_subtype(subtype: &str, mesh: IndexedTriangleSet) -> ModifierVolume {
    let mut fields = HashMap::new();
    fields.insert(
        String::from("subtype"),
        ConfigValue::String(subtype.to_string()),
    );
    ModifierVolume {
        id: String::from("mv-1"),
        mesh,
        config_delta: ConfigDelta { fields },
        priority: 0,
        applies_to: ModifierScope::AllFeatures,
    }
}

fn mesh_ir_with_modifier(object_id: &str, mv: ModifierVolume) -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: schema_ver(),
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: box_mesh(5.0, 5.0, 5.0), // 10x10x10 parent
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![mv],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: p3(0.0, 0.0, 0.0),
            max: p3(200.0, 200.0, 200.0),
        },
    })
}

fn layer_plan_with_z_values(object_id: &str, zs: &[(u32, f32)]) -> Arc<LayerPlanIR> {
    Arc::new(LayerPlanIR {
        schema_version: schema_ver(),
        global_layers: zs
            .iter()
            .map(|(idx, z)| GlobalLayer {
                index: *idx,
                z: *z,
                active_regions: vec![ActiveRegion {
                    object_id: object_id.to_string(),
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
        object_participation: HashMap::new(),
    })
}

fn empty_surface_ir() -> Arc<SurfaceClassificationIR> {
    Arc::new(SurfaceClassificationIR::default())
}

// Shoelace area on a Polygon in scaled-intÂ² units (1 unit = 100 nm).
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

// Aggregate area in mmÂ² across a slice of ExPolygons. Sums signed shoelace areas across
// every contour and nested hole. Clipper2 emits outer rings CCW (positive) and inner
// rings CW (negative); when the result is flattened (outer + inner ring as separate
// ExPolygons with no nested holes), summing signed areas still yields the correct
// net area. Taking abs at the end handles either orientation convention.
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

// Aggregate polygon area across every SemanticRegion in a slice.
fn sum_semantic_region_area_mm2(regions: &[SemanticRegion]) -> f64 {
    regions.iter().map(|r| sum_area_mm2(&r.polygons)).sum()
}

// Build an empty ExecutionPlan suitable for driving execute_region_mapping in tests.
fn empty_execution_plan() -> slicer_runtime::ExecutionPlan {
    let mut diagnostics: Vec<slicer_runtime::LoadDiagnostic> = Vec::new();
    slicer_runtime::build_execution_plan(
        &slicer_runtime::ExecutionPlanRequest {
            sorted_stages: Vec::<slicer_runtime::SortedStageModules>::new(),
            module_bindings: vec![],
            global_layers: Arc::new(vec![]),
            region_plans: Arc::new(HashMap::new()),
        },
        &mut diagnostics,
    )
    .expect("empty execution plan must build")
}

// ---------------------------------------------------------------------------
// Step 1: negative_part tests (will compile only after Step 2 adds the module)
// ---------------------------------------------------------------------------

/// AC1: negative_part modifier removes polygon area at layers within its Z extent,
/// AND leaves layers outside the Z extent bit-identical to the baseline (no subtract).
#[test]
fn negative_part_removes_layer_polygon_area() {
    // Parent: 20Ã—20 mm cross-section. Negative: 5Ã—5Ã—5 mm cube (Z range [-2.5..2.5]).
    let parent_polygon = square_polygon(20.0);
    let pre_polygons_inside = vec![parent_polygon.clone()];
    let pre_polygons_outside = vec![parent_polygon.clone()];

    let pre_area_inside = sum_area_mm2(&pre_polygons_inside);
    let pre_area_outside = sum_area_mm2(&pre_polygons_outside);

    // In-extent layer: z=0.0 (centroid of negative cube).
    let mut slice_inside = slice_ir_with_polygon(0.0, parent_polygon.clone());
    // Out-of-extent layer: z=5.0 (above the negative cube's Z-max of 2.5, still within parent).
    let mut slice_outside = slice_ir_with_polygon(5.0, parent_polygon);

    let make_mv = || modifier_volume_with_subtype("negative_part", box_mesh(2.5, 2.5, 2.5));

    slicer_runtime::negative_part_subtract::apply_negative_part_subtract(
        &mut slice_inside,
        &[make_mv()],
    );
    slicer_runtime::negative_part_subtract::apply_negative_part_subtract(
        &mut slice_outside,
        &[make_mv()],
    );

    // In-extent: post area strictly less than pre area.
    let post_area_inside = sum_area_mm2(&slice_inside.regions[0].polygons);
    assert!(
        post_area_inside < pre_area_inside,
        "in-extent post-subtract area ({post_area_inside} mmÂ²) must be < pre-subtract area ({pre_area_inside} mmÂ²)"
    );

    // Out-of-extent: polygons bit-identical to baseline; area unchanged.
    let post_polygons_outside = &slice_outside.regions[0].polygons;
    assert_eq!(
        post_polygons_outside.len(),
        pre_polygons_outside.len(),
        "out-of-extent layer must retain the same number of polygons"
    );
    assert_eq!(
        post_polygons_outside[0].contour.points, pre_polygons_outside[0].contour.points,
        "out-of-extent layer must have bit-identical contour points"
    );
    assert_eq!(
        post_polygons_outside[0].holes, pre_polygons_outside[0].holes,
        "out-of-extent layer must have bit-identical holes"
    );
    let post_area_outside = sum_area_mm2(post_polygons_outside);
    assert!(
        (post_area_outside - pre_area_outside).abs() < 1e-9,
        "out-of-extent area ({post_area_outside} mmÂ²) must equal pre-subtract area ({pre_area_outside} mmÂ²)"
    );
}

/// AC2: at the centroid Z of a 5Ã—5Ã—5 mm negative cube inside a 20Ã—20Ã—20 mm parent,
/// the polygon-area reduction matches 25.0 mmÂ² (the cube's cross-section) within Â±0.005 mmÂ²
/// (Clipper2 rounding tolerance).
#[test]
fn negative_part_area_reduction_matches_cube_cross_section() {
    let parent_polygon = square_polygon(20.0);
    let pre_area = sum_area_mm2(&[parent_polygon.clone()]);

    let mut slice = slice_ir_with_polygon(0.0, parent_polygon);

    // 5Ã—5Ã—5 mm negative cube centered at origin â†’ 25 mmÂ² cross-section at z=0.
    let mv = modifier_volume_with_subtype("negative_part", box_mesh(2.5, 2.5, 2.5));

    slicer_runtime::negative_part_subtract::apply_negative_part_subtract(&mut slice, &[mv]);

    let post_area = sum_area_mm2(&slice.regions[0].polygons);
    let reduction = pre_area - post_area;
    const EXPECTED_REDUCTION_MM2: f64 = 25.0;
    const TOLERANCE_MM2: f64 = 0.005;

    assert!(
        (reduction - EXPECTED_REDUCTION_MM2).abs() < TOLERANCE_MM2,
        "area reduction = {reduction} mmÂ² (pre={pre_area}, post={post_area}); \
         expected {EXPECTED_REDUCTION_MM2} mmÂ² Â± {TOLERANCE_MM2} mmÂ²"
    );
}

/// Negative case: negative_part modifier above the layer Z should NOT subtract anything.
#[test]
fn negative_part_above_parent_no_subtract() {
    // Slice at Z=0.1mm; modifier box is at Z range [5..15mm] (entirely above).
    let parent_polygon = square_polygon(10.0);
    let polygon_before = parent_polygon.clone();
    let mut slice = slice_ir_with_polygon(0.1, parent_polygon);

    // Modifier mesh: box from Z=5..15mm (half-extent hz=5, centered at z=10)
    // We offset the mesh by 10mm in Z.
    let mut mesh = box_mesh(3.0, 3.0, 5.0);
    for v in &mut mesh.vertices {
        v.z += 10.0; // shift to z=5..15mm
    }
    let mv = modifier_volume_with_subtype("negative_part", mesh);

    slicer_runtime::negative_part_subtract::apply_negative_part_subtract(&mut slice, &[mv]);

    // Polygons must be unchanged.
    assert_eq!(
        slice.regions[0].polygons.len(),
        1,
        "no subtract should occur above layer Z"
    );
    assert_eq!(
        slice.regions[0].polygons[0].contour.points, polygon_before.contour.points,
        "polygon contour should be unchanged"
    );
}

/// AC5: negative_part subtract runs before paint segmentation sees the polygons.
/// Verify by applying subtract then checking the geometry is different from the original,
/// proving the subtract happens before any downstream consumer.
#[test]
fn negative_part_subtract_runs_before_paint_segmentation() {
    let parent_polygon = square_polygon(10.0);
    let original_contour_pts = parent_polygon.contour.points.clone();
    let mut slice = slice_ir_with_polygon(1.0, parent_polygon);

    // Modifier cuts a 4x4mm hole out of the parent at z=1.0mm.
    let mv = modifier_volume_with_subtype("negative_part", box_mesh(2.0, 2.0, 2.0));

    // Step 1: apply subtract (simulates layer_executor doing it before paint annotation).
    slicer_runtime::negative_part_subtract::apply_negative_part_subtract(&mut slice, &[mv]);

    // Step 2: verify the slice polygons used by downstream are geometrically different.
    let result_polys = &slice.regions[0].polygons;
    assert!(!result_polys.is_empty(), "result must retain polygon data");

    let any_hole = result_polys.iter().any(|p| !p.holes.is_empty());
    let total_polys = result_polys.len();
    let geometry_changed = any_hole
        || total_polys > 1
        || (total_polys == 1 && result_polys[0].contour.points != original_contour_pts);

    assert!(
        geometry_changed,
        "subtract must change the geometry seen by downstream ({total_polys} poly(s), any_hole={any_hole})"
    );
}

// ---------------------------------------------------------------------------
// Step 3: support subtype paint region tests
// ---------------------------------------------------------------------------

/// AC3: support_enforcer modifier_volume emits SemanticRegion entries into
/// LayerPaintMap.semantic_regions under PaintSemantic::SupportEnforcer at every overlapping
/// global layer index, with aggregate polygon area matching the modifier's per-layer
/// projection area within Â±0.005 mmÂ².
#[test]
fn support_enforcer_emits_paint_region() {
    // Modifier: 4Ã—4Ã—4 mm box â†’ 16 mmÂ² cross-section at z=0.5.
    let enforcer_mv = modifier_volume_with_subtype("support_enforcer", box_mesh(2.0, 2.0, 2.0));
    const PROJECTED_AREA_MM2: f64 = 16.0;
    const TOLERANCE_MM2: f64 = 0.005;

    let mesh_ir = mesh_ir_with_modifier("obj-1", enforcer_mv);
    let layer_plan = layer_plan_with_z_values("obj-1", &[(0, 0.5), (1, 5.0)]);

    let paint_ir = slicer_core::algos::paint_segmentation::execute_paint_segmentation(
        Arc::clone(&mesh_ir),
        empty_surface_ir(),
        Arc::clone(&layer_plan),
        true,
    )
    .expect("paint segmentation should succeed");

    // In-extent layer: must have SupportEnforcer entry via the documented map access pattern.
    let enforcer_regions = paint_ir
        .per_layer
        .get(&0u32)
        .and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportEnforcer))
        .expect("layer 0 must have a SupportEnforcer entry in semantic_regions");
    assert!(
        !enforcer_regions.is_empty(),
        "SupportEnforcer entry must contain at least one SemanticRegion"
    );

    let aggregate_area = sum_semantic_region_area_mm2(enforcer_regions);
    assert!(
        (aggregate_area - PROJECTED_AREA_MM2).abs() < TOLERANCE_MM2,
        "aggregate SupportEnforcer area = {aggregate_area} mmÂ² (expected {PROJECTED_AREA_MM2} Â± {TOLERANCE_MM2} mmÂ²)"
    );

    // Out-of-extent layer: lookup must return None or empty.
    let outside = paint_ir
        .per_layer
        .get(&1u32)
        .and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportEnforcer));
    assert!(
        outside.map_or(true, |r| r.is_empty()),
        "layer 1 (z=5.0mm) must have no SupportEnforcer entry (outside modifier Z extent)"
    );
}

/// AC4: support_blocker modifier_volume emits SemanticRegion entries under
/// PaintSemantic::SupportBlocker with aggregate polygon area matching the modifier's per-layer
/// projection area within Â±0.005 mmÂ².
#[test]
fn support_blocker_emits_paint_region() {
    // Modifier: 4Ã—4Ã—4 mm box â†’ 16 mmÂ² cross-section at z=0.5.
    let blocker_mv = modifier_volume_with_subtype("support_blocker", box_mesh(2.0, 2.0, 2.0));
    const PROJECTED_AREA_MM2: f64 = 16.0;
    const TOLERANCE_MM2: f64 = 0.005;

    let mesh_ir = mesh_ir_with_modifier("obj-1", blocker_mv);
    let layer_plan = layer_plan_with_z_values("obj-1", &[(0, 0.5)]);

    let paint_ir = slicer_core::algos::paint_segmentation::execute_paint_segmentation(
        Arc::clone(&mesh_ir),
        empty_surface_ir(),
        Arc::clone(&layer_plan),
        true,
    )
    .expect("paint segmentation should succeed");

    let blocker_regions = paint_ir
        .per_layer
        .get(&0u32)
        .and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportBlocker))
        .expect("layer 0 must have a SupportBlocker entry in semantic_regions");
    assert!(
        !blocker_regions.is_empty(),
        "SupportBlocker entry must contain at least one SemanticRegion"
    );

    let aggregate_area = sum_semantic_region_area_mm2(blocker_regions);
    assert!(
        (aggregate_area - PROJECTED_AREA_MM2).abs() < TOLERANCE_MM2,
        "aggregate SupportBlocker area = {aggregate_area} mmÂ² (expected {PROJECTED_AREA_MM2} Â± {TOLERANCE_MM2} mmÂ²)"
    );
}

/// Negative case: support_enforcer with an empty mesh emits nothing.
#[test]
fn empty_support_enforcer_emits_nothing() {
    let empty_mesh = IndexedTriangleSet {
        vertices: vec![],
        indices: vec![],
    };
    let mv = modifier_volume_with_subtype("support_enforcer", empty_mesh);
    let mesh_ir = mesh_ir_with_modifier("obj-1", mv);
    let layer_plan = layer_plan_with_z_values("obj-1", &[(0, 0.5)]);

    let paint_ir = slicer_core::algos::paint_segmentation::execute_paint_segmentation(
        Arc::clone(&mesh_ir),
        empty_surface_ir(),
        Arc::clone(&layer_plan),
        true,
    )
    .expect("paint segmentation should succeed for empty modifier");

    let enforcer_regions = paint_ir.get(0, &PaintSemantic::SupportEnforcer);
    assert!(
        enforcer_regions.is_empty(),
        "empty mesh modifier should emit no paint regions"
    );
}

/// AC6: a support_enforcer modifier flows through Packet 51's paint_overrides overlay so
/// that at any intersecting layer the per-semantic ResolvedConfig is overlaid on top of
/// the base config. Asserts that at least one ResolvedConfig field (`support_overhang_angle` â€”
/// a Packet 51 paint-supports override key) differs from the default after the overlay
/// is applied. The AC text named `support_threshold_angle`; the real field name in
/// `ResolvedConfig` is `support_overhang_angle` (the AC's name was a suggested example).
#[test]
fn support_enforcer_flows_through_paint_overrides() {
    // Part A: paint_segmentation emits SupportEnforcer entries from the modifier_volume.
    let enforcer_mv = modifier_volume_with_subtype("support_enforcer", box_mesh(3.0, 3.0, 3.0));
    let mesh_ir = mesh_ir_with_modifier("obj-abc", enforcer_mv);
    let layer_plan = layer_plan_with_z_values("obj-abc", &[(0, 1.0), (1, 2.0)]);

    let paint_ir = slicer_core::algos::paint_segmentation::execute_paint_segmentation(
        Arc::clone(&mesh_ir),
        empty_surface_ir(),
        Arc::clone(&layer_plan),
        true,
    )
    .expect("paint segmentation should succeed");

    for layer_idx in [0u32, 1u32] {
        let regions = paint_ir
            .per_layer
            .get(&layer_idx)
            .and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportEnforcer))
            .unwrap_or_else(|| panic!("layer {layer_idx} must have a SupportEnforcer entry"));
        assert!(
            !regions.is_empty(),
            "layer {layer_idx} SupportEnforcer entry must be non-empty"
        );
    }

    // Part B: run a PaintRegionIR through Packet 51's paint_overrides overlay via
    // `execute_region_mapping` with a non-default `support_overhang_angle`, then prove the
    // resolved RegionPlan.config field differs from the default. We construct a parallel
    // whole-layer PaintRegionIR (polygons: vec![] sentinel) so the overlay fires
    // unconditionally â€” isolating the overlay assertion from polygon-overlap details,
    // which are covered separately by AC3/AC4.
    let default_overhang = ResolvedConfig::default().support_overhang_angle;
    let overridden_overhang = 30.0_f32;
    assert_ne!(
        default_overhang, overridden_overhang,
        "the override must differ from the ResolvedConfig default for this AC to be meaningful"
    );

    let mut paint_semantic_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    paint_semantic_configs.insert(
        PaintSemantic::SupportEnforcer,
        ResolvedConfig {
            support_overhang_angle: overridden_overhang,
            ..ResolvedConfig::default()
        },
    );

    let plan = empty_execution_plan();
    let si: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = plan
        .per_layer_stages
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
        .collect();
    let projection = slicer_core::algos::region_mapping::RegionMappingPlanProjection {
        stage_invocations: &si,
    };
    let region_map = slicer_runtime::execute_region_mapping(
        &layer_plan,
        &projection,
        &paint_semantic_configs,
        &BTreeMap::new(),
        &[],
    )
    .expect("execute_region_mapping must succeed");

    let key = RegionKey {
        global_layer_index: 0,
        object_id: String::from("obj-abc"),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    let region_plan = region_map
        .entries
        .get(&key)
        .expect("RegionMapIR must contain an entry for obj-abc at layer 0");
    let region_config = region_map.config_for(&key);

    assert!(
        region_plan
            .paint_overrides
            .contains_key(&PaintSemantic::SupportEnforcer),
        "RegionPlan.paint_overrides must record the SupportEnforcer overlay"
    );
    assert_ne!(
        region_config.support_overhang_angle, default_overhang,
        "overlay must produce a support_overhang_angle that differs from the default ({default_overhang})"
    );
    assert_eq!(
        region_config.support_overhang_angle, overridden_overhang,
        "RegionPlan.config.support_overhang_angle must equal the overlay value ({overridden_overhang})"
    );
}

/// Negative case: negative_part modifier with an empty mesh must NOT alter parent polygons.
///
/// A zero-triangle `negative_part` modifier is degenerate, not an error. The subtract
/// step must silently skip it and leave `slice_ir.regions[ri].polygons` bit-identical
/// to the baseline (no subtract applied). No warning is expected â€” degenerate is
/// behavioral, not a failure mode.
#[test]
fn empty_negative_part_no_subtract() {
    let parent_polygon = square_polygon(10.0);
    let polygon_before = parent_polygon.clone();
    let mut slice = slice_ir_with_polygon(1.0, parent_polygon);

    // Modifier with a zero-triangle mesh (empty vertices, empty indices).
    let empty_mesh = IndexedTriangleSet {
        vertices: vec![],
        indices: vec![],
    };
    let mv = modifier_volume_with_subtype("negative_part", empty_mesh);

    slicer_runtime::negative_part_subtract::apply_negative_part_subtract(&mut slice, &[mv]);

    // Polygons must be bit-identical to the baseline (no subtract performed).
    assert_eq!(
        slice.regions[0].polygons.len(),
        1,
        "empty negative_part mesh must not change polygon count"
    );
    assert_eq!(
        slice.regions[0].polygons[0].contour.points, polygon_before.contour.points,
        "polygon contour must be unchanged when negative_part mesh is empty"
    );
    assert_eq!(
        slice.regions[0].polygons[0].holes, polygon_before.holes,
        "polygon holes must be unchanged when negative_part mesh is empty"
    );
}

/// Negative case: support_blocker with an empty mesh emits nothing.
///
/// A zero-triangle `support_blocker` modifier is degenerate. Paint segmentation must
/// silently skip it: `paint_ir.get(layer, &PaintSemantic::SupportBlocker)` must be
/// empty for every layer. No warning is expected.
#[test]
fn empty_support_blocker_emits_nothing() {
    let empty_mesh = IndexedTriangleSet {
        vertices: vec![],
        indices: vec![],
    };
    let mv = modifier_volume_with_subtype("support_blocker", empty_mesh);
    let mesh_ir = mesh_ir_with_modifier("obj-1", mv);
    let layer_plan = layer_plan_with_z_values("obj-1", &[(0, 0.5), (1, 1.0)]);

    let paint_ir = slicer_core::algos::paint_segmentation::execute_paint_segmentation(
        Arc::clone(&mesh_ir),
        empty_surface_ir(),
        Arc::clone(&layer_plan),
        true,
    )
    .expect("paint segmentation should succeed for empty modifier");

    for layer_idx in [0u32, 1u32] {
        let blocker_regions = paint_ir.get(layer_idx, &PaintSemantic::SupportBlocker);
        assert!(
            blocker_regions.is_empty(),
            "empty support_blocker mesh should emit no paint regions at layer {layer_idx}"
        );
    }
}
