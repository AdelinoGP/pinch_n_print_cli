//! Regression: paint segmentation must not leak one object's cross-section
//! into another object's BASE region.
//!
//! `execute_paint_segmentation` derived its BASE-chain polygons from
//! `layer_total_contours` — the union of EVERY region on the layer, across all
//! objects — and then emitted that same polygon set once per matching
//! `RegionKey`.  On a multi-object plate every object therefore received every
//! other object's cross-section, and every toolpath downstream was emitted once
//! per object.
//!
//! Host-only: `paint_segmentation` is gated behind the `host-algos` feature.

#![cfg(feature = "host-algos")]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_ir::{
    ConfigDelta, ConfigValue, ExPolygon, IndexedTriangleSet, MeshIR, ModifierScope,
    ModifierVolume, ObjectMesh, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SliceIR, SlicedRegion, Transform3d, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
};

const IDENTITY: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

const LAYER_Z_MM: f32 = 0.5;

/// Object A occupies y in [0, 10] mm; object B occupies y in [50, 60] mm.
/// Both span x in [0, 10] mm. The two Y bands are disjoint, so a polygon can
/// be attributed to its owning object by Y alone.
const A_Y0: f32 = 0.0;
const A_Y1: f32 = 10.0;
const B_Y0: f32 = 50.0;
const B_Y1: f32 = 60.0;
const X0: f32 = 0.0;
const X1: f32 = 10.0;

/// Axis-aligned box as an `IndexedTriangleSet` (8 verts / 12 tris).
fn box_mesh(x0: f32, x1: f32, y0: f32, y1: f32, z0: f32, z1: f32) -> IndexedTriangleSet {
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        v(x0, y0, z0),
        v(x1, y0, z0),
        v(x1, y1, z0),
        v(x0, y1, z0),
        v(x0, y0, z1),
        v(x1, y0, z1),
        v(x1, y1, z1),
        v(x0, y1, z1),
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, // bottom
        4, 5, 6, 4, 6, 7, // top
        0, 1, 5, 0, 5, 4, // -y
        1, 2, 6, 1, 6, 5, // +x
        2, 3, 7, 2, 7, 6, // +y
        3, 0, 4, 3, 4, 7, // -x
    ];
    IndexedTriangleSet { vertices, indices }
}

/// Axis-aligned rectangle in scaled units (1 unit = 100 nm), CCW.
fn square(x0: f32, x1: f32, y0: f32, y1: f32) -> ExPolygon {
    let p = |x: f32, y: f32| Point2 {
        x: slicer_ir::mm_to_units(x),
        y: slicer_ir::mm_to_units(y),
    };
    ExPolygon {
        contour: Polygon {
            points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)],
        },
        holes: Vec::new(),
    }
}

/// Support-enforcer modifier volume over object A's footprint.
///
/// Its only role here is to admit the mesh into the segmentation pipeline
/// (`mesh_has_any_paint`) and to drive the BASE `segment_annotations` branch —
/// the same shape as `resources/bridge_support_enforcers.3mf`.
fn support_enforcer_volume() -> ModifierVolume {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "subtype".to_owned(),
        ConfigValue::String("support_enforcer".to_owned()),
    );
    // exhaustive: `ModifierVolume` has no `Default` impl
    ModifierVolume {
        id: "mv_a".to_owned(),
        mesh: box_mesh(2.0, 8.0, A_Y0 + 2.0, A_Y1 - 2.0, 0.0, 2.0),
        config_delta: ConfigDelta { fields },
        priority: 0,
        applies_to: ModifierScope::Support,
    }
}

fn build_mesh() -> Arc<MeshIR> {
    let obj_a = ObjectMesh {
        id: "objA".to_owned(),
        mesh: box_mesh(X0, X1, A_Y0, A_Y1, 0.0, 2.0),
        transform: Transform3d { matrix: IDENTITY },
        modifier_volumes: vec![support_enforcer_volume()],
        ..Default::default()
    };
    let obj_b = ObjectMesh {
        id: "objB".to_owned(),
        mesh: box_mesh(X0, X1, B_Y0, B_Y1, 0.0, 2.0),
        transform: Transform3d { matrix: IDENTITY },
        ..Default::default()
    };
    Arc::new(MeshIR {
        objects: vec![obj_a, obj_b],
        ..Default::default()
    })
}

fn build_slice_ir() -> Arc<Vec<SliceIR>> {
    let region = |object_id: &str, poly: ExPolygon| SlicedRegion {
        object_id: object_id.to_owned(),
        region_id: 0,
        polygons: vec![poly.clone()],
        infill_areas: vec![poly],
        effective_layer_height: LAYER_Z_MM,
        ..Default::default()
    };
    Arc::new(vec![SliceIR {
        global_layer_index: 0,
        z: LAYER_Z_MM,
        regions: vec![
            region("objA", square(X0, X1, A_Y0, A_Y1)),
            region("objB", square(X0, X1, B_Y0, B_Y1)),
        ],
        ..Default::default()
    }])
}

fn build_region_map() -> Arc<RegionMapIR> {
    let mut entries = HashMap::new();
    for object_id in ["objA", "objB"] {
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: object_id.to_owned(),
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

fn y_range(polys: &[ExPolygon]) -> (i64, i64) {
    let mut lo = i64::MAX;
    let mut hi = i64::MIN;
    for ep in polys {
        for p in &ep.contour.points {
            lo = lo.min(p.y);
            hi = hi.max(p.y);
        }
    }
    (lo, hi)
}

/// Each object's BASE region must carry only its own cross-section.
#[test]
fn base_region_must_not_carry_other_objects_contours() {
    let out = execute_paint_segmentation(build_mesh(), build_slice_ir(), build_region_map())
        .expect("execute_paint_segmentation must succeed");

    assert_eq!(out.len(), 1, "one layer in, one layer out");
    let layer = &out[0];

    let a_lo = slicer_ir::mm_to_units(A_Y0);
    let a_hi = slicer_ir::mm_to_units(A_Y1);
    let b_lo = slicer_ir::mm_to_units(B_Y0);
    let b_hi = slicer_ir::mm_to_units(B_Y1);

    let mut seen_a = false;
    let mut seen_b = false;
    for region in &layer.regions {
        if !region.variant_chain.is_empty() {
            continue;
        }
        assert!(
            !region.polygons.is_empty(),
            "BASE region for {} lost its geometry",
            region.object_id
        );
        let (lo, hi) = y_range(&region.polygons);
        match region.object_id.as_str() {
            "objA" => {
                seen_a = true;
                assert!(
                    lo >= a_lo && hi <= a_hi,
                    "objA BASE region leaked geometry outside its own Y band: \
                     got y in [{lo}, {hi}], expected within [{a_lo}, {a_hi}] \
                     ({} polygons)",
                    region.polygons.len()
                );
            }
            "objB" => {
                seen_b = true;
                assert!(
                    lo >= b_lo && hi <= b_hi,
                    "objB BASE region leaked geometry outside its own Y band: \
                     got y in [{lo}, {hi}], expected within [{b_lo}, {b_hi}] \
                     ({} polygons)",
                    region.polygons.len()
                );
            }
            other => panic!("unexpected object_id in output: {other}"),
        }
    }
    assert!(seen_a, "objA must have a BASE region on the layer");
    assert!(seen_b, "objB must have a BASE region on the layer");
}

/// The modifier-volume `segment_annotations` are indexed positionally against
/// the region's own polygons, so a per-object BASE region must carry exactly
/// one annotation "perimeter" per polygon it owns — not one per layer polygon.
#[test]
fn base_region_annotations_must_match_its_own_polygon_count() {
    let out = execute_paint_segmentation(build_mesh(), build_slice_ir(), build_region_map())
        .expect("execute_paint_segmentation must succeed");
    let layer = &out[0];

    for region in &layer.regions {
        if !region.variant_chain.is_empty() {
            continue;
        }
        for (semantic, perimeters) in &region.segment_annotations {
            assert_eq!(
                perimeters.len(),
                region.polygons.len(),
                "region {} / {semantic:?}: annotation perimeter count must match \
                 the region's own polygon count",
                region.object_id
            );
        }
    }
}
