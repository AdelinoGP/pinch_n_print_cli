//! TDD tests for the per-layer parallel executor (TASK-031).
//!
//! These tests define the contract for `execute_per_layer` which uses rayon to
//! process layers in parallel, running all per_layer_stages sequentially within
//! each layer. Each layer gets its own `LayerArena` for intermediate IR storage.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use slicer_host::{
    build_wasm_instance_pool, execute_per_layer, Blackboard, CompiledModule, CompiledStage,
    ConfigSchema, ExecutionModuleBinding, ExecutionPlan, IrAccessMask, LayerArena,
    LayerExecutionError, LayerStageError, LayerStageOutput, LayerStageRunner, WasmArtifactMetadata,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, MeshIR, ObjectMesh, Point3,
    ResolvedConfig, SemVer, StageId, Transform3d,
};

// ============================================================================
// Test 1: Layers processed in parallel with deterministic stage ordering
// ============================================================================

#[test]
fn layer_executor_processes_layers_in_parallel_with_deterministic_stage_ordering() {
    // Arrange: Create a plan with 4 layers and 2 stages with one module each
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 4);
    let plan = execution_plan_fixture(
        vec![
            compiled_stage("Layer::Perimeters", &["com.example.perimeters"]),
            compiled_stage("Layer::Infill", &["com.example.infill"]),
        ],
        4,
    );

    let runner = ScriptedRunner::new()
        .with_stage_sequence(vec![
            // Each layer should see: Perimeters then Infill (deterministic stage order)
            ("Layer::Perimeters", "com.example.perimeters"),
            ("Layer::Infill", "com.example.infill"),
        ])
        .with_default_success();

    // Act
    let result = execute_per_layer(&plan, &blackboard, &runner);

    // Assert: Execution should succeed
    let layer_outputs = result.expect("per-layer executor should produce outputs for all layers");
    assert_eq!(layer_outputs.len(), 4);

    // Verify that within each layer, stages were called in order (Perimeters before Infill)
    let invocations = runner.invocations();
    let layer_stage_orders = group_by_layer(&invocations);

    for (layer_index, stages) in layer_stage_orders.iter() {
        let stage_ids: Vec<&str> = stages.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(
            stage_ids,
            vec!["Layer::Perimeters", "Layer::Infill"],
            "layer {layer_index} should process stages in deterministic order"
        );
    }

    // Verify all 4 layers were processed
    let layer_indices: Vec<u32> = layer_stage_orders.keys().copied().collect();
    assert!(layer_indices.contains(&0));
    assert!(layer_indices.contains(&1));
    assert!(layer_indices.contains(&2));
    assert!(layer_indices.contains(&3));
}

// ============================================================================
// Test 2: Modules run in topological order within each stage
// ============================================================================

#[test]
fn layer_executor_runs_modules_in_topological_order_within_each_stage() {
    // Arrange: Create a plan with 2 layers and 1 stage with 3 modules in topo order
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 2);
    let plan = execution_plan_fixture(
        vec![compiled_stage(
            "Layer::Perimeters",
            &[
                "com.example.perimeters-a",
                "com.example.perimeters-b",
                "com.example.perimeters-c",
            ],
        )],
        2,
    );

    let runner = ScriptedRunner::new()
        .with_stage_sequence(vec![
            ("Layer::Perimeters", "com.example.perimeters-a"),
            ("Layer::Perimeters", "com.example.perimeters-b"),
            ("Layer::Perimeters", "com.example.perimeters-c"),
        ])
        .with_default_success();

    // Act
    let result = execute_per_layer(&plan, &blackboard, &runner);

    // Assert
    result.expect("per-layer executor should succeed with topological module ordering");

    // Verify that within each layer and stage, modules were called in order
    let invocations = runner.invocations();
    let layer_stage_orders = group_by_layer(&invocations);

    for (layer_index, stage_modules) in layer_stage_orders.iter() {
        let module_ids: Vec<&str> = stage_modules.iter().map(|(_, m)| m.as_str()).collect();
        assert_eq!(
            module_ids,
            vec![
                "com.example.perimeters-a",
                "com.example.perimeters-b",
                "com.example.perimeters-c"
            ],
            "layer {layer_index} should process modules in topological order"
        );
    }
}

// ============================================================================
// Test 3: Isolated LayerArena per layer
// ============================================================================

#[test]
fn layer_executor_provides_isolated_layer_arena_per_layer() {
    // Arrange: Create a plan with 3 layers; runner will record arena pointer addresses
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 3);
    let plan = execution_plan_fixture(
        vec![compiled_stage(
            "Layer::Perimeters",
            &["com.example.perimeters"],
        )],
        3,
    );

    let runner = ArenaIsolationRunner::new();

    // Act
    let result = execute_per_layer(&plan, &blackboard, &runner);

    // Assert
    result.expect("per-layer executor should succeed with isolated arenas");

    // Each layer should have gotten a distinct arena (tracked by arena identity)
    let arena_ids = runner.arena_identities();
    assert_eq!(
        arena_ids.len(),
        3,
        "should have 3 distinct arena identities"
    );

    // All arena identities should be unique (each layer gets its own)
    let unique_ids: std::collections::HashSet<_> = arena_ids.iter().collect();
    assert_eq!(
        unique_ids.len(),
        3,
        "each layer should have its own isolated arena"
    );
}

// ============================================================================
// Test 4: Commits layer outputs to blackboard slots
// ============================================================================

#[test]
fn layer_executor_commits_layer_outputs_to_blackboard_slots() {
    // Arrange: Create a plan with 3 layers
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 3);
    let plan = execution_plan_fixture(
        vec![compiled_stage(
            "Layer::Perimeters",
            &["com.example.perimeters"],
        )],
        3,
    );

    let runner = ScriptedRunner::new().with_default_success();

    // Act
    let result = execute_per_layer(&plan, &blackboard, &runner);

    // Assert: Should return Vec<LayerCollectionIR> with correct layer indices
    let layer_outputs = result.expect("per-layer executor should produce write-once slot outputs");
    assert_eq!(layer_outputs.len(), 3);

    // Verify each output has the correct layer index
    for (i, output) in layer_outputs.iter().enumerate() {
        assert_eq!(
            output.global_layer_index, i as u32,
            "layer output {i} should have correct global_layer_index"
        );
    }
}

// ============================================================================
// Test 5: Propagates fatal module error and aborts layer
// ============================================================================

#[test]
fn layer_executor_propagates_fatal_module_error_and_aborts_layer() {
    // Arrange: Create a plan with 2 layers; second layer will fail fatally
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 2);
    let plan = execution_plan_fixture(
        vec![
            compiled_stage("Layer::Perimeters", &["com.example.perimeters"]),
            compiled_stage("Layer::Infill", &["com.example.infill"]),
        ],
        2,
    );

    let runner = ScriptedRunner::new()
        .with_fatal_error(
            1, // layer index
            "Layer::Perimeters",
            "com.example.perimeters",
            "simulated fatal error in layer 1",
        )
        .with_default_success();

    // Act
    let result = execute_per_layer(&plan, &blackboard, &runner);

    // Assert: Should return FatalLayer error
    assert_eq!(
        result,
        Err(LayerExecutionError::FatalLayer {
            layer_index: 1,
            stage_id: String::from("Layer::Perimeters"),
            module_id: String::from("com.example.perimeters"),
            message: String::from("fatal layer stage module failure in Layer::Perimeters for com.example.perimeters: simulated fatal error in layer 1"),
        })
    );
}

// ============================================================================
// Test 6: Continues on non-fatal module error
// ============================================================================

#[test]
fn layer_executor_continues_on_non_fatal_module_error() {
    // Arrange: Create a plan with 2 layers; first module in each layer returns non-fatal error
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 2);
    let plan = execution_plan_fixture(
        vec![
            compiled_stage("Layer::Perimeters", &["com.example.perimeters"]),
            compiled_stage("Layer::Infill", &["com.example.infill"]),
        ],
        2,
    );

    let runner = ScriptedRunner::new()
        .with_non_fatal_error(0, "Layer::Perimeters", "com.example.perimeters")
        .with_non_fatal_error(1, "Layer::Perimeters", "com.example.perimeters")
        .with_default_success();

    // Act
    let result = execute_per_layer(&plan, &blackboard, &runner);

    // Assert: Should succeed despite non-fatal errors
    let layer_outputs = result.expect("per-layer executor should continue on non-fatal errors");
    assert_eq!(layer_outputs.len(), 2);

    // Verify both layers were fully processed (Infill should have run after non-fatal Perimeters)
    let invocations = runner.invocations();
    let layer_stage_orders = group_by_layer(&invocations);

    for (layer_index, stages) in layer_stage_orders.iter() {
        let stage_ids: Vec<&str> = stages.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(
            stage_ids,
            vec!["Layer::Perimeters", "Layer::Infill"],
            "layer {layer_index} should complete all stages despite non-fatal error"
        );
    }
}

// ============================================================================
// Test 7: Drains all layer outputs after parallel completion
// ============================================================================

#[test]
fn layer_executor_drains_all_layer_outputs_after_parallel_completion() {
    // Arrange: Create a plan with 5 layers to verify ordering in final Vec
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 5);
    let plan = execution_plan_fixture(
        vec![compiled_stage(
            "Layer::Perimeters",
            &["com.example.perimeters"],
        )],
        5,
    );

    let runner = ScriptedRunner::new().with_default_success();

    // Act
    let result = execute_per_layer(&plan, &blackboard, &runner);

    // Assert: Should return Vec with all 5 layers in correct order
    let layer_outputs = result.expect("per-layer executor should drain all outputs");
    assert_eq!(layer_outputs.len(), 5);

    // Verify ordering: layer outputs should be in layer index order
    for (i, output) in layer_outputs.iter().enumerate() {
        assert_eq!(
            output.global_layer_index, i as u32,
            "drained output at position {i} should have global_layer_index {i}"
        );
    }
}

// ============================================================================
// Scripted Runner Mock
// ============================================================================

#[derive(Debug)]
struct ScriptedRunner {
    /// Expected stage sequence for ordering verification
    expected_sequence: Vec<(String, String)>,
    /// Fatal errors by (layer_index, stage_id, module_id)
    fatal_errors: HashMap<(u32, String, String), String>,
    /// Non-fatal errors by (layer_index, stage_id, module_id)
    non_fatal_errors: HashMap<(u32, String, String), String>,
    /// Recorded invocations: (layer_index, stage_id, module_id)
    invocations: Mutex<Vec<(u32, String, String)>>,
    /// Invocation counter for generating unique arena IDs
    invocation_counter: AtomicU32,
}

impl ScriptedRunner {
    fn new() -> Self {
        Self {
            expected_sequence: Vec::new(),
            fatal_errors: HashMap::new(),
            non_fatal_errors: HashMap::new(),
            invocations: Mutex::new(Vec::new()),
            invocation_counter: AtomicU32::new(0),
        }
    }

    fn with_stage_sequence(mut self, sequence: Vec<(&str, &str)>) -> Self {
        self.expected_sequence = sequence
            .into_iter()
            .map(|(s, m)| (String::from(s), String::from(m)))
            .collect();
        self
    }

    fn with_fatal_error(
        mut self,
        layer_index: u32,
        stage_id: &str,
        module_id: &str,
        message: &str,
    ) -> Self {
        self.fatal_errors.insert(
            (layer_index, String::from(stage_id), String::from(module_id)),
            String::from(message),
        );
        self
    }

    fn with_non_fatal_error(mut self, layer_index: u32, stage_id: &str, module_id: &str) -> Self {
        self.non_fatal_errors.insert(
            (layer_index, String::from(stage_id), String::from(module_id)),
            String::from("non-fatal error"),
        );
        self
    }

    fn with_default_success(self) -> Self {
        self
    }

    fn invocations(&self) -> Vec<(u32, String, String)> {
        self.invocations.lock().unwrap().clone()
    }
}

impl LayerStageRunner for ScriptedRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        _arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        let key = (layer.index, stage_id.clone(), module.module_id.clone());

        // Record invocation
        self.invocations.lock().unwrap().push(key.clone());
        self.invocation_counter.fetch_add(1, Ordering::SeqCst);

        // Check for fatal error
        if let Some(message) = self.fatal_errors.get(&key) {
            return Err(LayerStageError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: message.clone(),
            });
        }

        // Check for non-fatal error
        if let Some(message) = self.non_fatal_errors.get(&key) {
            return Ok((
                LayerStageOutput::NonFatalError {
                    message: message.clone(),
                },
                Vec::new(),
                Vec::new(),
            ));
        }

        Ok((LayerStageOutput::Success, Vec::new(), Vec::new()))
    }
}

// ============================================================================
// Arena Isolation Runner
// ============================================================================

#[derive(Debug)]
struct ArenaIsolationRunner {
    /// Recorded arena identities by layer index
    arena_identities: Mutex<HashMap<u32, usize>>,
}

impl ArenaIsolationRunner {
    fn new() -> Self {
        Self {
            arena_identities: Mutex::new(HashMap::new()),
        }
    }

    fn arena_identities(&self) -> Vec<usize> {
        self.arena_identities
            .lock()
            .unwrap()
            .values()
            .copied()
            .collect()
    }
}

impl LayerStageRunner for ArenaIsolationRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        layer: &GlobalLayer,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        // Record the arena's address as an identity marker
        let arena_ptr = arena as *mut LayerArena as usize;
        self.arena_identities
            .lock()
            .unwrap()
            .insert(layer.index, arena_ptr);

        Ok((LayerStageOutput::Success, Vec::new(), Vec::new()))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn group_by_layer(invocations: &[(u32, String, String)]) -> HashMap<u32, Vec<(String, String)>> {
    let mut result: HashMap<u32, Vec<(String, String)>> = HashMap::new();
    for (layer_index, stage_id, module_id) in invocations {
        result
            .entry(*layer_index)
            .or_default()
            .push((stage_id.clone(), module_id.clone()));
    }
    result
}

fn execution_plan_fixture(
    per_layer_stages: Vec<CompiledStage>,
    layer_count: usize,
) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages,
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(
            (0..layer_count)
                .map(|i| GlobalLayer {
                    index: i as u32,
                    z: 0.2 * (i as f32 + 1.0),
                    active_regions: vec![ActiveRegion {
                        object_id: String::from("test-object"),
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
    let loaded_module = loaded_module(module_id, stage_id);
    let instance_pool = Arc::new(
        build_wasm_instance_pool(
            &loaded_module,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );

    let binding = ExecutionModuleBinding {
        module: loaded_module,
        instance_pool,
        config_view: Arc::new(ConfigView::from_map(HashMap::from([(
            String::from("fixture.enabled"),
            ConfigValue::Bool(true),
        )]))),
        wasm_component: None,
    };

    CompiledModule {
        module_id: binding.module.id.clone(),
        instance_pool: Arc::clone(&binding.instance_pool),
        ir_read_mask: IrAccessMask {
            paths: binding.module.ir_reads.clone(),
        },
        ir_write_mask: IrAccessMask {
            paths: binding.module.ir_writes.clone(),
        },
        config_view: Arc::clone(&binding.config_view),
        claims: Vec::new(),
        wasm_component: None,
        requires_modules: Vec::new(),
    }
}

fn loaded_module(id: &str, stage: &str) -> slicer_host::LoadedModule {
    slicer_host::LoadedModule {
        id: String::from(id),
        version: semver(1, 0, 0),
        stage: String::from(stage),
        wit_world: String::from("slicer:world-layer@1.0.0"),
        ir_reads: match stage {
            "Layer::Perimeters" => vec![String::from("SliceIR.regions")],
            "Layer::Infill" => vec![
                String::from("SliceIR.regions"),
                String::from("PerimeterIR.wall_loops"),
            ],
            _ => Vec::new(),
        },
        ir_writes: match stage {
            "Layer::Perimeters" => vec![String::from("PerimeterIR.wall_loops")],
            "Layer::Infill" => vec![String::from("InfillIR.paths")],
            _ => Vec::new(),
        },
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: ConfigSchema::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: true,
        wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
        placeholder_wasm: false,
    }
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("test-object"),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: identity4(),
            },
            config: slicer_ir::ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
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
    }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

// ============================================================================
// ordered_entities assembly from committed arena slots
// ============================================================================

/// Runner that stages pre-made IR into the arena so the executor can assemble
/// `LayerCollectionIR.ordered_entities` from it.
struct StagingRunner {
    perimeter: Mutex<Option<slicer_ir::PerimeterIR>>,
    infill: Mutex<Option<slicer_ir::InfillIR>>,
    support: Mutex<Option<slicer_ir::SupportIR>>,
}

impl StagingRunner {
    fn new(
        p: Option<slicer_ir::PerimeterIR>,
        i: Option<slicer_ir::InfillIR>,
        s: Option<slicer_ir::SupportIR>,
    ) -> Self {
        Self {
            perimeter: Mutex::new(p),
            infill: Mutex::new(i),
            support: Mutex::new(s),
        }
    }
}

impl LayerStageRunner for StagingRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        if let Some(p) = self.perimeter.lock().unwrap().take() {
            arena.set_perimeter(p).unwrap();
        }
        if let Some(i) = self.infill.lock().unwrap().take() {
            arena.set_infill(i).unwrap();
        }
        if let Some(s) = self.support.lock().unwrap().take() {
            arena.set_support(s).unwrap();
        }
        Ok((LayerStageOutput::Success, Vec::new(), Vec::new()))
    }
}

fn mk_path(x: f32) -> slicer_ir::ExtrusionPath3D {
    slicer_ir::ExtrusionPath3D {
        points: vec![slicer_ir::Point3WithWidth {
            x,
            y: 0.0,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        }],
        role: slicer_ir::ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    }
}

fn mk_path_role(x: f32, role: slicer_ir::ExtrusionRole) -> slicer_ir::ExtrusionPath3D {
    slicer_ir::ExtrusionPath3D {
        points: vec![slicer_ir::Point3WithWidth {
            x,
            y: 0.0,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        }],
        role,
        speed_factor: 1.0,
    }
}

fn perim_ir_two_regions() -> slicer_ir::PerimeterIR {
    slicer_ir::PerimeterIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        regions: vec![
            slicer_ir::PerimeterRegion {
                object_id: "obj-A".into(),
                region_id: 1,
                walls: vec![slicer_ir::WallLoop {
                    perimeter_index: 0,
                    loop_type: slicer_ir::LoopType::Outer,
                    path: mk_path(1.0),
                    width_profile: slicer_ir::WidthProfile { widths: vec![0.4] },
                    feature_flags: Vec::new(),
                    boundary_type: slicer_ir::WallBoundaryType::Interior,
                }],
                infill_areas: Vec::new(),
                seam_candidates: Vec::new(),
                resolved_seam: None,
            },
            slicer_ir::PerimeterRegion {
                object_id: "obj-B".into(),
                region_id: 2,
                walls: vec![slicer_ir::WallLoop {
                    perimeter_index: 0,
                    loop_type: slicer_ir::LoopType::Inner,
                    path: mk_path_role(2.0, slicer_ir::ExtrusionRole::InnerWall),
                    width_profile: slicer_ir::WidthProfile { widths: vec![0.4] },
                    feature_flags: Vec::new(),
                    boundary_type: slicer_ir::WallBoundaryType::Interior,
                }],
                infill_areas: Vec::new(),
                seam_candidates: Vec::new(),
                resolved_seam: None,
            },
        ],
    }
}

fn infill_ir_two_regions() -> slicer_ir::InfillIR {
    slicer_ir::InfillIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        regions: vec![
            slicer_ir::InfillRegion {
                object_id: "obj-A".into(),
                region_id: 1,
                sparse_infill: vec![mk_path_role(10.0, slicer_ir::ExtrusionRole::SparseInfill)],
                solid_infill: vec![mk_path_role(11.0, slicer_ir::ExtrusionRole::TopSolidInfill)],
                ironing: Vec::new(),
            },
            slicer_ir::InfillRegion {
                object_id: "obj-B".into(),
                region_id: 2,
                sparse_infill: vec![mk_path_role(20.0, slicer_ir::ExtrusionRole::SparseInfill)],
                solid_infill: Vec::new(),
                ironing: Vec::new(),
            },
        ],
    }
}

fn support_ir_simple() -> slicer_ir::SupportIR {
    slicer_ir::SupportIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        support_paths: vec![mk_path_role(
            100.0,
            slicer_ir::ExtrusionRole::SupportMaterial,
        )],
        interface_paths: vec![mk_path_role(
            101.0,
            slicer_ir::ExtrusionRole::SupportInterface,
        )],
        raft_paths: Vec::new(),
        ironing_paths: Vec::new(),
    }
}

#[test]
fn ordered_entities_assembled_with_preserved_region_identity() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    let plan = execution_plan_fixture(
        vec![compiled_stage("Layer::Perimeters", &["com.example.stage"])],
        1,
    );
    let runner = StagingRunner::new(
        Some(perim_ir_two_regions()),
        Some(infill_ir_two_regions()),
        Some(support_ir_simple()),
    );

    let layers = execute_per_layer(&plan, &blackboard, &runner).expect("layer exec");
    assert_eq!(layers.len(), 1);
    let l = &layers[0];
    // 2 walls + (1 sparse + 1 solid) + 1 sparse + 1 support + 1 interface = 7
    assert_eq!(l.ordered_entities.len(), 7, "all committed paths drained");

    let keys: Vec<(String, u64)> = l
        .ordered_entities
        .iter()
        .map(|e| (e.region_key.object_id.clone(), e.region_key.region_id))
        .collect();
    // Perimeter region order, then infill region order, then support (flat: "", 0).
    assert_eq!(
        keys,
        vec![
            ("obj-A".into(), 1), // perim region A wall
            ("obj-B".into(), 2), // perim region B wall
            ("obj-A".into(), 1), // infill A sparse
            ("obj-A".into(), 1), // infill A solid
            ("obj-B".into(), 2), // infill B sparse
            ("".into(), 0),      // support
            ("".into(), 0),      // interface
        ]
    );
    // topo_order is 0..N
    for (i, e) in l.ordered_entities.iter().enumerate() {
        assert_eq!(e.topo_order, i as u32, "topo_order is emit position");
        assert_eq!(e.region_key.global_layer_index, 0);
    }
}

#[test]
fn ordered_entities_empty_when_arena_has_no_committed_content() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    let plan = execution_plan_fixture(
        vec![compiled_stage("Layer::Perimeters", &["com.example.stage"])],
        1,
    );
    let runner = StagingRunner::new(None, None, None);
    let layers = execute_per_layer(&plan, &blackboard, &runner).expect("layer exec");
    assert_eq!(layers.len(), 1);
    assert!(
        layers[0].ordered_entities.is_empty(),
        "empty-input -> empty ordered_entities"
    );
}

#[test]
fn ordered_entities_assembly_is_deterministic_across_repeated_runs() {
    let mesh = Arc::new(mesh_fixture());
    let plan = execution_plan_fixture(
        vec![compiled_stage("Layer::Perimeters", &["com.example.stage"])],
        1,
    );

    let mut results = Vec::new();
    for _ in 0..3 {
        let blackboard = Blackboard::new(Arc::clone(&mesh), 1);
        let runner = StagingRunner::new(
            Some(perim_ir_two_regions()),
            Some(infill_ir_two_regions()),
            Some(support_ir_simple()),
        );
        let layers = execute_per_layer(&plan, &blackboard, &runner).expect("layer exec");
        results.push(layers);
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

// ============================================================================
// TASK-134: Catch-up metadata regression guards
// ============================================================================

/// AC-4 / TASK-134 regression guard: a catch-up ActiveRegion passing through
/// all nine per-layer stages keeps is_catchup_layer=true and catchup_z_bottom=B
/// unchanged on the source layer surface seen by every stage.
///
/// The nine per-layer stages are (docs/04 §Full Lifecycle):
///   Layer::Slice → Layer::SlicePostProcess → Layer::Perimeters →
///   Layer::PerimetersPostProcess → Layer::Infill → Layer::InfillPostProcess →
///   Layer::Support → Layer::SupportPostProcess → Layer::PathOptimization
///
/// Catch-up metadata is computed once in PrePass::LayerPlanning and must
/// never be recomputed in Tier 2. This test proves the metadata is not
/// mutated as the layer surface passes through each stage runner call.
#[test]
fn catchup_metadata_remains_stable_across_all_per_layer_stages() {
    // Arrange: build a catch-up layer at Z=0.6 where Object B (0.3mm/layer)
    // catches up to Object A (0.2mm/layer) on the 0.6mm sync plane.
    let catchup_z_bottom = 0.3_f32;
    let effective_layer_height = 0.3_f32;
    // Use "test-object" to match the mesh_fixture() object ID.
    let layer = GlobalLayer {
        index: 7,
        z: 0.6,
        active_regions: vec![ActiveRegion {
            object_id: "test-object".to_string(),
            region_id: 0,
            resolved_config: ResolvedConfig::default(),
            effective_layer_height,
            nonplanar_shell: None,
            is_catchup_layer: true,
            catchup_z_bottom,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // Use execution_plan_fixture but with a custom global_layers that has the
    // catch-up layer.  We build the plan manually using the fixture's stage
    // structure to stay compatible with both HEAD and post-module_region_index
    // builds: we use struct-literal only for the fields that are pub.
    let per_layer_stages = vec![
        compiled_stage(
            "Layer::SlicePostProcess",
            &["com.example.slice-postprocess"],
        ),
        compiled_stage("Layer::Perimeters", &["com.example.perimeters"]),
        compiled_stage(
            "Layer::PerimetersPostProcess",
            &["com.example.perimeters-postprocess"],
        ),
        compiled_stage("Layer::Infill", &["com.example.infill"]),
        compiled_stage(
            "Layer::InfillPostProcess",
            &["com.example.infill-postprocess"],
        ),
        compiled_stage("Layer::Support", &["com.example.support"]),
        compiled_stage(
            "Layer::SupportPostProcess",
            &["com.example.support-postprocess"],
        ),
        compiled_stage(
            "Layer::PathOptimization",
            &["com.example.path-optimization"],
        ),
    ];

    // Build ExecutionPlan using the same pattern as execution_plan_fixture
    // but with a catch-up global layer.  This approach works at HEAD; if
    // module_region_index is later added as pub(crate), a Default-based
    // construction or builder must be used instead.
    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages,
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![layer]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(Arc::clone(&mesh), 1);

    // RecordingRunner captures the active_regions surface at each stage call.
    let runner = CatchupMetadataRecordingRunner::new();
    let result = execute_per_layer(&plan, &blackboard, &runner);
    if let Err(e) = &result {
        panic!("execution failed: {e:?}");
    }

    // Assert: all eight module-driven stages saw identical catch-up metadata on
    // the source GlobalLayer.active_regions surface.
    //
    // NOTE: we assert on the source GlobalLayer (layer.z=0.6, is_catchup=true,
    // catchup_z_bottom=0.3) NOT on downstream IR types.  PerimeterIR, InfillIR,
    // SupportIR, and LayerCollectionIR do not define is_catchup_layer or
    // catchup_z_bottom — those fields exist only on GlobalLayer.active_regions.
    // SlicedRegion.effective_layer_height is separately tested in
    // layer_slice_tdd.rs.
    //
    // The nine per-layer stages per docs/04 §Full Lifecycle are:
    //   Layer::Slice → SlicePostProcess → Perimeters → PerimetersPostProcess →
    //   Infill → InfillPostProcess → Support → SupportPostProcess → PathOptimization
    // Layer::Slice is host-built-in and runs before the module loop via
    // execute_layer_slice; it does NOT call run_stage().  All eight module-driven
    // stages DO call run_stage() and are recorded here.
    let recordings = runner.recordings();
    assert_eq!(
        recordings.len(),
        8,
        "all eight module-driven per-layer stages should be invoked"
    );
    for (i, rec) in recordings.iter().enumerate() {
        let stage_name = format!("stage[{i}]");
        assert!(
            rec.is_catchup_layer,
            "{stage_name}: is_catchup_layer must remain true (was set at PrePass)"
        );
        assert_eq!(
            rec.catchup_z_bottom, catchup_z_bottom,
            "{stage_name}: catchup_z_bottom must remain B=0.3 unchanged"
        );
    }
}

/// RecordingRunner that captures catch-up metadata from GlobalLayer.active_regions
/// at each stage invocation.
struct CatchupMetadataRecordingRunner {
    recordings: Mutex<Vec<CatchupSnapshot>>,
}

#[derive(Debug, Clone, Copy)]
struct CatchupSnapshot {
    is_catchup_layer: bool,
    catchup_z_bottom: f32,
}

impl CatchupMetadataRecordingRunner {
    fn new() -> Self {
        Self {
            recordings: Mutex::new(Vec::new()),
        }
    }

    fn recordings(&self) -> Vec<CatchupSnapshot> {
        self.recordings.lock().unwrap().clone()
    }
}

impl LayerStageRunner for CatchupMetadataRecordingRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        layer: &GlobalLayer,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        // Record catch-up metadata from the source active_regions surface.
        // We take the first region as the canary; if there are multiple regions
        // all must carry the same catch-up flags per the pre-pass contract.
        if let Some(region) = layer.active_regions.first() {
            self.recordings.lock().unwrap().push(CatchupSnapshot {
                is_catchup_layer: region.is_catchup_layer,
                catchup_z_bottom: region.catchup_z_bottom,
            });
        }
        Ok((LayerStageOutput::Success, Vec::new(), Vec::new()))
    }
}
