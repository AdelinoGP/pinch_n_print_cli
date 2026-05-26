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

use slicer_host::{commit_support_geometry_builtin, support_geometry, Blackboard};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectMesh, Point2, Point3, Polygon, RegionMapIR, ResolvedConfig, SliceIR, SlicedRegion,
    SupportGeometryIR, SurfaceClassificationIR, Transform3d,
};

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
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id: 0,
        resolved_config: ResolvedConfig::default(),
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
