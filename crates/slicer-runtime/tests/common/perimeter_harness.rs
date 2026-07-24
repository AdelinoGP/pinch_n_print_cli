//! Shared real-pipeline capture and classic/Arachne coverage measurement.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use slicer_gcode::{DefaultGCodeEmitter, DefaultGCodeSerializer};
use slicer_ir::{ConfigValue, GlobalLayer, LayerStageCommit, ModuleId, PerimeterIR, StageId};
use slicer_runtime::pipeline::{
    run_pipeline_with_raw_config, PipelineConfig, PipelineStageRunners,
};
use slicer_runtime::{
    assemble_search_roots, build_live_execution_plan, load_live_modules_for_plan_with_config,
    resolve_global_config, resolve_per_object_configs, resolve_per_tool_configs,
    validate_support_layer_heights, CompiledModuleLive, ConfigBoundsIndex, LayerStageError,
    LayerStageInput, NoopLayerProgressSink, WasmComponent, WasmInstancePool, WasmRuntimeDispatcher,
};

/// The two values accepted by the production `wall_generator` selector.
#[derive(Clone, Copy, Debug)]
pub enum WallGenerator {
    Classic,
    Arachne,
}

impl WallGenerator {
    fn config_value(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Arachne => "arachne",
        }
    }
}

/// Captures the final perimeter value observed for each global layer while
/// delegating execution to the production stage runner.
pub struct PerimeterCapturingLayerStageRunner {
    inner: Box<dyn slicer_runtime::LayerStageRunner + Sync>,
    captured: Arc<Mutex<HashMap<u32, PerimeterIR>>>,
}

impl PerimeterCapturingLayerStageRunner {
    pub fn new(inner: Box<dyn slicer_runtime::LayerStageRunner + Sync>) -> Self {
        Self {
            inner,
            captured: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn sink(&self) -> Arc<Mutex<HashMap<u32, PerimeterIR>>> {
        Arc::clone(&self.captured)
    }
}

impl slicer_runtime::LayerStageRunner for PerimeterCapturingLayerStageRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        if let Some(perimeter) = input.perimeter {
            self.captured
                .lock()
                .expect("capture mutex poisoned")
                .insert(perimeter.global_layer_index, perimeter.clone());
        }
        self.inner.run_stage(stage_id, layer, module, input)
    }

    fn last_wasm_mem_sample(&self) -> (u64, u64) {
        self.inner.last_wasm_mem_sample()
    }

    fn last_runtime_reads(&self) -> Vec<String> {
        self.inner.last_runtime_reads()
    }

    fn last_log_messages(&self) -> Vec<(String, String)> {
        self.inner.last_log_messages()
    }
}

#[derive(Debug)]
pub struct PerimeterHarnessError(pub String);

impl std::fmt::Display for PerimeterHarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PerimeterHarnessError {}

fn num_cpus_guess() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Run the real pipeline with one explicitly selected wall generator.
pub fn run_pipeline_capturing_perimeters(
    mesh_path: &Path,
    config_path: &Path,
    module_dirs: &[PathBuf],
    wall_generator: WallGenerator,
) -> Result<Vec<PerimeterIR>, PerimeterHarnessError> {
    let mesh = Arc::new(
        slicer_model_io::load_model(mesh_path)
            .map_err(|e| PerimeterHarnessError(format!("model load failed: {e:?}")))?,
    );
    let config_text = std::fs::read_to_string(config_path)
        .map_err(|e| PerimeterHarnessError(format!("failed to read config file: {e}")))?;
    let mut config_source = slicer_runtime::parse_cli_config_source(&config_text)
        .map_err(|e| PerimeterHarnessError(format!("failed to parse config: {e:?}")))?;
    // The test arg is the single source of truth for the wall_generator selector.
    // Any `wall_generator` key in the config file is ignored (and replaced); the
    // paired-coverage test pattern runs both Classic and Arachne on the same input,
    // so the config's `wall_generator: arachne` must not constrain the Classic run.
    config_source.insert(
        "wall_generator".to_string(),
        ConfigValue::String(wall_generator.config_value().to_string()),
    );

    for object in &mesh.objects {
        let key = format!("object_height:{}", object.id);
        if let std::collections::hash_map::Entry::Vacant(entry) = config_source.entry(key) {
            if let Some((z_min, z_max)) = object.world_z_extent {
                entry.insert(ConfigValue::Float((z_max - z_min) as f64));
            }
        }
    }
    for object in &mesh.objects {
        for (subkey, value) in &object.config.data {
            let key = format!("object_config:{}:{}", object.id, subkey);
            config_source.entry(key).or_insert_with(|| value.clone());
        }
    }
    if mesh
        .objects
        .iter()
        .any(|object| object.paint_data.is_some())
        && !config_source.contains_key("slice_has_paint")
    {
        config_source.insert("slice_has_paint".to_string(), ConfigValue::Bool(true));
    }
    if !config_source.contains_key("wipe_tower_enabled") {
        use std::collections::BTreeSet;
        let mut tools = BTreeSet::new();
        for object in &mesh.objects {
            if let Some(paint_data) = &object.paint_data {
                for layer in &paint_data.layers {
                    for value in layer.facet_values.iter().flatten() {
                        if let slicer_ir::PaintValue::ToolIndex(tool) = value {
                            tools.insert(*tool);
                        }
                    }
                }
            }
        }
        if tools.len() >= 2 {
            config_source.insert("wipe_tower_enabled".to_string(), ConfigValue::Bool(true));
        }
    }

    let search_roots = assemble_search_roots(module_dirs, true);
    let mut loaded =
        load_live_modules_for_plan_with_config(&search_roots, num_cpus_guess(), &config_source)
            .map_err(|e| PerimeterHarnessError(format!("failed to load modules: {e}")))?;
    let config_bounds = ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));
    let default_resolved_config = resolve_global_config(&config_source, &config_bounds)
        .map_err(|e| PerimeterHarnessError(format!("config resolution failed: {e:?}")))?;
    let object_ids: Vec<&str> = mesh
        .objects
        .iter()
        .map(|object| object.id.as_str())
        .collect();
    let resolved_configs_map = resolve_per_object_configs(
        &default_resolved_config,
        &config_source,
        &object_ids,
        &config_bounds,
    )
    .map_err(|e| PerimeterHarnessError(format!("per-object config resolution failed: {e:?}")))?;
    validate_support_layer_heights(&resolved_configs_map).map_err(|e| {
        PerimeterHarnessError(format!("support layer height validation failed: {e:?}"))
    })?;
    let per_tool_configs_map =
        resolve_per_tool_configs(&default_resolved_config, &config_source, &config_bounds)
            .map_err(|e| {
                PerimeterHarnessError(format!("per-tool config resolution failed: {e:?}"))
            })?;
    let wasm_handles: HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)> =
        loaded
            .bindings
            .iter()
            .map(|binding| {
                (
                    binding.module.id().to_string(),
                    (
                        Arc::clone(&binding.instance_pool),
                        binding.wasm_component.clone(),
                    ),
                )
            })
            .collect();
    let plan = build_live_execution_plan(
        loaded.sorted_stages,
        loaded.bindings,
        &config_source,
        Arc::new(Vec::new()),
        Arc::new(HashMap::new()),
        &mut loaded.diagnostics,
    )
    .map_err(|e| PerimeterHarnessError(format!("failed to build execution plan: {e:?}")))?;
    let engine = Arc::clone(&loaded.engine);
    let relative = match config_source.get("use_relative_e_distances") {
        Some(ConfigValue::Bool(value)) => *value,
        _ => slicer_runtime::run::DEFAULT_USE_RELATIVE_E_DISTANCES,
    };
    let capturing_runner = PerimeterCapturingLayerStageRunner::new(Box::new(
        WasmRuntimeDispatcher::new(Arc::clone(&engine)),
    ));
    let sink = capturing_runner.sink();
    let pipeline_config = PipelineConfig {
        mesh_ir: mesh,
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(capturing_runner),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(
                DefaultGCodeEmitter::new("arachne_structural_invariants".to_string())
                    .with_resolved_config(default_resolved_config.clone())
                    .with_tool_configs(per_tool_configs_map.clone()),
            ),
            serializer: Box::new(DefaultGCodeSerializer::with_extrusion_mode(relative)),
        },
        resolved_configs: Arc::new(resolved_configs_map),
        default_resolved_config: Arc::new(default_resolved_config),
        bounds: Arc::new(config_bounds),
        wasm_handles,
        cancel_flag: None,
        support_tools: Default::default(),
    };
    run_pipeline_with_raw_config(pipeline_config, &config_source, &NoopLayerProgressSink)
        .map_err(|e| PerimeterHarnessError(format!("pipeline run failed: {e}")))?;

    let mut captured: Vec<_> = sink
        .lock()
        .map_err(|_| PerimeterHarnessError("capture mutex poisoned".to_string()))?
        .values()
        .cloned()
        .collect();
    captured.sort_by_key(|perimeter| perimeter.global_layer_index);
    Ok(captured)
}

#[derive(Debug, Clone)]
pub struct AlignedCoverageMeasurement {
    pub z_plane_mm: f32,
    pub arachne_extent_mm: f32,
    pub classic_extent_mm: f32,
    pub ratio: f32,
    pub global_layer_index: u32,
}

fn x_extent(perimeter: &PerimeterIR) -> Result<(f32, f32), PerimeterHarnessError> {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut z_plane = None;
    for region in &perimeter.regions {
        for wall in &region.walls {
            for point in &wall.path.points {
                if !point.x.is_finite() || !point.z.is_finite() {
                    return Err(PerimeterHarnessError(format!(
                        "non-finite perimeter point on layer {}",
                        perimeter.global_layer_index
                    )));
                }
                min_x = min_x.min(point.x);
                max_x = max_x.max(point.x);
                z_plane = Some(z_plane.map_or(point.z, |z: f32| z.min(point.z)));
            }
        }
    }
    let z_plane = z_plane.ok_or_else(|| {
        PerimeterHarnessError(format!(
            "empty perimeter on layer {}",
            perimeter.global_layer_index
        ))
    })?;
    let extent = max_x - min_x;
    if !extent.is_finite() || extent <= 0.0 {
        return Err(PerimeterHarnessError(format!(
            "non-positive X extent on layer {}",
            perimeter.global_layer_index
        )));
    }
    Ok((extent, z_plane))
}

/// Join paired output by global layer and require the corresponding Z planes
/// to match before calculating Arachne/classic X-extent coverage.
pub fn align_coverage_measurements(
    classic: &[PerimeterIR],
    arachne: &[PerimeterIR],
) -> Result<Vec<AlignedCoverageMeasurement>, PerimeterHarnessError> {
    let classic_by_layer: BTreeMap<_, _> = classic
        .iter()
        .map(|perimeter| (perimeter.global_layer_index, perimeter))
        .collect();
    let arachne_by_layer: BTreeMap<_, _> = arachne
        .iter()
        .map(|perimeter| (perimeter.global_layer_index, perimeter))
        .collect();
    if classic_by_layer.len() != arachne_by_layer.len()
        || classic_by_layer.keys().ne(arachne_by_layer.keys())
    {
        return Err(PerimeterHarnessError(format!(
            "global layer mismatch: classic={} arachne={}",
            classic_by_layer.len(),
            arachne_by_layer.len()
        )));
    }

    let mut aligned = Vec::with_capacity(classic_by_layer.len());
    for (layer_index, classic_layer) in classic_by_layer {
        let arachne_layer = arachne_by_layer[&layer_index];
        let (classic_extent, classic_z) = x_extent(classic_layer)?;
        let (arachne_extent, arachne_z) = x_extent(arachne_layer)?;
        if (classic_z - arachne_z).abs() > 0.0001 {
            return Err(PerimeterHarnessError(format!(
                "Z mismatch on global layer {layer_index}: classic={classic_z} arachne={arachne_z}"
            )));
        }
        aligned.push(AlignedCoverageMeasurement {
            z_plane_mm: (classic_z + arachne_z) * 0.5,
            arachne_extent_mm: arachne_extent,
            classic_extent_mm: classic_extent,
            ratio: arachne_extent / classic_extent,
            global_layer_index: layer_index,
        });
    }
    Ok(aligned)
}
