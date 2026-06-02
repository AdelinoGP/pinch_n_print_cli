#![allow(dead_code)]

// These shared fixtures and the engine/component cache helpers (`ctx_with_mesh`,
// `mesh_fixture`, `flat_plate_object`, `identity_transform`,
// `sloped_triangle_object`, `wasm_cache::{shared_engine, compiled_component_at,
// compiled_guest, compiled_wat}`) are duplicated in
// `crates/slicer-wasm-host/tests/common/` by design (P83.1, AC-N3 — no dev-edge
// back to runtime). A future packet can extract them into a shared
// `slicer-test-fixtures` crate if duplication grows.
pub mod model_cache;
pub mod seed;
pub mod slicer_cache;
pub mod wasm_cache;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

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
// via `HostExecutionContextBuilder`) into the new `LayerStageCommitData` IR
// struct, then delegates to `slicer_runtime::commit_layer_outputs_for_test`.
// Only the stage families exercised by the executor tests are handled; all other
// stage_ids produce an empty `LayerStageCommitData::default()`.

#[allow(dead_code)]
pub fn commit_hec_for_test(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    ctx: &slicer_runtime::wit_host::HostExecutionContext,
    arena: &mut slicer_runtime::LayerArena,
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
) -> Result<(), slicer_ir::LayerStageError> {
    use slicer_ir::LayerStageCommitData;
    use slicer_runtime::wit_host::{
        convert_infill_output, convert_perimeter_output, convert_support_output,
        GcodeCommandCollected,
    };

    let mk_fatal = |what: &str, reason: String| slicer_ir::LayerStageError::FatalModule {
        stage_id: stage_id.to_string(),
        module_id: module_id.to_string(),
        message: format!("invalid {what} output: {reason}"),
    };

    let mut data = LayerStageCommitData::default();

    match stage_id {
        "Layer::Infill" | "Layer::InfillPostProcess" => {
            let infill = ctx.infill_output();
            if !infill.sparse_paths.is_empty()
                || !infill.solid_paths.is_empty()
                || !infill.ironing_paths.is_empty()
            {
                data.infill_output = Some(
                    convert_infill_output(infill, layer_index)
                        .map_err(|r| mk_fatal("infill", r))?,
                );
            }
        }
        "Layer::Support" | "Layer::SupportPostProcess" => {
            let support = ctx.support_output();
            if !support.support_paths.is_empty()
                || !support.interface_paths.is_empty()
                || !support.raft_paths.is_empty()
            {
                data.support_output = Some(
                    convert_support_output(support, layer_index)
                        .map_err(|r| mk_fatal("support", r))?,
                );
            }
        }
        "Layer::Perimeters" | "Layer::PerimetersPostProcess" => {
            let perimeter = ctx.perimeter_output();
            let has_any = !perimeter.wall_loops.is_empty()
                || !perimeter.rotated_wall_loops.is_empty()
                || !perimeter.infill_areas.is_empty()
                || !perimeter.seam_candidates.is_empty();
            if has_any {
                data.perimeter_output = Some(
                    convert_perimeter_output(perimeter, layer_index)
                        .map_err(|r| mk_fatal("perimeter", r))?,
                );
            }
        }
        "Layer::PathOptimization" => {
            let anchor = 0u32;
            for (i, cmd) in ctx.gcode_output().commands.iter().enumerate() {
                match cmd {
                    GcodeCommandCollected::ToolChange {
                        after_entity_index,
                        from_tool,
                        to_tool,
                    } => {
                        data.tool_changes.push(slicer_ir::ToolChange {
                            after_entity_index: *after_entity_index,
                            from_tool: *from_tool,
                            to_tool: *to_tool,
                        });
                    }
                    GcodeCommandCollected::Comment(text) => {
                        data.annotations.push(slicer_ir::LayerAnnotation {
                            after_entity_index: anchor,
                            kind: slicer_ir::LayerAnnotationKind::Comment(text.clone()),
                        });
                    }
                    GcodeCommandCollected::Raw(text) => {
                        data.annotations.push(slicer_ir::LayerAnnotation {
                            after_entity_index: anchor,
                            kind: slicer_ir::LayerAnnotationKind::Raw(text.clone()),
                        });
                    }
                    GcodeCommandCollected::Move(cmd) => {
                        data.deferred_travel_moves
                            .push((anchor, cmd.x, cmd.y, cmd.z, cmd.f));
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
                        data.z_hops.push(slicer_ir::ZHop {
                            after_entity_index: anchor,
                            hop_height: *hop_height,
                        });
                    }
                    GcodeCommandCollected::Retract {
                        length,
                        speed,
                        mode,
                    } => {
                        data.retracts.push(slicer_ir::TravelRetract {
                            after_entity_index: anchor,
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
                        data.retracts.push(slicer_ir::TravelRetract {
                            after_entity_index: anchor,
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
        }
        _ => {}
    }

    slicer_runtime::commit_layer_outputs_for_test(
        stage_id,
        module_id,
        layer_index,
        data,
        arena,
        seam_plan_ir,
    )
}

// ── Stage input helpers ───────────────────────────────────────────────────────
// Construct *StageInput borrow structs from the legacy Blackboard/LayerArena
// so existing test call sites can use the new trait boundary with minimal churn.

pub fn layer_input<'a>(blackboard: &Blackboard, arena: &'a LayerArena) -> LayerStageInput<'a> {
    LayerStageInput {
        mesh: blackboard.mesh().clone(),
        paint_regions: blackboard.paint_regions().cloned(),
        seam_plan: blackboard.seam_plan().cloned(),
        support_plan: blackboard.support_plan().cloned(),
        region_map: blackboard.region_map().cloned(),
        slice: arena.slice(),
        perimeter: arena.perimeter(),
        layer_collection: arena.layer_collection(),
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

/// Convenience: dispatch a Layer stage AND commit the resulting LayerStageCommitData
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
    use slicer_wasm_host::LayerStageRunner;
    let live = module.as_live();
    let input = layer_input(blackboard, arena);
    let commit_data =
        LayerStageRunner::run_stage(dispatcher, &stage_id.to_string(), layer, &live, input)?;
    let seam_plan_arc = blackboard.seam_plan().cloned();
    slicer_runtime::commit_layer_outputs_for_test(
        stage_id,
        module.module_id(),
        layer.index,
        commit_data,
        arena,
        seam_plan_arc.as_deref(),
    )
}
