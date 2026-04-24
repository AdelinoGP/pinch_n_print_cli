//! Host-built-in `PrePass::RegionMapping` stage (TASK-106).
//!
//! Compiles a [`RegionMapIR`] from the committed [`LayerPlanIR`] and the
//! already-assembled `ExecutionPlan` so per-layer execution can resolve
//! active modules / configs by O(1) lookup (docs/04_host_scheduler.md
//! §"RegionMapIR Compilation", IR 5 in docs/02_ir_schemas.md).
//!
//! Scope for this step: produce one `RegionPlan` per `(layer, region)`
//! pair, snapshotting the region's `ResolvedConfig` and listing the
//! topo-sorted module invocations the scheduler has already bound (with
//! their per-module `ConfigView`). Claim resolution and per-region
//! config-based module disabling are left to later scheduler work —
//! those are higher-level rewrites of the active-modules list, not of
//! the region-map shape.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    LayerPlanIR, ModuleInvocation, RegionKey, RegionMapIR, RegionPlan, SemVer, StageId,
};

use crate::execution_plan::DEFAULT_REGION_MAP_CAP;
use crate::{CompiledStage, ExecutionPlan};

/// Top contributing module/object for overflow diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopContributor {
    /// Object that contributed the most regions.
    pub object_id: String,
    /// Number of regions contributed by this object.
    pub region_count: usize,
    /// Number of layers this object appears on.
    pub layer_count: usize,
}

/// Structured region-mapping failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionMappingError {
    /// `RegionMapIR` entry count exceeded the configured cap.
    CapExceeded {
        /// Computed entry count.
        entry_count: usize,
        /// Configured cap.
        cap: usize,
        /// Top contributing objects sorted by region_count descending.
        top_contributors: Vec<TopContributor>,
        /// Remediation hint.
        remediation: String,
    },
    /// `LayerPlanIR` contained duplicate `(layer_index, object_id, region_id)` keys.
    DuplicateRegionKey {
        /// The offending key.
        key: RegionKey,
    },
}

impl std::fmt::Display for RegionMappingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapExceeded { entry_count, cap, top_contributors, remediation } => {
                write!(f, "region map has {entry_count} entries, exceeding cap of {cap}; ")?;
                if !top_contributors.is_empty() {
                    let contribs: Vec<String> = top_contributors
                        .iter()
                        .map(|c| format!("{}({} regions, {} layers)", c.object_id, c.region_count, c.layer_count))
                        .collect();
                    write!(f, "top contributors: {}; ", contribs.join(", "))?;
                }
                write!(f, "{remediation}")
            }
            Self::DuplicateRegionKey { key } => write!(
                f,
                "layer plan has duplicate active region (layer={}, object='{}', region={})",
                key.global_layer_index, key.object_id, key.region_id
            ),
        }
    }
}

impl std::error::Error for RegionMappingError {}

/// Execute the built-in `PrePass::RegionMapping` stage.
///
/// Iteration is stable: layers, active regions within a layer, and
/// module invocations within a stage are all visited in the order they
/// appear in their source `Vec`s, so repeated invocations over the same
/// inputs produce a `RegionMapIR` with identical content.
pub fn execute_region_mapping(
    layer_plan: &LayerPlanIR,
    plan: &ExecutionPlan,
) -> Result<RegionMapIR, RegionMappingError> {
    execute_region_mapping_with_cap(layer_plan, plan, DEFAULT_REGION_MAP_CAP)
}

/// Same as [`execute_region_mapping`] with a caller-supplied cap.
pub fn execute_region_mapping_with_cap(
    layer_plan: &LayerPlanIR,
    plan: &ExecutionPlan,
    cap: usize,
) -> Result<RegionMapIR, RegionMappingError> {
    // --- Cap check with top-contributor diagnostics (docs/04 normative memory budget) ----
    let mut entry_count = 0usize;
    // Per-object region/layer counters for overflow diagnostics.
    let mut region_counts: HashMap<String, usize> = HashMap::new();
    let mut layer_counts: HashMap<String, usize> = HashMap::new();
    for layer in &layer_plan.global_layers {
        entry_count = entry_count.saturating_add(layer.active_regions.len());
        for region in &layer.active_regions {
            *region_counts.entry(region.object_id.clone()).or_insert(0) += 1;
        }
        layer_counts.insert(layer.index.to_string(), layer.active_regions.len());
    }
    if entry_count > cap {
        // Build top contributors: sort objects by region_count descending, take top 5.
        let mut sorted: Vec<(String, usize)> = region_counts.into_iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
        let top_contributors: Vec<TopContributor> = sorted
            .into_iter()
            .take(5)
            .map(|(object_id, region_count)| {
                let layer_count = layer_counts.len();
                TopContributor { object_id, region_count, layer_count }
            })
            .collect();
        let remediation = "reduce region granularity, raise cap, or split job".to_string();
        return Err(RegionMappingError::CapExceeded {
            entry_count,
            cap,
            top_contributors,
            remediation,
        });
    }

    // --- Precompute per-stage ModuleInvocation lists ------------------
    // These lists are identical across every region in this step
    // (we are not yet applying per-region config disables / claim
    // resolution). Computing them once outside the region loop keeps the
    // inner loop O(regions) instead of O(regions * stages).
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = plan
        .per_layer_stages
        .iter()
        .chain(plan.postpass_stages.iter())
        .map(|stage: &CompiledStage| {
            let invocations = stage
                .modules
                .iter()
                .map(|m| ModuleInvocation {
                    module_id: m.module_id.clone(),
                    config_view: m.config_view.as_ref().clone(),
                })
                .collect::<Vec<_>>();
            (stage.stage_id.clone(), invocations)
        })
        .collect();

    // --- Build entries ------------------------------------------------
    let mut entries: HashMap<RegionKey, RegionPlan> = HashMap::with_capacity(entry_count);
    for layer in &layer_plan.global_layers {
        for region in &layer.active_regions {
            let key = RegionKey {
                global_layer_index: layer.index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
            };

            let mut stage_modules: HashMap<StageId, Vec<ModuleInvocation>> =
                HashMap::with_capacity(stage_invocations.len());
            for (sid, invs) in &stage_invocations {
                stage_modules.insert(sid.clone(), invs.clone());
            }

            let plan_entry = RegionPlan {
                config: region.resolved_config.clone(),
                stage_modules,
            };
            if entries.insert(key.clone(), plan_entry).is_some() {
                return Err(RegionMappingError::DuplicateRegionKey { key });
            }
        }
    }

    Ok(RegionMapIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        entries,
    })
}

/// Commit the built-in region map into the blackboard (idempotent — if a
/// caller has already committed a map, the function is a no-op).
pub fn commit_region_mapping_builtin(
    plan: &ExecutionPlan,
    blackboard: &mut crate::Blackboard,
) -> Result<(), RegionMappingBuiltinError> {
    if blackboard.region_map().is_some() {
        return Ok(());
    }
    let Some(layer_plan) = blackboard.layer_plan().cloned() else {
        return Err(RegionMappingBuiltinError::MissingLayerPlan);
    };
    let ir = execute_region_mapping(layer_plan.as_ref(), plan)
        .map_err(RegionMappingBuiltinError::Mapping)?;
    blackboard
        .commit_region_map(Arc::new(ir))
        .map_err(|source| RegionMappingBuiltinError::Blackboard { source })?;
    Ok(())
}

/// Wrapper error used when the built-in runs on the real prepass path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionMappingBuiltinError {
    /// No `LayerPlanIR` committed to the blackboard yet.
    MissingLayerPlan,
    /// Region mapping itself failed.
    Mapping(RegionMappingError),
    /// Blackboard commit failed (e.g. duplicate commit).
    Blackboard {
        /// Underlying blackboard failure.
        source: crate::BlackboardError,
    },
}

impl std::fmt::Display for RegionMappingBuiltinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLayerPlan => write!(
                f,
                "built-in PrePass::RegionMapping requires a committed LayerPlanIR"
            ),
            Self::Mapping(e) => write!(f, "built-in PrePass::RegionMapping failed: {e}"),
            Self::Blackboard { source } => {
                write!(f, "built-in PrePass::RegionMapping commit failed: {source}")
            }
        }
    }
}

impl std::error::Error for RegionMappingBuiltinError {}
