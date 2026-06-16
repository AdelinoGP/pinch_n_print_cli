//! Direct `apply` regression guards for the three orchestration steps the
//! P83 Step 4d move dropped and ADR-0020 restructured. Each test exercises
//! `apply` (via `apply_for_test`) at the arena level — no guest, no executor
//! wiring — so it pins the per-stage commit contract itself:
//!
//! 1. `apply(Perimeters)` back-fills `resolved_seam` from the seam plan
//!    (the lost post-commit seam injection; unified into one back-fill helper).
//! 2. `apply(PathOptimization { order_proposal })` permutes the staged
//!    `ordered_entities` (the lost `apply_entity_order_proposal` call).
//! 3. `apply(PathOptimization)` stamps the z-hop anchor at
//!    `ordered_entities.len()-1` (the placeholder-`0` anchor bug — now
//!    structurally impossible since the producer carries no anchor).

use slicer_ir::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, LayerStageCommit, LoopType,
    ObjectId, PathOptimizationCommit, PerimeterIR, PerimeterRegion, Point2, Point3WithWidth,
    Polygon, PrintEntity, RegionId, RegionKey, SeamPlanEntry, SeamPlanIR, SeamPosition, SemVer,
    SliceIR, SlicedRegion, WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_runtime::{apply_for_test, LayerArena, StageApplyContext};

// ── minimal fixture helpers (mirror perimeter_postprocess_preserve_tdd) ──────

fn pt(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

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

fn synthetic_wall() -> WallLoop {
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: vec![pt(0.0, 0.0), pt(1.0, 0.0)],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile { widths: vec![0.4] },
        feature_flags: Vec::new(),
        boundary_type: WallBoundaryType::Interior,
    }
}

/// A `LayerCollectionIR` with `n` entities, `entity_id` 1..=n at topo 0..n.
fn layer_collection_with_entities(n: u32) -> LayerCollectionIR {
    let ordered_entities = (0..n)
        .map(|i| PrintEntity {
            entity_id: (i as u64) + 1,
            path: ExtrusionPath3D {
                points: vec![pt(i as f32, 0.0)],
                role: ExtrusionRole::SparseInfill,
                speed_factor: 1.0,
            },
            role: ExtrusionRole::SparseInfill,
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: ObjectId::from("obj-1"),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            topo_order: i,
        })
        .collect();
    LayerCollectionIR {
        ordered_entities,
        ..Default::default()
    }
}

fn pathopt_ctx() -> StageApplyContext<'static> {
    StageApplyContext {
        stage_id: "Layer::PathOptimization",
        module_id: "test",
        layer_index: 0,
        seam_plan: None,
    }
}

// ── 1. Perimeters seam back-fill ─────────────────────────────────────────────

#[test]
fn apply_perimeters_backfills_resolved_seam_from_seam_plan() {
    let wall_inset = square(0.0, 0.0, 10.0, 10.0);

    // Slice staged so the fill partition inside apply(Perimeters) succeeds.
    let mut slice = empty_slice_ir();
    slice
        .regions
        .push(sliced_region("obj-1", 0, vec![wall_inset.clone()]));
    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("set_slice");

    // Incoming Layer::Perimeters IR: region with NO resolved_seam (the guest
    // emits walls but never bakes the seam — it arrives via the seam plan).
    let mut ir = empty_perimeter_ir();
    ir.regions.push(PerimeterRegion {
        object_id: ObjectId::from("obj-1"),
        region_id: 0,
        walls: vec![synthetic_wall()],
        infill_areas: vec![wall_inset],
        seam_candidates: Vec::new(),
        resolved_seam: None,
    });

    let chosen = SeamPosition {
        point: pt(3.0, 4.0),
        wall_index: 1,
    };
    let mut seam = SeamPlanIR::default();
    seam.entries.push(SeamPlanEntry {
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: ObjectId::from("obj-1"),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        chosen_candidate: chosen.clone(),
        scored_candidates: Vec::new(),
    });

    apply_for_test(
        &mut arena,
        LayerStageCommit::Perimeters(ir),
        &StageApplyContext {
            stage_id: "Layer::Perimeters",
            module_id: "test",
            layer_index: 0,
            seam_plan: Some(&seam),
        },
    )
    .expect("apply(Perimeters)");

    let region = &arena.perimeter().expect("perimeter").regions[0];
    assert_eq!(
        region.resolved_seam,
        Some(chosen),
        "apply(Perimeters) must back-fill resolved_seam from the seam plan \
         (the post-commit seam injection P83 Step 4d dropped)"
    );
}

// ── 2. PathOptimization order proposal permutes ordered_entities ─────────────

#[test]
fn apply_path_optimization_applies_entity_order_proposal() {
    let mut arena = LayerArena::new();
    arena.set_layer_collection(layer_collection_with_entities(3)); // ids 1,2,3 at topo 0,1,2

    apply_for_test(
        &mut arena,
        LayerStageCommit::PathOptimization(PathOptimizationCommit {
            // new[0]=old[2], new[1]=old[0], new[2]=old[1]
            order_proposal: Some(vec![(2, false), (0, false), (1, false)]),
            ..Default::default()
        }),
        &pathopt_ctx(),
    )
    .expect("apply(PathOptimization)");

    let lc = arena.layer_collection().expect("layer_collection");
    let ids: Vec<u64> = lc.ordered_entities.iter().map(|e| e.entity_id).collect();
    assert_eq!(
        ids,
        vec![3, 1, 2],
        "apply(PathOptimization) must apply the set-entity-order proposal \
         (the apply_entity_order_proposal call P83 Step 4d dropped)"
    );
    let topo: Vec<u32> = lc.ordered_entities.iter().map(|e| e.topo_order).collect();
    assert_eq!(
        topo,
        vec![0, 1, 2],
        "topo_order must be reassigned to new slots"
    );
}

// ── 3. z-hop anchor stamped at end-of-layer (len-1), not placeholder 0 ───────

#[test]
fn apply_path_optimization_stamps_z_hop_anchor_at_end_of_layer() {
    let mut arena = LayerArena::new();
    arena.set_layer_collection(layer_collection_with_entities(3)); // len 3 → anchor 2

    apply_for_test(
        &mut arena,
        LayerStageCommit::PathOptimization(PathOptimizationCommit {
            z_hops: vec![0.6],
            ..Default::default()
        }),
        &pathopt_ctx(),
    )
    .expect("apply(PathOptimization)");

    let hops = arena.take_deferred_z_hops();
    assert_eq!(hops.len(), 1, "exactly one z-hop queued");
    assert_eq!(
        hops[0].after_entity_index, 2,
        "z-hop must anchor at ordered_entities.len()-1 (=2), not the placeholder 0 \
         that P83 Step 4d shipped"
    );
    assert_eq!(hops[0].hop_height, 0.6);
}
