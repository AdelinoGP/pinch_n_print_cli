#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::algos::prepass_slice::{
    classify_unsupported_bridge_areas, execute_prepass_slice_single_layer,
};
use slicer_core::polygon_ops::{difference, intersection};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, MeshIR, ObjectConfig,
    ObjectMesh, Point2, Point3, Polygon, RegionId, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SemVer, SlicedRegion, Transform3d,
};

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn identity_transform() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: p3(0.0, 0.0, 0.0),
        max: p3(200.0, 200.0, 200.0),
    }
}

fn cube_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    p3(0.0, 0.0, 0.0),
                    p3(10.0, 0.0, 0.0),
                    p3(10.0, 10.0, 0.0),
                    p3(0.0, 10.0, 0.0),
                    p3(0.0, 0.0, 10.0),
                    p3(10.0, 0.0, 10.0),
                    p3(10.0, 10.0, 10.0),
                    p3(0.0, 10.0, 10.0),
                ],
                indices: vec![
                    0, 1, 2, 0, 2, 3, // bottom (z=0)
                    4, 6, 5, 4, 7, 6, // top (z=10)
                    0, 4, 5, 0, 5, 1, // front
                    1, 5, 6, 1, 6, 2, // right
                    2, 6, 7, 2, 7, 3, // back
                    3, 7, 4, 3, 4, 0, // left
                ],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            ..Default::default()
        }],
        build_volume: build_volume(),
    }
}

fn make_global_layer(index: u32, z: f32, object_id: &str) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: vec![ActiveRegion {
            object_id: object_id.to_string(),
            region_id: RegionId::default(),
            resolved_config: slicer_ir::ResolvedConfig::default(),
            effective_layer_height: 0.2,
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn slice_at_mid_height_produces_nonempty_polygons() {
    let mesh = cube_mesh();
    let layer = make_global_layer(0, 5.0, "cube");

    let result =
        execute_prepass_slice_single_layer(&mesh, &layer, None).expect("slice must succeed");

    assert_eq!(result.global_layer_index, 0);
    assert!((result.z - 5.0).abs() < 1e-6);
    assert_eq!(result.regions.len(), 1);

    let region = &result.regions[0];
    assert_eq!(region.object_id, "cube");
    assert!(
        !region.polygons.is_empty(),
        "slice at z=5 through a 10mm cube must produce polygons"
    );
}

#[test]
fn slice_below_mesh_produces_empty_polygons() {
    let mesh = cube_mesh();
    let layer = make_global_layer(0, -1.0, "cube");

    let result =
        execute_prepass_slice_single_layer(&mesh, &layer, None).expect("slice must succeed");

    assert_eq!(result.regions.len(), 1);
    assert!(
        result.regions[0].polygons.is_empty(),
        "slice below mesh must produce no polygons"
    );
}

#[test]
fn slice_above_mesh_produces_empty_polygons() {
    let mesh = cube_mesh();
    let layer = make_global_layer(0, 15.0, "cube");

    let result =
        execute_prepass_slice_single_layer(&mesh, &layer, None).expect("slice must succeed");

    assert_eq!(result.regions.len(), 1);
    assert!(
        result.regions[0].polygons.is_empty(),
        "slice above mesh must produce no polygons"
    );
}

#[test]
fn unknown_object_returns_error() {
    let mesh = cube_mesh();
    let layer = make_global_layer(0, 5.0, "nonexistent");

    let err = execute_prepass_slice_single_layer(&mesh, &layer, None)
        .expect_err("must fail for unknown object");

    match err {
        slicer_core::algos::prepass_slice::LayerSliceError::UnknownObject {
            layer_index,
            ref object_id,
        } => {
            assert_eq!(layer_index, 0);
            assert_eq!(object_id, "nonexistent");
        }
        other => panic!("expected UnknownObject, got {other:?}"),
    }
}

fn square_expoly(cx: f32, cy: f32, half_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(cx - half_mm, cy - half_mm),
                Point2::from_mm(cx + half_mm, cy - half_mm),
                Point2::from_mm(cx + half_mm, cy + half_mm),
                Point2::from_mm(cx - half_mm, cy + half_mm),
            ],
        },
        holes: vec![],
    }
}

/// Canonical classification: every unsupported component that survives
/// `opening_ex(..., 0.1 x width, Miter, 3.0)` is a bridge. The enclosed gap and
/// the free-edge band deliberately coexist in one layer; the retired
/// enclosure gate kept only the gap, canonical keeps both (0.1 x 0.4 mm =
/// 0.04 mm removes nothing at a 4 mm feature width).
#[test]
fn whole_set_classification_keeps_gap_and_free_edge() {
    let body = square_expoly(2.0, 0.0, 22.0);
    let gap = square_expoly(0.0, 0.0, 2.0);
    let free_edge = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(20.0, -22.0),
                Point2::from_mm(24.0, -22.0),
                Point2::from_mm(24.0, 22.0),
                Point2::from_mm(20.0, 22.0),
            ],
        },
        holes: vec![],
    };
    let previous = difference(
        std::slice::from_ref(&body),
        &[gap.clone(), free_edge.clone()],
    );
    let mut region = SlicedRegion {
        polygons: vec![body.clone()],
        infill_areas: vec![body.clone()],
        ..Default::default()
    };
    classify_unsupported_bridge_areas(&mut region, &[body], Some(&previous), 0.4);

    assert!(region.is_bridge, "both components are canonical bridges");
    assert_eq!(
        region.bridge_areas.len(),
        2,
        "canonical keeps the enclosed gap AND the free-edge band; only the          opening (0.04 mm at a 0.4 mm width) may drop a component"
    );
    // 4x4 gap + 4x44 band = 192 mm2 (1 mm2 = 1e8 units2).
    let total_mm2: f64 = region
        .bridge_areas
        .iter()
        .map(expolygon_area_units2)
        .sum::<f64>()
        / 1.0e8;
    assert!(
        (total_mm2 - 192.0).abs() < 1.92,
        "opened bridge area must be 192 mm2 +/- 1%, got {total_mm2:.2} mm2"
    );
}

/// The canonical opening removes the sub-printable sliver tail: a feature
/// narrower than `2 x 0.1 x width` (0.08 mm at the default 0.4 mm width) is not
/// bridge area. A 0.06 mm-wide band vanishes, the 4 mm gap survives in full.
#[test]
fn whole_set_opening_drops_sub_width_sliver() {
    let body = square_expoly(0.0, 0.0, 30.0);
    let gap = square_expoly(0.0, 0.0, 2.0);
    let sliver = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(10.0, -25.0),
                Point2::from_mm(10.06, -25.0),
                Point2::from_mm(10.06, 25.0),
                Point2::from_mm(10.0, 25.0),
            ],
        },
        holes: vec![],
    };
    let previous = difference(std::slice::from_ref(&body), &[gap.clone(), sliver.clone()]);
    let mut region = SlicedRegion {
        polygons: vec![body.clone()],
        infill_areas: vec![body.clone()],
        ..Default::default()
    };
    classify_unsupported_bridge_areas(&mut region, &[body], Some(&previous), 0.4);

    assert!(region.is_bridge, "the 4 mm gap must survive the opening");
    assert_eq!(
        region.bridge_areas.len(),
        1,
        "the 0.06 mm sliver is narrower than 2 x 0.1 x 0.4 mm = 0.08 mm and must          be opened away; only the 4 mm gap remains"
    );
    assert!(
        intersection(&region.bridge_areas, std::slice::from_ref(&sliver)).is_empty(),
        "no part of the sub-width sliver may be reported as bridge area"
    );
    let total_mm2 = expolygon_area_units2(&region.bridge_areas[0]) / 1.0e8;
    assert!(
        (total_mm2 - 16.0).abs() < 0.16,
        "the surviving gap must be reported in full (16 mm2 +/- 1%), got {total_mm2:.2} mm2"
    );
}

/// The classifier clips the unsupported set to the region's own printable
/// area before opening; a shared object cross-section must not leak across a
/// region boundary.
#[test]
fn whole_set_classification_clips_to_the_region_infill_area() {
    let current = square_expoly(5.0, 5.0, 5.0); // x,y in [0,10]
    let region_area = square_expoly(2.5, 5.0, 2.5); // x in [0,5], y in [2.5,7.5]
    let mut region = SlicedRegion {
        polygons: vec![region_area.clone()],
        infill_areas: vec![region_area.clone()],
        ..Default::default()
    };
    classify_unsupported_bridge_areas(&mut region, &[current], Some(&[]), 0.4);

    assert!(region.is_bridge);
    let total_mm2: f64 = region
        .bridge_areas
        .iter()
        .map(expolygon_area_units2)
        .sum::<f64>()
        / 1.0e8;
    assert!(
        (total_mm2 - 25.0).abs() < 0.25,
        "the classified area must be clipped to the region's 5x5 mm infill area, got {total_mm2:.2} mm2"
    );
}

fn two_island_mesh() -> MeshIR {
    let mut vertices = Vec::with_capacity(16);
    let mut indices = Vec::with_capacity(36);
    for (x_min, x_max) in [(0.0_f32, 1.0_f32), (1.05_f32, 2.05_f32)] {
        let base = vertices.len() as u32;
        vertices.extend([
            p3(x_min, 0.0, 0.0),
            p3(x_max, 0.0, 0.0),
            p3(x_max, 1.0, 0.0),
            p3(x_min, 1.0, 0.0),
            p3(x_min, 0.0, 10.0),
            p3(x_max, 0.0, 10.0),
            p3(x_max, 1.0, 10.0),
            p3(x_min, 1.0, 10.0),
        ]);
        for index in [
            0_u32, 1, 2, 0, 2, 3, // bottom (z=0)
            4, 6, 5, 4, 7, 6, // top (z=10)
            0, 4, 5, 0, 5, 1, // front
            1, 5, 6, 1, 6, 2, // right
            2, 6, 7, 2, 7, 3, // back
            3, 7, 4, 3, 4, 0, // left
        ] {
            indices.push(base + index);
        }
    }

    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "islands".to_string(),
            mesh: IndexedTriangleSet { vertices, indices },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            ..Default::default()
        }],
        build_volume: build_volume(),
    }
}

fn region_map_with_config(config: ResolvedConfig) -> RegionMapIR {
    let key = RegionKey {
        global_layer_index: 0,
        object_id: "islands".to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    let mut region_map = RegionMapIR::default();
    let config = region_map.intern_config(config);
    region_map.entries.insert(
        key,
        RegionPlan {
            config,
            ..Default::default()
        },
    );
    region_map
}

fn expolygon_area_units2(expolygon: &ExPolygon) -> f64 {
    fn polygon_area_units2(polygon: &Polygon) -> f64 {
        let points = &polygon.points;
        if points.len() < 3 {
            return 0.0;
        }
        let twice_area: f64 = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let next = &points[(index + 1) % points.len()];
                point.x as f64 * next.y as f64 - next.x as f64 * point.y as f64
            })
            .sum();
        twice_area.abs() / 2.0
    }

    polygon_area_units2(&expolygon.contour)
        - expolygon.holes.iter().map(polygon_area_units2).sum::<f64>()
}

fn expolygon_bounds_units(expolygon: &ExPolygon) -> (i64, i64, i64, i64) {
    let mut points = expolygon.contour.points.iter();
    let first = points.next().expect("island contour must be non-empty");
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    (min_x, min_y, max_x, max_y)
}

fn expected_bounds_units(
    min_x_mm: f32,
    min_y_mm: f32,
    max_x_mm: f32,
    max_y_mm: f32,
) -> (i64, i64, i64, i64) {
    let expected_min = Point2::from_mm(min_x_mm, min_y_mm);
    let expected_max = Point2::from_mm(max_x_mm, max_y_mm);
    (
        expected_min.x,
        expected_min.y,
        expected_max.x,
        expected_max.y,
    )
}

#[test]
fn prepass_slice_closing_radius_gate_applies_only_when_positive() {
    let mesh = two_island_mesh();
    let layer = make_global_layer(0, 5.0, "islands");

    let zero_region_map = region_map_with_config(ResolvedConfig {
        slice_closing_radius: 0.0,
        ..ResolvedConfig::default()
    });
    let zero_radius = execute_prepass_slice_single_layer(&mesh, &layer, Some(&zero_region_map))
        .expect("zero-radius slice must succeed");
    assert_eq!(zero_radius.regions.len(), 1);
    let zero_polygons = &zero_radius.regions[0].polygons;
    assert_eq!(zero_polygons.len(), 2);
    let mut zero_bounds: Vec<_> = zero_polygons.iter().map(expolygon_bounds_units).collect();
    zero_bounds.sort_by(|left, right| left.0.partial_cmp(&right.0).expect("finite bounds"));
    assert_eq!(zero_bounds[0], expected_bounds_units(0.0, 0.0, 1.0, 1.0));
    assert_eq!(zero_bounds[1], expected_bounds_units(1.05, 0.0, 2.05, 1.0));

    let expected_unit_square = Point2::from_mm(1.0, 1.0);
    let expected_area_units2 = 2.0 * expected_unit_square.x as f64 * expected_unit_square.y as f64;
    assert_eq!(expected_area_units2, 200_000_000.0);
    let actual_area_units2: f64 = zero_polygons.iter().map(expolygon_area_units2).sum();
    assert_eq!(
        actual_area_units2, 200_000_000.0,
        "zero-radius area was {actual_area_units2} unit²"
    );

    let positive_region_map = region_map_with_config(ResolvedConfig {
        slice_closing_radius: 0.04,
        ..ResolvedConfig::default()
    });
    let positive_radius =
        execute_prepass_slice_single_layer(&mesh, &layer, Some(&positive_region_map))
            .expect("positive-radius slice must succeed");
    assert_eq!(positive_radius.regions.len(), 1);
    let positive_polygons = &positive_radius.regions[0].polygons;
    assert_eq!(positive_polygons.len(), 1);
    assert_eq!(
        expolygon_bounds_units(&positive_polygons[0]),
        expected_bounds_units(0.0, 0.0, 2.05, 1.0,)
    );
}
