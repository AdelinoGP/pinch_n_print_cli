//! Packet 240a AC-3 / AC-N2: the raft substrate band produced by
//! `com.core.layer-planner-default`.
//!
//! AC-3 — with `support_raft_layers > 0` the planner pushes exactly that many
//! `is_raft: true` proposals BEFORE any model proposal, each with at least one
//! `active_regions` entry; with `support_raft_layers = 0` it pushes none and the
//! model Z sequence is bit-identical to the pre-raft behaviour.
//!
//! AC-N2 — the resulting `LayerCollectionIR` sequence has monotonic global
//! layer INDICES across the raft/model boundary, so
//! `execute_layer_finalization_with_instrumentation`'s "layer indices must be
//! monotonic" gate accepts it, and Vec order matches index order. The gate is
//! index-based and never inspects `z`; raft and model Zs deliberately overlap
//! in 240a (see `raft_band_emitted_before_model_layers`).

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use layer_planner_default::DefaultLayerPlanner;
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, FinalizationError, LayerCollectionIR, MeshIR,
    ObjectMesh, Point3, SemVer, StageId, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_layer_finalization, Blackboard, CompiledModule,
    CompiledModuleBuilder, CompiledModuleLive, CompiledStage, ExecutionModuleBinding,
    ExecutionPlan, FinalizationOutput, FinalizationStageInput, FinalizationStageRunner,
    LoadedModuleBuilder, WasmArtifactMetadata,
};
use slicer_sdk::prepass_builders::LayerPlanOutput;
use slicer_sdk::prepass_types::LayerProposal;
use slicer_sdk::traits::PrepassModule;

const LAYER_HEIGHT: f64 = 0.2;
const FIRST_LAYER_HEIGHT: f64 = 0.3;
const OBJECT_HEIGHT: f64 = 1.0;
const OBJECT_ID: &str = "obj-1";

/// Drive the planner natively with the given raft count and return its
/// proposals in push order.
fn plan_with_raft_layers(raft_layers: i64) -> Vec<LayerProposal> {
    let mut map: HashMap<String, ConfigValue> = HashMap::new();
    map.insert("layer_height".into(), ConfigValue::Float(LAYER_HEIGHT));
    map.insert(
        "first_layer_height".into(),
        ConfigValue::Float(FIRST_LAYER_HEIGHT),
    );
    map.insert(
        format!("object_height:{OBJECT_ID}"),
        ConfigValue::Float(OBJECT_HEIGHT),
    );
    map.insert("support_raft_layers".into(), ConfigValue::Int(raft_layers));
    let config = ConfigView::from_map(map);

    let planner = DefaultLayerPlanner::from_config(&config).expect("planner from_config");
    let mut output = LayerPlanOutput::new();
    planner
        .run_layer_planning(&[OBJECT_ID.to_string()], &mut output, &config)
        .expect("layer planning");
    output.layers().to_vec()
}

/// Literal expected Z values for `layer_height 0.2`, `first_layer_height 0.3`,
/// `object_height 1.0`. Deliberately NOT recomputed from the planner's formula:
/// a test-side reimplementation tracks a formula change silently, which is how
/// an erroneous model-Z raft shift previously passed this suite.
const EXPECTED_MODEL_ZS: [f32; 4] = [0.3, 0.5, 0.7, 0.9];
/// Literal expected raft Z values for `support_raft_layers = 3`.
const EXPECTED_RAFT_ZS: [f32; 3] = [0.3, 0.5, 0.7];

/// Compare an f32 Z sequence against literal expectations. `0.3` etc. are not
/// exactly representable, so compare with a tolerance far below a layer height.
#[track_caller]
fn assert_zs_eq(actual: &[f32], expected: &[f32], what: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{what}: expected {expected:?}, got {actual:?}"
    );
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a - e).abs() < 1e-5,
            "{what}: index {i} expected {e}, got {a} (full: {actual:?})"
        );
    }
}

#[test]
fn raft_band_emitted_before_model_layers() {
    let raft_layers = 3usize;
    let proposals = plan_with_raft_layers(raft_layers as i64);

    let raft_count = proposals.iter().filter(|p| p.is_raft).count();
    assert_eq!(
        raft_count, raft_layers,
        "exactly `support_raft_layers` raft proposals expected, got {raft_count}"
    );

    // The raft proposals must be the leading prefix.
    for (i, p) in proposals.iter().enumerate() {
        assert_eq!(
            p.is_raft,
            i < raft_layers,
            "proposal {i} is_raft={} but raft prefix is 0..{raft_layers}",
            p.is_raft
        );
    }

    // AC-3: Z computed in f64 with a single terminal `as f32`.
    let actual_raft_zs: Vec<f32> = proposals[..raft_layers].iter().map(|p| p.z).collect();
    assert_zs_eq(
        &actual_raft_zs,
        &EXPECTED_RAFT_ZS,
        "raft Z values must be first + i*layer_height",
    );

    // AC-3: at least one active region per raft layer.
    for (i, p) in proposals[..raft_layers].iter().enumerate() {
        assert!(
            !p.active_regions.is_empty(),
            "raft layer {i} must carry at least one active region"
        );
    }

    // Model Zs carry NO raft offset: `GlobalLayer.z` is the mesh cutting plane
    // and PnP has no `print_z`/`slice_z` split to absorb one. Raft and model
    // bands therefore overlap in Z; that is accepted for this substrate packet
    // (no consumer of raft layers exists yet) and 240b owns the real Z model.
    let model_zs: Vec<f32> = proposals[raft_layers..].iter().map(|p| p.z).collect();
    assert_zs_eq(
        &model_zs,
        &EXPECTED_MODEL_ZS,
        "model Z values must be unshifted by the raft band",
    );
}

#[test]
fn no_raft_band_when_raft_layers_zero() {
    let proposals = plan_with_raft_layers(0);

    assert!(
        proposals.iter().all(|p| !p.is_raft),
        "no raft proposals may be emitted when support_raft_layers = 0"
    );
    assert!(!proposals.is_empty(), "model layers must still be emitted");

    // Regression safety: with a zero raft offset the model Z sequence is
    // bit-identical to the pre-raft behaviour.
    let zs: Vec<f32> = proposals.iter().map(|p| p.z).collect();
    assert_zs_eq(
        &zs,
        &EXPECTED_MODEL_ZS,
        "model Z sequence must be unchanged",
    );
}

#[test]
fn raft_band_satisfies_finalization_monotonic_gate() {
    let raft_layers = 3usize;
    let proposals = plan_with_raft_layers(raft_layers as i64);

    // Global layer indices follow push order: raft occupies 0..N-1, model N.. .
    let mut layers: Vec<LayerCollectionIR> = proposals
        .iter()
        .enumerate()
        .map(|(i, p)| layer_collection_fixture(i as u32, p.z))
        .collect();

    // Vec order matches index order, and the indices the gate actually inspects
    // are strictly increasing across the raft/model boundary. Z is NOT part of
    // the gate's contract and is deliberately overlapping here.
    for (i, layer) in layers.iter().enumerate() {
        assert_eq!(
            layer.global_layer_index, i as u32,
            "Vec order must match global layer index order"
        );
    }
    for window in layers.windows(2) {
        assert!(
            window[1].global_layer_index > window[0].global_layer_index,
            "global layer indices must be monotonic across the raft/model boundary: {} then {}",
            window[0].global_layer_index,
            window[1].global_layer_index
        );
    }

    // The real finalization gate must accept the sequence.
    let blackboard = Blackboard::new(Arc::new(mesh_fixture()), 0);
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["com.example.raft-noop-finalizer"],
    )));

    struct NoopRunner;
    impl FinalizationStageRunner for NoopRunner {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModuleLive<'_>,
            _input: FinalizationStageInput<'_>,
            _layers: &mut Vec<LayerCollectionIR>,
        ) -> Result<FinalizationOutput, FinalizationError> {
            Ok(FinalizationOutput::Success)
        }
    }

    let result = execute_layer_finalization(
        &plan,
        &blackboard,
        &NoopRunner,
        &mut layers,
        &Default::default(),
    );
    assert_eq!(
        result,
        Ok(()),
        "raft-prefixed layer sequence must pass the finalization monotonic gate"
    );
}

// ── fixtures ─────────────────────────────────────────────────────────────────

fn execution_plan_fixture(layer_finalization_stage: Option<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        layer_finalization_stage,
        ..Default::default()
    }
}

fn compiled_stage(stage_id: &str, module_ids: &[&str]) -> CompiledStage {
    CompiledStage {
        stage_id: String::from(stage_id),
        modules: module_ids
            .iter()
            .map(|module_id| compiled_module(stage_id, module_id))
            .collect(),
    }
}

fn compiled_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded_module = LoadedModuleBuilder::new(
        module_id,
        semver(1, 0, 0),
        stage_id,
        slicer_schema::TIER_FINALIZATION,
        PathBuf::from(format!("fixtures/{module_id}.wasm")),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build();

    let _instance_pool = Arc::new(
        build_wasm_instance_pool(
            loaded_module.id(),
            loaded_module.stage(),
            loaded_module.layer_parallel_safe(),
            8,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );

    let binding = ExecutionModuleBinding {
        module: loaded_module,
        config_view: Arc::new(ConfigView::new()),
    };

    CompiledModuleBuilder::new(binding.module.id().to_string()).build()
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from(OBJECT_ID),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![],
                indices: vec![],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
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
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

fn layer_collection_fixture(index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: index,
        z,
        ordered_entities: Vec::new(),
        ..Default::default()
    }
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
