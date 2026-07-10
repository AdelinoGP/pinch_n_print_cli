#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::algos::prepass_slice::{
    assemble_flat_bridge_areas, batch_bottom_surface_footprints, batch_slice_objects_by_layer,
    execute_prepass_slice_single_layer, execute_prepass_slice_single_layer_with_cache,
    PrepassSliceCache,
};
use slicer_core::polygon_ops::{closing_ex, difference};
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, FacetClass, GlobalLayer, IndexedTriangleSet, MeshIR,
    ObjectConfig, ObjectMesh, ObjectSurfaceData, Point2, Point3, Polygon, RegionId, SemVer,
    SlicedRegion, SurfaceClassificationIR, Transform3d,
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
/// A sharp-cornered star polygon with `tips` points, alternating between
/// `outer_mm` and `inner_mm` radii. Sharp convex tips are what make a
/// large-radius `Round`-join morphological offset pathological: Clipper2
/// tessellates every tip's exterior angle into an arc, so the intermediate
/// polygon's vertex count explodes with the join radius. A smooth high-vertex
/// circle does NOT stress `Round` joins (each vertex's exterior angle is
/// tiny); sharp tips do — and the real Benchy cross-section is full of them
/// (railings, cabin edges, chimney).
fn sharp_star(tips: usize, outer_mm: f32, inner_mm: f32) -> ExPolygon {
    use std::f32::consts::PI;
    let mut points = Vec::with_capacity(tips * 2);
    for i in 0..(tips * 2) {
        let angle = (i as f32) * PI / (tips as f32);
        let r = if i % 2 == 0 { outer_mm } else { inner_mm };
        points.push(Point2::from_mm(r * angle.cos(), r * angle.sin()));
    }
    ExPolygon {
        contour: Polygon { points },
        holes: vec![],
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

/// Regression test for the flat-bridge enclosure closing that dominated
/// `PrePass::Slice` (~28s of a ~30s stage on 3D Benchy — 92% of it, per
/// sub-stage instrumentation). `assemble_flat_bridge_areas`'s enclosure
/// discriminator used the shared `closing_ex` (dilate-then-erode with `Round`
/// joins at a 0.05mm arc tolerance) at a 12mm radius. On a high-vertex,
/// sharp-cornered cross-section this tessellates every convex tip into a large
/// arc, exploding the intermediate polygon before the erode — and it ran per
/// layer.
///
/// The fix replaced it with a `Square`-join closing. The discriminator is a
/// BOOLEAN "does a gap ≤ 2·R re-fill?" test gated by a loose 10% fraction, so
/// corner roundness is irrelevant, and `Square` adds one bevel point per corner
/// instead of an arc.
///
/// The robust, machine-independent signal is the **vertex count** of the
/// closing output: `Round` at 12mm / 0.05mm tessellates each sharp tip into an
/// arc, so the closed polygon carries several× more points than the `Square`
/// closing produces (measured: ~1100 vs ~220 on this fixture). That vertex
/// blow-up — multiplied across ~280 per-layer/-region closings — was the ~28s.
/// Wall-clock ratio alone is a weaker signal (Clipper2 offset has a high fixed
/// per-call cost, so the ~5× point reduction is only ~1.8× in time), so this
/// asserts on vertex count, not timing. It also runs the real
/// `assemble_flat_bridge_areas` and requires the enclosed gap to still be
/// flagged, proving the closing PATH actually executes.
#[test]
fn flat_bridge_enclosure_closing_avoids_round_arc_explosion() {
    use slicer_core::polygon_ops::{offset, OffsetJoinType};

    // Solid star with a small central gap: the gap is the flat-unsupported
    // span; the star body is the support that gets morphologically closed.
    let star = sharp_star(220, 40.0, 26.0);
    let gap = square_expoly(0.0, 0.0, 2.0); // 4mm square gap, well inside inner radius
    let bottom_fp = square_expoly(0.0, 0.0, 4.0); // covers the gap → flat_unsupported non-empty

    let support = difference(&[star.clone()], &[gap.clone()]);
    let round_pts: usize = closing_ex(&support, 12.0)
        .iter()
        .map(|e| e.contour.points.len())
        .sum();
    let sq_dil = offset(&support, 12.0, OffsetJoinType::Square, 0.0);
    let square_pts: usize = offset(&sq_dil, -12.0, OffsetJoinType::Square, 0.0)
        .iter()
        .map(|e| e.contour.points.len())
        .sum();

    assert!(
        square_pts > 0 && round_pts > 0,
        "both closings must produce geometry (fixture sanity): round={round_pts} square={square_pts}"
    );
    assert!(
        square_pts * 3 < round_pts,
        "Square enclosure closing emitted {square_pts} vertices but the old Round \
         `closing_ex` emitted {round_pts} on the same sharp-cornered 12mm-radius \
         support — expected Square to be ≥3× leaner (measured ~5×). A shrinking gap \
         means the enclosure discriminator reverted to `Round` arc tessellation, \
         which made `assemble_flat_bridge_areas` ~92% of PrePass::Slice (~28s) on \
         3D Benchy."
    );

    // The real fix site must still flag the enclosed gap — i.e. its (Square)
    // closing path actually ran, rather than short-circuiting before it.
    let mut region = SlicedRegion {
        object_id: "star".to_string(),
        polygons: vec![star.clone()],
        infill_areas: vec![star.clone()],
        ..Default::default()
    };
    assemble_flat_bridge_areas(
        &mut region,
        std::slice::from_ref(&bottom_fp),
        &[gap.clone()],
        true, // square_closing (default)
    );
    assert!(
        region.is_bridge,
        "the enclosed central gap must be flagged as a flat bridge — otherwise the \
         closing PATH short-circuited and this test would not guard it"
    );

    // The opt-in legacy Round path (`flat_bridge_square_closing = false`) must
    // still detect the same enclosed gap — the config knob only trades cost, not
    // correctness of the enclosure verdict on this fixture.
    let mut region_round = SlicedRegion {
        object_id: "star".to_string(),
        polygons: vec![star.clone()],
        infill_areas: vec![star.clone()],
        ..Default::default()
    };
    assemble_flat_bridge_areas(
        &mut region_round,
        std::slice::from_ref(&bottom_fp),
        &[gap.clone()],
        false, // square_closing off → legacy Round closing_ex
    );
    assert!(
        region_round.is_bridge,
        "legacy Round enclosure closing must flag the same enclosed gap"
    );
}

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
