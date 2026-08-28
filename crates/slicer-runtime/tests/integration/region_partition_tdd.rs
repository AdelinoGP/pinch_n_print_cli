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
use std::sync::Arc;

use slicer_ir::{
    ExPolygon, ObjectId, PerimeterIR, PerimeterRegion, Point2, Polygon, RegionId, RegionKey,
    RegionMapIR, RegionPlan, SemVer, SliceIR, SlicedRegion,
};
use slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice;
use slicer_runtime::wit_host::{
    ExtrusionPath3d, ExtrusionRole, HostExecutionContextBuilder, OriginId,
};
use slicer_runtime::LayerArena;
use slicer_runtime::{commit_shell_classification_builtin, Blackboard};

use crate::common::{commit_hec_for_test, point3_with_width};

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

        ..Default::default()
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

#[test]
fn internal_bridge_qualification_writes_gated_areas() {
    let object_id = ObjectId::from("qualification-cube");
    let candidate_square = square(-5.0, -5.0, 5.0, 5.0);
    let lower_fill = candidate_square.clone();
    let slices = (0..3)
        .map(|index| SliceIR {
            schema_version: SemVer {
                major: 4,
                minor: 1,
                patch: 0,
            },
            global_layer_index: index,
            z: 0.2 * (index + 1) as f32,
            regions: vec![SlicedRegion {
                object_id: object_id.clone(),
                region_id: 0,
                polygons: vec![candidate_square.clone()],
                infill_areas: if index == 0 {
                    vec![lower_fill.clone()]
                } else {
                    vec![candidate_square.clone()]
                },
                ..Default::default()
            }],
        })
        .collect::<Vec<_>>();
    let mut region_map = RegionMapIR::default();
    let resolved = slicer_ir::ResolvedConfig {
        infill_density: 0.2,
        top_shell_layers: 3,
        bottom_shell_layers: 0,
        ..Default::default()
    };
    let config = region_map.intern_config(resolved);
    for index in 0..3 {
        region_map.entries.insert(
            RegionKey {
                global_layer_index: index,
                object_id: object_id.clone(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config,
                ..Default::default()
            },
        );
    }
    let mut blackboard = Blackboard::new(Arc::new(Default::default()), 3);
    blackboard
        .commit_region_map(Arc::new(region_map))
        .expect("region map");
    blackboard
        .commit_slice_ir(Arc::new(slices))
        .expect("slice IR");

    commit_shell_classification_builtin(&mut blackboard).expect("shell classification");
    let classified = blackboard.slice_ir().expect("classified slices");
    let candidate = &classified[1].regions[0];
    assert!(!candidate.internal_solid_fill.is_empty());
    assert_eq!(candidate.internal_bridge_areas, candidate.bridge_areas);
    assert!(candidate.internal_bridge_areas.len() <= candidate.bridge_areas.len());
}

#[test]
fn shell_band_excludes_exposed_seed_but_keeps_propagated_under_top_fill() {
    let object_id = ObjectId::from("seed-square");
    let full_square = square(0.0, 0.0, 10.0, 10.0);
    let covered = square(5.0, 0.0, 10.0, 10.0);
    let slices = vec![
        SliceIR {
            schema_version: SemVer {
                major: 4,
                minor: 1,
                patch: 0,
            },
            global_layer_index: 0,
            z: 0.2,
            regions: vec![sliced_region("seed-square", 0, vec![full_square])],
        },
        SliceIR {
            schema_version: SemVer {
                major: 4,
                minor: 1,
                patch: 0,
            },
            global_layer_index: 1,
            z: 0.4,
            regions: vec![sliced_region("seed-square", 0, vec![covered])],
        },
    ];
    let mut region_map = RegionMapIR::default();
    let config = region_map.intern_config(slicer_ir::ResolvedConfig {
        top_shell_layers: 2,
        bottom_shell_layers: 0,
        ..Default::default()
    });
    for index in 0..2 {
        region_map.entries.insert(
            RegionKey {
                global_layer_index: index,
                object_id: object_id.clone(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config,
                ..Default::default()
            },
        );
    }
    let mut blackboard = Blackboard::new(Arc::new(Default::default()), 2);
    blackboard
        .commit_region_map(Arc::new(region_map))
        .expect("region map");
    blackboard
        .commit_slice_ir(Arc::new(slices))
        .expect("slice IR");

    commit_shell_classification_builtin(&mut blackboard).expect("shell classification");
    let classified = blackboard.slice_ir().expect("classified slices");
    let lower = &classified[0].regions[0].internal_solid_fill;
    let exposed_half = square(0.0, 0.0, 5.0, 10.0);
    let propagated_half = square(5.0, 0.0, 10.0, 10.0);
    assert!(intersection(lower, &[exposed_half]).is_empty());
    let propagated_area = ex_area_mm2(&intersection(lower, &[propagated_half]));
    // Pass-2 propagation shrinks each intersected step by one extrusion line
    // width. The pre-shrink expectation was 50 mm2; canonical propagation is
    // 38.64 mm2 for this 10 mm x 5 mm band.
    assert!(
        approx_eq(propagated_area, 38.64, 0.1),
        "lower={} propagated={}",
        ex_area_mm2(lower),
        propagated_area
    );
    assert!(approx_eq(ex_area_mm2(lower), 38.64, 0.1));
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

// ── AC-7: empty wall_inset preserves top/bottom solid fill ───────────────────

/// When the perimeter stage produces no infill area for a region
/// (thin-walled region, or a region whose perimeter dispatch never
/// reached `set_infill_areas`), the naive
/// `intersection(top_solid_fill, wall_inset)` would discard an exposed
/// top surface that the shell-classification step deliberately marked,
/// breaking surface-treatment stages such as ironing. The host
/// partition falls back to the original top/bottom polygons (minus
/// bridge precedence zones) so those surfaces still flow through to
/// the infill / ironing stages. The sparse role is empty by
/// construction (no infill center was produced).
#[test]
fn ac7_empty_wall_inset_preserves_top_solid_fill() {
    let top = square(0.0, 0.0, 10.0, 10.0);
    let bottom = square(0.0, 0.0, 10.0, 10.0);
    // Disjoint from top/bottom: with an empty `wall_inset` the bridge claim
    // passes through unclipped (packet 234 — a ceiling layer can have gated
    // bridge areas while the perimeter module produced no infill), so a
    // bridge overlapping the top square would legitimately take it under
    // precedence bridge > bottom > top.
    let bridge = square(20.0, 0.0, 30.0, 10.0);
    let region_polys = square(0.0, 0.0, 10.0, 10.0);

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![region_polys]);
    sr.top_solid_fill = vec![top.clone()];
    sr.bottom_solid_fill = vec![bottom.clone()];
    sr.bridge_areas = vec![bridge.clone()];
    slice.regions.push(sr);

    // Empty wall_inset (perimeter stage produced no infill for this region).
    let mut perim = empty_perimeter_ir();
    perim.regions.push(perimeter_region("obj-1", 0, Vec::new()));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition must not be fatal");

    let r = &arena.slice().expect("slice").regions[0];
    // top_solid_fill preserved (minus the disjoint bridge, which claims its
    // own area under the unconditional bridge claim).
    assert!(
        approx_eq(ex_area_mm2(&r.top_solid_fill), 100.0, 0.01),
        "top_solid_fill must be preserved when wall_inset is empty; got {} mm²",
        ex_area_mm2(&r.top_solid_fill)
    );
    // The gated bridge areas are claimed even when wall_inset is empty
    // (packet 234: ceiling-layer bridge sites must survive the partition).
    assert!(
        approx_eq(ex_area_mm2(&r.bridge_areas), 100.0, 0.01),
        "bridge_areas must be claimed when wall_inset is empty; got {} mm²",
        ex_area_mm2(&r.bridge_areas)
    );
    // bottom_solid_fill is empty by contract when wall_inset is empty
    // (bottom role has no precedence over an empty infill center, so it
    // contributes nothing to the ironing/solid path).
    assert!(
        r.bottom_solid_fill.is_empty(),
        "bottom_solid_fill must be empty when wall_inset is empty; got {} mm²",
        ex_area_mm2(&r.bottom_solid_fill)
    );
    // sparse is empty by construction (no infill center).
    assert!(r.sparse_infill_area.is_empty());
}

// ── bridge clip vs. the perimeter inset ──────────────────────────────────────

/// Regression: the bridge claim MUST be clipped to `perimeter.infill_areas`
/// when that inset is non-empty, exactly like `top_solid_fill` (AC-3).
///
/// Commit `83180d9e` replaced `intersection(&bridge_areas, wall_inset)` with an
/// unconditional `bridge_areas.clone()` to protect the packet-234 ceiling-layer
/// case (empty `wall_inset`). That made bridge the only one of the four
/// partitioned fills not clipped to the wall inset, so bridge extrusion ran
/// out over the outer and middle wall beads — measured on
/// `resources/A_upsidedown.obj`, the bridge polygon reached past the
/// outer-wall centerline.
#[test]
fn bridge_areas_are_clipped_to_wall_inset_when_inset_is_non_empty() {
    let wall_inset = square(2.0, 2.0, 8.0, 8.0); // 6×6 = 36 mm²
    let oversized_bridge = square(0.0, 0.0, 10.0, 10.0); // 10×10 = 100 mm²

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![wall_inset.clone()]);
    sr.bridge_areas = vec![oversized_bridge];
    slice.regions.push(sr);

    let mut perim = empty_perimeter_ir();
    perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![wall_inset.clone()]));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let r = &arena.slice().expect("slice").regions[0];

    assert!(
        approx_eq(ex_area_mm2(&r.bridge_areas), 36.0, 0.01),
        "bridge_areas must be clipped to the wall_inset area (36 mm²); got {} mm²          — an unclipped bridge claim extrudes over the wall beads",
        ex_area_mm2(&r.bridge_areas)
    );

    // Stronger than area: no part of the bridge claim may survive outside the
    // wall inset. Area equality alone could be met by a shifted polygon.
    let outside = slicer_core::polygon_ops::difference(&r.bridge_areas, &[wall_inset]);
    assert!(
        ex_area_mm2(&outside) < 1.0e-6,
        "no bridge area may lie outside the perimeter inset; {} mm² escaped",
        ex_area_mm2(&outside)
    );

    assert!(
        r.sparse_infill_area.is_empty(),
        "wall_inset fully covered by the clipped bridge → sparse must be empty"
    );
}

/// Packet 234 regression guard (companion to AC-7): when `wall_inset` is
/// EMPTY, the gated bridge site must survive the partition untouched. This is
/// the case commit `83180d9e` was fixing and it must keep passing alongside the
/// restored clip above — a ceiling layer whose whole cross-section is top
/// surface produces no perimeter infill area, and an unconditional
/// intersection would silently drop the canonical bridge site.
#[test]
fn bridge_areas_survive_empty_wall_inset_ceiling_layer() {
    let region_polys = square(0.0, 0.0, 10.0, 10.0);
    let bridge = square(1.0, 1.0, 9.0, 9.0); // 8×8 = 64 mm²

    let mut slice = empty_slice_ir();
    let mut sr = sliced_region("obj-1", 0, vec![region_polys]);
    sr.bridge_areas = vec![bridge.clone()];
    slice.regions.push(sr);

    // Ceiling layer: the perimeter module produced no infill area at all.
    let mut perim = empty_perimeter_ir();
    perim.regions.push(perimeter_region("obj-1", 0, Vec::new()));

    let mut arena = arena_with(slice, perim);
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition must not be fatal");

    let r = &arena.slice().expect("slice").regions[0];
    assert!(
        !r.bridge_areas.is_empty(),
        "packet 234: an empty wall_inset must not drop the gated bridge site"
    );
    assert!(
        approx_eq(ex_area_mm2(&r.bridge_areas), 64.0, 0.01),
        "bridge_areas must pass through unclipped when wall_inset is empty; got {} mm²",
        ex_area_mm2(&r.bridge_areas)
    );
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

#[test]
fn internal_bridge_disjoint_from_sparse_partition_after_executor_pass() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);
    let mut slice = empty_slice_ir();
    slice
        .regions
        .push(sliced_region("obj-1", 0, vec![wall_inset.clone()]));
    let mut arena = arena_with(slice, empty_perimeter_ir());
    arena.take_perimeter();
    arena
        .set_perimeter(perimeter_region_ir_for_test(wall_inset))
        .expect("set perimeter");
    sync_perimeter_infill_areas_into_slice(&mut arena, 0).expect("partition");

    let module_id = "test.internal-bridge";
    let mut ctx = HostExecutionContextBuilder::new(module_id, 0.2, 0.2).build();
    ctx.infill_output_mut().sparse_paths.push(ExtrusionPath3d {
        points: vec![
            point3_with_width(-1.0, -1.0, 0.2, 0.4),
            point3_with_width(11.0, 1.0, 0.2, 0.4),
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
        tool_index: None,
        order_lock: None,
    });
    ctx.infill_output_mut().sparse_paths.push(ExtrusionPath3d {
        points: vec![
            point3_with_width(-1.0, 9.0, 0.2, 0.4),
            point3_with_width(11.0, 11.0, 0.2, 0.4),
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
        tool_index: None,
        order_lock: None,
    });
    ctx.infill_output_mut().sparse_path_origins.extend([
        Some(OriginId {
            object_id: "obj-1".into(),
            region_id: 0,
        }),
        Some(OriginId {
            object_id: "obj-1".into(),
            region_id: 0,
        }),
    ]);
    commit_hec_for_test(
        "Layer::InfillPostProcess",
        module_id,
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect("infill postprocess commit");

    let sparse = &arena.slice().expect("slice").regions[0].sparse_infill_area;
    let infill = arena.infill().expect("infill");
    let bridge = &infill.regions[0].internal_bridge_infill;
    // This fixture has no persisted qualified internal-bridge carrier. The
    // pre-shrink pin expected paths from any overlap; canonical gating drops
    // the candidate before executor construction, so no paths are emitted.
    assert!(
        bridge.is_empty(),
        "unqualified candidates must not emit InternalBridgeInfill"
    );
    for path in bridge {
        let min_x = path
            .points
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min)
            - 0.2;
        let max_x = path
            .points
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max)
            + 0.2;
        let min_y = path
            .points
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min)
            - 0.2;
        let max_y = path
            .points
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max)
            + 0.2;
        let bridge_box = square(min_x, min_y, max_x, max_y);
        assert!(
            ex_area_mm2(&intersection(sparse, &[bridge_box])) < 0.01,
            "InternalBridgeInfill partition must be disjoint from sparse partition"
        );
    }
}

fn perimeter_region_ir_for_test(infill_area: ExPolygon) -> PerimeterIR {
    let mut perimeter = empty_perimeter_ir();
    perimeter
        .regions
        .push(perimeter_region("obj-1", 0, vec![infill_area]));
    perimeter
}
