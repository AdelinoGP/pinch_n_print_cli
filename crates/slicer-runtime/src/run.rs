//! Library entry point for one-shot slicing. Extracted from main.rs::HostCommands::Run.

/// Default for the `use_relative_e_distances` host config key (M83 relative-E)
/// when the user does not set it. Mirrored in `docs/config/host-keys.toml`
/// (`[host_runtime]`) and locked by `gcode_emit::host_keys_doc_lock`.
pub const DEFAULT_USE_RELATIVE_E_DISTANCES: bool = true;

use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::time::Instant;

use slicer_config::resolution::{
    query_z_grid, resolve_scope_stack, ResolutionTarget, ResolvedObjectLayerConfig,
};
use slicer_config::{
    ConfigIngestor, ConfigSchemaRegistry, ConfigScope, ExpansionContext, IngestionWarning,
    RegistryWarning, ScopedConfig,
};
use slicer_ir::{
    ConfigKey, ConfigValue, MeshIR, ModifierKind, PaintSemantic, PaintValue, ResolvedConfig,
};
use slicer_sdk::traits::LayerPlanningObject;

/// Parse Orca-style 1-indexed support filament selections into the runtime's
/// 0-indexed tool selection. Missing, zero, invalid, and out-of-range values
/// retain the default tool 0 behavior.
///
/// Also derives [`SupportToolSelection::tool_count`] from the same raw config
/// map, using the `filament_density` list length. That is the identical source
/// `ResolvedConfig.filament_density` is extracted from this same authored map,
/// and `extract_float_list` preserves element count, so the
/// value here equals `max(1, ResolvedConfig.filament_density.len())` without
/// needing a resolved config threaded to this call site.
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
    crate::layer_executor::SupportToolSelection {
        support_tool: rebase("support_filament"),
        interface_tool: rebase("support_interface_filament"),
        tool_count,
    }
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
use crate::prepass::ConfigExpansionAuthority;
use crate::profiling_report::{ProfileAggregator, ProfileSummary};
use crate::progress_events::{
    JsonLinesEmitter, NullEmitter, ProgressError, ProgressEvent, ProgressEventEmitter,
    ProgressPhase, ProgressStatus, RuntimeProgressSink, SliceEventCollector,
};
use crate::progress_instrumentation::{now_unix_ms, ProgressPipelineInstrumentation, ProgressTier};
#[cfg(feature = "report")]
use crate::report::{allocator as report_alloc, Collector};
use crate::validation::{validate_startup_dag, DagValidationPass, StageDag};
use slicer_gcode::{
    estimate_print, DefaultGCodeEmitter, DefaultGCodeSerializer, EstimatorLimits, GcodeFlavor,
};
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

const RESOLVED_TARGET_PREFIX: &str = "\0resolved-target:";
const RESOLVED_PAINT_PREFIX: &str = "\0resolved-paint:";
const RESOLVED_TOOL_PREFIX: &str = "\0resolved-tool:";

fn paint_semantic_name(semantic: &PaintSemantic) -> String {
    match semantic {
        PaintSemantic::Material => "material".to_owned(),
        PaintSemantic::FuzzySkin => "fuzzy_skin".to_owned(),
        PaintSemantic::SupportEnforcer => "support_enforcer".to_owned(),
        PaintSemantic::SupportBlocker => "support_blocker".to_owned(),
        PaintSemantic::Custom(name) => name.clone(),
    }
}

fn append_key_part(key: &mut String, value: &str) {
    use std::fmt::Write as _;
    let _ = write!(key, "{}:{value}", value.len());
}

fn resolved_target_key(
    object_id: &str,
    modifier_ids: &[String],
    paint_semantics: &[String],
    tool_index: Option<u32>,
) -> String {
    if modifier_ids.is_empty() && paint_semantics.is_empty() && tool_index.is_none() {
        return object_id.to_owned();
    }
    let mut key = RESOLVED_TARGET_PREFIX.to_owned();
    append_key_part(&mut key, object_id);
    key.push('|');
    for modifier_id in modifier_ids {
        append_key_part(&mut key, modifier_id);
        key.push(',');
    }
    key.push('|');
    for semantic in paint_semantics {
        append_key_part(&mut key, semantic);
        key.push(',');
    }
    key.push('|');
    if let Some(tool_index) = tool_index {
        use std::fmt::Write as _;
        let _ = write!(key, "{tool_index}");
    }
    key
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
    let mut modifier_ingestor = ConfigIngestor::new(registry);
    for object in &mesh.objects {
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

            modifier_ingestor
                .ingest_delta(scope, &values)
                .map_err(|error| {
                    SliceRunError(format!("modifier config ingestion failed: {error}"))
                })?;
        }
    }
    let modifier_outcome = modifier_ingestor.finish();
    validate_modifier_deltas(registry, &modifier_outcome.scoped)?;

    for object in &mesh.objects {
        let object_delta = scoped
            .deltas
            .entry(ConfigScope::Object(object.id.clone()))
            .or_default();
        for (key, value) in &object.config.data {
            object_delta
                .values
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }
    for (scope, delta) in modifier_outcome.scoped.deltas {
        let target = scoped.deltas.entry(scope).or_default();
        for (key, value) in delta.values {
            target.values.entry(key).or_insert(value);
        }
    }

    Ok(modifier_outcome.warnings)
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

#[cfg(test)]
fn expand_config(
    registry: &slicer_config::ConfigSchemaRegistry,
    config: &mut slicer_ir::ResolvedConfig,
    context: &ExpansionContext,
    tool_index: Option<u32>,
    scope: &str,
) -> Result<(), SliceRunError> {
    slicer_config::expand_automatic_values(registry, config, context, tool_index).map_err(|error| {
        SliceRunError(format!(
            "automatic value expansion failed for {scope}: {error}"
        ))
    })
}

/// Expand every already-merged scope map in place: the global default, each
/// per-object map, and each per-tool map with its tool index so
/// `ExpansionContext.tool_bases` selects the matching absolute base.
/// Runs after scope merge and before plan binding, feedrate construction, or
/// emitter use on every production entry point; the first failure aborts
/// before any consumer binds a map.
#[cfg(test)]
fn expand_scope_maps(
    registry: &slicer_config::ConfigSchemaRegistry,
    default_config: &mut slicer_ir::ResolvedConfig,
    object_configs: &mut std::collections::BTreeMap<String, slicer_ir::ResolvedConfig>,
    tool_configs: &mut std::collections::BTreeMap<u32, slicer_ir::ResolvedConfig>,
    context: &ExpansionContext,
) -> Result<(), SliceRunError> {
    expand_config(registry, default_config, context, None, "global scope")?;
    for (object_id, config) in object_configs.iter_mut() {
        expand_config(
            registry,
            config,
            context,
            None,
            &format!("object {object_id}"),
        )?;
    }
    for (&tool_index, config) in tool_configs.iter_mut() {
        expand_config(
            registry,
            config,
            context,
            Some(tool_index),
            &format!("tool {tool_index}"),
        )?;
    }
    Ok(())
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
            LayerPlanningObject {
                object_id,
                object_height,
                layer_height,
                first_layer_height,
                support_raft_layers,
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
fn run_pipeline_fork(
    opts: &SliceRunOptions,
    channel: &ProgressChannel,
    config: PipelineConfig,
    config_source: &std::collections::HashMap<String, ConfigValue>,
    registry: &slicer_config::ConfigSchemaRegistry,
    expansion_context: &ExpansionContext,
    profile: Option<&Arc<ProfileAggregator>>,
    #[cfg(feature = "report")] dag_snapshot: Option<crate::report::ReportDagSnapshot>,
) -> Result<crate::pipeline::PipelineOutput, SliceRunError> {
    let sink_arc = Arc::clone(&channel.sink);
    let expansion_authority = ConfigExpansionAuthority {
        _registry: registry,
        _context: expansion_context,
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
    // The projection is over the LOADED module set (packet-06 AC-4): claim
    // dedup drops a module from dispatch only, never from the config schema,
    // so the manifest-first registry still declares the claim-losing module's
    // keys for resolution — but they must not leak into the emitted block.
    let live_modules: std::collections::BTreeSet<String> = config
        .wasm_handles
        .keys()
        .map(ToString::to_string)
        .collect();
    let config_block =
        registry.config_block_map_for_modules(&config.default_resolved_config, &live_modules);

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
                )
            } else {
                run_pipeline_with_instrumentation_authority(
                    config,
                    config_source,
                    sink_arc.as_ref(),
                    report_collector.as_ref(),
                    expansion_authority,
                    Some(&config_block),
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
        ),
        (None, None) => run_pipeline_with_raw_config_authority(
            config,
            config_source,
            sink_arc.as_ref(),
            expansion_authority,
            Some(&config_block),
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
/// (`filament_density`, `first_layer_height`).
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
    /// First layer height in mm (`ResolvedConfig.first_layer_height`).
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

/// One-shot slice. Drives the pipeline end-to-end and returns the produced G-code.
///
/// Composes the 4-way instrumentation fork (report, progress, both, none)
/// internally based on `opts.report` and `opts.instrument_stderr`.
pub fn run_slice(opts: SliceRunOptions) -> Result<SliceOutcome, SliceRunError> {
    run_slice_with_collector(opts, None)
}

/// Test-support entry point that lets callers inspect the same collector used
/// by [`run_slice`], including when the pipeline returns an error.
#[doc(hidden)]
pub fn run_slice_with_collector(
    opts: SliceRunOptions,
    collector: Option<Arc<Mutex<SliceEventCollector>>>,
) -> Result<SliceOutcome, SliceRunError> {
    let t0 = Instant::now();
    let channel = build_progress_channel(opts.progress_events, collector);

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
    for (key, value) in &opts.config_overrides {
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
    // `wipe_tower_enabled`, enable it. Single-colour / unpainted prints keep the
    // default (false) and are unaffected. (The wipe-tower module is fully
    // implemented and wired; it was simply gated off by the resolved-config
    // default of `false` with no auto-enable signal.)
    if !config_source.contains_key("wipe_tower_enabled") {
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
            config_source.insert("wipe_tower_enabled".to_string(), ConfigValue::Bool(true));
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
    let (integrated_regs, native_entries) = if opts.no_integrated_modules {
        (Vec::new(), Vec::new())
    } else {
        (
            slicer_integrated_modules::integrated_registrations(),
            slicer_integrated_modules::native_entries(),
        )
    };
    let manifest_first = load_live_modules_for_plan_manifest_first(
        &search_roots,
        num_cpus_guess(),
        &config_source,
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
                    return Err(SliceRunError(msg));
                }
            }
        }

        let dag_modules: Vec<crate::manifest::LoadedModule> =
            loaded.bindings.iter().map(|b| b.module.clone()).collect();
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
            modules: dag_modules,
            stage_dags,
            host_ir_schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            host_version,
            claim_holders,
            access_audits: Vec::new(),
        };
        let report = validate_startup_dag(&request);

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
            return Err(SliceRunError(msg));
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

    let module_config_source = typed_module_config_source(&config_source, &typed_global_config);
    let expanded_global_source =
        overlay_expanded_global(&module_config_source, &default_resolved_config);
    let layer_planning_objects = layer_planning_objects(&object_layer_configs);

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
    let flavor = match config_source.get("gcode_flavor") {
        Some(ConfigValue::String(value)) => GcodeFlavor::from_config_str(value),
        _ => GcodeFlavor::Marlin,
    };
    let relative = match config_source.get("use_relative_e_distances") {
        Some(ConfigValue::Bool(b)) => *b,
        _ => DEFAULT_USE_RELATIVE_E_DISTANCES,
    };
    let support_line_width_mm = default_resolved_config.support_line_width.value as f32;

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
    let stats_first_layer_height_mm = default_resolved_config.first_layer_height as f32;

    let pipeline_config = PipelineConfig {
        cancel_flag: opts.cancel_flag.clone(),
        mesh_ir,
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(
                WasmRuntimeDispatcher::new(Arc::clone(&engine))
                    .with_layer_planning_objects(layer_planning_objects),
            ),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
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
                .with_resolved_config(default_resolved_config.clone())
                .with_tool_configs(per_tool_configs_map.clone()),
            ),
            serializer: Box::new(
                DefaultGCodeSerializer::with_extrusion_mode(relative)
                    .with_flavor(flavor)
                    .with_support_line_width(support_line_width_mm),
            ),
        },
        support_tools: parse_support_tool_selection(&config_source),
        resolved_configs: Arc::new(resolved_configs_map),
        default_resolved_config: Arc::new(default_resolved_config),
        bounds: Arc::new(config_bounds),
        wasm_handles,
        anchored_entities: Vec::new(),
    };

    // Run the pipeline through the 4-way instrumentation fork.
    let pipeline_result = run_pipeline_fork(
        &opts,
        &channel,
        pipeline_config,
        &expanded_global_source,
        &registry,
        &expansion_context,
        profile.as_ref(),
        #[cfg(feature = "report")]
        dag_snapshot,
    );

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

    let pipeline_output = pipeline_result?;

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

    Ok(SliceOutcome {
        gcode_text: pipeline_output.gcode_text,
        layer_count,
        wallclock_ms,
        profile: profile_summary,
        ingestion_warnings,
    })
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
    /// carries — `bed_shape` for visual-debug's `frame: "plate"` viewport
    /// being the first. Per-object overlays are irrelevant to those keys: the
    /// bed is a property of the printer, not of any object on it.
    pub default_resolved_config: Arc<slicer_ir::ResolvedConfig>,
}

/// Load modules, resolve config, build the live execution plan, and run
/// prepass — the shared prefix of [`run_slice`] up through Tier 1, factored
/// out so callers that only need a bounded Tier 2 closure (typed tap
/// capture, packet 158) do not have to duplicate it.
///
/// Deliberately narrower than `run_slice`'s setup: it skips the 14-pass
/// startup DAG validation, thumbnail/CONFIG_BLOCK wiring, relative-E and
/// MMU wipe-tower heuristics, `validate_support_layer_heights`, and
/// per-tool config resolution — none of which affect per-layer arena
/// commits, and all of which belong to gcode-emission concerns this entry
/// point never reaches.
///
/// # Errors
///
/// Returns [`SliceRunError`] if module loading, config resolution, plan
/// construction, or prepass execution fails.
pub fn prepare_prepass_context(
    mesh_ir: Arc<MeshIR>,
    mut config_source: std::collections::HashMap<String, ConfigValue>,
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
        tool_configs: _,
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
            _registry: &registry,
            _context: &expansion_context,
        },
    )
    .map_err(|e| SliceRunError(format!("prepass failed: {e}")))?;

    crate::layer_executor::promote_global_layers(&mut plan, &blackboard);

    let layer_runner = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    Ok(PrepassContext {
        plan,
        blackboard,
        wasm_handles,
        layer_runner,
        default_resolved_config: Arc::new(default_resolved_config),
    })
}

#[cfg(test)]
mod tests {
    use super::{emit_host_support_diagnostics, parse_support_tool_selection};
    use slicer_ir::{ConfigValue, Diagnostic, DiagnosticSeverity};
    use slicer_scheduler::validation::ModuleAccessAudit;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn expand_scope_maps_uses_tool_index_for_tool_placeholders() {
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

        let mut scoped = slicer_ir::ResolvedConfig::default();
        scoped.extensions.insert(
            "outer_wall_line_width".to_owned(),
            ConfigValue::Percent(150.0),
        );
        let mut global = scoped.clone();
        let mut objects = std::collections::BTreeMap::from([("obj".to_owned(), scoped.clone())]);
        let mut tools = std::collections::BTreeMap::from([(1u32, scoped)]);

        super::expand_scope_maps(&registry, &mut global, &mut objects, &mut tools, &context)
            .expect("literal fixture carries every required base");

        let width = |config: &slicer_ir::ResolvedConfig, label: &str| match config
            .to_config_map()
            .get("outer_wall_line_width")
        {
            Some(ConfigValue::Float(value)) => *value,
            other => panic!(
                "{label}: outer_wall_line_width must expand to absolute float, got {other:?}"
            ),
        };
        assert!(
            (width(&global, "global") - 0.6).abs() <= 1e-12,
            "global must expand 150% of the 0.4 merged base"
        );
        assert!(
            (width(&objects["obj"], "object") - 0.6).abs() <= 1e-12,
            "object must expand 150% of the 0.4 merged base"
        );
        assert!(
            (width(&tools[&1], "tool 1") - 0.9).abs() <= 1e-12,
            "tool 1 must expand 150% of its 0.6 tool base; deleting the \
             per-tool loop would leave the Percent placeholder"
        );
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
