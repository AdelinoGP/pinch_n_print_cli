use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigView, SemVer};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    CompiledModuleBuilder, CompiledModuleLive, LoadedModuleBuilder, WasmInstancePool,
    WasmRuntimeDispatcher,
};
use slicer_sdk::native::NativeStageEntry;

use super::wasm_cache;

pub struct IntegratedParitySpec {
    pub module_id: String,
    pub wasm_path: PathBuf,
    pub stage: String,
    pub version: SemVer,
    pub min_ir_schema: SemVer,
    pub max_ir_schema: SemVer,
    pub tier: String,
    pub claims: Vec<String>,
    pub config: Arc<ConfigView>,
    pub native_entry: NativeStageEntry,
}

/// Build the native and real-component bindings once, then let the family test
/// own its input carriers, stage invocation, and comparator result.
pub fn run_integrated_parity<F, R>(spec: IntegratedParitySpec, execute: F) -> R
where
    F: for<'a> FnOnce(
        &WasmRuntimeDispatcher,
        &CompiledModuleLive<'a>,
        &CompiledModuleLive<'a>,
    ) -> R,
{
    let wasm_module = CompiledModuleBuilder::new(spec.module_id.clone())
        .claims(spec.claims.clone())
        .config_view(Arc::clone(&spec.config))
        .build();
    let native_module = CompiledModuleBuilder::new(spec.module_id)
        .claims(spec.claims)
        .config_view(spec.config)
        .build();
    let loaded = LoadedModuleBuilder::new(
        wasm_module.module_id().as_str(),
        spec.version,
        &spec.stage,
        spec.tier,
        PathBuf::from("/dev/null"),
    )
    .min_host_version(spec.version)
    .min_ir_schema(spec.min_ir_schema)
    .max_ir_schema(spec.max_ir_schema)
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    let component = wasm_cache::compiled_component_at(&spec.wasm_path);
    let wasm_live = CompiledModuleLive::new(
        wasm_module.module_id(),
        pool,
        Some(component),
        wasm_module.claims(),
        Arc::clone(wasm_module.config_view()),
    );
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(spec.native_entry);
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&wasm_cache::shared_engine()));
    execute(&dispatcher, &native_live, &wasm_live)
}
