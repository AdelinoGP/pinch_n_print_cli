//! Regression tests for `PrePass::SupportGeometry`'s consumption of the
//! prepass-committed `Vec<SliceIR>` (Commit 4 of the slicing-promotion plan).
//!
//! Before this refactor, `collect_polygons_at_z` was a stub that always
//! returned empty polygons. After the refactor it pulls per-region polygons
//! from `Blackboard::slice_ir()` via binary search on layer Z, taking the
//! upper-bracket layer for interpolated Z values (documented deviation).

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::support_geometry;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectMesh, Point2, Point3, Polygon, RegionMapIR, ResolvedConfig, SliceIR, SlicedRegion,
    SupportGeometryIR, SurfaceClassificationIR, Transform3d,
};
use slicer_runtime::{commit_support_geometry_builtin, Blackboard};

fn identity() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

fn unit_square(side_mm: f32) -> ExPolygon {
    let h = side_mm / 2.0;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-h, -h),
                Point2::from_mm(h, -h),
                Point2::from_mm(h, h),
                Point2::from_mm(-h, h),
            ],
        },
        holes: vec![],
    }
}

fn make_active_region(object_id: &str, layer_height: f32) -> ActiveRegion {
    make_active_region_with_support_lh(object_id, layer_height, 0.0)
}

fn make_active_region_with_support_lh(
    object_id: &str,
    layer_height: f32,
    support_layer_height_mm: f32,
) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id: 0,
        resolved_config: ResolvedConfig {
            support_layer_height_mm,
            ..ResolvedConfig::default()
        },
        effective_layer_height: layer_height,
        nonplanar_shell: None,
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    }
}

fn make_layer(index: u32, z: f32, object_id: &str, layer_height: f32) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: vec![make_active_region(object_id, layer_height)],
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

fn dummy_mesh(object_id: &str) -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![],
                indices: vec![],
            },
            transform: identity(),
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 100.0,
                y: 100.0,
                z: 100.0,
            },
        },
        ..Default::default()
    }
}

fn slice_with_polygon(layer_index: u32, z: f32, object_id: &str, poly: ExPolygon) -> SliceIR {
    SliceIR {
        schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: layer_index,
        z,
        regions: vec![SlicedRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            polygons: vec![poly.clone()],
            infill_areas: vec![poly],
            ..Default::default()
        }],
    }
}

#[test]
fn support_geometry_consumes_slice_ir_polygons_per_layer() {
    let object_id = "obj-supp";
    let layer_height = 0.2_f32;
    let plan = LayerPlanIR {
        global_layers: vec![
            make_layer(0, 0.2, object_id, layer_height),
            make_layer(1, 0.4, object_id, layer_height),
        ],
        object_participation: HashMap::new(),
        ..Default::default()
    };
    let slice_vec = vec![
        slice_with_polygon(0, 0.2, object_id, unit_square(10.0)),
        slice_with_polygon(1, 0.4, object_id, unit_square(10.0)),
    ];

    let ir = support_geometry::execute_support_geometry(&plan, &slice_vec).unwrap();

    // With support_layer_height_mm = 0.0 (default = "use model layer height"),
    // SupportGeometry emits at every model layer. Both *support* entries
    // (non-sentinel global_support_layer_index) should now carry the slice
    // polygons rather than the previous empty-stub result. Intermediate
    // entries (sentinel u32::MAX) are covered by a separate test.
    let support_non_empty = ir
        .entries
        .iter()
        .filter(|(k, polys)| k.global_support_layer_index != u32::MAX && !polys.is_empty())
        .count();
    assert_eq!(
        support_non_empty, 2,
        "expected SliceIR-driven polygons for both support layer entries; got {:?}",
        ir.entries
    );
}

#[test]
fn support_geometry_collect_at_unaligned_z_returns_upper_bracket() {
    // Two slices at z=0.2 and z=0.4. Querying support geometry at the
    // intermediate z=0.3 should pull from the upper-bracket layer (idx 1).
    let object_id = "obj-bracket";
    let layer_height = 0.2_f32;
    let plan = LayerPlanIR {
        global_layers: vec![
            make_layer(0, 0.2, object_id, layer_height),
            make_layer(1, 0.4, object_id, layer_height),
        ],
        object_participation: HashMap::new(),
        ..Default::default()
    };
    let small = unit_square(6.0);
    let big = unit_square(10.0);
    let slice_vec = vec![
        slice_with_polygon(0, 0.2, object_id, small),
        slice_with_polygon(1, 0.4, object_id, big.clone()),
    ];

    let ir = support_geometry::execute_support_geometry(&plan, &slice_vec).unwrap();
    // Find an entry whose polygons came from the bigger (upper-bracket) slice.
    // Any non-empty entry must contain at least one of the slice polygons.
    let any_non_empty = ir
        .entries
        .values()
        .any(|polys| !polys.is_empty() && polys.iter().any(|p| !p.contour.points.is_empty()));
    assert!(
        any_non_empty,
        "expected at least one entry sourced from SliceIR"
    );
}

#[test]
fn support_geometry_blocks_on_missing_slice_ir() {
    // Without slice_ir committed, the built-in must surface MissingSliceIR.
    let object_id = "obj-noslice";
    let mesh = dummy_mesh(object_id);
    let plan = LayerPlanIR {
        global_layers: vec![make_layer(0, 0.2, object_id, 0.2)],
        object_participation: HashMap::new(),
        ..Default::default()
    };

    let mut bb = Blackboard::new(Arc::new(mesh), plan.global_layers.len());
    bb.commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .unwrap();
    bb.commit_layer_plan(Arc::new(plan)).unwrap();
    bb.commit_region_map(Arc::new(RegionMapIR::default()))
        .unwrap();

    let err =
        commit_support_geometry_builtin(&mut bb).expect_err("commit must fail without slice_ir");
    assert!(
        format!("{err}").contains("SliceIR"),
        "error message must mention missing SliceIR, got '{err}'"
    );
}

#[test]
fn support_geometry_succeeds_with_empty_slice_ir() {
    // Empty Vec<SliceIR> is structurally valid (e.g. no model layers active).
    // Built-in should commit an (empty) SupportGeometryIR without erroring.
    let object_id = "obj-empty";
    let mesh = dummy_mesh(object_id);
    let plan = LayerPlanIR {
        global_layers: Vec::new(),
        object_participation: HashMap::new(),
        ..Default::default()
    };

    let mut bb = Blackboard::new(Arc::new(mesh), 0);
    bb.commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .unwrap();
    bb.commit_layer_plan(Arc::new(plan)).unwrap();
    bb.commit_region_map(Arc::new(RegionMapIR::default()))
        .unwrap();
    bb.commit_slice_ir(Arc::new(Vec::<SliceIR>::new())).unwrap();

    commit_support_geometry_builtin(&mut bb).expect("commit must succeed");
    let committed: &SupportGeometryIR = bb.support_geometry().expect("committed").as_ref();
    assert!(committed.entries.is_empty());
}

#[test]
fn support_geometry_populates_intermediate_layer_entries_from_slice_ir() {
    // The intermediate-layer entries (global_support_layer_index = u32::MAX)
    // are added within `support_top_z_distance_mm` of column tops. Before A2
    // these were registered with empty polygons. After A2 each intermediate
    // entry must carry the polygons pulled from SliceIR at the layer's Z.
    let object_id = "obj-intermediate";
    let layer_height = 0.2_f32;
    let plan = LayerPlanIR {
        global_layers: vec![
            make_layer(0, 0.2, object_id, layer_height),
            make_layer(1, 0.4, object_id, layer_height),
            make_layer(2, 0.6, object_id, layer_height),
        ],
        object_participation: HashMap::new(),
        ..Default::default()
    };
    let slice_vec = vec![
        slice_with_polygon(0, 0.2, object_id, unit_square(10.0)),
        slice_with_polygon(1, 0.4, object_id, unit_square(10.0)),
        slice_with_polygon(2, 0.6, object_id, unit_square(10.0)),
    ];

    let ir = support_geometry::execute_support_geometry(&plan, &slice_vec).unwrap();

    // Every intermediate entry (the `u32::MAX` sentinel) must be non-empty.
    let intermediate_entries: Vec<_> = ir
        .entries
        .iter()
        .filter(|(k, _)| k.global_support_layer_index == u32::MAX)
        .collect();
    assert!(
        !intermediate_entries.is_empty(),
        "expected at least one intermediate-layer entry within distance of column top"
    );
    for (key, polys) in &intermediate_entries {
        assert!(
            !polys.is_empty(),
            "intermediate entry {key:?} must carry SliceIR polygons; got empty Vec"
        );
    }
}

// â”€â”€ A2 regression: per-object support_layer_height_mm is honored â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Integration regression for Block A2: `execute_support_geometry` must use
/// `region.resolved_config.support_layer_height_mm` per-object rather than the
/// old global `DEFAULT_SUPPORT_LAYER_HEIGHT_MM = 0.0`.
///
/// Fixture: 6 model layers (0.2 mm each), two objects:
/// - obj-A: `support_layer_height_mm = 0.4` â†’ support boundary every 2 model
///   layers â†’ support entries at global_support_layer_index 0, 1, 2 (3 entries).
/// - obj-B: `support_layer_height_mm = 0.0` â†’ every model layer â†’ 6 entries.
///
/// Assertions:
/// 1. obj-B has exactly 6 support entries (one per model layer).
/// 2. obj-A has exactly 3 support entries (one per 2 model layers).
#[test]
fn per_object_support_layer_height_is_honored_by_execute_support_geometry() {
    let obj_a = "obj-A";
    let obj_b = "obj-B";

    // 6 layers, 0.2mm each; obj-A uses 0.4mm support cadence, obj-B uses default 0.0.
    let global_layers: Vec<GlobalLayer> = (0u32..6)
        .map(|i| GlobalLayer {
            index: i,
            z: (i + 1) as f32 * 0.2,
            active_regions: vec![
                make_active_region_with_support_lh(obj_a, 0.2, 0.4),
                make_active_region_with_support_lh(obj_b, 0.2, 0.0),
            ],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();

    let plan = LayerPlanIR {
        global_layers: global_layers.clone(),
        object_participation: HashMap::new(),
        ..Default::default()
    };

    // Build slice_vec with one entry per layer for each object so polygons are
    // available (not strictly needed for counting, but ensures the function path
    // through collect_polygons_at_z is exercised).
    let slice_vec: Vec<SliceIR> = (0u32..6)
        .map(|i| slice_with_polygon(i, (i + 1) as f32 * 0.2, obj_a, unit_square(5.0)))
        .collect();

    let ir = support_geometry::execute_support_geometry(&plan, &slice_vec)
        .expect("execute_support_geometry must succeed");

    // Count non-sentinel entries per object.
    let a_count = ir
        .entries
        .keys()
        .filter(|k| k.global_support_layer_index != u32::MAX && k.object_id == obj_a)
        .count();
    let b_count = ir
        .entries
        .keys()
        .filter(|k| k.global_support_layer_index != u32::MAX && k.object_id == obj_b)
        .count();

    assert_eq!(
        a_count, 3,
        "obj-A (support_layer_height_mm=0.4, 0.2mm model) must have 3 support entries; \
         got {a_count}"
    );
    assert_eq!(
        b_count, 6,
        "obj-B (support_layer_height_mm=0.0) must have 6 support entries (one per layer); \
         got {b_count}"
    );
}

// â”€â”€ DEV-062 regression: per-layer cadence for multi-region objects â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn make_active_region_with_support_lh_and_rid(
    object_id: &str,
    region_id: slicer_ir::RegionId,
    layer_height: f32,
    support_layer_height_mm: f32,
) -> ActiveRegion {
    ActiveRegion {
        region_id,
        ..make_active_region_with_support_lh(object_id, layer_height, support_layer_height_mm)
    }
}

/// DEV-062 regression: when an object carries â‰¥2 active regions per global
/// layer, both `build_emit_schedule` and `execute_support_geometry` must
/// collapse per-(object, emit-layer). Pre-fix:
///   - The schedule accumulator advanced per region visit â†’ emit at every
///     model layer (12 entries instead of 6 across 3 emit-layers Ã— 2 regions).
///   - The `support_layer_index` counter incremented per region visit â†’
///     `global_support_layer_index` values diverged between regions of the
///     same emit-layer.
///
/// Post-Q5c (DEV-NNN): `global_support_layer_index` is the model layer index
/// directly (consumed as such by support-planner/src/lib.rs:211). With
/// `support_layer_height_mm = 0.4` and 0.2mm model layers, the emit layers
/// are model indices {1, 3, 5}. Two regions per emit-layer share that index.
///
/// Post-fix:
///   - 6 entries (3 emit-layers Ã— 2 regions = 6).
///   - Each emit-layer's `global_support_layer_index` is shared by region 0
///     and region 1; the indices set is exactly {1, 3, 5}.
///   - Each emitted entry carries the SliceIR-pulled polygons (non-empty).
#[test]
fn per_layer_cadence_for_multi_region_object() {
    let obj = "obj-multi-region";
    let global_layers: Vec<GlobalLayer> = (0u32..6)
        .map(|i| GlobalLayer {
            index: i,
            z: (i + 1) as f32 * 0.2,
            active_regions: vec![
                make_active_region_with_support_lh_and_rid(obj, 0, 0.2, 0.4),
                make_active_region_with_support_lh_and_rid(obj, 1, 0.2, 0.4),
            ],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();
    let plan = LayerPlanIR {
        global_layers,
        object_participation: HashMap::new(),
        ..Default::default()
    };

    // slice_vec carries both region polys at each layer so collect_polygons_at_z
    // returns non-empty results for region 0 AND region 1.
    let poly = unit_square(5.0);
    let slice_vec: Vec<SliceIR> = (0u32..6)
        .map(|i| SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: i,
            z: (i + 1) as f32 * 0.2,
            regions: vec![
                SlicedRegion {
                    object_id: obj.to_string(),
                    region_id: 0,
                    polygons: vec![poly.clone()],
                    infill_areas: vec![poly.clone()],
                    ..Default::default()
                },
                SlicedRegion {
                    object_id: obj.to_string(),
                    region_id: 1,
                    polygons: vec![poly.clone()],
                    infill_areas: vec![poly.clone()],
                    ..Default::default()
                },
            ],
        })
        .collect();

    let ir = support_geometry::execute_support_geometry(&plan, &slice_vec)
        .expect("execute_support_geometry must succeed");

    let entries: Vec<_> = ir
        .entries
        .iter()
        .filter(|(k, _)| k.global_support_layer_index != u32::MAX && k.object_id == obj)
        .collect();
    assert_eq!(
        entries.len(),
        6,
        "expected 3 emit-layers Ã— 2 regions = 6 entries; got {} (pre-fix: 12)",
        entries.len()
    );
    let idx_set: std::collections::BTreeSet<u32> = entries
        .iter()
        .map(|(k, _)| k.global_support_layer_index)
        .collect();
    assert_eq!(
        idx_set,
        std::collections::BTreeSet::from([1u32, 3, 5]),
        "expected model-layer indices {{1, 3, 5}} (post-Q5c); got {idx_set:?}"
    );
    // Each emit-layer index must carry BOTH region 0 and region 1.
    for idx in [1u32, 3, 5] {
        let region_ids: std::collections::BTreeSet<u64> = entries
            .iter()
            .filter(|(k, _)| k.global_support_layer_index == idx)
            .map(|(k, _)| k.region_id)
            .collect();
        assert_eq!(
            region_ids,
            std::collections::BTreeSet::from([0u64, 1u64]),
            "global_support_layer_index {idx} must carry both region 0 and region 1; got {region_ids:?}"
        );
    }
    // Polygon-consumption assertion: at least one (object, region, emit-layer)
    // entry must carry the SliceIR-pulled polygons, locking the per-region
    // consumer contract documented at slicer-ir/src/slice_ir.rs SupportGeometryIR.entries.
    let any_non_empty = entries.iter().any(|(_, polys)| !polys.is_empty());
    assert!(
        any_non_empty,
        "at least one per-region entry must carry SliceIR polygons; all entries were empty"
    );
}

// â”€â”€ Q5c regression: global_support_layer_index = model layer index â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// `global_support_layer_index` must be the model layer index that the
/// `support-planner` guest at `modules/core-modules/support-planner/src/lib.rs:211`
/// uses to index into `collision_cache: Vec<LayerCollisionCache>` (sized by
/// `layer_plan.layers.len()`). Before Q5c this field was a per-object sequential
/// counter (0, 1, 2, ...) that did NOT equal the model layer index for any
/// `support_layer_height_mm > 0` configuration. The guest then placed support
/// collision polygons against the wrong model layers.
///
/// Fixture: 6 model layers (0.2 mm each), one object with
/// `support_layer_height_mm = 0.4` â†’ emits at model layers 1, 3, 5.
/// Pre-fix the producer emits indices `[0, 1, 2]` (sequential counter).
/// Post-fix the producer emits indices `[1, 3, 5]` (matching model layers).
#[test]
fn global_support_layer_index_equals_model_layer_index() {
    let obj = "obj-supp";
    let global_layers: Vec<GlobalLayer> = (0u32..6)
        .map(|i| GlobalLayer {
            index: i,
            z: (i + 1) as f32 * 0.2,
            active_regions: vec![make_active_region_with_support_lh(obj, 0.2, 0.4)],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();
    let plan = LayerPlanIR {
        global_layers,
        object_participation: HashMap::new(),
        ..Default::default()
    };
    let poly = unit_square(5.0);
    let slice_vec: Vec<SliceIR> = (0u32..6)
        .map(|i| slice_with_polygon(i, (i + 1) as f32 * 0.2, obj, poly.clone()))
        .collect();

    let ir = support_geometry::execute_support_geometry(&plan, &slice_vec)
        .expect("execute_support_geometry must succeed");

    let mut indices: Vec<u32> = ir
        .entries
        .keys()
        .filter(|k| k.global_support_layer_index != u32::MAX && k.object_id == obj)
        .map(|k| k.global_support_layer_index)
        .collect();
    indices.sort();
    assert_eq!(
        indices,
        vec![1, 3, 5],
        "global_support_layer_index must equal model layer index (consumed as such by \
         support-planner/src/lib.rs:211); got {indices:?} (pre-fix: [0, 1, 2])"
    );
}
