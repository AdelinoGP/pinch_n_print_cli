#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::algos::prepass_slice::{
    batch_bottom_surface_footprints, batch_slice_objects_by_layer,
    execute_prepass_slice_single_layer, execute_prepass_slice_single_layer_with_cache,
    PrepassSliceCache,
};
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, FacetClass, GlobalLayer, IndexedTriangleSet, MeshIR,
    ObjectConfig, ObjectMesh, ObjectSurfaceData, Point2, Point3, Polygon, RegionId, SemVer,
    SurfaceClassificationIR, Transform3d,
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
            world_z_extent: None,
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
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

#[test]
fn slice_at_mid_height_produces_nonempty_polygons() {
    let mesh = cube_mesh();
    let layer = make_global_layer(0, 5.0, "cube");

    let result =
        execute_prepass_slice_single_layer(&mesh, &layer, None, None).expect("slice must succeed");

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
        execute_prepass_slice_single_layer(&mesh, &layer, None, None).expect("slice must succeed");

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
        execute_prepass_slice_single_layer(&mesh, &layer, None, None).expect("slice must succeed");

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

    let err = execute_prepass_slice_single_layer(&mesh, &layer, None, None)
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

/// `facet_count` mutually non-overlapping unit triangles, all flat at Z=0
/// (so they never straddle a slicing plane), spaced 10mm apart on the X
/// axis so their XY projections never touch. Mirrors the disjoint-facet
/// fixture already used to regression-test `compute_xy_footprint`'s own
/// batching fix in `mesh_analysis.rs`.
fn disjoint_flat_bottom_mesh(facet_count: usize) -> IndexedTriangleSet {
    let mut vertices = Vec::with_capacity(facet_count * 3);
    let mut indices = Vec::with_capacity(facet_count * 3);
    for i in 0..facet_count {
        let ox = i as f32 * 10.0;
        let base = vertices.len() as u32;
        vertices.push(p3(ox, 0.0, 0.0));
        vertices.push(p3(ox + 1.0, 0.0, 0.0));
        vertices.push(p3(ox, 1.0, 0.0));
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
    }
    IndexedTriangleSet { vertices, indices }
}

fn mesh_with_bottom_facets(facet_count: usize) -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "obj".to_string(),
            mesh: disjoint_flat_bottom_mesh(facet_count),
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    }
}

/// A `SurfaceClassificationIR` marking every facet of `"obj"` as
/// `BottomSurface`, plus a non-empty overhang-quartile band on every layer
/// in `0..layer_count` — the combination that makes
/// `execute_prepass_slice_single_layer`'s flat-bridge branch call
/// `bottom_surface_footprint` on every layer.
fn surface_classification_with_quartile_bands(
    facet_count: usize,
    layer_count: u32,
) -> SurfaceClassificationIR {
    let mut sc = SurfaceClassificationIR::default();
    sc.per_object.insert(
        "obj".to_string(),
        ObjectSurfaceData {
            facet_classes: vec![FacetClass::BottomSurface; facet_count],
            ..Default::default()
        },
    );
    // Position is irrelevant — this only needs to union into a non-empty
    // `unsupported` region so the flat-bridge branch actually runs.
    let band_polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-100.0, -100.0),
                Point2::from_mm(-99.0, -100.0),
                Point2::from_mm(-99.0, -99.0),
                Point2::from_mm(-100.0, -99.0),
            ],
        },
        holes: vec![],
    };
    for layer_index in 0..layer_count {
        sc.overhang_quartile_polygons.insert(
            layer_index,
            vec![QuartileBand {
                quartile: 1,
                polygons: vec![band_polygon.clone()],
            }],
        );
    }
    sc
}

/// Regression test for the redundant `bottom_surface_footprint` recomputation
/// that made `PrePass::Slice` take ~28s (of a ~40s total slice, down from an
/// original ~50s) on 3D Benchy: `execute_prepass_slice_single_layer`
/// recomputed the whole object's bottom-surface XY footprint (a
/// `compute_xy_footprint` union over every `BottomSurface` facet) from
/// scratch on *every layer*, even though the footprint depends only on the
/// object's mesh and facet classes and is identical across all layers.
/// `execute_prepass_slice_single_layer_with_cache`, fed by
/// `batch_bottom_surface_footprints`, computes it exactly once per object
/// instead.
///
/// Note the per-layer flat-bridge branch also runs
/// `assemble_flat_bridge_areas` (real, layer-varying geometry work
/// unaffected by this cache), so the achievable speedup here is bounded —
/// unlike the `compute_xy_footprint`/`annotate_overhangs` batching fixes
/// elsewhere in this session, which eliminated their per-item cost
/// entirely. Measured directly against the uncached per-layer path on this
/// fixture (800 disjoint bottom facets across 20 layers): ~840ms uncached
/// vs. ~240ms cached (~3.5x, consistent with the ~1.8x reduction measured
/// on the full Benchy pipeline once `assemble_flat_bridge_areas`'s own cost
/// is included). The 1.5x floor below leaves headroom across machines/build
/// profiles while still failing fast if this regresses back to per-layer
/// recomputation (which would collapse the ratio to ~1x).
#[test]
fn prepass_slice_caches_bottom_surface_footprint_across_layers() {
    const FACET_COUNT: usize = 800;
    const LAYER_COUNT: u32 = 20;

    let mesh = mesh_with_bottom_facets(FACET_COUNT);
    let sc = surface_classification_with_quartile_bands(FACET_COUNT, LAYER_COUNT);
    let layers: Vec<GlobalLayer> = (0..LAYER_COUNT)
        .map(|i| make_global_layer(i, 1.0, "obj"))
        .collect();

    let start = std::time::Instant::now();
    let uncached: Vec<_> = layers
        .iter()
        .map(|layer| {
            execute_prepass_slice_single_layer(&mesh, layer, Some(&sc), None)
                .expect("uncached slice must succeed")
        })
        .collect();
    let uncached_elapsed = start.elapsed();

    let raw_polygons_by_layer = batch_slice_objects_by_layer(&mesh, &layers);
    let bottom_surface_footprint_by_object = batch_bottom_surface_footprints(&mesh, Some(&sc));
    let empty_cache = HashMap::new();

    let start = std::time::Instant::now();
    let cached: Vec<_> = layers
        .iter()
        .map(|layer| {
            let raw_polygons = raw_polygons_by_layer
                .get(&layer.index)
                .unwrap_or(&empty_cache);
            let cache = PrepassSliceCache {
                raw_polygons,
                bottom_surface_footprint: &bottom_surface_footprint_by_object,
            };
            execute_prepass_slice_single_layer_with_cache(&mesh, layer, Some(&sc), None, &cache)
                .expect("cached slice must succeed")
        })
        .collect();
    let cached_elapsed = start.elapsed();

    assert_eq!(
        uncached, cached,
        "cached and uncached PrePass::Slice paths must produce identical SliceIR output"
    );
    assert!(
        cached_elapsed * 3 < uncached_elapsed * 2,
        "cached path took {cached_elapsed:?} vs. uncached {uncached_elapsed:?} for \
         {LAYER_COUNT} layers ({FACET_COUNT} bottom facets each) — expected at least \
         1.5x speedup (measured ~3.5x in debug / ~2.3x in release) from caching the \
         whole-object bottom-surface footprint instead of recomputing it on every \
         layer; this smells like a regression to per-layer `bottom_surface_footprint` \
         recomputation, which is what made PrePass::Slice take ~28s (of ~40s total, \
         down from an original ~50s) on 3D Benchy"
    );
}
