//! TDD file for packet 12-rev1: external surface classification at slice.
//!
//! This file is authored in the RED state. It will NOT compile until:
//!   - Step 3 implements `slicer_host::layer_slice::classify_region_surfaces`
//!     (the function must be `pub` so integration tests in `tests/` can reach it)
//!   - Step 4 extends `execute_layer_slice` with three optional parameters:
//!     `surface_class`, `next_layer_z`, `prev_layer_z`
//!
//! Z window semantics (locked by AC):
//!   Top:    facet z_min ∈ [layer_z,    next_layer_z)   — inclusive low, exclusive high
//!   Bottom: facet z_max ∈ (prev_layer_z, layer_z]      — exclusive low, inclusive high
//!   Bridge: facet z range straddles layer_z (z_min ≤ layer_z ≤ z_max)
//!
//! Coordinate system: 1 unit = 100 nm = 10⁻⁴ mm.
//! Use `Point2::from_mm` / `mm_to_units` per docs/08_coordinate_system.md.

use std::collections::HashMap;

use slicer_host::execute_layer_slice; // Step 4 will extend this signature
use slicer_host::layer_slice::classify_region_surfaces; // Step 3 will implement this
use slicer_ir::{
    ActiveRegion, BoundingBox3, BridgeRegion, FacetClass, GlobalLayer, IndexedTriangleSet, MeshIR,
    ObjectConfig, ObjectMesh, ObjectSurfaceData, Point2, Point3, Polygon, ResolvedConfig,
    SurfaceClassificationIR, SurfaceGroup, Transform3d,
};

// ============================================================================
// Shared fixture helpers
// ============================================================================

/// Identity 4×4 column-major transform (no rotation / translation).
fn identity_transform() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

/// Minimal resolved config (copied from layer_slice_tdd.rs pattern).
fn default_resolved() -> ResolvedConfig {
    ResolvedConfig::default()
}

/// A flat horizontal triangle at Z=`z_top` with vertices in the XY plane
/// centred near (0, 0). The triangle has its normal pointing UP (TopSurface).
///
/// Vertices (mm):  (-5, -5, z_top), (5, -5, z_top), (0, 5, z_top)
/// Facet index: 0
fn flat_top_triangle(z_top: f32) -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -5.0,
                y: -5.0,
                z: z_top,
            },
            Point3 {
                x: 5.0,
                y: -5.0,
                z: z_top,
            },
            Point3 {
                x: 0.0,
                y: 5.0,
                z: z_top,
            },
        ],
        // CCW winding when viewed from above → upward normal → TopSurface
        indices: vec![0, 1, 2],
    }
}

/// A flat horizontal triangle at Z=`z_bot` with normal pointing DOWN (BottomSurface).
///
/// Winding is CW from above (reversed) → downward normal.
fn flat_bottom_triangle(z_bot: f32) -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -5.0,
                y: -5.0,
                z: z_bot,
            },
            Point3 {
                x: 5.0,
                y: -5.0,
                z: z_bot,
            },
            Point3 {
                x: 0.0,
                y: 5.0,
                z: z_bot,
            },
        ],
        // CW winding from above → downward normal → BottomSurface
        indices: vec![0, 2, 1],
    }
}

/// A slanted triangle that spans `z_lo` to `z_hi` (bridge geometry).
/// Centroid XY: roughly (0, 0).
fn bridge_triangle(z_lo: f32, z_hi: f32) -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -5.0,
                y: -5.0,
                z: z_lo,
            },
            Point3 {
                x: 5.0,
                y: -5.0,
                z: z_lo,
            },
            Point3 {
                x: 0.0,
                y: 5.0,
                z: z_hi,
            },
        ],
        indices: vec![0, 1, 2],
    }
}

/// A triangle well away from the region polygon (XY at 100 mm from origin).
fn far_triangle(z: f32) -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 95.0,
                y: 95.0,
                z,
            },
            Point3 {
                x: 105.0,
                y: 95.0,
                z,
            },
            Point3 {
                x: 100.0,
                y: 105.0,
                z,
            },
        ],
        indices: vec![0, 1, 2],
    }
}

/// Build an `ObjectMesh` for the given `IndexedTriangleSet`.
fn make_object_mesh(id: &str, mesh: IndexedTriangleSet) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh,
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    }
}

/// Build a `MeshIR` with a single object.
fn make_mesh_ir(object_id: &str, mesh: IndexedTriangleSet) -> MeshIR {
    MeshIR {
        objects: vec![make_object_mesh(object_id, mesh)],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -200.0,
                y: -200.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    }
}

/// Rectangle polygon centred at origin, covering ±10 mm in X and Y.
/// This covers centroids of the flat/bridge triangles (centroid ≈ (0, -1.67)).
fn region_polygon_covering_origin() -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(-10.0, -10.0),
            Point2::from_mm(10.0, -10.0),
            Point2::from_mm(10.0, 10.0),
            Point2::from_mm(-10.0, 10.0),
        ],
    }
}

/// Build a `SurfaceClassificationIR` for one object with a single-facet
/// classification and optional bridge region.
fn make_surface_class(
    object_id: &str,
    facet_class: FacetClass,
    bridge_facet_indices: Option<Vec<u32>>,
) -> SurfaceClassificationIR {
    let bridge_regions = match bridge_facet_indices {
        Some(indices) => vec![BridgeRegion {
            id: 0,
            facet_indices: indices,
            bridge_direction_deg: 0.0,
            anchor_width_mm: 0.0,
            bridge_length_mm: 0.0,
            expansion_margin_mm: 0.0,
            is_valid: false,
            xy_footprint: vec![],
        }],
        None => vec![],
    };

    let per_object_data = ObjectSurfaceData {
        facet_classes: vec![facet_class],
        surface_groups: vec![SurfaceGroup {
            id: 0,
            facet_indices: vec![0],
            z_min: 0.0,
            z_max: 1.0,
            area_mm2: 25.0,
            printable: true,
            shell_count: 1,
        }],
        bridge_regions,
        overhang_regions: vec![],
    };

    let mut per_object = HashMap::new();
    per_object.insert(object_id.to_string(), per_object_data);

    SurfaceClassificationIR {
        per_object,
        ..Default::default()
    }
}

/// Build a `GlobalLayer` with one `ActiveRegion` for the given object.
fn make_layer(index: u32, z: f32, object_id: &str) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: vec![ActiveRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            resolved_config: default_resolved(),
            effective_layer_height: 0.2,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

// ============================================================================
// Helper-level tests (classify_region_surfaces)
// ============================================================================
// NOTE: classify_region_surfaces is `pub` in layer_slice so integration tests
// in tests/ can reach it directly. Step 3 implements the body.

/// AC-1: A TopSurface facet whose z_min is in [layer_z, next_layer_z) and whose
/// centroid XY lies inside the region polygon → is_top_surface=true, others false.
#[test]
fn top_surface_facet_within_window_flags_top() {
    // layer_z = 1.0 mm, next_layer_z = 1.2 mm
    // Triangle lies flat at z = 1.0 mm → z_min = z_max = 1.0 mm
    // z_min (1.0) ∈ [1.0, 1.2) → satisfies top-surface window
    let mesh_ir = make_mesh_ir("obj-a", flat_top_triangle(1.0));
    let surface_class = make_surface_class("obj-a", FacetClass::TopSurface, None);
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object["obj-a"],
        &polygons,
        1.0,       // layer_z
        Some(1.2), // next_layer_z
        Some(0.8), // prev_layer_z
        1,         // top_shell_layers
        1,         // bottom_shell_layers
    );

    assert!(is_top, "expected is_top_surface=true");
    assert!(!is_bot, "expected is_bottom_surface=false");
    assert!(!is_bridge, "expected is_bridge=false");
}

/// AC-2: A BottomSurface facet whose z_max is in (prev_layer_z, layer_z] and
/// whose centroid XY lies inside the region polygon → is_bottom_surface=true.
#[test]
fn bottom_surface_facet_within_window_flags_bottom() {
    // layer_z = 1.0 mm, prev_layer_z = 0.8 mm
    // Triangle at z = 1.0 mm → z_max = 1.0 mm
    // z_max (1.0) ∈ (0.8, 1.0] → satisfies bottom-surface window
    let mesh_ir = make_mesh_ir("obj-b", flat_bottom_triangle(1.0));
    let surface_class = make_surface_class("obj-b", FacetClass::BottomSurface, None);
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object["obj-b"],
        &polygons,
        1.0,       // layer_z
        Some(1.2), // next_layer_z
        Some(0.8), // prev_layer_z
        1,         // top_shell_layers
        1,         // bottom_shell_layers
    );

    assert!(!is_top, "expected is_top_surface=false");
    assert!(is_bot, "expected is_bottom_surface=true");
    assert!(!is_bridge, "expected is_bridge=false");
}

/// AC-3: A facet listed in bridge_regions whose Z range straddles layer_z and
/// whose centroid XY is in the region polygon → is_bridge=true.
#[test]
fn bridge_facet_in_z_span_flags_bridge() {
    // layer_z = 1.0 mm; bridge triangle spans [0.8, 1.2] → straddles 1.0
    // facet_index 0 is listed in BridgeRegion
    let mesh_ir = make_mesh_ir("obj-c", bridge_triangle(0.8, 1.2));
    let surface_class = make_surface_class("obj-c", FacetClass::Bridge, Some(vec![0]));
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object["obj-c"],
        &polygons,
        1.0,       // layer_z
        Some(1.2), // next_layer_z
        Some(0.8), // prev_layer_z
        1,         // top_shell_layers
        1,         // bottom_shell_layers
    );

    assert!(!is_top, "expected is_top_surface=false");
    assert!(!is_bot, "expected is_bottom_surface=false");
    assert!(is_bridge, "expected is_bridge=true");
}

/// AC-4: A TopSurface facet whose Z window matches but ALL vertices are
/// OUTSIDE the region polygon → none of the flags should be set.
#[test]
fn top_facet_outside_polygon_does_not_flag_top() {
    // Triangle at z=1.0 mm, but vertices are at XY ≈ (100 mm) — far from origin
    let mesh_ir = make_mesh_ir("obj-d", far_triangle(1.0));
    let surface_class = make_surface_class("obj-d", FacetClass::TopSurface, None);
    // Polygon covers only origin area; far triangle is outside
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object["obj-d"],
        &polygons,
        1.0,       // layer_z
        Some(1.2), // next_layer_z
        Some(0.8), // prev_layer_z
        1,         // top_shell_layers
        1,         // bottom_shell_layers
    );

    assert!(
        !is_top,
        "expected is_top_surface=false (centroid outside polygon)"
    );
    assert!(!is_bot, "expected is_bottom_surface=false");
    assert!(!is_bridge, "expected is_bridge=false");
}

/// AC-5: A TopSurface facet whose z_min is ABOVE next_layer_z (outside window)
/// → none of the flags should be set, even if centroid is in polygon.
#[test]
fn top_facet_outside_z_window_does_not_flag_top() {
    // Triangle at z = 5.0 mm; layer_z=1.0, next_layer_z=1.2
    // z_min (5.0) ≥ next_layer_z (1.2) → OUTSIDE the [1.0, 1.2) window
    let mesh_ir = make_mesh_ir("obj-e", flat_top_triangle(5.0));
    let surface_class = make_surface_class("obj-e", FacetClass::TopSurface, None);
    let polygons = vec![region_polygon_covering_origin()];

    let (is_top, is_bot, is_bridge) = classify_region_surfaces(
        &mesh_ir.objects[0],
        &surface_class.per_object["obj-e"],
        &polygons,
        1.0,       // layer_z
        Some(1.2), // next_layer_z
        Some(0.8), // prev_layer_z
        1,         // top_shell_layers
        1,         // bottom_shell_layers
    );

    assert!(
        !is_top,
        "expected is_top_surface=false (z_min outside window)"
    );
    assert!(!is_bot, "expected is_bottom_surface=false");
    assert!(!is_bridge, "expected is_bridge=false");
}

// ============================================================================
// execute_layer_slice-level tests (extended signature from Step 4)
// ============================================================================
// The extended signature is:
//   pub fn execute_layer_slice(
//       mesh: &MeshIR,
//       layer: &GlobalLayer,
//       surface_class: Option<&SurfaceClassificationIR>,
//       next_layer_z: Option<f32>,
//       prev_layer_z: Option<f32>,
//   ) -> Result<SliceIR, LayerSliceError>

/// AC-6: When a SurfaceClassificationIR with one TopSurface facet at layer top
/// is provided, the resulting SliceIR region must have is_top_surface=true and
/// the other two flags false.
#[test]
fn execute_layer_slice_writes_top_flag_on_sliced_region() {
    // Geometry: a unit box spanning z∈[0,1].  Slicing at z=0.5 produces
    // a square cross-section region near the origin.
    //
    // The "top cap" face is a flat triangle at z=1.0.  That face has
    // z_min = z_max = 1.0, which lies in the window [1.0, 1.2) for the
    // LAST layer (layer_z=1.0, next_layer_z=1.2).
    //
    // We deliberately do NOT slice at z=0.5 here; we slice at z=1.0 so
    // the flat-top facet's z_min=1.0 satisfies layer_z ≤ z_min < next_layer_z.
    // The tetrahedron will not produce a cross-section at exactly z=1.0
    // (it's a vertex), so we use a proper prism / flat-topped mesh instead.
    //
    // Simplest valid fixture: a flat triangle at z = layer_z with centroid
    // near origin, plus a closing base triangle so the mesh is closed.
    // flat_top_triangle(1.0) produces exactly that (see helper).
    let object_id = "obj-top";
    let mesh = IndexedTriangleSet {
        // Thin slab: base at z=0, cap at z=1.
        // Vertices: base triangle + cap triangle (same XY, offset Z)
        vertices: vec![
            Point3 {
                x: -5.0,
                y: -5.0,
                z: 0.0,
            }, // 0 base
            Point3 {
                x: 5.0,
                y: -5.0,
                z: 0.0,
            }, // 1 base
            Point3 {
                x: 0.0,
                y: 5.0,
                z: 0.0,
            }, // 2 base
            Point3 {
                x: -5.0,
                y: -5.0,
                z: 1.0,
            }, // 3 cap
            Point3 {
                x: 5.0,
                y: -5.0,
                z: 1.0,
            }, // 4 cap
            Point3 {
                x: 0.0,
                y: 5.0,
                z: 1.0,
            }, // 5 cap
        ],
        // bottom face (facet 0), top face (facet 1), three side faces
        indices: vec![
            0, 1, 2, // facet 0: base (BottomSurface, z_max=0)
            3, 5, 4, // facet 1: cap  (TopSurface,    z_min=1.0)
            0, 1, 4, 0, 4, 3, // side 1
            1, 2, 5, 1, 5, 4, // side 2
            2, 0, 3, 2, 3, 5, // side 3
        ],
    };
    let mesh_ir = make_mesh_ir(object_id, mesh.clone());

    // Only facet 1 (the cap at z=1.0) is marked TopSurface.
    let facet_count = mesh.indices.len() / 3;
    let mut facet_classes = vec![FacetClass::BottomSurface; facet_count];
    facet_classes[1] = FacetClass::TopSurface;

    let per_object_data = ObjectSurfaceData {
        facet_classes,
        surface_groups: vec![SurfaceGroup {
            id: 0,
            facet_indices: (0..facet_count as u32).collect(),
            z_min: 0.0,
            z_max: 1.0,
            area_mm2: 100.0,
            printable: true,
            shell_count: 1,
        }],
        bridge_regions: vec![],
        overhang_regions: vec![],
    };
    let mut per_object = HashMap::new();
    per_object.insert(object_id.to_string(), per_object_data);
    let surface_class = SurfaceClassificationIR {
        per_object,
        ..Default::default()
    };

    // layer_z = 0.5 mm; next_layer_z = 0.7 mm.
    // The cap facet has z_min = 1.0, which is NOT in [0.5, 0.7) — so we use
    // a layer near the top of the slab where z_min=1.0 is in [1.0, 1.2).
    // Slicing at z=1.0 gives a degenerate cross-section at the apex;
    // to get a real cross-section we slice slightly below: z=0.9 mm.
    // The cap facet z_min=1.0 ∈ [0.9, 1.1) → qualifies.
    let layer = make_layer(0, 0.9, object_id);

    // Step 4 extends execute_layer_slice to accept these three extra params.
    // layer_z=0.9, next_layer_z=1.1 → window [0.9, 1.1); cap z_min=1.0 ∈ [0.9,1.1) ✓
    let slice = execute_layer_slice(
        &mesh_ir,
        &layer,
        Some(&surface_class), // surface_class: Option<&SurfaceClassificationIR>
        Some(1.1_f32),        // next_layer_z
        Some(0.7_f32),        // prev_layer_z
        None,                 // region_map
        None,                 // layer_plan
    )
    .expect("slice should succeed");

    assert_eq!(slice.regions.len(), 1, "expected exactly one sliced region");
    let region = &slice.regions[0];
    assert!(
        region.top_shell_index == Some(0),
        "expected top_shell_index=Some(0) when TopSurface facet classification supplied"
    );
    assert!(
        region.bottom_shell_index.is_none(),
        "expected bottom_shell_index=None"
    );
    assert!(!region.is_bridge, "expected is_bridge=false");
}

/// AC-7: When no surface classification is provided (all three extra params are None),
/// every region's three flags must be false — preserving pre-packet behavior.
#[test]
fn execute_layer_slice_without_classification_keeps_flags_false() {
    let object_id = "obj-noclass";
    let mesh = IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        ],
        indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
    };
    let mesh_ir = make_mesh_ir(object_id, mesh);
    let layer = make_layer(0, 0.5, object_id);

    // Step 4 extends execute_layer_slice; passing None for all three
    // classification params must reproduce the pre-packet behavior.
    let slice = execute_layer_slice(
        &mesh_ir, &layer, None, // surface_class
        None, // next_layer_z
        None, // prev_layer_z
        None, // region_map
        None, // layer_plan
    )
    .expect("slice should succeed");

    for (i, region) in slice.regions.iter().enumerate() {
        assert!(
            region.top_shell_index.is_none(),
            "region {i}: expected top_shell_index=None when no classification supplied"
        );
        assert!(
            region.bottom_shell_index.is_none(),
            "region {i}: expected bottom_shell_index=None when no classification supplied"
        );
        assert!(
            !region.is_bridge,
            "region {i}: expected is_bridge=false when no classification supplied"
        );
    }
}
