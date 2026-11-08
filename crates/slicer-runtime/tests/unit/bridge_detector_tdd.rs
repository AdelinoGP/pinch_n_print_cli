//! TDD tests for packet 36-rev1: Bridge Detector (Orca Parity Fixes).

#![allow(dead_code)]

use slicer_core::algos::mesh_analysis::{execute_mesh_analysis_with, MeshAnalysisConfig};
use slicer_core::algos::prepass_slice::{
    execute_prepass_slice_single_layer_with_cache, PrepassSliceCache,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, FacetClass, GlobalLayer, IndexedTriangleSet, MeshIR,
    ObjectConfig, ObjectMesh, Point2, Point3, Polygon, RegionId, Transform3d,
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
    // exhaustive: bridge mesh fixture explicitly defines every ObjectMesh field
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
    // exhaustive: top-facing mesh fixture explicitly defines every ObjectMesh field
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

/// Build an axis-aligned box `[x0,x1]×[y0,y1]×[z0,z1]` as a watertight
/// `MeshIR` with correct outward normals: the bottom face (z0) points −Z
/// (→ `FacetClass::BottomSurface`), the top face (z1) points +Z
/// (→ `TopSurface`), and the four vertical sides are `Normal` (nz = 0).
///
/// NOTE: unlike `overhang_annotation::tests::flat_cube_mesh` (whose bottom
/// winding yields a +Z normal — harmless there because only cross-sections
/// are taken), this helper deliberately reverses the bottom winding so facet
/// classification sees a genuine `BottomSurface`, which the mesh-analysis
/// fixtures still exercise.
fn make_box(id: &str, x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> MeshIR {
    let pt3 = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        pt3(x0, y0, z0), // 0
        pt3(x1, y0, z0), // 1
        pt3(x1, y1, z0), // 2
        pt3(x0, y1, z0), // 3
        pt3(x0, y0, z1), // 4
        pt3(x1, y0, z1), // 5
        pt3(x1, y1, z1), // 6
        pt3(x0, y1, z1), // 7
    ];
    #[rustfmt::skip]
    let indices = vec![
        // bottom (z0), outward normal −Z (CW from above)
        0, 2, 1,  0, 3, 2,
        // top (z1), outward normal +Z (CCW from above)
        4, 5, 6,  4, 6, 7,
        // sides (all vertical → Normal)
        0, 1, 5,  0, 5, 4,
        1, 2, 6,  1, 6, 5,
        2, 3, 7,  2, 7, 6,
        3, 0, 4,  3, 4, 7,
    ];
    let mesh = IndexedTriangleSet { vertices, indices };
    // exhaustive: box mesh fixture explicitly defines every ObjectMesh field
    let object_mesh = ObjectMesh {
        id: id.to_string(),
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
                x: x0 - 10.0,
                y: y0 - 10.0,
                z: 0.0,
            },
            max: Point3 {
                x: x1 + 10.0,
                y: y1 + 10.0,
                z: z1 + 10.0,
            },
        },
    }
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

/// AC-4 (mesh-analysis bridge direction is no longer heuristic-fed).
/// Bridge orientation is derived later from gated geometry by the pre-pass;
/// the mesh-analysis compatibility field remains at its default value.
#[test]
fn bridge_direction_stays_default_after_heuristic_retirement() {
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

    assert!(
        (d - 0.0_f32).abs() <= f32::EPSILON,
        "bridge_direction_deg ({}) must remain at its default after heuristic retirement",
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

/// A region whose cross-section is fully supported has empty bridge_areas
/// under the whole-set rule: `diff(current, previous)` is empty.
#[test]
fn non_bridge_region_has_empty_bridge_areas() {
    let box_mesh = make_box("solid-block", 0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
    let footprint: HashMap<String, Vec<ExPolygon>> = HashMap::from([(
        "solid-block".to_string(),
        vec![rect_expoly_mm(0.0, 0.0, 10.0, 10.0)],
    )]);

    // exhaustive: solid-block fixture explicitly defines every GlobalLayer field
    let layer = GlobalLayer {
        index: 5,
        z: 2.1,
        active_regions: vec![{
            // exhaustive: solid-block fixture explicitly defines every ActiveRegion field
            ActiveRegion {
                object_id: "solid-block".to_string(),
                region_id: RegionId::default(),
                resolved_config: slicer_ir::ResolvedConfig::default(),
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }
        }],
        has_nonplanar: false,
        is_sync_layer: false,
        is_raft: false,
    };

    let cache = PrepassSliceCache {
        raw_polygons: &footprint,
        prev_raw_polygons: Some(&footprint),
    };
    let slice_ir = execute_prepass_slice_single_layer_with_cache(&box_mesh, &layer, None, &cache)
        .expect("slice must succeed");

    let region = slice_ir.regions.first().expect("region must be sliced");
    assert!(
        !region.polygons.is_empty(),
        "fixture precondition: the box layer must have a cross-section"
    );
    assert!(
        region.bridge_areas.is_empty(),
        "a fully supported region must have empty bridge_areas"
    );
    assert!(!region.is_bridge);
}

/// A valid mesh bridge candidate still needs unsupported area on the current
/// layer. The whole-set classifier compares the raw cross-sections before
/// assigning bridge flow; a fully supported candidate must remain ordinary fill.
#[test]
fn supported_bridge_candidate_does_not_emit_bridge_fill() {
    let mesh_ir =
        make_rotated_bridge_mesh_walls(5.0, 20.0, 0.0, false, WallPlacement::LeftAndBottom);

    let current_area = rect_expoly_mm(0.0, 0.0, 5.0, 20.0);
    let previous_area = current_area.clone();
    let mut current_by_object = HashMap::new();
    current_by_object.insert("bridge-obj".to_string(), vec![current_area]);
    let mut previous_by_object = HashMap::new();
    previous_by_object.insert("bridge-obj".to_string(), vec![previous_area]);
    let cache = PrepassSliceCache {
        raw_polygons: &current_by_object,
        prev_raw_polygons: Some(&previous_by_object),
    };

    let layer = GlobalLayer {
        index: 1,
        z: 1.0,
        active_regions: vec![ActiveRegion {
            object_id: "bridge-obj".to_string(),
            effective_layer_height: 0.2,
            ..ActiveRegion::default()
        }],
        ..GlobalLayer::default()
    };

    let slice_ir = execute_prepass_slice_single_layer_with_cache(&mesh_ir, &layer, None, &cache)
        .expect("slice must succeed");
    let region = slice_ir.regions.first().expect("region must be sliced");

    assert!(
        !region.polygons.is_empty(),
        "fixture precondition: the current cross-section must be non-empty"
    );
    assert!(
        !region.is_bridge,
        "supported area must not be marked as bridge"
    );
    assert!(
        region.bridge_areas.is_empty(),
        "supported area must not receive bridge fill"
    );
}

/// WHOLE-SET POSITIVE: a perfectly horizontal unsupported span — a flat beam
/// over a gap — is a bridge, exactly like a sloped-free-edge bottom.
///
/// The beam's underside is a `BottomSurface` facet, but the classifier does not
/// look at facet classes: it derives the unsupported region as
/// `diff(this-layer cross-section, safety_offset(previous-layer))` — the port of
/// `PrintObject::detect_surfaces_type`'s `diff_ex(..., ApplySafetyOffset::Yes)`.
/// Here the beam's full 30×10 footprint sits over a previous layer of two end
/// pillars ([0,5] and [25,30]), so the diff is the 20×10 middle gap, which
/// canonical's opening (`0.1 × external_perimeter_width` = 0.04 mm at the
/// default width) keeps in full. The test drives the real cache path (as
/// `execute_prepass_slice_all_layers` does) and asserts `bridge_areas` +
/// `is_bridge` over that gap.
#[test]
fn flat_bridge_span_over_gap_flagged_via_layer_diff() {
    // Flat slab (beam) spanning x∈[0,30], y∈[0,10], underside at z=10.
    let slab = make_box("slab-obj", 0.0, 0.0, 10.0, 30.0, 10.0, 13.0);

    // exhaustive: flat-bridge fixture explicitly defines every GlobalLayer field
    let layer = GlobalLayer {
        index: 5,
        z: 11.5,
        active_regions: vec![{
            // exhaustive: flat-bridge fixture explicitly defines every ActiveRegion field
            ActiveRegion {
                object_id: "slab-obj".to_string(),
                region_id: RegionId::default(),
                resolved_config: slicer_ir::ResolvedConfig::default(),
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            }
        }],
        has_nonplanar: false,
        is_sync_layer: false,
        is_raft: false,
    };

    // This layer's cross-section: the full beam. The previous layer: two end
    // pillars. Their diff is the 20×10 middle gap = the unsupported flat span.
    let current: HashMap<String, Vec<ExPolygon>> = HashMap::from([(
        "slab-obj".to_string(),
        vec![rect_expoly_mm(0.0, 0.0, 30.0, 10.0)],
    )]);
    let previous: HashMap<String, Vec<ExPolygon>> = HashMap::from([(
        "slab-obj".to_string(),
        vec![
            rect_expoly_mm(0.0, 0.0, 5.0, 10.0),
            rect_expoly_mm(25.0, 0.0, 30.0, 10.0),
        ],
    )]);
    let cache = PrepassSliceCache {
        raw_polygons: &current,
        prev_raw_polygons: Some(&previous),
    };

    let slice_ir = execute_prepass_slice_single_layer_with_cache(&slab, &layer, None, &cache)
        .expect("slice must succeed");

    let region = slice_ir
        .regions
        .iter()
        .find(|r| r.object_id == "slab-obj")
        .expect("slab region present");

    assert!(
        !region.bridge_areas.is_empty(),
        "flat unsupported span must produce non-empty bridge_areas (fix under test)"
    );
    assert!(
        region.is_bridge,
        "flat unsupported span must set is_bridge = true (fix under test)"
    );

    // The bridge area must be ≈ the gap span (20×10 = 200 mm²), i.e. the
    // unsupported region ∩ the region, NOT the whole beam (300 mm²).
    let total: f64 = region.bridge_areas.iter().map(expoly_area_mm2).sum();
    assert!(
        (total - 200.0).abs() / 200.0 <= 0.05,
        "bridge_areas total ({:.2} mm²) must be within 5% of the 200 mm² gap span",
        total
    );
}

/// WHOLE-SET NEGATIVE (packet 109 regression guard): a solid box on the build
/// plate must NOT be flagged as a bridge on its first layer or any bottom layer.
///
/// A constant-cross-section box never grows, so `diff(current, previous)` is
/// empty at every interior layer (the lower set is inflated by `SCALED_EPSILON`
/// before the difference, so a coincident boundary cannot leak a hairline ring).
/// The very first layer has **no** previous cross-section at all, and canonical
/// `detect_surfaces_type` demotes a bottom with `lower_layer == nullptr` to
/// `stBottom`, never a bridge. Both cases must produce no bridge.
#[test]
fn solid_box_bottom_layers_not_flagged_as_bridge() {
    let box_mesh = make_box("box-obj", 0.0, 0.0, 0.0, 10.0, 10.0, 10.0);

    let footprint: HashMap<String, Vec<ExPolygon>> = HashMap::from([(
        "box-obj".to_string(),
        vec![rect_expoly_mm(0.0, 0.0, 10.0, 10.0)],
    )]);

    // First layer (no predecessor → diff = whole cross-section) and a mid layer
    // (predecessor identical → diff = ∅). Neither may be flagged a bridge.
    let cases: [(u32, f32, Option<&HashMap<String, Vec<ExPolygon>>>); 2] =
        [(0, 0.1, None), (10, 2.1, Some(&footprint))];
    for (index, z, prev_raw_polygons) in cases {
        // exhaustive: solid-box fixture explicitly defines every GlobalLayer field
        let layer = GlobalLayer {
            index,
            z,
            active_regions: vec![{
                // exhaustive: solid-box fixture explicitly defines every ActiveRegion field
                ActiveRegion {
                    object_id: "box-obj".to_string(),
                    region_id: RegionId::default(),
                    resolved_config: slicer_ir::ResolvedConfig::default(),
                    effective_layer_height: 0.2,
                    nonplanar_shell: None,
                    is_catchup_layer: false,
                    catchup_z_bottom: 0.0,
                    tool_index: 0,
                }
            }],
            has_nonplanar: false,
            is_sync_layer: false,
            is_raft: false,
        };
        let cache = PrepassSliceCache {
            raw_polygons: &footprint,
            prev_raw_polygons,
        };
        let slice_ir =
            execute_prepass_slice_single_layer_with_cache(&box_mesh, &layer, None, &cache)
                .expect("slice must succeed");
        for region in &slice_ir.regions {
            assert!(
                region.bridge_areas.is_empty(),
                "solid box layer {index} (z={z}) must have empty bridge_areas \
                 (first-layer / plate-bottom guard)"
            );
            assert!(
                !region.is_bridge,
                "solid box layer {index} (z={z}) must not be flagged is_bridge"
            );
        }
    }
}
