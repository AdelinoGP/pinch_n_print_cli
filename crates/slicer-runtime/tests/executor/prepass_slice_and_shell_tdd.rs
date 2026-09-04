//! TDD coverage for the host built-ins `PrePass::Slice` and
//! `PrePass::ShellClassification`.
//!
//! Together these replace the per-layer `Layer::Slice` host built-in:
//! `PrePass::Slice` produces `Vec<SliceIR>` once for the whole print (with
//! `slice_closing_radius` wired in), and `PrePass::ShellClassification`
//! refines that Vec with cross-layer `top_shell_index` / `bottom_shell_index`
//! and polygon-precise `top_solid_fill` / `bottom_solid_fill`.
//!
//! Coordinate system: 1 unit = 100 nm; use `Point2::from_mm` for fixtures.

use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectMesh, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan, SliceIR, SlicedRegion,
    Transform3d, CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::{
    commit_shell_classification_builtin, commit_slice_builtin, execute_prepass_slice_all_layers,
    Blackboard, BlackboardError, BlackboardPrepassSlot, LayerSliceError, ShellClassificationError,
};

// ============================================================================
// Fixture helpers
// ============================================================================

fn identity() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

/// 10Ã—10Ã—L mm cuboid mesh centred on origin (XY), bottom at z=0, top at z=L.
fn cuboid_mesh(object_id: &str, l_mm: f32) -> MeshIR {
    // 8 corners
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        v(-5.0, -5.0, 0.0),
        v(5.0, -5.0, 0.0),
        v(5.0, 5.0, 0.0),
        v(-5.0, 5.0, 0.0),
        v(-5.0, -5.0, l_mm),
        v(5.0, -5.0, l_mm),
        v(5.0, 5.0, l_mm),
        v(-5.0, 5.0, l_mm),
    ];
    // 12 triangles (CCW from outside)
    let indices = vec![
        // bottom (-Z normal)
        0, 2, 1, 0, 3, 2, // top (+Z normal)
        4, 5, 6, 4, 6, 7, // +X
        1, 2, 6, 1, 6, 5, // -X
        0, 4, 7, 0, 7, 3, // +Y
        3, 7, 6, 3, 6, 2, // -Y
        0, 1, 5, 0, 5, 4,
    ];
    let mesh = IndexedTriangleSet { vertices, indices };
    MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh,
            transform: identity(),
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -100.0,
                y: -100.0,
                z: 0.0,
            },
            max: Point3 {
                x: 100.0,
                y: 100.0,
                z: 100.0,
            },
        },
        ..Default::default()
    }
}

fn make_layer(index: u32, z: f32, object_id: &str) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: vec![ActiveRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            effective_layer_height: 0.2,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn make_plan(n_layers: u32, layer_height: f32, object_id: &str) -> LayerPlanIR {
    let global_layers: Vec<GlobalLayer> = (0..n_layers)
        .map(|i| make_layer(i, layer_height * (i + 1) as f32, object_id))
        .collect();
    LayerPlanIR {
        global_layers,
        ..Default::default()
    }
}

fn make_region_map(plan: &LayerPlanIR, top_layers: u32, bottom_layers: u32) -> RegionMapIR {
    let mut region_map = RegionMapIR::default();
    for gl in &plan.global_layers {
        for active in &gl.active_regions {
            let mut config = active.resolved_config.clone();
            config.top_shell_layers = top_layers;
            config.bottom_shell_layers = bottom_layers;
            let config_id = region_map.intern_config(config);
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: gl.index,
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config: config_id,
                    ..Default::default()
                },
            );
        }
    }
    region_map
}

// exhaustive: Blackboard explicit test fixture preserves boundary data
fn seeded_blackboard(mesh: MeshIR, plan: LayerPlanIR, region_map: RegionMapIR) -> Blackboard {
    let n_layers = plan.global_layers.len();
    let mut bb = Blackboard::new(Arc::new(mesh), n_layers);
    bb.commit_layer_plan(Arc::new(plan))
        .expect("commit_layer_plan");
    bb.commit_region_map(Arc::new(region_map))
        .expect("commit_region_map");
    bb
    // exhaustive: Blackboard explicit test fixture preserves boundary data
}

// ============================================================================
// PrePass::Slice tests
// ============================================================================

#[test]
fn prepass_slice_produces_one_slice_per_global_layer() {
    // Cuboid taller than the plan span so every slice Z falls inside the
    // mesh interior (avoids the on-the-top-face empty-slice corner case).
    let mesh = cuboid_mesh("cube", 1.2);
    let plan = make_plan(5, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("PrePass::Slice committed");

    let slices = bb.slice_ir().expect("slice_ir committed");
    assert_eq!(slices.len(), 5);
    for (i, s) in slices.iter().enumerate() {
        assert_eq!(s.global_layer_index, i as u32);
        assert!(!s.regions.is_empty(), "layer {i} should have â‰¥1 region");
        assert!(
            !s.regions[0].polygons.is_empty(),
            "layer {i} region should have â‰¥1 polygon for a solid cuboid"
        );
    }
}

#[test]
fn prepass_slice_blocks_on_missing_layer_plan() {
    let mesh = cuboid_mesh("cube", 1.0);
    let mut bb = Blackboard::new(Arc::new(mesh), 0);
    let err = commit_slice_builtin(&mut bb).expect_err("missing LayerPlan must fail");
    match err {
        LayerSliceError::MissingLayerPlan => {}
        other => panic!("expected MissingLayerPlan, got {other:?}"),
    }
}

#[test]
fn prepass_slice_dup_commit_surfaces_as_blackboard_error() {
    let mesh = cuboid_mesh("cube", 1.0);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("first commit");
    let err = commit_slice_builtin(&mut bb).expect_err("dup commit must fail");
    match err {
        LayerSliceError::Blackboard(BlackboardError::DuplicatePrepassCommit { slot }) => {
            assert_eq!(slot, BlackboardPrepassSlot::SliceIR);
        }
        other => panic!("expected DuplicatePrepassCommit, got {other:?}"),
    }
}

#[test]
fn prepass_slice_all_layers_uses_blackboard_inputs() {
    // Pure-function form: ensures `execute_prepass_slice_all_layers` reads
    // from the blackboard without mutating it.
    let mesh = cuboid_mesh("cube", 0.6);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let bb = seeded_blackboard(mesh, plan, region_map);

    let slices: Vec<SliceIR> = execute_prepass_slice_all_layers(&bb).expect("ok");
    assert_eq!(slices.len(), 3);
    assert!(bb.slice_ir().is_none(), "pure path must not commit");
}

// ============================================================================
// PrePass::ShellClassification tests
// ============================================================================

#[test]
fn shell_classification_blocks_on_uncommitted_slice_ir() {
    let mesh = cuboid_mesh("cube", 1.0);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 3, 3);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    let err =
        commit_shell_classification_builtin(&mut bb).expect_err("must require committed SliceIR");
    match err {
        ShellClassificationError::SliceIRNotCommitted => {}
        other => panic!("expected SliceIRNotCommitted, got {other:?}"),
    }
}

#[test]
fn shell_classification_top_and_bottom_layers_for_single_object_cuboid() {
    // A 5-layer cuboid with top_shell=2 and bottom_shell=2 should classify:
    //   layer 0 (z=0.2): bottom_shell_index=Some(0) (no layer below in timeline)
    //   layer 1 (z=0.4): bottom_shell_index=Some(1) (shadow projection from layer 0)
    //   layer 2 (z=0.6): no shell index (interior)
    //   layer 3 (z=0.8): top_shell_index=Some(1)  (shadow from layer 4)
    //   layer 4 (z=1.0): top_shell_index=Some(0)  (exposed top — no layer above)
    //
    // With the OrcaSlicer edge-ownership convention, slicing at z=1.0 (the
    // exact top face) produces a non-empty cross-section (the top-face outline
    // from side-face top-edge ownership). Layer 4 has no upper neighbor, so
    // top_diff = r_polys → exposed top at depth 0. Pass 2 projects back one
    // layer (k=2-1) to mark layer 3 as depth 1.
    let mesh = cuboid_mesh("cube", 1.0);
    let plan = make_plan(5, 0.2, "cube");
    let region_map = make_region_map(&plan, 2, 2);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("PrePass::Slice");
    commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");

    let slices = bb.slice_ir().expect("classified slice_ir present");
    assert_eq!(slices.len(), 5);

    // Bottom shell zone (layers 0..2)
    let layer0 = &slices[0].regions[0];
    assert_eq!(
        layer0.bottom_shell_index,
        Some(0),
        "layer 0 should be exposed bottom"
    );
    assert!(
        !layer0.bottom_solid_fill.is_empty(),
        "exposed bottom must have non-empty bottom_solid_fill"
    );
    let layer1 = &slices[1].regions[0];
    assert_eq!(
        layer1.bottom_shell_index,
        Some(1),
        "layer 1 should be depth-1 below exposed bottom"
    );

    // Top shell zone — with edge-ownership, layer 4 (z=1.0) produces a
    // non-empty cross-section (top-face outline). It has no upper neighbor,
    // so it is the exposed top (depth 0). Layer 3 gets depth 1 via Pass 2
    // shadow projection.
    let layer4 = &slices[4].regions[0];
    assert_eq!(
        layer4.top_shell_index,
        Some(0),
        "layer 4 should be exposed top (no layer above)"
    );
    assert!(
        !layer4.top_solid_fill.is_empty(),
        "exposed top must have non-empty top_solid_fill"
    );
    let layer3 = &slices[3].regions[0];
    assert_eq!(
        layer3.top_shell_index,
        Some(1),
        "layer 3 should be depth-1 below exposed top"
    );
}

#[test]
fn shell_classification_apply_opening_suppresses_sliver_in_top_solid_fill() {
    // Sliver-suppression regression for A3.
    //
    // Construct a 2-layer SliceIR directly where layer 0 is a 10Ã—10 mm square
    // and layer 1 is a 10Ã—9.95 mm rectangle (same XY origin, top edge shifted
    // inward by 0.05 mm â€” half a tenth of a 0.4 mm extrusion line).
    //
    // The raw `difference(layer0_polys, layer1_polys)` yields a 10Ã—0.05 mm
    // sliver along the top edge: a sub-extrusion-width artifact that real
    // prints cannot reproduce. apply_opening with r = 0.2 mm (half line_width)
    // erodes by 0.2 mm â€” which obliterates a 0.05-wide feature â€” then dilates
    // by 0.2 mm. The net effect: layer 0's `top_solid_fill` must be EMPTY,
    // and `top_shell_index` must remain None.
    let object_id = "sliver-cube";
    let plan = make_plan(2, 0.2, object_id);
    let region_map = make_region_map(&plan, 1, 1);
    let mesh = cuboid_mesh(object_id, 0.6); // any mesh â€” direct slice commit
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    fn square_at(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(min_x, min_y),
                    Point2::from_mm(max_x, min_y),
                    Point2::from_mm(max_x, max_y),
                    Point2::from_mm(min_x, max_y),
                ],
            },
            holes: vec![],
        }
    }

    let layer0_polys = vec![square_at(-5.0, -5.0, 5.0, 5.0)];
    let layer1_polys = vec![square_at(-5.0, -5.0, 5.0, 4.95)];

    let slice_vec: Vec<SliceIR> = vec![
        SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.2,
            regions: vec![SlicedRegion {
                object_id: object_id.to_string(),
                region_id: 0,
                polygons: layer0_polys.clone(),
                infill_areas: layer0_polys,
                ..Default::default()
            }],
        },
        SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 1,
            z: 0.4,
            regions: vec![SlicedRegion {
                object_id: object_id.to_string(),
                region_id: 0,
                polygons: layer1_polys.clone(),
                infill_areas: layer1_polys,
                ..Default::default()
            }],
        },
    ];
    bb.commit_slice_ir(Arc::new(slice_vec))
        .expect("commit_slice_ir");

    commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");

    let classified = bb.slice_ir().expect("classified slice_ir");
    let layer0 = &classified[0].regions[0];
    assert!(
        layer0.top_solid_fill.is_empty(),
        "anti-sliver opening must wipe the 0.05 mm top-edge sliver; \
         got top_solid_fill = {:?}",
        layer0.top_solid_fill
    );
    assert_eq!(
        layer0.top_shell_index, None,
        "top_shell_index must remain None when the diff is sliver-only"
    );
}

#[test]
fn shell_classification_replace_is_atomic_against_prior_slice_ir() {
    // Verify replace_slice_ir behavior: after shell-classification commits, the
    // blackboard's slice_ir slot points at the new Vec, not the original.
    let mesh = cuboid_mesh("cube", 0.6);
    let plan = make_plan(3, 0.2, "cube");
    let region_map = make_region_map(&plan, 1, 1);
    let mut bb = seeded_blackboard(mesh, plan, region_map);

    commit_slice_builtin(&mut bb).expect("PrePass::Slice");
    let pre_addr = Arc::as_ptr(bb.slice_ir().unwrap());

    commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");
    let post_addr = Arc::as_ptr(bb.slice_ir().unwrap());

    assert_ne!(
        pre_addr, post_addr,
        "replace_slice_ir must publish a new Arc, not mutate the old one"
    );
}

// ============================================================================
// minimum_sparse_infill_area (wayfinder ticket 35)
// ============================================================================

/// Build a 1-layer SliceIR whose single region is `polygons`, run shell
/// classification with the given threshold, and return the classified region.
fn classify_one_layer_with_min_sparse_area(
    polygons: Vec<ExPolygon>,
    minimum_sparse_infill_area: f32,
    sparse_infill_density: f32,
) -> SlicedRegion {
    let object_id = "min-sparse";
    let plan = make_plan(1, 0.2, object_id);
    let mut region_map = RegionMapIR::default();
    for gl in &plan.global_layers {
        for active in &gl.active_regions {
            let mut config = active.resolved_config.clone();
            // Shell counts of 0 keep top/bottom classification out of the way,
            // so the only thing that can mark this region solid is the
            // minimum-sparse-area conversion under test.
            config.top_shell_layers = 0;
            config.bottom_shell_layers = 0;
            config.minimum_sparse_infill_area = minimum_sparse_infill_area;
            config.sparse_infill_density = sparse_infill_density;
            let config_id = region_map.intern_config(config);
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: gl.index,
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config: config_id,
                    ..Default::default()
                },
            );
        }
    }
    let mesh = cuboid_mesh(object_id, 0.6);
    let mut bb = seeded_blackboard(mesh, plan, region_map);
    bb.commit_slice_ir(Arc::new(vec![SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            polygons: polygons.clone(),
            infill_areas: polygons,
            ..Default::default()
        }],
    }]))
    .expect("commit_slice_ir");

    commit_shell_classification_builtin(&mut bb).expect("PrePass::ShellClassification");
    bb.slice_ir().expect("classified slice_ir")[0].regions[0].clone()
}

fn mm_square(half_side_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-half_side_mm, -half_side_mm),
                Point2::from_mm(half_side_mm, -half_side_mm),
                Point2::from_mm(half_side_mm, half_side_mm),
                Point2::from_mm(-half_side_mm, half_side_mm),
            ],
        },
        holes: vec![],
    }
}

#[test]
fn minimum_sparse_infill_area_converts_island_at_or_below_threshold() {
    // A 3x3 mm island is 9 mm^2. At the canonical default threshold of 15 mm^2
    // it is below the bar and must become solid; at a 5 mm^2 threshold the same
    // island is above the bar and must stay sparse. Two runs differing only in
    // the key under test.
    let converted = classify_one_layer_with_min_sparse_area(vec![mm_square(1.5)], 15.0, 20.0);
    assert!(
        !converted.internal_solid_fill.is_empty(),
        "a 9 mm^2 island must be reclassified as internal solid at \
         minimum_sparse_infill_area = 15; got internal_solid_fill = {:?}",
        converted.internal_solid_fill
    );
    assert!(
        converted.bottom_solid_fill.is_empty(),
        "converted islands land in the dedicated internal-solid domain; the \
         bottom bucket must stay empty (the bottom_shell_index stamp is retired)"
    );
    assert_eq!(
        converted.bottom_shell_index, None,
        "no bottom shell exists in this fixture; the old Some(1) stamp is retired"
    );

    let kept = classify_one_layer_with_min_sparse_area(vec![mm_square(1.5)], 5.0, 20.0);
    assert!(
        kept.internal_solid_fill.is_empty(),
        "the same 9 mm^2 island must stay sparse at minimum_sparse_infill_area = 5; \
         got internal_solid_fill = {:?}",
        kept.internal_solid_fill
    );
    assert_eq!(kept.bottom_shell_index, None);
}

#[test]
fn minimum_sparse_infill_area_leaves_large_islands_sparse() {
    // A 10x10 mm island is 100 mm^2 - far above the canonical default.
    let region = classify_one_layer_with_min_sparse_area(vec![mm_square(5.0)], 15.0, 20.0);
    assert!(
        region.internal_solid_fill.is_empty(),
        "a 100 mm^2 island must never be converted at threshold 15 mm^2; \
         got internal_solid_fill = {:?}",
        region.internal_solid_fill
    );
    assert_eq!(region.bottom_shell_index, None);
}

#[test]
fn minimum_sparse_infill_area_is_disabled_at_zero_and_for_hollow_regions() {
    // Canonical guards the block on `sparse_infill_density > 0`; `0` on the
    // threshold itself disables the feature outright. Both must leave the same
    // 9 mm^2 island sparse that the first test converts.
    let disabled = classify_one_layer_with_min_sparse_area(vec![mm_square(1.5)], 0.0, 20.0);
    assert!(
        disabled.internal_solid_fill.is_empty(),
        "minimum_sparse_infill_area = 0 must disable the conversion"
    );

    let hollow = classify_one_layer_with_min_sparse_area(vec![mm_square(1.5)], 15.0, 0.0);
    assert!(
        hollow.internal_solid_fill.is_empty(),
        "a hollow region (sparse_infill_density = 0) must never gain solid fill"
    );
}

#[test]
fn minimum_sparse_infill_area_converts_only_the_small_island_of_a_mixed_layer() {
    // Two disjoint islands on one layer: 9 mm^2 (below 15) and 100 mm^2 (above).
    // Only the small one may move, and the large one must remain outside the
    // solid set - canonical erases per expolygon, not per layer.
    let small = mm_square(1.5);
    let large = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(20.0, 20.0),
                Point2::from_mm(30.0, 20.0),
                Point2::from_mm(30.0, 30.0),
                Point2::from_mm(20.0, 30.0),
            ],
        },
        holes: vec![],
    };
    let region = classify_one_layer_with_min_sparse_area(vec![small, large], 15.0, 20.0);
    assert_eq!(
        region.internal_solid_fill.len(),
        1,
        "exactly one of the two islands must convert; got {:?}",
        region.internal_solid_fill
    );
    let converted_area_mm2 =
        slicer_core::polygon_ops::expolygon_area(&region.internal_solid_fill[0]) / 1e8;
    assert!(
        (converted_area_mm2 - 9.0).abs() < 0.01,
        "the converted island must be the 9 mm^2 one, got {converted_area_mm2} mm^2"
    );
}
