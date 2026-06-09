//! End-to-end guards for the host-side fill partition.
//!
//! Mirrors the production sequence at `Layer::Perimeters` commit:
//! `arena.set_perimeter(ir)` followed by
//! `sync_perimeter_infill_areas_into_slice(arena, layer_index)`. The two-step
//! sequence is what `commit_layer_outputs` runs at the Layer::Perimeters arm
//! in `crates/slicer-runtime/src/layer_executor.rs`. Unlike
//! `region_partition_tdd` (which tests the partition fn in isolation against
//! pre-staged arenas), this file locks the **integration contract** through
//! the canonical production-call ordering.

use slicer_ir::{
    ExPolygon, ObjectId, PerimeterIR, PerimeterRegion, Point2, Polygon, RegionId, SemVer, SliceIR,
    SlicedRegion,
};
use slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice;
use slicer_runtime::LayerArena;

fn square(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

fn make_slice_ir_one_region(
    obj: &str,
    rid: RegionId,
    polys: Vec<ExPolygon>,
    top_solid: Vec<ExPolygon>,
    bottom_solid: Vec<ExPolygon>,
    bridge: Vec<ExPolygon>,
) -> SliceIR {
    let r = SlicedRegion {
        object_id: ObjectId::from(obj),
        region_id: rid,
        polygons: polys.clone(),
        infill_areas: polys,
        effective_layer_height: 0.2,
        top_solid_fill: top_solid,
        bottom_solid_fill: bottom_solid,
        bridge_areas: bridge,
        ..Default::default()
    };
    SliceIR {
        schema_version: SemVer {
            major: 4,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![r],
    }
}

fn make_perimeter_ir(obj: &str, rid: RegionId, infill_areas: Vec<ExPolygon>) -> PerimeterIR {
    PerimeterIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: ObjectId::from(obj),
            region_id: rid,
            walls: Vec::new(),
            infill_areas,
            seam_candidates: Vec::new(),
            resolved_seam: None,
        }],
    }
}

/// Apply the production sequence: stage the perimeter IR, then run the host
/// partition. Mirrors `commit_layer_outputs` at the Layer::Perimeters arm.
fn apply_perimeter_commit_sequence(
    arena: &mut LayerArena,
    perimeter: PerimeterIR,
    layer_index: u32,
) {
    let _ = arena.take_perimeter();
    arena.set_perimeter(perimeter).expect("set_perimeter");
    sync_perimeter_infill_areas_into_slice(arena, layer_index).expect("partition");
}

// ── pure top layer → zero sparse, full top ───────────────────────────────────

#[test]
fn pure_top_layer_through_commit_yields_empty_sparse() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);

    let slice = make_slice_ir_one_region(
        "obj-1",
        0,
        vec![wall_inset.clone()],
        vec![wall_inset.clone()], // top covers entire inset
        Vec::new(),
        Vec::new(),
    );
    let perim = make_perimeter_ir("obj-1", 0, vec![wall_inset.clone()]);

    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("set_slice");

    apply_perimeter_commit_sequence(&mut arena, perim, 0);

    let r = &arena.slice().expect("slice").regions[0];
    assert!(
        r.sparse_infill_area.is_empty(),
        "pure top → empty sparse after commit"
    );
    assert!(
        !r.top_solid_fill.is_empty(),
        "top_solid_fill must be populated for the entire inset"
    );
}

// ── mid layer (no shells) → only sparse, no solid/bridge ─────────────────────

#[test]
fn mid_layer_through_commit_yields_only_sparse() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);

    let slice = make_slice_ir_one_region(
        "obj-1",
        0,
        vec![wall_inset.clone()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let perim = make_perimeter_ir("obj-1", 0, vec![wall_inset]);

    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("set_slice");

    apply_perimeter_commit_sequence(&mut arena, perim, 0);

    let r = &arena.slice().expect("slice").regions[0];
    assert!(
        !r.sparse_infill_area.is_empty(),
        "mid layer → non-empty sparse"
    );
    assert!(r.top_solid_fill.is_empty());
    assert!(r.bottom_solid_fill.is_empty());
    assert!(r.bridge_areas.is_empty());
}

// ── partial top layer → solid + sparse pairwise-disjoint after commit ────────

#[test]
fn partial_top_layer_through_commit_yields_disjoint_partition() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);
    let top_half = square(0.0, 5.0, 10.0, 10.0);

    let slice = make_slice_ir_one_region(
        "obj-1",
        0,
        vec![wall_inset.clone()],
        vec![top_half],
        Vec::new(),
        Vec::new(),
    );
    let perim = make_perimeter_ir("obj-1", 0, vec![wall_inset]);

    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("set_slice");

    apply_perimeter_commit_sequence(&mut arena, perim, 0);

    let r = &arena.slice().expect("slice").regions[0];
    assert!(!r.top_solid_fill.is_empty(), "top_solid_fill populated");
    assert!(
        !r.sparse_infill_area.is_empty(),
        "sparse_infill_area populated"
    );

    // Pairwise disjointness via point-in-bbox check: every vertex of one set
    // must lie outside the AABB union of the other.
    let top_aabb_contains = |pt: &Point2| {
        // top_half lives in (0..10, 5..10) in mm; in 100-nm units that's
        // (0..100_000, 50_000..100_000).
        pt.x >= 0 && pt.x <= 100_000 && pt.y >= 50_000 && pt.y <= 100_000
    };
    for ep in &r.sparse_infill_area {
        for p in &ep.contour.points {
            // sparse should be in (0..10, 0..5), entirely below the top half.
            assert!(
                p.y < 50_000 + 100, // 100-unit (10 µm) tolerance
                "sparse vertex y={} must be below top_half boundary (50_000)",
                p.y
            );
            assert!(
                !top_aabb_contains(p) || p.y == 50_000,
                "sparse vertex {:?} must not be inside top_half AABB",
                (p.x, p.y)
            );
        }
    }
}
