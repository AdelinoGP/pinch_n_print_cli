//! Slice-level bridge detection: unsupported spans become `bridge_areas`.
//!
//! The mesh-analysis half of this file (packet 36-rev1's `bridge_regions`
//! clusterer: AC-1…AC-6 and NEG-3) was **retired** with the product in
//! perf-vs-orca ticket 150. `PrePass::MeshAnalysis` no longer emits
//! `BridgeRegion`s — no production reader consumed them — so those tests lost
//! their subject. The classifier tests that remain here drive
//! `execute_prepass_slice_single_layer_with_cache`, whose whole-set
//! `diff(this layer, previous layer)` bridge detection is a different,
//! live product.

use slicer_core::algos::prepass_slice::{
    execute_prepass_slice_single_layer_with_cache, PrepassSliceCache,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, MeshIR,
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

/// Build a `MeshIR` with a `width_mm x length_mm` cluster of down-facing
/// (normal_z < 0) overhang facets at z = 1.0 mm, rotated `rotation_deg`
/// about the rectangle's centroid, with L-shaped anchor walls.
///
/// The walls sit on the left long side and the bottom short side (C0-C3 and
/// C0-C1), so they share top vertices with the bridge facets.
///
/// When `with_top_surfaces == true`, add a few up-facing (normal_z > 0)
/// triangles elsewhere; they must NOT be treated as bridge facets.
fn bridge_mesh_with_l_walls(
    width_mm: f32,
    length_mm: f32,
    rotation_deg: f32,
    with_top_surfaces: bool,
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

    // Left wall (C0-C3): bridge Tri1's edge 0->3 gets a wall neighbour.
    indices.extend_from_slice(&[0, 3, 7]);
    indices.extend_from_slice(&[0, 7, 4]);
    // Bottom wall (C0-C1): bridge Tri0's edge 1->0 gets a wall neighbour.
    indices.extend_from_slice(&[0, 1, 5]);
    indices.extend_from_slice(&[0, 5, 4]);

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
        bridge_mesh_with_l_walls(5.0, 20.0, 0.0, false);

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
