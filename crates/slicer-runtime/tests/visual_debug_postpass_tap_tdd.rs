//! Contract coverage for the whole-print PostPass visual-debug tap capture
//! path (packet 161, Step 5): `slicer_runtime::postpass::execute_postpass_with_capture`
//! plus the two new `CapturedIr` variants it feeds
//! (`CapturedIr::LayerFinalization`, `CapturedIr::GCodeEmit`).
//!
//! Distinct from the arena (`execute_captured_stages`, packet 158) and
//! Blackboard-read (`execute_blackboard_taps`, packet 161 Steps 3-4) tap
//! closures (ADR-0040 "three tap classes"): these two taps' source IR does
//! not exist until the *whole* per-layer -> finalization -> postpass
//! pipeline prefix has run — there is no bounded per-layer truncation
//! available. This test drives that whole prefix directly (no WASM, no
//! prepass) via:
//! - a custom `LayerStageRunner` that seeds a known `LayerCollectionIR` per
//!   layer, mirroring `tool_ordering_tdd.rs`'s
//!   `LiveDispatcherWithLayerCollection` / `LayerStageCommit::SeedLayerCollection`
//!   pattern;
//! - a `StubEmitter`/`StubSerializer` pair returning a fixed `GCodeIR`,
//!   mirroring `postpass_executor_tdd.rs`'s `StubEmitter`/`StubSerializer`.
//!
//! Both are existing deterministic fixture patterns already used elsewhere
//! in this test suite — no new geometry generator is authored here.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_ir::{
    BoundingBox3, ExtrusionPath3D, ExtrusionRole, GCodeCommand, GCodeIR, GlobalLayer,
    LayerCollectionIR, LayerStageCommit, MeshIR, Point3, Point3WithWidth, PrintEntity,
    PrintMetadata, RegionKey, SemVer, SliceIR, StageId, TravelMove,
    CURRENT_GCODE_IR_SCHEMA_VERSION, CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
};
use slicer_runtime::layer_executor::POSTPASS_TAP_STAGE_IDS;
use slicer_runtime::postpass::{execute_postpass_with_capture, PostPassCapture};
use slicer_runtime::{
    build_wasm_instance_pool, execute_layer_finalization, execute_per_layer_with_events,
    Blackboard, CapturedIr, CompiledModule, CompiledModuleBuilder, CompiledModuleLive,
    CompiledStage, ExecutionModuleBinding, ExecutionPlan, FinalizationError, FinalizationOutput,
    FinalizationStageInput, FinalizationStageRunner, GCodeEmitError, GCodeEmitter, GCodeSerializer,
    LayerStageError, LayerStageInput, LayerStageRunner, LoadedModuleBuilder, NoopInstrumentation,
    NoopLayerProgressSink, PostpassError, PostpassOutput, PostpassStageInput, PostpassStageRunner,
    WasmArtifactMetadata,
};

// ─────────────────────────────── Fixtures ──────────────────────────────────

fn semver_1_0_0() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        objects: Vec::new(),
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
        ..Default::default()
    })
}

/// Deterministic `LayerCollectionIR` for one global layer with `ordered_entities`
/// and `travel_moves` both populated (this test's assertions pin their
/// presence). No Skirt/WipeTower entities, so
/// `slicer_gcode::reconcile_finalization_travel` is a no-op on it — the
/// captured `finalized_layers` is byte-identical to what is seeded here.
fn seeded_layer_collection(index: u32, z: f32) -> LayerCollectionIR {
    let entity = PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            global_layer_index: index,
            ..Default::default()
        },
        topo_order: 0,
        tool_index: 0,
    };
    LayerCollectionIR {
        schema_version: CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
        global_layer_index: index,
        z,
        ordered_entities: vec![entity],
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: vec![TravelMove {
            entity_id: 1,
            x: Some(0.0),
            y: Some(0.0),
            z: Some(z),
            f: Some(1200.0),
        }],
    }
}

/// `LayerStageRunner` that seeds a known `LayerCollectionIR` for
/// `Layer::PathOptimization` on every layer (mirrors `tool_ordering_tdd.rs`'s
/// `LiveDispatcherWithLayerCollection`), and records every layer index it was
/// invoked for — proving the whole print (not a truncated subset) executed.
struct SeedingLayerRunner {
    executed_layers: Mutex<Vec<u32>>,
}

impl SeedingLayerRunner {
    fn new() -> Self {
        Self {
            executed_layers: Mutex::new(Vec::new()),
        }
    }

    fn executed_layers_sorted(&self) -> Vec<u32> {
        let mut v = self.executed_layers.lock().expect("lock").clone();
        v.sort_unstable();
        v
    }
}

impl LayerStageRunner for SeedingLayerRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        if stage_id == "Layer::PathOptimization" {
            self.executed_layers.lock().expect("lock").push(layer.index);
            return Ok(Some(LayerStageCommit::SeedLayerCollection(
                seeded_layer_collection(layer.index, layer.z),
            )));
        }
        Ok(None)
    }
}

/// Finalization runner: never invoked in this test (`layer_finalization_stage`
/// is `None`), but `FinalizationStageRunner` has no default impl so a
/// concrete type is still required to call `execute_layer_finalization`.
struct UnusedFinalizationRunner;
impl FinalizationStageRunner for UnusedFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: FinalizationStageInput<'_>,
        _layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        panic!("no layer_finalization_stage is bound in this test's plan; must not be invoked")
    }
}

/// Postpass runner: never invoked in this test (`postpass_stages` is empty),
/// but `PostpassStageRunner` has no default impl for its two required
/// methods either.
struct UnusedPostpassRunner;
impl PostpassStageRunner for UnusedPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        _commands: &mut Vec<GCodeCommand>,
    ) -> Result<PostpassOutput, PostpassError> {
        panic!(
            "no PostPass::GCodePostProcess stage is bound in this test's plan; must not be invoked"
        )
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        _text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        panic!(
            "no PostPass::TextPostProcess stage is bound in this test's plan; must not be invoked"
        )
    }
}

/// Fixed `GCodeIR` the `StubEmitter` returns regardless of input — mirrors
/// `postpass_executor_tdd.rs`'s `gcode_ir_fixture`, but with a `Move`
/// command (rather than only a `Comment`) so this test can pin `Move`
/// fields per the assigned step's requirement.
fn stub_gcode_ir() -> GCodeIR {
    GCodeIR {
        schema_version: CURRENT_GCODE_IR_SCHEMA_VERSION,
        commands: vec![
            GCodeCommand::Comment {
                text: "stub emit".to_string(),
            },
            GCodeCommand::Move {
                x: Some(1.0),
                y: Some(2.0),
                z: Some(0.3),
                e: Some(0.5),
                f: Some(1500.0),
                role: ExtrusionRole::OuterWall,
            },
        ],
        metadata: PrintMetadata {
            estimated_print_time_s: 42,
            filament_used_mm: vec![10.0],
            layer_count: 2,
            slicer_version: "test".to_string(),
        },
    }
}

struct StubEmitter;
impl GCodeEmitter for StubEmitter {
    fn emit_gcode(&self, _layer_irs: &[LayerCollectionIR]) -> Result<GCodeIR, GCodeEmitError> {
        Ok(stub_gcode_ir())
    }
}

struct StubSerializer;
impl GCodeSerializer for StubSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError> {
        Ok("stub gcode text".to_string())
    }
}

fn compiled_module_fixture(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModuleBuilder::new(
        module_id,
        semver_1_0_0(),
        stage_id,
        String::new(),
        PathBuf::from(format!("fixtures/{module_id}.wasm")),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let _pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture pool"),
    );
    let binding = ExecutionModuleBinding {
        module: loaded,
        config_view: Arc::new(slicer_ir::ConfigView::from_map(HashMap::new())),
    };
    CompiledModuleBuilder::new(binding.module.id().to_string())
        .config_view(Arc::clone(&binding.config_view))
        .build()
}

fn stage_fixture(stage_id: &str, module_id: &str) -> CompiledStage {
    CompiledStage {
        stage_id: stage_id.to_string(),
        modules: vec![compiled_module_fixture(stage_id, module_id)],
    }
}

/// A 2-layer plan: one `Layer::PathOptimization` stage (dispatched to
/// `SeedingLayerRunner`), no finalization module, no postpass modules — the
/// minimal shape needed to drive `execute_per_layer_with_events` ->
/// `execute_layer_finalization` -> `execute_postpass_with_capture`
/// end-to-end without any WASM or prepass machinery.
fn two_layer_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![stage_fixture(
            "Layer::PathOptimization",
            "com.test.path-optimization",
        )],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: true,
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

// ─────────────────────────────── The test ──────────────────────────────────

#[test]
fn postpass_whole_print_tap_contracts() {
    // Both new taps are documented in the third tap class's stage-id list.
    assert_eq!(
        POSTPASS_TAP_STAGE_IDS,
        &["PostPass::LayerFinalization", "PostPass::GCodeEmit"]
    );

    let plan = two_layer_plan();
    let mut blackboard = Blackboard::new(empty_mesh_ir(), 2);
    // Tier 2 requires `Blackboard::slice_ir()` to already be committed (the
    // host-built-in `PrePass::Slice` precondition, checked at the top of
    // every layer's per-layer closure) — seed one minimal `SliceIR` per
    // layer, indexed by position to match `GlobalLayer::index`.
    blackboard
        .commit_slice_ir(Arc::new(vec![
            SliceIR {
                schema_version: semver_1_0_0(),
                global_layer_index: 0,
                z: 0.2,
                regions: Vec::new(),
            },
            SliceIR {
                schema_version: semver_1_0_0(),
                global_layer_index: 1,
                z: 0.4,
                regions: Vec::new(),
            },
        ]))
        .expect("commit_slice_ir on a fresh Blackboard must succeed");
    let wasm_handles = HashMap::new();

    // ---- Tier 2: run every per-layer stage, for every layer. ----
    let seeding_runner = SeedingLayerRunner::new();
    let (mut layer_irs, _layer_audits) = execute_per_layer_with_events(
        &plan,
        &blackboard,
        &seeding_runner,
        &NoopLayerProgressSink,
        &wasm_handles,
    )
    .expect("per-layer execution must succeed");
    assert_eq!(
        seeding_runner.executed_layers_sorted(),
        vec![0, 1],
        "the whole print (both layers), not a truncated subset, must have executed"
    );
    assert_eq!(layer_irs.len(), 2);

    // ---- Tier 3: layer finalization (no module bound; must still run/no-op). ----
    execute_layer_finalization(
        &plan,
        &blackboard,
        &UnusedFinalizationRunner,
        &mut layer_irs,
        &wasm_handles,
    )
    .expect("finalization tier must succeed even with no bound module");

    // ---- Tier 4: postpass, with the read-only capture sink engaged. ----
    let emitter = StubEmitter;
    let serializer = StubSerializer;
    let mut postpass_runner = UnusedPostpassRunner;
    let mut capture = PostPassCapture::default();
    let (gcode_text, postpass_audits) = execute_postpass_with_capture(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut postpass_runner,
        &NoopInstrumentation,
        &wasm_handles,
        Some(&mut capture),
    )
    .expect("postpass execution must succeed");

    // Ordinary emission is unaffected by the capture sink: same serialized
    // text and audits a caller passing `None` would get (no GCodePostProcess/
    // TextPostProcess modules bound, so no audits either way).
    assert_eq!(gcode_text, "stub gcode text");
    assert!(postpass_audits.is_empty());

    // ---- Assert on the captured finalized layers (`PostPass::LayerFinalization`). ----
    assert_eq!(capture.finalized_layers.len(), 2);
    let mut seen_indices: Vec<u32> = capture
        .finalized_layers
        .iter()
        .map(|l| l.global_layer_index)
        .collect();
    seen_indices.sort_unstable();
    assert_eq!(seen_indices, vec![0, 1]);
    for layer in &capture.finalized_layers {
        assert_eq!(
            layer.schema_version, CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
            "captured finalized layer must carry the current LayerCollectionIR schema version"
        );
        assert_eq!(
            layer.ordered_entities.len(),
            1,
            "captured finalized layer must retain its seeded ordered_entities"
        );
        assert_eq!(
            layer.travel_moves.len(),
            1,
            "captured finalized layer must retain its seeded travel_moves \
             (no Skirt/WipeTower entities exist, so reconcile_finalization_travel is a no-op)"
        );
    }

    // ---- Assert on the captured emitted GCodeIR (`PostPass::GCodeEmit`). ----
    assert_eq!(
        capture.gcode_ir.schema_version,
        CURRENT_GCODE_IR_SCHEMA_VERSION
    );
    assert_eq!(capture.gcode_ir.commands.len(), 2);
    let move_cmd = capture
        .gcode_ir
        .commands
        .iter()
        .find(|c| matches!(c, GCodeCommand::Move { .. }))
        .expect("captured GCodeIR must contain the emitted Move command");
    match move_cmd {
        GCodeCommand::Move {
            x,
            y,
            z,
            e,
            f,
            role,
        } => {
            assert_eq!(*x, Some(1.0));
            assert_eq!(*y, Some(2.0));
            assert_eq!(*z, Some(0.3));
            assert_eq!(*e, Some(0.5));
            assert_eq!(*f, Some(1500.0));
            assert_eq!(*role, ExtrusionRole::OuterWall);
        }
        _ => unreachable!("matched above"),
    }

    // ---- CapturedIr wrapping + schema_version_string (feeds the manifest's ir_schema_version). ----
    let layer_finalization_ir = CapturedIr::LayerFinalization(capture.finalized_layers.clone());
    assert_eq!(
        layer_finalization_ir.schema_version_string(),
        format!(
            "{}.{}.{}",
            CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION.major,
            CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION.minor,
            CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION.patch
        )
    );
    let gcode_emit_ir = CapturedIr::GCodeEmit(capture.gcode_ir.clone());
    assert_eq!(
        gcode_emit_ir.schema_version_string(),
        format!(
            "{}.{}.{}",
            CURRENT_GCODE_IR_SCHEMA_VERSION.major,
            CURRENT_GCODE_IR_SCHEMA_VERSION.minor,
            CURRENT_GCODE_IR_SCHEMA_VERSION.patch
        )
    );

    // ---- Manifest-level closure semantics: only the request's selected ----
    // layer is rendered, but the recorded closure is whole-print (this
    // mirrors exactly what `pnp-cli::visual_debug::run_postpass_taps` builds
    // from these same primitives).
    let requested_layer_indices = vec![1u32]; // caller asked for layer 1 only
    let real_layer_indices: std::collections::BTreeSet<u32> = capture
        .finalized_layers
        .iter()
        .map(|l| l.global_layer_index)
        .collect();
    let applicable: std::collections::BTreeSet<u32> = requested_layer_indices
        .iter()
        .copied()
        .filter(|i| real_layer_indices.contains(i))
        .collect();
    assert_eq!(applicable, std::collections::BTreeSet::from([1u32]));

    let mut rendered_captures: Vec<(String, u32)> = Vec::new();
    for &layer_index in &applicable {
        for tap in POSTPASS_TAP_STAGE_IDS {
            rendered_captures.push((tap.to_string(), layer_index));
        }
    }
    assert_eq!(
        rendered_captures,
        vec![
            ("PostPass::LayerFinalization".to_string(), 1),
            ("PostPass::GCodeEmit".to_string(), 1),
        ],
        "only the request's selected layer (1) must appear as a rendered StageCapture row, \
         even though both layers 0 and 1 actually executed"
    );

    // Whole-print closure: every per-layer stage plus the two PostPass stage
    // ids plus GCodeSerialize, and every real layer index — NOT just the
    // requested subset.
    let mut closure_stage_ids: Vec<String> = plan
        .per_layer_stages
        .iter()
        .map(|s| s.stage_id.clone())
        .collect();
    closure_stage_ids.push("PostPass::LayerFinalization".to_string());
    closure_stage_ids.push("PostPass::GCodeEmit".to_string());
    closure_stage_ids.extend(plan.postpass_stages.iter().map(|s| s.stage_id.clone()));
    closure_stage_ids.push("PostPass::GCodeSerialize".to_string());
    assert_eq!(
        closure_stage_ids,
        vec![
            "Layer::PathOptimization".to_string(),
            "PostPass::LayerFinalization".to_string(),
            "PostPass::GCodeEmit".to_string(),
            "PostPass::GCodeSerialize".to_string(),
        ]
    );
    let executed_layer_indices: Vec<u32> = real_layer_indices.into_iter().collect();
    assert_eq!(
        executed_layer_indices,
        vec![0, 1],
        "executed_layer_indices must record the WHOLE print (both layers), \
         not just the request's selected layer (1) — the documented \
         minimal-closure deviation for PostPass taps"
    );
}
