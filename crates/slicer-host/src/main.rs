//! Binary entry point for the slicer-host runtime.
//!
//! Parses CLI arguments via clap and dispatches to the pipeline orchestration
//! or config-schema query functions.

use std::sync::{Arc, Mutex};

use clap::Parser;
use slicer_host::model_loader::{load_model, object_world_z_extent};
use slicer_host::pipeline::{run_pipeline_with_events, PipelineConfig, PipelineStageRunners};
use slicer_host::progress_events::{
    JsonLinesEmitter, ProgressEventEmitter, RuntimeProgressSink, SliceEventCollector,
};
use slicer_host::{
    build_config_schema_json, build_live_execution_plan, load_live_modules_for_plan,
    load_modules_from_roots, parse_cli_config_source, DefaultGCodeEmitter, DefaultGCodeSerializer,
    HostCli, HostCommands,
};
use slicer_host::dispatch::WasmRuntimeDispatcher;

/// No-op prepass runner for MVP (no WASM modules loaded yet).
#[allow(dead_code)]
struct NoopPrepassRunner;
impl slicer_host::PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
    ) -> Result<(slicer_host::PrepassStageOutput, Vec<String>), slicer_host::PrepassExecutionError> {
        Ok((slicer_host::PrepassStageOutput::None, Vec::new()))
    }
}

/// No-op layer runner for MVP.
#[allow(dead_code)]
struct NoopLayerRunner;
impl slicer_host::LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _layer: &slicer_ir::GlobalLayer,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        _arena: &mut slicer_host::LayerArena,
    ) -> Result<(slicer_host::LayerStageOutput, Vec<String>), slicer_host::LayerStageError> {
        Ok((slicer_host::LayerStageOutput::Success, Vec::new()))
    }
}

/// No-op finalization runner for MVP.
#[allow(dead_code)]
struct NoopFinalizationRunner;
impl slicer_host::FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        _layers: &mut Vec<slicer_ir::LayerCollectionIR>,
    ) -> Result<slicer_host::FinalizationOutput, slicer_host::FinalizationError> {
        Ok(slicer_host::FinalizationOutput::Success)
    }
}

/// No-op postpass runner for MVP.
#[allow(dead_code)]
struct NoopPostpassRunner;
impl slicer_host::PostpassStageRunner for NoopPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        _gcode_ir: &mut slicer_ir::GCodeIR,
    ) -> Result<slicer_host::PostpassOutput, slicer_host::PostpassError> {
        Ok(slicer_host::PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        text: String,
    ) -> Result<slicer_host::PostpassOutput, slicer_host::PostpassError> {
        Ok(slicer_host::PostpassOutput::TextSuccess { text })
    }
}

/// Conservative default for host parallelism when building instance pools
/// for `layer-parallel-safe` modules. The scheduler clamps to `>= 1`, so
/// keeping this at the process's logical-core count (falling back to 1)
/// is safe without pulling in a `num_cpus` dependency.
fn num_cpus_guess() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn main() {
    let cli = HostCli::parse();
    match cli.command {
        HostCommands::Run {
            module: _,
            model,
            config,
            output,
            module_dir,
        } => {
            // Load model
            let model_path = std::path::Path::new(&model);
            let mesh_ir = match load_model(model_path) {
                Ok(ir) => Arc::new(ir),
                Err(e) => {
                    eprintln!("error: failed to load model: {e}");
                    std::process::exit(1);
                }
            };

            // Parse the user-facing JSON config (if supplied) into the raw
            // `HashMap<ConfigKey, ConfigValue>` that every per-module
            // `Arc<ConfigView>` will be filtered from via
            // `bind_module_config_view` inside `build_live_execution_plan`.
            // Passing an empty source here is explicitly NOT a placeholder:
            // it's the real config source when the user doesn't supply one.
            let mut config_source = match config.as_deref() {
                Some(path) => match std::fs::read_to_string(path) {
                    Ok(text) => match parse_cli_config_source(&text) {
                        Ok(map) => map,
                        Err(e) => {
                            eprintln!("error: failed to parse --config: {e}");
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        eprintln!("error: failed to read --config file: {e}");
                        std::process::exit(1);
                    }
                },
                None => std::collections::HashMap::new(),
            };

            // Inject per-object Z extents from the loaded mesh as
            // `object_height:<id>` config keys so the layer planner
            // module (which otherwise falls back to a single first-
            // layer proposal when no height is known) sees real per-
            // object geometry. Extents are computed in world space by
            // applying each object's `Transform3d` so transformed
            // multi-object scenes (scale/rotation/translation) yield a
            // correct planner height. User-supplied values on
            // `--config` win over host-derived defaults.
            for object in &mesh_ir.objects {
                let key = format!("object_height:{}", object.id);
                if config_source.contains_key(&key) {
                    continue;
                }
                if let Some((z_min, z_max)) = object_world_z_extent(object) {
                    config_source.insert(
                        key,
                        slicer_ir::ConfigValue::Float((z_max - z_min) as f64),
                    );
                }
            }

            // Discover and plan every module under --module-dir. Modules
            // are topologically sorted within each stage and laid out in
            // the canonical STAGE_ORDER. Non-fatal discovery diagnostics
            // are surfaced on stderr; fatal failures terminate with a
            // structured message (docs/04 §Fixed Stage Order).
            let module_dir_path = std::path::PathBuf::from(&module_dir);
            let loaded = match load_live_modules_for_plan(
                std::slice::from_ref(&module_dir_path),
                num_cpus_guess(),
            ) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("error: failed to load modules from '{module_dir}': {e}");
                    std::process::exit(1);
                }
            };
            for diag in &loaded.diagnostics {
                eprintln!(
                    "{level:?}: {path}: {msg}",
                    level = diag.level,
                    path = diag.path.display(),
                    msg = diag.message,
                );
            }

            // The live plan/build path. Every per-module ConfigView is
            // synthesised through `bind_module_config_view` inside this
            // helper; the plan-build guardrail
            // (`ExecutionPlanError::UndeclaredConfigKey`) fails closed if
            // a module's declared-read invariant is ever violated (docs/03
            // §host-boundary enforcement; docs/02 §pre-filtered config).
            let plan = match build_live_execution_plan(
                loaded.sorted_stages,
                loaded.bindings,
                &config_source,
                Arc::new(Vec::new()),
                Arc::new(std::collections::HashMap::new()),
            ) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: failed to build execution plan: {e}");
                    std::process::exit(1);
                }
            };

            // Build WasmRuntimeDispatcher instances backed by the same engine
            // that compiled the module components.  Using the same engine is
            // required: wasmtime ties compiled Components to the Engine instance
            // that produced them, and creating a second Engine would cause
            // component instantiation to fail at dispatch time.
            let engine = Arc::clone(&loaded.engine);
            let config = PipelineConfig {
                mesh_ir,
                plan,
                runners: PipelineStageRunners {
                    prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    emitter: Box::new(DefaultGCodeEmitter::new("slicer-host 0.1.0".into())),
                    serializer: Box::new(DefaultGCodeSerializer::new()),
                },
            };

            // Route per-layer progress events (including host-built-in
            // paint-annotation degraded-success warnings) through the real
            // JSONL emitter on stderr and aggregate them into a
            // `SliceEventCollector`. G-code continues to go to stdout, so
            // the JSONL transport targets stderr to avoid interleaving.
            let emitter: Arc<dyn ProgressEventEmitter> =
                Arc::new(JsonLinesEmitter::new(std::io::stderr()));
            let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
            let sink = RuntimeProgressSink::new(emitter, Arc::clone(&collector));

            match run_pipeline_with_events(config, &sink) {
                Ok(result) => {
                    if let Some(out_path) = output {
                        if let Err(e) = std::fs::write(&out_path, &result.gcode_text) {
                            eprintln!("error: failed to write output: {e}");
                            std::process::exit(1);
                        }
                    } else {
                        print!("{}", result.gcode_text);
                    }
                }
                Err(e) => {
                    eprintln!("error: pipeline failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        HostCommands::ConfigSchema { module_dir } => {
            let path = std::path::PathBuf::from(module_dir);
            let report = match load_modules_from_roots(&[path]) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error loading modules: {e:?}");
                    std::process::exit(1);
                }
            };
            let json = build_config_schema_json(&report.modules);
            println!("{}", json);
        }
    }
}

#[cfg(any())]
mod _stale_build_plan {
                  /// Build an ExecutionPlan from loaded modules with compiled WASM components.
      ///                                                                                                                                                                                                                           
      /// For each loaded module, reads the companion .wasm file, compiles it to a                                                                                                                                                  
      /// WASM component via the shared engine, and stores it in the CompiledModule.                                                                                                                                         
      /// Compilation failures are reported as diagnostics (stderr) but do not prevent                                                                                                                                       
      /// plan construction — the dispatcher will surface a structured error at call time.                                                                                                                                   
      fn build_plan_from_loaded_modules(                                                                                                                                                                                     
          modules: &[slicer_host::LoadedModule],                                                                                                                                                                             
          engine: &Arc<WasmEngine>,                                                                                                                                                                                          
      ) -> slicer_host::ExecutionPlan {                                                                                                                                                                                      
          use slicer_host::{                                                                                                                                                                                                 
              build_wasm_instance_pool, CompiledModule, CompiledStage, ExecutionPlan, IrAccessMask,                                                                                                                          
              WasmArtifactMetadata,                                                                                                                                                                                          
          };                                                                                                                                                                                                                 
          use slicer_ir::ConfigView;                                                                                                                                                                                         
                                                                                                                                                                                                                             
          let mut prepass_stages: Vec<CompiledStage> = Vec::new();                                                                                                                                                           
          let mut per_layer_stages: Vec<CompiledStage> = Vec::new();                                                                                                                                                         
          let mut layer_finalization_stage: Option<CompiledStage> = None;                                                                                                                                                    
          let mut postpass_stages: Vec<CompiledStage> = Vec::new();                                                                                                                                                          
                                                                                                                                                                                                                             
          // Group modules by stage                                                                                                                                                                                          
          let mut stage_groups: std::collections::BTreeMap<String, Vec<&slicer_host::LoadedModule>> =                                                                                                                        
              std::collections::BTreeMap::new();                                                                                                                                                                             
          for module in modules {                                                                                                                                                                                            
              stage_groups                                                                                                                                                                                                   
                  .entry(module.stage.clone())                                                                                                                                                                               
                  .or_default()                                                                                                                                                                                              
                  .push(module);                                                                                                                                                                                             
          }                                                                                                                                                                                                                  
                                                                                                                                                                                                                             
         // Build compiled stages per group                                                                                                                                                                                 
         for (stage_id, stage_modules) in &stage_groups {                                                                                                                                                                   
              let compiled_modules: Vec<CompiledModule> = stage_modules                                                                                                                                                      
                  .iter()                                                                                                                                                                                                    
                  .map(|m| {                                                                                                                                                                                                 
                      let parallelism = if m.layer_parallel_safe { 4 } else { 1 };                                                                                                                                           
                      let pool = Arc::new(                                                                                                                                                                                   
                         build_wasm_instance_pool(                                                                                                                                                                          
                              m,                                                                                                                                                                                             
                             parallelism,                                                                                                                                                                                   
                              WasmArtifactMetadata {                                                                                                                                                                         
                                  uses_shared_memory: false,                                                                                                                                                                 
                              },                                                                                                                                                                                             
                         )                                                                                                                                                                                                  
                          .expect("pool build should succeed"),                                                                                                                                                              
                      );                                                                                                                                                                                                     
                                                                                                                                                                                                                             
                      // Compile the WASM component from the module's .wasm file.                                                                                                                                            
                      let wasm_component = match std::fs::read(&m.wasm_path) {                                                                                                                                               
                          Ok(bytes) => match engine.compile_component(&bytes) {                                                                                                                                              
                              Ok(component) => Some(Arc::new(component)),                                                                                                                                                    
                              Err(e) => {                                                                                                                                                                                    
                                  eprintln!(                                                                                                                                                                                 
                                      "warning: failed to compile WASM for module '{}': {}",                                                                                                                                 
                                      m.id, e                                                                                                                                                                                
                                  );                                                                                                                                                                                         
                                  None                                                                                                                                                                                       
                              }                                                                                                                                                                                              
                          },                                                                                                                                                                                                 
                          Err(e) => {                                                                                                                                                                                        
                              eprintln!(                                                                                                                                                                                     
                                  "warning: failed to read WASM file '{}' for module '{}': {}",                                                                                                                              
                                  m.wasm_path.display(),                                                                                                                                                                     
                                  m.id,                                                                                                                                                                                      
                                  e                                                                                                                                                                                          
                              );                                                                                                                                                                                             
                              None                                                                                                                                                                                           
                          }                                                                                                                                                                                                  
                      };                                                                                                                                                                                                     
                                                                                                                                                                                                                             
                      CompiledModule {                                                                                                                                                                                       
                          module_id: m.id.clone(),                                                                                                                                                                           
                         instance_pool: pool,                                                                                                                                                                               
                         ir_read_mask: IrAccessMask {                                                                                                                                                                       
                              paths: m.ir_reads.clone(),                                                                                                                                                                     
                          },                                                                                                                                                                                                 
                         ir_write_mask: IrAccessMask {                                                                                                                                                                      
                             paths: m.ir_writes.clone(),                                                                                                                                                                    
                       },                                                                                                                                                                                                 
                      config_view: Arc::new(ConfigView::new()),                                                                                                                                                                                                
                     wasm_component,                                                                                                                                                                                    
                  }                                                                                                                                                                                                      
              })                                                                                                                                                                                                         
             .collect();                                                                                                                                                                                                
                                                                                                                                                                                                                       
           let compiled_stage = CompiledStage {                                                                                                                                                                           
             stage_id: stage_id.clone(),                                                                                                                                                                                
             modules: compiled_modules,                                                                                                                                                                                 
          };                                                                                                                                                                                                             
                                                                                                                                                                                                                         
        if stage_id.starts_with("PrePass::") {                                                                                                                                                                         
              prepass_stages.push(compiled_stage);                                                                                                                                                                       
          } else if stage_id.starts_with("Layer::") {                                                                                                                                                                    
               per_layer_stages.push(compiled_stage);                                                                                                                                                                     
           } else if stage_id == "PostPass::LayerFinalization" {                                                                                                                                                          
               layer_finalization_stage = Some(compiled_stage);                                                                                                                                                           
         } else if stage_id.starts_with("PostPass::") {                                                                                                                                                                 
              postpass_stages.push(compiled_stage);                                                                                                                                                                      
          }                                                                                                                                                                                                              
      }                                                                                                                                                                                                                  
                                                                                                                                                                                                                            
        ExecutionPlan {                                                                                                                                                                                                    
              prepass_stages,                                                                                                                                                                                                
             per_layer_stages,                                                                                                                                                                                              
              layer_finalization_stage,                                                                                                                                                                                      
             postpass_stages,                                                                                                                                                                                               
       global_layers: Arc::new(Vec::new()),                                                                                                                                                                           
       region_plans: Arc::new(HashMap::new()),                                                                                                                                                                        
   }                                                                                                                                                                                                                  
}
}
