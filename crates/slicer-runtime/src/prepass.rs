//! PrePass execution contracts.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use rstar::RTree;
use slicer_core::paint_region::{PaintRegionRTreeEntry, PaintRegionRTreeIndex};
pub use slicer_core::{
    FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, PrepassStageOutput,
    SurfaceGroupRecord,
};
use slicer_ir::{
    ConfigKey, ConfigValue, ModuleId, PaintRegionIR, PaintSemantic, ResolvedConfig, StageId,
};

use crate::builtins::region_mapping_producer::{
    commit_region_mapping_builtin, RegionMappingBuiltinError,
};
use crate::config_resolution::resolve_per_paint_semantic_configs;
use crate::instrumentation::{
    NoopInstrumentation, PipelineInstrumentation, StageInstrumentationGuard,
};
use crate::validation::ModuleAccessAudit;
use crate::{Blackboard, BlackboardError, BlackboardPrepassSlot, ExecutionPlan};
use slicer_core::algos::mesh_analysis::{execute_mesh_analysis, MeshAnalysisError};
use slicer_core::algos::paint_segmentation::PaintSegmentationError;
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
    /// The host-built-in `PrePass::SupportGeometry` stage failed.
    SupportGeometry {
        /// Underlying support geometry failure.
        source: SupportGeometryBuiltinError,
    },
    /// The host-built-in `PrePass::PaintSegmentation` stage failed.
    PaintSegmentation {
        /// Underlying paint segmentation failure.
        source: PaintSegmentationError,
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
            Self::SupportGeometry { source } => {
                write!(f, "built-in PrePass::SupportGeometry failed: {source}")
            }
            Self::PaintSegmentation { source } => {
                write!(f, "built-in PrePass::PaintSegmentation failed: {source}")
            }
            Self::Slice { source } => {
                write!(f, "built-in PrePass::Slice failed: {source}")
            }
            Self::ShellClassification { source } => {
                write!(f, "built-in PrePass::ShellClassification failed: {source}")
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
                    instrumentation.on_stage_end(&stage.stage_id, None);
                    return Err(PrepassExecutionError::from(e));
                }
            };
            let runtime_reads: Vec<String> = runner.last_runtime_reads();

            // Determine IR path before committing (output is moved into commit).
            let ir_path = ir_path_for_prepass_output(&output);

            if let Err(e) =
                commit_stage_output(&stage.stage_id, module.module_id(), blackboard, output)
            {
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
        PrepassStageOutput::MeshSegmentation(_) => Some(String::from("MeshSegmentationIR")),
        PrepassStageOutput::LayerPlan(_) => Some(String::from("LayerPlanIR")),
        PrepassStageOutput::SeamPlan(_) => Some(String::from("SeamPlanIR")),
        PrepassStageOutput::SupportPlan(_) => Some(String::from("SupportPlanIR")),
        PrepassStageOutput::PaintRegions(..) => Some(String::from("PaintRegionIR")),
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
pub(crate) fn execute_prepass_with_builtins_configured_instr(
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
    /// Gather paint semantics from the blackboard and resolve per-semantic
    /// config overrides from the raw config source.  Called immediately
    /// before each `commit_region_mapping_builtin` invocation so that any
    /// `PaintRegionIR` committed during phase-1 user-prepass stages is
    /// visible (packet 51 — AC-4 ordering fix).
    fn build_paint_semantic_configs(
        blackboard: &Blackboard,
        default_resolved_config: &ResolvedConfig,
        raw_config_source: &HashMap<ConfigKey, ConfigValue>,
        bounds: &crate::ConfigBoundsIndex,
    ) -> BTreeMap<PaintSemantic, ResolvedConfig> {
        let Some(paint_ir) = blackboard.paint_regions() else {
            return BTreeMap::new();
        };
        let present_semantics: Vec<PaintSemantic> = {
            let mut seen: std::collections::BTreeSet<PaintSemantic> =
                std::collections::BTreeSet::new();
            for layer_map in paint_ir.per_layer.values() {
                for sem in layer_map.semantic_regions.keys() {
                    seen.insert(sem.clone());
                }
            }
            seen.into_iter().collect()
        };
        match resolve_per_paint_semantic_configs(
            default_resolved_config,
            raw_config_source,
            &present_semantics,
            bounds,
        ) {
            Ok((map, warnings)) => {
                for w in warnings {
                    log::warn!(
                        "paint_config: unknown semantic '{}' in config key '{}' — ignored",
                        w.semantic_name,
                        w.key,
                    );
                }
                map
            }
            Err(e) => {
                log::warn!(
                    "paint_config: resolution failed ({}), paint overrides skipped",
                    e,
                );
                BTreeMap::new()
            }
        }
    }

    // Region-mapping runs after `PrePass::LayerPlanning` (user-or-none) and
    // `PrePass::PaintSegmentation` (user-claimed or host built-in fallback),
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
    // Host built-in fallback for PrePass::PaintSegmentation: if no WASM module
    // committed paint regions during phase-1, run the host built-in so that
    // the subsequent RegionMapping sees paint semantics. Guard-based fallback
    // contract per docs/04_host_scheduler.md:704.
    run_builtin_stage(
        blackboard,
        instrumentation,
        "PrePass::PaintSegmentation",
        "host:paint_segmentation",
        |bb| {
            bb.paint_regions().is_none()
                && bb.surface_classification().is_some()
                && bb.layer_plan().is_some()
        },
        |bb| {
            let union_at_harvest = raw_config_source
                .get(&ConfigKey::from("union_paint_regions_at_harvest"))
                .map(|v| matches!(v, ConfigValue::Bool(true)))
                .unwrap_or(true);
            let paint_ir = slicer_core::algos::paint_segmentation::execute_paint_segmentation(
                bb.mesh().clone(),
                // SAFETY: guarded by .is_some() above
                bb.surface_classification().cloned().unwrap(),
                bb.layer_plan().cloned().unwrap(),
                union_at_harvest,
            )
            .map_err(|source| PrepassExecutionError::PaintSegmentation { source })?;
            let rtree = build_paint_region_rtree_index(&paint_ir);
            bb.commit_paint_regions(paint_ir, rtree).map_err(|source| {
                PrepassExecutionError::Blackboard {
                    stage_id: "PrePass::PaintSegmentation".to_string(),
                    module_id: "host:paint_segmentation".to_string(),
                    source,
                }
            })
        },
    )?;
    // Region-mapping: needs LayerPlan; reads any committed PaintRegionIR to
    // resolve per-paint-semantic config overlays into RegionPlan.paint_overrides.
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
            commit_region_mapping_builtin(
                plan,
                bb,
                resolved_configs,
                default_resolved_config,
                paint_semantic_configs,
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
            BlackboardPrepassSlot::MeshSegmentation => blackboard.mesh_segmentation().is_some(),
            BlackboardPrepassSlot::LayerPlan => blackboard.layer_plan().is_some(),
            BlackboardPrepassSlot::PaintRegions => blackboard.paint_regions().is_some(),
            BlackboardPrepassSlot::RegionMap => blackboard.region_map().is_some(),
            BlackboardPrepassSlot::SeamPlan => blackboard.seam_plan().is_some(),
            BlackboardPrepassSlot::SupportPlan => blackboard.support_plan().is_some(),
            BlackboardPrepassSlot::SliceIR => blackboard.slice_ir().is_some(),
            BlackboardPrepassSlot::SupportGeometry => blackboard.support_geometry().is_some(),
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
        "PrePass::SeamPlanning" => &[BlackboardPrepassSlot::LayerPlan],
        "PrePass::SupportGeometry" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
            BlackboardPrepassSlot::RegionMap,
            BlackboardPrepassSlot::SliceIR,
            BlackboardPrepassSlot::SupportGeometry,
        ],
        "PrePass::PaintSegmentation" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
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
            BlackboardPrepassSlot::PaintRegions,
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
        PrepassStageOutput::MeshSegmentation(ir) => blackboard.commit_mesh_segmentation(ir),
        PrepassStageOutput::LayerPlan(ir) => blackboard.commit_layer_plan(ir),
        PrepassStageOutput::SeamPlan(ir) => blackboard.commit_seam_plan(ir),
        PrepassStageOutput::SupportPlan(ir) => blackboard.commit_support_plan(ir),
        PrepassStageOutput::PaintRegions(ir, rtree) => blackboard.commit_paint_regions(ir, rtree),
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

/// Build an `Arc<PaintRegionRTreeIndex>` companion for a `PaintRegionIR`,
/// computing per-region AABBs where `aabb` is `None`.
fn build_paint_region_rtree_index(ir: &PaintRegionIR) -> Arc<PaintRegionRTreeIndex> {
    let mut trees: HashMap<u32, HashMap<PaintSemantic, RTree<PaintRegionRTreeEntry>>> =
        HashMap::new();
    for (&layer_index, layer_map) in &ir.per_layer {
        let mut semantic_map: HashMap<PaintSemantic, RTree<PaintRegionRTreeEntry>> = HashMap::new();
        for (semantic, regions) in &layer_map.semantic_regions {
            let entries: Vec<PaintRegionRTreeEntry> = regions
                .iter()
                .enumerate()
                .map(|(region_index, region)| {
                    let aabb = region.aabb.unwrap_or_default();
                    PaintRegionRTreeEntry {
                        min_x: aabb.min.x as f64,
                        min_y: aabb.min.y as f64,
                        max_x: aabb.max.x as f64,
                        max_y: aabb.max.y as f64,
                        region_index,
                    }
                })
                .collect();
            let tree = if entries.is_empty() {
                RTree::new()
            } else {
                RTree::bulk_load(entries)
            };
            semantic_map.insert(semantic.clone(), tree);
        }
        trees.insert(layer_index, semantic_map);
    }
    Arc::new(PaintRegionRTreeIndex { trees })
}
