//! TDD: host tool ordering before path-optimization (packet-19).
//!
//! Covers TASK-152b.

#![allow(missing_docs)]

use crate::common::seed::seed_slice_ir;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_runtime::instance_pool::build_wasm_instance_pool;
use slicer_runtime::manifest::{LoadedModule, LoadedModuleBuilder};
use slicer_ir::LayerStageCommitData;
use slicer_runtime::{
    execute_per_layer, Blackboard, CompiledModule, CompiledModuleBuilder, CompiledModuleLive,
    CompiledStage, ExecutionModuleBinding, ExecutionPlan, LayerStageError, LayerStageInput,
    LayerStageRunner, WasmArtifactMetadata, WasmEngine, WasmRuntimeDispatcher,
};

const PATH_OPT_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../modules/core-modules/path-optimization-default/path-optimization-default.wasm"
);
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer,
    IndexedTriangleSet, LayerCollectionIR, MeshIR, ObjectConfig, ObjectMesh, Point3,
    Point3WithWidth, PrintEntity, RegionKey, ResolvedConfig, SemVer, StageId, ToolChange,
    Transform3d,
};

// â”€â”€ Fixtures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn semver() -> SemVer {
    SemVer::default()
}

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

/// Build a PrintEntity with SparseInfill role and the given tool_index
/// embedded in region_id (the only available conduit for tool identity at
/// the IR surface â€” the module's PerimeterRegionView carries tool_index via
/// the ActiveRegion it is constructed from, even though PrintEntity itself
/// does not store it as a field).
fn entity_with_tool(
    x: f32,
    y: f32,
    tool_index: u32,
    object_id: &str,
    original_idx: u32,
) -> PrintEntity {
    PrintEntity {
        entity_id: (original_idx as u64) + 1,
        path: ExtrusionPath3D {
            points: vec![pt(x, y)],
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::SparseInfill,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: object_id.to_string(),
            // Tool index is propagated through region_id at assembly time
            // via the host's per-region ActiveRegion.tool_index.
            region_id: tool_index as u64,
        },
        topo_order: original_idx,
    }
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// TEST 1: mixed_tool_layer_emits_deterministic_tool_change_sequence
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-1 (TASK-152b): Given a mixed-tool layer with entities in raw assembly
/// order [tool0@30, tool2@10, tool1@0, tool0@20, tool2@15, tool1@5], the live
/// WASM dispatch for Layer::PathOptimization must group entities by tool in
/// ascending tool_index order: all tool0 first, then tool1, then tool2.
///
/// Expected ordered_entities tool sequence: [tool0, tool0, tool1, tool1, tool2, tool2]
/// Expected tool_changes: [after_entity_index=1, from=0, to=1], [after_entity_index=3, from=1, to=2]
///
/// Currently FAILS: path-optimization-default.wasm runs nearest-neighbor on
/// position only, ignoring tool_index entirely, so entities are reordered by
/// travel cost rather than tool grouping.
#[test]
fn mixed_tool_layer_emits_deterministic_tool_change_sequence() {
    // Raw assembly order has tools interleaved:
    // [tool0@30, tool2@10, tool1@0, tool0@20, tool2@15, tool1@5]
    let ordered_entities = vec![
        entity_with_tool(30.0, 0.0, 0, "test-object", 0), // tool 0
        entity_with_tool(10.0, 0.0, 2, "test-object", 1), // tool 2
        entity_with_tool(0.0, 0.0, 1, "test-object", 2),  // tool 1
        entity_with_tool(20.0, 0.0, 0, "test-object", 3), // tool 0
        entity_with_tool(15.0, 0.0, 2, "test-object", 4), // tool 2
        entity_with_tool(5.0, 0.0, 1, "test-object", 5),  // tool 1
    ];

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);

    // Pre-stage the layer collection with the raw (interleaved) entity order.
    // The executor will find this pre-staged IR before Layer::PathOptimization
    // and the module's set_entity_order will be validated against it.
    let layer_collection = LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities,
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    };

    let plan = plan_with_stages(
        vec![
            stage("Layer::Infill", "com.test.infill"),
            stage(
                "Layer::PathOptimization",
                "com.core.path-optimization-default",
            ),
        ],
        1,
    );
    seed_slice_ir(&mut blackboard, &plan);

    let engine = Arc::new(WasmEngine::new());
    let path_opt_component = load_path_optimization_module(&engine);

    let path_opt_loaded = path_optimization_loaded_module();
    let path_opt_pool = Arc::new(
        build_wasm_instance_pool(
            path_opt_loaded.id(),
            path_opt_loaded.stage(),
            path_opt_loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture pool"),
    );
    let path_opt_module = Arc::new(
        CompiledModuleBuilder::new(path_opt_loaded.id().to_string(), Arc::clone(&path_opt_pool))
            .wasm_component(Some(path_opt_component))
            .build(),
    );

    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let runner = LiveDispatcherWithLayerCollection::with_module(
        layer_collection,
        Arc::new(dispatcher),
        path_opt_module,
    );

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    // Assert: entities grouped by tool in ascending order.
    // region_id encodes tool_index (0, 1, 2 â†’ ascending).
    let tool_sequence: Vec<u64> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.region_key.region_id)
        .collect();
    assert_eq!(
        tool_sequence,
        vec![0u64, 0, 1, 1, 2, 2],
        "expected tool-grouped order [0,0,1,1,2,2], got {tool_sequence:?}"
    );

    // Assert: two tool change boundaries (tool0â†’tool1 at index 1, tool1â†’tool2 at index 3).
    assert_eq!(
        layers[0].tool_changes.len(),
        2,
        "expected 2 tool_changes, got {}",
        layers[0].tool_changes.len()
    );
    assert_eq!(
        layers[0].tool_changes[0],
        ToolChange {
            after_entity_index: 1,
            from_tool: 0,
            to_tool: 1
        },
        "first tool_change should be 0â†’1 at boundary after entity 1"
    );
    assert_eq!(
        layers[0].tool_changes[1],
        ToolChange {
            after_entity_index: 3,
            from_tool: 1,
            to_tool: 2
        },
        "second tool_change should be 1â†’2 at boundary after entity 3"
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// TEST 2: single_tool_layer_emits_no_synthetic_tool_changes
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-2: A layer where all entities share tool_index=0 emits no synthetic
/// tool_changes through the live WASM dispatch path.
///
/// This already PASSES with the current module: single-tool reduces to pure
/// NN behavior with no tool-change emission.
#[test]
fn single_tool_layer_emits_no_synthetic_tool_changes() {
    // All entities are tool 0 â€” no tool boundary exists.
    let ordered_entities = vec![
        entity_with_tool(0.0, 0.0, 0, "test-object", 0),
        entity_with_tool(30.0, 0.0, 0, "test-object", 1),
        entity_with_tool(10.0, 0.0, 0, "test-object", 2),
    ];

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);

    let layer_collection = LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities,
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    };

    let plan = plan_with_stages(
        vec![
            stage("Layer::Infill", "com.test.infill"),
            stage(
                "Layer::PathOptimization",
                "com.core.path-optimization-default",
            ),
        ],
        1,
    );
    seed_slice_ir(&mut blackboard, &plan);

    let engine = Arc::new(WasmEngine::new());
    let path_opt_component = load_path_optimization_module(&engine);

    let path_opt_loaded = path_optimization_loaded_module();
    let path_opt_pool = Arc::new(
        build_wasm_instance_pool(
            path_opt_loaded.id(),
            path_opt_loaded.stage(),
            path_opt_loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture pool"),
    );
    let path_opt_module = Arc::new(
        CompiledModuleBuilder::new(path_opt_loaded.id().to_string(), Arc::clone(&path_opt_pool))
            .wasm_component(Some(path_opt_component))
            .build(),
    );

    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let runner = LiveDispatcherWithLayerCollection::with_module(
        layer_collection,
        Arc::new(dispatcher),
        path_opt_module,
    );

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    // Single-tool: no tool_change boundaries should be emitted.
    assert!(
        layers[0].tool_changes.is_empty(),
        "single-tool layer must emit no tool_changes, got {:?}",
        layers[0].tool_changes
    );
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// TEST 3: canonical_or_single_tool_sequences_emit_no_redundant_tool_changes
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-3: A layer already in tool-ascending order [tool0, tool0, tool1, tool1]
/// must emit exactly ONE tool_change at the real boundary (tool0â†’tool1) and
/// no redundant changes.
///
/// This already PASSES with the current module because no tool_changes are
/// emitted at all â€” the module's NN ordering does not consider tool_index,
/// so entities stay in their original order and no synthetic tool_changes
/// are produced.
#[test]
fn canonical_or_single_tool_sequences_emit_no_redundant_tool_changes() {
    // Already in canonical tool-ascending order: [tool0, tool0, tool1, tool1]
    let ordered_entities = vec![
        entity_with_tool(30.0, 0.0, 0, "test-object", 0), // tool 0
        entity_with_tool(0.0, 0.0, 0, "test-object", 1),  // tool 0
        entity_with_tool(10.0, 0.0, 1, "test-object", 2), // tool 1
        entity_with_tool(5.0, 0.0, 1, "test-object", 3),  // tool 1
    ];

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);

    let layer_collection = LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities,
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    };

    let plan = plan_with_stages(
        vec![
            stage("Layer::Infill", "com.test.infill"),
            stage(
                "Layer::PathOptimization",
                "com.core.path-optimization-default",
            ),
        ],
        1,
    );
    seed_slice_ir(&mut blackboard, &plan);

    let engine = Arc::new(WasmEngine::new());
    let path_opt_component = load_path_optimization_module(&engine);

    let path_opt_loaded = path_optimization_loaded_module();
    let path_opt_pool = Arc::new(
        build_wasm_instance_pool(
            path_opt_loaded.id(),
            path_opt_loaded.stage(),
            path_opt_loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture pool"),
    );
    let path_opt_module = Arc::new(
        CompiledModuleBuilder::new(path_opt_loaded.id().to_string(), Arc::clone(&path_opt_pool))
            .wasm_component(Some(path_opt_component))
            .build(),
    );

    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let runner = LiveDispatcherWithLayerCollection::with_module(
        layer_collection,
        Arc::new(dispatcher),
        path_opt_module,
    );

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    // Canonical order: tool_changes should reflect exactly one real boundary
    // (tool0â†’tool1 after entity index 1), no redundant changes.
    assert_eq!(
        layers[0].tool_changes.len(),
        1,
        "canonical sequence must emit exactly 1 tool_change (the real boundary), got {}",
        layers[0].tool_changes.len()
    );
    assert_eq!(
        layers[0].tool_changes[0],
        ToolChange {
            after_entity_index: 1,
            from_tool: 0,
            to_tool: 1
        },
        "only boundary (tool0â†’tool1) should be emitted; no redundant changes"
    );
}

// â”€â”€ Custom runner that pre-stages a LayerCollectionIR â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// LayerStageRunner that pre-stages a LayerCollectionIR (for tests that need
/// a specific entity order / tool_index encoding) and delegates
/// Layer::PathOptimization to the live WasmRuntimeDispatcher.
struct LiveDispatcherWithLayerCollection {
    /// Pre-staged layer collection IR with correctly tool-indexed entities.
    layer_collection: LayerCollectionIR,
    dispatcher: Arc<WasmRuntimeDispatcher>,
    /// Stored CompiledModule with the real wasm_component for path-optimization.
    path_opt_module: Arc<CompiledModule>,
}

impl LiveDispatcherWithLayerCollection {
    fn with_module(
        layer_collection: LayerCollectionIR,
        dispatcher: Arc<WasmRuntimeDispatcher>,
        path_opt_module: Arc<CompiledModule>,
    ) -> Self {
        Self {
            layer_collection,
            dispatcher,
            path_opt_module,
        }
    }
}

impl LayerStageRunner for LiveDispatcherWithLayerCollection {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<LayerStageCommitData, LayerStageError> {
        if stage_id == “Layer::Infill” {
            // Inject the pre-staged layer_collection so the executor commits it to the
            // arena before Layer::PathOptimization runs (bypasses auto-assembly fallback).
            return Ok(LayerStageCommitData {
                layer_collection_output: Some(self.layer_collection.clone()),
                ..Default::default()
            });
        }
        // Delegate PathOptimization to the live WASM dispatcher.
        // input.layer_collection is already populated from the arena (committed above).
        let live = self.path_opt_module.as_live();
        self.dispatcher
            .run_stage(stage_id, layer, &live, input)
    }
}

// â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn load_path_optimization_module(engine: &WasmEngine) -> Arc<slicer_runtime::WasmComponent> {
    let path = PathBuf::from(PATH_OPT_WASM);
    assert!(
        path.exists(),
        "path-optimization-default.wasm missing at {}",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read path-optimization-default.wasm");
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile path-optimization-default.wasm"),
    )
}

fn path_optimization_loaded_module() -> LoadedModule {
    LoadedModuleBuilder::new(
        "com.core.path-optimization-default",
        semver(),
        "Layer::PathOptimization",
        String::new(),
        PathBuf::from("fixtures/com.core.path-optimization-default.wasm"),
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
    .build()
}

fn minimal_mesh(object_id: &str) -> Arc<MeshIR> {
    Arc::new(MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.1,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 0.1,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 0.1,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
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
        ..Default::default()
    })
}

fn plan_with_stages(per_layer_stages: Vec<CompiledStage>, layer_count: usize) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![],
        per_layer_stages,
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(
            (0..layer_count)
                .map(|i| GlobalLayer {
                    index: i as u32,
                    z: 0.2 * (i as f32 + 1.0),
                    active_regions: vec![ActiveRegion {
                        object_id: "test-object".to_string(),
                        region_id: 0,
                        resolved_config: ResolvedConfig::default(),
                        effective_layer_height: 0.2,
                        nonplanar_shell: None,
                        is_catchup_layer: false,
                        catchup_z_bottom: 0.0,
                        tool_index: 0,
                    }],
                    has_nonplanar: false,
                    is_sync_layer: i == 0,
                })
                .collect(),
        ),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    }
}

fn stage(stage_id: &str, module_id: &str) -> CompiledStage {
    CompiledStage {
        stage_id: stage_id.to_string(),
        modules: vec![compiled_module(stage_id, module_id)],
    }
}

fn compiled_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModuleBuilder::new(
        module_id,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
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
    let pool = Arc::new(
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
        instance_pool: Arc::clone(&pool),
        config_view: Arc::new(ConfigView::from_map(HashMap::new())),
        wasm_component: None,
    };
    CompiledModuleBuilder::new(binding.module.id().to_string(), Arc::clone(&pool))
        .config_view(Arc::clone(&binding.config_view))
        .build()
}
