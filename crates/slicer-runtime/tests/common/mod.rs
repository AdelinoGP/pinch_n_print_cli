#![allow(dead_code)]

// These shared fixtures and the engine/component cache helpers (`ctx_with_mesh`,
// `mesh_fixture`, `flat_plate_object`, `identity_transform`,
// `sloped_triangle_object`, `wasm_cache::{shared_engine, compiled_component_at,
// compiled_guest, compiled_wat}`) are duplicated in
// `crates/slicer-wasm-host/tests/common/` by design (P83.1, AC-N3 — no dev-edge
// back to runtime). A future packet can extract them into a shared
// `slicer-test-fixtures` crate if duplication grows.
pub mod dispatch_fixture;
pub mod ir_builders;
pub mod model_cache;
pub mod seed;
pub mod slicer_cache;
pub mod wasm_cache;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
};
use slicer_runtime::wit_host::{HostExecutionContext, HostExecutionContextBuilder};
use slicer_runtime::{
    Blackboard, FinalizationStageInput, LayerArena, LayerStageInput, PostpassStageInput,
    PrepassStageInput,
};

pub fn ctx_with_mesh(module_id: &str, mesh: Arc<MeshIR>) -> HostExecutionContext {
    HostExecutionContextBuilder::new(module_id.to_string(), 0.0, 0.0)
        .mesh_ir(Some(mesh))
        .build()
}

pub fn point3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

pub fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

pub fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: point3(0.0, 0.0, 0.0),
        max: point3(200.0, 200.0, 200.0),
    }
}

pub fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

pub fn translation_transform(tx: f64, ty: f64, tz: f64) -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, tz, 1.0,
        ],
    }
}

pub fn mesh_fixture(objects: Vec<ObjectMesh>) -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(),
        objects,
        build_volume: build_volume(),
    })
}

pub fn flat_plate_object(id: &str, local_z: f32, transform: Transform3d) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                point3(0.0, 0.0, local_z),
                point3(10.0, 0.0, local_z),
                point3(0.0, 10.0, local_z),
                point3(10.0, 10.0, local_z),
            ],
            indices: vec![0, 1, 2, 1, 3, 2],
        },
        transform,
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    }
}

/// Reserved for future non-axis-aligned surface normal tests.
#[allow(dead_code)]
pub fn sloped_triangle_object(id: &str, transform: Transform3d) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                point3(0.0, 0.0, 0.0),
                point3(10.0, 0.0, 0.0),
                point3(0.0, 10.0, 10.0),
            ],
            indices: vec![0, 1, 2],
        },
        transform,
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    }
}

// ── Ordered-entities counter serialization (D-113B) ───────────────────────
// `slicer_wasm_host::host::HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS` is a single
// process-wide `AtomicU32` shared by every test in this binary that drives a
// real `Layer::PathOptimization` WASM dispatch. Rust's default test harness
// runs `#[test]` fns concurrently on multiple threads, so a reset+read pair
// in one test races against increments from unrelated sibling tests unless
// all of them serialize through this lock for their dispatch critical
// section. See docs/DEVIATION_LOG.md D-113B-ORDERED-ENTITIES-COUNTER-RACE.
static ORDERED_ENTITIES_COUNTER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn ordered_entities_counter_lock() -> MutexGuard<'static, ()> {
    ORDERED_ENTITIES_COUNTER_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn assert_close(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() < 1.0e-4,
        "{label} expected {expected}, got {actual}"
    );
}

pub fn assert_unit_length(x: f32, y: f32, z: f32, label: &str) {
    let magnitude = (x * x + y * y + z * z).sqrt();
    assert_close(magnitude, 1.0, label);
}

pub fn assert_perpendicular(x: f32, y: f32, z: f32, edge1: [f32; 3], edge2: [f32; 3], label: &str) {
    let dot1 = x * edge1[0] + y * edge1[1] + z * edge1[2];
    let dot2 = x * edge2[0] + y * edge2[1] + z * edge2[2];
    assert_close(dot1, 0.0, &format!("{label} dot edge1"));
    assert_close(dot2, 0.0, &format!("{label} dot edge2"));
}

// ── commit_hec_for_test ───────────────────────────────────────────────────────
// Thin test helper that converts a legacy `HostExecutionContext` (built in tests
// via `HostExecutionContextBuilder`) into the new `LayerStageCommit` IR enum,
// then delegates to `slicer_runtime::apply_for_test`.
// Only the stage families exercised by the executor tests are handled; all other
// stage_ids are treated as a no-op (no commit).

#[allow(dead_code)]
pub fn commit_hec_for_test(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    ctx: &slicer_runtime::wit_host::HostExecutionContext,
    arena: &mut slicer_runtime::LayerArena,
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
) -> Result<(), slicer_ir::LayerStageError> {
    use slicer_ir::{
        LayerAnnotationKind, LayerStageCommit, PathOptimizationCommit, RetractSpec, TravelMoveDest,
    };
    use slicer_runtime::wit_host::{
        convert_infill_output, convert_perimeter_output, convert_support_output,
        GcodeCommandCollected,
    };
    use slicer_runtime::StageApplyContext;

    let mk_fatal = |what: &str, reason: String| slicer_ir::LayerStageError::FatalModule {
        stage_id: stage_id.to_string(),
        module_id: module_id.to_string(),
        message: format!("invalid {what} output: {reason}"),
    };

    let apply_ctx = StageApplyContext {
        stage_id,
        module_id,
        layer_index,
        seam_plan: seam_plan_ir,
    };

    let commit_opt: Option<LayerStageCommit> = match stage_id {
        "Layer::Infill" => {
            let infill = ctx.infill_output();
            if !infill.sparse_paths.is_empty()
                || !infill.solid_paths.is_empty()
                || !infill.ironing_paths.is_empty()
            {
                let ir = convert_infill_output(infill, layer_index)
                    .map_err(|r| mk_fatal("infill", r))?;
                Some(LayerStageCommit::Infill(ir))
            } else {
                None
            }
        }
        "Layer::InfillPostProcess" => {
            let infill = ctx.infill_output();
            if !infill.sparse_paths.is_empty()
                || !infill.solid_paths.is_empty()
                || !infill.ironing_paths.is_empty()
            {
                let ir = convert_infill_output(infill, layer_index)
                    .map_err(|r| mk_fatal("infill", r))?;
                Some(LayerStageCommit::InfillPostProcess(ir))
            } else {
                None
            }
        }
        "Layer::Support" => {
            let support = ctx.support_output();
            if !support.support_paths.is_empty()
                || !support.interface_paths.is_empty()
                || !support.raft_paths.is_empty()
            {
                let ir = convert_support_output(support, layer_index)
                    .map_err(|r| mk_fatal("support", r))?;
                Some(LayerStageCommit::Support(ir))
            } else {
                None
            }
        }
        "Layer::SupportPostProcess" => {
            let support = ctx.support_output();
            if !support.support_paths.is_empty()
                || !support.interface_paths.is_empty()
                || !support.raft_paths.is_empty()
            {
                let ir = convert_support_output(support, layer_index)
                    .map_err(|r| mk_fatal("support", r))?;
                Some(LayerStageCommit::SupportPostProcess(ir))
            } else {
                None
            }
        }
        "Layer::Perimeters" => {
            let perimeter = ctx.perimeter_output();
            let has_any = !perimeter.wall_loops.is_empty()
                || !perimeter.rotated_wall_loops.is_empty()
                || !perimeter.infill_areas.is_empty()
                || !perimeter.seam_candidates.is_empty();
            if has_any {
                let ir = convert_perimeter_output(perimeter, layer_index)
                    .map_err(|r| mk_fatal("perimeter", r))?;
                Some(LayerStageCommit::Perimeters(ir))
            } else {
                None
            }
        }
        "Layer::PerimetersPostProcess" => {
            let perimeter = ctx.perimeter_output();
            let has_any = !perimeter.wall_loops.is_empty()
                || !perimeter.rotated_wall_loops.is_empty()
                || !perimeter.infill_areas.is_empty()
                || !perimeter.seam_candidates.is_empty();
            if has_any {
                let ir = convert_perimeter_output(perimeter, layer_index)
                    .map_err(|r| mk_fatal("perimeter", r))?;
                Some(LayerStageCommit::PerimetersPostProcess(Some(ir)))
            } else {
                None
            }
        }
        "Layer::PathOptimization" => {
            let mut tool_changes = Vec::new();
            let mut z_hops = Vec::new();
            let mut annotations = Vec::new();
            let mut retracts = Vec::new();
            let mut travel_moves = Vec::new();
            for (i, cmd) in ctx.gcode_output().commands.iter().enumerate() {
                match cmd {
                    GcodeCommandCollected::ToolChange {
                        after_entity_index,
                        from_tool,
                        to_tool,
                    } => {
                        tool_changes.push(slicer_ir::ToolChange {
                            after_entity_index: *after_entity_index,
                            from_tool: *from_tool,
                            to_tool: *to_tool,
                        });
                    }
                    GcodeCommandCollected::Comment(text) => {
                        annotations.push(LayerAnnotationKind::Comment(text.clone()));
                    }
                    GcodeCommandCollected::Raw(text) => {
                        annotations.push(LayerAnnotationKind::Raw(text.clone()));
                    }
                    GcodeCommandCollected::Move(cmd) => {
                        travel_moves.push(TravelMoveDest {
                            x: cmd.x,
                            y: cmd.y,
                            z: cmd.z,
                            f: cmd.f,
                        });
                    }
                    GcodeCommandCollected::ZHop { hop_height, .. } => {
                        if !hop_height.is_finite() || *hop_height <= 0.0 {
                            return Err(slicer_ir::LayerStageError::FatalModule {
                                stage_id: stage_id.to_string(),
                                module_id: module_id.to_string(),
                                message: format!(
                                    "Layer::PathOptimization push-z-hop call {i} rejected: \
                                     hop-height={hop_height} not finite and strictly positive"
                                ),
                            });
                        }
                        z_hops.push(*hop_height);
                    }
                    GcodeCommandCollected::Retract {
                        length,
                        speed,
                        mode,
                    } => {
                        retracts.push(RetractSpec {
                            length: *length,
                            speed: *speed,
                            is_unretract: false,
                            mode: *mode,
                        });
                    }
                    GcodeCommandCollected::Unretract {
                        length,
                        speed,
                        mode,
                    } => {
                        retracts.push(RetractSpec {
                            length: *length,
                            speed: *speed,
                            is_unretract: true,
                            mode: *mode,
                        });
                    }
                    other => {
                        return Err(slicer_ir::LayerStageError::FatalModule {
                            stage_id: stage_id.to_string(),
                            module_id: module_id.to_string(),
                            message: format!(
                                "Layer::PathOptimization unsupported GCode command at {i}: \
                                 {:?}",
                                std::mem::discriminant(other)
                            ),
                        });
                    }
                }
            }
            let is_empty = tool_changes.is_empty()
                && z_hops.is_empty()
                && annotations.is_empty()
                && retracts.is_empty()
                && travel_moves.is_empty();
            if is_empty {
                None
            } else {
                Some(LayerStageCommit::PathOptimization(PathOptimizationCommit {
                    tool_changes,
                    z_hops,
                    annotations,
                    retracts,
                    travel_moves,
                    order_proposal: None,
                }))
            }
        }
        _ => None,
    };

    if let Some(commit) = commit_opt {
        slicer_runtime::apply_for_test(arena, commit, &apply_ctx)
    } else {
        Ok(())
    }
}

// ── Stage input helpers ───────────────────────────────────────────────────────
// Construct *StageInput borrow structs from the legacy Blackboard/LayerArena
// so existing test call sites can use the new trait boundary with minimal churn.

pub fn layer_input<'a>(blackboard: &'a Blackboard, arena: &'a LayerArena) -> LayerStageInput<'a> {
    LayerStageInput {
        mesh: blackboard.mesh().clone(),
        paint_regions: None,
        seam_plan: blackboard.seam_plan().cloned(),
        support_plan: blackboard.support_plan().cloned(),
        region_map: blackboard.region_map().cloned(),
        slice: arena.slice(),
        perimeter: arena.perimeter(),
        layer_collection: arena.layer_collection(),
        surface_classification: blackboard.surface_classification().map(|a| a.as_ref()),
        infill: arena.infill(),
    }
}

pub fn prepass_input(blackboard: &Blackboard) -> PrepassStageInput<'_> {
    PrepassStageInput {
        mesh: blackboard.mesh().clone(),
        layer_plan: blackboard.layer_plan().cloned(),
        region_map: blackboard.region_map().cloned(),
        support_geometry: blackboard.support_geometry().cloned(),
        _phantom: PhantomData,
    }
}

pub fn finalization_input(blackboard: &Blackboard) -> FinalizationStageInput<'_> {
    FinalizationStageInput {
        mesh: blackboard.mesh().clone(),
        _phantom: PhantomData,
    }
}

pub fn postpass_input(blackboard: &Blackboard) -> PostpassStageInput<'_> {
    PostpassStageInput {
        mesh: blackboard.mesh().clone(),
        _phantom: PhantomData,
    }
}

/// Test-only bundle that carries the `CompiledModule` together with the wasm-host
/// handles (`pool`, `component`) that are no longer stored inside the module itself
/// (post-P85 field migration). Use `as_live()` to get a `CompiledModuleLive<'_>`.
#[allow(dead_code)]
pub struct TestModuleBundle {
    pub module: slicer_runtime::CompiledModule,
    pub pool: Arc<slicer_wasm_host::WasmInstancePool>,
    pub component: Option<Arc<slicer_wasm_host::WasmComponent>>,
}

impl TestModuleBundle {
    #[allow(dead_code)]
    pub fn as_live(&self) -> slicer_wasm_host::CompiledModuleLive<'_> {
        slicer_wasm_host::CompiledModuleLive::new(
            self.module.module_id(),
            Arc::clone(&self.pool),
            self.component.clone(),
            self.module.claims(),
            Arc::clone(self.module.config_view()),
        )
    }

    /// Consume the bundle and return the inner `CompiledModule` plus the
    /// matching `wasm_handles` entry. Useful when building an ExecutionPlan
    /// that needs both the module (to put into a CompiledStage) and a
    /// `wasm_handles` map keyed by module_id (for execute_prepass).
    #[allow(dead_code)]
    pub fn into_module_and_handles(
        self,
    ) -> (
        slicer_runtime::CompiledModule,
        HashMap<
            String,
            (
                Arc<slicer_wasm_host::WasmInstancePool>,
                Option<Arc<slicer_wasm_host::WasmComponent>>,
            ),
        >,
    ) {
        let id = self.module.module_id().to_string();
        let mut handles = HashMap::new();
        handles.insert(id, (self.pool, self.component));
        (self.module, handles)
    }
}

/// Real-wasm variant of `run_layer_and_commit` that uses the bundle's actual
/// `pool` and `component` (not the placeholder). dispatch_tdd needs this to
/// drive real guest execution.
#[allow(dead_code)]
pub fn run_layer_and_commit_with_bundle(
    dispatcher: &slicer_wasm_host::WasmRuntimeDispatcher,
    stage_id: &str,
    layer: &slicer_ir::GlobalLayer,
    bundle: &TestModuleBundle,
    blackboard: &Blackboard,
    arena: &mut slicer_runtime::LayerArena,
) -> Result<(), slicer_ir::LayerStageError> {
    use slicer_runtime::StageApplyContext;
    use slicer_wasm_host::LayerStageRunner;
    let live = bundle.as_live();
    let input = layer_input(blackboard, arena);
    let commit_opt =
        LayerStageRunner::run_stage(dispatcher, &stage_id.to_string(), layer, &live, input)?;
    let seam_plan_arc = blackboard.seam_plan().cloned();
    let ctx = StageApplyContext {
        stage_id,
        module_id: bundle.module.module_id(),
        layer_index: layer.index,
        seam_plan: seam_plan_arc.as_deref(),
    };
    // PerimetersPostProcess(None) still needs apply so seam back-fill runs on
    // the existing arena perimeter even when the guest emitted no new perimeter.
    let effective_commit = if commit_opt.is_none() && stage_id == "Layer::PerimetersPostProcess" {
        Some(slicer_ir::LayerStageCommit::PerimetersPostProcess(None))
    } else {
        commit_opt
    };
    if let Some(commit) = effective_commit {
        slicer_runtime::apply_for_test(arena, commit, &ctx)
    } else {
        Ok(())
    }
}

/// Convenience: dispatch a Layer stage AND commit the resulting LayerStageCommit
/// to the arena in one call — bridges the orchestration split so tests that previously
/// expected `run_stage` to mutate `arena` continue to work via this single helper.
#[allow(dead_code)]
pub fn run_layer_and_commit(
    dispatcher: &slicer_wasm_host::WasmRuntimeDispatcher,
    stage_id: &str,
    layer: &slicer_ir::GlobalLayer,
    module: &slicer_runtime::CompiledModule,
    blackboard: &Blackboard,
    arena: &mut slicer_runtime::LayerArena,
) -> Result<(), slicer_ir::LayerStageError> {
    use slicer_runtime::StageApplyContext;
    use slicer_wasm_host::LayerStageRunner;
    let live = slicer_wasm_host::CompiledModuleLive::new(
        module.module_id(),
        slicer_wasm_host::WasmInstancePool::placeholder(),
        None,
        module.claims(),
        Arc::clone(module.config_view()),
    );
    let input = layer_input(blackboard, arena);
    let commit_opt =
        LayerStageRunner::run_stage(dispatcher, &stage_id.to_string(), layer, &live, input)?;
    let seam_plan_arc = blackboard.seam_plan().cloned();
    let ctx = StageApplyContext {
        stage_id,
        module_id: module.module_id(),
        layer_index: layer.index,
        seam_plan: seam_plan_arc.as_deref(),
    };
    // PerimetersPostProcess(None) still needs apply so seam back-fill runs on
    // the existing arena perimeter even when the guest emitted no new perimeter.
    let effective_commit = if commit_opt.is_none() && stage_id == "Layer::PerimetersPostProcess" {
        Some(slicer_ir::LayerStageCommit::PerimetersPostProcess(None))
    } else {
        commit_opt
    };
    if let Some(commit) = effective_commit {
        slicer_runtime::apply_for_test(arena, commit, &ctx)
    } else {
        Ok(())
    }
}
