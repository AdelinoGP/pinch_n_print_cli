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

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    LayerPlanIR, ModuleInvocation, PaintRegionIR, PaintSemantic, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SemVer, StageId,
};

use crate::config_resolution::paint_semantic_namespace_key;
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
            Self::CapExceeded {
                entry_count,
                cap,
                top_contributors,
                remediation,
            } => {
                write!(
                    f,
                    "region map has {entry_count} entries, exceeding cap of {cap}; "
                )?;
                if !top_contributors.is_empty() {
                    let contribs: Vec<String> = top_contributors
                        .iter()
                        .map(|c| {
                            format!(
                                "{}({} regions, {} layers)",
                                c.object_id, c.region_count, c.layer_count
                            )
                        })
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

/// Apply a paint-semantic `ResolvedConfig` on top of a base `ResolvedConfig`.
///
/// For each field in `overlay` that differs from `ResolvedConfig::default()`,
/// the overlay value is written into `base`. This implements the
/// global → per_object → per_paint_semantic precedence chain: the paint
/// overlay wins over the per-object config for any field it explicitly sets.
fn overlay_resolved(base: ResolvedConfig, overlay: &ResolvedConfig) -> ResolvedConfig {
    let d = ResolvedConfig::default();
    let mut r = base;
    if overlay.layer_height != d.layer_height {
        r.layer_height = overlay.layer_height;
    }
    if overlay.line_width != d.line_width {
        r.line_width = overlay.line_width;
    }
    if overlay.first_layer_height != d.first_layer_height {
        r.first_layer_height = overlay.first_layer_height;
    }
    if overlay.first_layer_line_width != d.first_layer_line_width {
        r.first_layer_line_width = overlay.first_layer_line_width;
    }
    if overlay.wall_count != d.wall_count {
        r.wall_count = overlay.wall_count;
    }
    if overlay.outer_wall_speed != d.outer_wall_speed {
        r.outer_wall_speed = overlay.outer_wall_speed;
    }
    if overlay.inner_wall_speed != d.inner_wall_speed {
        r.inner_wall_speed = overlay.inner_wall_speed;
    }
    if overlay.wall_generator != d.wall_generator {
        r.wall_generator = overlay.wall_generator;
    }
    if overlay.arachne_min_feature_size != d.arachne_min_feature_size {
        r.arachne_min_feature_size = overlay.arachne_min_feature_size;
    }
    if overlay.infill_type != d.infill_type {
        r.infill_type = overlay.infill_type;
    }
    if overlay.infill_density != d.infill_density {
        r.infill_density = overlay.infill_density;
    }
    if overlay.infill_angle != d.infill_angle {
        r.infill_angle = overlay.infill_angle;
    }
    if overlay.infill_speed != d.infill_speed {
        r.infill_speed = overlay.infill_speed;
    }
    if overlay.solid_infill_speed != d.solid_infill_speed {
        r.solid_infill_speed = overlay.solid_infill_speed;
    }
    if overlay.top_shell_layers != d.top_shell_layers {
        r.top_shell_layers = overlay.top_shell_layers;
    }
    if overlay.bottom_shell_layers != d.bottom_shell_layers {
        r.bottom_shell_layers = overlay.bottom_shell_layers;
    }
    if overlay.top_fill_holder != d.top_fill_holder {
        r.top_fill_holder = overlay.top_fill_holder.clone();
    }
    if overlay.bottom_fill_holder != d.bottom_fill_holder {
        r.bottom_fill_holder = overlay.bottom_fill_holder.clone();
    }
    if overlay.bridge_fill_holder != d.bridge_fill_holder {
        r.bridge_fill_holder = overlay.bridge_fill_holder.clone();
    }
    if overlay.sparse_fill_holder != d.sparse_fill_holder {
        r.sparse_fill_holder = overlay.sparse_fill_holder.clone();
    }
    if overlay.support_enabled != d.support_enabled {
        r.support_enabled = overlay.support_enabled;
    }
    if overlay.support_type != d.support_type {
        r.support_type = overlay.support_type;
    }
    if overlay.support_overhang_angle != d.support_overhang_angle {
        r.support_overhang_angle = overlay.support_overhang_angle;
    }
    if overlay.nonplanar_max_angle_deg != d.nonplanar_max_angle_deg {
        r.nonplanar_max_angle_deg = overlay.nonplanar_max_angle_deg;
    }
    if overlay.nonplanar_shell_count != d.nonplanar_shell_count {
        r.nonplanar_shell_count = overlay.nonplanar_shell_count;
    }
    if overlay.nonplanar_amplitude != d.nonplanar_amplitude {
        r.nonplanar_amplitude = overlay.nonplanar_amplitude;
    }
    if overlay.smoothificator_target_height != d.smoothificator_target_height {
        r.smoothificator_target_height = overlay.smoothificator_target_height;
    }
    if overlay.smoothificator_adaptive != d.smoothificator_adaptive {
        r.smoothificator_adaptive = overlay.smoothificator_adaptive;
    }
    // Merge extension keys from overlay into base.
    for (k, v) in &overlay.extensions {
        r.extensions.insert(k.clone(), v.clone());
    }
    r
}

/// Compute overlapping paint semantics for a region at a given layer.
///
/// Returns semantics sorted ascending by `paint_semantic_namespace_key`
/// (spec: lexicographically-last semantic wins because it overlays last).
///
/// When a `SemanticRegion` has an empty `polygons` vec, it is treated as
/// "whole-layer" coverage and unconditionally overlaps the region.
fn overlapping_semantics_for_region(
    global_layer_index: u32,
    paint_regions: &PaintRegionIR,
) -> Vec<PaintSemantic> {
    let layer_map = match paint_regions.per_layer.get(&global_layer_index) {
        None => return Vec::new(),
        Some(lm) => lm,
    };

    let mut overlapping: Vec<PaintSemantic> = layer_map
        .semantic_regions
        .keys()
        .filter(|semantic| {
            let srs = paint_regions.get(global_layer_index, semantic);
            srs.iter().any(|sr| {
                // Empty polygons → unconditional (whole-layer) coverage.
                if sr.polygons.is_empty() {
                    return true;
                }
                // Non-empty polygons → actual geometric intersection check.
                // Since ActiveRegion carries no polygon data at this stage,
                // any non-empty SemanticRegion polygon set is treated as
                // overlapping (the region polygon set is logically the full
                // layer slice for the object, which is not yet materialised
                // at RegionMapping time).
                true
            })
        })
        .cloned()
        .collect();

    overlapping.sort_by_key(|s| paint_semantic_namespace_key(s));
    overlapping
}

/// Execute the built-in `PrePass::RegionMapping` stage.
///
/// Iteration is stable: layers, active regions within a layer, and
/// module invocations within a stage are all visited in the order they
/// appear in their source `Vec`s, so repeated invocations over the same
/// inputs produce a `RegionMapIR` with identical content.
///
/// When `paint_regions` is `None` or `paint_semantic_configs` is empty, the
/// output is bit-identical to the pre-packet path (invariant 9).
pub fn execute_region_mapping(
    layer_plan: &LayerPlanIR,
    plan: &ExecutionPlan,
    paint_regions: Option<&PaintRegionIR>,
    paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
) -> Result<RegionMapIR, RegionMappingError> {
    execute_region_mapping_with_cap(
        layer_plan,
        plan,
        paint_regions,
        paint_semantic_configs,
        DEFAULT_REGION_MAP_CAP,
    )
}

/// Same as [`execute_region_mapping`] with a caller-supplied cap.
pub fn execute_region_mapping_with_cap(
    layer_plan: &LayerPlanIR,
    plan: &ExecutionPlan,
    paint_regions: Option<&PaintRegionIR>,
    paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
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
                TopContributor {
                    object_id,
                    region_count,
                    layer_count,
                }
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

            // Compute paint-semantic overlay (no-op when paint_regions is None).
            let (effective_config, paint_overrides) = if let Some(pr) = paint_regions {
                let semantics = overlapping_semantics_for_region(layer.index, pr);
                if semantics.is_empty() {
                    // No overlap → bit-identical to pre-packet path.
                    (region.resolved_config.clone(), BTreeMap::new())
                } else {
                    // Apply each overlapping semantic in lex-ascending order;
                    // the last semantic in sort order wins.
                    let mut effective = region.resolved_config.clone();
                    let mut overrides: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
                    for sem in &semantics {
                        if let Some(sem_cfg) = paint_semantic_configs.get(sem) {
                            effective = overlay_resolved(effective, sem_cfg);
                            overrides.insert(sem.clone(), sem_cfg.clone());
                        }
                    }
                    (effective, overrides)
                }
            } else {
                // No paint data → bit-identical to pre-packet path.
                (region.resolved_config.clone(), BTreeMap::new())
            };

            let plan_entry = RegionPlan {
                config: effective_config,
                stage_modules,
                paint_overrides,
            };
            if entries.insert(key.clone(), plan_entry).is_some() {
                return Err(RegionMappingError::DuplicateRegionKey { key });
            }
        }
    }

    Ok(RegionMapIR {
        schema_version: SemVer {
            major: 1,
            minor: 1,
            patch: 0,
        },
        entries,
    })
}

/// Commit the built-in region map into the blackboard (idempotent — if a
/// caller has already committed a map, the function is a no-op).
///
/// `resolved_configs` is a per-object map keyed by `object_id` that supplies
/// the authoritative [`ResolvedConfig`] for each object.  When an object has
/// no entry in the map, `default_resolved_config` is used as the fallback.
/// The host (not the module-emitted `region.resolved_config`) is now the
/// stamping authority for `RegionPlan.config`.
///
/// `paint_semantic_configs` is a map of per-paint-semantic resolved configs for
/// overlay.  Pass an empty `BTreeMap` to preserve the pre-packet bit-identical
/// output for the no-paint path.
pub fn commit_region_mapping_builtin(
    plan: &ExecutionPlan,
    blackboard: &mut crate::Blackboard,
    resolved_configs: &BTreeMap<String, ResolvedConfig>,
    default_resolved_config: &ResolvedConfig,
    paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
) -> Result<(), RegionMappingBuiltinError> {
    if blackboard.region_map().is_some() {
        return Ok(());
    }
    let Some(layer_plan) = blackboard.layer_plan().cloned() else {
        return Err(RegionMappingBuiltinError::MissingLayerPlan);
    };
    let paint_regions = blackboard.paint_regions().map(|arc| arc.as_ref());
    let mut ir = execute_region_mapping(
        layer_plan.as_ref(),
        plan,
        paint_regions,
        paint_semantic_configs,
    )
    .map_err(RegionMappingBuiltinError::Mapping)?;

    // Stamp each RegionPlan.config from the per-object map, falling back to
    // the caller-supplied default when the object has no dedicated entry.
    // When the region carries paint overlays (computed by execute_region_mapping
    // above), re-apply them on top of the per-object base config so that the
    // paint override is not clobbered.
    for (key, region_plan) in ir.entries.iter_mut() {
        let base_config = resolved_configs
            .get(&key.object_id)
            .cloned()
            .unwrap_or_else(|| default_resolved_config.clone());
        if region_plan.paint_overrides.is_empty() {
            // No paint data → bit-identical to pre-packet path.
            region_plan.config = base_config;
        } else {
            // Re-apply each paint overlay on top of the (freshly resolved)
            // per-object base config, in lex-ascending order (same order used
            // in execute_region_mapping) so the result is deterministic.
            let mut effective = base_config;
            for sem_cfg in region_plan.paint_overrides.values() {
                effective = overlay_resolved(effective, sem_cfg);
            }
            region_plan.config = effective;
        }
    }

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
