//! Packet 137: `PrePass::LightningTreeGen` skip / commit behavior (AC-3).
//!
//! Verifies the host built-in is **skipped** (no commit, slot stays `None`)
//! when the print's `sparse_fill_holder` is not `lightning-infill`, and
//! commits an empty-but-valid `LightningTreeIR` when it is.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::SliceIR;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectLayerRef, ObjectMesh, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SlicedRegion, SurfaceClassificationIR, Transform3d,
};
use slicer_runtime::{
    commit_lightning_tree_ir_builtin, execute_prepass_with_builtins_configured, Blackboard,
    ExecutionPlan, PrepassStageInput, PrepassStageOutput, PrepassStageRunner,
};
use slicer_sdk::PaintRegionLayerView;

fn minimal_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        x: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        y: 1.0,
                        ..Default::default()
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    }
}

fn blackboard_with_prereqs(mesh: MeshIR) -> Blackboard {
    let num_layers = 2u32;
    let layer_height = 0.2f32;
    let global_layers: Vec<GlobalLayer> = (0..num_layers)
        .map(|i| GlobalLayer {
            index: i,
            z: (i + 1) as f32 * layer_height,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();

    let mut object_participation = HashMap::new();
    for obj in &mesh.objects {
        object_participation.insert(
            obj.id.clone(),
            (0..num_layers)
                .map(|i| ObjectLayerRef {
                    local_layer_index: i,
                    global_layer_index: i,
                    effective_layer_height: layer_height,
                })
                .collect::<Vec<_>>(),
        );
    }

    let mut region_entries = HashMap::new();
    for obj in &mesh.objects {
        for i in 0..num_layers {
            region_entries.insert(
                RegionKey {
                    global_layer_index: i,
                    object_id: obj.id.clone(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                RegionPlan::default(),
            );
        }
    }

    let mut bb = Blackboard::new(Arc::new(mesh), 0);
    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        global_layers,
        object_participation,
        ..Default::default()
    }))
    .expect("commit_layer_plan must succeed");
    bb.commit_region_map(Arc::new(RegionMapIR {
        entries: region_entries,
        ..Default::default()
    }))
    .expect("commit_region_map must succeed");
    let stub_slices: Vec<SliceIR> = (0..num_layers)
        .map(|i| SliceIR {
            global_layer_index: i,
            ..SliceIR::default()
        })
        .collect();
    bb.commit_slice_ir(Arc::new(stub_slices))
        .expect("commit_slice_ir must succeed");
    bb.commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("pre-seeding SurfaceClassificationIR must succeed");
    bb
}

struct NoopRunner;

impl PrepassStageRunner for NoopRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_runtime::CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, slicer_ir::PrepassRunnerError> {
        Ok(PrepassStageOutput::None)
    }
}

fn empty_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

fn square(size_mm: f32, origin_x_mm: f32) -> ExPolygon {
    let size = slicer_ir::mm_to_units(size_mm);
    let origin_x = slicer_ir::mm_to_units(origin_x_mm);
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: origin_x, y: 0 },
                Point2 {
                    x: origin_x + size,
                    y: 0,
                },
                Point2 {
                    x: origin_x + size,
                    y: size,
                },
                Point2 {
                    x: origin_x,
                    y: size,
                },
            ],
        },
        holes: Vec::new(),
    }
}

fn lightning_slice_ir_with_two_regions() -> Vec<SliceIR> {
    let region = |region_id, size_mm| SlicedRegion {
        object_id: String::from("cube"),
        region_id,
        polygons: vec![square(size_mm, if region_id == 1 { 0.0 } else { 30.0 })],
        effective_layer_height: 0.2,
        ..SlicedRegion::default()
    };
    vec![
        SliceIR {
            global_layer_index: 0,
            z: 0.2,
            regions: vec![region(1, 10.0), region(2, 10.0)],
            ..SliceIR::default()
        },
        SliceIR {
            global_layer_index: 1,
            z: 0.4,
            regions: vec![region(1, 12.0), region(2, 12.0)],
            ..SliceIR::default()
        },
    ]
}

#[test]
fn lightning_producer_per_region_keying() {
    let mut blackboard = Blackboard::new(Arc::new(minimal_mesh()), 0);
    blackboard
        .commit_slice_ir(Arc::new(lightning_slice_ir_with_two_regions()))
        .expect("commit_slice_ir must succeed");
    let mut lightning_config = ResolvedConfig {
        sparse_fill_holder: String::from("lightning-infill"),
        ..ResolvedConfig::default()
    };
    let active_region = |region_id| {
        let mut resolved_config = lightning_config.clone();
        if region_id == 2 {
            resolved_config.line_width = 0.5;
        }
        ActiveRegion {
            object_id: String::from("cube"),
            region_id,
            resolved_config,
            effective_layer_height: 0.2,
            ..ActiveRegion::default()
        }
    };
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR {
            global_layers: (0..2)
                .map(|index| GlobalLayer {
                    index,
                    z: (index + 1) as f32 * 0.2,
                    active_regions: vec![active_region(1), active_region(2)],
                    ..GlobalLayer::default()
                })
                .collect(),
            ..LayerPlanIR::default()
        }))
        .expect("commit_layer_plan must succeed");

    commit_lightning_tree_ir_builtin(&mut blackboard, &lightning_config)
        .expect("lightning producer must commit");
    let ir = blackboard
        .lightning_tree_ir()
        .expect("lightning tree IR must be committed");
    // Exactly one entry per region, both on layer 1 — two in total.
    //
    // The fixture is 2 global layers x 2 regions. Per region, layer 0 is a
    // 10 mm square and layer 1 a 12 mm square, so:
    //
    //  * layer 0 has no predecessor, hence no internal overhang, hence no
    //    seeded trees and no segments of its own;
    //  * layer 1's overhang is the band between the dilated 10 mm square and
    //    the 12 mm square, so layer 1 seeds trees grounded on its own outline;
    //  * every node of those trees lies in that band, i.e. outside layer 0's
    //    10 mm outline, so canonical `Node::realign` takes its "outside" branch
    //    at every node and the propagated copy is discarded. Layer 0 therefore
    //    stays empty and contributes no entry.
    //
    // The two regions are identical up to a 30 mm x-offset, so the result is
    // symmetric. Before `realign` was ported, `propagate_to_next_layer` copied
    // layer 1's trees down unconditionally and the IR carried 3 entries — only
    // three of the four possible (layer, region) keys, so at least one spurious
    // layer-0 entry and an asymmetry between two geometrically identical
    // regions.
    assert_eq!(ir.entries.len(), 2);
    let region_ids: std::collections::BTreeSet<_> =
        ir.entries.iter().map(|entry| entry.region_id).collect();
    assert_eq!(region_ids, [1, 2].into_iter().collect());
    assert!(ir.entries.iter().all(|entry| entry.object_id == "cube"));
    assert!(ir.entries.iter().all(|entry| entry.global_layer_index == 1));

    let view = PaintRegionLayerView::new(1).with_lightning_tree_ir(Arc::clone(ir));
    let region_one = view.lightning_tree_segments_for("cube", 1);
    let region_two = view.lightning_tree_segments_for("cube", 2);
    assert!(!region_one.is_empty());
    assert!(!region_two.is_empty());
    assert!(region_one
        .iter()
        .flatten()
        .all(|point| point.x < slicer_ir::mm_to_units(20.0)));
    assert!(region_two
        .iter()
        .flatten()
        .all(|point| point.x >= slicer_ir::mm_to_units(30.0)));
    lightning_config.sparse_fill_holder = String::from("rectilinear-infill");
    assert_eq!(
        slicer_core::algos::lightning::generate_lightning_trees(
            &lightning_slice_ir_with_two_regions(),
            &lightning_config,
        )
        .expect("skip path must not fail")
        .entries,
        Vec::new()
    );
}

#[test]
fn lightning_prepass_is_skipped_when_sparse_fill_holder_is_rectilinear() {
    let mesh = minimal_mesh();
    let mut blackboard = blackboard_with_prereqs(mesh);

    let default_resolved = ResolvedConfig::default();
    assert_ne!(default_resolved.sparse_fill_holder, "lightning-infill");

    let result = execute_prepass_with_builtins_configured(
        &empty_plan(),
        &mut blackboard,
        &NoopRunner,
        &BTreeMap::new(),
        &default_resolved,
        &HashMap::new(),
        &slicer_runtime::ConfigBoundsIndex::default(),
        &HashMap::new(),
    );

    result.expect("prepass execution must succeed when lightning is not configured");
    assert!(
        blackboard.lightning_tree_ir().is_none(),
        "LightningTreeIR slot must stay None when sparse_fill_holder != lightning-infill (AC-3 skip promise)"
    );
}

#[test]
fn lightning_prepass_commits_when_sparse_fill_holder_is_lightning() {
    let mesh = minimal_mesh();
    let mut blackboard = blackboard_with_prereqs(mesh);

    let default_resolved = ResolvedConfig {
        sparse_fill_holder: String::from("lightning-infill"),
        ..ResolvedConfig::default()
    };

    let result = execute_prepass_with_builtins_configured(
        &empty_plan(),
        &mut blackboard,
        &NoopRunner,
        &BTreeMap::new(),
        &default_resolved,
        &HashMap::new(),
        &slicer_runtime::ConfigBoundsIndex::default(),
        &HashMap::new(),
    );

    result.expect("prepass execution must succeed when lightning is configured");
    assert!(
        blackboard.lightning_tree_ir().is_some(),
        "LightningTreeIR must be committed when sparse_fill_holder == lightning-infill"
    );
}
