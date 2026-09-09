//! Packet 240a AC-4 / AC-N3: the raft band is a POSITIVE OFFSET band, so every
//! positional consumer contract is UPHELD, not repaired.
//!
//! AC-4 - `GlobalLayer.index` still equals its position in
//! `LayerPlanIR.global_layers`; the layer executor resolves a raft layer's
//! `SliceIR` (via its private `hydrate_slice_arena`, exercised indirectly
//! through `execute_per_layer`) without producing a `FatalLayer`; and
//! `batch_slice_objects_by_layer` - the map `raw_polygons_by_layer` in
//! `crates/slicer-runtime/src/builtins/prepass_slice_producer.rs` binds - stays
//! keyed by `u32`, raft indices included. No `i32` migration.
//!
//! AC-N3 - a raft layer whose Z lies below all object geometry yields a
//! `SliceIR` with ZERO region polygons and NOT a `FatalLayer`. Canonical
//! `slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`) returns
//! one inner `Vec` per requested z, so a non-intersecting Z yields an EMPTY
//! inner entry, not a missing one.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_core::algos::prepass_slice::batch_slice_objects_by_layer;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerStageCommit,
    MeshIR, ObjectConfig, ObjectMesh, Point3, ResolvedConfig, SliceIR, StageId, Transform3d,
};
use slicer_runtime::{
    execute_per_layer, execute_prepass_slice_single_layer, Blackboard, CompiledModuleLive,
    ExecutionPlan, LayerStageError, LayerStageInput, LayerStageRunner,
};

const OBJECT_ID: &str = "obj-raft";
/// Raft layers occupy global indices `0..RAFT_LAYERS-1`.
const RAFT_LAYERS: u32 = 2;
/// Every triangle of the fixture mesh sits at or above this Z, so both raft
/// layers slice through empty space.
const OBJECT_BASE_Z: f32 = 1.0;

fn identity_transform() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

/// Unit tetrahedron whose base sits at `OBJECT_BASE_Z`, well above the raft.
fn lifted_tetra() -> IndexedTriangleSet {
    let z0 = OBJECT_BASE_Z;
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            v(0.0, 0.0, z0),
            v(1.0, 0.0, z0),
            v(0.0, 1.0, z0),
            v(0.0, 0.0, z0 + 1.0),
        ],
        indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
    }
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: OBJECT_ID.to_string(),
            mesh: lifted_tetra(),
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: OBJECT_BASE_Z + 1.0,
            },
        },
        ..Default::default()
    }
}

fn layer_at(index: u32, z: f32, is_raft: bool) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        is_raft,
        active_regions: vec![ActiveRegion {
            object_id: OBJECT_ID.to_string(),
            region_id: 0,
            resolved_config: ResolvedConfig::default(),
            effective_layer_height: 0.2,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Two raft layers (Z below all geometry) followed by three model layers, in
/// push order. Global index equals Vec position throughout. This is a
/// hand-built positional fixture, not the planner's Z-emission test; its
/// overlapping bands isolate the index and empty-slice contracts.
fn raft_band() -> Vec<GlobalLayer> {
    vec![
        layer_at(0, 0.3, true),
        layer_at(1, 0.5, true),
        layer_at(2, OBJECT_BASE_Z + 0.2, false),
        layer_at(3, OBJECT_BASE_Z + 0.4, false),
        layer_at(4, OBJECT_BASE_Z + 0.6, false),
    ]
}

fn plan_for(layers: Vec<GlobalLayer>) -> ExecutionPlan {
    ExecutionPlan {
        global_layers: Arc::new(layers),
        ..Default::default()
    }
}

/// Commits one `SliceIR` per global layer, positionally, exactly as the prepass
/// slice producer does.
fn seed_slice_ir(blackboard: &mut Blackboard, plan: &ExecutionPlan) -> Vec<SliceIR> {
    let mesh = blackboard.mesh().clone();
    let slices: Vec<SliceIR> = plan
        .global_layers
        .iter()
        .map(|gl| {
            execute_prepass_slice_single_layer(mesh.as_ref(), gl, None, None)
                .expect("prepass slice must not fail on a raft layer")
        })
        .collect();
    blackboard
        .commit_slice_ir(Arc::new(slices.clone()))
        .expect("commit_slice_ir");
    slices
}

struct NoopRunner;
impl LayerStageRunner for NoopRunner {
    fn run_stage(
        &self,
        _stage: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        Ok(None)
    }
}

fn polygon_count(slice: &SliceIR) -> usize {
    slice
        .regions
        .iter()
        .map(|region| region.polygons.len())
        .sum()
}

#[test]
fn raft_layer_index_equals_vec_position() {
    let layers = raft_band();
    let plan = plan_for(layers.clone());

    // The positional contract on the plan itself.
    for (position, layer) in plan.global_layers.iter().enumerate() {
        assert_eq!(
            layer.index, position as u32,
            "GlobalLayer.index must equal its position in global_layers, \
             raft layers included"
        );
    }
    assert!(
        plan.global_layers[..RAFT_LAYERS as usize]
            .iter()
            .all(|l| l.is_raft),
        "the fixture's leading {RAFT_LAYERS} layers must be the raft prefix"
    );

    let mut blackboard = Blackboard::new(Arc::new(mesh_fixture()), plan.global_layers.len());
    seed_slice_ir(&mut blackboard, &plan);

    // `hydrate_slice_arena` is private: exercise it indirectly through the
    // executor entry point. A raft layer must resolve its `SliceIR` slot
    // without a `FatalLayer`.
    let layer_irs = execute_per_layer(&plan, &blackboard, &NoopRunner, &Default::default())
        .expect("execute_per_layer must not produce a FatalLayer for a raft layer");

    assert_eq!(layer_irs.len(), plan.global_layers.len());
    for (position, layer_ir) in layer_irs.iter().enumerate() {
        assert_eq!(
            layer_ir.global_layer_index, position as u32,
            "emitted layer IR order must match global layer index order"
        );
    }

    // `raw_polygons_by_layer` stays `HashMap<u32, _>`: the explicit annotation
    // makes the key type a compile-time assertion, and a raft index is a valid
    // key like any other.
    let raw_polygons_by_layer: HashMap<u32, HashMap<String, Vec<ExPolygon>>> =
        batch_slice_objects_by_layer(blackboard.mesh().as_ref(), &plan.global_layers);
    for layer in plan.global_layers.iter() {
        assert!(
            raw_polygons_by_layer.contains_key(&layer.index),
            "layer {} must be keyed by its u32 global index",
            layer.index
        );
    }
}

#[test]
fn raft_layer_below_geometry_slices_empty_not_fatal() {
    let plan = plan_for(raft_band());
    let mut blackboard = Blackboard::new(Arc::new(mesh_fixture()), plan.global_layers.len());
    let slices = seed_slice_ir(&mut blackboard, &plan);

    // Every raft layer sits below all object geometry: an EMPTY slice, present
    // at its position, never a missing entry and never an error.
    for raft_index in 0..RAFT_LAYERS as usize {
        let slice = &slices[raft_index];
        assert_eq!(
            slice.global_layer_index, raft_index as u32,
            "the raft layer's SliceIR must occupy its own position"
        );
        assert_eq!(
            polygon_count(slice),
            0,
            "raft layer {raft_index} lies below all object geometry and must \
             carry zero region polygons"
        );
    }

    // The model layers above the raft still slice to real geometry, so the
    // emptiness above is the raft's Z, not a broken fixture.
    assert!(
        slices[RAFT_LAYERS as usize..]
            .iter()
            .any(|slice| polygon_count(slice) > 0),
        "model layers must still produce polygons"
    );

    // And the executor consumes the empty raft slices without a FatalLayer.
    let layer_irs = execute_per_layer(&plan, &blackboard, &NoopRunner, &Default::default())
        .expect("an empty raft slice must not be fatal");
    assert_eq!(layer_irs.len(), plan.global_layers.len());
}
