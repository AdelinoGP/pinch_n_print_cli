//! TDD: host entity ordering before path-optimization (packet-18).
//!
//! Covers TASK-152 / TASK-152a / TASK-152d / TASK-152e.

#![allow(missing_docs)]

use crate::common::seed::seed_slice_ir;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_runtime::instance_pool::build_wasm_instance_pool;
use slicer_runtime::manifest::LoadedModuleBuilder;
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
    IndexedTriangleSet, InfillIR, InfillRegion, MeshIR, ObjectConfig, ObjectMesh, Point3,
    Point3WithWidth, ResolvedConfig, SemVer, StageId, Transform3d,
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

// â”€â”€ AC-1: same-object nearest-neighbor ordering â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-1: Given three same-object entities at (0,0), (30,0), (10,0), the
/// live WASM dispatch for Layer::PathOptimization produces the sequence
/// 0.0, 10.0, 30.0 with topo_order 0,1,2.
///
/// Uses path-optimization-default.wasm driven through WasmRuntimeDispatcher
/// via execute_per_layer, with a mock InfillIR stage that seeds sparse_infill
/// paths at x = [0.0, 30.0, 10.0]. The real NN ordering is applied by the
/// host pre-staging fallback (packet-18) before the module dispatch; the test
/// verifies the end-to-end ordered result.
#[test]
fn same_object_nearest_neighbor_ordering_is_applied_before_path_optimization() {
    // Raw infill order: (30,0), (10,0), (0,0) â€” expected NN order: (0,0),(10,0),(30,0).
    let infill = InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "test-object".to_string(),
            region_id: 0,
            sparse_infill: vec![path_at(30.0, 0.0), path_at(10.0, 0.0), path_at(0.0, 0.0)],
            solid_infill: vec![],
            ironing: vec![],
        }],
    };

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);

    // Build ExecutionPlan with Layer::Infill (mock) then Layer::PathOptimization (live).
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

    // Runner that injects the mock infill IR and delegates PathOptimization to
    // WasmRuntimeDispatcher so the test exercises the live dispatch path.
    let engine = Arc::new(WasmEngine::new());
    let path_opt_component = load_path_optimization_module(&engine);

    // Build a proper CompiledModule with the real wasm_component.
    let path_opt_loaded = LoadedModuleBuilder::new(
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
    .build();
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
    let runner =
        LiveDispatcherWithInfill::with_module(infill, Arc::new(dispatcher), path_opt_module);

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    let xs: Vec<f32> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.path.points[0].x)
        .collect();
    assert_eq!(
        xs,
        vec![0.0_f32, 10.0, 30.0],
        "expected NN-ordered x sequence [0.0, 10.0, 30.0], got {xs:?}"
    );
    let topos: Vec<u32> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.topo_order)
        .collect();
    assert_eq!(
        topos,
        vec![0u32, 1, 2],
        "expected topo_order [0, 1, 2], got {topos:?}"
    );
}

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

/// LayerStageRunner that injects a pre-seeded InfillIR for Layer::Infill
/// and delegates Layer::PathOptimization to a live WasmRuntimeDispatcher.
struct LiveDispatcherWithInfill {
    infill: InfillIR,
    dispatcher: Arc<WasmRuntimeDispatcher>,
    /// Stored CompiledModule with the real wasm_component for path-optimization.
    /// Used instead of the stage module when calling dispatcher.run_stage,
    /// because the stage module has wasm_component: None.
    path_opt_module: Arc<CompiledModule>,
}

impl LiveDispatcherWithInfill {
    /// Create a LiveDispatcherWithInfill with the real path-optimization CompiledModule.
    fn with_module(
        infill: InfillIR,
        dispatcher: Arc<WasmRuntimeDispatcher>,
        path_opt_module: Arc<CompiledModule>,
    ) -> Self {
        Self {
            infill,
            dispatcher,
            path_opt_module,
        }
    }
}

impl LayerStageRunner for LiveDispatcherWithInfill {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<LayerStageCommitData, LayerStageError> {
        if stage_id == "Layer::Infill" {
            return Ok(LayerStageCommitData {
                infill_output: Some(self.infill.clone()),
                ..Default::default()
            });
        }
        // Delegate PathOptimization to the live WASM dispatcher.
        // Pass self.path_opt_module (which has the real wasm_component) instead of
        // the stage module (which has wasm_component: None due to compiled_module()).
        let live = self.path_opt_module.as_live();
        self.dispatcher
            .run_stage(stage_id, layer, &live, input)
    }
}

// â”€â”€ AC-2: cross-tool ordering â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-2: Given a mixed-tool layer (tool = region_id) whose raw order is
/// [A1(0,0), A2(0,100), B1(1,0), B2(1,1)], the live WASM dispatch groups by
/// tool_index first, then applies nearest-neighbor within each cluster.
/// Result: tool-0 cluster [A1, A2] then tool-1 cluster [B1, B2] â†’ x = [0.0, 0.0, 1.0, 1.0].
/// Within-cluster NN still applies (A1 before A2, B1 before B2).
#[test]
fn cross_object_ordering_resequences_entities_by_travel_cost() {
    // A1(0,0) A2(0,100) B1(1,0) B2(1,1) â€” raw order is all A then all B.
    // Tool grouping: cluster 0 [A1, A2], cluster 1 [B1, B2].
    // Within cluster 0 (NN from 0,0): A1â†’A2 (both in same cluster).
    // Within cluster 1 (NN from 0,0): B1â†’B2 (B1 nearest to 0,0, B2 nearest to B1).
    let infill = InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![
            InfillRegion {
                object_id: "test-object".to_string(),
                region_id: 0,
                sparse_infill: vec![path_at(0.0, 0.0), path_at(0.0, 100.0)],
                solid_infill: vec![],
                ironing: vec![],
            },
            InfillRegion {
                object_id: "test-object".to_string(),
                region_id: 1,
                sparse_infill: vec![path_at(1.0, 0.0), path_at(1.0, 1.0)],
                solid_infill: vec![],
                ironing: vec![],
            },
        ],
    };

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
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

    let path_opt_loaded = LoadedModuleBuilder::new(
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
    .build();
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
    let runner =
        LiveDispatcherWithInfill::with_module(infill, Arc::new(dispatcher), path_opt_module);

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    let xs: Vec<f32> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.path.points[0].x)
        .collect();
    // Grouped: [A1(0,0), A2(0,100), B1(1,0), B2(1,1)]: x = [0.0, 0.0, 1.0, 1.0]
    assert_eq!(
        xs,
        vec![0.0_f32, 0.0, 1.0, 1.0],
        "expected grouped-tool ordering [0.0, 0.0, 1.0, 1.0], got {xs:?}"
    );
}

// â”€â”€ AC-3: bridge-sensitive priority â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-3: When a BridgeInfill and a SparseInfill entity are equidistant (within
/// 0.001 mm) from the current position, BridgeInfill appears first in the live
/// WASM dispatch result.
#[test]
fn bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill() {
    // Both at exactly (5.0, 0.0) â€” equidistant from start (0,0). Bridge wins.
    let infill = InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "obj".to_string(),
            region_id: 0,
            sparse_infill: vec![
                path_at_explicit(5.0, 0.0, ExtrusionRole::SparseInfill),
                path_at_explicit(5.0, 0.0, ExtrusionRole::BridgeInfill),
            ],
            solid_infill: vec![],
            ironing: vec![],
        }],
    };

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
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

    let path_opt_loaded = LoadedModuleBuilder::new(
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
    .build();
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
    let runner =
        LiveDispatcherWithInfill::with_module(infill, Arc::new(dispatcher), path_opt_module);

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    // Packet-61 unified all four infill variants
    // (BottomSolidInfill | TopSolidInfill | SparseInfill | BridgeInfill) into
    // a single role-priority group (= 4). Inside one group, equidistant
    // candidates fall back to deterministic insertion order rather than the
    // older explicit "bridge wins" tie-break. The fixture inserts
    // `SparseInfill` first, then `BridgeInfill`, so SparseInfill leads here.
    assert_eq!(
        layers[0].ordered_entities[0].role,
        ExtrusionRole::SparseInfill,
        "with role-priority unified, insertion order is preserved on tie"
    );
    assert_eq!(
        layers[0].ordered_entities[1].role,
        ExtrusionRole::BridgeInfill,
        "BridgeInfill comes second when inserted second at equal distance"
    );
}

// â”€â”€ AC-4: determinism across repeated runs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-4: Running the live dispatch path twice on identical input produces a
/// byte-identical ordered entity sequence.
#[test]
fn path_ordering_is_deterministic_across_repeated_runs() {
    fn make_infill() -> InfillIR {
        InfillIR {
            schema_version: semver(),
            global_layer_index: 0,
            regions: vec![InfillRegion {
                object_id: "test-object".to_string(),
                region_id: 0,
                sparse_infill: vec![path_at(30.0, 0.0), path_at(0.0, 0.0), path_at(15.0, 0.0)],
                solid_infill: vec![],
                ironing: vec![],
            }],
        }
    }

    fn run() -> Vec<slicer_ir::LayerCollectionIR> {
        let infill = make_infill();
        let mesh = minimal_mesh("test-object");
        let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
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

        let path_opt_loaded = LoadedModuleBuilder::new(
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
        .build();
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
            CompiledModuleBuilder::new(
                path_opt_loaded.id().to_string(),
                Arc::clone(&path_opt_pool),
            )
            .wasm_component(Some(path_opt_component))
            .build(),
        );

        let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
        let runner =
            LiveDispatcherWithInfill::with_module(infill, Arc::new(dispatcher), path_opt_module);

        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed")
    }

    let layers1 = run();
    let layers2 = run();

    // Byte-identical assertion at the ordered_entities slice level.
    assert_eq!(
        &layers1[0].ordered_entities, &layers2[0].ordered_entities,
        "ordered_entities must be byte-identical across runs"
    );
}

// â”€â”€ AC-NEG: single / already-optimal sequence unchanged â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Negative: An already-optimal sequence (0.0, 10.0, 30.0) is returned
/// unchanged through live WASM dispatch â€” no reordering needed.
#[test]
fn single_or_already_optimal_sequence_is_left_unchanged() {
    // Already-optimal: (0,0),(10,0),(30,0) â€” NN order from origin is this order.
    let infill = InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "test-object".to_string(),
            region_id: 0,
            sparse_infill: vec![path_at(0.0, 0.0), path_at(10.0, 0.0), path_at(30.0, 0.0)],
            solid_infill: vec![],
            ironing: vec![],
        }],
    };

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
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

    let path_opt_loaded = LoadedModuleBuilder::new(
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
    .build();
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
    let runner =
        LiveDispatcherWithInfill::with_module(infill, Arc::new(dispatcher), path_opt_module);

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    let xs: Vec<f32> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.path.points[0].x)
        .collect();
    assert_eq!(
        xs,
        vec![0.0_f32, 10.0, 30.0],
        "already-optimal sequence must be unchanged: {xs:?}"
    );
}

// â”€â”€ AC-6: no module proposal leaves raw assembled order â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-6 (Step 5): When the path-optimization module returns Success without
/// emitting a `set-entity-order` proposal, the host must NOT apply its own
/// fallback ordering. The raw assembled order [30.0, 0.0, 10.0] must be
/// preserved.
///
/// With the host fallback removed,
/// entities now pass through in raw assembled order unless the module
/// emits an explicit proposal.
#[test]
fn no_module_proposal_leaves_raw_assembled_order() {
    // Raw start-x: [30.0, 0.0, 10.0] â€” no NN reordering should occur
    // if the module emits no proposal.
    let infill = InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "test-object".to_string(),
            region_id: 0,
            sparse_infill: vec![path_at(30.0, 0.0), path_at(0.0, 0.0), path_at(10.0, 0.0)],
            solid_infill: vec![],
            ironing: vec![],
        }],
    };

    let mesh = minimal_mesh("test-object");
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    let plan = plan_with_stages(
        vec![
            stage("Layer::Infill", "com.test.infill"),
            stage("Layer::PathOptimization", "com.test.path-opt"),
        ],
        1,
    );
    seed_slice_ir(&mut blackboard, &plan);

    // Stub runner: injects infill, returns Success for PathOptimization
    // without ever calling set_entity_order.
    let runner = NoProposalStubRunner { infill };

    let layers =
        execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    let xs: Vec<f32> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.path.points[0].x)
        .collect();
    assert_eq!(
        xs,
        vec![30.0_f32, 0.0, 10.0],
        "expected raw assembled order [30.0, 0.0, 10.0], got {xs:?}"
    );
}

/// Stub LayerStageRunner that injects infill IR for Layer::Infill and
/// returns Success for Layer::PathOptimization without any set_entity_order call.
struct NoProposalStubRunner {
    infill: InfillIR,
}

impl LayerStageRunner for NoProposalStubRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<LayerStageCommitData, LayerStageError> {
        if stage_id == “Layer::Infill” {
            return Ok(LayerStageCommitData {
                infill_output: Some(self.infill.clone()),
                ..Default::default()
            });
        }
        // No proposal â€” no set_entity_order call, no layer_collection_proposal.
        Ok(LayerStageCommitData::default())
    }
}

// â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn path_at(x: f32, y: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![pt(x, y)],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn path_at_explicit(x: f32, y: f32, role: ExtrusionRole) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![pt(x, y)],
        role,
        speed_factor: 1.0,
    }
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
