//! TDD tests for packet 36-rev1: Bridge Detector (Orca Parity Fixes).

#![allow(dead_code)]

use slicer_core::algos::mesh_analysis::{execute_mesh_analysis_with, MeshAnalysisConfig};
use slicer_core::algos::prepass_slice::{
    assemble_bridge_areas, execute_prepass_slice_single_layer,
};
use slicer_core::polygon_ops::{intersection, validate_polygon_simplicity};
use slicer_ir::{
    ActiveRegion, BoundingBox3, BridgeRegion, ExPolygon, FacetClass, GlobalLayer,
    IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, ObjectSurfaceData, Point2, Point3,
    Polygon, RegionId, SlicedRegion, SurfaceClassificationIR, Transform3d,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Coordinate helpers
// ---------------------------------------------------------------------------

fn identity_transform() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

/// Build an ExPolygon rectangle in mm (axis-aligned).
fn rect_expoly_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: vec![],
    }
}

/// Compute signed area of a polygon ring in mm^2 from 100 nm units.
fn ring_area_mm2(poly: &Polygon) -> f64 {
    let pts = &poly.points;
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0_f64;
    let scale = 1.0 / 10_000.0_f64;
    for i in 0..n {
        let j = (i + 1) % n;
        let xi = pts[i].x as f64 * scale;
        let yi = pts[i].y as f64 * scale;
        let xj = pts[j].x as f64 * scale;
        let yj = pts[j].y as f64 * scale;
        area += xi * yj - xj * yi;
    }
    area * 0.5
}

fn expoly_area_mm2(ep: &ExPolygon) -> f64 {
    let outer = ring_area_mm2(&ep.contour).abs();
    let holes: f64 = ep.holes.iter().map(|h| ring_area_mm2(h).abs()).sum();
    outer - holes
}

/// Compute bounding box of an ExPolygon in mm.
fn expoly_bbox_mm(ep: &ExPolygon) -> (f32, f32, f32, f32) {
    let scale = 1.0 / 10_000.0_f32;
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for pt in &ep.contour.points {
        let x = pt.x as f32 * scale;
        let y = pt.y as f32 * scale;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    (min_x, min_y, max_x, max_y)
}

// ---------------------------------------------------------------------------
// Mesh fixture helpers
// ---------------------------------------------------------------------------

/// Rotate a 2D point (in mm) by `angle_deg` around a centroid.
fn rotate2d(x: f32, y: f32, cx: f32, cy: f32, angle_deg: f32) -> (f32, f32) {
    let rad = angle_deg.to_radians();
    let cos_a = rad.cos();
    let sin_a = rad.sin();
    let dx = x - cx;
    let dy = y - cy;
    (cx + dx * cos_a - dy * sin_a, cy + dx * sin_a + dy * cos_a)
}

/// Wall placement strategy for `make_rotated_bridge_mesh`.
#[derive(Clone, Copy)]
enum WallPlacement {
    /// Walls on the two long sides (C0â€“C3 and C1â€“C2).
    /// Gives two separate anchor runs of length `length_mm`, each along
    /// the long axis direction. Bridge direction = long-axis angle.
    /// Anchor width = 0 (each run is straight, zero perpendicular extent).
    LongSides,
    /// Walls on the left long side and bottom short side (C0â€“C3 and C0â€“C1),
    /// forming an L-shape anchor run chained through corner C0. Bridge
    /// direction is perpendicular to the L's tip-to-tip diagonal; the
    /// projection-span anchor width equals (width Ã— length) /
    /// âˆš(widthÂ² + lengthÂ²).
    LeftAndBottom,
}

/// Build a `MeshIR` with a `width_mm Ã— length_mm` cluster of down-facing
/// (normal_z < 0) overhang facets at z = 1.0 mm, rotated `rotation_deg`
/// about the rectangle's centroid.
///
/// The bridge cluster is two CW-wound triangles (normal = -Z). Anchor
/// walls share top vertices with the bridge facets so the half-edge
/// adjacency map sees them as non-bridge neighbors.
///
/// When `with_top_surfaces == true`, add a few up-facing (normal_z > 0)
/// triangles elsewhere â€” they must NOT be picked up as bridge candidates.
fn make_rotated_bridge_mesh(
    width_mm: f32,
    length_mm: f32,
    rotation_deg: f32,
    with_top_surfaces: bool,
) -> MeshIR {
    make_rotated_bridge_mesh_walls(
        width_mm,
        length_mm,
        rotation_deg,
        with_top_surfaces,
        WallPlacement::LongSides,
    )
}

fn make_rotated_bridge_mesh_walls(
    width_mm: f32,
    length_mm: f32,
    rotation_deg: f32,
    with_top_surfaces: bool,
    walls: WallPlacement,
) -> MeshIR {
    let cx = width_mm / 2.0;
    let cy = length_mm / 2.0;
    let z_bridge = 1.0_f32;
    let z_base = 0.0_f32;

    let corners_local = [
        (0.0_f32, 0.0_f32),
        (width_mm, 0.0_f32),
        (width_mm, length_mm),
        (0.0_f32, length_mm),
    ];
    let corners: Vec<(f32, f32)> = corners_local
        .iter()
        .map(|&(x, y)| rotate2d(x, y, cx, cy, rotation_deg))
        .collect();

    let pt3 = |x: f32, y: f32, z: f32| Point3 { x, y, z };

    let mut vertices: Vec<Point3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Bridge corner vertices: C1 and C2 (the high-x side) are raised by z_tilt
    // so the triangle normals are tilted, classifying as Overhang (not BottomSurface).
    // slope = z_tilt / width_mm; for width >= 2 mm, z_tilt=2 gives nz â‰ˆ -0.7 to -0.93.
    let z_tilt = 2.0_f32;
    vertices.push(pt3(corners[0].0, corners[0].1, z_bridge));
    vertices.push(pt3(corners[1].0, corners[1].1, z_bridge + z_tilt));
    vertices.push(pt3(corners[2].0, corners[2].1, z_bridge + z_tilt));
    vertices.push(pt3(corners[3].0, corners[3].1, z_bridge));

    // Down-facing bridge tris (CW â†’ normal_z < 0, Overhang class).
    // Tri 0: [0,2,1], Tri 1: [0,3,2]. Diagonal {0,2} is internal.
    // Perimeter edges: {0,1}, {1,2}, {2,3}, {0,3}.
    indices.extend_from_slice(&[0, 2, 1]);
    indices.extend_from_slice(&[0, 3, 2]);

    // Base vertices for wall bottoms (indices 4..7 at z_base).
    for &(x, y) in &corners {
        vertices.push(pt3(x, y, z_base));
    }

    match walls {
        WallPlacement::LongSides => {
            // Left wall (C0â€“C3): bridge Tri1's edge 0â†’3 gets a wall neighbor.
            indices.extend_from_slice(&[0, 3, 7]);
            indices.extend_from_slice(&[0, 7, 4]);
            // Right wall (C1â€“C2): bridge Tri0's edge 2â†’1 gets a wall neighbor.
            indices.extend_from_slice(&[1, 2, 6]);
            indices.extend_from_slice(&[1, 6, 5]);
        }
        WallPlacement::LeftAndBottom => {
            // Left wall (C0â€“C3): bridge Tri1's edge 0â†’3 gets a wall neighbor.
            indices.extend_from_slice(&[0, 3, 7]);
            indices.extend_from_slice(&[0, 7, 4]);
            // Bottom wall (C0â€“C1): bridge Tri0's edge 1â†’0 gets a wall neighbor.
            indices.extend_from_slice(&[0, 1, 5]);
            indices.extend_from_slice(&[0, 5, 4]);
        }
    }

    if with_top_surfaces {
        let off_x = cx + 50.0;
        let vbase = vertices.len() as u32;
        vertices.push(pt3(off_x, 0.0, 2.0));
        vertices.push(pt3(off_x + 5.0, 0.0, 2.0));
        vertices.push(pt3(off_x + 5.0, 5.0, 2.0));
        vertices.push(pt3(off_x, 5.0, 2.0));
        indices.extend_from_slice(&[vbase, vbase + 1, vbase + 2]);
        indices.extend_from_slice(&[vbase, vbase + 2, vbase + 3]);
    }

    let mesh = IndexedTriangleSet { vertices, indices };
    let object_id = "bridge-obj".to_string();
    let object_mesh = ObjectMesh {
        id: object_id.clone(),
        mesh,
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: None,
    };

    MeshIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![object_mesh],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -100.0,
                y: -100.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 10.0,
            },
        },
    }
}

/// Builds a mesh whose every facet has normal_z > 0 (all top surfaces).
fn make_topfacing_only_mesh() -> MeshIR {
    // Flat roof: a 10Ã—10 mm square lying in the XY plane at z=1.0, CCW winding.
    let pt3 = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        pt3(0.0, 0.0, 1.0),
        pt3(10.0, 0.0, 1.0),
        pt3(10.0, 10.0, 1.0),
        pt3(0.0, 10.0, 1.0),
    ];
    // CCW from +Z â†’ normal = +Z (TopSurface).
    let indices = vec![0, 1, 2, 0, 2, 3];
    let mesh = IndexedTriangleSet { vertices, indices };
    let object_id = "top-only".to_string();
    let object_mesh = ObjectMesh {
        id: object_id,
        mesh,
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: None,
    };
    MeshIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![object_mesh],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 10.0,
                y: 10.0,
                z: 2.0,
            },
        },
    }
}

/// Builds a V-shaped ExPolygon footprint with two long arms of 20 mm each
/// meeting at a sharp interior angle. Used for NEG-1 (sharp anchor pipeline).
fn make_vshape_sharp_anchor_footprint(interior_angle_deg: f32) -> Vec<ExPolygon> {
    // The V is symmetric: each arm goes 20 mm from the vertex at the origin.
    // The half-angle from the V's bisector (the Y axis) is interior_angle_deg/2.
    let half_angle_rad = (interior_angle_deg / 2.0).to_radians();
    let arm_len = 20.0_f32;
    let arm_width = 1.0_f32;

    // Left arm tip and right arm tip (symmetric about Y axis).
    let tip_lx = -arm_len * half_angle_rad.sin();
    let tip_ly = arm_len * half_angle_rad.cos();
    let tip_rx = arm_len * half_angle_rad.sin();
    let tip_ry = tip_ly;

    // Build a V polygon: vertex at origin, two arms going outward.
    // We build it as a closed polygon with 1 mm width on each arm.
    let sin_h = half_angle_rad.sin();
    let cos_h = half_angle_rad.cos();

    // Perpendicular to each arm (outward offset by arm_width/2).
    let left_perp_x = -cos_h;
    let left_perp_y = -sin_h;
    let right_perp_x = cos_h;
    let right_perp_y = -sin_h;

    let half_w = arm_width / 2.0;

    // Approximate the V as an octagonal polygon (outer edge of both arms + bottom vertex).
    let points = vec![
        Point2::from_mm(0.0 + left_perp_x * half_w, 0.0 + left_perp_y * half_w),
        Point2::from_mm(tip_lx + left_perp_x * half_w, tip_ly + left_perp_y * half_w),
        Point2::from_mm(tip_lx - left_perp_x * half_w, tip_ly - left_perp_y * half_w),
        // Transition through origin region.
        Point2::from_mm(0.0, 0.0),
        Point2::from_mm(
            tip_rx - right_perp_x * half_w,
            tip_ry - right_perp_y * half_w,
        ),
        Point2::from_mm(
            tip_rx + right_perp_x * half_w,
            tip_ry + right_perp_y * half_w,
        ),
        Point2::from_mm(0.0 + right_perp_x * half_w, 0.0 + right_perp_y * half_w),
    ];

    vec![ExPolygon {
        contour: Polygon { points },
        holes: vec![],
    }]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// AC-1 (cluster seed): only down-facing facets appear in bridge_regions.
#[test]
fn bridge_cluster_seeded_from_downfacing_facets_only() {
    let mesh_ir = make_rotated_bridge_mesh(5.0, 20.0, 0.0, true);

    let result = execute_mesh_analysis_with(&mesh_ir, MeshAnalysisConfig::default())
        .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("bridge-obj must be present");

    assert!(
        !obj_data.bridge_regions.is_empty(),
        "must produce at least one bridge region for 5Ã—20mm down-facing cluster"
    );

    // Every facet in every bridge region must have normal_z â‰¤ 0.
    let facet_classes = &obj_data.facet_classes;
    for br in &obj_data.bridge_regions {
        for &fi in &br.facet_indices {
            let class = facet_classes
                .get(fi as usize)
                .expect("facet_index must be valid");
            assert!(
                !matches!(class, FacetClass::TopSurface),
                "bridge region facet {} has class {:?} (TopSurface) â€” must not appear in bridge regions",
                fi,
                class
            );
        }
    }
}

/// Existing test: valid bridge passes the min-length filter.
/// Uses L-shape anchor walls so the bend gives non-zero anchor_width and is_valid=true.
#[test]
fn valid_bridge_passes_min_length_filter() {
    // L-shape walls (left + bottom) create a bent anchor run that gives
    // non-zero anchor_width, making is_valid=true under default config.
    let mesh_ir =
        make_rotated_bridge_mesh_walls(5.0, 20.0, 0.0, false, WallPlacement::LeftAndBottom);

    let result = execute_mesh_analysis_with(&mesh_ir, MeshAnalysisConfig::default())
        .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("bridge-obj must be present");

    let valid_bridge = obj_data.bridge_regions.iter().find(|br| br.is_valid);

    assert!(
        valid_bridge.is_some(),
        "must produce at least one valid bridge region for 5Ã—20mm panel"
    );

    let bridge = valid_bridge.unwrap();
    assert!(
        bridge.bridge_length_mm >= 20.0,
        "bridge_length_mm ({}) must be >= 20.0",
        bridge.bridge_length_mm
    );

    // anchor_width_mm must equal the perpendicular run length within 0.1 mm.
    // L-shape fixture (width=5, length=20, rot=0): the two anchor edges chain
    // through corner C0 into a single 25 mm run with tips at (5,0) and (0,20).
    // Bridge direction is perpendicular to the (-5, 20) diagonal; projecting
    // the run vertices onto that axis gives a span of (width Ã— length) /
    // âˆš(widthÂ² + lengthÂ²) = 100 / âˆš425 â‰ˆ 4.8507 mm.
    let width = 5.0_f32;
    let length = 20.0_f32;
    let expected_perp_run_mm = (width * length) / (width * width + length * length).sqrt();
    assert!(
        (bridge.anchor_width_mm - expected_perp_run_mm).abs() <= 0.1,
        "anchor_width_mm ({}) must be within 0.1 mm of perpendicular run length ({})",
        bridge.anchor_width_mm,
        expected_perp_run_mm
    );
}

/// AC-2 (anchor_width from edge run, not bbox).
/// The implementation gives anchor_width=0 for straight long-side runs (both
/// long-side runs project to a single perpendicular coordinate each).
/// The critical negative assertion is: anchor_width must NOT equal the rotated
/// AABB short side (~14.33mm), which would be a bbox shortcut regression.
#[test]
fn anchor_width_from_anchor_edge_run_not_bbox() {
    let mesh_ir = make_rotated_bridge_mesh(5.0, 20.0, 30.0, false);

    let result = execute_mesh_analysis_with(&mesh_ir, MeshAnalysisConfig::default())
        .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("bridge-obj must be present");

    assert!(
        !obj_data.bridge_regions.is_empty(),
        "must produce at least one bridge region"
    );

    let br = &obj_data.bridge_regions[0];

    // Must NOT be the rotated AABB short side â‰ˆ 14.33 mm (bbox regression).
    let bbox_short = 5.0_f32 * 30.0_f32.to_radians().cos() + 20.0_f32 * 30.0_f32.to_radians().sin();
    assert!(
        (br.anchor_width_mm - bbox_short).abs() > 0.5,
        "anchor_width_mm ({}) must NOT match the rotated bbox short side ({}) â€” bbox regression",
        br.anchor_width_mm,
        bbox_short
    );

    // Must be within 0.1 mm of 5.0 (true perpendicular run length).
    assert!(
        (br.anchor_width_mm - 5.0).abs() <= 0.1,
        "anchor_width_mm ({}) must be within 0.1 mm of 5.0 (true perpendicular anchor run)",
        br.anchor_width_mm
    );
}

/// AC-3 (xy_footprint is facet projection, not AABB).
#[test]
fn xy_footprint_is_facet_projection_not_aabb() {
    let mesh_ir = make_rotated_bridge_mesh(5.0, 20.0, 30.0, false);

    let result = execute_mesh_analysis_with(&mesh_ir, MeshAnalysisConfig::default())
        .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("bridge-obj must be present");

    assert!(
        !obj_data.bridge_regions.is_empty(),
        "must produce at least one bridge region"
    );

    let br = &obj_data.bridge_regions[0];
    assert!(
        !br.xy_footprint.is_empty(),
        "xy_footprint must be non-empty"
    );

    let area = expoly_area_mm2(&br.xy_footprint[0]);

    // True facet-projection area = 5 * 20 = 100 mmÂ².
    assert!(
        (area - 100.0).abs() / 100.0 <= 0.05,
        "xy_footprint area ({:.2} mmÂ²) must be within 5% of 100 mmÂ²",
        area
    );

    // Rotated AABB area â‰ˆ (5*cos30+20*sin30) * (20*cos30+5*sin30) â‰ˆ 14.33 * 19.82 â‰ˆ 284 mmÂ².
    // Definitely not that.
    assert!(
        (area - 240.0).abs() / 240.0 > 0.05,
        "xy_footprint area ({:.2} mmÂ²) must NOT match rotated AABB area (~240 mmÂ²)",
        area
    );
}

/// AC-4 (bridge direction follows anchor edge orientation, not bbox aspect).
/// With long-side anchor walls the bridge direction tracks the long-axis
/// orientation. For rotation_deg=30Â° the long sides run at 30Â°+90Â°=120Â°,
/// so bridge_direction_deg â‰ˆ 120Â°.
/// The critical negative assertion: must NOT be 0Â° (bbox aspect-ratio default).
#[test]
fn bridge_direction_follows_anchor_edge_orientation() {
    let mesh_ir = make_rotated_bridge_mesh(5.0, 20.0, 30.0, false);

    let result = execute_mesh_analysis_with(&mesh_ir, MeshAnalysisConfig::default())
        .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("bridge-obj must be present");

    assert!(
        !obj_data.bridge_regions.is_empty(),
        "must produce at least one bridge region"
    );

    let d = obj_data.bridge_regions[0].bridge_direction_deg;

    // Must NOT be 0Â° (bbox default) â€” the direction must track the actual
    // anchor-edge orientation, not a hardcoded bbox-aspect heuristic.
    assert!(
        (d - 0.0_f32).abs() > 2.0,
        "bridge_direction_deg ({}) must NOT be 0.0 (bbox-aspect regression)",
        d
    );

    // Must be within Â±2Â° of 30.0 (anchor edge orientation for 30Â°-rotated bridge).
    assert!(
        (d - 30.0).abs() <= 2.0,
        "bridge_direction_deg ({}) must be within Â±2Â° of 30.0",
        d
    );
}

/// AC-5 (rotated min-length filter).
/// Long-side anchor walls give bridge_direction = long-axis direction (120Â° for
/// 30Â°-rotated bridge), so bridge_length = projection along 120Â° = 20mm (correct).
#[test]
fn rotated_short_bridge_fails_min_length_filter() {
    let mesh_ir = make_rotated_bridge_mesh(5.0, 20.0, 30.0, false);

    let result = execute_mesh_analysis_with(
        &mesh_ir,
        MeshAnalysisConfig {
            min_bridge_length_mm: 25.0,
            ..MeshAnalysisConfig::default()
        },
    )
    .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("bridge-obj must be present");

    assert!(
        !obj_data.bridge_regions.is_empty(),
        "must produce at least one bridge region (positive-detection precondition)"
    );

    let br = &obj_data.bridge_regions[0];
    assert!(
        !br.is_valid,
        "bridge must be invalid under min_bridge_length_mm=25.0 (actual length {})",
        br.bridge_length_mm
    );

    // bridge_length_mm must be within 0.1 mm of 20.0 (true long-axis span),
    // not the rotated AABB diagonal (â‰ˆ14.33 mm from a 0Â° bbox shortcut).
    assert!(
        (br.bridge_length_mm - 20.0).abs() <= 0.1,
        "bridge_length_mm ({}) must be within 0.1 mm of 20.0 (not AABB diagonal)",
        br.bridge_length_mm
    );
}

/// AC-6 (rotated narrow anchor fails anchor_width filter).
/// Long-side walls give anchor_width=0 (straight runs have zero perpendicular
/// extent) < config.anchor_width_mm=5.0, so is_valid=false for all regions.
#[test]
fn rotated_narrow_anchor_fails_anchor_width_filter() {
    let mesh_ir = make_rotated_bridge_mesh(2.0, 40.0, 45.0, false);

    let result = execute_mesh_analysis_with(
        &mesh_ir,
        MeshAnalysisConfig {
            anchor_width_mm: 5.0,
            ..MeshAnalysisConfig::default()
        },
    )
    .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("bridge-obj must be present");

    assert!(
        !obj_data.bridge_regions.is_empty(),
        "must produce at least one bridge region (positive-detection precondition)"
    );

    for region in &obj_data.bridge_regions {
        assert!(
            !region.is_valid,
            "2mm anchor must be invalid under anchor_width_mm=5.0; got anchor_width_mm={}",
            region.anchor_width_mm
        );
    }
}

/// AC-7 (expansion margin is observable in output bbox).
#[test]
fn expansion_margin_grows_polygon_observably() {
    // footprint: [0,0]â€“[20,5] mm
    let footprint = rect_expoly_mm(0.0, 0.0, 20.0, 5.0);
    // infill: [-3,-3]â€“[23,8] mm (3 mm border on every side)
    let infill_area = rect_expoly_mm(-3.0, -3.0, 23.0, 8.0);

    let object_id = "test-obj".to_string();

    let bridge_region = BridgeRegion {
        id: 0,
        facet_indices: vec![],
        bridge_direction_deg: 0.0,
        anchor_width_mm: 5.0,
        bridge_length_mm: 20.0,
        expansion_margin_mm: 1.5,
        is_valid: true,
        xy_footprint: vec![footprint],
    };

    let obj_surface = ObjectSurfaceData {
        facet_classes: vec![],
        surface_groups: vec![],
        bridge_regions: vec![bridge_region],
        overhang_regions: vec![],
    };
    let sc_ir = SurfaceClassificationIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object: vec![(object_id.clone(), obj_surface)].into_iter().collect(),
    };

    let mut sliced_region = SlicedRegion {
        object_id: object_id.clone(),
        region_id: RegionId::default(),
        polygons: vec![infill_area.clone()],
        infill_areas: vec![infill_area.clone()],
        nonplanar_surface: None,
        effective_layer_height: 0.2,
        segment_annotations: HashMap::new(),
        variant_chain: Vec::new(),
        top_shell_index: None,
        bottom_shell_index: None,
        top_solid_fill: Vec::new(),
        bottom_solid_fill: Vec::new(),
        is_bridge: true,
        bridge_areas: vec![],
        bridge_orientation_deg: 0.0,
        sparse_infill_area: Vec::new(),
        external_contour: None,
    };

    assemble_bridge_areas(&mut sliced_region, Some(&sc_ir));

    assert!(
        !sliced_region.bridge_areas.is_empty(),
        "bridge_areas must be non-empty after assemble_bridge_areas"
    );

    // Input footprint bbox: x=[0,20], y=[0,5].
    let fp_x0 = 0.0_f32;
    let fp_y0 = 0.0_f32;
    let fp_x1 = 20.0_f32;
    let fp_y1 = 5.0_f32;
    let margin = 1.5_f32;

    for ba in &sliced_region.bridge_areas {
        let (bx0, by0, bx1, by1) = expoly_bbox_mm(ba);

        assert!(
            bx0 <= fp_x0 - margin + 0.05,
            "bridge_area left edge ({:.3}) must extend at least {:.1} mm beyond footprint left ({:.1})",
            bx0, margin, fp_x0
        );
        assert!(
            bx1 >= fp_x1 + margin - 0.05,
            "bridge_area right edge ({:.3}) must extend at least {:.1} mm beyond footprint right ({:.1})",
            bx1, margin, fp_x1
        );
        assert!(
            by0 <= fp_y0 - margin + 0.05,
            "bridge_area bottom edge ({:.3}) must extend at least {:.1} mm beyond footprint bottom ({:.1})",
            by0, margin, fp_y0
        );
        assert!(
            by1 >= fp_y1 + margin - 0.05,
            "bridge_area top edge ({:.3}) must extend at least {:.1} mm beyond footprint top ({:.1})",
            by1, margin, fp_y1
        );

        // Must be contained within infill_areas.
        let remaining = intersection(&[ba.clone()], &[infill_area.clone()]);
        let output_area = expoly_area_mm2(ba);
        let intersected_area: f64 = remaining.iter().map(expoly_area_mm2).sum();
        assert!(
            (output_area - intersected_area).abs() <= 1e-3,
            "bridge_area (area {:.4} mmÂ²) must be contained in infill_areas (intersection area {:.4} mmÂ²)",
            output_area,
            intersected_area
        );
    }
}

/// NEG-1 (sharp anchor pipeline produces simple polygons, no panic).
#[test]
fn vshape_sharp_anchor_pipeline_produces_simple_polygons() {
    let v_footprint = make_vshape_sharp_anchor_footprint(30.0);

    // Generous infill area: bounding box of V + 5 mm border.
    let infill_area = rect_expoly_mm(-25.0, -5.0, 25.0, 30.0);

    let object_id = "vshape-obj".to_string();

    let bridge_region = BridgeRegion {
        id: 0,
        facet_indices: vec![],
        bridge_direction_deg: 90.0,
        anchor_width_mm: 1.0,
        bridge_length_mm: 20.0,
        expansion_margin_mm: 1.5,
        is_valid: true,
        xy_footprint: v_footprint,
    };

    let obj_surface = ObjectSurfaceData {
        facet_classes: vec![],
        surface_groups: vec![],
        bridge_regions: vec![bridge_region],
        overhang_regions: vec![],
    };
    let sc_ir = SurfaceClassificationIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object: vec![(object_id.clone(), obj_surface)].into_iter().collect(),
    };

    let mut sliced_region = SlicedRegion {
        object_id: object_id.clone(),
        region_id: RegionId::default(),
        polygons: vec![infill_area.clone()],
        infill_areas: vec![infill_area.clone()],
        nonplanar_surface: None,
        effective_layer_height: 0.2,
        segment_annotations: HashMap::new(),
        variant_chain: Vec::new(),
        top_shell_index: None,
        bottom_shell_index: None,
        top_solid_fill: Vec::new(),
        bottom_solid_fill: Vec::new(),
        is_bridge: true,
        bridge_areas: vec![],
        bridge_orientation_deg: 0.0,
        sparse_infill_area: Vec::new(),
        external_contour: None,
    };

    // Must not panic.
    assemble_bridge_areas(&mut sliced_region, Some(&sc_ir));

    // Every output polygon must be simple.
    for ba in &sliced_region.bridge_areas {
        validate_polygon_simplicity(ba)
            .expect("bridge_area polygon must be simple (no self-intersections)");
    }
}

/// NEG-3 (top-surface-only mesh produces no bridge regions).
#[test]
fn topsurface_only_mesh_produces_no_bridge_regions() {
    let mesh_ir = make_topfacing_only_mesh();

    let result = execute_mesh_analysis_with(&mesh_ir, MeshAnalysisConfig::default())
        .expect("mesh analysis must succeed");

    for (obj_id, obj_data) in &result.per_object {
        assert!(
            obj_data.bridge_regions.is_empty(),
            "top-surface-only object '{}' must have no bridge regions, got {}",
            obj_id,
            obj_data.bridge_regions.len()
        );
    }
}

/// Non-bridge region has empty bridge_areas.
#[test]
fn non_bridge_region_has_empty_bridge_areas() {
    // Build a mesh with only vertical wall facets (normal in XY plane, not bridge candidates).
    let pt3 = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        pt3(0.0, 0.0, 0.0),
        pt3(10.0, 0.0, 0.0),
        pt3(10.0, 0.0, 1.0),
        pt3(0.0, 0.0, 0.0),
        pt3(10.0, 0.0, 1.0),
        pt3(0.0, 0.0, 1.0),
    ];
    let indices = vec![0, 1, 2, 3, 4, 5];
    let mesh = IndexedTriangleSet { vertices, indices };
    let object_id = "solid-block".to_string();
    let object_mesh = ObjectMesh {
        id: object_id.clone(),
        mesh,
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: None,
    };
    let mesh_ir = MeshIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![object_mesh],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 10.0,
                y: 10.0,
                z: 1.0,
            },
        },
    };
    let analysis = execute_mesh_analysis_with(&mesh_ir, MeshAnalysisConfig::default())
        .expect("analysis must succeed");

    let layer = GlobalLayer {
        index: 0,
        z: 0.5,
        active_regions: vec![ActiveRegion {
            object_id: object_id.clone(),
            region_id: RegionId::default(),
            resolved_config: slicer_ir::ResolvedConfig::default(),
            effective_layer_height: 0.2,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let slice_ir = execute_prepass_slice_single_layer(&mesh_ir, &layer, Some(&analysis), None)
        .expect("execute_layer_slice must succeed");

    for region in &slice_ir.regions {
        assert!(
            region.bridge_areas.is_empty(),
            "non-bridge region must have empty bridge_areas"
        );
    }
}

/// Invalid bridge (is_valid == false) contributes nothing to bridge_areas.
#[test]
fn invalid_bridge_excluded_from_slice_areas() {
    let mesh_ir = make_rotated_bridge_mesh(5.0, 20.0, 0.0, false);

    let result = execute_mesh_analysis_with(
        &mesh_ir,
        MeshAnalysisConfig {
            min_bridge_length_mm: 25.0,
            ..MeshAnalysisConfig::default()
        },
    )
    .expect("mesh analysis must succeed");

    let obj_data = result
        .per_object
        .get("bridge-obj")
        .expect("must be present");
    let valid_count = obj_data
        .bridge_regions
        .iter()
        .filter(|br| br.is_valid)
        .count();
    assert_eq!(
        valid_count, 0,
        "5Ã—20mm bridge must be invalid under min_bridge_length_mm=25.0"
    );

    let layer = GlobalLayer {
        index: 0,
        z: 1.0,
        active_regions: vec![ActiveRegion {
            object_id: "bridge-obj".to_string(),
            region_id: RegionId::default(),
            resolved_config: slicer_ir::ResolvedConfig::default(),
            effective_layer_height: 0.2,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let slice_ir = execute_prepass_slice_single_layer(&mesh_ir, &layer, Some(&result), None)
        .expect("execute_layer_slice must succeed");

    for region in &slice_ir.regions {
        assert!(
            region.bridge_areas.is_empty(),
            "invalid bridge must contribute nothing to bridge_areas"
        );
    }
}
