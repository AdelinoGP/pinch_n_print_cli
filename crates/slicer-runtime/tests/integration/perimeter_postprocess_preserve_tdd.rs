//! Regression coverage for the `Layer::PerimetersPostProcess` `(Some, Some)`
//! arm in `crates/slicer-runtime/src/layer_executor.rs`.
//!
//! Three regressions are guarded here:
//!
//! 1. **Fix 4** — when an incoming post-process `PerimeterIR` has empty
//!    `infill_areas` / `seam_candidates` (the "wall-only" emit case typical of
//!    fuzzy-skin and seam-placer), the arm must preserve those fields from the
//!    original perimeter. Without this, the cube "no sparse infill" symptom
//!    returns because the host-side fill partition that re-fires below sees an
//!    empty `wall_inset`.
//! 2. **HIGH-1** — pairing between `ir_owned.regions` and `orig_perim.regions`
//!    is by `(object_id, region_id)`, NOT positional. Reverting the production
//!    `iter().find(...)` back to `regions.get(idx)` must make
//!    [`pairs_regions_by_object_id_not_by_position`] fail.
//! 3. **Partition re-fire** — `sync_perimeter_infill_areas_into_slice` runs
//!    after the commit (even on the `(Some, None)` path), so
//!    `SliceIR.regions[*].sparse_infill_area` ends up populated.

use slicer_ir::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, LayerStageCommit, LoopType, ObjectId, PerimeterIR,
    PerimeterRegion, Point2, Point3WithWidth, Polygon, RegionId, SeamCandidate, SeamReason, SemVer,
    SliceIR, SlicedRegion, WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_runtime::{apply_for_test, LayerArena, StageApplyContext};

// ── fixture helpers (mirrors region_partition_tdd.rs:24-96) ──────────────────

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

// Local copy of `ex_area_mm2` from
// `tests/integration/region_partition_tdd.rs:98-128` — the integration-test
// bucket has no shared common module exposing area math, so the helper is
// duplicated here verbatim with attribution.
fn ex_area_mm2(polys: &[ExPolygon]) -> f64 {
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
            signed_sum += signed_ring_area_units(&hole.points);
        }
    }
    signed_sum.abs() / 1.0e8
}

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// Synthetic wall loop used by the post-process IR. The actual geometry is
// irrelevant — Fix 4 only inspects `infill_areas` / `seam_candidates`.
fn synthetic_wall() -> WallLoop {
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
                Point3WithWidth {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile { widths: vec![0.4] },
        feature_flags: Vec::new(),
        boundary_type: WallBoundaryType::Interior,
    }
}

fn synthetic_seam_candidate() -> SeamCandidate {
    SeamCandidate {
        position: Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
            dist_to_top_mm: 0.0,
        },
        score: 0.5,
        reason: SeamReason::Aligned,
    }
}

fn commit_with_perimeter(ir: PerimeterIR) -> LayerStageCommit {
    LayerStageCommit::PerimetersPostProcess(Some(ir))
}

// ── Test 1: Fix 4 — preserve fields when post-process emits empty ───────────

#[test]
fn preserves_infill_areas_when_post_process_emits_empty() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);
    let preserved_seam = synthetic_seam_candidate();

    // Stage the original Layer::Perimeters output with non-empty infill_areas
    // and one seam candidate.
    let mut slice = empty_slice_ir();
    slice
        .regions
        .push(sliced_region("obj-1", 0, vec![wall_inset.clone()]));

    let mut orig_perim = empty_perimeter_ir();
    let mut orig_region = perimeter_region("obj-1", 0, vec![wall_inset.clone()]);
    orig_region.seam_candidates = vec![preserved_seam.clone()];
    orig_perim.regions.push(orig_region);

    let mut arena = arena_with(slice, orig_perim);

    // Build the incoming post-process IR: walls present, but infill_areas and
    // seam_candidates empty — the "fuzzy-skin / seam-placer wall-only emit".
    let mut ir_owned = empty_perimeter_ir();
    ir_owned.regions.push(PerimeterRegion {
        object_id: ObjectId::from("obj-1"),
        region_id: 0,
        walls: vec![synthetic_wall()],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        resolved_seam: None,
    });

    apply_for_test(
        &mut arena,
        commit_with_perimeter(ir_owned),
        &StageApplyContext {
            stage_id: "Layer::PerimetersPostProcess",
            module_id: "test",
            layer_index: 0,
            seam_plan: None,
        },
    )
    .expect("commit");

    let perim = arena.perimeter().expect("perimeter");
    let region = &perim.regions[0];
    assert_eq!(
        region.infill_areas.len(),
        1,
        "infill_areas must be preserved from orig perim (Fix 4)"
    );
    assert!(
        approx_eq(ex_area_mm2(&region.infill_areas), 100.0, 0.01),
        "preserved infill_areas should equal the original wall_inset (100 mm²); got {} mm²",
        ex_area_mm2(&region.infill_areas)
    );
    assert_eq!(
        region.seam_candidates.len(),
        1,
        "seam_candidates must be preserved from orig perim (Fix 4)"
    );
    assert_eq!(region.seam_candidates[0], preserved_seam);

    // Partition re-fired after commit: sparse_infill_area is populated.
    let sparse = &arena.slice().expect("slice").regions[0].sparse_infill_area;
    assert!(
        !sparse.is_empty(),
        "partition must re-fire after PerimetersPostProcess commit"
    );
    assert!(
        approx_eq(ex_area_mm2(sparse), 100.0, 0.01),
        "sparse_infill_area should cover the full wall_inset; got {} mm²",
        ex_area_mm2(sparse)
    );
}

// ── Test 2: HIGH-1 — pair by (object_id, region_id), not by index ───────────

#[test]
fn pairs_regions_by_object_id_not_by_position() {
    // Region A and B occupy disjoint XY footprints so a positional mis-pairing
    // would route A's preserved infill_areas onto B (and vice versa), which the
    // post-partition area check below catches via mismatched mm² totals.
    let square_a = square(0.0, 0.0, 10.0, 10.0); // 100 mm² at origin
    let square_b = square(50.0, 50.0, 55.0, 55.0); // 25 mm² at (50,50)

    // Original perimeter: A first, B second.
    let mut orig_perim = empty_perimeter_ir();
    orig_perim
        .regions
        .push(perimeter_region("obj-1", 0, vec![square_a.clone()]));
    orig_perim
        .regions
        .push(perimeter_region("obj-2", 0, vec![square_b.clone()]));

    // Slice mirrors the same two regions; top/bottom/bridge empty so the
    // partition's sparse output equals the wall_inset.
    let mut slice = empty_slice_ir();
    slice
        .regions
        .push(sliced_region("obj-1", 0, vec![square_a.clone()]));
    slice
        .regions
        .push(sliced_region("obj-2", 0, vec![square_b.clone()]));

    let mut arena = arena_with(slice, orig_perim);

    // Incoming post-process IR: REVERSED order (B first, A second). Both
    // regions emit walls only — empty infill_areas / seam_candidates. A
    // positional `regions.get(idx)` lookup in the production code would pair
    // idx=0 (B) against orig_perim.regions[0] (A), routing A's `infill_areas`
    // (10×10 = 100 mm²) onto B, and vice versa.
    let mut ir_owned = empty_perimeter_ir();
    ir_owned.regions.push(PerimeterRegion {
        object_id: ObjectId::from("obj-2"),
        region_id: 0,
        walls: vec![synthetic_wall()],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        resolved_seam: None,
    });
    ir_owned.regions.push(PerimeterRegion {
        object_id: ObjectId::from("obj-1"),
        region_id: 0,
        walls: vec![synthetic_wall()],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        resolved_seam: None,
    });

    apply_for_test(
        &mut arena,
        commit_with_perimeter(ir_owned),
        &StageApplyContext {
            stage_id: "Layer::PerimetersPostProcess",
            module_id: "test",
            layer_index: 0,
            seam_plan: None,
        },
    )
    .expect("commit");

    // Locate the committed perimeter regions by `(object_id, region_id)` —
    // NOT by index. Indexing by position would still pass even if the
    // production pairing were positional, because the post-commit IR keeps
    // the reversed order. Looking up by key is what makes this test a real
    // regression guard for HIGH-1.
    let perim = arena.perimeter().expect("perimeter");
    let region_a = perim
        .regions
        .iter()
        .find(|r| r.object_id == "obj-1" && r.region_id == 0)
        .expect("region A by key");
    let region_b = perim
        .regions
        .iter()
        .find(|r| r.object_id == "obj-2" && r.region_id == 0)
        .expect("region B by key");

    assert!(
        approx_eq(ex_area_mm2(&region_a.infill_areas), 100.0, 0.01),
        "region A must receive its OWN preserved infill_areas (100 mm²); \
         got {} mm² — positional pairing would have routed B's 25 mm² here",
        ex_area_mm2(&region_a.infill_areas)
    );
    assert!(
        approx_eq(ex_area_mm2(&region_b.infill_areas), 25.0, 0.01),
        "region B must receive its OWN preserved infill_areas (25 mm²); \
         got {} mm² — positional pairing would have routed A's 100 mm² here",
        ex_area_mm2(&region_b.infill_areas)
    );

    // The partition re-fire downstream also depends on correct pairing: the
    // slice region for (obj-1, 0) only ends up with sparse area ≈ 100 mm² if
    // the preserved infill_areas were correctly routed to A's perimeter
    // region (positional mis-routing would leave A's wall_inset empty, and
    // the partition would emit empty sparse).
    let slice_regions = &arena.slice().expect("slice").regions;
    let slice_a = slice_regions
        .iter()
        .find(|r| r.object_id == "obj-1" && r.region_id == 0)
        .expect("slice region A");
    let slice_b = slice_regions
        .iter()
        .find(|r| r.object_id == "obj-2" && r.region_id == 0)
        .expect("slice region B");
    assert!(
        approx_eq(ex_area_mm2(&slice_a.sparse_infill_area), 100.0, 0.01),
        "slice region A sparse must equal its 10x10 footprint; got {} mm²",
        ex_area_mm2(&slice_a.sparse_infill_area)
    );
    assert!(
        approx_eq(ex_area_mm2(&slice_b.sparse_infill_area), 25.0, 0.01),
        "slice region B sparse must equal its 5x5 footprint; got {} mm²",
        ex_area_mm2(&slice_b.sparse_infill_area)
    );
}

// ── Test 3: partition re-fires under the (Some, None) path ──────────────────

#[test]
fn partition_re_fires_under_post_process_only_path() {
    // Scenario: Layer::Perimeters produced no output (orig_perim = None) and
    // Layer::PerimetersPostProcess stages a perimeter directly. This is the
    // `(Some, None)` arm at layer_executor.rs ~1182. The re-fire guard at the
    // bottom of the arm must still call sync_perimeter_infill_areas_into_slice
    // so the slice region picks up sparse_infill_area.
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);

    let mut slice = empty_slice_ir();
    slice
        .regions
        .push(sliced_region("obj-1", 0, vec![wall_inset.clone()]));

    // Leave the arena's perimeter slot empty by taking it out immediately
    // after construction (LayerArena::new + set_slice without set_perimeter
    // leaves the slot vacant by default).
    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("set_slice");

    let mut ir_owned = empty_perimeter_ir();
    ir_owned.regions.push(PerimeterRegion {
        object_id: ObjectId::from("obj-1"),
        region_id: 0,
        walls: vec![synthetic_wall()],
        infill_areas: vec![wall_inset.clone()],
        seam_candidates: Vec::new(),
        resolved_seam: None,
    });

    apply_for_test(
        &mut arena,
        commit_with_perimeter(ir_owned),
        &StageApplyContext {
            stage_id: "Layer::PerimetersPostProcess",
            module_id: "test",
            layer_index: 0,
            seam_plan: None,
        },
    )
    .expect("commit");

    let sparse = &arena.slice().expect("slice").regions[0].sparse_infill_area;
    assert!(
        !sparse.is_empty(),
        "partition must re-fire after (Some, None) post-process commit"
    );
    assert!(
        approx_eq(ex_area_mm2(sparse), 100.0, 0.01),
        "sparse_infill_area should cover the full wall_inset; got {} mm²",
        ex_area_mm2(sparse)
    );
}
