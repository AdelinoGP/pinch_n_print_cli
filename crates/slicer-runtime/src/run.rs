//! Library entry point for one-shot slicing. Extracted from main.rs::HostCommands::Run.

/// Default for the `use_relative_e_distances` host config key (M83 relative-E)
/// when the user does not set it. Mirrored in `docs/config/host-keys.toml`
/// (`[host_runtime]`) and locked by `gcode_emit::host_keys_doc_lock`.
pub const DEFAULT_USE_RELATIVE_E_DISTANCES: bool = true;

/// Default for the `gcode_add_line_number` host config key when the user does
/// not set it. Canonical declares it `coBool` default `0` (`PrintConfig.cpp`)
/// and its only behavioural read is the export-time post-processor
/// `gcode_add_line_number` (`PostProcessor.cpp`). Read straight from the
/// CLI/JSON config source at this crate's export seam — no `ResolvedConfig`
/// field — so it is mirrored in `docs/config/host-keys.toml`
/// (`[host_runtime]`) and locked by the `host_keys_doc_lock` test.
pub const DEFAULT_GCODE_ADD_LINE_NUMBER: bool = false;

/// Default for the `post_process` host config key when the user does not set
/// it: no scripts, so the exported artifact is byte-identical. Canonical
/// declares it `coStrings` default `{}` (`PrintConfig.cpp`) and runs the listed
/// commands at export time (`PostProcessor.cpp::run_post_process_scripts`).
/// Read straight from the CLI/JSON config source at this crate's export seam —
/// no `ResolvedConfig` field — so it is mirrored in
/// `docs/config/host-keys.toml` (`[host_runtime]`) and locked by the
/// `host_keys_doc_lock` test.
pub const DEFAULT_POST_PROCESS: &[&str] = &[];

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::time::Instant;

use slicer_config::resolution::{
    query_z_grid, resolve_scope_stack, resolved_target_key, ResolutionTarget,
    ResolvedObjectLayerConfig,
};
use slicer_config::{
    ConfigIngestor, ConfigSchemaRegistry, ConfigScope, ExpansionContext, IngestionWarning,
    RegistryWarning, ScopedConfig,
};
use slicer_ir::{
    ConfigKey, ConfigValue, MeshIR, ModifierKind, PaintSemantic, PaintValue, ResolvedConfig,
    StageId,
};
use slicer_sdk::traits::LayerPlanningObject;

/// Parse Orca-style 1-indexed support filament selections into the runtime's
/// 0-indexed tool selection. Missing, zero, invalid, and out-of-range values
/// retain the default tool 0 behavior.
///
/// Callers must pass a typed resolved config map, not the authored source map.
/// Also derives [`SupportToolSelection::tool_count`] from that map using the
/// `filament_density` list length.
///
/// [`SupportToolSelection::tool_count`]: crate::layer_executor::SupportToolSelection::tool_count
pub fn parse_support_tool_selection<K>(
    config_source: &std::collections::HashMap<K, ConfigValue>,
) -> crate::layer_executor::SupportToolSelection
where
    K: std::borrow::Borrow<str> + Eq + std::hash::Hash,
{
    let rebase = |key: &str| match config_source.get(key) {
        Some(ConfigValue::Int(value)) if *value >= 1 => value
            .checked_sub(1)
            .and_then(|rebased| u32::try_from(rebased).ok())
            .unwrap_or(0),
        _ => 0,
    };
    // `filament_density` is documented "one entry per filament … indexed by
    // extruder/tool id" and is the repo's only per-tool count carrier. A bare
    // scalar resolves to a one-element list (same rule as `extract_float_list`),
    // and an absent key means a single-tool machine.
    let tool_count = match config_source.get("filament_density") {
        Some(ConfigValue::List(items)) => u32::try_from(items.len()).unwrap_or(u32::MAX).max(1),
        _ => 1,
    };
    // Canonical `support_interface_not_for_body` (`PrintConfig.cpp` coBool,
    // default true; `ToolOrdering::collect_extruders` + `GCode::process_layer`
    // in `ToolOrdering.cpp` / `GCode.cpp`). Bool wins; Int/String spellings
    // are honoured so an explicit false never silently decays to the default.
    let support_interface_not_for_body = match config_source.get("support_interface_not_for_body") {
        Some(ConfigValue::Bool(b)) => *b,
        Some(ConfigValue::Int(v)) => *v != 0,
        Some(ConfigValue::String(s)) if s.eq_ignore_ascii_case("true") || s == "1" => true,
        Some(ConfigValue::String(s)) if s.eq_ignore_ascii_case("false") || s == "0" => false,
        _ => true,
    };
    let support_tool = rebase("support_filament");
    let interface_tool = rebase("support_interface_filament");
    // Only an explicitly configured interface filament engages the gate
    // (canonical `process_layer` arms on a configured interface extruder; a
    // default profile must never change tool assignment). When the gate is
    // on, the body collides with the interface tool, and another tool
    // exists, advance the body to the smallest configured tool that is not
    // the interface tool. This is the PnP simplification of canonical's
    // `get_next_extruder` flush-volume ordering (no flush matrix exists here)
    // and deliberately ignores the `WipingExtrusions::mark_wiping_extrusions`
    // override arm, which has no port analogue (named non-borrow).
    let interface_explicit = matches!(
        config_source.get("support_interface_filament"),
        Some(ConfigValue::Int(v)) if *v >= 1
    );
    let support_tool = if support_interface_not_for_body
        && interface_explicit
        && support_tool == interface_tool
        && tool_count > 1
    {
        (0..tool_count)
            .find(|t| *t != interface_tool)
            .unwrap_or(support_tool)
    } else {
        support_tool
    };
    crate::layer_executor::SupportToolSelection {
        support_tool,
        interface_tool,
        support_interface_not_for_body,
        tool_count,
        feature_filaments: parse_feature_filament_selection(&config_source, tool_count),
    }
}

/// Parse the six canonical feature-filament selectors (`*_filament_id`,
/// coInt, default 0 = Default/inherit) into explicit 0-based tools.
///
/// A configured value N ≥ 1 selects tool N−1, clamped to the configured tool
/// count (the canonical `clamp_feature_filament_to_valid` shape — unlike the
/// support selectors above, which reject out-of-range values to tool 0, an
/// out-of-range explicit selector clamps to the last configured tool).
/// A 0, absent, negative, or non-`Int` spelling means inherit (`None`): the
/// existing paint/variant/spatial/modifier resolution decides, so a default
/// profile never changes any tool assignment.
fn parse_feature_filament_selection<K>(
    config_source: &std::collections::HashMap<K, ConfigValue>,
    tool_count: u32,
) -> crate::layer_executor::FeatureFilamentSelection
where
    K: std::borrow::Borrow<str> + Eq + std::hash::Hash,
{
    let selector = |key: &str| match config_source.get(key) {
        Some(ConfigValue::Int(value)) if *value >= 1 => {
            let rebased = u32::try_from(value.saturating_sub(1)).unwrap_or(u32::MAX);
            Some(rebased.min(tool_count.saturating_sub(1)))
        }
        _ => None,
    };
    crate::layer_executor::FeatureFilamentSelection {
        sparse: selector("sparse_infill_filament_id"),
        internal_solid: selector("internal_solid_filament_id"),
        top_surface: selector("top_surface_filament_id"),
        bottom_surface: selector("bottom_surface_filament_id"),
        inner_wall: selector("inner_wall_filament_id"),
        outer_wall: selector("outer_wall_filament_id"),
    }
}

fn resolved_host_options(
    config: &ResolvedConfig,
) -> (bool, crate::layer_executor::SupportToolSelection) {
    let map = config.to_config_map();
    let relative = match map.get("use_relative_e_distances") {
        Some(ConfigValue::Bool(value)) => *value,
        _ => DEFAULT_USE_RELATIVE_E_DISTANCES,
    };
    (relative, parse_support_tool_selection(&map))
}

use crate::config_resolution::{validate_support_layer_heights, ConfigBoundsIndex};
use crate::dag::Producer;
use crate::execution_plan::parse_cli_config_source;
#[cfg(feature = "report")]
use crate::instrumentation::CompositeInstrumentation;
use crate::layer_executor::LayerProgressSink;
use crate::module_search_path::assemble_search_roots;
use crate::pipeline::{
    run_pipeline_with_instrumentation_authority, run_pipeline_with_raw_config_authority,
    PipelineConfig, PipelineStageRunners,
};
use crate::preparation_lifecycle::PreparationVisualCaptureError;
use crate::prepass::ConfigExpansionAuthority;
use crate::profiling_report::{ProfileAggregator, ProfileSummary};
use crate::progress_events::{
    JsonLinesEmitter, NullEmitter, ProgressError, ProgressEvent, ProgressEventEmitter,
    ProgressPhase, ProgressStatus, RuntimeProgressSink, SliceEventCollector,
};
use crate::progress_instrumentation::{now_unix_ms, ProgressPipelineInstrumentation, ProgressTier};
#[cfg(feature = "report")]
use crate::report::{allocator as report_alloc, Collector};
use crate::validation::{
    validate_startup_dag_with_configured_holders, DagValidationPass, SchedulerError, StageDag,
};
use slicer_gcode::{
    estimate_print, DefaultGCodeEmitter, DefaultGCodeSerializer, EstimatorLimits, GcodeFlavor,
};
use slicer_ir::resolved_config::resolve_support_line_width_mm;
use slicer_wasm_host::build_live_execution_plan;
use slicer_wasm_host::execution_plan_live::load_live_modules_for_plan_manifest_first;
use slicer_wasm_host::WasmRuntimeDispatcher;

fn typed_global_config(scoped: &ScopedConfig) -> std::collections::HashMap<ConfigKey, ConfigValue> {
    scoped
        .global()
        .into_iter()
        .flat_map(|delta| delta.iter())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub(crate) const RESOLVED_PAINT_PREFIX: &str = "\0resolved-paint:";
pub(crate) const RESOLVED_TOOL_PREFIX: &str = "\0resolved-tool:";

fn paint_semantic_name(semantic: &PaintSemantic) -> String {
    match semantic {
        PaintSemantic::Material => "material".to_owned(),
        PaintSemantic::FuzzySkin => "fuzzy_skin".to_owned(),
        PaintSemantic::SupportEnforcer => "support_enforcer".to_owned(),
        PaintSemantic::SupportBlocker => "support_blocker".to_owned(),
        PaintSemantic::Custom(name) => name.clone(),
    }
}

fn ordered_subsets(values: &[String]) -> Vec<Vec<String>> {
    let mut subsets = vec![Vec::new()];
    for value in values {
        let additions: Vec<Vec<String>> = subsets
            .iter()
            .map(|subset| {
                let mut next = subset.clone();
                next.push(value.clone());
                next
            })
            .collect();
        subsets.extend(additions);
    }
    subsets
}

fn first_out_of_bounds_value(
    value: &ConfigValue,
    min: Option<f64>,
    max: Option<f64>,
) -> Option<f64> {
    let numeric = match value {
        ConfigValue::Float(value) => Some(*value),
        ConfigValue::Int(value) => Some(*value as f64),
        ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        } => Some(*value),
        ConfigValue::List(values) => {
            return values
                .iter()
                .find_map(|value| first_out_of_bounds_value(value, min, max));
        }
        _ => None,
    };
    numeric
        .filter(|value| min.is_some_and(|min| *value < min) || max.is_some_and(|max| *value > max))
}

fn validate_modifier_deltas(
    registry: &ConfigSchemaRegistry,
    scoped: &ScopedConfig,
) -> Result<(), SliceRunError> {
    for (scope, delta) in &scoped.deltas {
        if !matches!(scope, ConfigScope::Modifier { .. }) {
            continue;
        }
        for (key, value) in &delta.values {
            let Some(entry) = registry.entry(key.as_str()) else {
                continue;
            };
            if entry.denied_scopes.iter().any(|scope| scope == "modifier") {
                return Err(SliceRunError(format!(
                    "modifier config ingestion failed: ScopeDenied {{ key: \"{key}\", scope: Modifier }}"
                )));
            }
            if let Some(value) = first_out_of_bounds_value(value, entry.min, entry.max) {
                return Err(SliceRunError(format!(
                    "modifier config ingestion failed: BoundsViolation {{ key: \"{key}\", value: {value}, min: {:?}, max: {:?}, scope: Modifier }}",
                    entry.min, entry.max
                )));
            }
        }
    }
    Ok(())
}

fn merge_model_scopes(
    registry: &ConfigSchemaRegistry,
    scoped: &mut ScopedConfig,
    mesh: &MeshIR,
) -> Result<Vec<IngestionWarning>, SliceRunError> {
    // Model adapters preserve source spelling. Admit every model-authored
    // scope through the same registry boundary before merging authored deltas;
    // explicit flat-config entries retain precedence within each scope.
    let mut model_ingestor = ConfigIngestor::new(registry);
    for object in &mesh.objects {
        model_ingestor
            .ingest_delta(ConfigScope::Object(object.id.clone()), &object.config.data)
            .map_err(|error| SliceRunError(format!("object config ingestion failed: {error}")))?;
        for modifier in &object.modifier_volumes {
            let scope = ConfigScope::Modifier {
                object_id: object.id.clone(),
                modifier_id: modifier.id.clone(),
            };
            let values = modifier
                .config_delta
                .fields
                .iter()
                .filter(|(key, value)| {
                    key.as_str() != "subtype"
                        && !matches!(value, ConfigValue::String(value) if value.is_empty())
                        && !matches!(value, ConfigValue::List(value) if value.is_empty())
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<std::collections::HashMap<_, _>>();

            model_ingestor
                .ingest_delta(scope, &values)
                .map_err(|error| {
                    SliceRunError(format!("modifier config ingestion failed: {error}"))
                })?;
        }
    }
    let model_outcome = model_ingestor.finish();
    validate_modifier_deltas(registry, &model_outcome.scoped)?;
    for (scope, delta) in model_outcome.scoped.deltas {
        let target = scoped.deltas.entry(scope).or_default();
        let explicit_keys: std::collections::BTreeSet<_> = target
            .values
            .keys()
            .map(|key| slicer_config::canonical_config_key(key).to_owned())
            .collect();
        for (key, value) in delta.values {
            let canonical = slicer_config::canonical_config_key(&key);
            if explicit_keys.contains(canonical) {
                continue;
            }
            target.values.entry(key).or_insert(value);
        }
    }

    Ok(model_outcome.warnings)
}

struct RuntimeResolvedScopes {
    default_config: ResolvedConfig,
    target_configs: std::collections::BTreeMap<String, ResolvedConfig>,
    tool_configs: std::collections::BTreeMap<u32, ResolvedConfig>,
    object_layer_configs: Vec<ResolvedObjectLayerConfig>,
    expansion_context: ExpansionContext,
}

fn resolve_runtime_scopes(
    registry: &slicer_config::ConfigSchemaRegistry,
    scoped: &ScopedConfig,
    mesh: &MeshIR,
) -> Result<RuntimeResolvedScopes, SliceRunError> {
    use std::collections::{BTreeMap, BTreeSet};

    let configured_tools: BTreeSet<u32> = scoped
        .deltas
        .keys()
        .filter_map(|scope| match scope {
            ConfigScope::Tool(tool_index) => Some(*tool_index),
            _ => None,
        })
        .collect();
    let seed_context = seed_expansion_context(registry, scoped)?;
    let preliminary_default = resolve_scope_stack(
        registry,
        scoped,
        &ResolutionTarget::default(),
        &seed_context,
    )
    .map_err(|error| SliceRunError(format!("config resolution failed: {error}")))?;
    let mut preliminary_tools = BTreeMap::new();
    for &tool_index in &configured_tools {
        let target = ResolutionTarget {
            tool_index: Some(tool_index),
            ..ResolutionTarget::default()
        };
        preliminary_tools.insert(
            tool_index,
            resolve_scope_stack(registry, scoped, &target, &seed_context)
                .map_err(|error| SliceRunError(format!("config resolution failed: {error}")))?,
        );
    }
    let expansion_context = build_expansion_context(&preliminary_default, &preliminary_tools)?;

    // Resolve every modifier once before composing target subsets. Parameter
    // modifiers are resolved again below as applicable target combinations;
    // support modifiers intentionally route through paint semantics, so this
    // pass still subjects their authored deltas to the same admission, typing,
    // and bounds contract. No resolved config escapes if any modifier fails.
    for object in &mesh.objects {
        for modifier in &object.modifier_volumes {
            let target = ResolutionTarget {
                object_id: object.id.clone(),
                modifier_ids: vec![modifier.id.clone()],
                ..ResolutionTarget::default()
            };
            resolve_scope_stack(registry, scoped, &target, &expansion_context)
                .map_err(|error| SliceRunError(format!("config resolution failed: {error}")))?;
        }
    }

    let default_config = resolve_scope_stack(
        registry,
        scoped,
        &ResolutionTarget::default(),
        &expansion_context,
    )
    .map_err(|error| SliceRunError(format!("config resolution failed: {error}")))?;

    let mut tool_configs = BTreeMap::new();
    for &tool_index in &configured_tools {
        let target = ResolutionTarget {
            tool_index: Some(tool_index),
            ..ResolutionTarget::default()
        };
        tool_configs.insert(
            tool_index,
            resolve_scope_stack(registry, scoped, &target, &expansion_context)
                .map_err(|error| SliceRunError(format!("config resolution failed: {error}")))?,
        );
    }

    let mut target_configs = BTreeMap::new();
    for object in &mesh.objects {
        let mut modifiers: Vec<_> = object
            .modifier_volumes
            .iter()
            .enumerate()
            .filter(|(_, modifier)| {
                !matches!(
                    modifier.kind(),
                    ModifierKind::SupportEnforcer | ModifierKind::SupportBlocker
                )
            })
            .map(|(index, modifier)| {
                (
                    modifier.priority,
                    std::cmp::Reverse(index),
                    modifier.id.clone(),
                )
            })
            .collect();
        modifiers.sort_by_key(|(priority, reverse_index, _)| (*priority, *reverse_index));
        let modifier_ids: Vec<String> = modifiers.into_iter().map(|(_, _, id)| id).collect();

        let mut paint_semantics = BTreeSet::new();
        let mut painted_tools = BTreeSet::new();
        if let Some(paint_data) = &object.paint_data {
            for layer in &paint_data.layers {
                if layer.facet_values.iter().any(Option::is_some) || !layer.strokes.is_empty() {
                    paint_semantics.insert(paint_semantic_name(&layer.semantic));
                }
                for value in layer.facet_values.iter().flatten() {
                    if let PaintValue::ToolIndex(tool_index) = value {
                        painted_tools.insert(*tool_index);
                    }
                }
            }
        }
        for modifier in &object.modifier_volumes {
            match modifier.kind() {
                ModifierKind::SupportEnforcer => {
                    paint_semantics.insert("support_enforcer".to_owned());
                }
                ModifierKind::SupportBlocker => {
                    paint_semantics.insert("support_blocker".to_owned());
                }
                ModifierKind::ParameterModifier => {}
                ModifierKind::NegativePart => {}
            }
        }

        let paint_semantics: Vec<String> = paint_semantics.into_iter().collect();
        let mut target_tools: Vec<Option<u32>> = vec![None];
        target_tools.extend(configured_tools.union(&painted_tools).copied().map(Some));
        for modifiers in ordered_subsets(&modifier_ids) {
            for paints in ordered_subsets(&paint_semantics) {
                for &tool_index in &target_tools {
                    let target = ResolutionTarget {
                        object_id: object.id.clone(),
                        modifier_ids: modifiers.clone(),
                        paint_semantics: paints.clone(),
                        tool_index,
                        layer_top_z: None,
                    };
                    let config = resolve_scope_stack(registry, scoped, &target, &expansion_context)
                        .map_err(|error| {
                            SliceRunError(format!("config resolution failed: {error}"))
                        })?;
                    target_configs.insert(
                        resolved_target_key(&object.id, &modifiers, &paints, tool_index),
                        config,
                    );
                }
            }
        }
    }

    for semantic in scoped.deltas.keys().filter_map(|scope| match scope {
        ConfigScope::PaintSemantic(semantic) => Some(semantic.clone()),
        _ => None,
    }) {
        let target = ResolutionTarget {
            paint_semantics: vec![semantic.clone()],
            ..ResolutionTarget::default()
        };
        let config = resolve_scope_stack(registry, scoped, &target, &expansion_context)
            .map_err(|error| SliceRunError(format!("config resolution failed: {error}")))?;
        target_configs.insert(format!("{RESOLVED_PAINT_PREFIX}{semantic}"), config);
    }
    for (&tool_index, config) in &tool_configs {
        target_configs.insert(
            format!("{RESOLVED_TOOL_PREFIX}{tool_index}"),
            config.clone(),
        );
    }

    let object_heights = mesh
        .objects
        .iter()
        .filter_map(|object| {
            object
                .world_z_extent
                .map(|(z_min, z_max)| (object.id.clone(), (z_max - z_min) as f64))
        })
        .collect();
    let object_layer_configs = query_z_grid(registry, scoped, &object_heights, &expansion_context)
        .map_err(|error| SliceRunError(format!("config resolution failed: {error}")))?;
    // Positional layer-planning contract (`validate_layer_planning_object_configs`):
    // object configs pair with `mesh.objects` index-wise at dispatch, so
    // restore mesh order here — the z-grid query iterates its sorted
    // object-height map and would otherwise swap pairs on multi-object prints.
    let mut object_layer_configs = object_layer_configs;
    let mesh_order: std::collections::HashMap<&str, usize> = mesh
        .objects
        .iter()
        .enumerate()
        .map(|(index, object)| (object.id.as_str(), index))
        .collect();
    object_layer_configs.sort_by_key(|config| {
        mesh_order
            .get(config.object_id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });

    Ok(RuntimeResolvedScopes {
        default_config,
        target_configs,
        tool_configs,
        object_layer_configs,
        expansion_context,
    })
}

/// Preserve the retained authored shape so each host key's declared DSL
/// extractor applies its documented leniency (for example,
/// `extract_float_or_first` reads index 0 of a per-filament list).
/// The matching `UntypedValue` warning is still surfaced by
/// `append_config_startup_diagnostics`, so the fallback remains observable.
/// Scheduler resolution in this composition root uses the retained
/// `ScopedConfig` directly. This transitional module/prepass transport is
/// therefore prefix-free: object, paint, and tool scope prefixes were decoded
/// by the single manifest-first ingestion and are not forwarded for the
/// prepass compatibility adapter to decode again.
fn typed_module_config_source(
    source: &std::collections::HashMap<ConfigKey, ConfigValue>,
    typed_global: &std::collections::HashMap<ConfigKey, ConfigValue>,
) -> std::collections::HashMap<ConfigKey, ConfigValue> {
    const SCOPED_PREFIXES: [&str; 3] = ["object_config:", "paint_config:", "tool_config:"];

    let mut compatibility: std::collections::HashMap<_, _> = source
        .iter()
        .filter(|(key, _)| !SCOPED_PREFIXES.iter().any(|prefix| key.starts_with(prefix)))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    for (key, value) in typed_global {
        compatibility.insert(key.clone(), value.clone());
    }
    debug_assert!(
        compatibility
            .keys()
            .all(|key| !SCOPED_PREFIXES.iter().any(|prefix| key.starts_with(prefix))),
        "module/prepass config transport must not contain raw scope prefixes"
    );
    compatibility
}

/// Build the seed `ExpansionContext` before any scope stack is resolved.
///
/// An authored `nozzle_diameter` always wins. When it is unauthored the
/// fallback comes from the assembled registry's reconciled default for the
/// key — never a code literal — so a registry-declared machine default (for
/// example the 0.4 mm default in seven-plus core-module manifests) reaches
/// expansion exactly like an authored value would.
fn seed_expansion_context(
    registry: &slicer_config::ConfigSchemaRegistry,
    scoped: &ScopedConfig,
) -> Result<ExpansionContext, SliceRunError> {
    let authored = scoped
        .global()
        .and_then(|delta| delta.values.get("nozzle_diameter"));
    let nozzle_diameter_mm = match authored {
        // An authored entry is authoritative: a non-absolute authored value
        // is still a hard error, never a silent fallback.
        Some(authored) => absolute_config_number(authored).ok_or_else(|| {
            SliceRunError(
                "automatic value expansion failed: unknown base key nozzle_diameter required by line_width"
                    .to_string(),
            )
        })?,
        None => registry_numeric_default(registry, "nozzle_diameter").ok_or_else(|| {
            SliceRunError(
                "automatic value expansion failed: unknown base key nozzle_diameter required by line_width"
                    .to_string(),
            )
        })?,
    };
    if !nozzle_diameter_mm.is_finite() || nozzle_diameter_mm <= 0.0 {
        return Err(SliceRunError(format!(
            "automatic value expansion failed: base key nozzle_diameter required by line_width must be positive and finite, got {nozzle_diameter_mm}"
        )));
    }

    Ok(ExpansionContext {
        nozzle_diameter_mm,
        tool_bases: std::collections::BTreeMap::new(),
    })
}

/// Render a registry entry's reconciled default as an absolute number.
///
/// The registry stores each declaration's default as its wire string; a
/// numeric key's fallback is that string parsed per the entry's declared field
/// type. Accepts exactly the shapes [`absolute_config_number`] accepts for an
/// authored value, so a fallback and an authored entry resolve identically.
/// Returns `None` when the key is undeclared, carries no default, or declares a
/// type that cannot supply an absolute number.
fn registry_numeric_default(
    registry: &slicer_config::ConfigSchemaRegistry,
    key: &str,
) -> Option<f64> {
    let entry = registry.entry(key)?;
    let default = entry.default.as_deref()?.trim();
    match entry.field_type.as_str() {
        "float" => default.parse::<f64>().ok(),
        "int" => default.parse::<i64>().ok().map(|value| value as f64),
        "float_or_percent" => default.parse::<f64>().ok(),
        _ => None,
    }
}

fn absolute_config_number(value: &ConfigValue) -> Option<f64> {
    match value {
        ConfigValue::Float(value) => Some(*value),
        ConfigValue::Int(value) => Some(*value as f64),
        ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        } => Some(*value),
        // Orca wire shape leniency, matching `extract_float_or_first`: a real
        // `project_settings.config` stores scalar options as JSON arrays of
        // strings (e.g. `["0.4"]`), so a list resolves through its first
        // element and a numeric string parses. A percent form stays
        // non-absolute and remains a hard error.
        ConfigValue::List(values) => values.first().and_then(absolute_config_number),
        ConfigValue::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn build_expansion_context(
    global: &slicer_ir::ResolvedConfig,
    per_tool: &std::collections::BTreeMap<u32, slicer_ir::ResolvedConfig>,
) -> Result<ExpansionContext, SliceRunError> {
    let global_values = global.to_config_map();
    let nozzle_diameter_mm = global_values
        .get("nozzle_diameter")
        .and_then(absolute_config_number)
        .ok_or_else(|| {
            SliceRunError(
                "automatic value expansion failed: unknown base key nozzle_diameter required by line_width"
                    .to_string(),
            )
        })?;
    if !nozzle_diameter_mm.is_finite() || nozzle_diameter_mm <= 0.0 {
        return Err(SliceRunError(format!(
            "automatic value expansion failed: base key nozzle_diameter required by line_width must be positive and finite, got {nozzle_diameter_mm}"
        )));
    }

    let tool_bases = per_tool
        .iter()
        .map(|(&tool_index, config)| {
            let bases = config
                .to_config_map()
                .into_iter()
                .filter_map(|(key, value)| absolute_config_number(&value).map(|value| (key, value)))
                .collect();
            (tool_index, bases)
        })
        .collect();

    Ok(ExpansionContext {
        nozzle_diameter_mm,
        tool_bases,
    })
}

fn overlay_expanded_global(
    source: &std::collections::HashMap<ConfigKey, ConfigValue>,
    expanded_global: &slicer_ir::ResolvedConfig,
) -> std::collections::HashMap<ConfigKey, ConfigValue> {
    let mut overlaid = source.clone();
    for (key, value) in expanded_global.to_config_map() {
        overlaid.insert(key, value);
    }
    overlaid
}

fn layer_planning_objects(object_layers: &[ResolvedObjectLayerConfig]) -> Vec<LayerPlanningObject> {
    object_layers
        .iter()
        .map(|object| {
            let object_id = object.object_id.clone();
            let object_height = object.object_height;
            let layer_height = object.layer_height;
            let first_layer_height = object.first_layer_height;
            let support_raft_layers = object.support_raft_layers;
            let layer_zs = object.layer_z_tops.clone();
            LayerPlanningObject {
                object_id,
                object_height,
                layer_height,
                first_layer_height,
                support_raft_layers,
                layer_zs,
            }
        })
        .collect()
}

fn append_config_startup_diagnostics(
    diagnostics: &mut Vec<crate::manifest::LoadDiagnostic>,
    registry_warnings: &[RegistryWarning],
    ingestion_warnings: &[IngestionWarning],
) {
    for warning in registry_warnings {
        diagnostics.push(crate::manifest::LoadDiagnostic {
            level: crate::manifest::DiagnosticLevel::Warning,
            path: PathBuf::from("<config-registry>"),
            field: None,
            message: format!("{warning:?}"),
        });
    }
    for warning in ingestion_warnings {
        diagnostics.push(crate::manifest::LoadDiagnostic {
            level: crate::manifest::DiagnosticLevel::Warning,
            path: PathBuf::from("<config-ingestion>"),
            field: None,
            message: format!("{warning:?}"),
        });
    }
}

fn emit_host_support_diagnostics(
    sink: &RuntimeProgressSink,
    slice_id: &str,
    audits: &[crate::ModuleAccessAudit],
) {
    for audit in audits {
        for diagnostic in &audit.diagnostics {
            if !matches!(diagnostic.code, 1200..=1202) {
                continue;
            }
            sink.record(ProgressEvent::module_error(
                slice_id.to_string(),
                ProgressPhase::Prepass,
                "PrePass::SupportGeometry".to_string(),
                diagnostic.layer.and_then(|layer| u32::try_from(layer).ok()),
                audit.module_id.clone(),
                now_unix_ms(),
                ProgressError {
                    code: diagnostic.code,
                    message: diagnostic.message.clone(),
                    fatal: false,
                    suggestion: None,
                    reason: None,
                },
            ));
        }
    }
}

/// Validated runtime options derived from CLI arguments.
///
/// Hosted in `run` rather than a CLI module because the runtime library — not
/// the CLI — defines and consumes this contract. The `pnp_cli` binary builds
/// a `SliceRunOptions` value and hands it to [`run_slice`].
#[derive(Debug, Clone)]
pub struct SliceRunOptions {
    /// Pre-loaded mesh IR. Loaded by the caller (e.g., `pnp-cli`) before invoking `run_slice`.
    pub mesh: Arc<MeshIR>,
    /// Display label for the mesh source (file path, "<stdin>", etc.); used in the HTML report.
    pub model_label: String,
    /// Optional path to a JSON configuration file.
    pub config_path: Option<PathBuf>,
    /// Optional path to the output G-code file.
    pub output_path: Option<PathBuf>,
    /// Directories to search for additional modules, in CLI order.
    pub module_dirs: Vec<PathBuf>,
    /// When true, suppress the platform default module search paths.
    pub no_default_module_paths: bool,
    /// When true, disable the integrated-module tier entirely (ADR-0057).
    pub no_integrated_modules: bool,
    /// Optional path to a PNG thumbnail image for the G-code header.
    pub thumbnail: Option<PathBuf>,
    /// Optional path for an HTML slicer report. When the `report` feature is
    /// disabled, supplying a non-`None` value causes [`run_slice`] to return
    /// an error explaining the build was compiled without report support.
    pub report: Option<PathBuf>,
    /// Verbose report mode (per-layer-per-module rows).
    pub report_verbose: bool,
    /// When true, emit per-stage / per-module timing events on the stderr
    /// JSONL stream during the slice (schema version `"1.2.0"`).
    pub instrument_stderr: bool,
    /// When true, meter wasmtime fuel and collect scope marks, then emit one
    /// `profile_summary` event at slice end (ADR-0055).
    ///
    /// Separate from `instrument_stderr` because fuel metering costs
    /// throughput: a plain instrumented run must stay unaffected. Setting this
    /// also builds the shared `WasmEngine` with `consume_fuel`, which is what
    /// makes `profile-enabled` answer `true` to every guest.
    pub profile: bool,
    /// When true, additionally attach each dispatch call's own scope fold to
    /// its `module_complete` event. Requires `profile`; ignored without it.
    ///
    /// Mirrors `report_verbose`: aggregation is always the default because a
    /// benchy emits thousands of `module_complete` events, and this is the
    /// opt-in for finding one pathological layer.
    pub profile_verbose: bool,
    /// When true (the default for `pnp_cli slice`), emit the docs/09 core
    /// progress-event contract (phase/layer/validation/module_error/
    /// slice_complete) as JSONL on stderr. When false
    /// (`--no-progress-events`), nothing is written to stderr, though error
    /// aggregation still runs internally.
    pub progress_events: bool,
    /// Cooperative cancellation flag checked by the slicing pipeline.
    pub cancel_flag: Option<Arc<AtomicBool>>,
    /// Config values derived from the loaded model (e.g. the 3MF project's
    /// `filament_colour`) that seed `config_source` as defaults. An explicit
    /// `--config` key always wins over an override with the same name.
    pub config_overrides: std::collections::HashMap<String, ConfigValue>,
    /// Model-authored layer configuration ranges, parsed once by the caller.
    pub layer_ranges: Vec<slicer_config::LayerRangeInput>,
}

/// Quiet test baseline - `progress_events: false` deliberately differs from
/// `pnp_cli slice`'s CLI default (`true`); production callers set every field explicitly.
impl Default for SliceRunOptions {
    fn default() -> Self {
        Self {
            mesh: Arc::new(MeshIR::default()),
            model_label: String::new(),
            config_path: None,
            output_path: None,
            module_dirs: Vec::new(),
            no_default_module_paths: false,
            no_integrated_modules: false,
            thumbnail: None,
            report: None,
            report_verbose: false,
            instrument_stderr: false,
            profile: false,
            profile_verbose: false,
            progress_events: false,
            cancel_flag: None,
            config_overrides: std::collections::HashMap::new(),
            layer_ranges: Vec::new(),
        }
    }
}

/// Output produced by a successful `run_slice` call.
#[derive(Debug, Clone)]
pub struct SliceOutcome {
    /// The final G-code text.
    pub gcode_text: String,
    /// Number of layers sliced (best-effort: derived from gcode markers).
    pub layer_count: u32,
    /// Wall-clock time of the pipeline in milliseconds.
    pub wallclock_ms: u64,
    /// Run-wide fuel/scope fold, present only when
    /// [`SliceRunOptions::profile`] was set (ADR-0055).
    ///
    /// Returned as well as emitted on the JSONL stream so `pnp_cli` can print
    /// its ranked table without parsing back the stream it just wrote.
    pub profile: Option<crate::profiling_report::ProfileSummary>,
    /// Non-fatal ingestion warnings (retained mode) raised while the authored
    /// configuration was ingested against the assembled registry — e.g.
    /// [`IngestionWarning::UnrecognizedKey`] for genuinely undeclared keys.
    /// In retained mode these keys still reach deltas and resolved config; the
    /// warn-to-drop flip (packet config-scope-resolution_06, Step 6b) consumes
    /// this list instead of dropping silently (docs/22 §4, AC-5).
    pub ingestion_warnings: Vec<IngestionWarning>,
    /// The `RegionMapIR` the prepass committed, when one was committed.
    ///
    /// The authoritative per-region resolved config for the slice that ran,
    /// exposed so a caller can observe per-layer values the emitted G-code does
    /// not carry (the emitter writes one config block per print, not per
    /// region). `None` when no prepass committed a region map.
    pub region_map: Option<Arc<slicer_ir::RegionMapIR>>,
}

/// Error returned by `run_slice`.
///
/// Wraps the underlying cause as a formatted string so the library does not
/// require `anyhow` as a public dependency.
#[derive(Debug)]
pub struct SliceRunError(pub String);

impl std::fmt::Display for SliceRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SliceRunError {}

impl From<String> for SliceRunError {
    fn from(s: String) -> Self {
        SliceRunError(s)
    }
}

impl From<&str> for SliceRunError {
    fn from(s: &str) -> Self {
        SliceRunError(s.to_string())
    }
}

impl From<SliceRunError> for PreparationVisualCaptureError {
    fn from(error: SliceRunError) -> Self {
        Self::Slice(error)
    }
}

/// Collapse a visual-capture failure back to the ordinary `SliceRunError`
/// surface for callers that did not request a visual capture.
fn into_slice_run_error(error: PreparationVisualCaptureError) -> SliceRunError {
    match error {
        PreparationVisualCaptureError::Slice(error) => error,
        other => SliceRunError(other.to_string()),
    }
}

fn num_cpus_guess() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Build the static-DAG snapshot that the HTML report renders in its
/// "Pipeline (DAG)" section. One `StageOut` per stage in canonical
/// `STAGE_ORDER` — empty stages are kept with `modules: []` so the
/// section mirrors the pipeline shape rather than only the populated
/// subset.
#[cfg(feature = "report")]
fn build_report_dag_snapshot(producers: &[&dyn Producer]) -> crate::report::ReportDagSnapshot {
    use slicer_scheduler::dag_cli::{
        run_dag_claims, run_dag_global_edges, run_dag_stage, StageOut,
    };
    use slicer_scheduler::execution_plan::STAGE_ORDER;
    use slicer_scheduler::stage_order::tier_of;

    let stages: Vec<StageOut> = STAGE_ORDER
        .iter()
        .map(|stage_id| {
            run_dag_stage(producers, &(*stage_id).to_string()).unwrap_or_else(|| StageOut {
                id: (*stage_id).to_string(),
                tier: tier_of(stage_id).to_string(),
                modules: Vec::new(),
                serial_edges: Vec::new(),
            })
        })
        .collect();

    crate::report::ReportDagSnapshot {
        stages,
        cross_stage_edges: run_dag_global_edges(producers),
        claims: Some(run_dag_claims(producers)),
    }
}

/// The slice-wide progress-event channel: one emitter, one collector, one
/// `slice_id`, constructed once per `run_slice` call (before validation) so
/// validation events and pipeline events share the same stream.
struct ProgressChannel {
    slice_id: String,
    sink: Arc<RuntimeProgressSink>,
}

/// Build the progress channel. `emit_to_stderr == false`
/// (`--no-progress-events`) swaps the JSONL stderr emitter for a
/// [`NullEmitter`]: the collector still aggregates error counts but nothing
/// reaches stderr.
fn build_progress_channel(
    emit_to_stderr: bool,
    collector: Option<Arc<Mutex<SliceEventCollector>>>,
) -> ProgressChannel {
    let emitter_arc: Arc<dyn ProgressEventEmitter> = if emit_to_stderr {
        Arc::new(JsonLinesEmitter::new(std::io::stderr()))
    } else {
        Arc::new(NullEmitter)
    };
    let collector = collector.unwrap_or_else(|| Arc::new(Mutex::new(SliceEventCollector::new())));
    let sink = Arc::new(RuntimeProgressSink::new(
        emitter_arc,
        Arc::clone(&collector),
    ));
    ProgressChannel {
        slice_id: format!("slice-{}", now_unix_ms()),
        sink,
    }
}

/// Select the pipeline execution path based on report and progress options.
///
/// This is the 4-way instrumentation fork originally in `main.rs`, now a
/// private helper inside `run.rs`. Uses the slice-wide `ProgressChannel`
/// built by [`run_slice`] before validation.
fn freeze_selected_preparation_bindings(
    lifecycle: &crate::preparation_lifecycle::PrintPreparationScope,
    loaded: &slicer_wasm_host::execution_plan_live::LiveModuleLoadOutput,
    registrations: &[slicer_scheduler::IntegratedModuleRegistration],
    requested_views: &[crate::preparation_lifecycle::PreparationViewRequest],
) -> Result<(), PreparationVisualCaptureError> {
    use slicer_wasm_host::preparation::{freeze_native, freeze_wasm, NativePreparationArtifact};

    // Request shape first: empty/duplicate owner or view names are malformed
    // regardless of which modules happen to be selected.
    let mut seen_requests = std::collections::HashSet::new();
    for request in requested_views {
        if request.module_id.is_empty() {
            return Err(PreparationVisualCaptureError::InvalidRequest {
                detail: "empty preparation view owner module id".into(),
            });
        }
        if request.view.is_empty() {
            return Err(PreparationVisualCaptureError::InvalidRequest {
                detail: format!(
                    "empty preparation view name for owner '{}'",
                    request.module_id
                ),
            });
        }
        if !seen_requests.insert((request.module_id.clone(), request.view.clone())) {
            return Err(PreparationVisualCaptureError::InvalidRequest {
                detail: format!(
                    "duplicate preparation view '{}' for owner '{}'",
                    request.view, request.module_id
                ),
            });
        }
    }

    // Stable opaque fingerprint, not a security digest. Native identity covers
    // the actual linked executable, not ASLR-dependent function addresses.
    fn fingerprint(bytes: &[u8]) -> String {
        let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
        format!("{hash:016x}")
    }
    let mut native_identity = None;
    let mut selected = Vec::new();
    for live in &loaded.bindings {
        let module = &live.module;
        if !slicer_schema::STAGES.iter().any(|stage| {
            stage.stage_id == module.stage() && stage.tier_id == slicer_schema::TIER_LAYER
        }) {
            continue;
        }
        let native = module.provenance() == slicer_scheduler::ModuleProvenance::Integrated;
        let manifest = if native {
            registrations
                .iter()
                .find_map(|registration| {
                    let value: toml::Value = registration.manifest_toml.parse().ok()?;
                    (value.get("module")?.get("id")?.as_str()? == module.id())
                        .then(|| registration.manifest_toml.to_owned())
                })
                .ok_or_else(|| {
                    SliceRunError(format!("selected native manifest missing: {}", module.id()))
                })?
        } else {
            std::fs::read_to_string(module.wasm_path().with_extension("toml")).map_err(|error| {
                SliceRunError(format!("selected manifest {}: {error}", module.id()))
            })?
        };
        let declaration = slicer_scheduler::preparation::parse_preparation_declaration(&manifest)
            .map_err(|error| {
            SliceRunError(format!(
                "selected preparation declaration {}: {error}",
                module.id()
            ))
        })?;
        let (binding, artifact_identity) = if native {
            let entry = live.native_entry.ok_or_else(|| {
                SliceRunError(format!("selected native entry missing: {}", module.id()))
            })?;
            let artifact = match lifecycle
                .prepared_native
                .iter()
                .find(|(id, _, _)| id == module.id())
            {
                Some((_, paired, schema)) => NativePreparationArtifact::Prepared {
                    entry: *paired,
                    schema: *schema,
                },
                None => NativePreparationArtifact::Plain(entry),
            };
            if native_identity.is_none() {
                let executable =
                    std::env::current_exe()
                        .and_then(std::fs::read)
                        .map_err(|error| {
                            SliceRunError(format!("selected native artifact identity: {error}"))
                        })?;
                native_identity = Some(fingerprint(&executable));
            }
            (
                freeze_native(module.id().into(), declaration, artifact),
                format!(
                    "native:{}:{}",
                    native_identity.as_ref().expect("native identity"),
                    fingerprint(manifest.as_bytes())
                ),
            )
        } else {
            let component = live.wasm_component.as_ref().ok_or_else(|| {
                SliceRunError(format!("selected component missing: {}", module.id()))
            })?;
            let compiled = component
                .wasmtime_component()
                .serialize()
                .map_err(|error| {
                    SliceRunError(format!(
                        "selected component identity {}: {error}",
                        module.id()
                    ))
                })?;
            (
                freeze_wasm(
                    module.id().into(),
                    declaration,
                    loaded.engine.wasmtime_engine().clone(),
                    component.wasmtime_component().clone(),
                    module.stage().into(),
                ),
                format!("wasm:{}", fingerprint(&compiled)),
            )
        };
        selected.push(crate::preparation_lifecycle::SelectedPreparationBinding {
            module_id: module.id().into(),
            module_version: module.version().to_string(),
            artifact_identity,
            consumer_stage: module.stage().into(),
            binding: binding.map_err(|error| {
                SliceRunError(format!("freeze selected module {}: {error}", module.id()))
            })?,
        });
    }

    // Owner/view declaration resolution happens here, before `set_selected`
    // publishes the print selection and before any module invocation: the
    // frozen declarations are only reachable through these private bindings.
    for request in requested_views {
        let frozen = selected
            .iter()
            .find(|binding| binding.module_id == request.module_id)
            .ok_or_else(|| PreparationVisualCaptureError::UnknownOwner {
                module_id: request.module_id.clone(),
            })?;
        let declaration = frozen.binding.declaration().ok_or_else(|| {
            PreparationVisualCaptureError::UnsupportedOwner {
                module_id: request.module_id.clone(),
            }
        })?;
        if declaration.views().is_empty() {
            return Err(PreparationVisualCaptureError::UnsupportedOwner {
                module_id: request.module_id.clone(),
            });
        }
        if !declaration.views().iter().any(|view| view == &request.view) {
            return Err(PreparationVisualCaptureError::UnknownView {
                module_id: request.module_id.clone(),
                view: request.view.clone(),
            });
        }
    }

    lifecycle.set_selected(selected);
    Ok(())
}

fn run_pipeline_fork(
    opts: &SliceRunOptions,
    channel: &ProgressChannel,
    config: PipelineConfig,
    config_source: &std::collections::HashMap<String, ConfigValue>,
    registry: &slicer_config::ConfigSchemaRegistry,
    scoped_config: &slicer_config::ScopedConfig,
    expansion_context: &ExpansionContext,
    profile: Option<&Arc<ProfileAggregator>>,
    lifecycle: &crate::preparation_lifecycle::PrintPreparationScope,
    #[cfg(feature = "report")] dag_snapshot: Option<crate::report::ReportDagSnapshot>,
) -> Result<crate::pipeline::PipelineOutput, SliceRunError> {
    let sink_arc = Arc::clone(&channel.sink);
    let expansion_authority = ConfigExpansionAuthority {
        registry,
        scoped: scoped_config,
        expansion: expansion_context,
    };

    // Profiling needs the adapter even under `--no-progress-events`: it is the
    // only thing that sees every module bracket. Nothing reaches stderr in that
    // case — the channel's emitter is a `NullEmitter` — so the flag keeps its
    // meaning while `--profile --no-progress-events` still produces a summary
    // for the caller to print.
    let progress_pi = if opts.progress_events || profile.is_some() {
        let tier = if opts.instrument_stderr {
            ProgressTier::Instrumented
        } else {
            ProgressTier::Core
        };
        let sink_dyn: Arc<dyn LayerProgressSink + Send + Sync> =
            Arc::clone(&sink_arc) as Arc<dyn LayerProgressSink + Send + Sync>;
        let pi =
            ProgressPipelineInstrumentation::with_tier(sink_dyn, channel.slice_id.clone(), tier);
        // Profiling rides the progress adapter because that is where every
        // module bracket already lands. It is independent of `tier`: a
        // `--profile` run without `--instrument-stderr` still folds marks, it
        // just does not emit the per-module timing events.
        Some(match profile {
            Some(agg) => pi.with_profiling(Arc::clone(agg), opts.profile_verbose),
            None => pi,
        })
    } else {
        None
    };

    // The registry-driven CONFIG_BLOCK projection is computed once and handed
    // to every arm: the pre-resolved map is the effective config surface, so
    // the production path never falls back to the legacy raw overlay (which
    // leaked undeclared extension keys and the four `config_block = false`
    // keys). `registry` is only borrowed here, so the projection outlives each
    // call without changing any public signature.
    //
    // Claim selection affects dispatch, not the reconciled config schema.
    // Packet 06's projection emits effective values for every non-omitted
    // registered key, including keys declared only by a claim-losing module.
    let config_block = registry.config_block_map(&config.default_resolved_config);

    let result = match (opts.report.as_ref(), progress_pi.as_ref()) {
        #[cfg(feature = "report")]
        (Some(report_path), maybe_progress_pi) => {
            if let Some(parent) = report_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!(
                        "warning: failed to create report parent directory {}: {e}",
                        parent.display()
                    );
                }
            }
            report_alloc::enable();
            let report_collector = Arc::new(Collector::new_with_verbose(
                opts.model_label.clone(),
                opts.report_verbose,
            ));
            if let Some(snap) = dag_snapshot {
                report_collector.set_dag_snapshot(snap);
            }
            let r = if let Some(progress_pi) = maybe_progress_pi {
                let composite = CompositeInstrumentation::new(
                    progress_pi as &dyn crate::instrumentation::PipelineInstrumentation,
                    report_collector.as_ref()
                        as &dyn crate::instrumentation::PipelineInstrumentation,
                );
                run_pipeline_with_instrumentation_authority(
                    config,
                    config_source,
                    sink_arc.as_ref(),
                    &composite,
                    expansion_authority,
                    Some(&config_block),
                    Some(lifecycle),
                )
            } else {
                run_pipeline_with_instrumentation_authority(
                    config,
                    config_source,
                    sink_arc.as_ref(),
                    report_collector.as_ref(),
                    expansion_authority,
                    Some(&config_block),
                    Some(lifecycle),
                )
            };
            report_alloc::disable();
            if let Err(e) = report_collector.finish_and_render_to(report_path) {
                eprintln!("warning: failed to write slicer report: {e}");
            }
            r
        }
        #[cfg(not(feature = "report"))]
        (Some(_), _) => {
            return Err(SliceRunError(
                "--report support not compiled (build with default features or --features report)"
                    .to_string(),
            ));
        }
        (None, Some(progress_pi)) => run_pipeline_with_instrumentation_authority(
            config,
            config_source,
            sink_arc.as_ref(),
            progress_pi,
            expansion_authority,
            Some(&config_block),
            Some(lifecycle),
        ),
        (None, None) => run_pipeline_with_raw_config_authority(
            config,
            config_source,
            sink_arc.as_ref(),
            expansion_authority,
            Some(&config_block),
            Some(lifecycle),
        ),
    };

    result.map_err(|e| {
        if opts
            .cancel_flag
            .as_ref()
            .is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed))
        {
            channel.sink.record(ProgressEvent::cancelled(
                channel.slice_id.clone(),
                now_unix_ms(),
            ));
        }
        SliceRunError(format!("{e}"))
    })
}

/// Whole-print statistics for the `slice_stats` progress event (packet 169).
///
/// Derived in [`run_slice`] from `slicer_gcode::estimate_print` over the
/// final postpass `GCodeIR` plus the resolved config
/// (`filament_density`, `initial_layer_print_height`).
#[derive(Debug, Clone, PartialEq)]
pub struct SliceStatsInputs {
    /// Estimated print time in whole seconds.
    pub gcode_prediction_seconds: u64,
    /// Estimated filament weight in grams; `None` when `filament_density`
    /// is not configured (the event key is then omitted entirely).
    pub gcode_weight_grams: Option<f64>,
    /// Total filament length across all tools, in mm.
    pub gcode_filament_length_mm: f64,
    /// Number of emitted layers (`GCodeIR.metadata.layer_count`).
    pub layer_count: u32,
    /// First layer height in mm (`ResolvedConfig.initial_layer_print_height`).
    pub first_layer_height_mm: f32,
    /// Extruded volume per extruder index, in mm³.
    pub extruded_volume_mm3: std::collections::BTreeMap<u32, f64>,
    /// Number of tool changes in the print.
    pub toolchange_count: u32,
}

/// Production end-of-slice emission path (packet 169 Step 3).
///
/// Records exactly one `slice_stats` event (when `stats` is `Some`, i.e. the
/// slice produced a G-code artifact — including degraded-but-successful
/// runs), strictly followed by exactly one `slice_complete` event built from
/// the sink's [`SliceEventCollector`] aggregate counts. `slice_complete`
/// status stays `Ok` even when degraded — "degraded success" is signalled by
/// the degraded flag plus `non_fatal_error_count` (docs/09 §Required Events).
///
/// Public so integration tests can assert the emitted JSONL stream from the
/// same code path `run_slice` uses in production.
pub fn emit_end_of_slice_events(
    sink: &RuntimeProgressSink,
    slice_id: &str,
    wallclock_ms: u64,
    stats: Option<SliceStatsInputs>,
) {
    if let Some(s) = stats {
        sink.record(ProgressEvent::slice_stats(
            slice_id.to_string(),
            now_unix_ms(),
            s.gcode_prediction_seconds,
            s.gcode_weight_grams,
            s.gcode_filament_length_mm,
            s.layer_count,
            s.first_layer_height_mm,
            s.extruded_volume_mm3,
            s.toolchange_count,
        ));
    }

    let (fatal, non_fatal, degraded) = {
        let collector = sink.collector();
        let c = collector
            .lock()
            .expect("slice event collector mutex poisoned");
        (c.fatal_count(), c.non_fatal_count(), c.is_degraded())
    };
    sink.record(ProgressEvent::slice_complete(
        slice_id.to_string(),
        now_unix_ms(),
        wallclock_ms,
        ProgressStatus::Ok,
        degraded,
        fatal,
        non_fatal,
    ));
}

/// Extensions for the ordinary entry points.
///
/// An integrated module that declares `[preparation]` cannot be frozen from a
/// plain artifact: `freeze_native` rejects it with `MissingExport`. The
/// generated prepared pairs therefore have to travel with every run that may
/// select such a module, not only with runs that explicitly ask for views. The
/// registry accessor returns an empty vector when the integrated
/// `lightning-infill` feature is off, so this stays inert in builds without
/// prepared integrated modules.
fn default_runtime_extensions() -> crate::preparation_lifecycle::SliceRuntimeExtensions {
    crate::preparation_lifecycle::SliceRuntimeExtensions::default()
        .with_prepared_native(slicer_integrated_modules::prepared_native_entries())
}

/// One-shot slice. Drives the pipeline end-to-end and returns the produced G-code.
///
/// Composes the 4-way instrumentation fork (report, progress, both, none)
/// internally based on `opts.report` and `opts.instrument_stderr`.
pub fn run_slice(opts: SliceRunOptions) -> Result<SliceOutcome, SliceRunError> {
    run_slice_with_runtime_extensions(opts, default_runtime_extensions())
}

/// Run the ordinary slice with print-private artifact registrations and hooks.
pub fn run_slice_with_runtime_extensions(
    opts: SliceRunOptions,
    extensions: crate::preparation_lifecycle::SliceRuntimeExtensions,
) -> Result<SliceOutcome, SliceRunError> {
    match run_slice_shared(opts, None, extensions, SliceCompletion::FullSlice)
        .map_err(into_slice_run_error)?
    {
        SliceRunResult::FullSlice(outcome) => Ok(*outcome),
        SliceRunResult::PreparationOnly(_) | SliceRunResult::PreparationVisual(_) => {
            unreachable!("full slice requested")
        }
    }
}

/// Run the full validated startup and committed preparation prefix, snapshot
/// metadata, then dispose without entering any Layer or synthetic consumer.
pub fn run_slice_preparation_capture(
    opts: SliceRunOptions,
    extensions: crate::preparation_lifecycle::SliceRuntimeExtensions,
) -> Result<crate::preparation_lifecycle::PreparationCapture, SliceRunError> {
    match run_slice_shared(opts, None, extensions, SliceCompletion::PreparationOnly)
        .map_err(into_slice_run_error)?
    {
        SliceRunResult::PreparationOnly(capture) => Ok(capture),
        SliceRunResult::FullSlice(_) | SliceRunResult::PreparationVisual(_) => {
            unreachable!("preparation capture requested")
        }
    }
}

/// Run one real prepared print and capture requested typed preparation views
/// plus selected-layer stage taps, through the shared startup prefix.
///
/// The requested views live on `extensions` (see
/// [`SliceRuntimeExtensions::with_preparation_views`]); `layers` and
/// `stage_taps` select the render/execution evidence. Preparation runs exactly
/// once: this facade never starts an independent preparation-only slice and a
/// second slice to compare output.
///
/// Request shape, owner/view declarations and layer/eligibility resolution all
/// fail closed before module invocation. Postpass taps are rejected explicitly
/// until the whole-print capture closure is forwarded (Step 13).
///
/// [`SliceRuntimeExtensions::with_preparation_views`]:
///     crate::preparation_lifecycle::SliceRuntimeExtensions::with_preparation_views
pub fn run_slice_preparation_visual_capture(
    opts: SliceRunOptions,
    extensions: crate::preparation_lifecycle::SliceRuntimeExtensions,
    layers: Vec<u32>,
    stage_taps: Vec<StageId>,
) -> Result<
    crate::preparation_lifecycle::PreparationVisualCapture,
    crate::preparation_lifecycle::PreparationVisualCaptureError,
> {
    validate_preparation_visual_request(&layers, &stage_taps)?;
    match run_slice_shared(
        opts,
        None,
        extensions,
        SliceCompletion::PreparationVisual { layers, stage_taps },
    )? {
        SliceRunResult::PreparationVisual(capture) => Ok(capture),
        SliceRunResult::FullSlice(_) | SliceRunResult::PreparationOnly(_) => {
            unreachable!("preparation visual capture requested")
        }
    }
}

/// Reject malformed visual requests before the slice starts.
///
/// Owner/view declaration and owner support are validated at freeze time
/// (`freeze_selected_preparation_bindings`), layer and eligibility resolution
/// after the committed selection. This function owns only request-shape and
/// tap-class checks, which need no scheduler state.
fn validate_preparation_visual_request(
    layers: &[u32],
    stage_taps: &[StageId],
) -> Result<(), PreparationVisualCaptureError> {
    use crate::preparation_lifecycle::PreparationVisualCaptureError as VisualError;

    if layers.is_empty() {
        return Err(VisualError::InvalidRequest {
            detail: "no global layer indices requested".into(),
        });
    }
    let mut seen_layers = std::collections::HashSet::new();
    for &global_layer_index in layers {
        if !seen_layers.insert(global_layer_index) {
            return Err(VisualError::InvalidRequest {
                detail: format!("duplicate global layer index {global_layer_index}"),
            });
        }
    }

    let mut seen_taps = std::collections::HashSet::new();
    for tap in stage_taps {
        if tap.is_empty() {
            return Err(VisualError::InvalidRequest {
                detail: "empty stage tap".into(),
            });
        }
        if !seen_taps.insert(tap.as_str()) {
            return Err(VisualError::InvalidRequest {
                detail: format!("duplicate stage tap '{tap}'"),
            });
        }
        if crate::layer_executor::POSTPASS_TAP_STAGE_IDS.contains(&tap.as_str()) {
            return Err(VisualError::Slice(SliceRunError(format!(
                "preparation visual capture: whole-print postpass tap '{tap}' is not forwarded by \
                 the Step 12 truncated capture path; Step 13 must route it through the existing \
                 whole-print closure (seam: crates/slicer-runtime/src/pipeline.rs::\
                 run_pipeline_until_preparation's postpass branch over \
                 crates/slicer-runtime/src/layer_executor.rs::execute_postpass_with_capture)"
            ))));
        }
        let known = crate::layer_executor::SUPPORTED_TAP_STAGE_IDS.contains(&tap.as_str())
            || crate::layer_executor::BLACKBOARD_TAP_STAGE_IDS.contains(&tap.as_str());
        if !known {
            return Err(VisualError::InvalidRequest {
                detail: format!("unsupported stage tap '{tap}'"),
            });
        }
    }
    Ok(())
}

enum SliceCompletion {
    FullSlice,
    PreparationOnly,
    PreparationVisual {
        layers: Vec<u32>,
        stage_taps: Vec<StageId>,
    },
}

enum SliceRunResult {
    FullSlice(Box<SliceOutcome>),
    PreparationOnly(crate::preparation_lifecycle::PreparationCapture),
    PreparationVisual(crate::preparation_lifecycle::PreparationVisualCapture),
}

/// Test-support entry point that lets callers inspect the same collector used
/// by [`run_slice`], including when the pipeline returns an error.
#[doc(hidden)]
pub fn run_slice_with_collector(
    opts: SliceRunOptions,
    collector: Option<Arc<Mutex<SliceEventCollector>>>,
) -> Result<SliceOutcome, SliceRunError> {
    match run_slice_shared(
        opts,
        collector,
        default_runtime_extensions(),
        SliceCompletion::FullSlice,
    )
    .map_err(into_slice_run_error)?
    {
        SliceRunResult::FullSlice(outcome) => Ok(*outcome),
        SliceRunResult::PreparationOnly(_) | SliceRunResult::PreparationVisual(_) => {
            unreachable!("full slice requested")
        }
    }
}

fn run_slice_shared(
    opts: SliceRunOptions,
    collector: Option<Arc<Mutex<SliceEventCollector>>>,
    extensions: crate::preparation_lifecycle::SliceRuntimeExtensions,
    completion: SliceCompletion,
) -> Result<SliceRunResult, PreparationVisualCaptureError> {
    let t0 = Instant::now();
    let channel = build_progress_channel(opts.progress_events, collector);
    let (extension_registrations, anchored_entities, lifecycle) = extensions.into_run_parts(
        opts.instrument_stderr || opts.report.is_some() || opts.profile,
        matches!(
            &completion,
            SliceCompletion::PreparationOnly | SliceCompletion::PreparationVisual { .. }
        ),
    );
    let mut visual_completion = match &completion {
        SliceCompletion::PreparationVisual { layers, stage_taps } => {
            Some(crate::pipeline::PreparationVisualCompletion {
                layers: layers.clone(),
                stage_taps: stage_taps.clone(),
                parts: None,
                error: None,
            })
        }
        SliceCompletion::FullSlice | SliceCompletion::PreparationOnly => None,
    };

    // Fuel-based profiling (ADR-0055). The aggregator's existence is the single
    // switch: nothing downstream records anything without it. The native sink
    // is installed here too so host built-ins report under the same
    // `polygon_ops::*` vocabulary the guests use — with wall-clock only, since
    // native code is not fuel-metered.
    let profile = opts.profile.then(|| {
        if !crate::profiling_report::begin_native_profiling() {
            eprintln!(
                "warning: another slicer-core profiling sink is already installed; \
                 host built-in attribution will be unavailable this run"
            );
        }
        Arc::new(ProfileAggregator::new())
    });

    // Mesh is pre-loaded by the caller (see SliceRunOptions::mesh).
    let mesh_ir = Arc::clone(&opts.mesh);

    // Parse user-facing JSON config (empty map when not supplied).
    let mut config_source = match opts.config_path.as_ref() {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| SliceRunError(format!("failed to read --config file: {e}")))?;
            parse_cli_config_source(&text)
                .map_err(|e| SliceRunError(format!("failed to parse --config: {e}")))?
        }
        None => std::collections::HashMap::new(),
    };

    // Seed model-derived config (e.g. the 3MF project's filament_colour) as
    // defaults: only fill keys the user did not set explicitly via --config.
    //
    // `post_process` is deliberately excluded. It names host commands to run
    // against the exported artifact, and a 3MF carries the full print config in
    // `project_settings.config`, so honouring it from a model file would let a
    // downloaded model execute arbitrary commands on the slicing machine.
    // Canonical runs the model's value; only an explicit `--config` value is
    // honoured here (deliberate divergence, DEV-206).
    for (key, value) in &opts.config_overrides {
        if key == "post_process" {
            if !config_source.contains_key(key) {
                eprintln!(
                    "warning: ignoring post_process from model metadata; set it in the slice config file instead"
                );
            }
            continue;
        }
        config_source
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    // Insert thumbnail_path into config_source when --thumbnail is supplied.
    if let Some(ref thumb_path) = opts.thumbnail {
        config_source.insert(
            "thumbnail_path".to_string(),
            ConfigValue::String(thumb_path.to_string_lossy().to_string()),
        );
    }

    // Seed the host-injected `slice_has_paint` gate (classic-perimeters.toml
    // `[config.schema.slice_has_paint]`, "Slice contains painted regions
    // (host-injected)"): the module manifest declares this key expecting the
    // host to populate it, but nothing ever did, so `medial_axis_enabled`'s
    // painted-slice gate (added 2026-06-24 for exactly this boostvoronoi
    // instability) was permanently inert — `_config.get_bool("slice_has_paint")`
    // always saw `None` and fell back to `false` regardless of actual paint
    // data. Set `true` whenever any object in the mesh carries paint data;
    // never overrides an explicit user-supplied value.
    if mesh_ir.objects.iter().any(|o| o.paint_data.is_some())
        && !config_source.contains_key("slice_has_paint")
    {
        config_source.insert("slice_has_paint".to_string(), ConfigValue::Bool(true));
    }

    // MMU wipe-tower auto-enable (diagnose 2026-06-24, gap #3). OrcaSlicer turns
    // on a prime/wipe tower automatically for multi-tool prints so colour
    // transitions are purged off-part. Mirror that: if the model paints >= 2
    // distinct tool indices and the user did NOT explicitly set
    // `enable_prime_tower`, enable it. Single-colour / unpainted prints keep the
    // default (false) and are unaffected. (The wipe-tower module is fully
    // implemented and wired; it was simply gated off by the resolved-config
    // default of `false` with no auto-enable signal.)
    if !config_source.contains_key("enable_prime_tower") {
        use std::collections::BTreeSet;
        let mut tools: BTreeSet<u32> = BTreeSet::new();
        for object in &mesh_ir.objects {
            if let Some(pd) = &object.paint_data {
                for layer in &pd.layers {
                    for fv in layer.facet_values.iter().flatten() {
                        if let slicer_ir::PaintValue::ToolIndex(n) = fv {
                            tools.insert(*n);
                        }
                    }
                }
            }
        }
        if tools.len() >= 2 {
            config_source.insert("enable_prime_tower".to_string(), ConfigValue::Bool(true));
        }
    }

    // Discover and plan every module under --module-dir.
    let search_roots = assemble_search_roots(&opts.module_dirs, opts.no_default_module_paths);
    // Config-aware loader: resolves the `perimeter-generator` claim
    // collision (classic-perimeters vs arachne-perimeters) via the user's
    // `wall_generator` config key rather than alphabetical module-id order
    // (packet 112 Step 10 — see `load_live_modules_for_plan_with_config`'s
    // doc comment for the production defect this closes).
    // `opts.profile` reaches every guest through this one engine: it turns on
    // `consume_fuel` *and* is what `profile-enabled` answers with.
    let (mut integrated_regs, mut native_entries) = if opts.no_integrated_modules {
        (Vec::new(), Vec::new())
    } else {
        (
            slicer_integrated_modules::integrated_registrations(),
            slicer_integrated_modules::native_entries(),
        )
    };
    // Suppress only the selected extension's same-ID built-in pair. Claim
    // overlap belongs to the scheduler's config-aware arbitration below.
    let extension_modules =
        slicer_scheduler::load_modules_from_roots_with_integrated(&[], &extension_registrations)
            .map_err(|error| SliceRunError(format!("invalid extension registration: {error:?}")))?;
    let builtins = slicer_scheduler::load_modules_from_roots_with_integrated(&[], &integrated_regs)
        .map_err(|error| SliceRunError(format!("invalid integrated registration: {error:?}")))?;
    let extension_winners = slicer_scheduler::load_modules_from_roots_with_integrated(
        &search_roots,
        &extension_registrations,
    )
    .map_err(|error| SliceRunError(format!("extension provenance discovery: {error:?}")))?;
    let suppressed: std::collections::HashSet<_> = builtins
        .modules
        .iter()
        .filter(|builtin| {
            extension_winners
                .modules
                .iter()
                .filter(|winner| {
                    extension_modules
                        .modules
                        .iter()
                        .any(|candidate| candidate.id() == winner.id())
                })
                .any(|extension| builtin.id() == extension.id())
        })
        .map(|module| module.id().to_owned())
        .collect();
    integrated_regs.retain(|registration| {
        let manifest: toml::Value = registration
            .manifest_toml
            .parse()
            .expect("validated manifest");
        !suppressed.contains(
            manifest["module"]["id"]
                .as_str()
                .expect("validated module id"),
        )
    });
    native_entries.retain(|(id, _)| !suppressed.contains(id));
    integrated_regs.extend(extension_registrations);
    native_entries.extend(
        lifecycle
            .prepared_native
            .iter()
            .map(|(id, entry, _)| (id.clone(), entry.stage)),
    );
    let manifest_first = load_live_modules_for_plan_manifest_first(
        &search_roots,
        num_cpus_guess(),
        &config_source,
        &opts.layer_ranges,
        opts.profile,
        &integrated_regs,
        &native_entries,
    )
    .map_err(|e| {
        SliceRunError(format!(
            "failed to load modules from {} root(s) {:?}: {e}",
            search_roots.len(),
            search_roots
        ))
    })?;
    let registry_warnings = manifest_first.registry_warnings;
    let mut ingestion_warnings = manifest_first.ingestion.warnings;
    let mut scoped_config = manifest_first.ingestion.scoped;
    let typed_global_config = typed_global_config(&scoped_config);
    let mut loaded = manifest_first.live;
    // Resolution runs against the same manifest-first registry that typed the
    // authored deltas. Claim dedup drops a module from dispatch only, never
    // from the config schema, so a claim-losing module's keys stay declared
    // and resolvable instead of failing `admission_set` as undeclared.
    let registry = manifest_first.registry;
    ingestion_warnings.extend(merge_model_scopes(
        &registry,
        &mut scoped_config,
        mesh_ir.as_ref(),
    )?);
    append_config_startup_diagnostics(
        &mut loaded.diagnostics,
        &registry_warnings,
        &ingestion_warnings,
    );
    for diag in &loaded.diagnostics {
        eprintln!(
            "{level:?}: {path}: {msg}",
            level = diag.level,
            path = diag.path.display(),
            msg = diag.message,
        );
    }

    let config_bounds = ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));
    let RuntimeResolvedScopes {
        default_config: default_resolved_config,
        target_configs: resolved_configs_map,
        tool_configs: per_tool_configs_map,
        object_layer_configs,
        expansion_context,
    } = resolve_runtime_scopes(&registry, &scoped_config, mesh_ir.as_ref())?;
    validate_support_layer_heights(&resolved_configs_map)
        .map_err(|e| SliceRunError(format!("{e}")))?;

    // Build-volume height gate. Canonical rejects the same condition in
    // `Print::validate` (`Print.cpp`); here it runs on the object's world-space
    // Z extent before slicing. Per-object config wins so a per-object
    // `printable_height` override is honoured, falling back to the global value.
    // The object's empty target (no modifier / paint / tool) is keyed by the
    // object id itself in the pre-resolved target map.
    for object in &mesh_ir.objects {
        let printable_height = resolved_configs_map
            .get(object.id.as_str())
            .unwrap_or(&default_resolved_config)
            .printable_height;
        slicer_model_io::loader::validate_printable_height(object, printable_height)
            .map_err(|e| SliceRunError(format!("{e}")))?;
    }

    let module_config_source = typed_module_config_source(&config_source, &typed_global_config);
    let expanded_global_source =
        overlay_expanded_global(&module_config_source, &default_resolved_config);
    let layer_planning_objects = layer_planning_objects(&object_layer_configs);
    let dag_modules: Vec<crate::manifest::LoadedModule> =
        loaded.bindings.iter().map(|b| b.module.clone()).collect();

    // Static-DAG snapshot for the HTML report's "Pipeline (DAG)" section.
    // Captured here so it can borrow the same `dag_producers` slice the
    // validator builds below; the snapshot itself stores owned strings so
    // it can outlive that borrow.
    #[cfg(feature = "report")]
    let mut dag_snapshot: Option<crate::report::ReportDagSnapshot> = None;

    // 14-pass startup DAG validation, bracketed by phase_start/phase_complete
    // (validation) on the progress stream (docs/09 §Required Events). Fatal
    // failures emit a validation_error + phase_complete(fatal_error) before
    // bailing; advisories/warnings stay human-readable on stderr (the
    // validation_error wire event is fatal-only by construction).
    {
        use slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION;

        let vstart = Instant::now();
        channel.sink.record(ProgressEvent::phase_start(
            channel.slice_id.clone(),
            ProgressPhase::Validation,
            now_unix_ms(),
        ));
        // Emit the fatal-failure triple (validation_error + fatal
        // phase_complete) for a given stable code/message. The caller bails
        // immediately afterwards.
        let emit_validation_failure = |code: u32, message: &str| {
            channel.sink.record(ProgressEvent::validation_error(
                channel.slice_id.clone(),
                now_unix_ms(),
                ProgressError {
                    code,
                    message: message.to_string(),
                    fatal: true,
                    suggestion: None,
                    reason: None,
                },
            ));
            channel.sink.record(ProgressEvent::phase_complete(
                channel.slice_id.clone(),
                ProgressPhase::Validation,
                now_unix_ms(),
                vstart.elapsed().as_millis() as u64,
                ProgressStatus::FatalError,
            ));
        };

        let mut dag_producers: Vec<&dyn Producer> = crate::runtime_builtins();
        dag_producers.extend(loaded.bindings.iter().map(|b| &b.module as &dyn Producer));

        #[cfg(feature = "report")]
        if opts.report.is_some() {
            dag_snapshot = Some(build_report_dag_snapshot(&dag_producers));
        }

        let mut stage_dags: Vec<StageDag> = Vec::with_capacity(loaded.sorted_stages.len());
        for stage_entry in &loaded.sorted_stages {
            match crate::dag::build_intra_stage_dag(stage_entry.stage_id.clone(), &dag_producers) {
                Ok(nodes) => stage_dags.push(StageDag {
                    stage: stage_entry.stage_id.clone(),
                    nodes,
                }),
                Err(err) => {
                    let msg = format!(
                        "intra-stage DAG construction failed for {}: {err:?}",
                        stage_entry.stage_id
                    );
                    emit_validation_failure(
                        crate::progress_events::VALIDATION_DAG_CONSTRUCTION_CODE,
                        &msg,
                    );
                    return Err(PreparationVisualCaptureError::Slice(SliceRunError(msg)));
                }
            }
        }

        // Build global-scope ClaimHolder entries from each loaded module's
        // declared `claims` so the validator can resolve fill-role-claim
        // owners (claim:sparse-fill, claim:top-fill, claim:bottom-fill,
        // claim:bridge-fill, claim:ironing, etc.). Pre-fix this was an
        // empty Vec which produced startup `MissingDependency` warnings
        // for every fill-role claim — see `docs/specs/infill-fill-partition-plan.md`
        // Phase A2 (the user-reproducible cube slice that exposed it).
        let claim_holders: Vec<crate::validation::ClaimHolder> = dag_modules
            .iter()
            .flat_map(|m| {
                m.claims()
                    .iter()
                    .map(|claim| crate::validation::ClaimHolder {
                        claim: claim.clone(),
                        module_id: m.id().to_string(),
                        scope: crate::validation::ConflictScope::Global,
                    })
            })
            .collect();
        let host_version = crate::manifest::parse_semver(env!("CARGO_PKG_VERSION"))
            .expect("slicer-runtime CARGO_PKG_VERSION must be valid semver");
        let request = crate::validation::DagValidationRequest {
            modules: dag_modules.clone(),
            stage_dags,
            host_ir_schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            host_version,
            claim_holders,
            access_audits: Vec::new(),
        };
        let configured_fill_holders = [
            (
                "claim:top-fill",
                default_resolved_config.top_fill_holder.as_str(),
            ),
            (
                "claim:bottom-fill",
                default_resolved_config.bottom_fill_holder.as_str(),
            ),
            (
                "claim:bridge-fill",
                default_resolved_config.bridge_fill_holder.as_str(),
            ),
            (
                "claim:sparse-fill",
                default_resolved_config.sparse_fill_holder.as_str(),
            ),
        ];
        let report =
            validate_startup_dag_with_configured_holders(&request, &configured_fill_holders);

        let holder_errors: Vec<_> = report
            .errors
            .iter()
            .filter(|diagnostic| {
                matches!(
                    &diagnostic.detail,
                    SchedulerError::UnmatchedClaimHolder { .. }
                        | SchedulerError::ClaimHolderDoesNotDeclareClaim { .. }
                )
            })
            .collect();
        if !holder_errors.is_empty() {
            let detail = holder_errors
                .iter()
                .map(|diagnostic| format!("{:?}", diagnostic.detail))
                .collect::<Vec<_>>()
                .join("; ");
            let msg = format!("startup DAG claim-holder validation failed: {detail}");
            emit_validation_failure(crate::progress_events::VALIDATION_CLAIM_HOLDER_CODE, &msg);
            return Err(PreparationVisualCaptureError::Slice(SliceRunError(msg)));
        }

        let version_errors: Vec<_> = report
            .errors
            .iter()
            .filter(|d| {
                matches!(
                    d.pass,
                    DagValidationPass::IrVersionCompatibility
                        | DagValidationPass::HostVersionCompatibility
                )
            })
            .collect();
        if !version_errors.is_empty() {
            let detail = version_errors
                .iter()
                .map(|d| format!("{:?}", d.detail))
                .collect::<Vec<_>>()
                .join("; ");
            let msg = format!("startup DAG version-compatibility validation failed: {detail}");
            emit_validation_failure(crate::progress_events::VALIDATION_VERSION_COMPAT_CODE, &msg);
            return Err(PreparationVisualCaptureError::Slice(SliceRunError(msg)));
        }

        for diag in &report.errors {
            if matches!(
                diag.pass,
                DagValidationPass::IrVersionCompatibility
                    | DagValidationPass::HostVersionCompatibility
            ) {
                continue;
            }
            eprintln!(
                "warning: startup DAG advisory ({:?}): {:?}",
                diag.pass, diag.detail
            );
        }
        for warning in &report.warnings {
            eprintln!(
                "warning: startup DAG ({:?}): {:?}",
                warning.pass, warning.detail
            );
        }

        channel.sink.record(ProgressEvent::phase_complete(
            channel.slice_id.clone(),
            ProgressPhase::Validation,
            now_unix_ms(),
            vstart.elapsed().as_millis() as u64,
            ProgressStatus::Ok,
        ));
    }

    freeze_selected_preparation_bindings(
        &lifecycle,
        &loaded,
        &integrated_regs,
        lifecycle.preparation_views(),
    )?;

    // Build wasm_handles side-table before consuming bindings.
    let wasm_handles: std::collections::HashMap<
        slicer_ir::ModuleId,
        (
            Arc<slicer_wasm_host::WasmInstancePool>,
            Option<Arc<slicer_wasm_host::WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    > = loaded
        .bindings
        .iter()
        .map(|b| {
            (
                b.module.id().to_string(),
                (
                    Arc::clone(&b.instance_pool),
                    b.wasm_component.clone(),
                    b.native_entry,
                ),
            )
        })
        .collect();

    let plan = build_live_execution_plan(
        loaded.sorted_stages,
        loaded.bindings,
        &default_resolved_config,
        Arc::new(Vec::new()),
        Arc::new(std::collections::HashMap::new()),
        &mut loaded.diagnostics,
    )
    .map_err(|e| SliceRunError(format!("failed to build execution plan: {e}")))?;

    let engine = Arc::clone(&loaded.engine);
    let flavor = match expanded_global_source.get("gcode_flavor") {
        Some(ConfigValue::String(value)) => GcodeFlavor::from_config_str(value),
        _ => GcodeFlavor::Marlin,
    };
    let (relative, support_tools) = resolved_host_options(&default_resolved_config);
    // Canonical `gcode_add_line_number` (`PrintConfig.cpp` coBool default 0).
    // `extract_bool` also accepts an `Int` 0/1, the hand-written-JSON spelling
    // `is_declared_bool_key` cannot classify; an absent or mistyped value keeps
    // the default, matching the sibling read above.
    let gcode_add_line_number = match config_source.get("gcode_add_line_number") {
        Some(value) => slicer_ir::resolved_config::extract_bool("gcode_add_line_number", value)
            .unwrap_or(DEFAULT_GCODE_ADD_LINE_NUMBER),
        None => DEFAULT_GCODE_ADD_LINE_NUMBER,
    };
    // Canonical `post_process` (`PrintConfig.cpp` coStrings): commands run
    // against the exported artifact after the file exists
    // (`PostProcessor.cpp::run_post_process_scripts`). Each script rewrites the
    // file in place, so the export must produce one — a stdout-only run has no
    // file to hand them, and silently skipping configured scripts is worse than
    // saying so.
    let post_process_scripts = parse_post_process_scripts(&config_source)?;
    if !post_process_scripts.is_empty() && opts.output_path.is_none() {
        return Err(PreparationVisualCaptureError::Slice(SliceRunError(
            "post_process requires an output file: the configured scripts rewrite the exported G-code in place, so this run needs --output".to_string(),
        )));
    }
    let nozzle_diameter_mm = match config_source.get("nozzle_diameter") {
        Some(ConfigValue::Float(value)) => *value as f32,
        Some(ConfigValue::Int(value)) => *value as f32,
        _ => 0.4,
    };
    // Canonical `Flow::support_material_flow` fallback chain: authored value →
    // object `line_width` → nozzle diameter.
    let support_line_width_mm = resolve_support_line_width_mm(
        default_resolved_config.support_line_width,
        default_resolved_config.line_width as f32,
        nozzle_diameter_mm,
    );

    // Packet 169 Step 3: capture the estimator inputs the slice_stats event
    // needs before `default_resolved_config` / `per_tool_configs_map` are
    // moved into the pipeline config. Tool diameters mirror the emitter's own
    // `tool_configs` map (the estimator defaults missing tools to 1.75 mm).
    let estimator_limits = EstimatorLimits::from_config(&default_resolved_config);
    let stats_tool_diameters: std::collections::BTreeMap<u32, f32> = per_tool_configs_map
        .iter()
        .map(|(&tool, cfg)| (tool, cfg.filament_diameter))
        .collect();
    let stats_filament_density = default_resolved_config.filament_density.clone();
    let stats_first_layer_height_mm = default_resolved_config.initial_layer_print_height as f32;

    let pipeline_config = PipelineConfig {
        cancel_flag: opts.cancel_flag.clone(),
        mesh_ir,
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(
                WasmRuntimeDispatcher::new(Arc::clone(&engine))
                    .with_layer_planning_objects(layer_planning_objects),
            ),
            layer: Box::new(crate::preparation_lifecycle::PreparedLayerStageRunner::new(
                Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                WasmRuntimeDispatcher::new(Arc::clone(&engine)),
                Arc::clone(&lifecycle),
            )),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(
                // Wire the host `[speeds]` keys (Orca names, mm/s) from the
                // raw config into the emitter's feedrate table; without this
                // every F value was FeedrateConfig::default() scaled by module
                // speed factors.
                DefaultGCodeEmitter::new_with_config(
                    concat!("pnp_cli ", env!("CARGO_PKG_VERSION")).into(),
                    slicer_ir::FeedrateConfig::from_raw_config(&expanded_global_source),
                )
                .with_extrusion_mode(relative)
                .with_resolved_config(default_resolved_config.clone())
                .with_tool_configs(per_tool_configs_map.clone())
                .with_flavor(flavor),
            ),
            serializer: Box::new(
                DefaultGCodeSerializer::with_extrusion_mode(relative)
                    .with_flavor(flavor)
                    .with_support_line_width(support_line_width_mm)
                    .with_manual_filament_change(default_resolved_config.manual_filament_change),
            ),
        },
        support_tools,
        resolved_configs: Arc::new(resolved_configs_map),
        default_resolved_config: Arc::new(default_resolved_config),
        bounds: Arc::new(config_bounds),
        wasm_handles,
        anchored_entities,
    };

    // Run the pipeline through the 4-way instrumentation fork.
    let pipeline_result = if matches!(completion, SliceCompletion::PreparationOnly) {
        crate::pipeline::run_pipeline_until_preparation(
            pipeline_config,
            &expanded_global_source,
            channel.sink.as_ref(),
            &crate::instrumentation::NoopInstrumentation,
            Some(ConfigExpansionAuthority {
                registry: &registry,
                scoped: &scoped_config,
                expansion: &expansion_context,
            }),
            None,
            Some(lifecycle.as_ref()),
            true,
            true,
            None,
        )
        .map_err(|error| SliceRunError(error.to_string()))
    } else if matches!(completion, SliceCompletion::PreparationVisual { .. }) {
        crate::pipeline::run_pipeline_until_preparation(
            pipeline_config,
            &expanded_global_source,
            channel.sink.as_ref(),
            &crate::instrumentation::NoopInstrumentation,
            Some(ConfigExpansionAuthority {
                registry: &registry,
                scoped: &scoped_config,
                expansion: &expansion_context,
            }),
            None,
            Some(lifecycle.as_ref()),
            true,
            false,
            visual_completion.as_mut(),
        )
        .map_err(|error| SliceRunError(error.to_string()))
    } else {
        run_pipeline_fork(
            &opts,
            &channel,
            pipeline_config,
            &expanded_global_source,
            &registry,
            &scoped_config,
            &expansion_context,
            profile.as_ref(),
            lifecycle.as_ref(),
            #[cfg(feature = "report")]
            dag_snapshot,
        )
        .map(Some)
    };

    // Fold the host built-ins' native marks in and stop recording, on both the
    // success and the failure path: leaving the sink recording would let a
    // later profiling-off run in the same process accumulate silently.
    let profile_summary: Option<ProfileSummary> = profile.as_ref().map(|agg| {
        crate::profiling_report::end_native_profiling(agg);
        agg.finish()
    });

    // Report the human channel's suppressed module-log repeats before
    // returning, on both the success and the failure path. `forward_module_logs`
    // collapses identical `(level, message)` pairs to one emission so a
    // per-call module warn cannot drown stderr; this is where the occurrence
    // counts land. The `--instrument-stderr` `module_log` stream is unaffected —
    // it carries every occurrence.
    slicer_wasm_host::dispatch::emit_module_log_repeat_summary();

    if matches!(completion, SliceCompletion::FullSlice) {
        // The pipeline joins module calls and synthetic host work before return.
        // Cleanup belongs to the run, not the lifetime of an outlet-held Arc.
        let cleanup = lifecycle.dispose_after_run();
        if pipeline_result.is_ok() {
            cleanup.map_err(|error| SliceRunError(error.to_string()))?;
        }
        // Preserve an original failed/cancelled pipeline outcome even if cleanup
        // reports an invariant violation.
    }

    let pipeline_output = if let Some(visual) = visual_completion.as_mut() {
        match pipeline_result {
            Ok(Some(_)) => {
                let _ = lifecycle.dispose_after_run();
                return Err(PreparationVisualCaptureError::Slice(SliceRunError(
                    "preparation visual capture: pipeline completed a non-visual output".into(),
                )));
            }
            Ok(None) => {
                if let Some(error) = visual.error.take() {
                    let _ = lifecycle.dispose_after_run();
                    return Err(error);
                }
                let parts = visual.parts.take().unwrap_or_default();
                let capture = lifecycle.capture_visual_and_dispose(
                    &visual.layers,
                    parts.executed_stage_ids,
                    parts.executed_layer_indices,
                    parts.captures,
                );
                match capture {
                    Ok(capture) => return Ok(SliceRunResult::PreparationVisual(capture)),
                    Err(error) => {
                        let _ = lifecycle.dispose_after_run();
                        return Err(error);
                    }
                }
            }
            Err(error) => {
                let typed = visual.error.take();
                // The projection sidecar is populated only by the host's own
                // projection validator, so a module error/trap/cancellation
                // never reaches it; the original typed/pipeline outcome wins.
                let projection =
                    lifecycle
                        .take_first_projection_failure()
                        .map(|(module_id, view, reason)| {
                            PreparationVisualCaptureError::ProjectionRejected {
                                module_id,
                                view,
                                reason,
                            }
                        });
                // Best-effort cleanup; the original typed/projection outcome wins.
                let _ = lifecycle.dispose_after_run();
                return Err(typed
                    .or(projection)
                    .unwrap_or(PreparationVisualCaptureError::Slice(error)));
            }
        }
    } else {
        let Some(pipeline_output) = pipeline_result? else {
            let capture = lifecycle
                .snapshot_and_dispose()
                .map_err(|error| SliceRunError(error.to_string()))?;
            return Ok(SliceRunResult::PreparationOnly(capture));
        };
        pipeline_output
    };

    // Host support aggregation diagnostics are collected with the prepass
    // audit, but must also enter the slice event channel so a successful run
    // is visibly degraded to progress consumers.
    emit_host_support_diagnostics(
        &channel.sink,
        &channel.slice_id,
        &pipeline_output.prepass_audits,
    );

    let wallclock_ms = t0.elapsed().as_millis() as u64;

    // Derive layer_count: count layer-change markers in gcode (best-effort proxy).
    let layer_count = pipeline_output
        .gcode_text
        .lines()
        .filter(|l| l.starts_with(";LAYER_CHANGE") || l.starts_with("; layer"))
        .count() as u32;

    // slice_stats + slice_complete (packet 169 Step 3): slice_stats is
    // emitted whenever the slice produced a G-code artifact (including
    // degraded-but-successful runs), strictly before the success-only,
    // exactly-once slice_complete (docs/09 §Required Events). On fatal
    // failure the stream ends at the error event with neither. The whole-print
    // numbers come from `estimate_print` over the final post-postprocess
    // `GCodeIR` surfaced by `crate::postpass::take_final_gcode_ir` — no
    // estimator math is re-implemented here.
    let stats = crate::postpass::take_final_gcode_ir().map(|ir| {
        let estimate = estimate_print(&ir, &estimator_limits, &stats_tool_diameters);
        // Weight is summed per tool, each priced with its own filament's
        // density (Orca `filament_density` is `coFloats`, one entry per
        // filament — canonical reads it as `filament_density.get_at(m_id)` in
        // `Extruder::filament_density`). A single-density config prices every
        // tool identically, so single-material output is unchanged.
        let weight_grams = (!stats_filament_density.is_empty()).then(|| {
            estimate
                .extruded_volume_mm3
                .iter()
                .map(|(tool, volume)| {
                    let density = stats_filament_density
                        .get(*tool as usize)
                        .or_else(|| stats_filament_density.first())
                        .copied()
                        .unwrap_or_default();
                    (volume / 1000.0) * density
                })
                .sum::<f64>()
        });
        SliceStatsInputs {
            gcode_prediction_seconds: estimate.total_time_s.round() as u64,
            // Weight only when filament_density (g/cm³) is configured; the
            // event key is omitted otherwise (never 0, never null). The
            // serializer's header default density is deliberately not used.
            gcode_weight_grams: weight_grams,
            gcode_filament_length_mm: estimate.filament_length_mm.values().sum(),
            layer_count: ir.metadata.layer_count,
            first_layer_height_mm: stats_first_layer_height_mm,
            extruded_volume_mm3: estimate.extruded_volume_mm3,
            toolchange_count: estimate.toolchange_count,
        }
    });
    // `profile_summary` precedes `slice_stats` / `slice_complete`: the
    // terminal events stay terminal, so a consumer that stops reading at
    // `slice_complete` never misses the profile (ADR-0055).
    if let Some(summary) = profile_summary.as_ref() {
        channel.sink.record(ProgressEvent::profile_summary(
            channel.slice_id.clone(),
            now_unix_ms(),
            wallclock_ms,
            summary.clone(),
        ));
    }
    emit_end_of_slice_events(&channel.sink, &channel.slice_id, wallclock_ms, stats);

    // Canonical's export steps, in canonical order
    // (`BackgroundSlicingProcess::finalize_gcode`): the configured
    // `post_process` scripts rewrite the exported artifact first, then
    // `gcode_add_line_number` numbers it. Both work on the artifact rather than
    // the file the caller writes, so the `--output` file and the stdout path
    // receive identical bytes; both are off by default, so the emitted bytes of
    // a config that sets neither key are unchanged.
    let gcode_text = if post_process_scripts.is_empty() {
        pipeline_output.gcode_text
    } else {
        let output_path = opts.output_path.as_ref().ok_or_else(|| {
            SliceRunError(
                "post_process requires an output file: the configured scripts rewrite the exported G-code in place, so this run needs --output".to_string(),
            )
        })?;
        run_post_process_scripts(
            &pipeline_output.gcode_text,
            output_path,
            &post_process_scripts,
        )?
    };
    let gcode_text = if gcode_add_line_number {
        add_line_numbers(&gcode_text)
    } else {
        gcode_text
    };

    Ok(SliceRunResult::FullSlice(Box::new(SliceOutcome {
        gcode_text,
        layer_count,
        wallclock_ms,
        profile: profile_summary,
        ingestion_warnings,
        region_map: pipeline_output.region_map,
    })))
}

/// Canonical `gcode_add_line_number` (`PostProcessor.cpp`): prefix **every**
/// exported line with `N<line> `, numbered from 1 in file order. Canonical
/// rewrites the exported file line by line, so the whole artifact is covered
/// (header, comments, and CONFIG_BLOCK included) and the result ends with a
/// newline. The port emits `\n`-only G-code, so [`str::lines`] reproduces
/// canonical's `std::getline` split exactly.
fn add_line_numbers(text: &str) -> String {
    let mut out = String::new();
    for (index, line) in text.lines().enumerate() {
        out.push('N');
        out.push_str(&(index + 1).to_string());
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Parse the `post_process` host config key from the raw CLI/JSON config
/// source: the canonical `coStrings` spelling (`["cmd1", "cmd2"]`) and the
/// single-command string form.
///
/// Entries come back unsplit: canonical splits each one on `\r`/`\n` at run
/// time (one entry may pack several commands), so that split lives in
/// [`run_post_process_scripts`].
fn parse_post_process_scripts(
    config_source: &std::collections::HashMap<String, ConfigValue>,
) -> Result<Vec<String>, SliceRunError> {
    match config_source.get("post_process") {
        None => Ok(Vec::new()),
        Some(ConfigValue::String(script)) => Ok(vec![script.clone()]),
        Some(ConfigValue::List(items)) => items
            .iter()
            .enumerate()
            .map(|(index, item)| match item {
                ConfigValue::String(script) => Ok(script.clone()),
                _ => Err(SliceRunError(format!(
                    "post_process entry {index} must be a string"
                ))),
            })
            .collect(),
        Some(_) => Err(SliceRunError(
            "post_process must be a string or a list of strings".to_string(),
        )),
    }
}

/// Run the configured `post_process` scripts over a working copy of the
/// exported artifact and return the rewritten text.
///
/// Canonical (`PostProcessor.cpp::run_post_process_scripts`) hands each script
/// a file path — for a file export, `<output>.pp`, a copy next to the artifact
/// — and treats a non-zero exit or a file the script deleted as fatal. The port
/// keeps that shape: the caller writes `output_path` itself, so the scripts
/// work on the sibling `<output>.pp` working copy and the result is folded back
/// into the artifact both export paths carry.
fn run_post_process_scripts(
    text: &str,
    output_path: &Path,
    scripts: &[String],
) -> Result<String, SliceRunError> {
    let working_copy = post_process_working_copy_path(output_path);
    if let Some(parent) = working_copy.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SliceRunError(format!(
                    "failed to create the output directory for post-processing: {e}"
                ))
            })?;
        }
    }
    std::fs::write(&working_copy, text).map_err(|e| {
        SliceRunError(format!(
            "failed to write the post-processing working copy {}: {e}",
            working_copy.display()
        ))
    })?;

    let rewritten = (|| -> Result<String, SliceRunError> {
        for entry in scripts {
            for script in entry.split(['\r', '\n']) {
                let script = script.trim();
                if script.is_empty() {
                    continue;
                }
                run_post_process_script(script, &working_copy, output_path)?;
            }
        }
        std::fs::read_to_string(&working_copy).map_err(|e| {
            SliceRunError(format!(
                "post-processing left {} unreadable: {e}",
                working_copy.display()
            ))
        })
    })();

    // Canonical removes the copy on failure and the caller removes it on
    // success; here both happen in one place.
    let _ = std::fs::remove_file(&working_copy);
    rewritten
}

/// Canonical names a file export's working copy `<path>.pp`
/// (`PostProcessor.cpp::run_post_process_scripts`), so scripts never touch the
/// file the caller keeps.
fn post_process_working_copy_path(output_path: &Path) -> PathBuf {
    let mut path = output_path.as_os_str().to_os_string();
    path.push(".pp");
    PathBuf::from(path)
}

/// Run one configured command with the working-copy path appended as the final
/// argument, mirroring canonical's `run_script`: `$SHELL -c "<script> '<path>'"`
/// on POSIX, `cmd /C` on Windows. `SLIC3R_PP_HOST` /
/// `SLIC3R_PP_OUTPUT_NAME` carry the post-process values canonical exports to
/// the script's environment.
fn run_post_process_script(
    script: &str,
    working_copy: &Path,
    output_path: &Path,
) -> Result<(), SliceRunError> {
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new("cmd");
        // `raw_arg` keeps the command line verbatim: the entry is a command
        // line, and the working-copy path is appended as one quoted argument.
        command
            .raw_arg("/C")
            .raw_arg(format!("{script} \"{}\"", working_copy.display()));
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut command = Command::new(shell);
        command
            .arg("-c")
            .arg(format!("{script} {}", shell_quote(working_copy)));
        command
    };

    let output = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .env("SLIC3R_PP_HOST", "File")
        .env("SLIC3R_PP_OUTPUT_NAME", output_path)
        .output()
        .map_err(|e| {
            SliceRunError(format!(
                "failed to start post-processing script {script}: {e}"
            ))
        })?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut message = format!(
            "Post-processing script {script} on file {} failed.\nError code: {code}",
            working_copy.display()
        );
        if !stderr.is_empty() {
            message.push_str(&format!("\nOutput:\n{stderr}"));
        }
        return Err(SliceRunError(message));
    }
    if !working_copy.exists() {
        return Err(SliceRunError(format!(
            "Post-processing script {script} failed.\nThe script deleted or renamed {} instead of modifying it in place.",
            working_copy.display()
        )));
    }
    Ok(())
}

/// Quote `path` for a POSIX shell (`'` becomes `'\''`), the escaping canonical
/// `run_script` applies to the path it appends.
#[cfg(not(windows))]
fn shell_quote(path: &Path) -> String {
    let mut quoted = String::from("'");
    for character in path.to_string_lossy().chars() {
        if character == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(character);
        }
    }
    quoted.push('\'');
    quoted
}

/// Assembled scheduler/runtime context after prepass execution, ready for a
/// bounded per-layer closure (e.g.
/// [`crate::layer_executor::execute_captured_stages`]).
///
/// Built by [`prepare_prepass_context`]; used by `pnp-cli`'s visual-debug
/// command (packet 158) so it does not have to duplicate module loading,
/// config resolution, and plan construction to run a typed-tap capture.
pub struct PrepassContext {
    /// Execution plan with `global_layers` promoted from the
    /// prepass-committed `LayerPlanIR` (mirrors `run_pipeline_core`'s Step 2b).
    pub plan: crate::ExecutionPlan,
    /// Blackboard after prepass execution.
    pub blackboard: crate::Blackboard,
    /// Per-module wasmtime handles, keyed by `ModuleId`.
    pub wasm_handles: std::collections::HashMap<
        slicer_ir::ModuleId,
        (
            Arc<slicer_wasm_host::WasmInstancePool>,
            Option<Arc<slicer_wasm_host::WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
    /// Dispatcher ready to run per-layer (Tier 2) stages against the same
    /// `wasmtime::Engine` used for prepass.
    pub layer_runner: WasmRuntimeDispatcher,
    /// The global fallback [`slicer_ir::ResolvedConfig`] this context resolved
    /// (the same value `run_slice` passes to prepass).
    ///
    /// Exposed for printer-level keys a caller needs but no stage output
    /// carries — `printable_area` for visual-debug's `frame: "plate"` viewport
    /// being the first. Per-object overlays are irrelevant to those keys: the
    /// bed is a property of the printer, not of any object on it.
    pub default_resolved_config: Arc<slicer_ir::ResolvedConfig>,
    /// Per-tool resolved configs this context produced, keyed by tool index
    /// (the same `resolve_runtime_scopes` result `run_slice` hands to the
    /// emitter's `with_tool_configs`). Retained rather than discarded so the
    /// visual-debug model `PostPass::GCodeEmit` seam can consume the normal
    /// resolver's map instead of re-resolving (or silently defaulting) it.
    pub tool_configs: std::collections::BTreeMap<u32, slicer_ir::ResolvedConfig>,
}

/// Load modules, resolve config, build the live execution plan, and run
/// prepass — the shared prefix of [`run_slice`] up through Tier 1, factored
/// out so callers that only need a bounded Tier 2 closure (typed tap
/// capture, packet 158) do not have to duplicate it.
///
/// Deliberately narrower than `run_slice`'s setup: it skips the 14-pass
/// startup DAG validation, thumbnail/CONFIG_BLOCK wiring, relative-E and
/// MMU wipe-tower heuristics, and `validate_support_layer_heights` — none of
/// which affect per-layer arena commits or the gcode-emission handoff this
/// entry point serves. Per-tool configs are still resolved (they are part of
/// `resolve_runtime_scopes` and are retained on [`PrepassContext`] for the
/// emitter handoff).
///
/// # Errors
///
/// Returns [`SliceRunError`] if module loading, config resolution, plan
/// construction, or prepass execution fails.
pub fn prepare_prepass_context(
    mesh_ir: Arc<MeshIR>,
    mut config_source: std::collections::HashMap<String, ConfigValue>,
    layer_ranges: Vec<slicer_config::LayerRangeInput>,
    module_dirs: &[PathBuf],
    no_default_module_paths: bool,
    no_integrated_modules: bool,
) -> Result<PrepassContext, SliceRunError> {
    // Seed the host-injected `slice_has_paint` gate (mirrors `run_slice`):
    // set `true` whenever any object carries paint data, never overriding
    // an explicit user-supplied value.
    if mesh_ir.objects.iter().any(|o| o.paint_data.is_some())
        && !config_source.contains_key("slice_has_paint")
    {
        config_source.insert("slice_has_paint".to_string(), ConfigValue::Bool(true));
    }

    let search_roots = assemble_search_roots(module_dirs, no_default_module_paths);
    let integrated_registrations = if no_integrated_modules {
        Vec::new()
    } else {
        slicer_integrated_modules::integrated_registrations()
    };
    let native_entries = if no_integrated_modules {
        Vec::new()
    } else {
        slicer_integrated_modules::native_entries()
    };
    let manifest_first = load_live_modules_for_plan_manifest_first(
        &search_roots,
        num_cpus_guess(),
        &config_source,
        &layer_ranges,
        false,
        &integrated_registrations,
        &native_entries,
    )
    .map_err(|e| {
        SliceRunError(format!(
            "failed to load modules from {} root(s) {:?}: {e}",
            search_roots.len(),
            search_roots
        ))
    })?;
    let registry_warnings = manifest_first.registry_warnings;
    let mut ingestion_warnings = manifest_first.ingestion.warnings;
    let mut scoped_config = manifest_first.ingestion.scoped;
    let typed_global_config = typed_global_config(&scoped_config);
    let mut loaded = manifest_first.live;
    // Resolution runs against the same manifest-first registry that typed the
    // authored deltas. Claim dedup drops a module from dispatch only, never
    // from the config schema, so a claim-losing module's keys stay declared
    // and resolvable instead of failing `admission_set` as undeclared.
    let registry = manifest_first.registry;
    ingestion_warnings.extend(merge_model_scopes(
        &registry,
        &mut scoped_config,
        mesh_ir.as_ref(),
    )?);
    append_config_startup_diagnostics(
        &mut loaded.diagnostics,
        &registry_warnings,
        &ingestion_warnings,
    );

    for diag in &loaded.diagnostics {
        eprintln!(
            "{level:?}: {path}: {msg}",
            level = diag.level,
            path = diag.path.display(),
            msg = diag.message,
        );
    }

    let config_bounds = ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));
    let RuntimeResolvedScopes {
        default_config: default_resolved_config,
        target_configs: resolved_configs_map,
        tool_configs: per_tool_configs_map,
        object_layer_configs,
        expansion_context,
    } = resolve_runtime_scopes(&registry, &scoped_config, mesh_ir.as_ref())?;
    let module_config_source = typed_module_config_source(&config_source, &typed_global_config);
    let expanded_global_source =
        overlay_expanded_global(&module_config_source, &default_resolved_config);
    let layer_planning_objects = layer_planning_objects(&object_layer_configs);

    let wasm_handles: std::collections::HashMap<
        slicer_ir::ModuleId,
        (
            Arc<slicer_wasm_host::WasmInstancePool>,
            Option<Arc<slicer_wasm_host::WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    > = loaded
        .bindings
        .iter()
        .map(|b| {
            (
                b.module.id().to_string(),
                (
                    Arc::clone(&b.instance_pool),
                    b.wasm_component.clone(),
                    b.native_entry,
                ),
            )
        })
        .collect();

    let mut plan = build_live_execution_plan(
        loaded.sorted_stages,
        loaded.bindings,
        &default_resolved_config,
        Arc::new(Vec::new()),
        Arc::new(std::collections::HashMap::new()),
        &mut loaded.diagnostics,
    )
    .map_err(|e| SliceRunError(format!("failed to build execution plan: {e}")))?;

    let engine = Arc::clone(&loaded.engine);
    let prepass_runner = WasmRuntimeDispatcher::new(Arc::clone(&engine))
        .with_layer_planning_objects(layer_planning_objects);
    let mut blackboard = crate::Blackboard::new(Arc::clone(&mesh_ir), 0);
    crate::prepass::execute_prepass_with_builtins_configured_authority(
        &plan,
        &mut blackboard,
        &prepass_runner,
        &resolved_configs_map,
        &default_resolved_config,
        &expanded_global_source,
        &config_bounds,
        &wasm_handles,
        ConfigExpansionAuthority {
            registry: &registry,
            scoped: &scoped_config,
            expansion: &expansion_context,
        },
    )
    .map_err(|e| SliceRunError(format!("prepass failed: {e}")))?;

    crate::layer_executor::promote_global_layers(&mut plan, &blackboard);
    crate::layer_executor::build_and_commit_layer_selection_index(&plan, &mut blackboard)
        .map_err(|e| SliceRunError(format!("layer selection construction failed: {e}")))?;

    let layer_runner = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    Ok(PrepassContext {
        plan,
        blackboard,
        wasm_handles,
        layer_runner,
        default_resolved_config: Arc::new(default_resolved_config),
        tool_configs: per_tool_configs_map,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        add_line_numbers, emit_host_support_diagnostics, parse_feature_filament_selection,
        parse_post_process_scripts, parse_support_tool_selection, resolve_support_line_width_mm,
        run_post_process_scripts,
    };
    use slicer_ir::resolved_config::ResolvedFloatOrPercent;
    use slicer_ir::{ConfigValue, Diagnostic, DiagnosticSeverity};
    use slicer_scheduler::validation::ModuleAccessAudit;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn add_line_numbers_prefixes_every_line_from_one() {
        // Canonical `gcode_add_line_number`: every line — comments, commands,
        // and blanks alike — prefixed `N<line> ` from 1, newline-terminated.
        assert_eq!(
            add_line_numbers("; header\nG1 X0 Y0\n\nG1 X1\n"),
            "N1 ; header\nN2 G1 X0 Y0\nN3 \nN4 G1 X1\n"
        );
        // An unterminated final line gains a newline, matching canonical's
        // `getline` + `"\n"` write loop; empty input stays empty.
        assert_eq!(add_line_numbers("G28"), "N1 G28\n");
        assert_eq!(add_line_numbers(""), "");
    }

    /// Script text that appends `marker` to the file path the runner appends as
    /// its final argument, in each platform shell's spelling.
    #[cfg(windows)]
    fn appending_script(marker: &str) -> String {
        format!("echo {marker}>>")
    }

    #[cfg(not(windows))]
    fn appending_script(marker: &str) -> String {
        format!("printf '{marker}\\n' >>")
    }

    #[test]
    fn post_process_scripts_parse_string_list_and_reject_other_shapes() {
        let mut source = HashMap::new();
        assert!(
            parse_post_process_scripts(&source).unwrap().is_empty(),
            "an absent key means no scripts"
        );

        source.insert(
            "post_process".to_string(),
            ConfigValue::String("run-me.sh".to_string()),
        );
        assert_eq!(
            parse_post_process_scripts(&source).unwrap(),
            vec!["run-me.sh".to_string()]
        );

        source.insert(
            "post_process".to_string(),
            ConfigValue::List(vec![
                ConfigValue::String("first.sh".to_string()),
                ConfigValue::String("second.sh".to_string()),
            ]),
        );
        assert_eq!(
            parse_post_process_scripts(&source).unwrap(),
            vec!["first.sh".to_string(), "second.sh".to_string()]
        );

        source.insert("post_process".to_string(), ConfigValue::Bool(true));
        assert!(
            parse_post_process_scripts(&source).is_err(),
            "a non-list, non-string value must be rejected"
        );

        source.insert(
            "post_process".to_string(),
            ConfigValue::List(vec![ConfigValue::Int(3)]),
        );
        assert!(
            parse_post_process_scripts(&source).is_err(),
            "a non-string list entry must be rejected"
        );
    }

    #[test]
    fn post_process_scripts_rewrite_the_working_copy_and_clean_up() {
        let dir = tempfile::tempdir().expect("temp dir");
        let output = dir.path().join("artifact.gcode");

        // One entry packing two commands across a newline, wrapped in blank
        // lines: canonical splits each entry on `\r`/`\n` and skips the empties.
        let scripts = vec![format!(
            "\n{}\n{}\n",
            appending_script("POST_PROCESSED"),
            appending_script("SECOND_LINE")
        )];

        let rewritten = run_post_process_scripts("G1 X0 Y0\n", &output, &scripts)
            .expect("the working-copy rewrite must succeed");

        let mut lines = rewritten.lines();
        assert_eq!(lines.next(), Some("G1 X0 Y0"));
        assert_eq!(lines.next(), Some("POST_PROCESSED"));
        assert_eq!(
            lines.next(),
            Some("SECOND_LINE"),
            "one entry's newline-separated commands must run in order"
        );
        assert_eq!(lines.next(), None);

        assert!(
            !dir.path().join("artifact.gcode.pp").exists(),
            "the working copy must not survive the run"
        );
        assert!(
            !output.exists(),
            "the runner works on the copy; writing `output` stays the caller's job"
        );
    }

    #[test]
    fn post_process_script_failure_is_fatal_and_removes_the_working_copy() {
        let dir = tempfile::tempdir().expect("temp dir");
        let output = dir.path().join("artifact.gcode");
        let scripts = vec!["definitely-not-a-real-command-pnp-92".to_string()];

        let error = run_post_process_scripts("G1 X0 Y0\n", &output, &scripts)
            .expect_err("a script that cannot run must fail the slice");
        assert!(
            error.0.contains("definitely-not-a-real-command-pnp-92"),
            "the failure must name the script, got: {}",
            error.0
        );
        assert!(
            !dir.path().join("artifact.gcode.pp").exists(),
            "a failed run must not leave the working copy behind"
        );
    }

    #[test]
    fn post_process_script_deleting_the_artifact_is_fatal() {
        #[cfg(windows)]
        let scripts = vec!["del".to_string()];
        #[cfg(not(windows))]
        let scripts = vec!["rm".to_string()];

        let dir = tempfile::tempdir().expect("temp dir");
        let output = dir.path().join("artifact.gcode");
        let error = run_post_process_scripts("G1 X0 Y0\n", &output, &scripts)
            .expect_err("a script that deletes the artifact must fail the slice");
        assert!(
            error.0.contains("deleted"),
            "the failure must say the artifact was deleted, got: {}",
            error.0
        );
    }

    #[test]
    fn support_line_width_resolution_handles_mm_percent_and_auto() {
        assert_eq!(
            resolve_support_line_width_mm(
                ResolvedFloatOrPercent {
                    value: 0.42,
                    is_percent: false
                },
                0.4,
                0.4,
            ),
            0.42
        );
        assert_eq!(
            resolve_support_line_width_mm(
                ResolvedFloatOrPercent {
                    value: 50.0,
                    is_percent: true
                },
                0.4,
                0.4,
            ),
            0.2
        );
        // Canonical auto (`Flow::support_material_flow`): 0 resolves to
        // `line_width`, not the nozzle.
        assert_eq!(
            resolve_support_line_width_mm(
                ResolvedFloatOrPercent {
                    value: 0.0,
                    is_percent: false
                },
                0.5,
                0.4,
            ),
            0.5
        );
        // Orca's auto `line_width` of 0 resolves on to the nozzle diameter.
        assert_eq!(
            resolve_support_line_width_mm(
                ResolvedFloatOrPercent {
                    value: 0.0,
                    is_percent: false
                },
                0.0,
                0.4,
            ),
            0.4
        );
    }

    #[test]
    fn model_object_strings_are_typed_before_production_resolution() {
        let registry = nozzle_registry(Some("0.4"));
        let mut scoped = global_nozzle(Some(0.4));
        let mesh = slicer_ir::MeshIR {
            // exhaustive: ObjectMesh has no Default; empty geometry isolates config admission.
            objects: vec![slicer_ir::ObjectMesh {
                id: "object".into(),
                mesh: slicer_ir::IndexedTriangleSet {
                    vertices: Vec::new(),
                    indices: Vec::new(),
                },
                transform: slicer_ir::Transform3d { matrix: [0.0; 16] },
                config: slicer_ir::ObjectConfig {
                    data: HashMap::from([(
                        "nozzle_diameter".into(),
                        ConfigValue::String("0.6".into()),
                    )]),
                },
                modifier_volumes: Vec::new(),
                paint_data: None,
                world_z_extent: Some((0.0, 1.0)),
            }],
            ..Default::default()
        };
        super::merge_model_scopes(&registry, &mut scoped, &mesh).unwrap();
        let resolved = super::resolve_runtime_scopes(&registry, &scoped, &mesh)
            .expect("runtime must ingest model strings before resolving typed keys");
        assert_eq!(
            resolved.target_configs["object"].to_config_map()["nozzle_diameter"],
            ConfigValue::Float(0.6)
        );
        assert_eq!(
            scoped
                .delta(&slicer_config::ConfigScope::Object("object".into()))
                .unwrap()
                .values
                .len(),
            1,
            "model ingestion must not manufacture defaults"
        );
    }

    #[test]
    fn public_scope_resolution_uses_tool_index_for_tool_placeholders() {
        use slicer_config::{assemble_registry, HostChannels, ModuleDeclaration};
        use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};

        let mut schema = ConfigSchema::default();
        schema.entries.insert(
            "nozzle_diameter".to_owned(),
            ConfigFieldEntry {
                field_type: "float".to_owned(),
                ..ConfigFieldEntry::default()
            },
        );
        schema.entries.insert(
            "outer_wall_line_width".to_owned(),
            ConfigFieldEntry {
                field_type: "float_or_percent".to_owned(),
                base_key: Some("nozzle_diameter".to_owned()),
                ..ConfigFieldEntry::default()
            },
        );
        let registry = assemble_registry(
            &[ModuleDeclaration {
                module_id: "dev.pinch.test.scope-maps".to_owned(),
                schema,
                ..ModuleDeclaration::default()
            }],
            &HostChannels::from_parts(Vec::new(), Vec::new(), Vec::new()),
        )
        .expect("scope-maps fixture registry must be valid")
        .registry;
        let context = slicer_config::ExpansionContext {
            nozzle_diameter_mm: 0.4,
            tool_bases: std::collections::BTreeMap::from([(
                1u32,
                std::collections::BTreeMap::from([("nozzle_diameter".to_owned(), 0.6)]),
            )]),
        };

        let mut ingestor = slicer_config::ConfigIngestor::new(&registry);
        ingestor
            .ingest_delta(
                slicer_config::ConfigScope::Global,
                &HashMap::from([(
                    "outer_wall_line_width".to_owned(),
                    ConfigValue::Percent(150.0),
                )]),
            )
            .unwrap();
        let scoped = ingestor.finish().scoped;
        let resolve = |target| {
            slicer_config::resolve_scope_stack(&registry, &scoped, &target, &context).unwrap()
        };
        let global = resolve(slicer_config::ResolutionTarget::default());
        let object = resolve(slicer_config::ResolutionTarget {
            object_id: "obj".to_owned(),
            ..Default::default()
        });
        let tool = resolve(slicer_config::ResolutionTarget {
            tool_index: Some(1),
            ..Default::default()
        });

        let width = |config: &slicer_ir::ResolvedConfig, label: &str| match config
            .to_config_map()
            .get("outer_wall_line_width")
        {
            Some(ConfigValue::FloatOrPercent {
                value,
                is_percent: false,
            }) => *value,
            other => panic!("{label}: expected absolute FloatOrPercent, got {other:?}"),
        };
        assert!(
            (width(&global, "global") - 0.6).abs() <= 1e-12,
            "global must expand 150% of the 0.4 merged base"
        );
        assert!(
            (width(&object, "object") - 0.6).abs() <= 1e-12,
            "object must expand 150% of the 0.4 merged base"
        );
        assert!(
            (width(&tool, "tool 1") - 0.9).abs() <= 1e-12,
            "tool 1 must expand 150% of its 0.6 tool base; deleting the \
             resolver's tool context would use the global base"
        );
    }

    #[test]
    fn resolved_host_options_use_authored_boolean_and_tool_strings() {
        let registry =
            slicer_config::assemble_registry(&[], &slicer_config::HostChannels::from_live())
                .unwrap()
                .registry;
        let mut ingestor = slicer_config::ConfigIngestor::new(&registry);
        ingestor
            .ingest_delta(
                slicer_config::ConfigScope::Global,
                &HashMap::from([
                    (
                        "use_relative_e_distances".into(),
                        ConfigValue::String("0".into()),
                    ),
                    ("support_filament".into(), ConfigValue::String("2".into())),
                    (
                        "support_interface_filament".into(),
                        ConfigValue::String("3".into()),
                    ),
                    (
                        "filament_density".into(),
                        ConfigValue::List(vec![ConfigValue::Float(1.24); 3]),
                    ),
                ]),
            )
            .unwrap();
        let outcome = ingestor.finish();
        assert!(
            outcome.warnings.is_empty(),
            "host keys must be recognized by the live registry"
        );
        let config = slicer_config::resolve_scope_stack(
            &registry,
            &outcome.scoped,
            &slicer_config::ResolutionTarget::default(),
            &slicer_config::ExpansionContext {
                nozzle_diameter_mm: 0.4,
                ..Default::default()
            },
        )
        .unwrap();
        let (relative, tools) = super::resolved_host_options(&config);
        assert!(!relative, "authored string 0 must select absolute E");
        assert_eq!(tools.support_tool, 1);
        assert_eq!(tools.interface_tool, 2);
        assert_eq!(tools.tool_count, 3);
    }

    #[test]
    fn expanded_global_overlay_feeds_feedrate_config() {
        use super::overlay_expanded_global;
        // The raw source still carries the unexpanded overhang percent
        // placeholder; the host-expanded global carries the absolute value.
        let mut raw = HashMap::new();
        raw.insert("outer_wall_speed".to_string(), ConfigValue::Float(60.0));
        raw.insert("overhang_1_4_speed".to_string(), ConfigValue::Percent(25.0));
        let mut expanded = slicer_ir::ResolvedConfig {
            outer_wall_speed: 60.0,
            ..slicer_ir::ResolvedConfig::default()
        };
        expanded
            .extensions
            .insert("overhang_1_4_speed".to_owned(), ConfigValue::Float(15.0));
        let overlaid = overlay_expanded_global(&raw, &expanded);
        let feed = slicer_ir::FeedrateConfig::from_raw_config(&overlaid);
        assert_eq!(
            feed.overhang_1_4_speed, 15.0,
            "the expanded overlay must deliver absolute overhang mm/s"
        );
        // Without the overlay the percent placeholder keeps the 0.0 default.
        let feed_raw = slicer_ir::FeedrateConfig::from_raw_config(&raw);
        assert_eq!(
            feed_raw.overhang_1_4_speed, 0.0,
            "a raw percent placeholder must not leak into the feedrate table"
        );
    }

    #[test]
    fn parse_support_tool_selection_rebases_valid_orca_filament_indices() {
        let absent = HashMap::<String, ConfigValue>::new();
        let selection = parse_support_tool_selection(&absent);
        assert_eq!(selection.support_tool, 0);
        assert_eq!(selection.interface_tool, 0);

        let mut zero = HashMap::new();
        zero.insert("enable_support".to_string(), ConfigValue::Bool(false));
        zero.insert("support_filament".to_string(), ConfigValue::Int(0));
        zero.insert(
            "support_interface_filament".to_string(),
            ConfigValue::Int(0),
        );
        let selection = parse_support_tool_selection(&zero);
        assert_eq!(selection.support_tool, 0);
        assert_eq!(selection.interface_tool, 0);

        let mut configured = HashMap::new();
        configured.insert("enable_support".to_string(), ConfigValue::Bool(true));
        configured.insert("support_filament".to_string(), ConfigValue::Int(2));
        configured.insert(
            "support_interface_filament".to_string(),
            ConfigValue::Int(3),
        );
        let selection = parse_support_tool_selection(&configured);
        assert_eq!(selection.support_tool, 1);
        assert_eq!(selection.interface_tool, 2);
    }

    #[test]
    fn parse_support_tool_selection_rejects_invalid_or_out_of_range_indices() {
        let mut invalid = HashMap::new();
        invalid.insert("support_filament".to_string(), ConfigValue::Int(-1));
        invalid.insert(
            "support_interface_filament".to_string(),
            ConfigValue::Int(i64::from(u32::MAX) + 2),
        );
        let selection = parse_support_tool_selection(&invalid);
        assert_eq!(selection.support_tool, 0);
        assert_eq!(selection.interface_tool, 0);

        let mut extreme = HashMap::new();
        extreme.insert("support_filament".to_string(), ConfigValue::Int(i64::MAX));
        extreme.insert(
            "support_interface_filament".to_string(),
            ConfigValue::Int(i64::MIN),
        );
        let selection = parse_support_tool_selection(&extreme);
        assert_eq!(selection.support_tool, 0);
        assert_eq!(selection.interface_tool, 0);
    }

    #[test]
    fn support_interface_not_for_body_defaults_true_and_advances_colliding_body() {
        // Absent key means canonical default true; a default profile never
        // changes assignment (both tools 0, single tool).
        let absent = HashMap::<String, ConfigValue>::new();
        let selection = parse_support_tool_selection(&absent);
        assert!(selection.support_interface_not_for_body);
        assert_eq!(selection.support_tool, 0);

        // Explicit false is honoured (all spellings) and never advances.
        for value in [
            ConfigValue::Bool(false),
            ConfigValue::Int(0),
            ConfigValue::String("false".to_string()),
            ConfigValue::String("0".to_string()),
        ] {
            let mut cfg = HashMap::new();
            cfg.insert(
                "filament_density".to_string(),
                ConfigValue::List(vec![ConfigValue::Float(1.24), ConfigValue::Float(1.24)]),
            );
            cfg.insert("support_filament".to_string(), ConfigValue::Int(2));
            cfg.insert(
                "support_interface_filament".to_string(),
                ConfigValue::Int(2),
            );
            cfg.insert("support_interface_not_for_body".to_string(), value);
            let selection = parse_support_tool_selection(&cfg);
            assert!(!selection.support_interface_not_for_body);
            assert_eq!(selection.support_tool, 1);
            assert_eq!(selection.interface_tool, 1);
        }

        // Default true with an explicitly configured colliding interface and
        // another tool available advances the body to the smallest
        // non-interface tool.
        let mut colliding = HashMap::new();
        colliding.insert(
            "filament_density".to_string(),
            ConfigValue::List(vec![ConfigValue::Float(1.24), ConfigValue::Float(1.24)]),
        );
        colliding.insert("support_filament".to_string(), ConfigValue::Int(2));
        colliding.insert(
            "support_interface_filament".to_string(),
            ConfigValue::Int(2),
        );
        let selection = parse_support_tool_selection(&colliding);
        assert!(selection.support_interface_not_for_body);
        assert_eq!(selection.interface_tool, 1);
        assert_eq!(selection.support_tool, 0);

        // No advancement when the interface was never explicitly configured,
        // even on a multi-tool machine with colliding tools.
        let mut implicit = HashMap::new();
        implicit.insert(
            "filament_density".to_string(),
            ConfigValue::List(vec![ConfigValue::Float(1.24), ConfigValue::Float(1.24)]),
        );
        let selection = parse_support_tool_selection(&implicit);
        assert_eq!(selection.support_tool, 0);

        // No advancement on a single-tool machine.
        let mut single = HashMap::new();
        single.insert("support_filament".to_string(), ConfigValue::Int(2));
        single.insert(
            "support_interface_filament".to_string(),
            ConfigValue::Int(2),
        );
        let selection = parse_support_tool_selection(&single);
        assert_eq!(selection.support_tool, 1);
    }

    #[test]
    fn parse_feature_filament_selection_rebases_configured_one_based_indices() {
        let absent = HashMap::<String, ConfigValue>::new();
        let selection = parse_feature_filament_selection(&absent, 1);
        assert_eq!(selection.sparse, None);
        assert_eq!(selection.internal_solid, None);
        assert_eq!(selection.top_surface, None);
        assert_eq!(selection.bottom_surface, None);
        assert_eq!(selection.inner_wall, None);
        assert_eq!(selection.outer_wall, None);

        // Three configured tools; every key carries a 1-based Orca index.
        let mut configured = HashMap::new();
        configured.insert(
            "filament_density".to_string(),
            ConfigValue::List(vec![
                ConfigValue::Float(1.24),
                ConfigValue::Float(1.24),
                ConfigValue::Float(1.24),
            ]),
        );
        configured.insert("sparse_infill_filament_id".to_string(), ConfigValue::Int(2));
        configured.insert(
            "internal_solid_filament_id".to_string(),
            ConfigValue::Int(3),
        );
        configured.insert("top_surface_filament_id".to_string(), ConfigValue::Int(1));
        configured.insert(
            "bottom_surface_filament_id".to_string(),
            ConfigValue::Int(3),
        );
        configured.insert("inner_wall_filament_id".to_string(), ConfigValue::Int(2));
        configured.insert("outer_wall_filament_id".to_string(), ConfigValue::Int(1));
        let selection = parse_support_tool_selection(&configured);
        assert_eq!(selection.tool_count, 3);
        let features = selection.feature_filaments;
        assert_eq!(features.sparse, Some(1));
        assert_eq!(features.internal_solid, Some(2));
        assert_eq!(features.top_surface, Some(0));
        assert_eq!(features.bottom_surface, Some(2));
        assert_eq!(features.inner_wall, Some(1));
        assert_eq!(features.outer_wall, Some(0));
    }

    #[test]
    fn parse_feature_filament_selection_zero_and_invalid_mean_inherit() {
        // Canonical 0 = Default: inherit, never an explicit tool — so the
        // parser yields `None`, and so do absent, negative, and non-`Int`
        // spellings (the support-selector precedent maps those to tool 0;
        // here tool 0 must stay reachable only as an explicit `1`).
        let mut zero = HashMap::new();
        zero.insert("sparse_infill_filament_id".to_string(), ConfigValue::Int(0));
        zero.insert(
            "outer_wall_filament_id".to_string(),
            ConfigValue::Float(2.0),
        );
        zero.insert(
            "inner_wall_filament_id".to_string(),
            ConfigValue::String("2".to_string()),
        );
        zero.insert("top_surface_filament_id".to_string(), ConfigValue::Int(-1));
        let selection = parse_feature_filament_selection(&zero, 3);
        assert_eq!(selection.sparse, None);
        assert_eq!(selection.outer_wall, None);
        assert_eq!(selection.inner_wall, None);
        assert_eq!(selection.top_surface, None);
        assert_eq!(selection.internal_solid, None);
        assert_eq!(selection.bottom_surface, None);
    }

    #[test]
    fn parse_feature_filament_selection_clamps_explicit_index_to_tool_count() {
        // Canonical `clamp_feature_filament_to_valid`: an explicit index past
        // the last configured tool clamps to it instead of rejecting.
        let mut clamped = HashMap::new();
        clamped.insert(
            "filament_density".to_string(),
            ConfigValue::List(vec![ConfigValue::Float(1.24), ConfigValue::Float(1.24)]),
        );
        clamped.insert("sparse_infill_filament_id".to_string(), ConfigValue::Int(9));
        clamped.insert(
            "outer_wall_filament_id".to_string(),
            ConfigValue::Int(i64::MAX),
        );
        let selection = parse_support_tool_selection(&clamped);
        assert_eq!(selection.tool_count, 2);
        assert_eq!(selection.feature_filaments.sparse, Some(1));
        assert_eq!(selection.feature_filaments.outer_wall, Some(1));

        // A single-tool machine folds every explicit selector onto tool 0.
        let mut single = HashMap::new();
        single.insert(
            "bottom_surface_filament_id".to_string(),
            ConfigValue::Int(4),
        );
        let selection = parse_feature_filament_selection(&single, 1);
        assert_eq!(selection.bottom_surface, Some(0));
    }

    #[test]
    fn host_support_warning_reaches_degraded_slice_collector() {
        let collector = Arc::new(Mutex::new(
            crate::progress_events::SliceEventCollector::new(),
        ));
        let sink = crate::progress_events::RuntimeProgressSink::new(
            Arc::new(crate::progress_events::NullEmitter),
            Arc::clone(&collector),
        );
        // exhaustive: no Default impl for ModuleAccessAudit; audit fixture pins every access-audit field
        let audit = ModuleAccessAudit {
            module_id: "support-geometry".into(),
            runtime_reads: Vec::new(),
            runtime_writes: Vec::new(),
            batch_calls: Vec::new(),
            // exhaustive: no Default impl for Diagnostic; audit fixture pins the full diagnostic shape
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Warn,
                code: 1200,
                layer: None,
                object_id: None,
                message: "support demand unmet".into(),
            }],
        };

        emit_host_support_diagnostics(&sink, "slice", &[audit]);

        let collector = collector.lock().unwrap();
        assert!(collector.is_degraded());
        assert_eq!(collector.non_fatal_count(), 1);
        assert_eq!(collector.events()[0].error.as_ref().unwrap().code, 1200);
        assert_eq!(
            collector.events()[0].error.as_ref().unwrap().message,
            "support demand unmet"
        );
    }

    #[test]
    fn host_duplicate_support_diagnostic_reaches_degraded_slice_collector() {
        let collector = Arc::new(Mutex::new(
            crate::progress_events::SliceEventCollector::new(),
        ));
        let sink = crate::progress_events::RuntimeProgressSink::new(
            Arc::new(crate::progress_events::NullEmitter),
            Arc::clone(&collector),
        );
        // exhaustive: no Default impl for ModuleAccessAudit; audit fixture pins every access-audit field
        let audit = ModuleAccessAudit {
            module_id: "support-geometry".into(),
            runtime_reads: Vec::new(),
            runtime_writes: Vec::new(),
            batch_calls: Vec::new(),
            // exhaustive: no Default impl for Diagnostic; audit fixture pins the full diagnostic shape
            diagnostics: vec![Diagnostic {
                severity: DiagnosticSeverity::Warn,
                code: 1202,
                layer: Some(7),
                object_id: Some("body-1".into()),
                message: "duplicate support region rejected".into(),
            }],
        };

        emit_host_support_diagnostics(&sink, "slice", &[audit]);

        let collector = collector.lock().unwrap();
        assert!(collector.is_degraded());
        assert_eq!(collector.non_fatal_count(), 1);
        let error = collector.events()[0].error.as_ref().unwrap();
        assert_eq!(error.code, 1202);
        assert_eq!(error.message, "duplicate support region rejected");
        assert_eq!(error.fatal, false);
    }

    // ── Step 6a: seed_expansion_context nozzle_diameter fallback ────────────

    /// Registry fixture declaring one module-manifest `nozzle_diameter` with the
    /// given default — the same channel `assemble_registry` reads from
    /// `modules/core-modules/*/*.toml` on the live path.
    fn nozzle_registry(default: Option<&str>) -> slicer_config::ConfigSchemaRegistry {
        use slicer_config::{HostChannels, ModuleDeclaration};
        use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};

        let mut schema = ConfigSchema::default();
        schema.entries.insert(
            "nozzle_diameter".to_owned(),
            ConfigFieldEntry {
                field_type: "float".to_owned(),
                default: default.map(str::to_owned),
                ..ConfigFieldEntry::default()
            },
        );
        slicer_config::assemble_registry(
            &[ModuleDeclaration {
                module_id: "dev.pinch.test.seed-expansion".to_owned(),
                schema,
                ..ModuleDeclaration::default()
            }],
            &HostChannels::from_parts(Vec::new(), Vec::new(), Vec::new()),
        )
        .expect("seed-expansion fixture registry must be valid")
        .registry
    }

    fn global_nozzle(value: Option<f64>) -> slicer_config::ScopedConfig {
        let mut scoped = slicer_config::ScopedConfig::default();
        if let Some(value) = value {
            scoped.deltas.insert(
                slicer_config::ConfigScope::Global,
                slicer_config::ScopeDelta {
                    values: std::collections::BTreeMap::from([(
                        "nozzle_diameter".to_owned(),
                        ConfigValue::Float(value),
                    )]),
                },
            );
        }
        scoped
    }

    /// Residual-red session: `absolute_config_number` applies the Orca wire
    /// shape leniency `extract_float_or_first` documents — a per-filament
    /// `List` resolves through its first element and a numeric string parses —
    /// while percent forms stay non-absolute and an empty envelope carries no
    /// value.
    #[test]
    fn absolute_config_number_accepts_orca_wire_shapes() {
        use slicer_ir::ConfigValue;
        assert_eq!(
            super::absolute_config_number(&ConfigValue::List(vec![ConfigValue::String(
                "0.4".to_owned()
            )])),
            Some(0.4)
        );
        assert_eq!(
            super::absolute_config_number(&ConfigValue::List(vec![ConfigValue::Float(0.6)])),
            Some(0.6)
        );
        assert_eq!(
            super::absolute_config_number(&ConfigValue::String(" 0.5 ".to_owned())),
            Some(0.5)
        );
        assert_eq!(
            super::absolute_config_number(&ConfigValue::String("50%".to_owned())),
            None,
            "percent forms are not absolute"
        );
        assert_eq!(
            super::absolute_config_number(&ConfigValue::List(vec![])),
            None,
            "an empty envelope carries no value"
        );
    }

    /// Exit condition 1: an authored value must keep winning over the
    /// registry default. Deleting the authored branch and always reading the
    /// registry would resolve 0.4 here and fail this assertion.
    #[test]
    fn seed_expansion_context_prefers_authored_nozzle_over_registry_default() {
        let registry = nozzle_registry(Some("0.4"));
        let context = super::seed_expansion_context(&registry, &global_nozzle(Some(0.7)))
            .expect("authored nozzle_diameter must seed the expansion context");
        assert_eq!(
            context.nozzle_diameter_mm, 0.7,
            "an authored nozzle_diameter must win over the registry default"
        );
    }

    /// Exit condition 2: the unauthored fallback must be derived from the
    /// assembled registry, never a pasted 0.4 literal. The fixture declares a
    /// non-0.4 default (0.55) so a hardcoded literal fails this assertion.
    #[test]
    fn seed_expansion_context_falls_back_to_registry_default() {
        let registry = nozzle_registry(Some("0.55"));
        let context = super::seed_expansion_context(&registry, &global_nozzle(None))
            .expect("unauthored nozzle_diameter must fall back to the registry default");
        assert_eq!(
            context.nozzle_diameter_mm, 0.55,
            "the fallback must be the registry-declared default, not a code literal"
        );
    }

    /// A registry that declares no default seeds nothing: the missing-base
    /// error is preserved instead of inventing a value.
    #[test]
    fn seed_expansion_context_errors_when_registry_declares_no_default() {
        let registry = nozzle_registry(None);
        let error = super::seed_expansion_context(&registry, &global_nozzle(None))
            .expect_err("absent authored value and absent registry default must error");
        assert!(
            error.0.contains("nozzle_diameter"),
            "the missing-base error must name nozzle_diameter, got {error:?}"
        );
    }
}
