//! Red-first TDD coverage for `sync_perimeter_infill_areas_into_slice` —
//! the host-side fill-polygon partition that runs at `Layer::Perimeters` commit.
//!
//! Contract (per `docs/specs/infill-fill-partition-plan.md` Q1–Q5):
//! - Reads `arena.slice()` + `arena.perimeter()`.
//! - For each `(object_id, region_id)` present in `SliceIR`, finds the matching
//!   `PerimeterIR.regions` entry; absence is fatal.
//! - Computes pairwise-disjoint canonical fill polygons by precedence
//!   `bridge > bottom > top > sparse` and writes them back onto the arena's
//!   `SlicedRegion` in place.
//! - `top_solid_fill` / `bottom_solid_fill` / `bridge_areas` end up clipped to
//!   `perimeter.infill_areas` AND deduped against higher-precedence siblings.
//! - `sparse_infill_area` is the remainder of `perimeter.infill_areas` after
//!   subtracting the three solid/bridge polygons.

use slicer_core::polygon_ops::intersection;
use slicer_ir::{
    ExPolygon, ObjectId, PerimeterIR, PerimeterRegion, Point2, Polygon, RegionId, SemVer, SliceIR,
    SlicedRegion,
};
use slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice;
use slicer_runtime::LayerArena;

// ── fixture helpers ──────────────────────────────────────────────────────────

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

fn empty_slice_ir() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 4,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: Vec::new(),
    }
}

fn empty_perimeter_ir() -> PerimeterIR {
    PerimeterIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: Vec::new(),
    }
}

fn sliced_region(object_id: &str, region_id: RegionId, polys: Vec<ExPolygon>) -> SlicedRegion {
    SlicedRegion {
        object_id: ObjectId::from(object_id),
        region_id,
        polygons: polys.clone(),
        infill_areas: polys,
        effective_layer_height: 0.2,
        ..Default::default()
    }
}

fn perimeter_region(
    object_id: &str,
    region_id: RegionId,
    infill_areas: Vec<ExPolygon>,
) -> PerimeterRegion {
    PerimeterRegion {
        object_id: ObjectId::from(object_id),
        region_id,
        walls: Vec::new(),
        infill_areas,
        seam_candidates: Vec::new(),
        resolved_seam: None,
    }
}

fn arena_with(slice: SliceIR, perimeter: PerimeterIR) -> LayerArena {
    let mut a = LayerArena::new();
    a.set_slice(slice).expect("set_slice");
    a.set_perimeter(perimeter).expect("set_perimeter");
    a
}

fn ex_area_mm2(polys: &[ExPolygon]) -> f64 {
    // slicer_core / Clipper2 may return "polygon-with-hole" as two ExPolygons
    // with opposite windings (the outer ring CW and the hole CCW, or vice
    // versa). Summing signed shoelace areas across the Vec correctly cancels
    // hole contributions; taking |sum| at the end yields the net mm² area
    // regardless of which orientation convention Clipper2 chose for output.
    fn signed_ring_area_units(pts: &[Point2]) -> f64 {
        let n = pts.len();
        if n < 3 {
            return 0.0;
        }
        let mut a = 0.0_f64;
        for i in 0..n {
            let j = (i + 1) % n;
            a += pts[i].x as f64 * pts[j].y as f64 - pts[j].x as f64 * pts[i].y as f64;
        }
        a / 2.0
    }

    let mut signed_sum = 0.0_f64;
    for ep in polys {
        signed_sum += signed_ring_area_units(&ep.contour.points);
        for hole in &ep.holes {
            // Explicit holes already encoded in the ExPolygon — their signed
            // contribution will be opposite-winding from the contour.
            signed_sum += signed_ring_area_units(&hole.points);
        }
    }
    // 1 internal unit = 100 nm = 1e-4 mm; area unit² → mm² requires divide by 1e8.
    signed_sum.abs() / 1.0e8
}

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ── AC-1: sparse partition ────────────────────────────────────────────────────

#[test]
fn ac1_sparse_partition_left_half_when_top_covers_right_half() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);
    let top_solid = square(5.0, 0.0, 10.0, 10.0);

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![wall_inset.clone()]);
    sr.top_solid_fill = vec![top_solid.clone()];
    slice.regions.push(sr);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![wall_inset.clone()]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let r = &arena.slice().expect("slice still present").regions[0];

    assert!(
        approx_eq(ex_area_mm2(&r.sparse_infill_area), 50.0, 0.01),
        "sparse_infill_area should be 50 mm² (left half); got {} mm²",
        ex_area_mm2(&r.sparse_infill_area)
    );
    assert!(
        approx_eq(ex_area_mm2(&r.top_solid_fill), 50.0, 0.01),
        "top_solid_fill should be 50 mm² (right half); got {} mm²",
        ex_area_mm2(&r.top_solid_fill)
    );
    assert!(r.bottom_solid_fill.is_empty(), "bottom must be empty");
    assert!(r.bridge_areas.is_empty(), "bridge must be empty");
}

// ── AC-2: precedence dedup (bridge > bottom > top > sparse) ─────────────────

#[test]
fn ac2_precedence_bridge_wins_when_all_three_overlap_fully() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![wall_inset.clone()]);
    sr.top_solid_fill = vec![wall_inset.clone()];
    sr.bottom_solid_fill = vec![wall_inset.clone()];
    sr.bridge_areas = vec![wall_inset.clone()];
    slice.regions.push(sr);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![wall_inset.clone()]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let r = &arena.slice().expect("slice").regions[0];
    let total = 100.0_f64; // 10 x 10
    assert!(
        approx_eq(ex_area_mm2(&r.bridge_areas), total, 0.01),
        "bridge wins precedence; got {} mm²",
        ex_area_mm2(&r.bridge_areas)
    );
    assert!(
        r.bottom_solid_fill.is_empty(),
        "bottom must be subtracted by bridge"
    );
    assert!(
        r.top_solid_fill.is_empty(),
        "top must be subtracted by bridge+bottom"
    );
    assert!(
        r.sparse_infill_area.is_empty(),
        "sparse must be subtracted by all higher-precedence; got {} mm²",
        ex_area_mm2(&r.sparse_infill_area)
    );
}

#[test]
fn ac2_precedence_pairwise_disjoint_under_partial_overlap() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);
    // Three overlapping rectangles inside the wall_inset.
    let top = square(0.0, 0.0, 8.0, 8.0); // big square top-left
    let bottom = square(2.0, 2.0, 10.0, 10.0); // overlapping bottom-right
    let bridge = square(4.0, 4.0, 6.0, 6.0); // tiny central bridge

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![wall_inset.clone()]);
    sr.top_solid_fill = vec![top];
    sr.bottom_solid_fill = vec![bottom];
    sr.bridge_areas = vec![bridge];
    slice.regions.push(sr);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![wall_inset]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let r = &arena.slice().expect("slice").regions[0];

    // Pairwise disjointness — every intersection must have zero area.
    let pairs: [(&[ExPolygon], &[ExPolygon], &str); 6] = [
        (&r.bridge_areas, &r.bottom_solid_fill, "bridge ∩ bottom"),
        (&r.bridge_areas, &r.top_solid_fill, "bridge ∩ top"),
        (&r.bridge_areas, &r.sparse_infill_area, "bridge ∩ sparse"),
        (&r.bottom_solid_fill, &r.top_solid_fill, "bottom ∩ top"),
        (
            &r.bottom_solid_fill,
            &r.sparse_infill_area,
            "bottom ∩ sparse",
        ),
        (&r.top_solid_fill, &r.sparse_infill_area, "top ∩ sparse"),
    ];
    for (a, b, label) in pairs.iter() {
        let inter = intersection(a, b);
        let area = ex_area_mm2(&inter);
        assert!(
            area < 0.01,
            "{label} must be empty after precedence dedup; got {area:.4} mm² overlap"
        );
    }

    // Sum-of-four invariant: with all four polygons pairwise disjoint AND each
    // contained inside wall_inset, the sum of their areas must be ≤ wall_inset
    // (100 mm²). The plan-mode partition formula additionally requires the
    // four to cover all of wall_inset, so the sum equals 100 within Clipper
    // rounding tolerance.
    let br_area = ex_area_mm2(&r.bridge_areas);
    let bot_area = ex_area_mm2(&r.bottom_solid_fill);
    let top_area = ex_area_mm2(&r.top_solid_fill);
    let sp_area = ex_area_mm2(&r.sparse_infill_area);
    let total_area = br_area + bot_area + top_area + sp_area;

    assert!(
        approx_eq(total_area, 100.0, 0.01),
        "sum of four canonical polygons must equal wall_inset area;\n  \
         bridge={br_area:.3} (polys: {bcnt}, fixture area 4)\n  \
         bottom={bot_area:.3} (polys: {botcnt}, fixture area 64)\n  \
         top={top_area:.3} (polys: {topcnt}, fixture area 64)\n  \
         sparse={sp_area:.3} (polys: {spcnt})\n  \
         total={total_area:.3} (expected 100)",
        bcnt = r.bridge_areas.len(),
        botcnt = r.bottom_solid_fill.len(),
        topcnt = r.top_solid_fill.len(),
        spcnt = r.sparse_infill_area.len(),
    );
}

// ── AC-3: clip-in-place ──────────────────────────────────────────────────────

#[test]
fn ac3_clip_in_place_top_solid_fill_does_not_exit_wall_inset() {
    let wall_inset = square(2.0, 2.0, 8.0, 8.0); // 6×6 = 36 mm²
    let oversized_top = square(0.0, 0.0, 10.0, 10.0); // 10×10 = 100 mm²

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![wall_inset.clone()]);
    sr.top_solid_fill = vec![oversized_top];
    slice.regions.push(sr);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![wall_inset.clone()]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let r = &arena.slice().expect("slice").regions[0];

    // After clipping, top_solid_fill must equal the wall-inset (36 mm²),
    // not the original oversized 100 mm².
    assert!(
        approx_eq(ex_area_mm2(&r.top_solid_fill), 36.0, 0.01),
        "top_solid_fill must be clipped to wall_inset area; got {} mm²",
        ex_area_mm2(&r.top_solid_fill)
    );
    assert!(
        r.sparse_infill_area.is_empty(),
        "wall_inset fully covered by top after clip → sparse must be empty"
    );
}

// ── AC-4: pure top → empty sparse ────────────────────────────────────────────

#[test]
fn ac4_pure_top_layer_yields_empty_sparse() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![wall_inset.clone()]);
    sr.top_shell_index = Some(0);
    sr.top_solid_fill = vec![wall_inset.clone()];
    slice.regions.push(sr);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![wall_inset.clone()]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let r = &arena.slice().expect("slice").regions[0];
    assert!(r.sparse_infill_area.is_empty(), "pure top → empty sparse");
    assert!(
        approx_eq(ex_area_mm2(&r.top_solid_fill), 100.0, 0.01),
        "top_solid_fill should cover entire wall_inset"
    );
}

// ── AC-5: no perimeter entry → skip that region, partition remains untouched ─

#[test]
fn ac5_no_perimeter_entry_leaves_region_polygons_untouched() {
    // A SliceIR region without a matching PerimeterIR entry is a legitimate
    // configuration (variant_chain region_split work, packets 92–95): the
    // variant region shares wall geometry with its base region and does not
    // get its own perimeter commit. The host partition skips such regions
    // silently — their four canonical polygons keep whatever PrePass values
    // they had.
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);
    let other_inset = square(20.0, 20.0, 30.0, 30.0);

    let mut slice = empty_slice_ir();
    // Region A (object 'obj-other', region 99) has a matching perimeter entry.
    let mut sr_a = sliced_region("obj-other", 99, vec![other_inset.clone()]);
    sr_a.top_solid_fill = vec![other_inset.clone()];
    slice.regions.push(sr_a);
    // Region B (object 'obj-1', region 7) has NO matching perimeter entry —
    // simulates a virtual variant region.
    let mut sr_b = sliced_region("obj-1", 7, vec![wall_inset.clone()]);
    sr_b.top_solid_fill = vec![wall_inset.clone()];
    slice.regions.push(sr_b);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-other", 99, vec![other_inset.clone()]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition must not be fatal");

    let regions = &arena.slice().expect("slice").regions;
    // Region A was partitioned: top_solid_fill clipped to other_inset, sparse
    // is empty (top covers the entire inset).
    let a = regions
        .iter()
        .find(|r| r.region_id == 99)
        .expect("region A");
    assert!(approx_eq(ex_area_mm2(&a.top_solid_fill), 100.0, 0.01));
    assert!(a.sparse_infill_area.is_empty());
    // Region B was skipped: top_solid_fill remains at the original wall_inset,
    // sparse_infill_area stays empty (never touched by the partition).
    let b = regions.iter().find(|r| r.region_id == 7).expect("region B");
    assert!(approx_eq(ex_area_mm2(&b.top_solid_fill), 100.0, 0.01));
    assert!(b.sparse_infill_area.is_empty());
}

// ── AC-6: preserves untouched fields ─────────────────────────────────────────

#[test]
fn ac6_partition_preserves_unrelated_fields() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![wall_inset.clone()]);
    sr.effective_layer_height = 0.32;
    sr.top_shell_index = Some(2);
    sr.bottom_shell_index = Some(3);
    sr.is_bridge = true;
    slice.regions.push(sr);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![wall_inset.clone()]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let r = &arena.slice().expect("slice").regions[0];
    assert_eq!(r.polygons.len(), 1);
    assert!(approx_eq(ex_area_mm2(&r.polygons), 100.0, 0.01));
    assert_eq!(r.effective_layer_height, 0.32);
    assert_eq!(r.top_shell_index, Some(2));
    assert_eq!(r.bottom_shell_index, Some(3));
    assert!(r.is_bridge);
}
