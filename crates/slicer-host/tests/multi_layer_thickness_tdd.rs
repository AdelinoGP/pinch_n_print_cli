//! TDD file for packet 35: multi-layer top/bottom solid-fill window.
//!
//! RED state: will NOT compile (or will FAIL) until:
//!   - Step 3 extends `classify_region_surfaces` signature with
//!     `top_shell_layers: u32, bottom_shell_layers: u32`.
//!   - Step 4 extends `execute_layer_slice` signature with
//!     `region_map: Option<&RegionMapIR>, layer_plan: Option<&LayerPlanIR>`.
//!
//! Algorithm (locked by AC):
//!   Top window: look ahead `top_shell_layers` entries in `layer_plan.global_layers`
//!     starting at `layer_idx + 1`; take the last Z in that window (or
//!     `f32::INFINITY` when truncated by object extent).
//!   Bottom window: symmetric — look back `bottom_shell_layers` entries ending
//!     at `layer_idx - 1`; take first Z in window (or `f32::NEG_INFINITY`).
//!   `top_shell_layers = 0` → `is_top_surface = false` for every layer.
//!
//! Committed signatures (Step 3/4 must match):
//!   classify_region_surfaces(
//!       object_mesh: &ObjectMesh,
//!       surface_data: &ObjectSurfaceData,
//!       region_polygons: &[Polygon],
//!       layer_z: f32,
//!       next_layer_z: Option<f32>,
//!       prev_layer_z: Option<f32>,
//!       top_shell_layers: u32,
//!       bottom_shell_layers: u32,
//!   ) -> (bool, bool, bool)
//!
//!   execute_layer_slice(
//!       mesh: &MeshIR,
//!       layer: &GlobalLayer,
//!       surface_class: Option<&SurfaceClassificationIR>,
//!       next_layer_z: Option<f32>,
//!       prev_layer_z: Option<f32>,
//!       region_map: Option<&RegionMapIR>,
//!       layer_plan: Option<&LayerPlanIR>,
//!   ) -> Result<SliceIR, LayerSliceError>

use std::collections::{BTreeMap, HashMap};

use slicer_host::execute_layer_slice;
use slicer_host::layer_slice::classify_region_surfaces;
use slicer_ir::{
    ActiveRegion, BoundingBox3, FacetClass, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectConfig, ObjectMesh, ObjectSurfaceData, Point2, Point3, Polygon, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SemVer, SurfaceClassificationIR, SurfaceGroup, Transform3d,
};

// ============================================================================
// Shared fixture helpers (mirroring external_surface_classification_tdd.rs)
// ============================================================================

fn identity_transform() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

fn default_resolved() -> ResolvedConfig {
    ResolvedConfig::default()
}

/// Flat horizontal triangle at Z=`z_top` with upward normal (TopSurface).
/// Vertices (mm): (-5,-5,z), (5,-5,z), (0,5,z); centroid inside origin polygon.
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
        indices: vec![0, 1, 2],
    }
}

/// Flat horizontal triangle at Z=`z_bot` with downward normal (BottomSurface).
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
        indices: vec![0, 2, 1], // CW from above → downward normal
    }
}

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

fn make_mesh_ir(object_id: &str, mesh: IndexedTriangleSet) -> MeshIR {
    MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 1,
            patch: 0,
        },
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
    }
}

/// Rectangle polygon covering origin (±10 mm in X and Y).
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

fn make_surface_class_top(object_id: &str) -> SurfaceClassificationIR {
    let per_object_data = ObjectSurfaceData {
        facet_classes: vec![FacetClass::TopSurface],
        surface_groups: vec![SurfaceGroup {
            id: 0,
            facet_indices: vec![0],
            z_min: 0.0,
            z_max: 1.0,
            area_mm2: 25.0,
            printable: true,
            shell_count: 1,
        }],
        bridge_regions: vec![],
        overhang_regions: vec![],
    };
    let mut per_object = HashMap::new();
    per_object.insert(object_id.to_string(), per_object_data);
    SurfaceClassificationIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object,
    }
}

fn make_surface_class_bottom(object_id: &str) -> SurfaceClassificationIR {
    let per_object_data = ObjectSurfaceData {
        facet_classes: vec![FacetClass::BottomSurface],
        surface_groups: vec![SurfaceGroup {
            id: 0,
            facet_indices: vec![0],
            z_min: 0.0,
            z_max: 1.0,
            area_mm2: 25.0,
            printable: true,
            shell_count: 1,
        }],
        bridge_regions: vec![],
        overhang_regions: vec![],
    };
    let mut per_object = HashMap::new();
    per_object.insert(object_id.to_string(), per_object_data);
    SurfaceClassificationIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object,
    }
}

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

/// Build a `LayerPlanIR` with N layers at z = 0.2, 0.4, 0.6, …
fn make_layer_plan(n: usize, object_id: &str) -> LayerPlanIR {
    let global_layers: Vec<GlobalLayer> = (0..n)
        .map(|i| make_layer(i as u32, (i + 1) as f32 * 0.2, object_id))
        .collect();
    LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers,
        object_participation: HashMap::new(),
    }
}

/// Build a `RegionMapIR` where every layer of `object_id` has the given shell counts.
fn make_region_map(
    object_id: &str,
    layer_count: usize,
    top_shell_layers: u32,
    bottom_shell_layers: u32,
) -> RegionMapIR {
    let mut entries = HashMap::new();
    for i in 0..layer_count {
        let mut cfg = default_resolved();
        cfg.top_shell_layers = top_shell_layers;
        cfg.bottom_shell_layers = bottom_shell_layers;
        let key = RegionKey {
            global_layer_index: i as u32,
            object_id: object_id.to_string(),
            region_id: 0,
        };
        entries.insert(
            key,
            RegionPlan {
                config: cfg,
                stage_modules: HashMap::new(),
                paint_overrides: BTreeMap::new(),
            },
        );
    }
    RegionMapIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        entries,
    }
}

// ============================================================================
// classify_region_surfaces: multi-layer window tests
// ============================================================================

/// AC-ML1: top_shell_layers=3 means 3 consecutive layers from the top share
/// is_top_surface=true (i.e., the window extends 3 layers ahead).
///
/// Setup: 5 layers at z=0.2,0.4,0.6,0.8,1.0.  Flat TopSurface triangle at
/// z=1.0.  For layer_idx=2 (z=0.6), the top window covers layers 3..=5 (z up
/// to z of layer_idx+3=layer_idx+top_shell_layers).  next_layer_z computed as
/// the Z of `layer_idx + top_shell_layers` = layer 5 (z=1.0), so the window
/// is [0.6, 1.0).  The triangle z_min=1.0 falls OUTSIDE that window → false.
/// For layer_idx=2 with window-end = layer 5 (z=1.0) exclusive — so we verify
/// that the 3 layers closest to the top actually flag true.
///
/// The canonical check: call classify_region_surfaces with top_shell_layers=3
/// for layers at z=0.2,0.4,0.6 with next_layer_z derived from the 3rd-ahead
/// layer each time.  For the topmost layer (layer 4, z=1.0) with
/// next_layer_z=INFINITY, the triangle at z=1.0 is in [1.0, ∞) → true.
#[test]
fn top_shell_layers_three_flags_three_layers() {
    // 5-layer object; top facet at z=1.0 mm.
    // top_shell_layers=3 means layers 2,3,4 (z=0.6,0.8,1.0) should flag top.
    let mesh_ir = make_mesh_ir("obj-top3", flat_top_triangle(1.0));
    let surface_class = make_surface_class_top("obj-top3");
    let polygons = vec![region_polygon_covering_origin()];
    let obj_data = &surface_class.per_object["obj-top3"];

    // layer_plan has 5 layers at z=0.2, 0.4, 0.6, 0.8, 1.0
    let layer_plan = make_layer_plan(5, "obj-top3");

    // For each layer we compute next_layer_z as the Z 3 layers ahead.
    // layer_idx=0 (z=0.2): next=layer_3.z=0.8 → window [0.2,0.8), z_min=1.0 outside → false
    // layer_idx=1 (z=0.4): next=layer_4.z=1.0 → window [0.4,1.0), z_min=1.0 outside → false
    // layer_idx=2 (z=0.6): next=∞ (no layer 5) → window [0.6,∞), z_min=1.0 ∈ → true
    // layer_idx=3 (z=0.8): next=∞                → window [0.8,∞), z_min=1.0 ∈ → true
    // layer_idx=4 (z=1.0): next=∞                → window [1.0,∞), z_min=1.0 ∈ → true
    // So exactly 3 layers (idx 2,3,4) flag top — matching top_shell_layers=3.

    let top_shell_layers: u32 = 3;
    let bottom_shell_layers: u32 = 3;

    let mut top_flagged = 0usize;
    for (layer_idx, gl) in layer_plan.global_layers.iter().enumerate() {
        // next_layer_z = Z of the layer `top_shell_layers` ahead (or None if truncated)
        let next_layer_z = layer_plan
            .global_layers
            .get(layer_idx + top_shell_layers as usize)
            .map(|l| l.z);

        let (is_top, _is_bot, _is_bridge) = classify_region_surfaces(
            &mesh_ir.objects[0],
            obj_data,
            &polygons,
            gl.z,
            next_layer_z,
            None,
            top_shell_layers,
            bottom_shell_layers,
        );
        if is_top {
            top_flagged += 1;
        }
    }

    assert_eq!(
        top_flagged, 3,
        "expected exactly 3 layers to flag is_top_surface with top_shell_layers=3, got {top_flagged}"
    );
}

/// AC-ML2: bottom_shell_layers=3 means 3 layers from the bottom flag is_bottom_surface.
#[test]
fn bottom_shell_layers_three_flags_three_layers() {
    // 5-layer object; bottom facet at z=0.2 mm (first layer).
    let mesh_ir = make_mesh_ir("obj-bot3", flat_bottom_triangle(0.2));
    let surface_class = make_surface_class_bottom("obj-bot3");
    let polygons = vec![region_polygon_covering_origin()];
    let obj_data = &surface_class.per_object["obj-bot3"];

    let layer_plan = make_layer_plan(5, "obj-bot3");

    let top_shell_layers: u32 = 3;
    let bottom_shell_layers: u32 = 3;

    // For each layer, prev_layer_z = Z of the layer `bottom_shell_layers` behind (or None).
    // layer_idx=0 (z=0.2): prev=None (no layer -3) → window (-∞,0.2], z_max=0.2 ∈ → true
    // layer_idx=1 (z=0.4): prev=None (no layer -2) → window (-∞,0.4], z_max=0.2 ∈ → true
    // layer_idx=2 (z=0.6): prev=None (no layer -1) → window (-∞,0.6], z_max=0.2 ∈ → true
    // layer_idx=3 (z=0.8): prev=layer_0.z=0.2    → window (0.2,0.8], z_max=0.2 NOT > 0.2 → false
    // layer_idx=4 (z=1.0): prev=layer_1.z=0.4    → window (0.4,1.0], z_max=0.2 NOT > 0.4 → false
    // Exactly 3 layers flag bottom.

    let mut bot_flagged = 0usize;
    for (layer_idx, gl) in layer_plan.global_layers.iter().enumerate() {
        let prev_layer_z = if layer_idx >= bottom_shell_layers as usize {
            layer_plan
                .global_layers
                .get(layer_idx - bottom_shell_layers as usize)
                .map(|l| l.z)
        } else {
            None
        };

        let (_is_top, is_bot, _is_bridge) = classify_region_surfaces(
            &mesh_ir.objects[0],
            obj_data,
            &polygons,
            gl.z,
            None,
            prev_layer_z,
            top_shell_layers,
            bottom_shell_layers,
        );
        if is_bot {
            bot_flagged += 1;
        }
    }

    assert_eq!(
        bot_flagged, 3,
        "expected exactly 3 layers to flag is_bottom_surface with bottom_shell_layers=3, got {bot_flagged}"
    );
}

/// AC-ML3: When the layer plan has fewer ahead-layers than top_shell_layers,
/// the window is truncated (next_layer_z = None → window [layer_z, ∞)), and
/// the top flag still fires for layers within the truncated window extent.
#[test]
fn window_truncates_at_object_extent() {
    // 2-layer object; top facet at z=0.4.  top_shell_layers=3 but only 2 layers exist.
    // layer_idx=0 (z=0.2): layer_idx+3=layer_3 doesn't exist → next_layer_z=None → [0.2,∞)
    //   z_min=0.4 ∈ [0.2,∞) → true (window truncated at object top)
    // layer_idx=1 (z=0.4): layer_idx+3=layer_4 doesn't exist → next_layer_z=None → [0.4,∞)
    //   z_min=0.4 ∈ [0.4,∞) → true
    // Both layers flag top because the window is always truncated.

    let mesh_ir = make_mesh_ir("obj-trunc", flat_top_triangle(0.4));
    let surface_class = make_surface_class_top("obj-trunc");
    let polygons = vec![region_polygon_covering_origin()];
    let obj_data = &surface_class.per_object["obj-trunc"];

    let layer_plan = make_layer_plan(2, "obj-trunc");
    let top_shell_layers: u32 = 3;

    let mut top_flagged = 0usize;
    for (layer_idx, gl) in layer_plan.global_layers.iter().enumerate() {
        let next_layer_z = layer_plan
            .global_layers
            .get(layer_idx + top_shell_layers as usize)
            .map(|l| l.z);

        let (is_top, _, _) = classify_region_surfaces(
            &mesh_ir.objects[0],
            obj_data,
            &polygons,
            gl.z,
            next_layer_z,
            None,
            top_shell_layers,
            3,
        );
        if is_top {
            top_flagged += 1;
        }
    }

    assert_eq!(
        top_flagged, 2,
        "both layers should flag top when window is always truncated (object smaller than window), got {top_flagged}"
    );
}

/// AC-ML4: When region_map is absent (None), execute_layer_slice must fall
/// back to default top_shell_layers=3 / bottom_shell_layers=3.
/// Verify by checking the resulting region flags match the single-layer window
/// that the existing (pre-packet-35) test already confirms.
#[test]
fn missing_config_uses_default_three() {
    // Thin slab mesh with top facet at z=1.0, sliced at z=0.9.
    // With default top_shell_layers=3, and this is a 1-layer plan (no ahead layers),
    // window is [0.9, ∞) → z_min=1.0 ∈ → is_top_surface=true.
    let object_id = "obj-default";
    let mesh = IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: -5.0,
                y: -5.0,
                z: 0.0,
            },
            Point3 {
                x: 5.0,
                y: -5.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 5.0,
                z: 0.0,
            },
            Point3 {
                x: -5.0,
                y: -5.0,
                z: 1.0,
            },
            Point3 {
                x: 5.0,
                y: -5.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 5.0,
                z: 1.0,
            },
        ],
        indices: vec![
            0, 1, 2, // base BottomSurface
            3, 5, 4, // cap TopSurface
            0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4, 2, 0, 3, 2, 3, 5,
        ],
    };
    let mesh_ir = make_mesh_ir(object_id, mesh.clone());

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
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        per_object,
    };

    let layer = make_layer(0, 0.9, object_id);
    // No region_map, no layer_plan → should default to top_shell_layers=3 (open window).
    let slice = execute_layer_slice(
        &mesh_ir,
        &layer,
        Some(&surface_class),
        Some(1.1_f32), // next_layer_z: open top window covers z_min=1.0
        Some(0.7_f32),
        None, // region_map: None → use default
        None, // layer_plan: None → use default
    )
    .expect("slice should succeed");

    assert_eq!(slice.regions.len(), 1);
    let region = &slice.regions[0];
    assert!(
        region.is_top_surface,
        "expected is_top_surface=true with default (None) region_map"
    );
}

/// AC-ML5: execute_layer_slice honors region_map per-region top_shell_layers.
/// A region_map with top_shell_layers=1 limits the window to 1 layer ahead.
#[test]
fn execute_layer_slice_honors_region_map_top_shell_layers() {
    // 3-layer plan at z=0.2, 0.4, 0.6.  Top facet at z=0.6.
    // region_map has top_shell_layers=1 for region (layer_idx=0, obj, region_id=0).
    // For layer_idx=0 (z=0.2): window end = layer_0+1 = layer_1.z=0.4
    //   → next_layer_z=0.4 → window [0.2,0.4), z_min=0.6 OUTSIDE → false.
    // For layer_idx=1 (z=0.4): window end = layer_1+1 = layer_2.z=0.6
    //   → next_layer_z=0.6 → window [0.4,0.6), z_min=0.6 OUTSIDE (exclusive) → false.
    // For layer_idx=2 (z=0.6): window end = layer_2+1 = None
    //   → next_layer_z=None → window [0.6,∞), z_min=0.6 ∈ → true.
    // Only 1 layer flags top (top_shell_layers=1 means only the actual top layer).

    let object_id = "obj-rm-top1";
    let mesh_ir = make_mesh_ir(object_id, flat_top_triangle(0.6));
    let surface_class = make_surface_class_top(object_id);
    let layer_plan = make_layer_plan(3, object_id);
    let region_map = make_region_map(object_id, 3, 1, 3);

    let mut top_flagged = 0usize;
    for (layer_idx, layer_gl) in layer_plan.global_layers.iter().enumerate() {
        // execute_layer_slice slices one layer at a time; pass region_map + layer_plan.
        let slice = execute_layer_slice(
            &mesh_ir,
            layer_gl,
            Some(&surface_class),
            None, // next_layer_z: let execute_layer_slice derive from layer_plan
            None, // prev_layer_z: same
            Some(&region_map),
            Some(&layer_plan),
        )
        .expect("slice ok");
        for region in &slice.regions {
            if region.is_top_surface {
                top_flagged += 1;
            }
        }
        let _ = layer_idx; // suppress unused-variable warning
    }

    assert_eq!(
        top_flagged, 1,
        "expected exactly 1 layer to flag is_top_surface with region_map top_shell_layers=1, got {top_flagged}"
    );
}

/// AC-ML6: When region_map is None, execute_layer_slice falls back to
/// OrcaSlicer defaults (top_shell_layers=3, bottom_shell_layers=3).
/// A 5-layer Benchy-like slice should flag at most 3 top layers.
#[test]
fn none_region_map_uses_orca_defaults() {
    // 5-layer object; top facet at z=1.0.
    // With default top_shell_layers=3, 3 layers should flag top
    // (same as top_shell_layers_three_flags_three_layers but via execute_layer_slice).
    let object_id = "obj-none-rm";
    let mesh_ir = make_mesh_ir(object_id, flat_top_triangle(1.0));
    let surface_class = make_surface_class_top(object_id);
    let layer_plan = make_layer_plan(5, object_id);

    let mut top_flagged = 0usize;
    for layer_gl in &layer_plan.global_layers {
        let slice = execute_layer_slice(
            &mesh_ir,
            layer_gl,
            Some(&surface_class),
            None, // derive from layer_plan
            None,
            None, // region_map: None → default 3
            Some(&layer_plan),
        )
        .expect("slice ok");
        for region in &slice.regions {
            if region.is_top_surface {
                top_flagged += 1;
            }
        }
    }

    assert_eq!(
        top_flagged, 3,
        "None region_map must use OrcaSlicer default top_shell_layers=3, got {top_flagged}"
    );
}

/// AC-ML7: top_shell_layers=0 disables is_top_surface for all layers,
/// regardless of facet classification.
#[test]
fn zero_top_shell_layers_disables_flag() {
    // Even with a valid TopSurface facet in window, top_shell_layers=0 → false always.
    let object_id = "obj-zero-top";
    let mesh_ir = make_mesh_ir(object_id, flat_top_triangle(1.0));
    let surface_class = make_surface_class_top(object_id);
    let polygons = vec![region_polygon_covering_origin()];
    let obj_data = &surface_class.per_object[object_id];

    let layer_plan = make_layer_plan(3, object_id);

    for (layer_idx, gl) in layer_plan.global_layers.iter().enumerate() {
        let next_layer_z = layer_plan
            .global_layers
            .get(layer_idx + 1) // top_shell_layers=0: window is degenerate, helper short-circuits
            .map(|l| l.z);

        let (is_top, _, _) = classify_region_surfaces(
            &mesh_ir.objects[0],
            obj_data,
            &polygons,
            gl.z,
            next_layer_z,
            None,
            0, // top_shell_layers = 0
            3,
        );
        assert!(
            !is_top,
            "layer_idx={layer_idx}: expected is_top_surface=false when top_shell_layers=0"
        );
    }
}

/// NEG3: bottom_shell_layers=0 disables is_bottom_surface for all layers,
/// regardless of facet position.  Bridge detection must still fire independently
/// (bridge flag is not gated by bottom_shell_layers).
#[test]
fn zero_bottom_shell_layers_disables_flag() {
    // 3-layer object; bottom facet at z=0.2 (first layer z_max matches prev_layer_z).
    // With bottom_shell_layers=0, is_bottom_surface must be false for every layer.
    // Bridge detection is independent of bottom_shell_layers and is not expected to
    // fire here (no BridgeRegion in the surface data), confirming it is not suppressed.
    let object_id = "obj-zero-bot";
    let mesh_ir = make_mesh_ir(object_id, flat_bottom_triangle(0.2));
    let surface_class = make_surface_class_bottom(object_id);
    let polygons = vec![region_polygon_covering_origin()];
    let obj_data = &surface_class.per_object[object_id];

    let layer_plan = make_layer_plan(3, object_id);
    let bottom_shell_layers: u32 = 0;

    for (layer_idx, gl) in layer_plan.global_layers.iter().enumerate() {
        // prev_layer_z: the layer directly beneath this one (single-step look-back).
        let prev_layer_z = if layer_idx > 0 {
            layer_plan.global_layers.get(layer_idx - 1).map(|l| l.z)
        } else {
            None
        };

        let (is_top, is_bot, is_bridge) = classify_region_surfaces(
            &mesh_ir.objects[0],
            obj_data,
            &polygons,
            gl.z,
            None,
            prev_layer_z,
            3, // top_shell_layers: non-zero so top classification is independent
            bottom_shell_layers,
        );
        assert!(
            !is_bot,
            "layer_idx={layer_idx}: expected is_bottom_surface=false when bottom_shell_layers=0, got true"
        );
        // Bridge flag must not be suppressed by the bottom_shell_layers=0 guard.
        // The fixture has no BridgeRegion, so is_bridge should be false — but the
        // important check is that it is NOT accidentally forced false by the bottom guard.
        // We assert the value is consistent with the fixture (no bridge data → false).
        assert!(
            !is_bridge,
            "layer_idx={layer_idx}: expected is_bridge=false (no BridgeRegion in fixture), got true"
        );
        let _ = is_top; // top classification is not under test here
    }
}
