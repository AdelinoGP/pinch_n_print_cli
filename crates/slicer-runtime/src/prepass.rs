//! PrePass execution contracts.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

pub use slicer_core::{
    FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, PrepassStageOutput,
    SurfaceGroupRecord,
};
use slicer_ir::{ConfigKey, ConfigValue, ModuleId, ResolvedConfig, StageId};

use crate::builtins::overhang_annotation_producer::{
    commit_overhang_annotation_builtin, OverhangAnnotationBuiltinError,
};
use crate::builtins::region_mapping_producer::{
    commit_region_mapping_builtin, RegionMappingBuiltinError,
};
use crate::instrumentation::{
    NoopInstrumentation, PipelineInstrumentation, StageInstrumentationGuard,
};
use crate::validation::ModuleAccessAudit;
use crate::{Blackboard, BlackboardError, BlackboardPrepassSlot, ExecutionPlan};
use slicer_core::algos::mesh_analysis::{execute_mesh_analysis, MeshAnalysisError};
use slicer_core::algos::support_geometry::SupportGeometryBuiltinError;
use slicer_wasm_host::{
    CompiledModuleLive, PrepassStageInput, PrepassStageRunner, WasmComponent, WasmInstancePool,
};

// PrepassStageRunner trait is now defined in slicer-wasm-host::traits and re-exported
// from slicer_runtime via the transitional re-exports block in lib.rs (P83 Step 4c+4d).
// The trait signature changed to take CompiledModuleLive<'_> + PrepassStageInput<'_>
// and return PrepassStageOutput (not a tuple with runtime_reads).

/// Structured prepass executor failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepassExecutionError {
    /// A stage started before one of its required prepass inputs existed.
    MissingRequiredPrepass {
        /// Stage that required the missing input.
        stage_id: StageId,
        /// Missing blackboard slot.
        slot: BlackboardPrepassSlot,
    },
    /// A module returned a fatal error.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// A blackboard commit contract failed.
    Blackboard {
        /// Stage being committed.
        stage_id: StageId,
        /// Module whose commit failed.
        module_id: ModuleId,
        /// Underlying blackboard failure.
        source: BlackboardError,
    },
    /// The host-built-in `PrePass::MeshAnalysis` stage failed.
    MeshAnalysis {
        /// Underlying mesh-analysis failure.
        source: MeshAnalysisError,
    },
    /// The host-built-in `PrePass::RegionMapping` stage failed.
    RegionMapping {
        /// Underlying region-mapping failure.
        source: RegionMappingBuiltinError,
    },
    /// The host-built-in `PrePass::OverhangAnnotation` stage failed.
    OverhangAnnotation {
        /// Underlying overhang-annotation failure.
        source: OverhangAnnotationBuiltinError,
    },
    /// The host-built-in `PrePass::SupportGeometry` stage failed.
    SupportGeometry {
        /// Underlying support geometry failure.
        source: SupportGeometryBuiltinError,
    },
    /// The host-built-in `PrePass::LightningTreeGen` stage failed
    /// (packet 137).
    LightningTree {
        /// Underlying blackboard failure (e.g. duplicate commit).
        source: BlackboardError,
    },
    /// The host-built-in `PrePass::Slice` stage failed.
    Slice {
        /// Underlying slice failure.
        source: slicer_core::algos::prepass_slice::LayerSliceError,
    },
    /// The host-built-in `PrePass::ShellClassification` stage failed.
    ShellClassification {
        /// Underlying shell-classification failure.
        source: crate::slice_postprocess_prepass::ShellClassificationError,
    },
    /// The host-built-in `PrePass::PaintSegmentation` stage (sub-step 15 / AC-14) failed.
    /// The source is stored as a message string because `PaintSegmentationError` does not
    /// implement `Clone + PartialEq + Eq` (required by the outer derive).
    PaintSegmentation {
        /// Human-readable description of the paint-segmentation failure.
        message: String,
    },
}

impl fmt::Display for PrepassExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredPrepass { stage_id, slot } => {
                write!(f, "stage {stage_id} requires committed prepass slot {slot}")
            }
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal prepass module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::Blackboard {
                stage_id,
                module_id,
                source,
            } => write!(
                f,
                "blackboard commit failed in {stage_id} for {module_id}: {source}"
            ),
            Self::MeshAnalysis { source } => {
                write!(f, "built-in PrePass::MeshAnalysis failed: {source}")
            }
            Self::RegionMapping { source } => {
                write!(f, "built-in PrePass::RegionMapping failed: {source}")
            }
            Self::OverhangAnnotation { source } => {
                write!(f, "built-in PrePass::OverhangAnnotation failed: {source}")
            }
            Self::SupportGeometry { source } => {
                write!(f, "built-in PrePass::SupportGeometry failed: {source}")
            }
            Self::LightningTree { source } => {
                write!(f, "built-in PrePass::LightningTreeGen failed: {source}")
            }
            Self::Slice { source } => {
                write!(f, "built-in PrePass::Slice failed: {source}")
            }
            Self::ShellClassification { source } => {
                write!(f, "built-in PrePass::ShellClassification failed: {source}")
            }
            Self::PaintSegmentation { message } => {
                write!(f, "built-in PrePass::PaintSegmentation failed: {message}")
            }
        }
    }
}

impl std::error::Error for PrepassExecutionError {}

impl From<slicer_ir::PrepassRunnerError> for PrepassExecutionError {
    fn from(e: slicer_ir::PrepassRunnerError) -> Self {
        match e {
            slicer_ir::PrepassRunnerError::FatalModule {
                stage_id,
                module_id,
                message,
            } => Self::FatalModule {
                stage_id,
                module_id,
                message,
            },
            slicer_ir::PrepassRunnerError::Blackboard {
                stage_id,
                module_id,
                source,
            } => Self::Blackboard {
                stage_id,
                module_id,
                source,
            },
        }
    }
}

/// Executes the sequential Tier 1 prepass pipeline.
///
/// Returns collected runtime access audits for all user modules that executed.
/// Host built-ins (MeshAnalysis, RegionMapping) are not audited as they are
/// not subject to the module access contract.
pub fn execute_prepass(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
    execute_prepass_with_instrumentation(
        plan,
        blackboard,
        runner,
        &NoopInstrumentation,
        wasm_handles,
    )
}

/// Instrumented variant of [`execute_prepass`] that brackets each stage and
/// module dispatch via `instrumentation`. Identical semantics to
/// `execute_prepass` when `&NoopInstrumentation` is passed.
pub fn execute_prepass_with_instrumentation(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
    let mut audits = Vec::new();

    for stage in &plan.prepass_stages {
        ensure_stage_prerequisites(&stage.stage_id, blackboard)?;

        instrumentation.on_stage_start(&stage.stage_id, None);
        for module in &stage.modules {
            instrumentation.on_module_start(&stage.stage_id, None, module.module_id());
            // Build IR-typed borrow structs for the new slicer-wasm-host trait boundary.
            let (instance_pool, wasm_component) = wasm_handles
                .get(module.module_id().as_str())
                .map(|(p, c)| (Arc::clone(p), c.clone()))
                .unwrap_or_else(|| (WasmInstancePool::placeholder(), None));
            let live_module = CompiledModuleLive::new(
                module.module_id(),
                instance_pool,
                wasm_component,
                module.claims(),
                Arc::clone(module.config_view()),
            );
            let input = PrepassStageInput {
                mesh: std::sync::Arc::clone(blackboard.mesh()),
                layer_plan: blackboard.layer_plan().cloned(),
                region_map: blackboard.region_map().cloned(),
                support_geometry: blackboard.support_geometry().cloned(),
                _phantom: std::marker::PhantomData,
            };
            let run_result = runner.run_stage(&stage.stage_id, &live_module, input);
            instrumentation.on_module_end(&stage.stage_id, None, module.module_id(), 0, 0);
            // Map PrepassRunnerError → PrepassExecutionError via the From impl.
            let output = match run_result {
                Ok(o) => o,
                Err(e) => {
                    instrumentation.on_module_error(
                        &stage.stage_id,
                        None,
                        module.module_id(),
                        &e.to_string(),
                        true,
                    );
                    instrumentation.on_stage_end(&stage.stage_id, None);
                    return Err(PrepassExecutionError::from(e));
                }
            };
            let runtime_reads: Vec<String> = runner.last_runtime_reads();
            // Drain module log messages (already forwarded to the log facade
            // inside the dispatcher; this clears the thread-local stash).
            let _log_messages = runner.last_log_messages();

            // Determine IR path before committing (output is moved into commit).
            let ir_path = ir_path_for_prepass_output(&output);

            if let Err(e) =
                commit_stage_output(&stage.stage_id, module.module_id(), blackboard, output)
            {
                instrumentation.on_module_error(
                    &stage.stage_id,
                    None,
                    module.module_id(),
                    &e.to_string(),
                    true,
                );
                instrumentation.on_stage_end(&stage.stage_id, None);
                return Err(e);
            }

            // Record runtime audit if the module produced output.
            // Always record the audit when there is a runtime_reads vector,
            // even if the output is None (read-performing modules that produce
            // no IR output still have their reads audited).
            if let Some(ir_path) = ir_path {
                audits.push(ModuleAccessAudit {
                    module_id: module.module_id().to_owned(),
                    runtime_reads,
                    runtime_writes: vec![ir_path],
                });
            } else if !runtime_reads.is_empty() {
                // Module performed reads but produced no output — still record audit.
                audits.push(ModuleAccessAudit {
                    module_id: module.module_id().to_owned(),
                    runtime_reads,
                    runtime_writes: Vec::new(),
                });
            }
        }
        instrumentation.on_stage_end(&stage.stage_id, None);
    }

    Ok(audits)
}

/// Maps a prepass stage output variant to the canonical IR field path written.
fn ir_path_for_prepass_output(output: &PrepassStageOutput) -> Option<String> {
    match output {
        PrepassStageOutput::None => None,
        PrepassStageOutput::SurfaceClassification(_) => {
            Some(String::from("SurfaceClassificationIR"))
        }
        PrepassStageOutput::LayerPlan(_) => Some(String::from("LayerPlanIR")),
        PrepassStageOutput::SeamPlan(_) => Some(String::from("SeamPlanIR")),
        PrepassStageOutput::SupportPlan(_) => Some(String::from("SupportPlanIR")),
        PrepassStageOutput::RegionMap(_) => Some(String::from("RegionMapIR")),
        PrepassStageOutput::SupportGeometry(_) => Some(String::from("SupportGeometryIR")),
        // MeshAnalysisAuxiliary is auxiliary data, not a primary IR commit.
        PrepassStageOutput::MeshAnalysisAuxiliary(_) => None,
    }
}

/// Run the host-built-in [`PrePass::MeshAnalysis`](execute_mesh_analysis)
/// stage and then [`execute_prepass`].
///
/// This is the prepass entry-point used by the real pipeline (docs/04
/// §Full Lifecycle — prepass block): the built-in commits
/// `SurfaceClassificationIR` into the blackboard before any user prepass
/// module runs. If a caller has already committed a surface
/// classification (e.g. an earlier integration test pre-seeded one) the
/// built-in step is skipped so commits remain exactly-once.
///
/// Returns collected runtime access audits from user prepass modules.
/// Host built-ins (MeshAnalysis, RegionMapping) are not audited as they are
/// not subject to the module access contract.
///
/// This is the backwards-compatible public entry point. It delegates to
/// [`execute_prepass_with_builtins_configured`] with empty per-object configs and
/// a default global config, which preserves the existing behaviour for all callers
/// that do not yet supply resolved configs (e.g. test helpers).
pub fn execute_prepass_with_builtins(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
    let empty_resolved: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    let default_resolved = ResolvedConfig::default();
    let empty_raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    let empty_bounds = crate::ConfigBoundsIndex::empty();
    execute_prepass_with_builtins_configured(
        plan,
        blackboard,
        runner,
        &empty_resolved,
        &default_resolved,
        &empty_raw,
        &empty_bounds,
        wasm_handles,
    )
}

/// Like [`execute_prepass_with_builtins`] but threads per-object resolved configs
/// into the RegionMapping built-in so region plans carry live config values.
///
/// This is the authoritative implementation; the public wrapper above forwards
/// to this with empty / default values for backwards compatibility.
pub fn execute_prepass_with_builtins_configured(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
    resolved_configs: &BTreeMap<String, ResolvedConfig>,
    default_resolved_config: &ResolvedConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    bounds: &crate::ConfigBoundsIndex,
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
    execute_prepass_with_builtins_configured_instr(
        plan,
        blackboard,
        runner,
        resolved_configs,
        default_resolved_config,
        raw_config_source,
        bounds,
        &NoopInstrumentation,
        wasm_handles,
    )
}

/// Instrumented version of [`execute_prepass_with_builtins_configured`] that
/// brackets each prepass stage and module (including host built-ins) via
/// `instrumentation`.
///
/// Made `pub` (rather than `pub(crate)`) so integration tests can observe the
/// host built-ins' stage-order trace (e.g. asserting `PrePass::
/// OverhangAnnotation` runs strictly after `PrePass::MeshAnalysis` /
/// `PrePass::LayerPlanning`) without needing the full `pipeline::run_pipeline*`
/// stack. See `crates/slicer-runtime/tests/executor/
/// prepass_overhang_annotation_stage_order_tdd.rs`.
pub fn execute_prepass_with_builtins_configured_instr(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
    resolved_configs: &BTreeMap<String, ResolvedConfig>,
    default_resolved_config: &ResolvedConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    bounds: &crate::ConfigBoundsIndex,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::MeshAnalysis",
        "host:mesh_analysis",
        |bb| bb.surface_classification().is_none(),
        |bb| {
            let ir = execute_mesh_analysis(bb.mesh().as_ref())
                .map_err(|source| PrepassExecutionError::MeshAnalysis { source })?;
            bb.commit_surface_classification(std::sync::Arc::new(ir))
                .map_err(|source| PrepassExecutionError::Blackboard {
                    stage_id: "PrePass::MeshAnalysis".to_string(),
                    module_id: "host:mesh_analysis".to_string(),
                    source,
                })
        },
    )?;
    // PrePass::SupportGeometry moved to the post-RegionMapping / post-Slice
    // phase below, since it now depends on SliceIR (Commit 4 will consume real
    // slice polygons via collect_polygons_at_z; Commit 2 keeps the stub).
    /// Build per-semantic config overrides for the region-mapping builtin.
    ///
    /// Per packet 95 D10/D11: paint semantics present in the mesh are discovered
    /// by walking each object's `paint_data` (facet_values + strokes) and its
    /// `modifier_volumes` (support_enforcer / support_blocker subtypes).  The
    /// `paint_config:<semantic>:<key>` overlays in the raw config source are
    /// then resolved per-semantic via
    /// `slicer_scheduler::config_resolution::resolve_per_paint_semantic_configs`.
    ///
    /// Unknown-semantic warnings are silently dropped here — they surface at
    /// manifest-load time per P1b (the scheduler's CLI config resolver).
    fn build_paint_semantic_configs(
        blackboard: &Blackboard,
        default_resolved_config: &ResolvedConfig,
        raw_config_source: &HashMap<ConfigKey, ConfigValue>,
        bounds: &crate::ConfigBoundsIndex,
    ) -> BTreeMap<slicer_ir::PaintSemantic, ResolvedConfig> {
        use slicer_ir::PaintSemantic;
        let mesh = blackboard.mesh();
        let mut present: Vec<PaintSemantic> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut record = |sem: PaintSemantic, present: &mut Vec<PaintSemantic>| {
            if seen.insert(format!("{sem:?}")) {
                present.push(sem);
            }
        };
        for obj in &mesh.objects {
            if let Some(pd) = &obj.paint_data {
                for layer in &pd.layers {
                    let has_any =
                        layer.facet_values.iter().any(|v| v.is_some()) || !layer.strokes.is_empty();
                    if has_any {
                        record(layer.semantic.clone(), &mut present);
                    }
                }
            }
            for mv in &obj.modifier_volumes {
                if let Some(slicer_ir::ConfigValue::String(s)) =
                    mv.config_delta.fields.get("subtype")
                {
                    match s.as_str() {
                        "support_enforcer" => record(PaintSemantic::SupportEnforcer, &mut present),
                        "support_blocker" => record(PaintSemantic::SupportBlocker, &mut present),
                        _ => {}
                    }
                }
            }
        }
        if present.is_empty() {
            return BTreeMap::new();
        }
        let (map, _warnings) =
            slicer_scheduler::config_resolution::resolve_per_paint_semantic_configs(
                default_resolved_config,
                raw_config_source,
                &present,
                bounds,
            )
            .unwrap_or_else(|_| (BTreeMap::new(), Vec::new()));
        map
    }

    // Region-mapping runs after `PrePass::LayerPlanning` (user-or-none),
    // per canonical `STAGE_ORDER` in `docs/04_host_scheduler.md:111-119`.
    // The host built-in fallbacks honor the guard-based fallback contract at
    // `docs/04_host_scheduler.md:704`.
    //
    // Phase-1: early stages that don't require RegionMap.
    let early_stages: Vec<_> = plan
        .prepass_stages
        .iter()
        .filter(|s| !stage_requires_region_map(&s.stage_id))
        .collect();
    let mut audits = Vec::new();
    if !early_stages.is_empty() {
        let early_plan = ExecutionPlan {
            prepass_stages: early_stages.into_iter().cloned().collect(),
            ..plan.clone()
        };
        audits = execute_prepass_with_instrumentation(
            &early_plan,
            blackboard,
            runner,
            instrumentation,
            wasm_handles,
        )?;
    }
    // Region-mapping: needs LayerPlan; resolves per-paint-semantic config overlays
    // into RegionPlan.paint_overrides.
    //
    // `build_paint_semantic_configs` is computed *outside* the instrument
    // bracket (as it was before this refactor): it reads — never mutates — the
    // blackboard, so its placement cannot affect the byte snapshot, but keeping
    // it out of the bracket preserves the stage's wall-clock attribution exactly.
    let region_mapping_should_run =
        blackboard.layer_plan().is_some() && blackboard.region_map().is_none();
    let paint_semantic_configs = region_mapping_should_run.then(|| {
        build_paint_semantic_configs(
            blackboard,
            default_resolved_config,
            raw_config_source,
            bounds,
        )
    });
    // Per-tool/extruder config overlays (`tool_config:<n>:<key>`). Consumed by
    // region mapping for painted/MMU tools (the tool is known there via the
    // material variant chain) and applied at highest precedence. Empty unless
    // the user sets `tool_config:` keys, so default behaviour is unchanged.
    let tool_configs = region_mapping_should_run.then(|| {
        slicer_scheduler::config_resolution::resolve_per_tool_configs(
            default_resolved_config,
            raw_config_source,
            bounds,
        )
        .unwrap_or_default()
    });
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::RegionMapping",
        "host:region_mapping",
        |_bb| region_mapping_should_run,
        |bb| {
            let paint_semantic_configs = paint_semantic_configs
                .as_ref()
                .expect("computed whenever region_mapping_should_run is true");
            let tool_configs = tool_configs
                .as_ref()
                .expect("computed whenever region_mapping_should_run is true");
            commit_region_mapping_builtin(
                plan,
                bb,
                resolved_configs,
                default_resolved_config,
                paint_semantic_configs,
                tool_configs,
            )
            .map_err(|source| PrepassExecutionError::RegionMapping { source })
        },
    )?;
    // PrePass::Slice — host built-in. Runs once RegionMap is committed
    // (needs per-region slice_closing_radius / shell counts via RegionPlan).
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::Slice",
        "host:slice",
        |bb| bb.slice_ir().is_none() && bb.layer_plan().is_some() && bb.region_map().is_some(),
        |bb| {
            crate::builtins::prepass_slice_producer::commit_slice_builtin(bb)
                .map_err(|source| PrepassExecutionError::Slice { source })
        },
    )?;
    // PrePass::OverhangAnnotation — host built-in. Runs strictly AFTER Slice:
    // it derives each object's overhang bands from the committed SliceIR by
    // diffing consecutive-layer footprints (OrcaSlicer's
    // `detect_overhangs_for_lift`, `PrintObject.cpp:880-908`), never re-slicing
    // the mesh. Merges per-object `annotate_overhangs` output into a
    // replacement `SurfaceClassificationIR` via `replace_surface_classification`
    // for layer-tier consumers (perimeters, fuzzy-skin, infill).
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::OverhangAnnotation",
        "host:overhang_annotation",
        |bb| {
            bb.slice_ir().is_some()
                && bb.layer_plan().is_some()
                && bb.surface_classification().is_some()
        },
        |bb| {
            commit_overhang_annotation_builtin(bb, raw_config_source)
                .map_err(|source| PrepassExecutionError::OverhangAnnotation { source })
        },
    )?;
    // PrePass::ShellClassification — host built-in. Refines the freshly
    // committed SliceIR with top_shell_index / bottom_shell_index and
    // polygon-precise top_solid_fill / bottom_solid_fill via the two-pass
    // OrcaSlicer algorithm.
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::ShellClassification",
        "host:shell_classification",
        |bb| bb.slice_ir().is_some() && bb.region_map().is_some(),
        |bb| {
            crate::slice_postprocess_prepass::commit_shell_classification_builtin(bb)
                .map_err(|source| PrepassExecutionError::ShellClassification { source })
        },
    )?;
    // PrePass::PaintSegmentation — host built-in (sub-step 15 / AC-14).
    // Runs AFTER ShellClassification (needs annotated SliceIR) and BEFORE
    // SupportGeometry (support geometry reads the colour-resolved SliceIR).
    // Requires: slice_ir + region_map. Writes back via replace_slice_ir.
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::PaintSegmentation",
        "host:paint_segmentation",
        |bb| bb.slice_ir().is_some() && bb.region_map().is_some(),
        |bb| {
            let mesh = bb.mesh().clone();
            let slice_ir = bb.slice_ir().expect("guarded by should_run").clone();
            let region_map = bb.region_map().expect("guarded by should_run").clone();
            let new_slice_ir = slicer_core::algos::paint_segmentation::execute_paint_segmentation(
                mesh, slice_ir, region_map,
            )
            .map_err(|e| PrepassExecutionError::PaintSegmentation {
                message: format!("{e:?}"),
            })?;
            bb.replace_slice_ir(new_slice_ir)
                .map_err(|source| PrepassExecutionError::Blackboard {
                    stage_id: "PrePass::PaintSegmentation".to_string(),
                    module_id: "host:paint_segmentation".to_string(),
                    source,
                })
        },
    )?;
    // PrePass::SupportGeometry — host built-in. Moved from the pre-RegionMap
    // position so it can consume SliceIR (Commit 4 will replace the
    // collect_polygons_at_z stub with real SliceIR reads). For Commit 2,
    // SupportGeometry still uses the stub; the relocation is structural.
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::SupportGeometry",
        "host:support_geometry",
        |bb| {
            bb.support_geometry().is_none() && bb.layer_plan().is_some() && bb.slice_ir().is_some()
        },
        |bb| {
            crate::builtins::support_geometry_producer::commit_support_geometry_builtin(bb)
                .map_err(|source| PrepassExecutionError::SupportGeometry { source })
        },
    )?;
    // PrePass::LightningTreeGen — host built-in (packet 137). Skipped
    // (no commit) when the print's `sparse_fill_holder` is not
    // `lightning-infill` — the zero-cost promise from ADR-0029. The
    // `default_resolved_config` is captured by reference in the closure
    // below; it lives for the duration of this `execute_prepass_*` call.
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::LightningTreeGen",
        "host:lightning_tree",
        |bb| {
            bb.lightning_tree_ir().is_none()
                && default_resolved_config.sparse_fill_holder == "lightning-infill"
        },
        |bb| {
            crate::builtins::lightning_tree_producer::commit_lightning_tree_ir_builtin(
                bb,
                default_resolved_config,
            )
            .map_err(|source| PrepassExecutionError::LightningTree { source })
        },
    )?;
    // Phase-2: late stages that require RegionMap.
    let late_stages: Vec<_> = plan
        .prepass_stages
        .iter()
        .filter(|s| stage_requires_region_map(&s.stage_id))
        .collect();
    if !late_stages.is_empty() {
        let late_plan = ExecutionPlan {
            prepass_stages: late_stages.into_iter().cloned().collect(),
            ..plan.clone()
        };
        let late_audits = execute_prepass_with_instrumentation(
            &late_plan,
            blackboard,
            runner,
            instrumentation,
            wasm_handles,
        )?;
        audits.extend(late_audits);
    }
    Ok(audits)
}

/// Run one host-built-in prepass stage behind the shared instrument bracket.
///
/// Owns the uniform bracket the six built-ins previously hand-rolled:
/// the `should_run` produce-guard, the `estimated_size` byte snapshot, and the
/// [`StageInstrumentationGuard`] start/finish pair. The stage's own work — and
/// its blackboard commit — lives in `execute`. Commit stays **in-stage**: the
/// built-ins commit inside their own functions (`commit_slice_ir`,
/// `replace_slice_ir`, `commit_region_map`, …), so they are deliberately not
/// routed through [`commit_stage_output`] (which serves the guest path). See
/// ADR-0001: `replace_slice_ir` has no `PrepassStageOutput` shape, so a single
/// commit path is infeasible.
///
/// On an `Err` from `execute`, `guard` is dropped without `finish`, matching the
/// prior inline behaviour (the `Drop` path emits `on_module_end`/`on_stage_end`
/// but no bytes event).
fn run_builtin_stage(
    blackboard: &mut Blackboard,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    stage_id: &'static str,
    module_id: &'static str,
    should_run: impl FnOnce(&Blackboard) -> bool,
    execute: impl FnOnce(&mut Blackboard) -> Result<(), PrepassExecutionError>,
) -> Result<(), PrepassExecutionError> {
    if should_run(blackboard) {
        let before = blackboard.estimated_size();
        let guard =
            StageInstrumentationGuard::start(instrumentation, stage_id, None, module_id, before);
        execute(blackboard)?;
        guard.finish(blackboard.estimated_size());
    }
    Ok(())
}

/// Ensures all prerequisite IR artifacts are present on the blackboard
/// before a prepass stage is executed.
pub fn ensure_stage_prerequisites(
    stage_id: &StageId,
    blackboard: &Blackboard,
) -> Result<(), PrepassExecutionError> {
    for &slot in required_slots(stage_id) {
        let present = match slot {
            BlackboardPrepassSlot::SurfaceClassification => {
                blackboard.surface_classification().is_some()
            }
            BlackboardPrepassSlot::LayerPlan => blackboard.layer_plan().is_some(),
            BlackboardPrepassSlot::RegionMap => blackboard.region_map().is_some(),
            BlackboardPrepassSlot::SeamPlan => blackboard.seam_plan().is_some(),
            BlackboardPrepassSlot::SupportPlan => blackboard.support_plan().is_some(),
            BlackboardPrepassSlot::SliceIR => blackboard.slice_ir().is_some(),
            BlackboardPrepassSlot::SupportGeometry => blackboard.support_geometry().is_some(),
            BlackboardPrepassSlot::LightningTreeIR => blackboard.lightning_tree_ir().is_some(),
        };

        if !present {
            return Err(PrepassExecutionError::MissingRequiredPrepass {
                stage_id: stage_id.clone(),
                slot,
            });
        }
    }

    Ok(())
}

fn required_slots(stage_id: &StageId) -> &'static [BlackboardPrepassSlot] {
    match stage_id.as_str() {
        "PrePass::MeshAnalysis" => &[],
        "PrePass::LayerPlanning" => &[BlackboardPrepassSlot::SurfaceClassification],
        "PrePass::OverhangAnnotation" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
        ],
        "PrePass::SeamPlanning" => &[BlackboardPrepassSlot::LayerPlan],
        "PrePass::SupportGeometry" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
            BlackboardPrepassSlot::RegionMap,
            BlackboardPrepassSlot::SliceIR,
            BlackboardPrepassSlot::SupportGeometry,
        ],
        "PrePass::LightningTreeGen" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
            BlackboardPrepassSlot::RegionMap,
            BlackboardPrepassSlot::SliceIR,
        ],
        "PrePass::RegionMapping" => &[BlackboardPrepassSlot::LayerPlan],
        // Host-only built-ins. `PrePass::Slice` does NOT self-list (it writes
        // SliceIR; no user-module satisfaction path exists). `PrePass::ShellClassification`
        // lists SliceIR among its reads.
        "PrePass::Slice" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
            BlackboardPrepassSlot::RegionMap,
        ],
        "PrePass::ShellClassification" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
            BlackboardPrepassSlot::RegionMap,
            BlackboardPrepassSlot::SliceIR,
        ],
        _ => &[],
    }
}

/// Returns true if a stage requires RegionMap as a prerequisite.
/// Used to split execution into early (pre-RegionMapping) and late (post-RegionMapping) phases.
fn stage_requires_region_map(stage_id: &StageId) -> bool {
    required_slots(stage_id).contains(&BlackboardPrepassSlot::RegionMap)
}

fn commit_stage_output(
    stage_id: &StageId,
    module_id: &ModuleId,
    blackboard: &mut Blackboard,
    output: PrepassStageOutput,
) -> Result<(), PrepassExecutionError> {
    let result = match output {
        PrepassStageOutput::None => Ok(()),
        PrepassStageOutput::SurfaceClassification(ir) => {
            blackboard.commit_surface_classification(ir)
        }
        PrepassStageOutput::LayerPlan(ir) => blackboard.commit_layer_plan(ir),
        PrepassStageOutput::SeamPlan(ir) => blackboard.commit_seam_plan(ir),
        PrepassStageOutput::SupportPlan(ir) => blackboard.commit_support_plan(ir),
        PrepassStageOutput::RegionMap(ir) => blackboard.commit_region_map(ir),
        PrepassStageOutput::SupportGeometry(ir) => blackboard.commit_support_geometry(ir),
        // Mesh-analysis auxiliary pushes are surfaced for observability
        // but do not commit to the blackboard. The production
        // SurfaceClassificationIR slot is still owned by the host
        // built-in (`mesh_analysis::execute_mesh_analysis`); letting a
        // guest overwrite it here would require a conflict-resolution
        // design that is deliberately out of scope for STEP G.
        PrepassStageOutput::MeshAnalysisAuxiliary(_) => Ok(()),
    };

    result.map_err(|source| PrepassExecutionError::Blackboard {
        stage_id: stage_id.clone(),
        module_id: module_id.clone(),
        source,
    })
}
